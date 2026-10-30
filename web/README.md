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
| UI | Glass UX 0.9.10 installed for migration; current screens use Tailwind CSS 4 + HeroUI 3/custom primitives |
| Forms | react-hook-form + zod |
| Motion / 3D | CSS animations, `motion`, React Three Fiber for ambient scenes |
| Video | hls.js with native fallback, Media Session API |
| PWA | vite-plugin-pwa (Workbox), generated icons |

## Glass UX migration

The [migration plan](../docs/glass-ux-migration-plan.md) maps the complete site and
library catalogue. [Reference installation](../docs/glass-ux/README.md) records
the downloaded docs, pinned release, six project skills and shared rules.
The package is installed; global CSS/providers and screen replacement are pending
the foundation and pilot slices. Do not import its global CSS into the existing
HeroUI token cascade without that migration.

## Commands

```bash
pnpm install
pnpm schema:pull   # refresh schema.graphql from a running backend (SCHEMA_URL, default :3001)
pnpm codegen       # regenerate entity documents + TypeScript types
pnpm codegen:watch # watch SDL/config/handwritten documents; serialize regeneration
pnpm codegen:check # reject stale generated entity documents without rewriting them
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
   Changed subscriptions select invalidation metadata only; refetch an authorized query for
   entity data. The ORM rejects computed/object-valued entity fields in changed payloads.
3. Hand-written operations (relations, custom resolvers) live in `src/graphql/documents/` and
   spread the generated fragments.
4. `graphql-codegen` turns everything into typed document nodes in `src/graphql/generated/`.

Components import `XDocument` constants and use Apollo hooks; no GraphQL string or type is
hand-written anywhere in `src/`.

Mounted data reads use `useLiveQuery` from `lib/graphql/live/hooks`. The shared
registry derives generated root/nested dependencies, shares listeners, coalesces
refreshes and reconciles after reconnect or missed events. Imperative complete
snapshots and route loaders register their own refresh callbacks through
`useLiveInvalidation`/`useLiveRoute`; an Apollo cache update alone does not replace
their local snapshots. Keep loaded windows visible while staged refreshes run.
Principal changes clear old cache/snapshot epochs. Domain queries do not use
recurring polls; playback checkpoints, session renewal and PWA updates are
separate timers. Automatic HTTP retry applies to queries only.

The generator checks all 46 registered public entities, including their change
roots, before replacing any document. Keyset operations are additional documents
inside the matching entity file and must not overwrite ordinary list operations.
Custom action, capability and computed-read documents remain in `documents/`
and receive the same schema validation/type generation.

The repository also provides `scripts/export-schema-low-memory.sh` for an
offline export of the actual merged Rust schema on a constrained Linux host.
It needs an existing build cgroup with an aggregate CPU quota at or below 80%.
Its guarded tooling mode stubs generated runtime bodies; successful export and
codegen do not qualify the backend runtime. CI separately compares its SDL with
the normal backend export. See [macro tooling notes](../vendor/graphql-orm-macros/PATCH.md)
and [backend alignment gates](../docs/backend-alignment-plan.md).

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

Installable LG webOS, Samsung Tizen, and Android TV / Google TV applications are planned in
[TV applications implementation plan](../docs/tv-applications-plan.md). The existing TV input
mode is a starting point; native packages and device compatibility are not yet qualified.
An [Android TV / Google TV debug prototype](../apps/android-tv/README.md) now builds and can be
installed directly on a TV for testing; it loads this frontend from the user's selected server.

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
