# Frontend Consolidated Audit Plan

Date: 2026-02-13
Sources: `frontend/audit1.md`, `frontend/audit2.md`

This file consolidates and deduplicates all findings from both audits into one prioritized remediation plan.

## Status Update (2026-02-13)

Completed:
- Security: open-redirect mitigation for sign-in callback flows.
- Security: `graphqlClient` global exposure gated to dev-only.
- Security: token prefix logging removed from frontend auth helpers.
- Security: shared authenticated REST helper added and applied to torrent upload, health, and Ollama tags calls.
- Security (partial migration): backend now rotates/clears refresh token via HttpOnly cookie on auth mutations; frontend refresh/logout flow uses cookie fallback.
- Maintainability: collection routing normalized to internal `Collection.Id` (no TMDB-ID route navigation).
- Maintainability: shared collection card UI primitives added and reused across collection routes and add-collection modal.

In progress / remaining from security track:
- Full server-managed auth migration still pending for access token (access token remains JS-readable in frontend auth storage).

Recommended next continuation point:
- Section 2: GraphQL naming and legacy operation cleanup.

## 1. Security (Highest Priority)

1. Open redirect in sign-in callback handling.
- Files:
`frontend/src/routes/index.tsx:44`
`frontend/src/components/SignInModal.tsx:113`
`frontend/src/components/SignInModal.tsx:130`
- Fix:
validate `redirect` as internal-only (relative path or strict allowlist), fallback to `/`.
- Acceptance:
external redirect attempts (for example `https://evil.example`) are rejected.
- Status:
Done (2026-02-13).

2. Authenticated GraphQL client exposed on `globalThis` in production.
- File:
`frontend/src/lib/graphql/client.ts:317`
- Fix:
remove global exposure or gate under `import.meta.env.DEV`.
- Acceptance:
no production access to `graphqlClient` helpers through browser global scope.
- Status:
Done (2026-02-13).

3. JS-readable auth cookies and token prefix logging.
- File:
`frontend/src/lib/auth.ts`
- Fix:
remove token prefix logging; document and/or migrate auth cookie approach toward HttpOnly server-set cookies.
- Acceptance:
no token material appears in logs; auth storage risk is explicitly resolved or documented.
- Status:
Partially done (2026-02-13): token prefix logging removed and refresh token moved to HttpOnly cookie flow; access token remains JS-readable pending full migration.

4. Security-adjacent consistency issue: raw authenticated REST calls.
- Files:
`frontend/src/routes/downloads/index.tsx:437`
`frontend/src/routes/settings/index.tsx:68`
`frontend/src/routes/settings/organization.tsx:723`
- Fix:
introduce shared authenticated REST fetch helper for non-GraphQL endpoints (upload/health/tags).
- Acceptance:
these endpoints use one shared auth/header/error path.
- Status:
Done (2026-02-13).

## 2. GraphQL Naming and Legacy Operation Cleanup

1. Migrate active legacy camelCase/lowercase root fields to PascalCase.
- Files:
`frontend/src/lib/graphql/queries.ts`
`frontend/src/lib/graphql/mutations.ts`
subscription documents/usages in active components.
- Fix:
backend resolver migration first, then frontend operation updates, then codegen regeneration.
- Acceptance:
no active frontend operation depends on camelCase/lowercase root fields.

2. Remove mixed casing subscription patterns.
- Fix:
eliminate alias-based transitional patterns once backend naming is corrected.
- Acceptance:
subscription documents are consistently PascalCase and generated types compile cleanly.

## 3. Data Fetching Standardization

1. Standardize on generated `TypedDocumentNode` + Apollo hooks (`useQuery`, `useMutation`).
- Convert runtime string + `gql(...)` usages.
- Convert avoidable imperative `apolloClient.query/mutate` flows.

2. Keep raw `fetch` only for true REST endpoints, behind shared helper.

3. Priority conversion targets:
- `frontend/src/components/library/LibraryMoviesTab.tsx`
- `frontend/src/components/library/LibraryShowsTab.tsx`
- `frontend/src/components/library/LibraryAlbumsTab.tsx`
- `frontend/src/components/library/LibraryAudiobooksTab.tsx`
- `frontend/src/routes/settings/casting.tsx`
- `frontend/src/components/downloads/MediaFilesMatchDialog.tsx`
- `frontend/src/contexts/PlaybackContext.tsx` (only where imperative calls are avoidable)

Acceptance:
- no inline GraphQL operation strings in feature components.
- minimal justified imperative Apollo calls.
- REST calls routed through one helper.

## 4. HeroUI Compliance and Shared UI Patterns

1. Replace disallowed raw primitives (`button`, `img`) with HeroUI components.
- Files flagged across audits:
`frontend/src/routes/movies/$movieId.tsx`
`frontend/src/routes/shows/$showId.tsx`
`frontend/src/routes/albums/$albumId.tsx`
`frontend/src/routes/audiobooks/$audiobookId.tsx`
`frontend/src/components/filters/SearchInput.tsx`
`frontend/src/components/Navbar.tsx`
`frontend/src/routes/collections/$collectionId.tsx`
`frontend/src/components/downloads/TorrentTable.tsx`
`frontend/src/components/library/LibraryGridCard.tsx`
`frontend/src/components/library/MovieCard.tsx`

2. Extract shared poster/cover play overlay.
- Fix:
create reusable component wrapping HeroUI `Button` (`isIconOnly`) to replace repeated overlay pattern in movie/show/album/audiobook detail routes.

3. Keep allowed exception:
`<input type="file">` in `frontend/src/components/downloads/AddTorrentModal.tsx`.

Acceptance:
- no disallowed raw primitives remain outside approved exceptions.

## 5. Modal and Error UX Consistency

1. Unify confirmation dialogs.
- Standard:
simple confirmations use shared `ConfirmModal`.
entity-specific complex delete flows use dedicated `Delete*Modal`.
- Fix:
remove inline delete modals from show/album/audiobook routes and align with chosen pattern.

2. Standardize error feedback behavior.
- Fix:
define per-surface convention (form submit, table action, background refresh) and remove throw/console-only inconsistency.
- Example hotspots:
`frontend/src/components/search/AddToLibraryModal.tsx`
`frontend/src/routes/settings/sources.tsx`
`frontend/src/routes/settings/usenet.tsx`
`frontend/src/routes/settings/index.tsx`

Acceptance:
- similar user actions show consistent confirmation and error behavior across routes.

## 6. Query UX Stability and Cache Discipline

1. Add `previousData` fallback where missing for list/table queries.
- Priority files:
`frontend/src/components/library/LibraryMoviesTab.tsx`
`frontend/src/components/library/LibraryShowsTab.tsx`
`frontend/src/components/library/LibraryTracksTab.tsx`
`frontend/src/components/library/LibraryAuthorsTab.tsx`
`frontend/src/components/library/LibraryArtistsTab.tsx`
`frontend/src/components/downloads/MediaFilesMatchDialog.tsx`
`frontend/src/components/search/AddToLibraryModal.tsx`
`frontend/src/routes/settings/organization.tsx`

2. Reduce broad `fetchPolicy: "network-only"` usage where unnecessary.
- Hotspots:
`frontend/src/contexts/PlaybackContext.tsx:262`
`frontend/src/routes/settings/sources.tsx:236`

Acceptance:
- reduced loading flashes during refetch.
- less avoidable cache churn.

## 7. Maintainability Track (After Functional and Consistency Fixes)

1. Split oversized files into focused modules/components.
- `frontend/src/routes/settings/organization.tsx`
- `frontend/src/components/downloads/TorrentInfoModal.tsx`
- `frontend/src/components/downloads/MediaFilesMatchDialog.tsx`

2. Consolidate duplicated helper patterns.
- Status:
Partially done (2026-02-13): shared collection card primitives extracted and reused.

3. Optional React 19 enhancements (targeted, not blanket):
- `useDeferredValue` for search-heavy UIs.
- `useActionState` for modal/settings submit state.
- `useOptimistic` for low-risk optimistic row/toggle interactions.

Acceptance:
- clearer component boundaries and lower maintenance overhead without behavior regressions.

## Suggested Implementation Order

1. Security fixes (redirect, client exposure, auth/token logging, shared REST auth helper).
2. GraphQL naming migration and generated document/type regeneration.
3. Data fetching standardization onto generated docs + hooks.
4. HeroUI primitive replacement and shared play overlay component extraction.
5. Modal confirmation and error UX standardization.
6. `previousData` rollout and fetch policy tuning.
7. Large-file refactors and optional React 19 ergonomic improvements.

## Verification Checklist

1. Typecheck:
`pnpm exec tsc --noEmit`

2. Tests:
`pnpm test`

3. Grep-based consistency checks:
- no legacy camelCase/lowercase GraphQL root operations in active frontend docs/usages.
- no forbidden raw `button`/`img` usage (except approved exceptions).
- no token value/prefix logging.

4. Manual QA:
- sign-in redirect cannot leave site via crafted query params.
- key list/detail screens do not flash unnecessarily on refetch.
- delete/confirm flows are consistent for movie/show/album/audiobook and settings actions.
