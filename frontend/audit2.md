# Frontend Audit 2

Date: 2026-02-13  
Scope: `frontend/` full consistency and best-practices review

## Findings (Ordered by Severity)

1. **High: Open redirect on sign-in callback**
- `redirect` is accepted as any string and then used directly in `window.location.href`.
- File refs:
`frontend/src/routes/index.tsx:44`
`frontend/src/components/SignInModal.tsx:113`
`frontend/src/components/SignInModal.tsx:130`
- Risk: crafted `/?signin=true&redirect=https://evil.example` can redirect users to attacker-controlled domains after auth.

2. **Medium: Auth tokens are JS-readable cookies (XSS exfiltration risk) + token prefix logging**
- Tokens are written/read via `document.cookie` (not `HttpOnly`), and token prefix is logged in development.
- File refs:
`frontend/src/lib/auth.ts:90`
`frontend/src/lib/auth.ts:96`
`frontend/src/lib/auth.ts:169`
`frontend/src/lib/auth.ts:85`

3. **Medium: Active mixed GraphQL conventions (legacy camelCase/lowercase resolvers still used)**
- Legacy mutation/query strings with lowercase root fields remain and are consumed by active components.
- File refs:
`frontend/src/lib/graphql/mutations.ts:85`
`frontend/src/lib/graphql/mutations.ts:101`
`frontend/src/lib/graphql/mutations.ts:778`
`frontend/src/lib/graphql/queries.ts:12`
`frontend/src/components/downloads/TorrentInfoModal.tsx:651`
- Impact: schema evolution friction and inconsistent developer patterns.

4. **Medium: Data fetching patterns are highly inconsistent**
- Mixed `useQuery/useMutation`, direct `apolloClient.query/mutate`, inline runtime `gql`, and raw `fetch`.
- File refs:
`frontend/src/routes/settings/casting.tsx:150`
`frontend/src/contexts/PlaybackContext.tsx:260`
`frontend/src/routes/downloads/index.tsx:437`
`frontend/src/routes/settings/index.tsx:68`
`frontend/src/components/downloads/MediaFilesMatchDialog.tsx:64`

5. **Medium: HeroUI rule violations (`.cursor/rules/frontend-ui.mdc`)**
- Raw primitives used where HeroUI components are expected (`button`, `img`).
- File refs:
`frontend/src/components/filters/SearchInput.tsx:59`
`frontend/src/routes/movies/$movieId.tsx:426`
`frontend/src/routes/shows/$showId.tsx:844`
`frontend/src/routes/albums/$albumId.tsx:792`
`frontend/src/routes/audiobooks/$audiobookId.tsx:657`
`frontend/src/components/Navbar.tsx:89`
`frontend/src/routes/collections/$collectionId.tsx:259`

6. **Medium: Modal/dialog patterns differ significantly across similar surfaces**
- Movies route uses reusable `DeleteMovieModal`; show/album/audiobook routes use inline delete modals.
- File refs:
`frontend/src/routes/movies/$movieId.tsx:790`
`frontend/src/routes/shows/$showId.tsx:1124`
`frontend/src/routes/albums/$albumId.tsx:998`
`frontend/src/routes/audiobooks/$audiobookId.tsx:892`
- Additional inconsistency: mixed `ConfirmModal` vs ad-hoc modal confirmations.
`frontend/src/routes/libraries/$libraryId/albums.tsx:96`

7. **Medium: Error UX is inconsistent (toast vs inline card vs throw + console)**
- Similar failures are handled in multiple incompatible ways.
- File refs:
`frontend/src/components/search/AddToLibraryModal.tsx:300`
`frontend/src/components/search/AddToLibraryModal.tsx:360`
`frontend/src/routes/settings/sources.tsx:698`
`frontend/src/routes/settings/usenet.tsx:358`
`frontend/src/routes/settings/index.tsx:75`

8. **Low: `previousData` not consistently used where `useQuery` is used**
- These callsites use `useQuery` without `previousData` fallback handling.
- File refs:
`frontend/src/components/downloads/MediaFilesMatchDialog.tsx:867`
`frontend/src/components/search/AddToLibraryModal.tsx:201`
`frontend/src/routes/settings/organization.tsx:311`

9. **Low: DataTable consistency**
- Good: no raw `<table>` usage found in `frontend/src`.
- Gap: several list-like UIs are hand-rendered instead of DataTable/card mode patterns.
- File refs:
`frontend/src/components/SearchModal.tsx:194`
`frontend/src/components/search/AddToLibraryModal.tsx:195`

10. **Low: Function style consistency is mostly good but still mixed helper style + duplication**
- Most components use `export function ...`, but helper arrows are duplicated across files.
- File refs:
`frontend/src/components/NotificationPopover.tsx:103`
`frontend/src/routes/notifications.tsx:148`

11. **Low: React/Vite maintainability hotspots**
- Very large files have accumulated too many responsibilities.
- File refs:
`frontend/src/routes/settings/organization.tsx`
`frontend/src/components/downloads/TorrentInfoModal.tsx`
`frontend/src/components/downloads/MediaFilesMatchDialog.tsx`
- Additional cache-churn concern from broad `fetchPolicy: "network-only"` usage.
- File refs:
`frontend/src/contexts/PlaybackContext.tsx:262`
`frontend/src/routes/settings/sources.tsx:236`

12. **React 19 hook opportunities**
- `useOptimistic`: wanted toggles / row action updates before server ack.
- `useActionState`: modal submit flows (`Add*Modal`, settings forms) to centralize pending/error/result state.
- `useDeferredValue`: search-heavy list UIs (`SearchModal`, Add-to-library search) for smoother typing.

## Assumptions

1. `.cursor/rules/frontend-ui.mdc` “no raw primitives” applies to route-level play overlays.
2. Team direction is to standardize on generated Apollo documents/hooks and phase out legacy runtime string operations.

## Quick Summary

- Top immediate risks: open redirect, JS-readable auth tokens, and active legacy GraphQL operation patterns.
- Top consistency debt: fetch layer style, modal/error handling, and raw primitive usage against UI rules.
