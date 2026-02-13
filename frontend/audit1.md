# Frontend Code Audit — February 2026

## 1. Security Issues

### 1a. Raw `fetch()` calls bypass GraphQL auth layer
Three places use raw `fetch()` instead of the GraphQL client, manually reconstructing auth headers. This is fragile and bypasses the centralized error/auth handling in `client.ts`.

| File | Line | Call |
|---|---|---|
| `routes/downloads/index.tsx` | ~437 | `fetch(\`${API_URL}/api/torrents/upload\`)` — torrent file upload |
| `routes/settings/index.tsx` | ~68 | `fetch(\`${API_URL}/api/healthz\`)` — health check |
| `routes/settings/organization.tsx` | ~723 | `fetch(\`${baseUrl}/api/tags\`)` — Ollama tags |

**Recommendation:** The torrent upload is acceptable (multipart form), but wrap it in a shared helper that reads auth from the same source as the Apollo client. The health/tags calls should also use a shared fetch wrapper with auth headers.

### 1b. `globalThis` exposure of GraphQL client
`client.ts` lines 317-322 expose `graphqlClient`, `queryPromise`, `mutationPromise`, and `subscriptionStream` on `globalThis`. This makes the authenticated GraphQL client accessible from the browser console in production.

```typescript
if (typeof globalThis !== "undefined") {
  (globalThis as any).graphqlClient = graphqlClient;
  // ...
}
```

**Recommendation:** Gate this behind `import.meta.env.DEV` or remove entirely.

### 1c. User JSON stored in non-HttpOnly cookie
`lib/auth.ts` stores the full user object (`librarian_user`) as a JSON cookie set via `document.cookie`. These cookies are not HttpOnly, meaning any XSS vulnerability would give access to tokens.

**Recommendation:** Consider moving tokens to HttpOnly cookies set by the server, or at minimum document this trade-off.

### 1d. Stale comment references localStorage
`client.ts` line 39 says `// Helper to get auth token from localStorage` but tokens are actually stored in cookies. Misleading for future developers.

---

## 2. GraphQL Naming: camelCase vs PascalCase (Legacy Resolvers)

The project mandates PascalCase for all GraphQL-exposed names, but a large number of queries and mutations in `queries.ts` and `mutations.ts` still use legacy camelCase root fields. This is the single biggest consistency issue.

### Queries still using camelCase root fields

| Query Constant | camelCase root field |
|---|---|
| `TORRENTS_QUERY` | `torrents { ... }` |
| `TORRENT_QUERY` | `torrent(id: $id)` |
| `TORRENT_DETAILS_QUERY` | `torrentDetails(id: $id)` |
| `PENDING_FILE_MATCHES_QUERY` | `pendingFileMatches(...)` |
| `ACTIVE_DOWNLOAD_COUNT_QUERY` | `activeDownloadCount` |
| `TORRENT_SETTINGS_QUERY` | `torrentSettings { ... }` |
| `UPnP_STATUS_QUERY` | `upnpStatus { ... }` |
| `TEST_PORT_ACCESSIBILITY_QUERY` | `testPortAccessibility(...)` |
| `LLM_PARSER_SETTINGS_QUERY` | `llmParserSettings { ... }` |
| `LIBRARIES_QUERY` | `libraries { ... }` (the old one; `LIBRARIES_WITH_COUNTS_QUERY` is PascalCase) |
| `ALBUM_QUERY` | `album(id: $id) { ... }` (uses camelCase fields too) |
| `TRACKS_QUERY` | `tracks(albumId: $albumId) { ... }` |
| `TRACKS_CONNECTION_QUERY` | `tracksConnection(...)` with camelCase fields (`edges`, `node`, `pageInfo`) |
| `RSS_FEEDS_QUERY` | `rssFeeds(...)` |
| `PARSE_AND_IDENTIFY_QUERY` | `parseAndIdentifyMedia(...)` |
| `LOGS_QUERY` | `logs(...)` with camelCase fields |
| `LOG_TARGETS_QUERY` | `logTargets(...)` |
| `LOG_STATS_QUERY` | `logStats { ... }` |
| `UPCOMING_EPISODES_QUERY` | `upcomingEpisodes(...)` |
| `LIBRARY_UPCOMING_EPISODES_QUERY` | `libraryUpcomingEpisodes(...)` |
| `UNMATCHED_FILES_QUERY` | `unmatchedFiles(...)` |
| `UNMATCHED_FILES_COUNT_QUERY` | `unmatchedFilesCount(...)` |
| `MEDIA_FILE_BY_PATH_QUERY` | `mediaFileByPath(...)` |
| `MOVIE_MEDIA_FILE_QUERY` | `movieMediaFile(...)` |
| `MEDIA_FILE_DETAILS_QUERY` | `mediaFileDetails(...)` |
| `QUICK_PATHS_QUERY` | `quickPaths { ... }` |
| `VALIDATE_PATH_QUERY` | `validatePath(...)` |
| `NOTIFICATIONS_QUERY` | `notifications(...)` |
| `RECENT_NOTIFICATIONS_QUERY` | `recentNotifications(...)` |
| `NOTIFICATION_COUNTS_QUERY` | `notificationCounts { ... }` |
| `UNREAD_NOTIFICATION_COUNT_QUERY` | `unreadNotificationCount` |

### Mutations still using camelCase root fields

| Mutation Constant | camelCase root field |
|---|---|
| `ORGANIZE_TORRENT_MUTATION` | `organizeTorrent(...)` |
| `REMATCH_SOURCE_MUTATION` | `rematchSource(...)` |
| `PROCESS_SOURCE_MUTATION` | `processSource(...)` |
| `SET_MATCH_MUTATION` | `setMatch(...)` |
| `REMOVE_MATCH_MUTATION` | `removeMatch(...)` |
| `UPDATE_TORRENT_SETTINGS_MUTATION` | `updateTorrentSettings(...)` |
| `CREATE_LIBRARY_MUTATION` | `createLibrary(...)` |
| `CONSOLIDATE_LIBRARY_MUTATION` | `consolidateLibrary(...)` |
| `DOWNLOAD_EPISODE_MUTATION` | `downloadEpisode(...)` |
| `CLEAR_ALL_LOGS_MUTATION` | `clearAllLogs` |
| `CLEAR_OLD_LOGS_MUTATION` | `clearOldLogs(...)` |
| All RSS Feed mutations | `createRssFeed`, `updateRssFeed`, `deleteRssFeed`, `testRssFeed`, `pollRssFeed` |
| All Playback mutations | `startPlayback`, `updatePlayback`, `stopPlayback`, `updatePlaybackSettings` |
| `TRIGGER_AUTO_HUNT_MUTATION` | `triggerAutoHunt(...)` |
| All Notification mutations | `markNotificationRead`, `markAllNotificationsRead`, `resolveNotification`, `resolveNotificationWithAction`, `deleteNotification` |
| `MANUAL_MATCH_MUTATION` | `manualMatch(...)` |
| `UNMATCH_MEDIA_FILE_MUTATION` | `unmatchMediaFile(...)` |

### Subscriptions with mixed casing

| Subscription | Issue |
|---|---|
| `TORRENT_PROGRESS_SUBSCRIPTION` | Uses alias pattern `TorrentProgress: torrentProgress` — the backend field is still camelCase |
| `MEDIA_FILE_UPDATED_SUBSCRIPTION` | `mediaFileUpdated(...)` — fully camelCase |
| `DIRECTORY_CONTENTS_CHANGED_SUBSCRIPTION` | `directoryContentsChanged(...)` — fully camelCase |
| `NOTIFICATION_RECEIVED_SUBSCRIPTION` | `notificationReceived { ... }` — fully camelCase |
| `NOTIFICATION_COUNTS_SUBSCRIPTION` | `notificationCounts { ... }` — fully camelCase |
| `LOG_EVENTS_SUBSCRIPTION` | `logEvents(...)` — fully camelCase |

**Recommendation:** These all need backend resolver migration to PascalCase, then frontend updates. Track them in a migration checklist.

---

## 3. Data Fetching Pattern Inconsistencies

The codebase uses **four different approaches** to fetch GraphQL data:

### Pattern A: Codegen `TypedDocumentNode` from `.graphql` files (preferred)
Used by the detail routes (`movies/$movieId.tsx`, `shows/$showId.tsx`, `albums/$albumId.tsx`, `audiobooks/$audiobookId.tsx`, `downloads/index.tsx`) and newer components.

```typescript
import { MovieDetailRouteDocument } from "../../lib/graphql/generated/graphql";
const { data } = useQuery(MovieDetailRouteDocument, { variables: { Id: movieId } });
```

### Pattern B: Inline string queries with `gql()` wrapping
Used by `LibraryMoviesTab.tsx`, `LibraryShowsTab.tsx` — imports a string constant from `queries.ts` and wraps with `gql()`.

```typescript
import { MOVIES_CONNECTION_QUERY } from "../../lib/graphql";
const { data } = useQuery<MoviesConnectionResponse>(gql(MOVIES_CONNECTION_QUERY), { ... });
```

### Pattern C: Direct `apolloClient.query()` calls (imperative)
Used by `LibraryAlbumsTab.tsx`, `LibraryAudiobooksTab.tsx` — manually calls `apolloClient.query()` inside `useEffect` or callbacks, managing loading/error state by hand.

```typescript
const result = await apolloClient.query({ query: LibraryAlbumsTabDocument, variables: { ... } });
```

### Pattern D: Raw `fetch()` for REST endpoints
Used by `downloads/index.tsx` (file upload), `settings/index.tsx` (health), `settings/organization.tsx` (Ollama tags).

**Recommendation:** Standardize on Pattern A (codegen TypedDocumentNode + `useQuery`). Convert Pattern B and C files. Pattern D is acceptable only for non-GraphQL endpoints (file upload, health).

---

## 4. Raw HTML `<button>` Usage Instead of HeroUI `<Button>`

Found `<button>` elements in 8 files:

| File | Context |
|---|---|
| `routes/movies/$movieId.tsx` | Play overlay on poster image |
| `routes/shows/$showId.tsx` | Play overlay on poster image |
| `routes/albums/$albumId.tsx` | Play overlay on cover art |
| `routes/audiobooks/$audiobookId.tsx` | Play overlay on cover art |
| `components/downloads/TorrentTable.tsx` | Unknown context |
| `components/library/LibraryGridCard.tsx` | Unknown context |
| `components/library/MovieCard.tsx` | Unknown context |
| `components/filters/SearchInput.tsx` | Unknown context |

The play overlay buttons on poster images are all using this identical pattern:
```html
<button onClick={...} className="absolute inset-0 z-10 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 ...">
```

**Recommendation:** Extract a shared `<PosterPlayOverlay>` component that wraps a HeroUI `<Button>` with `isIconOnly`. This eliminates duplication and follows the HeroUI rule.

### Raw `<input>` usage

| File | Context |
|---|---|
| `components/downloads/AddTorrentModal.tsx` | File input (`<input type="file">`) |

This one is acceptable per project rules (HeroUI doesn't have a file input).

---

## 5. Missing `previousData` Usage (UI Flash Prevention)

**Files using `useQuery` WITHOUT `previousData`** (will flash to loading state on refetch):

| File | Queries without previousData |
|---|---|
| `components/library/LibraryMoviesTab.tsx` | Uses `gql(MOVIES_CONNECTION_QUERY)` with `useQuery` but no `previousData` |
| `components/library/LibraryShowsTab.tsx` | Uses `gql(TV_SHOWS_CONNECTION_QUERY)` with `useQuery` but no `previousData` |
| `components/library/LibraryTracksTab.tsx` | `useQuery` without `previousData` |
| `components/library/LibraryAuthorsTab.tsx` | `useQuery` without `previousData` |
| `components/library/LibraryArtistsTab.tsx` | `useQuery` without `previousData` |
| `routes/settings/torrent.tsx` | Manually manages loading, never reads `previousData` |
| `routes/settings/metadata.tsx` | Same pattern |
| `routes/settings/casting.tsx` | Same pattern |
| `routes/settings/logs.tsx` | Uses `apolloClient` directly, no previousData concept |
| `components/downloads/TorrentInfoModal.tsx` | `previousData` is used — good |
| `components/downloads/MediaFilesMatchDialog.tsx` | No previousData |
| `components/search/AddToLibraryModal.tsx` | No previousData |

**Files correctly using `previousData`** (good examples):
- `routes/movies/$movieId.tsx`
- `routes/shows/$showId.tsx`
- `routes/albums/$albumId.tsx`
- `routes/audiobooks/$audiobookId.tsx`
- `routes/downloads/index.tsx`
- `routes/libraries/index.tsx`
- `routes/notifications.tsx`
- `routes/settings/index.tsx`
- `components/NotificationPopover.tsx`
- `components/NotificationIcon.tsx`
- `components/DownloadIndicator.tsx`

**Recommendation:** Add `previousData` fallback to all `useQuery` calls that display data in lists/tables, particularly the Library*Tab components.

---

## 6. Delete/Confirm Modal Pattern Inconsistency

Three different patterns are used for delete confirmation:

### Pattern A: Dedicated `Delete*Modal` component (best)
Used by movies (`DeleteMovieModal`), libraries (`DeleteLibraryModal`), shows (`DeleteShowModal`).
```tsx
<DeleteMovieModal isOpen={isDeleteOpen} onClose={onDeleteClose} movie={movie} onDeleted={handleDeleted} />
```

### Pattern B: Inline `<Modal>` in the route file
Used by shows (`$showId.tsx`), albums (`$albumId.tsx`), audiobooks (`$audiobookId.tsx`).
```tsx
<Modal isOpen={isDeleteOpen} onClose={onDeleteClose}>
  <ModalContent>
    <ModalHeader>Delete Album</ModalHeader>
    <ModalBody>Are you sure?</ModalBody>
    <ModalFooter>
      <Button variant="flat" onPress={onDeleteClose}>Cancel</Button>
      <Button color="danger" onPress={handleDelete} isLoading={isDeleting}>Delete</Button>
    </ModalFooter>
  </ModalContent>
</Modal>
```

### Pattern C: Generic `<ConfirmModal>` component
Exists at `components/ConfirmModal.tsx` but is only used by `settings/logs.tsx`.

**Recommendation:** Adopt Pattern C (`ConfirmModal`) for all simple confirmations, and Pattern A (dedicated modal) for complex ones. Remove the inline Pattern B modals.

---

## 7. Modal State Management

All files consistently use `useDisclosure()` from HeroUI. No files use `useState(false)` for modal open/close. This is **good and consistent**.

However, there are two naming conventions:

- **Short form:** `const { isOpen, onOpen, onClose } = useDisclosure()` — used for single-modal pages
- **Prefixed form:** `const { isOpen: isDeleteOpen, onOpen: onDeleteOpen, onClose: onDeleteClose } = useDisclosure()` — used for multi-modal pages

Both are fine and contextually appropriate. No action needed.

---

## 8. Function Declaration Style

### Routes: Consistent — all use `function` declarations
Every route component and helper in `src/routes/` uses `function` declarations. There is one minor exception:

| File | Issue |
|---|---|
| `routes/notifications.tsx` line 148 | `const getNotificationIcon = (type) => { ... }` — arrow function |

### Components: Consistent — all exports use `function` declarations
Every exported component in `src/components/` uses `export function`. No `React.FC` or `export const Component = () =>` patterns found.

### Private helpers: Minor inconsistency
`components/NotificationPopover.tsx` and `components/NotificationDetailModal.tsx` define private helpers as arrow functions (`const getNotificationIcon = (...) => { ... }`), while route files define private helpers as function declarations.

**Recommendation:** Minor issue. If standardizing, prefer `function` declarations for all named helpers (better stack traces, hoisting).

---

## 9. Error Handling Patterns

### Toast-based error handling (consistent and good)
All routes and most components use `addToast()` from `@heroui/toast` for mutation errors. This is the correct pattern.

### `console.error` usage
32 files contain `console.error` calls. Most are paired with `addToast()` (log + notify), which is fine. A few files only `console.error` without user notification:

| File | Issue |
|---|---|
| `routes/settings/index.tsx` | Health check failure only logged, no toast |
| `routes/downloads/index.tsx` | `console.error(e)` in file upload catch |
| `contexts/PlaybackContext.tsx` | Multiple `console.error` without toast |

**Recommendation:** Ensure every catch block that affects user experience shows a toast, not just a console log.

### No `alert()` or `window.confirm()` usage — good

---

## 10. Areas Not Using DataTable

The codebase generally uses `DataTable` well. However, a few areas that render lists/tables could benefit from it:

| File | Current Pattern | Recommendation |
|---|---|---|
| `routes/settings/sources.tsx` | Uses HeroUI `Table` directly for indexer sources | Consider DataTable for consistency (search, filters) |
| `routes/settings/organization.tsx` | Large settings form — no table needed | N/A |
| `components/NotificationPopover.tsx` | Renders notification list with map | DataTable would be overkill here — fine as-is |

---

## 11. Duplicate Helper Functions

Several helper functions are duplicated across route files:

| Function | Duplicated In |
|---|---|
| `formatAudioCodec()` | `shows/$showId.tsx`, `albums/$albumId.tsx` (different implementations) |
| `formatVideoCodec()` | `shows/$showId.tsx` only |
| `formatAirDate()` | `shows/$showId.tsx`, `routes/index.tsx` (different implementations) |
| `getNotificationIcon()` | `NotificationPopover.tsx`, `NotificationDetailModal.tsx` |
| `formatTimestamp()` | `routes/notifications.tsx`, `routes/settings/logs.tsx` |

**Recommendation:** Extract these into `lib/format.ts` (which already exists and has `formatBytes`, `formatDuration`, `sanitizeError`, `formatDate`). Consolidate the duplicates.

---

## 12. React 19 Hook Opportunities

### `useActionState` (formerly `useFormState`)
Settings pages (`torrent.tsx`, `metadata.tsx`, `organization.tsx`, `casting.tsx`) manually manage `isSaving` state with `useState` + try/catch. These could use `useActionState` for cleaner form submission handling.

### `useOptimistic`
The wanted/unwanted toggle on movies, shows, albums, and audiobooks could use `useOptimistic` for instant UI feedback while the mutation is in flight, instead of waiting for refetch.

### `use()` for suspense-based data loading
Route `beforeLoad` could return promises resolved with `use()` instead of the current loading spinner pattern, enabling Suspense boundaries. This is a larger architectural change.

### `useTransition` for non-urgent updates
Tab switching in library views, filter changes, and search input could wrap state updates in `useTransition` to keep the UI responsive during re-renders.

---

## 13. Legacy `graphqlClient` Wrapper

`client.ts` contains a `graphqlClient` object that mimics urql's `.query().toPromise()` API. The comment says "Legacy wrapper for compatibility with existing code that uses urql-style API." However, a grep shows **zero components** currently import or use `graphqlClient` from their own code — the migration appears complete.

**Recommendation:** Remove the `graphqlClient` wrapper, the `globalThis` exposure, and the legacy comment. Keep only `apolloClient`, `queryPromise`, `mutationPromise`, and the Apollo hooks.

---

## 14. Inconsistent HeroUI Import Style

Some files import from the bundle package, others from individual packages:

```typescript
// Pattern A: Individual packages (most files)
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";

// Pattern B: Bundle package (a few files)
import { Button, Card, CardBody, Modal } from "@heroui/react";
```

Both work since the bundle is installed, but Pattern A is used in ~95% of files.

**Recommendation:** Standardize on Pattern A (individual packages) for tree-shaking benefits, even though the bundle is installed.

---

## 15. Inconsistent Page Container Classes

| Route | Container Class |
|---|---|
| `movies/$movieId.tsx` | `container mx-auto px-4 sm:px-6 lg:px-8 py-8 mb-20` |
| `shows/$showId.tsx` | `container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col mb-20` |
| `albums/$albumId.tsx` | `container mx-auto p-4` (no responsive padding, no mb-20) |
| `audiobooks/$audiobookId.tsx` | `container mx-auto p-4  mb-20` (double space typo) |
| `downloads/index.tsx` | `container mx-auto px-4 sm:px-6 lg:px-8 py-8 min-w-0 grow flex flex-col` |

**Recommendation:** Extract a shared `<PageContainer>` component or standardize on `container mx-auto px-4 sm:px-6 lg:px-8 py-8 mb-20`.

---

## Summary: Priority Ranking

| Priority | Issue | Scope |
|---|---|---|
| **P0 — Security** | Remove `globalThis` GraphQL client exposure | 1 file |
| **P1 — Consistency** | Migrate camelCase GraphQL resolvers to PascalCase | ~30 queries, ~25 mutations, ~6 subscriptions (backend + frontend) |
| **P1 — Consistency** | Standardize data fetching on codegen TypedDocumentNode | ~6 components using Pattern B/C |
| **P2 — UX** | Add `previousData` fallback to Library*Tab queries | ~6 components |
| **P2 — Consistency** | Extract shared `<PosterPlayOverlay>` component, remove raw `<button>` | 4 route files |
| **P2 — Consistency** | Standardize delete confirmation on `ConfirmModal` | 3 route files |
| **P2 — DRY** | Extract duplicate helpers to `lib/format.ts` | 5+ functions across 6 files |
| **P3 — Cleanup** | Remove legacy `graphqlClient` wrapper and `globalThis` | 1 file |
| **P3 — Cleanup** | Standardize page container classes | 5 route files |
| **P3 — Cleanup** | Fix double-space typo in audiobook container class | 1 file |
| **P3 — Modern** | Adopt React 19 `useOptimistic` for wanted toggles | 4 route files |
| **P3 — Modern** | Adopt React 19 `useActionState` for settings forms | 4 settings pages |
