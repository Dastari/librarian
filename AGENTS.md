# Repository Guidelines

## Project Structure & Module Organization

- `backend/`: Rust Axum API,service.
- `frontend/`: TanStack Start app with `src/components/`, `src/hooks/`, `src/lib/graphql/`, and file-based routes in `src/routes/`.
- `librarian-macros/`: internal Rust proc-macro crate used by the backend.
- `docs/`: architecture, design, and workflow notes.

## Build, Test, and Development Commands

- `make dev`: start backend and frontend dev servers.
- `make dev-backend` / `make dev-frontend`: run one side only.
- `make build`: build backend (`cargo build --release`) and frontend (`pnpm run build`).
- `make test`: run `cargo test` and `pnpm test` (Vitest).
- `make lint`: backend `cargo clippy` + `cargo fmt --check` and frontend `pnpm run lint`.
- `make db-migrate`: run `sqlx` migrations against SQLite (`DATABASE_PATH` defaults to `./data/librarian.db`).
- `make docker-up`: start the Docker dev stack; `make prod-up` for production compose.
- `cargo run`: runs the proudction or development backend (never use this command, I will run it manually, you can use cargo check and cargo test)
- `pnpm dev`: runs the development frontend (never use this command I will always have the dev server running)

## Coding Style & Naming Conventions

- Rust uses `rustfmt` defaults; run `cargo fmt` before pushing.
- Rust modules and functions use `snake_case`; types and traits use `CamelCase`.
- Frontend components use `PascalCase` filenames (`MediaCard.tsx`); hooks are `useThing.ts`.
- Tailwind + HeroUI drive UI styling; follow `docs/style-guide.md` for layout and UI patterns.

## Testing Guidelines

- Backend: `cargo test`; integration tests live in `backend/tests/`.
- Frontend: `pnpm test` (Vitest); colocate tests with features (e.g., `src/.../*.test.tsx`).
- No explicit coverage gate is defined; keep new tests focused on new behavior.

## Commit & Pull Request Guidelines

- Recent history uses short, sentence-case summaries (no strict Conventional Commits).
- Keep commits small and scoped; call out migrations or schema changes in the message.
- PRs should include a clear description, testing notes, and screenshots for UI changes.
- If you add migrations or env vars, mention them in the PR body and update `.env.example` when needed.

## Configuration & Security Notes

- For encryption or indexer changes, note key handling (see `README.md`).
