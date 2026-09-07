# Librarian web

The Librarian frontend: a single-page app with full PWA support, built for phones, tablets,
desktops and TVs. It talks to the Rust backend over GraphQL only (plus the `/api` media and
artwork endpoints) and renders a Plex-style browsing and playback experience.

## Stack

| Concern | Choice |
| --- | --- |
| Build | Vite 8, TypeScript 7, React 19 |
| Routing | TanStack Router (file based, code split per route) |
| Data | Apollo Client 4 over HTTP + graphql-ws, cookie sessions |
| Types | GraphQL Codegen client preset; every operation is a `.graphql` document |
| UI | Tailwind CSS 4 + HeroUI 3 primitives, Librarian tokens (`src/styles`) |
| Forms | react-hook-form + zod |
| Motion / 3D | CSS animations, `motion`, React Three Fiber for ambient scenes |
| Video | hls.js with native fallback, Media Session API |
| PWA | vite-plugin-pwa (Workbox), generated icons |

## Commands

```bash
pnpm install
pnpm schema:pull   # refresh schema.graphql from a running backend (SCHEMA_URL, default :3001)
pnpm codegen       # regenerate entity documents + TypeScript types
pnpm dev           # http://localhost:3000 (PORT=… to change), proxies /graphql and /api to BACKEND_PROXY_TARGET
./live.sh start    # run the live site on :3000 in the background (./live.sh start --preview for the production build)
pnpm lint          # tsc --noEmit
pnpm test          # vitest
pnpm test:coverage # vitest + v8 coverage report in coverage/
pnpm test:e2e      # playwright against the sandbox stack on :3003
pnpm build         # codegen + typecheck + production bundle in dist/
pnpm icons         # regenerate PWA/favicon assets from src/assets/brand
pnpm film-graph    # rebuild the sign-in constellation dataset (Wikidata + Wikipedia; needs network)
node e2e/shoot.mjs <name> <path> [--login] [--viewport=phone|tablet|tv|1440x900]
e2e/sandbox.sh reset|start|stop   # disposable copy of the real backend on :3011 + dev server on :3003
```

`e2e/sandbox.sh` copies the real database and artwork into `/tmp/librarian-sandbox`, disables scans
and organization in the copy, and serves it on port 3011 with a dev server on 3003. Use it for
screenshots and QA without touching real data; sign in with the account you create at
`/register`, then run `e2e/sandbox.sh adopt` so the copied libraries belong to it.

Copy `.env.example` to `.env.local` to point the dev proxy at another backend.

## Testing

Two suites, both run from `web/`.

**Unit and component tests** (`pnpm test`) use Vitest with jsdom and Testing Library. Specs live in
`__tests__` folders beside the code they cover. `renderWithProviders` from `src/test` mounts a
component with Apollo's `MockedProvider`, the toast outlet and (by default) a memory router, so a
test only supplies the GraphQL mocks it needs; pass `shell: true` when the component reads the theme
or input mode. Apollo mocks are reused rather than consumed once, so components that refetch work
without duplicate mocks. `pnpm test:coverage` writes a v8 report to `coverage/`.

**End-to-end smoke tests** (`pnpm test:e2e`) drive a real browser with Playwright against the
sandbox stack: `playwright.config.ts` starts `e2e/sandbox.sh start` if nothing is already listening
on `http://127.0.0.1:3003` and reuses it otherwise. Never point them at the live site on :3000 — the
specs sign in, open dialogs and save. Credentials come from `LIBRARIAN_E2E_USER` and
`LIBRARIAN_E2E_PASSWORD` (default `toby` / `sandbox-password-123`, which only exists in the sandbox
copy). Refresh tokens rotate on every use, so each spec signs in for itself through
`signInViaApi` instead of sharing a saved `storageState`. Anything a spec changes on the server it
changes back, so the sandbox stays usable for screenshots.

```bash
e2e/sandbox.sh start          # or let playwright start it
pnpm test:e2e                 # everything
pnpm test:e2e --project=desktop --headed
pnpm exec playwright show-trace e2e/.results/<test>/trace.zip
```

## How the GraphQL layer works

1. `schema.graphql` is a committed snapshot of the backend schema (`pnpm schema:pull`).
2. `scripts/generate-entity-documents.mjs` reads the schema and writes one document per
   macro-generated entity into `src/graphql/entities/` with a `<Entity>Fields` fragment and the
   standard list / get / create / update / delete / changed operations. Nothing in that folder is
   written by hand.
3. Hand-written operations (relations, custom resolvers) live in `src/graphql/documents/` and
   spread the generated fragments.
4. `graphql-codegen` turns everything into typed document nodes in `src/graphql/generated/`.

Components import `XDocument` constants and use Apollo hooks; no GraphQL string or type is
hand-written anywhere in `src/`.

## Layout of `src/`

```
app/          providers and router setup
components/   ui (design-system kit), shell (rail, top bar, tabs, dock), three (3D scenes)
features/     one folder per product area: home, libraries, media, player, search, downloads,
              activity, settings, auth
graphql/      documents, entities (generated), generated (codegen output)
hooks/        shared hooks (server table state, content statuses, media queries)
lib/          apollo client, session, theme, input mode, formatting, status vocab
routes/       TanStack file routes; `_app` is the authenticated shell, `_player` the immersive
              player, `_auth` the sign-in screens
styles/       tokens, typography, glass, motion, utilities
```

See `STYLE_GUIDE.md` for the visual rules.

## Auth

Credentials are HttpOnly cookies set by the backend. The session store (`lib/auth/session.ts`)
renews the access cookie before it expires, on focus and network recovery, serialises renewal
across tabs with Web Locks, and the Apollo error link retries an operation once after a
successful renewal.

## Input modes

`data-input-mode` on `<html>` is `pointer`, `touch` or `tv`. TV mode is detected from the user
agent or repeated arrow-key use and can be forced from the account menu. Arrow keys move focus
spatially between `[data-focusable]` elements everywhere; media keys and Escape/Backspace work
as remote buttons.
