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
crates/lexicon        Definition / WordPos shared contract models (no DB dependency)
crates/dict-core      portable search core: plugin registry + recipe engine + SearchTransport
crates/dict-reqwest   reqwest transport (timeout / redirect host allowlist / body cap)
crates/entity         SeaORM 2 dense entities (Definition moved to lexicon)
crates/migration      sea-orm-migration schema migrations
crates/xtask          repo maintenance: validate example plugins
examples/             example plugin packages + fixtures + goldens (not loaded at runtime)
```

Data flow: router parses HTTP (State/Path/Query/Json) → service validates & queries via repo → returns `dto::resp` structs serialized as JSON. Errors bubble as domain errors → `ApiError` → `IntoResponse` (404/400/500 + `{"error": msg}`). Dictionary flow: `business/dict` → `SearchEngine` → recipe plugin → `SearchTransport` → upstream; parsing is data-driven, no per-provider Rust module.

Key backend decisions:
- `init::db::init_db` runs `PRAGMA foreign_keys = ON` **before** migrations for SQLite — without it, deleting a wordbook does not cascade-delete words.
- Word counting uses a single `GROUP BY` aggregate query (no N+1 per-book count).
- `definitions` stored as JSON column; `Definition` lives in `crates/lexicon` (shared by entity / dict-core / server), converted with `serde_json::to_value/from_value`.
- `timestamp_with_time_zone()` in migrations — `timestamp()` breaks PG (DateTimeUtc needs TIMESTAMPTZ); both work on SQLite.
- Shared contract types are exported to TypeScript via `ts-rs`: `#[ts(export, export_to = "...ts")]` generates a test; running `cargo test` writes the files to `frontend/src/generated/` (see `.cargo/config.toml` `TS_RS_EXPORT_DIR`).
- **批量导入（模板约定）**: 6 列表头 `单词,音标,词性,释义,例句,标签`；**每行一个义项**（一个词性对应一个释义），同一单词的多义项写多行（单词列重复），导入时按拼写（trim+小写）合并为一个单词——音标/例句取组内首个非空、标签取并集、义项按行序拼接；标签列多个标签用 `；`/`;` 分隔，缺失标签自动创建；全空行跳过；5 列旧模板缺列取空自动兼容。
- **词性白名单 21 项书写形式**（前后端一致，`crates/lexicon/src/word_pos.rs` 的 `WordPos` 枚举——strum 声明式映射，c/C、u/U、cu/CU 合并为同义变体——与前端下拉）：`n. / c / C / u / U / cu / CU / v. / vt. / vi. / adj. / adv. / prep. / conj. / pron. / num. / art. / interj. / aux. / abbr. / phr.`（C=可数、U=不可数名词；vt./vi.=及物/不及物动词；留空合法）。
- **导入三步会话（后端为主）**: ①`POST /words/import/preview`（multipart 上传，文件只传一次；格式按 `ImportFileType` 枚举解析，csv/xlsx/xls/ods）解析后把**全部解析行**（含全空行）缓存为 token 会话；②`POST /words/import/rows`（token + page/page_size/filter/updates）在会话内应用行级修正 → 重新校验 → 按**组切片**分页返回（组不跨页）；`filter` 为 `ImportFilter` 枚举（all/error/duplicate，serde `deserialize_with` 边界解析，非法值返回中文错误）；③`POST /words/import`（token + update_rows）一次性消费会话执行导入。前端仅显示与草稿编辑（防抖 500ms 提交），统计/错误标记/分页/筛选全部由后端计算。
- **导入容错**: 校验失败的行跳过并报告行号（`skipped_errors`），其余正常导入；同书重复拼写的单词默认更新（覆盖字段、标签合并 union），预览中可跳过（`skipped_duplicates`）；结果统计 `imported/updated/skipped_errors/skipped_duplicates/created_tags`。
- **导入会话缓存生命周期配置化**: `config.import` 段（`max_rows`/`cache_ttl_secs`/`cache_cap`/`cache_cleanup_secs`，缺省 5000/1800/16/60，加载时校验 >0）；`AppState::spawn_import_cache_cleaner` 后台定时清理过期会话（随 `router::build` 启动）；`cache_cap` 超出整体清空。
- **词典抓取是运行时插件（声明式配方）**: 每个源是一个 `data/plugins/<id>.json` = `{manifest, recipe}`；运行时只读 `dict.plugins_dir`（缺省 `./data/plugins`），仓库不内置任何源，8 个示例包在 `examples/plugins/`，仅测试/手动导入用。目录按文件名排序装载；同 id 两个包都拒绝，超过 `dict.max_plugins` 拒绝；空目录允许启动。`dict.providers: []` = 全部已安装；非空时按数组顺序启用/聚合。完整格式与调试见 `docs/dev/plugins.md`，校验用 `cargo run -p xtask -- validate-plugins`。
- **配方引擎边界**: `dict-core` 只依赖 serde/serde_json/scraper/url，不依赖 reqwest/tokio/文件系统/网络；触网只通过 `SearchTransport::execute(SearchRequest) -> SearchResponse`，因此 iOS/Android/WASI 宿主只需实现传输层。CI 的 `cross` job 对 `lexicon`/`dict-core` 做三目标 `cargo check` 门禁。
- **插件安全上限**: URL 仅 http/https；host 必须命中 manifest `hosts`（含重定向后的最终 host）；包体 ≤256 KiB、响应 ≤1 MiB、释义 ≤20、例句 ≤5、建议 ≤24、单条释义 ≤300 字符；插件 manifest `limits` 只能调低不能突破。解析失败/越界/能力与配方不一致只拒绝该文件；失败原因进启动日志，不再有面向 UI 的装载报告。
- **插件详情蓝图**: `GET /api/dict/plugins/{id}` 从注册表 + 配方派生只读 DTO（`business/dict/dto/plugin_detail/`：能力、hosts、lookup/suggest 请求、注册字段、释义策略、变量、插件上限 vs 引擎硬上限），不返回原始 JSON；未知 id 返回 404。`DictService::plugin_detail` 通过 `SearchEngine::entry` 克隆注册表项，注册表仍在 `ArcSwap` 内原子替换。
- **设置分区（插件管理 UI）**: `HomeTab` 是 `dict | books | settings`，首页一级导航三段「查词 / 单词书 / 设置」并列；`pages/SettingsPage.tsx` 是可扩展壳层（`SECTIONS` 注册表 + 桌面侧栏/移动横向导航 + 内容槽），当前分区「词典源 / 数据」；`components/DictSettings.tsx` 列表只有插件行、启停开关、导入按钮、删除按钮，不展示目录/统计/装载报告/刷新/重载，也没有“内置/外部”标记；点击行进入 `components/DictPluginDetail.tsx` 只读蓝图（能力、请求、注册字段、释义策略、变量、建议、上限环），不显示原始 JSON 或长段说明。导入走 `POST /api/dict/plugins`（multipart `file`），删除走 `DELETE /api/dict/plugins/{id}`，启停走 `PUT /api/dict/plugins/{id}/enabled`；目录默认 5 秒轮询，文件增删自动重载。
- **词典缓存/熔断按插件版本**: `DictKey = (provider_id, plugin_version, word)`，插件改解析逻辑必须升 `version`，否则旧缓存不会失效；导入/删除/轮询重载会串行清空词典缓存、建议缓存与熔断状态，进行中的请求继续持有旧注册表 `Arc`。前端设置页不提供手动清缓存，数据分区提供「查词缓存 / 搜索历史」清理。

## Key Directories

```
crates/lexicon/src/        Definition (definition.rs) + WordPos (word_pos.rs)，无 DB 依赖
crates/dict-core/src/       model / plugin_* / recipe/ / search_engine / transport（无 reqwest/tokio/fs/net）
crates/dict-reqwest/src/    ReqwestTransport（超时、重定向白名单、响应上限）
crates/entity/src/          SeaORM 2.0 dense entities (word.rs, wordbook.rs)
crates/migration/src/       Migrator + m20260802_* migration files (chronological)
crates/xtask/src/           validate-plugins（默认 examples/plugins）
examples/plugins/           8 个示例插件；examples/fixtures/ + examples/golden/；不进运行时
crates/server/src/
  business/wordbooks.rs + wordbooks/   /api/wordbooks... (router/service/repo/dto/error)
  business/wordbooks/words.rs + words/ /api/wordbooks/{book_id}/words... (router/service/repo/dto/error/import/order/sort/sort_dir/tag_group/tag_match/file_type/import_filter/template_format)
  business/dict.rs + dict/             /api/dict... (router/service/suggest/state/cache/breaker/plugin_store/dto)
  common/error.rs           ApiError aggregate (HTTP boundary)
  common/http/              Asset (rust-embed) + spa.rs static handler + json.rs/path.rs (ApiJson/ApiPath 提取器) + normalize.rs (错误归一中间件)
  common/model/page.rs      PageResp<T> pagination model
  common/model/paging.rs    parse_paging shared pagination query parsing
  common/state.rs           AppState (db + wordbooks_cache + shuffle_cache + import_cache + dict + config)
  config/                   config.rs root + server.rs / database.rs / import.rs / dict.rs
  init/                     run() entry + db.rs connection/migration
frontend/src/
  generated/                ts-rs generated TS contract types (source of truth, do not hand-edit)
  api.ts                    fetch wrapper; re-exports types from generated/
  dictSources.ts            来源下拉纯函数 buildSourceRows（插件列表 + 结果 → 行）
  dictCache.ts              词典查询会话缓存 / 单飞 / 源列表单例
  pages/ components/        UI
frontend/dist/              build output (embedded by rust-embed; .gitkeep placeholder)
data/plugins/               运行时插件目录（与 SQLite 同在 ./data；从设置页导入）
```

## Development Commands

```bash
# Backend (workspace root)
cargo build                    # dev build, whole workspace
cargo run -p server            # dev server on :3000 (config.yaml if present, else built-in defaults)
cargo build --release          # production binary (embeds frontend/dist at compile time)
cargo clippy --workspace --all-targets --all-features -- -D warnings   # validation gate
cargo test --workspace         # unit tests + ts-rs export tests (refreshes frontend/src/generated/)
cargo run -p xtask -- validate-plugins            # 校验 examples/plugins
cargo run -p xtask -- validate-plugins ./data/plugins  # 校验运行时目录

# Frontend
cd frontend && npm run dev     # Vite dev :5173, proxies /api → :3000
cd frontend && npm run build   # tsc -b + vite build → dist/ (MUST run before cargo build for fresh embed)
cd frontend && npm run test    # vitest run（jsdom + @testing-library/react）

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
- Handlers return `Result<impl IntoResponse, ApiError>`; `?` converts domain errors and `DbErr` via `From`. `ApiError` maps NotFound/BadRequest/Unauthorized/**TooManyRequests**/Db to 404/400/401/**429**/500 with user-facing Chinese messages; 500 responses never leak internals (Db details only go to the log).
- **Error response spec (enforced):** every error response is `{"error": 中文消息}` JSON. Request-body extraction MUST use `ApiJson<T>` (`common/http/json.rs`) and path params MUST use `ApiPath<T>` (`common/http/path.rs`) — axum 0.8 handler extraction failures short-circuit via `rejection.into_response()` and never call `From<Rejection>`, so raw `Json`/`Path` extractors produce English plain-text/422 responses; `ApiJson`/`ApiPath` have `Rejection = ApiError` (400, Chinese). `Query` DTO fields stay `String`-typed (no rejection possible). Framework-level plain-text responses (405 Method Not Allowed, uncaught 500) are wrapped into JSON by the `common/http/normalize.rs` middleware (mounted inside the compression layer in `router.rs`).
- Validation returns domain errors whose `thiserror` text is user-facing Chinese.
- SeaORM: `Entity::find().filter(Column::X.eq(v)).order_by_asc(...).paginate(db, size)`, then `fetch_page(page - 1)` (**0-based offset**) + `num_items()`; API pages are 1-based. `total_pages = total.div_ceil(page_size)`.
- SQLite: `init/db.rs` sets `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`, `cache_size=-20000` after connect.
- Static assets: `assets/` (Vite hashed) and `fonts/` (versioned woff2) served with `Cache-Control: public, max-age=31536000, immutable`; index.html `no-cache`. Whole router wrapped in tower-http `CompressionLayer` (gzip). Fonts are self-hosted in `frontend/public/fonts/` (no Google Fonts CDN).
- **词典建议接口按 IP 限流**：`/api/dict/suggest` 会扇出上游（第一个声明 `suggest` 的启用插件，配方 `take` 上限 24），随机词又能绕过按词缓存，所以 `business/dict/suggest_limiter.rs` 的 `SuggestRateLimiter`（`DictState.suggest_limiter`）按 IP 做固定窗口限流（60s / 120 次，超出 429 中文消息）；IP 来自 `ConnectInfo<SocketAddr>`，因此 `init.rs` 必须用 `app.into_make_service_with_connect_info::<SocketAddr>()` 提供服务，否则 handler 提取失败。**鉴权由全局 `require_api_key` 负责**：配置 `server.auth_key` 后所有 `/api`（含 suggest、插件导入/删除）都要带密钥；未配置时整个 `/api` 不启用鉴权，是部署特例，不是 suggest 单独无鉴权。
- Axum 0.8 path syntax: `/api/wordbooks/{id}` (curly braces); multiple path params use a tuple extractor `Path((book_id, id)): Path<(i32, i32)>` (one `Path` extractor per handler).

**Frontend patterns:**
- Components receive plain props; pages hold state; `api.ts` centralizes fetch and re-exports contract types from `frontend/src/generated/` (ts-rs output).
- Reader settings (page size, font px) persisted in `localStorage` under `qw_page_size` / `qw_font_scale`; loaded via lazy `useState(loader)`.
- 标签筛选（纸质/列表两模式共享，按书持久化）：WordbookDetail 持有 `tagFilter: TagFilter`（`{ groups: TagFilterGroup[]; links: ('and'|'or')[] }`，`qw_filter_${bookId}` 持久化；旧 `qw_filter_tags_*`/`qw_filter_match_*` 自动迁移后清除）；`links[i]` = 组 i 与组 i+1 之间的连接词，**「且」优先于「或」**（标准布尔优先级，后端按或切段、段内且组合）。筛选面板 `TagFilterPanel.tsx`（导航「标签」按钮下拉，`w-72 max-w-[calc(100vw-2rem)]`，手机/电脑同一交互）：每组卡片 = 组头（序号 + 删除按钮，仅组数>1 时）+ 独占一行的三态分段（全部匹配/任一匹配/无标签，避免窄面板换行）+ 全量标签 chips（每组选中独立；无标签组 chips 禁用）；**优先级分段视觉**：按「或」链接把组切成段（段内全「且」），段容器用边框 + 浅背景包裹（括号感），段间「或」为醒目大按钮（带左右分隔线），段内「且」为小胶囊按钮——点击任一连接词按钮即切换（段合并/切开实时变化），先后逻辑所见即所得；空筛选渲染空态（「未筛选 · 点击标签开始筛选」+ 全量标签 chips，点标签即建组选中）。「全部单词」清空（净化结果为空时才高亮）。**面板 onChange 传更新函数** `(prev: TagFilter) => TagFilter`（函数式 setState 按序应用，连续点选不互相覆盖）；清缓存重载第 1 页由 WordbookDetail 的 `tagFilter` 变化 effect 统一处理（首次挂载跳过，打乱状态保持）；发送前 `api.ts::sanitizeTagFilter`（已导出）剔除空组并重排 links（取后一组紧前连接，缺省且）；导航「标签」按钮 `hasFilter` 同用净化结果——「无标签」组视为有效筛选（激活 + 徽标，纯 none 时显示「标签(无标签)」），「全部单词」仅在真正无筛选时高亮。后端 `words.list`/`words.query` 接受 `tag_groups`（JSON：`{"groups":[{"mode":"and"|"or"|"none","ids":[...]}...],"links":["and"|"or"...]}`，links 长度必须 = 组数-1，无隐式缺省；none 组 ids 必须为空，均校验报中文错误），`repo.rs::with_tag_filter` 按组生成 `word_id IN/IN NOT (子查询)` 条件（And = GROUP BY + `HAVING COUNT(DISTINCT tag_id)=N` 交集子查询；Or = 并集子查询，word_tag 复合主键无重复行故无需 DISTINCT；None = `NOT IN (SELECT word_id FROM word_tag)` 无标签匹配），组间按「且优先于或」组合（`combine` 按或切段、段内且、段间或）。
- PaperBookView data flow (scroll mode): `WordbookDetail` holds `pages: Array<{ d: Page<Word>; pageNo: number }>` (loaded pages, ascending); page cache `Map<"${bookId}:${page}:${size}:${seed}:${JSON.stringify(tagFilter)}", Page<Word>>` in `useRef` + `fetchPageRaw` (cache hit + in-flight dedupe via `inflight` map) + `fetchPage` (seq guard for stale responses); `loadFrom(p)` rebuilds from page p with prefetch of next 2 pages, `loadMore(nextPageNo, totalPages)` appends on sentinel trigger; mutations clear cache and reload from `pages[0]`; page number persisted as first loaded page (`qw_page_${bookId}`).
- PaperBookView (scroll mode): continuous vertical scroll; all loaded pages' words merged into ONE grid (row-group slicing: `cols` word cells → `cols` ruled lines → `cols` definition cells per row-group, so lines align across the row; `cols` = 2/3/4/5 by Tailwind breakpoint, recomputed on resize); lazy loading via IntersectionObserver sentinel (800px early trigger, observer rebuilt on `pages.length`/`loading` change to keep filling viewport, 1.5s no-progress debounce against retry storms); long words/phonetics wrap (break-words + min-w-0), definitions render in full (no line-clamp); tap word/meaning toggles the cover (`Covered` = `.paper-veil` 静态雾字：`color: transparent` + 高斯 `text-shadow` + `opacity`，**禁止 filter/mask/常驻 will-change**——一屏几十个 blur 图层会把滚动压到 30fps；`aria-pressed` + persisted `wordDiff`/`defDiff` in `qw_cover_${bookId}`); tag chips are display-only, all tag add/remove happens in the TagQuickModal opened via the wrench button; tag chip row is a `flex flex-wrap` container (chips + wrench as flex items, chip `max-w-full truncate` + `title` for over-wide names) so narrow columns / many tags wrap to extra lines instead of being clipped, wrench rides the last wrapped line; chip font & wrench icon scale with fontScale (`FontStyles.chipFont`/`iconSize`).
- 主页查词结果分页：后端 `/api/dict/suggest` 一次返回 24 条（配方 `take: 24`，引擎硬上限 `max_suggestions = 24`），前端 `DictResultList` 首屏 6 条，**滚动到 sentinel 自动 +6（`IntersectionObserver`，`rootMargin: 200px`），没有「加载更多」按钮**；换词重置回首屏；卡片等高约 120px（`px-4 py-3` + 17px 标题 + 两行 13px 摘要 + chips），列表间距 `space-y-2.5`；stagger 延迟封顶 `Math.min(index, 6) * 55ms`。
- 主页壳层拆分（查词/单词书/设置三个一级分区）：`pages/WordbookList.tsx` 只做壳层（一级 tab、书架数据、详情开关、加入流程、弹窗、toast），`components/HomeNav.tsx` 是 fixed 胶囊导航（`view-transition-name: app-nav-home` 挂在它身上，类名 `.vt-nav-home`；外层 wrapper 不再带名字；`HomeTab` 三段、`.nav-seg-pill` 宽度 1/3），`components/DictHome.tsx` 是查词分区（搜索引擎式首页：居中 `LexiconMark` 图标字标 + 大搜索框 + 最近查询 chips；搜索时图标收缩、结果落到 48rem 中轴列，不再有独立空态面板），`components/Bookshelf.tsx` 是单词书分区（`shelf-grid` 用 `items-start`，新建卡固定 `h-56`，与封面同高，不被下方标题/操作按钮撑高），`components/DictSettings.tsx` 是设置分区（词典源插件管理）。首页根 `.vt-home`（`home-content`）、书页根 `.vt-book-page`（`book-page`）；**两页导航各自独立命名**（首页 `.vt-nav-home` → `app-nav-home`，书页 `.vt-nav-book` → `app-nav-book`，同一时刻文档里只允许一个：App 是 list/book 二选一渲染，首页壳层在书页显示时整棵卸载），避免离开的首页快照半透明盖在书页上（双重曝光）。
- **标题栏转场动画（不要合并回一个名字）**：两条导航内容完全不同，共用一个 `view-transition-name` 时 group 几何全程 identity、old/new 又是"立刻隐藏 / 立刻显示"→ 读起来就是标题栏凭空消失/凭空出现（用户报的"没有任何动画"就是这个）。现在各自退场/入场：`navBarOut`（0.2s 上浮淡出）+ `navBarIn`（0.46s 从顶边落位，延迟 0.08s，旧栏先走完）；group 只保留 UA 定位动画 + `z-index: 3`（**不能写 `animation: none`**，会把快照盒子尺寸一起清掉）。动画只在 `html.vt-nav-motion` 时生效——分区切换、纸质/列表切换这类同页变化里导航原地不动，给它做淡出淡入等于"切个 tab 整条栏闪一下"；**选择器必须写成复合形式** `html.vt-nav-motion::view-transition-new(...)`（后代选择器 `html.vt-nav-motion ::view-transition-new(...)` 不匹配：转场伪树挂在根元素上）。
- **入场动画在 settle 时必须「用掉」**（用户报的"加载完成后抽搐/动画重播"就是这个）：`html.vt-running` 只是把入场动画的 duration 压到 0.001s，转场结束摘掉这个类时，浏览器会按"元素上仍声明着这条动画"**重建一个 CSSAnimation**，currentTime 按真实经过时间算 → 入场动画在转场后又播一遍（书架卡片、列表项最明显；`animation.finish()` 救不了，重建的是新对象）。所以 `viewTransition.ts` 在 settle 里先 `consumeEntranceAnimations()`：把 `ENTRANCE_CLASSES`（必须与 index.css 的 `html.vt-running :is(...)` 名单逐项一致）从当前 DOM 上摘掉，元素停在自然终态；之后才挂载的内容照常播放入场动画。
- **环境背景是全局底图**：`<AmbientBackground/>` 挂在 `App` 壳层（不再放某个页面里），视图切换时**不卸载**，所以换页不会重播/重绘背景；它 portal 到 `<body>`、`position: fixed`、`z-index: 0`，页面内容在 `relative z-10` 包裹层里。**转场期间不要给 `.vt-home` 加整屏不透明底色**：底色画在"新首页快照"上，会把背景盖住 0.5s 再"啪"地冒出来（旧首页快照本来就由 `old(home-content){opacity:0}` 整片隐藏，不存在叠影）。全屏覆盖层（词典详情 z-50）必须用 `<AmbientBackground inline/>`：portal 到 body 的节点在根层叠上下文里只有 z-index 0，会被不透明覆盖层整片盖住；inline 渲染时它是覆盖层自己的子节点（fixed 且无 transform 祖先 → 仍钉在视口，不随详情内滚动移动）。
- 书页模式切换与空白页（`WordbookDetail` + `PaperBookView`）：模式胶囊用 `.book-mode-switch` + 绝对定位 `.book-mode-pill`（`translateX(100%)` 滑到列表），内容仍用 keyed `animate-fade-in-up`；纸质书专属按钮（显示设置/隐藏显示/打乱/更多）不再条件卸载，统一包 `.book-nav-reveal[data-open]`，关闭时轨道 0fr + opacity 0 + `visibility: hidden`，切模式时按钮宽度和透明度一起过渡。空书/筛选无结果用 `.paper-empty` 铺满整页（`min-height: clamp(28rem, 62vh, 44rem)` + 左侧装订线 + 右下 `Aa` 水印），不要再缩成 `max-w-md` 的居中块，否则宽屏上就是“倒三角”。
- 书页移动端导航（`WordbookDetail`）：两行胶囊，第一行 返回 / 标题（`flex-1 min-w-0 truncate`，手机不截成「测…」）/ 添加单词（手机只留 44 图标，`aria-label="添加单词"`）；第二行 模式切换 + 标签 + 「更多」溢出菜单（`MoreIcon`，内含 显示设置/隐藏单词/显示单词/隐藏释义/显示释义/打乱/重新打乱，按状态过滤）。**禁止横向隐藏滚动**（原来的 `overflow-x-auto` 会把 8 个控件藏到屏外）；390px 下导航高度 ≈142（原 194），控件全部在视口内且 ≥44。`main` 的内容起点用 `calc(env(safe-area-inset-top,0px) + N)` 计算，刘海屏不与导航重叠。
- 词典详情/加入流程：`components/DictDetailPage.tsx` portal 全屏 `role="dialog" aria-modal="true"`（进入把焦点移到返回键、背景 `#root` 加 `inert`、Tab 焦点陷阱、Esc 逐级返回、关闭后焦点还给触发卡片）；内部三步骤 `detail → books → definitions`，只有一本书时跳过选书直接进释义，浏览器后退与 Esc 同一套层级（definitions→books→detail→关闭，由壳层 `onRegisterBack` 注入，popstate 消费后补回历史条目）；释义按词性分组、每组默认 3 条 + 「展开全部 N 条」，编号按当前展示顺序连续；**底栏不要挂 `view-transition-name`**（曾经是 `detail-bar`：转场会把它从页面快照里单独摘出来、以 `translateY(100%)` 起步，进详情的一瞬间整条底栏不在画面里，随后才「突然出现」）；「创建单词书」分支记住当前词（`pendingWord`），建完书自动回到该词详情继续加入。`DictBookStep`（选书，含「新建单词书」行）/ `DictDefinitionStep`（勾选义项 + 保存态）为两个子步骤。
- 转场统一入口 `frontend/src/viewTransition.ts`：`withViewTransition(change, { navMotion })` 串行化并返回 Promise（`navMotion: true` = 整页切换，给 `<html>` 挂 `vt-nav-motion` 让标题栏走退场/落位；同页变化不传）；过渡进行中的调用**入队 + 跳过当前过渡**（`pending: Array<() => void>` 累积变更，同时 `active.skipTransition()` 让 `finished` reject → settle 立刻用队列里全部变更开一条新过渡：既不等待旧过渡跑完，也不丢中间变更；不用 `data-vt-entered` 长效属性——入场抑制只在 `vt-running` 期间由 CSS 完成），start 前给 `<html>` 加 `vt-running`，`transition.finished.then(settle, settle)` 同时处理 reject（AbortError/skip），settle 时**先 `consumeEntranceAnimations()` 再摘 `vt-running`**（见下条）；入场动画的快进由 motion 的 `html.vt-running` 规则负责（不要用 `animate-none` 销毁动画：类名摘掉后会从 0 重建 = 二次入场）；`wordTransitionName()` 生成 `dict-word-<slug>`，`setTransitionWord()/transitionNameFor()` 让**只有被点击的那个词**挂 `view-transition-name`（列表里其它词不挂，避免几十个命名元素拖垮转场）；`App.tsx` 的 navigate/popstate、`WordbookList` 的详情开关走它，禁止再裸调 `document.startViewTransition`（会互相 skip）。**`WordbookDetail` 的纸质/列表切换不要走 VT**：转场期间 `html.vt-running` 会把 keyed 内容的 `animate-fade-in-up` 快进到 0.001s，看起来就是“没有动画”；改为直接 `setMode`，由 `book-page.css` 的胶囊滑块 + `.book-nav-reveal`（`grid-template-columns: 0fr ↔ 1fr` + opacity）负责按钮收放。视图未变时 App 不开过渡，详情关闭动画由 WordbookList 自己跑。打开详情时 `setTransitionWord(word)` 必须在 `openDetail` 内部、紧跟一次 `flushSync` 强制渲染（模块变量不触发渲染；只在 pointerdown/focus 里登记会导致旧快照里没有被点词的名字，共享词飞行不出现）。
- 滚动锁 `frontend/src/scrollLock.ts`：引用计数式 `lockScroll()`（返回释放函数），`Modal.tsx` 与 `DictDetailPage` 共用；详情是 VT 内异步卸载，若各自保存/恢复 `body.overflow` 会把 body 永久锁在 hidden（详情里开创建词书弹窗必现），所以必须共用一个锁。
- **消失/取消也要有动画**（用户报的"退出动画完全没有"）：条件渲染（`{open && <X/>}`）关闭时是瞬间卸载，节点直接从 DOM 里消失，任何 transition 都来不及跑。统一用 `hooks/usePresence.ts`：`const p = usePresence(open, 220)` → `{p.mounted && <X data-state={p.entering ? 'entering' : p.closing ? 'closing' : 'open'} />}`；`entering` 只在"从关到开"的头两帧为 true（先以收起态画一帧，翻成展开态 transition 才有起点），`closing` 让节点多活 durationMs 播退场。样式侧：`.ui-pop[data-state='closing']`（淡出 + 下沉 6px，180ms）、`.collapse`（`grid-template-rows: 0fr ↔ 1fr`，高度未知也能平滑展开/收起）、`.wb-rail[data-state='entering'|'closing']`（桌面收列宽 + 右移淡出，负 margin 吃掉父级 gap；窄屏整张滑下）。已接入：词典栏、设置/筛选/更多菜单、来源下拉、拼写建议、toast（2.4s 停留 + 0.2s 退场）、词典例句折叠；弹窗另有 `.modal-exit*`（见上条）。新增浮层一律照这个模式，禁止裸条件渲染。
- 轻提示 `frontend/src/toast.ts`（模块级 `toast(msg)` + `subscribeToast`）与 `components/Toast.tsx` 的 `<ToastHost />`：**createPortal 到 document.body**、`z-[80]`（高于详情/弹窗的 z-50，否则被壳层的 `relative z-10` 夹住看不见）、`bottom: calc(env(safe-area-inset-bottom) + 96px)`（不压详情/表单底栏的 CTA），`role="status" aria-live="polite"`，2.6s 自动消失；保存/加入后先 `await closeDetail()` 再 toast（转场期间快照覆盖整屏，提前 toast 会被吞掉）；加入词书、创建/删除词书、清空最近查询、展开全部、朗读失败、打乱/隐藏显示、词典回填（填入全部/仅音标/追加释义/追加例句）等有副作用按钮都必须给 toast 或 inline 状态，不能静默。
- 弹窗语义与焦点 `components/Modal.tsx`：卡片 `role="dialog" aria-modal="true" tabIndex={-1}`，`aria-labelledby` 自动指向卡片内第一个 `h2/h3`（无标题的弹窗退化为无标签 dialog）；挂载时 `#root` 加 `inert`（**引用计数**，嵌套弹窗不互相摘）、焦点进卡片（优先第一个 `input/textarea/select`，否则第一个可聚焦控件）、Tab/Shift+Tab 在卡片内循环（焦点陷阱）、卸载时把焦点还给触发元素。验收：连按 40 次 Tab 不应有一次落到背景。
- 弹窗关闭动画 `components/Modal.tsx`：卸载时克隆 overlay/card，把 `animate-fade-in`/`animate-pop-in` 换成 motion 的 `.modal-exit-overlay`(180ms)/`.modal-exit`(220ms panelOut)，`animationend` 后移除（320ms 兜底），`prefers-reduced-motion` 直接卸载；节点在 `useEffect` 挂载时抓取（卸载阶段 ref 已清空、DOM 可能已移除，不能靠 `isConnected` 判断）。所有用 Modal 的弹窗（词书表单/确认框/导入/标签/单词表单）共用这套退出动画。
- 设置分区 `pages/SettingsPage.tsx` + `components/DictSettings.tsx`：与查词/单词书并列的一级页面（`HomeTab = 'dict' | 'books' | 'settings'`，`HomeNav` 三段胶囊；`.nav-seg-pill` 宽度按 1/3 计算，新增 tab 必须同步改 CSS）；`SettingsPage` 是壳层，`SECTIONS` 注册表决定侧栏/移动导航，新增设置只加一个 section 并渲染内容组件；当前「词典源 / 数据」。词典源列表行本身是整行按钮（pin + label + 能力图标 + 版本 + chevron），开关/删除是兄弟按钮且不冒泡；点击行进入 `components/DictPluginDetail.tsx` 只读蓝图——顶部 sigil/状态/能力徽章、请求→变量→释义→输出四段流程、注册字段 tile、释义策略 chip、请求卡（method/path/host/header/form）、建议卡、变量 chip、三枚上限环；只用图标/计数/短标签，禁止段落堆砌或原始 JSON。导入走 `dict.importPlugin(file)`，删除走 `dict.deletePlugin(id)`，启停走 `dict.setPluginEnabled`。设置页动效集中在 `frontend/src/styles/settings.css`：左下角线稿仪表盘（刻度/三环/雷达扫线/双行星/脉冲核心）、标题逐字落墨 + 浮点、分区导航弹入 + 滑线、内容面板 3D 翻页 + 竖向扫描线、插件行错峰入场 + 状态圆点呼吸 + hover 沙色扫过、开关轨道脉冲、删除抖动、导入浮动；详情页另有 sigil 双环旋转/轨道点、流程节点错峰与脉冲、字段/策略/上限环错峰描边；空目录时显示居中的导入轨道（双环/扫线/浮起上传图标），导入行居中；`prefers-reduced-motion` 全部关闭。开关按钮本体 44×44，视觉轨道 44×24。
- 网络词典（表单右侧常驻栏）`components/DictSearchPanel.tsx`：**不再覆盖/弹窗**——桌面是表单右侧的白纸卡，`.wb-sheet`（纸卡）/ `.wb-rail`（词典卡）是同一 flex 父级 `.wb-body` 的兄弟节点（桌面 `row + gap:20` 靠 stretch 天然等高，两卡 top/bottom 严格一致）；**<768 不再分屏**（`.wb-handle` 把手与「分屏/仅表单」开关已删除，禁止加回）：词典是覆盖式底部面板（`.wb-scrim` 遮罩 z-40 + `.wb-rail` fixed z-41 / 86dvh / 只留上圆角，`wbSheetUp` 升起），点遮罩或「收起」关闭。**搜索框是实时模糊搜索**（`hooks/useDictSuggest.ts`，400ms 防抖、≥2 字符）：下方白卡浮层列候选（词 + 简短释义，44 行高，`role=combobox/listbox` + `aria-activedescendant`），点一行 = 立刻查该词；↑↓ 移动、Enter 选中、Esc 只收候选（`preventDefault+stopPropagation`，不冒泡到 Modal）。手机（<768）词典头部压到 ≤96：词头 22px、音标与「加入音标」同行内联（按钮视觉/命中仍 44）、来源徽章行隐藏（来源由上方下拉承担）；义项行 68 → 56（`padding-block 8` + 15/1.5 居中），桌面仍 28px 头部 + 68 行高。**来源是下拉不是胶囊墙**：44 高触发器（`来源` 小标 + 当前源名 + chevron）打开白卡 `role=listbox` 面板（行数 = 配置启用的插件数，来自 `dictSources.ts::buildSourceRows`；每行 44，当前项 clay 对勾，无结果的源 `aria-disabled` 并标「无结果」，超出按白卡下沿裁高并内部滚动；键盘 ↑↓/Home/End/Enter/Esc + roving tabindex，Esc 把焦点还给触发器）。义项逐条勾选：行高 68（桌面，`padding-block 12` + 行尾 44 圆形描边 `+`；手机 56）、正文 15px/1.6、行间 `divide-y divide-charcoal/5`、hover `bg-sand/20`；例句默认 1 条（原文 + 译文），其余折成 44 高「另有 N 条例句」。底栏 = `已选 N 条`（`text-xs charcoal/40`）+ 主按钮「全部加入」（胶囊 + `shadow-lg shadow-charcoal/10`）。回填动作分层仍在表单侧：字段旁内联状态（sand 徽章 + 「撤销」44 命中区），`added === 0` 时不渲染撤销。
- 词典回填计数必须真实：`hooks/useDictFill.ts` 的 `applyFill` 返回 `{ added, skipped }`（added = 真正写入条数，去重后可能为 0），面板 toast 与字段旁内联状态都按它说话——`added === 0` 时写「没有新增（这些义项都已存在）」且**不渲染撤销**（没有可撤销的改动）；只有真实 added 才显示 `已追加 N 条释义`。单词字段单独处理：表单拼写为空时回填词条 word（否则保存必报「单词不能为空」），已有拼写不覆盖、只在字段旁给「词典为『x』/ 改用」。
- 撤销按字段：`WordFormModal` 的 `notes` 是 `{ word, phonetic, definitions, example }` 四份独立状态，每份的 `action` 只回滚自己那个字段（曾经三个字段共用一个 restoreAll，点音标旁的撤销会把释义/例句一起清空）。
- 布局稳定（防 CLS）：异步内容必须预留与最终态同高的占位——词书归属 chips 行常驻 `min-h-[26px]`（最近查询 `min-h-[22px]`）、结果卡骨架/空态/错误态统一 `h-[120px]`（与真实卡实测同高：17px 标题 + 两行 13px 摘要 `min-h-[39px]` + chips 行 `min-h-[22px]`；卡片 `px-4 py-3`）、详情骨架 `min-h-[380px]`、词典面板骨架 `min-h-[200px]`、空态/错误态与结果卡同高；`useWordPresence` 请求失败不写缓存（写空会让「已收录」状态 5 分钟内失真）。
- Styling: **Visual Organic** design system from `docs/dev/style.md` — palette `ivory #F8F4EF` / `charcoal #2F2A25` / `clay #C58F6D` / `sage #C9D5C6` / `sand #E5D8C8` (defined in `index.css` `@theme`), fonts DM Serif Display (headings, `font-serif`) + Plus Jakarta Sans (body). **No emoji, no gradients** in UI (user-mandated); SVG icons in `components/Icons.tsx` (stroke-based). 专用样式各占一个文件：`frontend/src/styles/bench.css`（单词编辑页网格与滚动条）、`rail.css`（词典栏菜单/勾选/遮挡条）、`tag-page.css`（标签页铺满与列表滚动）、`settings.css`（设置页轨道/落墨/翻页/行错峰/开关脉冲）、`dict-home.css`（搜索引擎式查词首页/结果卡）、`shelf.css`（书架标题/新建空白书）、`book-page.css`（空白纸页/模式切换胶囊/导航控件收放），由 `index.css` `@import` 引入——**一个界面一个文件，禁止跨文件互相覆盖**。单词编辑与管理标签都是 page 形态 Modal（`bg-ivory` + `AmbientBackground` + 居中纸卡 `bg-[#FDFAF4] rounded-2xl shadow-sm border-charcoal/5`，下层 `bg-[#F2EAE0] translate-x-2 translate-y-2` 垫纸，四周 16–24px 让底图透出，纸卡铺满视口、列表区自身滚动、操作条钉在卡内底部）；字段 `bg-white border-charcoal/15 rounded-xl focus:border-clay`，主按钮 `bg-charcoal text-ivory rounded-full shadow-lg shadow-charcoal/10`，次按钮 `border-charcoal/20 rounded-full`，徽章 `bg-sand/30 border-sand rounded-full`；三处滚动容器统一 6px 圆角拇指 + 全透明轨道 + 隐藏箭头（`scrollbar-width: thin` + `::-webkit-scrollbar` 全套），不许露 UA 灰条。

## Important Files

| File | Why |
|---|---|
| `crates/server/src/init/db.rs` | Connection + PRAGMA foreign_keys + migration order (failure here breaks cascades) |
| `crates/server/src/router.rs` | Top-level route aggregation; add new top-level domains here (domain routes live in each domain's router.rs) |
| `crates/server/src/business/wordbooks/words/service.rs` | Pagination, validation, seeded shuffle, JSON conversion, 导入预览/执行/分页重校验（import_preview/page_rows/import_words） |
|`crates/server/src/business/wordbooks/words/import.rs`|导入解析：行式义项解析（行号=物理行号，csv 空行不丢位）、行级校验（prepare_rows）、按拼写分组（group_rows）、标签解析（parse_tags，20 字符上限）；文件格式匹配走 `file_type.rs` 的 `ImportFileType` 枚举（csv/xlsx/xls/ods）|
|`crates/server/src/business/wordbooks/words/order.rs`|WordOrder enum + query param parsing（词性白名单在 `crates/lexicon/src/word_pos.rs` 的 `WordPos` 枚举；导入行筛选枚举在 `import_filter.rs` 的 `ImportFilter`；模板格式枚举在 `template_format.rs` 的 `TemplateFormat`）|
|`crates/server/src/business/wordbooks/words/tag_group.rs`|标签筛选组模型：`TagGroup`（mode + ids）+ `TagGroupsParam`（groups + links，JSON 边界反序列化，链接长度 = 组数-1）|
|`crates/server/src/business/wordbooks/words/tag_match.rs`|`TagMatch` 枚举（and/or/none）+ `deserialize_tag_match`/`deserialize_links`（serde deserialize_with，中文错误）|
| `crates/server/src/config/import.rs` | 导入配置：max_rows / cache_ttl_secs / cache_cap / cache_cleanup_secs（缺省 5000/1800/16/60，serde default + Default） |
| `crates/server/src/config/dict.rs` | 词典配置：enabled / providers / plugins_dir / plugins_poll_secs / max_plugins / timeout / 缓存 / 熔断 / max_word_len；启动校验中文错误 |
| `crates/server/src/common/http/asset.rs` | rust-embed folder `../../frontend/dist/` — must exist at compile time (keep `.gitkeep`) |
| `Dockerfile` | Multi-stage: node → `rust:1.96-alpine` (musl static) → scratch; `--mount=type=cache` persists cargo registry/target |
| `docker-compose.yml` | `./data` bind mount (SQLite inspectable), port, PG-switch comment template |
| `.cargo/config.toml` | `TS_RS_EXPORT_DIR` for ts-rs generated contract output |
| `frontend/src/components/PaperBookView.tsx` | Core paper-book UI: continuous scroll, row-group grid (aligned ruled lines), tap-to-cover, lazy-load sentinel |
| `frontend/src/components/TagFilterPanel.tsx` | 标签筛选面板：平铺组卡片（三态匹配 + chips）+ 优先级分段视觉（或=段边界大按钮+段容器，且=段内小按钮）+ 空态（未筛选提示 + 点标签建组）；受控组件，onChange 传更新函数（函数式 setState） |
| `frontend/src/components/ImportModal.tsx` | 导入弹窗：上传-预览-确认三步，后端分页/筛选/校验，草稿防抖提交（fetchRows/reqSeq 竞态保护），重复组跳过集合 |
| `frontend/src/components/DictDetailPage.tsx` | 词典详情（portal dialog + 焦点陷阱）+ 三步加入流程（detail/books/definitions）+ 单本跳过选书 + 浏览器后退逐级回退 + 释义分组/连续编号 |
| `frontend/src/dictSources.ts` | 来源下拉纯函数 `buildSourceRows(providers, entries)`：只列启用插件、按注册表顺序、结果里多出的源追加；配套 `dictSources.test.ts` |
| `frontend/src/pages/SettingsPage.tsx` + `components/DictSettings.tsx` | 设置壳层（分区注册表/侧栏/内容槽）+ 词典源列表/启停/导入/删除；行点击进入详情；配套 `SettingsPage.test.tsx` / `DictSettings.test.tsx` |
| `frontend/src/components/DictPluginDetail.tsx` | 插件详情可视化蓝图：能力徽章、请求→变量→释义→输出流程、注册字段 tile、释义策略 chip、请求/建议/变量卡、上限环；只用图标/计数/短标签 |
| `frontend/src/components/LexiconMark.tsx` | 查词首页图标字标：线稿摊开词典 + 陶土放大镜（镜片 `Aa`），替代“查词”文字大标题；`prefers-reduced-motion` 停止虚线环旋转 |
| `frontend/src/styles/dict-home.css` | 搜索引擎式查词首页：居中图标/大搜索框/最近查询 chips、搜索态收缩、48rem 结果列与结果卡；空结果不写前端建议缓存（见 `useDictSuggest`） |
| `frontend/src/styles/book-page.css` | 书页空白纸页 `.paper-empty`、模式胶囊 `.book-mode-pill`、导航按钮收放 `.book-nav-reveal`；切模式不走 VT 的配套动画 |
| `frontend/src/styles/settings.css` | 设置页动效：线稿仪表盘、标题落墨、面板翻页、行错峰/呼吸圆点、开关脉冲、删除抖动；详情页 sigil 旋转/流程脉冲/字段错峰/上限环描边；`prefers-reduced-motion` 关闭 |
| `crates/dict-core/src/search_engine.rs` | 平台无关门面：`new(transport, limits)` / `load(sources)` / `lookup` / `suggest`；注册表 `ArcSwap` 原子替换 |
| `crates/dict-core/src/plugin_registry.rs` | 只装载外部源：按文件名排序 + 同 id 重复拒绝 + 数量上限 + 装载报告 |
| `crates/dict-core/src/recipe/execute.rs` | 配方运行时：JSON/HTML 提取、变量、变换、条件、释义策略、例句/建议映射 |
| `crates/dict-core/tests/examples.rs` | 8 个示例配方对 fixtures 的 golden 回归（与旧 Rust 解析器逐字段一致）+ 兜底分支/错误映射/建议去重 |
| `crates/xtask/src/main.rs` | `validate-plugins` 维护命令（默认 `examples/plugins`） |
| `docs/dev/plugins.md` | 插件包格式、配方字段、安全上限、装载/导入/删除、移动端嵌入完整说明 |
| `frontend/src/viewTransition.ts` | 转场唯一入口 `withViewTransition`（串行化、pending 队列合并、vt-running、finished reject 兜底）+ 共享词命名 `wordTransitionName/setTransitionWord/transitionNameFor` |
| `frontend/src/scrollLock.ts` | 引用计数 body 滚动锁，Modal 与 DictDetailPage 共用（避免异步卸载把 body 永久锁在 hidden） |
| `frontend/src/components/Toast.tsx` | 全局轻提示 `<ToastHost/>`（portal 到 body、z-[80]、避开底栏 + 安全区；role=status/aria-live），模块级 `toast()` 在 `frontend/src/toast.ts` |
| `crates/server/src/business/dict/suggest_limiter.rs` | 建议接口按 IP 固定窗口限流（60s / 120 次，超出 429）；`DictState.suggest_limiter`，依赖 `ConnectInfo` |
| `frontend/src/hooks/usePresence.ts` | 退场/入场状态机（`mounted`/`closing`/`entering`），所有浮层显隐都走它 |
| `frontend/src/index.css` | Design tokens (`@theme`), keyframes, texture overlays |
| `config.example.yaml` | Runtime config template — copy to `config.yaml` (git-ignored): `server.host/port`, `database.url` (sqlite:// or postgres://), `import` 段（可选）, `dict` 段（providers / plugins_dir / plugins_poll_secs / max_plugins / 超时/缓存/熔断/长度） |

## Runtime/Tooling Preferences

- **Git**: 提交必须经用户明确允许，每次允许只提交一次（一次性提交，不自动追加/重复提交）。
- **Rust**: workspace `rust-version = "1.96"`（Docker 构建用 `rust:1.96-alpine`；本机当前 rustc 1.99.1）；edition 2021; workspace resolver 2, crates under `crates/`. CI 使用 stable + 三目标交叉 `cargo check`. Cargo add for deps; shared deps live in `[workspace.dependencies]`; axum must stay 0.8.x (`{id}` path syntax).
- **Node**: v25, npm 11. Vite 8 + Tailwind v4 via `@tailwindcss/vite` plugin (no tailwind.config.js; CSS-first `@theme`).
- **前端测试**：vitest 5 + jsdom + @testing-library/react（devDependencies）；`npm run test` = `vitest run`，`npm run test:watch` 监听。测试文件与被测代码同目录（`*.test.tsx`）。
- **No linter/formatter gate** configured in repo (no eslint run in build); `tsc -b` runs as part of `npm run build`; Rust side gates on `cargo clippy -D warnings` + `cargo fmt --check`.
- UI language: Chinese. Backend error messages in Chinese.

## Testing & QA

- Unit tests live next to the logic they cover (`#[cfg(test)]`): wordbook query-param parsing whitelists, validation rules, seeded shuffle determinism/permutation, error→status mapping（含 429）, 导入行号/分组/标签解析（prepare_rows/group_rows/parse_tags）, 建议限流窗口（配额内放行、超配额拦截、窗口滚动后重置）。
- 词典侧测试：`dict-core` 覆盖 ProviderId 校验、JSON 路径解析、host 白名单、插件包校验（未知字段/越界 host/wasm/缺配方）、注册表重复拒绝/数量上限/文件名排序；`crates/dict-core/tests/examples.rs` 用 8 个示例包 + fixtures 对旧解析器 golden 做逐字段回归，并覆盖兜底分支、错误映射与建议去重；`server` 的 `business/dict/service.rs` 用 `MockTransport` + 合成插件覆盖校验、缓存/负缓存、refresh、单飞、熔断、单源失败隔离。
- **前端单测**（`frontend/vitest.config.ts`，jsdom + @testing-library/react，`cd frontend && npm run test`）：`hooks/usePresence.test.tsx`（初始不挂载 / entering 两帧 / closing 到点卸载 / 退场中重开取消卸载）、`components/DictResultList.test.tsx`（首屏 6 条、sentinel 自动加载 6→12→18→24、换词重置、少于 6 条无 sentinel、点击回调、空态/回显态/错误态/骨架）、`hooks/useDictSuggest.test.tsx`（空结果不写缓存：导入插件后同一个词能重新搜出来）、`dictSources.test.ts`（启用插件顺序 / 隐藏禁用源 / 结果可用标记 / 未列出插件追加 / short_label 兜底）、`components/HomeNav.test.tsx`（三段 tab 渲染 / 点击切设置 / 设置页隐藏新建 CTA）、`pages/SettingsPage.test.tsx`（分区导航/默认词典源/`aria-current`）、`components/DictSettings.test.tsx`（列表/启停/导入/删除、点击行进入详情并返回、无内置标记与报告）。新增交互状态机、分页、来源派生或设置分区逻辑时同步补测试。
- ts-rs export tests (`export_bindings_*`) regenerate `frontend/src/generated/` on every `cargo test`; commit the generated files alongside DTO changes.
- No HTTP integration test suite yet. QA approach: `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo test --workspace` + `cargo build` + `npm run build` + `cargo run -p xtask -- validate-plugins` must pass; CI 另跑 `cargo fmt --check`、`git diff --exit-code -- frontend/src/generated` 与三目标交叉 `cargo check`。词典插件冒烟：清空 `data/plugins/` 启动后 `GET /api/dict/providers` 返回 `[]`；从设置页导入 `examples/plugins/*.json` 后列表立即出现；切换开关后 `data/plugins/.state.json` 持久化、重启保持；删除后文件消失且列表更新；直接往 `data/plugins/` 放文件在轮询周期内自动出现；配置 `server.auth_key` 后无密钥请求返回 401（含 suggest 与导入/删除）。API smoke-tested via curl against a running server (create wordbook → add words → paginate → update → delete → verify cascade with `sqlite3 data/quan_word.db 'SELECT count(*) FROM word;'`; 导入冒烟：preview（含标签/重复/错误行的 6 列 csv，断言 total/valid/invalid/duplicate 统计与行号）→ import/rows（修正错误行后统计刷新、翻页/筛选）→ import（断言 imported/updated/skipped_*/created_tags）→ 查询确认标签关联与义项合并)；UI verified manually/browser-driven (cover toggle, continuous scroll + lazy-load sentinel (auto-fill short content, prefetch cache), long-word wrap & ruled-line row alignment at 2/3/4/5 columns, settings sliders, 导入预览分组卡片编辑/筛选/翻页)。查词/详情/加入流程的验收：真实点击（无 force/JS click）走「搜索→结果→详情→加入→选义项→保存→toast→presence 刷新」，检查控制台 0 条 `Transition was skipped/aborted`、转场期间 `html.vt-running` 生效（入场动画被快进到终态）、连续帧无闪白/无二次入场、任何弹窗/详情关闭后 `document.body.style.overflow` 为空且列表可滚动。触控验收：逐屏用脚本审计所有可交互元素的 `getBoundingClientRect()`，宽高都要 ≥44（勾选框等小控件用 `label` 撑命中区、纸质书单元格用 `::before` 扩命中区而不改变版面），390×844 下任何控件不得超出视口（禁止用横向隐藏滚动收纳）。
- When changing pagination: remember API pages are 1-based but `fetch_page` is 0-based. 导入分页按组切（组不跨页），`page_size` clamp 1..=100。
- When changing schema: add a migration (chronological `mYYYYMMDD_*` file, registered in `crates/migration/src/lib.rs`), and verify BOTH sqlite (default) and postgres (config.example.yaml URL + local `postgres:17` container, e.g. `-p 5433:5432` if a local PG occupies 5432).
- When changing DTOs: rerun `cargo test` to refresh ts-rs output, then `npm run build` to verify frontend type alignment; contract types in `frontend/src/generated/` are the source of truth. ts-rs 只生成不删除——DTO 删除后手动清理对应 `frontend/src/generated/*.ts` 孤儿文件。
