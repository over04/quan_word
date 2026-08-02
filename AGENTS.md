# Repository Guidelines

## Project Overview

**单词本** — a paper-notebook-style vocabulary learning web app. Users organize words into independent "wordbooks" (单词书), review them in a simulated paper-book view (ruled lines, tap-to-cover self-testing), and manage them in a list view. Backend: Rust (axum 0.8 + SeaORM 2.0 + sea-orm-migration). Frontend: React 19 + Vite 8 + TailwindCSS v4. Single-binary deployment (rust-embed embeds `frontend/dist`). Database: SQLite by default, PostgreSQL switchable via YAML config.

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

## Key Directories

```
crates/entity/src/          SeaORM 2.0 dense entities (word.rs, wordbook.rs) + definition.rs (JSON column model)
crates/migration/src/       Migrator + m20260802_* migration files (chronological)
crates/server/src/
  business/wordbooks.rs + wordbooks/   /api/wordbooks... (router/service/repo/dto/error)
  business/wordbooks/words.rs + words/ /api/wordbooks/{book_id}/words... (router/service/repo/dto/error/order/sort/sort_dir)
  common/error.rs           ApiError aggregate (HTTP boundary)
  common/http/              Asset (rust-embed) + spa.rs static handler
  common/model/page.rs      PageResp<T> pagination model
  common/model/paging.rs    parse_paging shared pagination query parsing
  common/state.rs           AppState (db + wordbooks_cache + shuffle_cache)
  config/                   config.rs root + server.rs / database.rs
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
```

Verification: clippy gate + `cargo test`; API smoke-tested via curl against a running server (see Testing & QA).

## Code Conventions & Common Patterns

**Strict project rules (user-mandated, enforce on all new code):**
- **One struct/trait/enum per `.rs` file** (entity/migration follow SeaORM conventions; same-action Req/Resp DTOs may share a file).
- **No `pub use` re-export barrels.** Reference full module paths. `foo.rs` declares child modules; never `mod.rs`.
- **Layered business domains (URL-aligned):** `business/` mirrors the API path structure — `wordbooks/` owns `/api/wordbooks...`, nested `wordbooks/words/` owns `/api/wordbooks/{book_id}/words...` (no top-level `/api/words`). Each `business/<域>/` owns its `router.rs` (HTTP only, registers its own routes), `service.rs` (business rules), `repo.rs` (SeaORM persistence), `dto/` (boundary models), `error.rs` (thiserror domain errors). Each layer's `router.rs` merges its subdomains recursively; the top-level `router.rs` only aggregates top-level domains. Controllers are thin; validation and queries live in services.
- **DTO files:** named by action without suffixes — `dto/create.rs`, `dto/update.rs`, `dto/resp.rs`; type names stay fully semantic (`CreateWordbookReq`, `UpdateWordbookReq`, `WordbookResp`). HTTP query params are typed too: `dto/list.rs` (`ListWordsQuery`) / `dto/search.rs` (`SearchWordsQuery`); shared pagination parsing lives in `common/model/paging.rs` (`parse_paging`).
- **Errors:** each business domain defines its own `thiserror` enum with semantic variants (no vague `BadRequest(String)` in services); `common::error::ApiError` is the HTTP aggregate, mapped via `From` in each domain's `error.rs`.
- **Closed sets are enums:** query params (`order`, `seed`, `sort`) are parsed once at the router boundary into `WordOrder` / `SortField` / `SortDir`; business logic matches enum variants, never string literals. `SortField`/`SortDir` use `strum::EnumString` (declarative mapping); `WordOrder` keeps a handwritten parse because `Random(String)` carries a payload strum cannot express.
- **ts-rs contract:** frontend-facing DTOs derive `TS` with `#[ts(export, export_to = "...ts")]` (+ `rename` where the TS name differs, `#[ts(type = "number")]` for u64, `#[ts(optional)]` for omitted request fields). Regenerate with `cargo test` after DTO changes; never hand-edit `frontend/src/generated/`.

**Backend patterns:**
- Services are unit structs (`pub struct WordbookService;`) with associated async fns taking `&AppState` as first arg (access DB via `state.db.as_ref()`); repos are unit structs taking `&DatabaseConnection`.
- `AppState` holds the DB pool plus two process-memory caches: `wordbooks_cache` (list result, invalidated via `state.invalidate_wordbooks()` by wordbook CRUD + word create/delete) and `shuffle_cache` ((book_id, seed) → shuffled id sequence, cap `SHUFFLE_CACHE_CAP`, cleared on word create/delete). Locks are `parking_lot::Mutex`, never held across `.await`.
- Handlers return `Result<impl IntoResponse, ApiError>`; `?` converts domain errors and `DbErr` via `From`. `ApiError` maps NotFound/BadRequest/Db to 400/404/500 with user-facing Chinese messages.
- Validation returns domain errors whose `thiserror` text is user-facing Chinese.
- SeaORM: `Entity::find().filter(Column::X.eq(v)).order_by_asc(...).paginate(db, size)`, then `fetch_page(page - 1)` (**0-based offset**) + `num_items()`; API pages are 1-based. `total_pages = total.div_ceil(page_size)`.
- SQLite: `init/db.rs` sets `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`, `cache_size=-20000` after connect.
- Static assets: `assets/` (Vite hashed) and `fonts/` (versioned woff2) served with `Cache-Control: public, max-age=31536000, immutable`; index.html `no-cache`. Whole router wrapped in tower-http `CompressionLayer` (gzip). Fonts are self-hosted in `frontend/public/fonts/` (no Google Fonts CDN).
- Axum 0.8 path syntax: `/api/wordbooks/{id}` (curly braces); multiple path params use a tuple extractor `Path((book_id, id)): Path<(i32, i32)>` (one `Path` extractor per handler).

**Frontend patterns:**
- Components receive plain props; pages hold state; `api.ts` centralizes fetch and re-exports contract types from `frontend/src/generated/` (ts-rs output).
- Reader settings (page size, font px) persisted in `localStorage` under `qw_page_size` / `qw_font_scale`; loaded via lazy `useState(loader)`.
- PaperBookView page cache: `Map<"${bookId}:${page}:${size}", Page<Word>>` in `useRef`, with adjacent-page prefetch; mutations clear the cache.
- Paper book interactions: tap word/meaning toggles ink-stripe cover (`aria-pressed` + transparent-text span preserving layout width); page turns via left/right third click zones, pointer drag (threshold 90px, `dragDxRef`), or ←/→ keys; flip animation states `idle|out|in`.
- Styling: **Visual Organic** design system from `docs/dev/style.md` — palette `ivory #F8F4EF` / `charcoal #2F2A25` / `clay #C58F6D` / `sage #C9D5C6` / `sand #E5D8C8` (defined in `index.css` `@theme`), fonts DM Serif Display (headings, `font-serif`) + Plus Jakarta Sans (body). **No emoji, no gradients** in UI (user-mandated); SVG icons in `components/Icons.tsx` (stroke-based).

## Important Files

| File | Why |
|---|---|
| `crates/server/src/init/db.rs` | Connection + PRAGMA foreign_keys + migration order (failure here breaks cascades) |
| `crates/server/src/router.rs` | Top-level route aggregation; add new top-level domains here (domain routes live in each domain's router.rs) |
| `crates/server/src/business/wordbooks/words/service.rs` | Pagination, validation, seeded shuffle, JSON conversion |
| `crates/server/src/business/wordbooks/words/order.rs` | WordOrder enum + query param parsing (13 POS whitelist in service validate) |
| `crates/server/src/common/http/asset.rs` | rust-embed folder `../../frontend/dist/` — must exist at compile time (keep `.gitkeep`) |
| `.cargo/config.toml` | `TS_RS_EXPORT_DIR` for ts-rs generated contract output |
| `frontend/src/components/PaperBookView.tsx` | Core paper-book UI: ruled grid, tap-to-cover, gesture/zone/keyboard page turns |
| `frontend/src/index.css` | Design tokens (`@theme`), keyframes, texture overlays |
| `config.example.yaml` | Runtime config template — copy to `config.yaml` (git-ignored): `server.host/port`, `database.url` (sqlite:// or postgres://) |

## Runtime/Tooling Preferences

- **Git**: 提交必须经用户明确允许，每次允许只提交一次（一次性提交，不自动追加/重复提交）。
- **Rust**: stable-equivalent (local toolchain is nightly 1.94); edition 2021; workspace resolver 2, crates under `crates/`. Cargo add for deps; shared deps live in `[workspace.dependencies]`; axum must stay 0.8.x (`{id}` path syntax).
- **Node**: v25, npm 11. Vite 8 + Tailwind v4 via `@tailwindcss/vite` plugin (no tailwind.config.js; CSS-first `@theme`).
- **No linter/formatter gate** configured in repo (no eslint run in build); `tsc -b` runs as part of `npm run build`; Rust side gates on `cargo clippy -D warnings` + `cargo fmt --check`.
- UI language: Chinese. Backend error messages in Chinese.

## Testing & QA

- Unit tests live next to the logic they cover (`#[cfg(test)]` in order.rs / sort.rs / sort_dir.rs / service.rs / error.rs): query-param parsing whitelists, validation rules, seeded shuffle determinism/permutation, error→status mapping.
- ts-rs export tests (`export_bindings_*`) regenerate `frontend/src/generated/` on every `cargo test`; commit the generated files alongside DTO changes.
- No integration test suite yet. QA approach: `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo test --workspace` + `cargo build` + `npm run build` must pass; API smoke-tested via curl against a running server (create wordbook → add words → paginate → update → delete → verify cascade with `sqlite3 data/quan_word.db 'SELECT count(*) FROM word;'`); UI verified manually/browser-driven (cover toggle, drag/zone/keyboard page turns, settings sliders, responsive columns 2/3/4/5 by breakpoint).
- When changing pagination: remember API pages are 1-based but `fetch_page` is 0-based.
- When changing schema: add a migration (chronological `mYYYYMMDD_*` file, registered in `crates/migration/src/lib.rs`), and verify BOTH sqlite (default) and postgres (config.example.yaml URL + local `postgres:17` container, e.g. `-p 5433:5432` if a local PG occupies 5432).
- When changing DTOs: rerun `cargo test` to refresh ts-rs output, then `npm run build` to verify frontend type alignment; contract types in `frontend/src/generated/` are the source of truth.
