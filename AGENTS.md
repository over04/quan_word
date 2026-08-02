# Repository Guidelines

## Project Overview

**单词本** — a paper-notebook-style vocabulary learning web app. Users organize words into independent "wordbooks" (单词书), review them in a simulated paper-book view (ruled lines, tap-to-cover self-testing), and manage them in a list view. Backend: Rust (axum 0.8 + SeaORM 2.0 + sea-orm-migration). Frontend: React 19 + Vite 8 + TailwindCSS v4. Single-binary deployment (rust-embed embeds `frontend/dist`). Database: SQLite by default, PostgreSQL switchable via YAML config.

## Architecture & Data Flow

```
Browser (React SPA)
   │  /api/* (dev: Vite proxy → :3000)
   ▼
server (axum 0.8)
   controller/   HTTP layer: extractors, thin handlers, param parsing
      │
      ▼
   service/      business logic: validation, SeaORM queries, DTO assembly
      │
      ▼
   model/        Definition domain type
   entity/       SeaORM 2 dense entities (wordbook, word)
   migration/    schema migrations (sea-orm-migration)
   db.rs         connect + PRAGMA foreign_keys + Migrator::up
```

Data flow: controller parses HTTP (State/Path/Query/Json) → service validates & queries via SeaORM → returns `dto::resp` structs serialized as JSON. Errors bubble as `ApiError` → `IntoResponse` (404/400/500 + `{"error": msg}`).

Key backend decisions:
- `db::init` runs `PRAGMA foreign_keys = ON` **before** migrations for SQLite — without it, deleting a wordbook does not cascade-delete words.
- Word counting uses a single `GROUP BY` aggregate query (no N+1 per-book count).
- `definitions` stored as JSON column; converted with `serde_json::to_value/from_value` (SeaORM 2 does not support custom types as Json columns).
- `timestamp_with_time_zone()` in migrations — `timestamp()` breaks PG (DateTimeUtc needs TIMESTAMPTZ); both work on SQLite.

## Key Directories

```
entity/src/          SeaORM 2.0 dense entities (wordbook.rs, word.rs) — conventional, one entity per file
migration/src/       Migrator + m20260802_* migration files (chronological)
server/src/
  controller/        HTTP handlers (word_controller.rs, wordbook_controller.rs, static_controller/)
  service/           business logic (word_service.rs, wordbook_service.rs)
  model/             domain types (definition.rs)
  dto/req/           request bodies (create_*_req.rs, update_*_req.rs)
  dto/resp/          response bodies (word_resp.rs, wordbook_resp.rs, page_resp.rs)
  config/            YAML config structs (one per file)
  db.rs router.rs state.rs error.rs main.rs
frontend/src/
  pages/             WordbookList, WordbookDetail
  components/        PaperBookView (core paper-book UI), WordTable, modals, SettingsPanel, Icons, Pagination, WavyDivider
  api.ts             fetch wrapper + TS types matching backend DTOs
frontend/dist/       build output (embedded by rust-embed; .gitkeep placeholder)
```

## Development Commands

```bash
# Backend (workspace root)
cargo build                    # dev build, whole workspace
cargo run -p server            # dev server on :3000 (config.yaml or built-in defaults)
cargo build --release          # production binary (embeds frontend/dist at compile time)

# Frontend
cd frontend && npm run dev     # Vite dev :5173, proxies /api → :3000
cd frontend && npm run build   # tsc -b + vite build → dist/ (MUST run before cargo build for fresh embed)

# Production
./target/release/server        # run from project root (reads config.yaml)
```

No test suite exists yet; verification is via API smoke tests (curl) and manual browser flows.

## Code Conventions & Common Patterns

**Strict project rules (user-mandated, enforce on all new code):**
- **One struct per `.rs` file.** No exceptions in server crate (entity/migration follow SeaORM conventions instead).
- **No `pub use` re-exports.** Reference full module paths: `use crate::dto::req::create_word_req::CreateWordReq;`. `mod.rs` declares modules only.
- **MVC layering:** controller (HTTP only) → service (business) → model/dto. Controllers are thin; validation and queries live in services.
- **Req/Resp split:** `dto/req/*_req.rs` (Deserialize) and `dto/resp/*_resp.rs` (Serialize), suffixed `Req`/`Resp` (e.g. `CreateWordReq`, `WordbookResp`).
- **Naming:** `word_service.rs` / `word_controller.rs` / `create_word_req.rs` — kebab snake_case files matching their primary symbol.

**Backend patterns:**
- Services are unit structs (`pub struct WordService;`) with associated async fns taking `&DatabaseConnection` as first arg.
- Handlers return `Result<impl IntoResponse, ApiError>`; `?` converts `DbErr` via `From`. `ApiError::BadRequest/NotFound/Db/Internal` map to 400/404/500.
- Validation returns `ApiError::BadRequest("中文错误信息")` — error messages are user-facing Chinese.
- SeaORM: `Entity::find().filter(Column::X.eq(v)).order_by_asc(...).paginate(db, size)`, then `fetch_page(page - 1)` (**0-based offset**) + `num_items()`; API pages are 1-based. `total_pages = total.div_ceil(page_size)`.
- Axum 0.8 path syntax: `/api/wordbooks/{id}` (curly braces).

**Frontend patterns:**
- Components receive plain props; pages hold state; `api.ts` centralizes fetch + types (mirror backend DTOs: `Wordbook`, `Word`, `Page<T>`, `Definition`).
- Reader settings (page size, font px) persisted in `localStorage` under `qw_page_size` / `qw_font_scale`; loaded via lazy `useState(loader)`.
- PaperBookView page cache: `Map<"${bookId}:${page}:${size}", Page<Word>>` in `useRef`, with adjacent-page prefetch; mutations clear the cache.
- Paper book interactions: tap word/meaning toggles ink-stripe cover (`aria-pressed` + transparent-text span preserving layout width); page turns via left/right third click zones, pointer drag (threshold 90px, `dragDxRef`), or ←/→ keys; flip animation states `idle|out|in`.
- Styling: **Visual Organic** design system from `docs/dev/style.md` — palette `ivory #F8F4EF` / `charcoal #2F2A25` / `clay #C58F6D` / `sage #C9D5C6` / `sand #E5D8C8` (defined in `index.css` `@theme`), fonts DM Serif Display (headings, `font-serif`) + Plus Jakarta Sans (body). **No emoji, no gradients** in UI (user-mandated); SVG icons in `components/Icons.tsx` (stroke-based).

## Important Files

| File | Why |
|---|---|
| `server/src/db.rs` | Connection + PRAGMA foreign_keys + migration order (failure here breaks cascades) |
| `server/src/router.rs` | Full API route table; add new endpoints here |
| `server/src/service/word_service.rs` | Pagination, POS enum validation (13 allowed values), JSON conversion |
| `server/src/controller/static_controller/asset.rs` | rust-embed folder `../frontend/dist/` — must exist at compile time (keep `.gitkeep`) |
| `frontend/src/components/PaperBookView.tsx` | Core paper-book UI: ruled grid, tap-to-cover, gesture/zone/keyboard page turns |
| `frontend/src/index.css` | Design tokens (`@theme`), keyframes, texture overlays |
| `config.yaml` | Runtime config: `server.host/port`, `database.url` (sqlite:// or postgres://) |
| `migration/src/m20260802_*.rs` | Schema; use `timestamp_with_time_zone` for datetimes |

## Runtime/Tooling Preferences

- **Git**: 提交必须经用户明确允许，每次允许只提交一次（一次性提交，不自动追加/重复提交）。
- **Rust**: stable-equivalent (local toolchain is nightly 1.94); edition 2021; workspace resolver 2. Cargo add for deps; axum must stay 0.8.x (`{id}` path syntax).
- **Node**: v25, npm 11. Vite 8 + Tailwind v4 via `@tailwindcss/vite` plugin (no tailwind.config.js; CSS-first `@theme`).
- **No linter/formatter gate** configured in repo (no rustfmt/clippy config, no eslint run in build); `tsc -b` runs as part of `npm run build`.
- UI language: Chinese. Backend error messages in Chinese.
- Fonts load from Google Fonts CDN (offline falls back to system stacks).

## Testing & QA

- No automated test suite yet. QA approach: `cargo build` + `npm run build` must pass; API smoke-tested via curl against a running server (create wordbook → add words → paginate → update → delete → verify cascade with `sqlite3 data/quan_word.db 'SELECT count(*) FROM word;'`); UI verified manually/browser-driven (cover toggle, drag/zone/keyboard page turns, settings sliders, responsive columns 2/3/4/5 by breakpoint).
- When changing pagination: remember API pages are 1-based but `fetch_page` is 0-based.
- When changing schema: add a migration (chronological `mYYYYMMDD_*` file, registered in `migration/src/lib.rs`), and verify BOTH sqlite (default) and postgres (config.yaml URL + local `postgres:17` container, e.g. `-p 5433:5432` if a local PG occupies 5432).
- Frontend type drift: TS types in `frontend/src/api.ts` must match `dto/resp` structs; JSON `definitions` array shape is `[{pos, meaning}]`.
