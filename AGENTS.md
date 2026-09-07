# Repository Guidelines

## Project Structure & Module Organization

- `backend/`: Rust Axum API,service.
- `web/`: the frontend, a full-screen SPA/PWA (Vite + TanStack Router + Apollo + HeroUI 3). GraphQL documents are generated per entity by `web/scripts/generate-entity-documents.mjs`; see `web/README.md` and `web/STYLE_GUIDE.md`. Serves the live site on :3000 (`web/live.sh start|stop|restart|status`, or `make live-web`). `DEV_SERVER_PUBLIC_URL` in `web/.env.local` sets the public origin for HMR and allowed hosts. Release binaries embed `web/dist` (`cargo build --release --features embed-frontend`), so `make build-web` must run first. The previous `frontend/` app was retired in September 2026 and exists only in git history.
- `macros/`: internal Rust proc-macro crate used by the backend.
- `docs/`: architecture, design, and workflow notes.

## Rule Hierarchy (All Agents)

- Primary architecture and product rules: `docs/design.md`
- Repository workflow rules: `AGENTS.md` (this file)
- Implementation rule packs: `.cursor/rules/*.mdc`

All agents must consult `.cursor/rules/` before making changes. At minimum, review:
- `.cursor/rules/frontend-ui.mdc` for frontend/UI work.
- `.cursor/rules/entity-single-source-of-truth.mdc` for GraphQL/entity data access.
- `.cursor/rules/graphql-naming-convention.mdc` for GraphQL naming and schema shape.
- `.cursor/rules/media-pipeline.mdc` for matching/pipeline behavior.
- `.cursor/rules/logging.mdc` for logging standards.

## Build, Test, and Development Commands

- `make dev`: start backend and frontend dev servers.
- `make dev-backend` / `make dev-web`: run one side only.
- `make build`: build the web bundle (`pnpm run build`) then the backend with it embedded (`cargo build --release --features embed-frontend`).
- `make test`: run `cargo test` and `pnpm test` (Vitest).
- `make lint`: backend `cargo clippy` + `cargo fmt --check`; web `pnpm run lint` (tsc).
- `make db-migrate`: run `sqlx` migrations against SQLite (`DATABASE_PATH` defaults to `./data/librarian.db`).
- `make docker-up`: start the Docker dev stack; `make prod-up` for production compose.
- `cargo run`: runs the proudction or development backend (never use this command, I will run it manually, you can use cargo check and cargo test)
- `pnpm dev`: runs the web dev server (never use this command; the live server on :3000 is managed by `web/live.sh`)
- Cap builds at **80% of the CPUs available to the machine/container**, using an aggregate CPU quota that includes compiler/linker child processes and concurrent builds. For example, four available CPUs means a cgroup `cpu.max` of `320000 100000` (systemd `CPUQuota=320%`). `cargo -j 1` alone is not a CPU cap. Keep interactive tools and application servers outside the build cgroup.

## Coding Style & Naming Conventions

- Rust uses `rustfmt` defaults; run `cargo fmt` before pushing.
- Rust modules and functions use `snake_case`; types and traits use `CamelCase`.
- Frontend components use `PascalCase` filenames (`MediaCard.tsx`); hooks are `useThing.ts`.
- Tailwind + HeroUI drive UI styling; follow `.cursor/rules/` (especially `frontend-ui.mdc`) for layout and UI patterns.
- Follow `docs/design.md` for product, architecture, and agent rules.

## Backend Data Access Rules (Mandatory)

- Never use direct SQL for application domain reads/writes in services, resolvers, or jobs.
- Required path for data changes: generated GraphQL entity mutations via `execute_mutation` (or generated resolvers in GraphQL context).
- Required path for reads: generated GraphQL queries/types (or existing typed repository/query abstractions that are already part of the GraphQL entity layer).
- Do not add new `sqlx::query*`, raw `SELECT/INSERT/UPDATE/DELETE`, or ad-hoc table access for domain entities.
- Allowed exceptions:
  - `backend/src/db/schema_sync.rs` for schema/table creation + migration sync logic.
  - startup/infra plumbing where no entity layer exists yet (must be explicitly documented in PR notes).
- If a required mutation/query does not exist, add it to the GraphQL entity layer first; do not bypass with SQL.
- Before finishing backend work, run a quick grep and ensure no new direct SQL was introduced outside allowed files.

## Testing Guidelines

- Backend: `cargo test`; integration tests live in `backend/tests/`.
- Web: `pnpm test` (Vitest, colocated under `src/**/__tests__`) and `pnpm test:e2e` (Playwright against the sandbox stack, `web/e2e/sandbox.sh`).
- No explicit coverage gate is defined; keep new tests focused on new behavior.

## Commit & Pull Request Guidelines

- Recent history uses short, sentence-case summaries (no strict Conventional Commits).
- Keep commits small and scoped; call out migrations or schema changes in the message.
- PRs should include a clear description, testing notes, and screenshots for UI changes.
- If you add migrations or env vars, mention them in the PR body and update `.env.example` when needed.

## Configuration & Security Notes

- For encryption or indexer changes, note key handling (see `README.md`).
