# Repository Guidelines

## Project Overview

**单词本** — a paper-notebook-style vocabulary learning web app. Users organize words into independent "wordbooks" (单词书), review them in a simulated paper-book view (ruled lines, tap-to-cover self-testing, continuous vertical scroll with lazy-loaded pages), and manage them in a list view. Backend: Rust (axum 0.8 + SeaORM 2.0 + sea-orm-migration). Frontend: React 19 + Vite 8 + TailwindCSS v4. Single-binary deployment (rust-embed embeds `frontend/dist`). Database: SQLite by default, PostgreSQL switchable via YAML config.

## Architecture & Data Flow

```
Browser (React SPA)
   │  /api/* (dev: Vite proxy → :3000)
   ▼
crates/server (axum 0.8)
   router.rs          entry: aggregates top-level domain routers; assembles AppState
   business/          URL-aligned nested domains (Next.js-style recursion)
     wordbooks.rs + wordbooks/      /api/wordbooks...
       router.rs                   registers own routes + merges subdomain
       words.rs + words/           /api/wordbooks/{book_id}/words...
         router.rs                 registers own routes (no top-level /api/words)
     <域>/router.rs   HTTP layer: extractors, thin handlers, param parsing, own route registration
     <域>/service.rs  business logic: validation, cache invalidation, DTO assembly
     <域>/repo.rs     SeaORM persistence access
     <域>/dto/        request/response boundary models (create.rs / update.rs / resp.rs / list.rs / search.rs)
     <域>/error.rs    thiserror domain errors, mapped to ApiError at the boundary
   common/            shared cross-business: error.rs (ApiError aggregate), http/ (SPA static), model/ (PageResp + paging), state.rs (AppState)
   config/            YAML config structs
   init/              startup sequencing (logging, config load, db init)
crates/entity         SeaORM 2 dense entities + Definition (definitions JSON column model)
crates/migration      sea-orm-migration schema migrations
```

Data flow: router parses HTTP (State/Path/Query/Json) → service validates & queries via repo → returns `dto::resp` structs serialized as JSON. Errors bubble as domain errors → `ApiError` → `IntoResponse` (404/400/500 + `{"error": msg}`).

Key backend decisions:
- `init::db::init_db` runs `PRAGMA foreign_keys = ON` **before** migrations for SQLite — without it, deleting a wordbook does not cascade-delete words.
- Word counting uses a single `GROUP BY` aggregate query (no N+1 per-book count).
- `definitions` stored as JSON column; `Definition` (entity crate) is the typed model for that column, converted with `serde_json::to_value/from_value`.
- `timestamp_with_time_zone()` in migrations — `timestamp()` breaks PG (DateTimeUtc needs TIMESTAMPTZ); both work on SQLite.
- Shared contract types are exported to TypeScript via `ts-rs`: `#[ts(export, export_to = "...ts")]` generates a test; running `cargo test` writes the files to `frontend/src/generated/` (see `.cargo/config.toml` `TS_RS_EXPORT_DIR`).
- **批量导入（模板约定）**: 6 列表头 `单词,音标,词性,释义,例句,标签`；**每行一个义项**（一个词性对应一个释义），同一单词的多义项写多行（单词列重复），导入时按拼写（trim+小写）合并为一个单词——音标/例句取组内首个非空、标签取并集、义项按行序拼接；标签列多个标签用 `；`/`;` 分隔，缺失标签自动创建；全空行跳过；5 列旧模板缺列取空自动兼容。
- **词性白名单 21 项书写形式**（前后端一致，后端 `words/pos.rs` 的 `WordPos` 枚举——strum 声明式映射，c/C、u/U、cu/CU 合并为同义变体——与前端下拉）：`n. / c / C / u / U / cu / CU / v. / vt. / vi. / adj. / adv. / prep. / conj. / pron. / num. / art. / interj. / aux. / abbr. / phr.`（C=可数、U=不可数名词；vt./vi.=及物/不及物动词；留空合法）。
- **导入三步会话（后端为主）**: ①`POST /words/import/preview`（multipart 上传，文件只传一次；格式按 `ImportFileType` 枚举解析，csv/xlsx/xls/ods）解析后把**全部解析行**（含全空行）缓存为 token 会话；②`POST /words/import/rows`（token + page/page_size/filter/updates）在会话内应用行级修正 → 重新校验 → 按**组切片**分页返回（组不跨页）；`filter` 为 `ImportFilter` 枚举（all/error/duplicate，serde `deserialize_with` 边界解析，非法值返回中文错误）；③`POST /words/import`（token + update_rows）一次性消费会话执行导入。前端仅显示与草稿编辑（防抖 500ms 提交），统计/错误标记/分页/筛选全部由后端计算。
- **导入容错**: 校验失败的行跳过并报告行号（`skipped_errors`），其余正常导入；同书重复拼写的单词默认更新（覆盖字段、标签合并 union），预览中可跳过（`skipped_duplicates`）；结果统计 `imported/updated/skipped_errors/skipped_duplicates/created_tags`。
- **导入会话缓存生命周期配置化**: `config.import` 段（`max_rows`/`cache_ttl_secs`/`cache_cap`/`cache_cleanup_secs`，缺省 5000/1800/16/60，加载时校验 >0）；`AppState::spawn_import_cache_cleaner` 后台定时清理过期会话（随 `router::build` 启动）；`cache_cap` 超出整体清空。

## Key Directories

```
crates/entity/src/          SeaORM 2.0 dense entities (word.rs, wordbook.rs) + definition.rs (JSON column model)
crates/migration/src/       Migrator + m20260802_* migration files (chronological)
crates/server/src/
  business/wordbooks.rs + wordbooks/   /api/wordbooks... (router/service/repo/dto/error)
  business/wordbooks/words.rs + words/ /api/wordbooks/{book_id}/words... (router/service/repo/dto/error/import/order/sort/sort_dir/tag_group/tag_match/file_type/import_filter/pos/template_format)
  common/error.rs           ApiError aggregate (HTTP boundary)
  common/http/              Asset (rust-embed) + spa.rs static handler + json.rs/path.rs (ApiJson/ApiPath 提取器) + normalize.rs (错误归一中间件)
  common/model/page.rs      PageResp<T> pagination model
  common/model/paging.rs    parse_paging shared pagination query parsing
  common/state.rs           AppState (db + wordbooks_cache + shuffle_cache + import_cache + config)
  config/                   config.rs root + server.rs / database.rs / import.rs
  init/                     run() entry + db.rs connection/migration
frontend/src/
  generated/                ts-rs generated TS contract types (source of truth, do not hand-edit)
  api.ts                    fetch wrapper; re-exports types from generated/
  pages/ components/        UI
frontend/dist/              build output (embedded by rust-embed; .gitkeep placeholder)
```

## Development Commands

```bash
# Backend (workspace root)
cargo build                    # dev build, whole workspace
cargo run -p server            # dev server on :3000 (config.yaml if present, else built-in defaults)
cargo build --release          # production binary (embeds frontend/dist at compile time)
cargo clippy --workspace --all-targets --all-features -- -D warnings   # validation gate
cargo test --workspace         # unit tests + ts-rs export tests (refreshes frontend/src/generated/)

# Frontend
cd frontend && npm run dev     # Vite dev :5173, proxies /api → :3000
cd frontend && npm run build   # tsc -b + vite build → dist/ (MUST run before cargo build for fresh embed)

# Production
./target/release/server        # run from project root (reads ./config.yaml; copy from config.example.yaml)

# Docker (Dockerfile: cache mount 持久化 cargo registry/target，依赖未变时改码重建约 1 分钟)
docker compose up -d --build   # build & start (data in ./data/, port 3000)
docker compose logs -f         # follow logs
docker compose down            # stop (./data 保留)
```

Verification: clippy gate + `cargo test`; API smoke-tested via curl against a running server (see Testing & QA).

## Code Conventions & Common Patterns

**Strict project rules (user-mandated, enforce on all new code):**
- **Rust 开发严格遵守 `skill:rust-project-backend-standards` 的所有规范**（模块布局、分层、typed boundaries、enum-first、错误边界、删除不留死代码、clippy 门禁等；任务开始与交付前按该 skill 的 checklist 自检）。
- **One struct/trait/enum per `.rs` file** (entity/migration follow SeaORM conventions; same-action Req/Resp DTOs may share a file).
- **No `pub use` re-export barrels.** Reference full module paths. `foo.rs` declares child modules; never `mod.rs`.
- **Layered business domains (URL-aligned):** `business/` mirrors the API path structure — `wordbooks/` owns `/api/wordbooks...`, nested `wordbooks/words/` owns `/api/wordbooks/{book_id}/words...` (no top-level `/api/words`). Each `business/<域>/` owns its `router.rs` (HTTP only, registers its own routes), `service.rs` (business rules), `repo.rs` (SeaORM persistence), `dto/` (boundary models), `error.rs` (thiserror domain errors). Each layer's `router.rs` merges its subdomains recursively; the top-level `router.rs` only aggregates top-level domains. Controllers are thin; validation and queries live in services.
- **DTO files:** named by action without suffixes — `dto/create.rs`, `dto/update.rs`, `dto/resp.rs`; type names stay fully semantic (`CreateWordbookReq`, `UpdateWordbookReq`, `WordbookResp`). HTTP query params are typed too: `dto/list.rs` (`ListWordsQuery`) / `dto/search.rs` (`SearchWordsQuery`); shared pagination parsing lives in `common/model/paging.rs` (`parse_paging`).
- **Errors:** each business domain defines its own `thiserror` enum with semantic variants (no vague `BadRequest(String)` in services); `common::error::ApiError` is the HTTP aggregate, mapped via `From` in each domain's `error.rs`.
- **Closed sets are enums:** query params (`order`, `seed`, `sort`) are parsed once at the router boundary into `WordOrder` / `SortField` / `SortDir`; business logic matches enum variants, never string literals. `SortField`/`SortDir`/`TagMatch` use `strum::EnumString` (declarative mapping); `WordOrder` keeps a handwritten parse because `Random(String)` carries a payload strum cannot express. **JSON 边界反序列化必须 enum-first（禁止 `Option<String>` 中间态再手工校验）**：`TagMatch`（组内匹配 and/or/none + 组间连接词）与 `ImportFilter` 一样，在 serde `deserialize_with`（`tag_match.rs::deserialize_tag_match` / `deserialize_links`）里一次性解析为枚举变体，非法值返回中文错误消息；业务层只匹配枚举变体。洗牌缓存 key 直接使用 `TagMatch` 枚举分量（`common::state` 已依赖 business DTO 类型，不再字符串化）。请求体/查询里的封闭集合同此处理：`ImportFilter`（导入行筛选，serde `deserialize_with` + TS union 导出）、`TemplateFormat`（模板下载格式）、`ImportFileType`（导入文件格式）、`WordPos`（词性白名单）。
- **ts-rs contract:** frontend-facing DTOs derive `TS` with `#[ts(export, export_to = "...ts")]` (+ `rename` where the TS name differs, `#[ts(type = "number")]` for single u64, `#[ts(type = "Array<number>")]` for u64 collections, `#[ts(optional)]` for omitted request fields). Regenerate with `cargo test` after DTO changes; never hand-edit `frontend/src/generated/`. workspace 依赖 `ts-rs` 启用 `no-serde-warnings` feature：DTO 字段可用 `#[serde(deserialize_with)]` 做带中文错误消息的自定义反序列化（如 `ImportFilter`），ts-rs 静默忽略该属性。
- **完成任务必须同步更新 `AGENTS.md`:** 每次功能/修复/重构落地后，把仓库事实变化写回本文件（新增约定、架构决策、文件职责、验证方式变更），过时描述同步修正，保证 `AGENTS.md` 始终反映当前代码；纯实验或未落地的工作不写。

**Backend patterns:**
- Services are unit structs (`pub struct WordbookService;`) with associated async fns taking `&AppState` as first arg (access DB via `state.db.as_ref()`); repos are unit structs taking `&DatabaseConnection`.
- `AppState` holds the DB pool, the full `Config`, and three process-memory caches: `wordbooks_cache` (list result, invalidated via `state.invalidate_wordbooks()` by wordbook CRUD + word create/delete), `shuffle_cache` ((book_id, 筛选组 `Vec<TagGroup>`, 组间连接词 `Vec<TagMatch>`, seed) → shuffled id sequence — 枚举分量直接做 key（common 不重复字符串化）；cap `SHUFFLE_CACHE_CAP`, cleared on word create/delete/update_tags/batch_tag), and `import_cache` (导入预览会话：token → book_id + 全量行 typed 数据 `Arc<Vec<ImportRowData>>`（跨请求缓存不中继 serde_json 字节）+ 创建时间；TTL/容量由 `config.import` 控制，后台清理任务按 `cache_cleanup_secs` 扫描；导入执行时一次性消费)。Locks are `parking_lot::Mutex`, never held across `.await`（async fn 内必须用块作用域限定 guard，否则 Future 非 Send 无法作 axum handler）。
- Handlers return `Result<impl IntoResponse, ApiError>`; `?` converts domain errors and `DbErr` via `From`. `ApiError` maps NotFound/BadRequest/Unauthorized/Db to 404/400/401/500 with user-facing Chinese messages; 500 responses never leak internals (Db details only go to the log).
- **Error response spec (enforced):** every error response is `{"error": 中文消息}` JSON. Request-body extraction MUST use `ApiJson<T>` (`common/http/json.rs`) and path params MUST use `ApiPath<T>` (`common/http/path.rs`) — axum 0.8 handler extraction failures short-circuit via `rejection.into_response()` and never call `From<Rejection>`, so raw `Json`/`Path` extractors produce English plain-text/422 responses; `ApiJson`/`ApiPath` have `Rejection = ApiError` (400, Chinese). `Query` DTO fields stay `String`-typed (no rejection possible). Framework-level plain-text responses (405 Method Not Allowed, uncaught 500) are wrapped into JSON by the `common/http/normalize.rs` middleware (mounted inside the compression layer in `router.rs`).
- Validation returns domain errors whose `thiserror` text is user-facing Chinese.
- SeaORM: `Entity::find().filter(Column::X.eq(v)).order_by_asc(...).paginate(db, size)`, then `fetch_page(page - 1)` (**0-based offset**) + `num_items()`; API pages are 1-based. `total_pages = total.div_ceil(page_size)`.
- SQLite: `init/db.rs` sets `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`, `cache_size=-20000` after connect.
- Static assets: `assets/` (Vite hashed) and `fonts/` (versioned woff2) served with `Cache-Control: public, max-age=31536000, immutable`; index.html `no-cache`. Whole router wrapped in tower-http `CompressionLayer` (gzip). Fonts are self-hosted in `frontend/public/fonts/` (no Google Fonts CDN).
- Axum 0.8 path syntax: `/api/wordbooks/{id}` (curly braces); multiple path params use a tuple extractor `Path((book_id, id)): Path<(i32, i32)>` (one `Path` extractor per handler).

**Frontend patterns:**
- Components receive plain props; pages hold state; `api.ts` centralizes fetch and re-exports contract types from `frontend/src/generated/` (ts-rs output).
- Reader settings (page size, font px) persisted in `localStorage` under `qw_page_size` / `qw_font_scale`; loaded via lazy `useState(loader)`.
- 标签筛选（纸质/列表两模式共享，按书持久化）：WordbookDetail 持有 `tagFilter: TagFilter`（`{ groups: TagFilterGroup[]; links: ('and'|'or')[] }`，`qw_filter_${bookId}` 持久化；旧 `qw_filter_tags_*`/`qw_filter_match_*` 自动迁移后清除）；`links[i]` = 组 i 与组 i+1 之间的连接词，**「且」优先于「或」**（标准布尔优先级，后端按或切段、段内且组合）。筛选面板 `TagFilterPanel.tsx`（导航「标签」按钮下拉，`w-72 max-w-[calc(100vw-2rem)]`，手机/电脑同一交互）：每组卡片 = 组头（序号 + 删除按钮，仅组数>1 时）+ 独占一行的三态分段（全部匹配/任一匹配/无标签，避免窄面板换行）+ 全量标签 chips（每组选中独立；无标签组 chips 禁用）；组间连接词控件（且/或）在两组之间；空筛选渲染一个默认空组占位。「全部单词」清空。**面板 onChange 传更新函数** `(prev: TagFilter) => TagFilter`（函数式 setState 按序应用，连续点选不互相覆盖）；清缓存重载第 1 页由 WordbookDetail 的 `tagFilter` 变化 effect 统一处理（首次挂载跳过，打乱状态保持）；发送前 `api.ts::sanitizeTagFilter` 剔除空组并重排 links。后端 `words.list`/`words.query` 接受 `tag_groups`（JSON：`{"groups":[{"mode":"and"|"or"|"none","ids":[...]}...],"links":["and"|"or"...]}`，links 长度必须 = 组数-1，无隐式缺省；none 组 ids 必须为空，均校验报中文错误），`repo.rs::with_tag_filter` 按组生成 `word_id IN/IN NOT (子查询)` 条件（And = GROUP BY + `HAVING COUNT(DISTINCT tag_id)=N` 交集子查询；Or = 并集子查询，word_tag 复合主键无重复行故无需 DISTINCT；None = `NOT IN (SELECT word_id FROM word_tag)` 无标签匹配），组间按「且优先于或」嵌套组合（`Condition` 嵌套括号）。
- PaperBookView data flow (scroll mode): `WordbookDetail` holds `pages: Array<{ d: Page<Word>; pageNo: number }>` (loaded pages, ascending); page cache `Map<"${bookId}:${page}:${size}:${seed}:${JSON.stringify(tagFilter)}", Page<Word>>` in `useRef` + `fetchPageRaw` (cache hit + in-flight dedupe via `inflight` map) + `fetchPage` (seq guard for stale responses); `loadFrom(p)` rebuilds from page p with prefetch of next 2 pages, `loadMore(nextPageNo, totalPages)` appends on sentinel trigger; mutations clear cache and reload from `pages[0]`; page number persisted as first loaded page (`qw_page_${bookId}`).
- PaperBookView (scroll mode): continuous vertical scroll; all loaded pages' words merged into ONE grid (row-group slicing: `cols` word cells → `cols` ruled lines → `cols` definition cells per row-group, so lines align across the row; `cols` = 2/3/4/5 by Tailwind breakpoint, recomputed on resize); lazy loading via IntersectionObserver sentinel (800px early trigger, observer rebuilt on `pages.length`/`loading` change to keep filling viewport, 1.5s no-progress debounce against retry storms); long words/phonetics wrap (break-words + min-w-0), definitions render in full (no line-clamp); tap word/meaning toggles blur cover (`Covered`, `aria-pressed` + persisted `wordDiff`/`defDiff` in `qw_cover_${bookId}`); tag chips are display-only, all tag add/remove happens in the TagQuickModal opened via the wrench button; chip font & wrench icon scale with fontScale (`FontStyles.chipFont`/`iconSize`).
- Styling: **Visual Organic** design system from `docs/dev/style.md` — palette `ivory #F8F4EF` / `charcoal #2F2A25` / `clay #C58F6D` / `sage #C9D5C6` / `sand #E5D8C8` (defined in `index.css` `@theme`), fonts DM Serif Display (headings, `font-serif`) + Plus Jakarta Sans (body). **No emoji, no gradients** in UI (user-mandated); SVG icons in `components/Icons.tsx` (stroke-based).

## Important Files

| File | Why |
|---|---|
| `crates/server/src/init/db.rs` | Connection + PRAGMA foreign_keys + migration order (failure here breaks cascades) |
| `crates/server/src/router.rs` | Top-level route aggregation; add new top-level domains here (domain routes live in each domain's router.rs) |
| `crates/server/src/business/wordbooks/words/service.rs` | Pagination, validation, seeded shuffle, JSON conversion, 导入预览/执行/分页重校验（import_preview/page_rows/import_words） |
|`crates/server/src/business/wordbooks/words/import.rs`|导入解析：行式义项解析（行号=物理行号，csv 空行不丢位）、行级校验（prepare_rows）、按拼写分组（group_rows）、标签解析（parse_tags，20 字符上限）；文件格式匹配走 `file_type.rs` 的 `ImportFileType` 枚举（csv/xlsx/xls/ods）|
|`crates/server/src/business/wordbooks/words/order.rs`|WordOrder enum + query param parsing（词性白名单在 `pos.rs` 的 `WordPos` 枚举；导入行筛选枚举在 `import_filter.rs` 的 `ImportFilter`；模板格式枚举在 `template_format.rs` 的 `TemplateFormat`）|
|`crates/server/src/business/wordbooks/words/tag_group.rs`|标签筛选组模型：`TagGroup`（mode + ids）+ `TagGroupsParam`（groups + links，JSON 边界反序列化，链接长度 = 组数-1）|
|`crates/server/src/business/wordbooks/words/tag_match.rs`|`TagMatch` 枚举（and/or/none）+ `deserialize_tag_match`/`deserialize_links`（serde deserialize_with，中文错误）|
| `crates/server/src/config/import.rs` | 导入配置：max_rows / cache_ttl_secs / cache_cap / cache_cleanup_secs（缺省 5000/1800/16/60，serde default + Default） |
| `crates/server/src/common/http/asset.rs` | rust-embed folder `../../frontend/dist/` — must exist at compile time (keep `.gitkeep`) |
| `Dockerfile` | Multi-stage: node → `rust:1.96-alpine` (musl static) → scratch; `--mount=type=cache` persists cargo registry/target |
| `docker-compose.yml` | `./data` bind mount (SQLite inspectable), port, PG-switch comment template |
| `.cargo/config.toml` | `TS_RS_EXPORT_DIR` for ts-rs generated contract output |
| `frontend/src/components/PaperBookView.tsx` | Core paper-book UI: continuous scroll, row-group grid (aligned ruled lines), tap-to-cover, lazy-load sentinel |
| `frontend/src/components/TagFilterPanel.tsx` | 标签筛选面板：组卡片（三态匹配 + chips）+ 组间且/或连接词；受控组件，onChange 传更新函数（函数式 setState） |
| `frontend/src/components/ImportModal.tsx` | 导入弹窗：上传-预览-确认三步，后端分页/筛选/校验，草稿防抖提交（fetchRows/reqSeq 竞态保护），重复组跳过集合 |
| `frontend/src/index.css` | Design tokens (`@theme`), keyframes, texture overlays |
| `config.example.yaml` | Runtime config template — copy to `config.yaml` (git-ignored): `server.host/port`, `database.url` (sqlite:// or postgres://), `import` 段（可选） |

## Runtime/Tooling Preferences

- **Git**: 提交必须经用户明确允许，每次允许只提交一次（一次性提交，不自动追加/重复提交）。
- **Rust**: stable-equivalent (local toolchain is nightly 1.94); edition 2021; workspace resolver 2, crates under `crates/`. Cargo add for deps; shared deps live in `[workspace.dependencies]`; axum must stay 0.8.x (`{id}` path syntax).
- **Node**: v25, npm 11. Vite 8 + Tailwind v4 via `@tailwindcss/vite` plugin (no tailwind.config.js; CSS-first `@theme`).
- **No linter/formatter gate** configured in repo (no eslint run in build); `tsc -b` runs as part of `npm run build`; Rust side gates on `cargo clippy -D warnings` + `cargo fmt --check`.
- UI language: Chinese. Backend error messages in Chinese.

## Testing & QA

- Unit tests live next to the logic they cover (`#[cfg(test)]` in order.rs / sort.rs / sort_dir.rs / service.rs / error.rs / import.rs): query-param parsing whitelists, validation rules, seeded shuffle determinism/permutation, error→status mapping, 导入行号/分组/标签解析（prepare_rows/group_rows/parse_tags）。
- ts-rs export tests (`export_bindings_*`) regenerate `frontend/src/generated/` on every `cargo test`; commit the generated files alongside DTO changes.
- No integration test suite yet. QA approach: `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo test --workspace` + `cargo build` + `npm run build` must pass; API smoke-tested via curl against a running server (create wordbook → add words → paginate → update → delete → verify cascade with `sqlite3 data/quan_word.db 'SELECT count(*) FROM word;'`; 导入冒烟：preview（含标签/重复/错误行的 6 列 csv，断言 total/valid/invalid/duplicate 统计与行号）→ import/rows（修正错误行后统计刷新、翻页/筛选）→ import（断言 imported/updated/skipped_*/created_tags）→ 查询确认标签关联与义项合并)；UI verified manually/browser-driven (cover toggle, continuous scroll + lazy-load sentinel (auto-fill short content, prefetch cache), long-word wrap & ruled-line row alignment at 2/3/4/5 columns, settings sliders, 导入预览分组卡片编辑/筛选/翻页)。
- When changing pagination: remember API pages are 1-based but `fetch_page` is 0-based. 导入分页按组切（组不跨页），`page_size` clamp 1..=100。
- When changing schema: add a migration (chronological `mYYYYMMDD_*` file, registered in `crates/migration/src/lib.rs`), and verify BOTH sqlite (default) and postgres (config.example.yaml URL + local `postgres:17` container, e.g. `-p 5433:5432` if a local PG occupies 5432).
- When changing DTOs: rerun `cargo test` to refresh ts-rs output, then `npm run build` to verify frontend type alignment; contract types in `frontend/src/generated/` are the source of truth. ts-rs 只生成不删除——DTO 删除后手动清理对应 `frontend/src/generated/*.ts` 孤儿文件。
