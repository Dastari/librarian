# Frontend GraphQL and live-data audit / implementation plan

2026-10-24. Static inspection of the shared, already-dirty checkout. Research only; no application edits, schema export, code generation, servers, builds, tests, migrations, data writes, commits, PRs or browser work were performed. Only this new audit document was written. Findings describe inspected source and committed SDL, not a qualified running deployment. This document supplies the frontend portion of the parent backend/entity audit.

## Scope, counts and confidence

Repository rules consulted: `AGENTS.md`, `docs/design.md`, `.cursor/rules/frontend-ui.mdc`, `entity-single-source-of-truth.mdc`, `graphql-naming-convention.mdc`, `logging.mdc`, `media-pipeline.mdc`; product follow-ups from `docs/responsive-media-audit.md`. AGENTS' domain-SQL prohibition takes precedence over the older entity rule's SQL examples/exceptions. Backend follow-ups below must use generated entity reads/mutations or existing typed GraphQL-layer abstractions; action mutations remain legitimate for orchestration, provider/network/filesystem operations, playback and other non-CRUD work.

| Inventory | Statically established result | What it does / does not establish |
| --- | --- | --- |
| Production TS/TSX scan | 217 files, including 50 route modules; 88 feature, 50 component, 20 lib, 4 hook, 2 app modules, plus 3 source-root files | Every file read by the inventory scanner and searched for operations/transports/timers; not 217 manually line-reviewed implementations. Full production file list and matched line evidence appended below. |
| GraphQL source documents | 65: 46 entity files and 19 feature document files | All parsed as AST for roots, operation kinds and dependencies. |
| Operation definitions | 511 = 174 queries + 287 mutations + 50 subscriptions | Entity files: 92 queries, 230 mutations, 45 subscriptions; feature documents: 82 queries, 57 mutations, 5 subscriptions. Definitions are not mounted listeners. |
| Committed merged SDL roots | 115 Query, 293 Mutation, 50 Subscription fields | Parsed actual `Query`/`Mutation`/`Subscription`, not inferred from manual Rust impl blocks. No schema export in this audit. |
| Generator entity discovery | 46 connection entities; all 46 have List/Get and all 5 mutation forms in documents | No current missing list entity among committed-SDL discovery candidates. Torrent is missing its entity changed subscription, below. Read-only schema support is a generator-design gap, not 46 current missing roots. |
| Mounted subscription code | 19 `useSubscription(...)` call sites across 8 modules; 14 distinct documents | 10 distinct entity documents out of 45 available (22.2%); 4 of 5 custom subscriptions. 35 entity subscriptions and `FilesystemChanged` have no production listener. Route/skip/auth conditions determine actual runtime mounting; these are source-wiring counts, not network measurements. |
| Domain reload polling | Exactly 2 source sites: `LiveTorrents` 15s; `ActiveDownloadCount` 30s | Other timers are playback writes, session lifecycle, deployment updates or UI animation/debounce. No `startPolling`, `stopPolling`, `refetchInterval`, `subscribeToMore` or direct `client.subscribe` production use found. |
| Raw data transports | 0 production `gql` literals or `graphql(...)` strings; 0 raw GraphQL `fetch` in production TS/TSX; 1 explicit production `fetch`, media playback info | Auth uses generated GraphQL over cookies. Media/artwork/HLS are explicit media exceptions. Schema pull is a tooling-only GraphQL HTTP fetch. |
| Existing validation | Parent reports zero document validation errors against committed SDL | This audit parsed source; it did not execute validation tests or prove source/runtime SDL equivalence. Parent machine inventory: `schema-inventory.json`. |

The 19 call sites are Activity(1), ScansPanel(2), useLibraryScan(1), Wanted(5), CastingSettings(1), LogsSettings(1), useLiveTorrents(4), useShellCounts(4). `useShellCounts` is called by `NavRail.tsx:25`, `TopBar.tsx:27`, `BottomTabs.tsx:15`. All three shells are rendered by `AppShell.tsx:43`, `:48`, `:56`; CSS hiding does not unmount them. Thus each shell root can install three sets of the four hook listeners, with three callback paths per notification/lifecycle event. Apollo may deduplicate matching transport operations; that does not eliminate the three hook callbacks or prove only one refetch. The audit does not sum mutually exclusive route listeners into a fictitious simultaneous total.

## Verified findings

### F1 — Generated invalidations have no shared consumer (high)

`web/src/app/providers.tsx:20` installs Apollo and appearance/input providers; `web/src/routes/__root.tsx:23` renders the outlet; `web/src/components/shell/AppShell.tsx:24` installs shell/player UI. None mounts a global entity-subscription manager. The production scan found no such module or direct client subscription. `web/src/lib/graphql/__tests__/entity-subscriptions.test.ts:9` tests document payload shape only; its filename is not evidence of a mounted subscription mechanism.

`web/scripts/generate-entity-documents.mjs:238` generates invalidation metadata (`action`, `changeKind`, `id`, `sourceEntity`, `sourceId`, `path` at `:249`), not rich entities. An incoming metadata event cannot change a `Movie`/`Episode`/connection/count merely through normalization. It needs a registered consumer that re-queries authorized data and updates the owning result. Existing callbacks generally ignore event metadata and refresh a whole screen. Installing more generated documents alone fixes none of this.

### F2 — Parent propagation is absent; Torrent has a merged-root collision (high)

Pinned dependency evidence: `backend/Cargo.toml:57` pins graphql-orm **0.57.0**, commit `a90b5bd41f952b02416f23999c729b9ceffcbb8d`. All ORM claims here use `/root/.cargo/git/checkouts/graphql-orm-7d966b6f17beb3bd/a90b5bd/` (abbreviated **ORM/** below).

`ORM/crates/graphql-orm-macros/src/entity.rs:2663` defaults relation propagation to `none`; `ORM/crates/graphql-orm-macros/src/operations.rs:1667` only generates propagation for explicit `propagate_change = "up"`. Repository scan of every entity file found **zero** such declarations. Having a relation does not notify its parent. Actual child links include `episode.rs:126` (Show), `track.rs:119` (Album), `chapter.rs:94` (Audiobook), while parent collections are `show.rs:175`, `album.rs:158`, `audiobook.rs:162`, `library.rs:127`. Scalar foreign keys in `media_file.rs:24` and `playback_progress.rs:28` alone are not generated relations or notification edges.

`backend/src/services/graphql/schema.rs:13` glob-imports entities, but `:22` explicitly imports custom **TorrentSubscriptions**; `:44` lists Torrent among generated entities and `:78` adds the custom type. `ORM/crates/graphql-orm-macros/src/schema_roots.rs:263` names generated subscription types `<Entity>Subscriptions` using unqualified identifiers. The explicit custom import shadows the generated Torrent type in the macro invocation's scope. The committed SDL corroborates the missing `torrentChanged`: `web/schema.graphql:12034` has `chapterChanged`, followed by `:12044` `torrentFileChanged`, with four custom torrent fields elsewhere. This is a concrete naming-collision diagnosis from source + SDL; no build/export was run to requalify it. Alias/rename the custom type (e.g. `TorrentRuntimeSubscriptions`) and include both roots; export merged SDL, generate the missing document and verify it in later authorized implementation.

Until backend parent edges exist and their semantics are qualified, clients must register direct child dependencies too. Do not claim subscribing to `libraryChanged` alone covers content acquisition or `showChanged` alone covers episode additions.

### F3 — Reactive rows do not establish reactive collections (high)

`web/src/lib/apollo/typePolicies.ts:8` defines normalization and only a Query.me merge; there is no connection membership/filter/sort/count policy. Apollo entity writes can update fields on already referenced normalized objects. They cannot infer an inserted row's list placement, deletion, filter transition, total count, related-object result, or custom computed status.

* `useInfiniteConnection.ts:51` watches a head with a fixed limit of 60 (`:10`); appended pages are merged into that head at `:83`. `:95` invalidates pending append generation then calls head `result.refetch()`. With the inspected default policies, that replacement loses previously loaded windows. `useCompleteConnection.ts:24` drains again, adding full-reload traffic and transient incompleteness. These race guards are useful, but a global blanket `refetchQueries("active")` would regress their UX.
* `useSettingsConnection.ts:46` uses imperative network-only `client.query`; `:75` stores row objects in local React snapshots. There is no cache watch. `client.refetchQueries({include:"active"})` will not reliably find these completed one-shot requests or replace their snapshots. Its own `refetch` already stages all complete rows / the current infinite window atomically (`:59`–`:75`); retain and register this explicit callback. Scope key `:33` omits principal and client; add both/epoch cleanup so an ACL/user change cannot retain old rows.
* `useShowEpisodes.ts:13` watches a consolidated first-page operation; append `:34` is no-cache and writes the consolidated result at `:41`. Its dedicated refresh `:54` stages **every** episode off-cache and writes once at `:80`. Generic Apollo head refetch would bypass this behavior and discard later seasons. Register this hook's callback.
* `useAudioPages.ts:15` stores later track/chapter/artist-album/collection-movie pages in local `extra`; `:76` explicitly acknowledges imperative snapshots. Artist/collection continuation is no-cache (`ArtistPage.tsx:21`, `CollectionPage.tsx:21`); album/book continuation is network-only (`AlbumPage.tsx:99`, `AudiobookPage.tsx:100`). Updating normalized objects does not replace copied extra objects or list membership. The helper's local `updateRow` (`:78`) covers its initiating action, not changes from another client. First-page identity changes may cause repeated full continuation loads (`:60`); the shared controller must coalesce them.
* `useMediaProgress.ts:24` creates batches of observable queries and `:26` subscribes to those **query results**, not a GraphQL WS subscription. Existing progress updates normalize; newly created progress rows need connection membership re-query. No `EntityPlaybackProgressChanged` listener is mounted anywhere. Batch callbacks exist (`:25`, `:39`) and should be registered explicitly.
* `useContentStatuses.ts:45` similarly watches computed-query batches (500 inputs each), not server event streams. `activeContentStatusQueries` (`web/src/lib/graphql/content-status.ts:6`) finds active `ContentStatuses` watches; its callers (`MoviePage.tsx:51`, `EpisodeList.tsx:131`, `AlbumPage.tsx:108`, `AudiobookPage.tsx:112`) only cover their local mutations. There is no remote dependency invalidation. The normalized key `ContentStatusResult:{contentType,id}` at `typePolicies.ts:29` is separate from media entity IDs. Torrent/PendingFileMatch/MediaFile/quality-policy changes must invalidate relevant status batches even when the media row itself is untouched.

### F4 — Current callbacks can generate refresh storms and still miss dependencies (high)

`useWanted.ts:186` refetches 4–5 active complete-list sources plus four status hooks for **each** of five entity event streams (`:190`). None uses event id/path or a scheduler. Large scans/imports can repeatedly restart full lists and prevent a stable complete snapshot. It misses Show/Album/Audiobook label changes, PendingFileMatch/Torrent state changes and quality-rule changes unless another listened event happens.

`ScansPanel.tsx:61` and `:62` refresh entire lists for every run/issue event. `useLibraryScan.ts:10` listens to **run** changes only, though `LibraryScanState` selects both latest run and unresolved issue count (`libraries.graphql:110`, `:118`). Resolving an issue without a run update misses the badge. Activity and three shell copies listen to the same Notification document (`ActivityPage.tsx:52`, `useShellCounts.ts:25`). Logs auto-refresh `useSettingsConnection` on every AppLog event (`LogsSettings.tsx:48`), potentially re-reading the loaded window continuously. Retain manual refresh/retry and the user's Live switch, but coalesce server events.

### F5 — Reconnection and authority changes do not resynchronize state (high)

Transport: `web/src/lib/apollo/client.ts:27` creates a lazy graphql-ws client, infinite retry, 20s keepalive. `ConnectionBanner.tsx:17` toggles a banner on connect/close/error; no callback reconciles missed changes. ORM broadcasts are not a durable replay log: generated receivers use BroadcastStream and drop receive errors (`ORM/.../operations.rs:4992`). A reconnected socket can be healthy while lists/counts remain stale forever.

Session: `session.ts:94` loads `CurrentUser` once during boot, `:109` establishes local state, `:143` refreshes tokens/expiry but does not reload the user or call auth-change notification on success (`:148`). `useSession.ts:6` observes that external store; Apollo updates to MeUser/User do not update the store. `MeUser` and `User` are different GraphQL types, so `userChanged` normalization would not update `me` anyway. There is **no** mounted current-user/policy listener. Role demotion, disablement, ownership/ACL changes, or profile edits from another session can leave navigation, route access, snapshots and player state stale. Generic `EntityUserChanged` is admin-read-only (`entities/user.rs:20`), so it cannot be the member's policy-update channel.

Logout and expiry **do** clear Apollo (`session.ts:124`, `:173`) and terminate the socket via `client.ts:76`. Preserve those behaviors; also clear registered local snapshots, outstanding generation callbacks, player queue and policy-scoped state. `useContentStatuses` includes id/role in its scope (`:25`); Wanted has principal snapshots (`useWanted.ts:43`). These good guards do not add an ACL revision or cover all hooks. Login `establish` does not independently clear the old cache; use a principal epoch before authenticated queries start. `routes/_app.tsx:11` and `_player.tsx:7` check authentication on route entry, not continuous permission change; invalidate router loaders after session changes.

Backend WS identity is authenticated at upgrade/connection init (`service.rs:331`, `:352`). `policy.rs:44` and `row_policy.rs` use context AuthUser identity/role; do not assume a live socket sees a changed role. Generated ORM subscriptions reauthorize row/provenance/fields at dispatch (`ORM/.../operations.rs:5003`), but application policy still needs current authority. Custom torrent streams check only AuthUser presence at admission (`subscriptions/torrents.rs:76`, `:134`, `:184`, `:234`), forwarding the shared stream without owner checks or current-session revocation. Secure those runtime subscriptions before using them as the authoritative replacement for polling. This is a static exposure finding, not a demonstrated exploit.

### F6 — Transport retry currently replays mutations (high)

`web/src/lib/apollo/client.ts:39` configures RetryLink for 4 attempts; `:43` accepts any truthy non-unauthorized error. `:63` puts it before the HTTP transport for **queries and mutations**, with no operation-kind guard. If a request executes and its response is lost, an unguarded retry can replay `AddTorrent`, provider-add actions, scan/processing requests, backups/restores, invite creation, file move/copy/delete/rename and generated creates. Concrete action documents: `torrents.graphql:75`, `backup.graphql:22`, `:46`, `files.graphql:1`, `:21`, `libraries.graphql:125`, `search.graphql:202`. Their replay safety is not established by this frontend audit.

First implementation change: automatic network RetryLink retry only when `getMainDefinition(operation.query)` is a QUERY; preserve visible error and explicit retry UX for mutations. Permit mutation retry only after a specific action has documented idempotency/deduplication. Do not add a new action framework merely to address this link bug. `sessionLink` (`:48`) separately retries after an authorization error; ensure the backend guarantees authorization failure precedes side effects before retaining mutation replay there. Never conflate network failure with proof that a mutation did not run.

### F7 — Codegen largely follows the contract; harden discovery and security regression checks

`web/codegen.ts:8` reads the committed snapshot and only the two GraphQL directories, generating typed document nodes (`:13`) with fragment masking disabled (`:15`) and explicit scalar mapping. All audited production operations import generated nodes; hand-authored **.graphql composition** of nested selections remains typed/codegenerated and is not a manual untyped transport defect.

`web/scripts/pull-schema.mjs:16` performs tooling introspection over HTTP; production disables introspection at `backend/.../schema.rs:115`. Builds should not depend on a running/introspectable service. CI already exports merged SDL through `schema_builds_without_recursing` (`.github/workflows/ci.yml:77`, backend `schema.rs:233`/`:242`), regenerates (`ci.yml:83`) and diffs committed SDL and entity documents (`:86`). Generated TypeScript is deliberately ignored (`web/.gitignore:9`), so omitting it from `git diff` is expected, not an absent drift gate. CI frontend build typechecks regenerated output. No runtime/source drift was established by offline operation validation alone.

Generator issues and specific fixes:

* `generate-entity-documents.mjs:40` requires Query **and** Mutation **and** Subscription. Support absent mutation/subscription roots for read-only schemas while generating available queries; do not invent writable operations.
* Discovery requires `*Connection`, `where`, `page` (`:63`); comments promise `orderBy`, code tolerates its absence (`:86`). Build an explicit coverage manifest from merged schema roots plus backend entity registry, classify unsupported/read-only/custom roots, and fail on unexplained omissions. Do not test coverage via `documents.length > 45` (`lib/__tests__/entity-documents.test.ts:14`).
* CRUD and changed fields are conditionally detected (`:153`, `:162`, `:182`, `:202`, `:214`, `:226`, `:238`). Verify operation names/arg types exactly; 46 present entities have full CRUD documents, but Torrent lacks changed. Subscription suffixes must come from actual roots, not assumed derive existence.
* Broad fragments select every exposed leaf (`:114`). Private hashes/password/token/credentials are currently absent from relevant output fragments: User password_hash (`user.rs:38`), RefreshToken token_hash/session/roles (`refresh_token.rs:29`, `:44`), InviteToken token (`invite_token.rs:24`), UsenetServer password/nonce (`usenet_server.rs:49`), Source credentials (`source.rs:101`). Retain schema-level privacy and add denylisted-field assertions over output types/fragments and where/order inputs. AppSetting `value` (`app_setting.rs:33`) is inherently sensitive for some keys and admin-only (`:18`); do not mount an admin settings fragment for ordinary members or reintroduce secret-valued events. A generator leaf whitelist cannot substitute for backend policy.
* Generated changed documents select only invalidation metadata, consistent with pinned ORM selection admission (`ORM/crates/graphql-orm/src/graphql/subscriptions.rs:340`). Do not add full rich fragments to events to save refetches.
* Snapshot root coverage is complete except unused custom query `showcaseArtwork` and six unrepresented mutations: `pauseTorrentByInfoHash`, `resumeTorrentByInfoHash`, `removeTorrentByInfoHash`, `markAllNotificationsRead`, `upsertShow`, `upsertEpisode`. Missing documents are not necessarily missing product features; classify intentional/internal/custom surfaces. All 50 available subscription roots have documents, only 14 distinct documents are wired into production listeners.
* `codegen:watch` runs graphql-codegen directly (`package.json` scripts), bypassing entity-document regeneration. Watch SDL changes through the entity generator too, or document/use the combined command. Add a read-only check mode that computes output without deleting/recreating the directory (`generator:99`) for audit/CI use.

### F8 — Polling and REST exceptions need precise boundaries

| Mechanism and evidence | Current cadence | Classification | Target policy |
| --- | --- | --- | --- |
| `features/downloads/useLiveTorrents.ts:22` | 15s | Runtime torrent snapshot polling; domain rows merged alongside runtime data | Remove default poll after lifecycle/progress subscription + reconnect reconciliation is qualified. If a nonpersisted external runtime source genuinely lacks push, allow a named, visible-only, documented exception with reason/expiry/backoff; current torrent runtime already supplies push and is not justified as a permanent exception. |
| `components/shell/useShellCounts.ts:18` | 30s | Domain/navigation aggregate reload; three shell instances | Remove. Refresh count on lifecycle plus state-transition events and reconnect. Pause/resume must change active count; add/complete/remove alone is insufficient. |
| `features/player/useProgressSync.ts:107` | Wake every 5s; writes gated by server interval, default15s/min5s (`:34`, `:56`) | Client playback state checkpoint writes, **not a query poll** | Keep bounded persistence and pause/pagehide flush; single flight and cross-device write ordering should be qualified. |
| `lib/auth/session.ts:186` | Access expiry−90s, min5s; outage retry30s (`:162`) | Cookie/session security lifecycle | Keep, plus auth-change push/revalidation; not a domain-query polling exemption. |
| `lib/pwa.ts:12` | 1h | Deployment/service-worker update discovery | Keep; own cleanup if provider can remount. Not persisted domain data. |
| `lib/apollo/client.ts:30`, `:33`, `:40` | WS lazy close10s/keepalive20s; HTTP retry400ms→6s | Transport lifecycle | Keep keepalive/reconnect, constrain retries as F6. |
| `hooks/useDebouncedValue.ts:6` | Caller delay; search default inspected at consumers | One-shot input debounce | Keep. Does not reload without input. |
| `features/player/VideoPlayer.tsx:76` | UI controls hide delay | One-shot presentation timer | Keep. |
| `components/constellation/FilmConstellation.tsx:262`, `:275`, `:297`, `:301` | RAF; static repaint400ms / clear8s | Animation/presentation | Keep. |
| `features/libraries/browser/useLetterJump.ts:43`, `:56` | One RAF per jump | Scroll focus scheduling | Keep. |
| `features/home/TvGuide.tsx:52`, `useCompleteConnection.ts:24`, `useAudioPages.ts:45`, `useShowEpisodes.ts:96` | On page/result change, until loaded | Pagination/effect continuation, no recurring timer | Preserve but single-flight and avoid automatic infinite retry on an unchanged failure. |
| Backend `services/torrent/service.rs:1069`, `:1127` | Runtime sampling1s, DB synchronization10s | **Server** sampling/persistence, not frontend reload | Keep backend responsibility; emit authorized events after persisted state updates. More backend loops exist; this row does not claim a full backend scheduler audit. |

`web/src/lib/api/urls.ts:61` is the sole explicit production fetch: `/api/media/:id/info`, once for resolving playback suitability/HLS (`:58`); stream URL `:23`, artwork `:10`. Player/HLS/media-element requests are media bytes/manifests, not domain-table REST reads. Move stable domain metadata into generated GraphQL fields when practical; retain media-runtime suitability/stream negotiation as an explicit media exception. No custom REST auth fetch was found: `session.ts:97`, `:117`, `:146` use generated GraphQL. No `axios`, `XMLHttpRequest`, `EventSource`, raw `new WebSocket`, raw production gql, or untyped GraphQL fetch was found outside generated code. The schema pull script's fetch is tooling, not application data access.

## Route / feature family implementation matrix

Names below are document operation names, not SDL field names; all have generated typed nodes. `E(X)` means existing `EntityXChangedDocument` **must be mounted through the shared registry** (except Torrent, whose root/document must first be restored). It does not imply that listener exists today. Every row has polling **none** unless shown. Child/parent arrows in the target column are required backend notifications, currently unqualified/absent per F2; retain direct-dependency fallback until implemented. Query document AST inventory and exact caller lines follow in appendices.

| Route or feature | Query operations / entity dependencies | Listeners actually wired here | Poll | Target invalidation and missed notifications |
| --- | --- | --- | --- | --- |
| Shared navigation/shell | `NavLibraries`, `ActiveDownloadCount`, `UnreadNotificationCount`; Library, Torrent runtime/persisted, Notification; session MeUser | Notification; TorrentAdded/Completed/Removed, repeated in three shell hook instances | count30s | Single shell state owner. E(Library), E(Notification), restored E(Torrent) and runtime state-transition handling; reconcile count on pause/resume and reconnect. Library rename/create/delete/reorder/ACL must replace navigation membership. |
| `/` Home shelves | `LibrariesOverview`, `RecentMovies`, `RecentShows`, `RecentAlbums`, `RecentAudiobooks`; Library + nested counts and movie/show/album/book/mediafile previews | None | — | E(Library/Movie/Show/Album/Audiobook/MediaFile); propagate content→Library; re-query affected top-N shelf and counts, not every library. Full library shelf pagination if all libraries are promised. |
| Home Continue Watching | `ContinueWatching`, `MediaFilesByIds`, `MoviesByIds`, `EpisodesByIds`, `TracksByIds`, `ChaptersByIds`; PlaybackProgress + media rows + Show/Album/Audiobook grouping | None | — | E(PlaybackProgress) is essential for first insertion, watched removal and updatedAt order; re-query top12, then only changed ID batches. E(MediaFile/Movie/Episode/Track/Chapter/Show/Album/Audiobook) refresh links/artwork/group identity. Preserve per-title dedupe; do not patch just existing progress field values. |
| Home TV guide | `UpcomingLibraryEpisodes`, `NextLibraryEpisodes`, `ScheduleWeek`, `ScheduleCountries`; Episode/Show, ScheduleCache, ScheduleSyncState | None | — | E(Episode/Show/ScheduleCache), admin-safe schedule sync/completion signal. Reconcile country/week pages once per successful import batch. Clock rollover is local date state, not domain polling. Current date fixed at mount (`TvGuide.tsx:78`); schedule local midnight transition + wake check. |
| Home theatrical release guide | `MovieReleaseGuide`, `LibraryMoviesByTmdbIds`, `LibrariesOverview`; external TMDB cached discovery + Movie/Library ownership | None; own add refetches ownership query (`MovieReleases.tsx:66`) | — | E(Movie/Library) for owned badges/pickers. Cached external release query refresh on explicit request, cache invalidation signal or foreground staleness; no default recurring reload. Keep region/page cache key. |
| `/libraries`, settings library list | `LibrariesOverview` via complete connection / settings snapshot; Library + nested child counts | None | — | E(Library) and child-parent notifications, register exact snapshot refresh; preserve last complete data. |
| `/libraries/$id` layout/index/settings | `LibraryDetail` route loader, `LibraryScanState`; Library + nested counts, scan run/issue; form `NamingPatternsForType`, `QualityProfilesList` | useLibraryScan run events only | — | E(Library/LibraryScanRun/LibraryScanIssue/NamingPattern/QualityProfile), plus child→Library counts. Invalidate route loader context, not just an independent query. Issue changes must refresh unresolved badge. |
| Library movies | `LibraryMovies`, ContentStatuses; Movie/Library/MediaFile plus status dependencies | None | — | E(Movie/MediaFile), parent→Library, collection membership changes, status dependencies; refresh loaded window atomically with where/order/filter/count. |
| Library shows | `LibraryShows`; Show/Library + nested episode counts | None | — | E(Show/Episode/MediaFile), Episode→Show→Library. Parent event must re-query child count and available-file badges. |
| Library collections | `LibraryCollections`; Collection/Library/Movie | None | — | E(Collection/Movie), Movie→Collection/Library including **old and new** collection on reassignment. Preserve collection movie counts and cover selection. |
| Library albums | `LibraryAlbums`; Album/Artist/Track/Library | None | — | E(Album/Artist/Track/MediaFile), Track→Album→Artist/Library. Child additions and file linkage affect counts/availability. |
| Library artists | `LibraryArtists`; Artist/Library, computed album/track counts | None | — | E(Artist/Album/Track); Album→Artist and library, refresh server filter/order and aggregates. |
| Library tracks | `LibraryTracks`, ContentStatuses; Track/Album/Artist/MediaFile | None | — | E(Track/Album/Artist/MediaFile) + status dependencies; update existing normalized rows and replace membership/count/order as necessary. |
| Library audiobooks | `LibraryAudiobooks`; Audiobook/Library, nested Chapter counts | None | — | E(Audiobook/Chapter/MediaFile), Chapter→Audiobook→Library; complete counts and playback suitability. |
| Library files | `LibraryMediaFiles`; MediaFile + movie/episode/show/track/album/chapter/book relations | None | — | E(MediaFile) and linked content changes; owner/link reassignment invalidates both old/new titles/library. Analysis/quality/path changes must refresh affected table predicate/count. |
| Library scans | EntityLibraryScanRunList, EntityLibraryScanIssueList; LibraryScanRun/Issue | Run + Issue unfiltered callbacks | — | Shared coalesced E(Run/Issue), scope by result IDs/library mapping or future server filter; parent badge notification. Preserve loaded run/issue windows and selections. |
| `/movies/$id` | `MovieDetail`, `MovieCastCredits`, `MovieCollectionDetails`, `PlaybackProgressForFile`, ContentStatuses; Movie/Library/MediaFile/Person/MovieCastCredit/collection data/progress | None | — | E(Movie/MediaFile/MovieCastCredit/Person/PlaybackProgress/Collection/Library) + status dependencies. Credit/person changes→Movie; movie collection changes refresh owned/missing collection parts. |
| `/shows/$id` | `ShowDetail`, `ShowEpisodes`, `ShowEpisodesPage`, batched `PlaybackProgressForFiles`, ContentStatuses | None | — | E(Show/Episode/MediaFile/PlaybackProgress/Library) + status dependencies. Register useShowEpisodes atomic callback, not generic head refetch. Episode→Show for title totals; keep loaded seasons/focus. |
| `/albums/$id` | `AlbumDetail`, `AlbumTracksPage`, ContentStatuses; Album/Artist/Track/MediaFile/Library | None | — | E(Album/Artist/Track/MediaFile/Library) + status dependencies. Replace complete useAudioPages snapshot; refresh first and extra rows without clearing queue/focus. |
| `/artists/$id` | `ArtistDetail` paged albums; Artist/Album/Library + count aggregates | None | — | E(Artist/Album/Track), old/new parent IDs; atomic continuation refresh retaining visible rows on errors. |
| `/audiobooks/$id` | `AudiobookDetail`, `AudiobookChaptersPage`, `PlaybackProgressForFiles`, ContentStatuses; Audiobook/Chapter/MediaFile/Library/progress | None | — | E(Audiobook/Chapter/MediaFile/PlaybackProgress/Library) + status dependencies; Chapter→Audiobook; refresh progress membership and complete chapter snapshot separately. |
| `/collections/$id` | `CollectionDetail`, `MovieCollectionDetails`; Collection/Movie/Library + external metadata | None | — | E(Collection/Movie/MediaFile), Movie→old/new collection, refresh provider-owned overlays only when membership changes. Continue all pages atomically. |
| `/wanted` all tabs | WantedListEpisodes/Movies/Tracks/Chapters, UpgradableFiles, ContentStatuses; Show/Album/Audiobook label parents, PendingFileMatch, Torrent, quality decisions | E(Episode/Movie/Track/Chapter/MediaFile) | — | Add E(Show/Album/Audiobook/PendingFileMatch/Torrent/QualityProfile/Library), relevant app quality setting invalidation via authorized channel. Coalesce complete snapshots and only relevant status batches; do not reload all four lists per unrelated event. |
| `/downloads` | `LiveTorrents`, EntityTorrentList; live runtime + persisted torrent linkage/import status | TorrentProgress modifies existing cache; Added/Completed/Removed refresh both sources | live15s | Restore E(Torrent); E(TorrentFile/PendingFileMatch/MediaFile) for processing outcomes where selected; lifecycle/snapshot resync and owner-safe progress stream. Existing cache.modify cannot insert unknown live torrent membership. |
| `/activity` | EntityNotificationList | E(Notification) | — | Share event with shell, coalesce loaded-window refresh incl read/unread filter and count. Quality approval side effects need media/quality events too. |
| `/search` local | `LibrariesOverview`, `LibrarySearch`; Movie/Show/Album/Artist/Audiobook plus library ownership | None | — | Dependency registry per active needle/library scope; E(content/Library) re-query affected top20 lists on persisted title changes. Debounce input remains; bounded results must be explicit, not locally filtered pretend-complete data. |
| Provider search / Add dialogs | SearchMovies/TvShows/Albums/Audiobooks/MovieCollections; LibrariesOverview/pickers; external provider results + owned checks | None | — | Remote results stay demand-driven; E(Library/content) updates picker/membership. Keep custom provider-add action mutations; generated CRUD is not a metadata-import replacement. |
| Acquisition/release/link dialogs | EntityLibraryGet, QualityProfilesList, EntitySourceList, SearchSources, EntityLibraryList/NavLibraries; Library, QualityProfile, Source, Torrent plus remote releases | None | — | Mount dependencies only while open; E(Library/QualityProfile/Source/Torrent). Preserve current search results; show changed eligibility/partial data rather than rerun all indexers per event. |
| `/settings/general` | AppSettingsByCategory playback/metadata/subtitles; local appearance prefs | None | — | Admin-only E(AppSetting), member-safe public playback defaults channel; do not issue admin setting queries solely because controls are hidden. Handle incoming changes without resetting dirty forms (`GeneralSettings.tsx:31`). |
| `/settings/metadata` | AppSettingsByCategory; provider/LLM test action mutations | None | — | E(AppSetting) by category/key, secret redaction; preserve dirty editing and explicit provider tests. |
| `/settings/sources` | EntitySourceList; AvailableSourceDefinitions, SourceSettingDefinitions in SourceDialog | None | — | E(Source/SourcePriorityRule) for order/enabled/config changes. Definitions are deployment-static until reload; don't poll. Register local complete snapshot callback. |
| `/settings/quality` and BlockedReleases | EntityQualityProfileList, EntityReleaseBlocklistList | None | — | E(QualityProfile/ReleaseBlocklist); policy changes trigger computed status/affected library invalidation; backend status recompute events after completion. |
| `/settings/organization` | EntityNamingPatternList + AppSettingsByCategory | None | — | E(NamingPattern/AppSetting), refresh complete snapshot; default changes via qualified action/CRUD path with consistent notifications. |
| `/settings/downloads` | AppSettingsByCategory + triggerAutoDownload action | None | — | E(AppSetting), auto-download outcomes use content/Torrent/PendingFileMatch notifications. Do not make a settings timer into hunt-result polling. |
| `/settings/casting` | EntityCastDeviceList, EntityCastSettingList; runtime discovery actions | E(CastDevice) only | — | E(CastDevice/CastSetting/CastSession) as consumed; current CastSetting query limit1 needs an explicit singleton contract. Device runtime discovery explicit; no unbounded polling. |
| `/settings/users` | EntityUserList, EntityInviteTokenList, EntityLibraryList | None | — | Admin E(User/InviteToken/Library); separate current-user policy invalidation channel for ordinary members. Disable/demotion clears privileged snapshots/route context immediately; list events alone do not refresh session store. |
| `/settings/backup` | BackupOverview; snapshot storage/runtime summaries | None | — | Add explicit authorized backup lifecycle invalidation/snapshot completion signal if cross-client updates matter; refresh on own completed actions and reconnect. Preserve custom backup/restore actions; don't invent entity CRUD for bytes. |
| `/settings/logs` | EntityAppLogList, incremental settings hook | E(AppLog) when Live enabled | — | Coalesced window refresh with bounded flush frequency; disabled Live remains a user choice. Do not make every log write re-read thousands of loaded rows. |
| `/settings/about`, settings redirects/layouts | Static application metadata / session role | None | — | No domain polling/subscription needed; role-aware routes must re-evaluate on session epoch change. |
| `/watch/$mediaFileId`, persistent audio player | EntityMediaFileGet, MovieDetail/EpisodeDetail, MediaFileStreams, PlaybackProgressForFile, PlaybackSyncInterval; MediaFile/content/video/audio/subtitle/chapter/progress/settings | None | 5s checkpoint writes only | E(MediaFile/content/streams/PlaybackProgress); don't jump active playback on remote progress. Update resume candidates, availability and track menus; safety/access invalidation stops unauthorized playback and clears queues. MediaInfo fetch/HLS remain media exceptions. |
| Folder picker | BrowseDirectory, filesystem action documents available | FilesystemChanged **not mounted** | — | Mount filesystemChanged only while browsing affected path, refresh on path changes/user request, preserve focus. External filesystem runtime may use approved reasoned exception only if watcher unavailable. |
| Login/register | NeedsSetup route query, Login/Register actions, CurrentUser/RefreshSession/Logout session operations | None | Session renewal only | Authenticate via cookies; shared live-data owner starts after valid principal, terminates on logout; no admin User entity subscription for members. |

## Required backend notification graph for the client plan

These are proposed dependency edges, not claims they already fire. Add explicit typed relations + `propagate_change = "up"` where ownership/semantics fit the pinned ORM. For computed cross-entity results that lack a valid relation, add a typed authorized invalidation contract; do not fabricate scalar-FK propagation or leak owner-specific progress into shared titles.

| Origin | Required affected results / parent notifications | Existing source evidence and implementation constraint |
| --- | --- | --- |
| Episode create/update/delete/reparent | old/new Show, old/new Library; seasons/count/guide rows/status | `episode.rs:126`, `show.rs:175`, `library.rs:127`; no propagation opted in. Qualify delete preimages and both parents on reassign. |
| Track | old/new Album and Artist, Library; queue/count/status | `track.rs:119`, `:123`, `album.rs:158`; avoid cycles/duplicate transitive events. |
| Chapter | old/new Audiobook and Library; chapter counts/resume | `chapter.rs:94`, `audiobook.rs:162`; preserve owner authorization. |
| Movie | Library + old/new Collection, collection ownership overlays | `movie.rs:195`, collection fields in generated fragment; membership may be provider ID based, so add a real relation/typed matching invalidation rather than assuming IDs align. |
| Album/Artist/Audiobook/Show | Library and their selected parents' aggregates | Existing parent relations, no up propagation. Parent rename/artwork also changes nested consumers even when children are untouched. |
| MediaFile link/unlink/analyze/quality/delete | old/new Library, Movie/Episode/Track/Chapter, their parents; availability/status/stream metadata | `media_file.rs:24`–`:42` are scalar FKs with no relation metadata. Add typed dependencies and ensure actions publish after state commit. |
| VideoStream/AudioStream/Subtitle/MediaChapter | MediaFile detail + playback menus | `media.graphql:34` selects four connection roots; create/delete require membership refresh, not just normalized row updates. |
| MovieCastCredit/Person | Movie cast + artwork/name; person update fans out only to authorized titles | `media.graphql:16`; distinguish relation propagation direction from query nesting. |
| PlaybackProgress | owner's ContinueWatching membership/order, progress batches, title grouping | `playback_progress.rs:23` owner; `:28` scalar media FK only. Prefer owner-scoped progress listener and explicit joins on refetch, not cross-owner parent broadcasts. |
| PlaybackSession/CastSession/CastDevice/CastSetting | current sessions/runtime actions/device menus | Keep private session ownership and shared device policy distinct. ActiveCastSessions document exists without production consumer. |
| Torrent/TorrentFile/PendingFileMatch | persisted download outcomes + contentStatuses / Wanted, counts on state transition | Restore torrentChanged merged root. Runtime progress alone does not carry persisted processing/linkage fields. |
| QualityProfile/quality settings/Library policy/ReleaseBlocklist | affected content statuses, wanted/upgrade subsets and quality badges | Invalidate when policy changes and on recompute completion. Simple entity normalization leaves ContentStatusResult stale. |
| LibraryScanRun/LibraryScanIssue | library latest-run/badge + run/issue windows | `libraries.graphql:110`; issue changes currently missed by useLibraryScan. |
| User/ACL/session revoke | member-safe current-session policy update; owner/ACL revision + cache/snapshot epoch | User root is admin-only. Add a current-session invalidation surface using existing auth mechanisms where possible, then authorized `me` reload. Do not broaden User entity read policy merely to power navigation. |
| ScheduleCache/ScheduleSyncState | guide week/country list; one completion invalidation per import window | ScheduleSyncState is admin-scoped; public discovery needs safe metadata, not admin listener errors. |
| AppSetting/Source/NamingPattern/Notification/AppLog/backup | corresponding settings, badges, runtime summaries | Separate static definitions from persisted state; notifications/list count share scheduler; logs throttle; backup lifecycle is an action signal. |

Pinned ORM limits matter: `ORM/crates/graphql-orm/src/graphql/subscriptions.rs:15` bounds propagation; `operations.rs:5003` authorizes provenance and deletion preimages; `schema_roots.rs:263` merges generated subscriptions. Qualify multi-hop fan-out, reassignment and deletion against this pin during implementation. No polling removal can compensate for services bypassing event-producing mutations; parent backend audit must close those paths first.

## Shared typed subscription / invalidation implementation plan

1. **Restore the schema contract, then regenerate.** Rename/alias custom TorrentSubscriptions to retain generated `torrentChanged`. Add tested parent edges and typed missing dependencies in the entity layer. Export merged SDL through the existing schema test path; update `schema.graphql`, run generator/codegen later under implementation authorization. Keep action mutations where actual orchestration is needed. Fix query-only RetryLink independently before changing refresh behavior.
2. **One principal-scoped live-data owner.** Mount inside Apollo but above both authenticated shell and immersive player routes. Own it by authenticated principal + authorization revision + client instance, not just component lifetime. Subscribe lazily to required entity families using generated TypedDocumentNodes, reference-count subscribers and share shell counts. Never subscribe every account to all 45 entities: admin-only entities, source secrets and current-user policy use explicit capability gates. Startup may attach owner-safe entity streams with only the available SubscriptionFilterInput; don't invent arbitrary `libraryId`/`userId` filters. The inspected generated filter only supports action filtering (and an invalid dummy field), so client scoping uses registered dependencies/known IDs and authorized query scopes, unless backend adds a real typed filter.
3. **A typed dependency registry, including non-observable results.** Register operation identity + canonical variables + principal epoch + dependency families/known IDs/parent IDs + adapter refresh callback + result kind (`detail`, `window`, `complete`, `aggregate`, `computed`, `runtime`). Derive selected object dependencies from AST/schema in tooling, but keep explicit overrides for computed status, owner joins, custom search, counts and provider overlays. Normalization cannot infer these. Register useSettingsConnection/useAudioPages/useShowEpisodes/useMediaProgress callbacks directly; watching Apollo's active query list alone misses them. `ContentStatuses` registration maps inputs to concrete media IDs and status dependency classes; refresh only intersecting batches.
4. **Normalize event envelopes and schedule bounded work.** Use generated event metadata union (`action/changeKind/id/sourceEntity/sourceId/path`), distinguish entity changed vs custom torrent runtime events. Within a short bounded window, merge dirty dataset IDs/families; one refresh in flight per dataset with at most one dirty rerun. Bound latency during an endless stream (e.g. flush <=500ms, log windows <=1–2s) instead of trailing debounce that can starve. Deduplicate source + propagated parent effects in the same batch; there is no current event sequence/transaction ID, so don't claim exact dedupe across reconnects. Add server revision/correlation only if needed for a concrete race/replay guarantee.
5. **Correct cache effects.** An event metadata payload triggers authorized reads. Upsert returned entities into normalization; delete authorized known references; refetch affected membership/order/count for CREATE/DELETE/filter/sort/reparent. Metadata lacks old values and row fields, so new/unseen IDs must conservatively dirty candidate datasets of that family. Never skip a CREATE just because its ID isn't already loaded. If old parent is unavailable, dirty candidate old scopes until backend event provenance includes sufficient safe information. Recompute status through ContentStatuses, not by locally guessing pipeline rules. Keep custom query result types and User/MeUser aliases explicit.
6. **Preserve infinite windows.** Extend useInfiniteConnection with a dedicated live refresh adapter: freeze append, record raw loaded edge/window size and generation, fetch fresh pages off-cache up to that loaded extent (respect backend max page limit) with identical where/order plus stable ID tiebreaker; stage atomically, then swap pageInfo/rows, clamp deleted tails, resume append from new raw offsets. Retain focus/selection for still-present IDs in the same dataset and preserve scroll anchor; show prior rows on refresh failure. Dataset search/filter/sort changes intentionally reset. Use existing useSettingsConnection window staging and useShowEpisodes atomic revalidation as patterns. Complete collections re-read to completion before enabling local filter/group actions. Offset scans may race with concurrent writes; add a dataset revision/snapshot contract or bounded restart if a consistent window is required, not a silent claim of consistency. Preserve append/refresh generation guards and failed-page Retry.
7. **Recovery is event-driven.** On first subscription establishment/read handshake avoid query-before-subscribe gaps; on reconnect, visibility/network wake after an outage, lost stream or authority revision, reconcile each active registered dataset once. With current in-memory broadcasts there is no resume cursor: require a snapshot resync, not replay assumptions. Cancel stale epoch requests; after each adapter completes, rerun once if events arrived while loading. Show stale/reconnecting state until reconciliation succeeds. If adding durable versions later, handle gap detection explicitly.
8. **Session/policy isolation.** Add a member-safe current-user/session invalidation stream or existing auth event bridge; on signal, reload authorized `me`, update SessionStore, renew/reconnect WS authority and invalidate router context. Role/ACL changes advance epoch and evict privileged data/local snapshots before re-render, clear player/queues when access is lost, and re-register authorized listeners. Logout/expiry aborts inflight reads and rejects old responses before clearing stores. Cross-tab signaling carries only state/epoch, never credentials. Don't rely on UserChanged to a member or a static socket AuthUser as revocation enforcement.
9. **Remove recurring domain polling.** Delete the 30s navigation and 15s download query poll only after appropriate lifecycle/state/persisted events and reconnect snapshots are covered. Runtime exceptions require an explicit nonpersisted external source lacking push, a recorded reason and owner, only while visible/needed, bounded cadence/backoff and a removal condition. The two current sites should not be grandfathered as approved exceptions. Keep playback checkpoint writes, auth renewal, WS keepalive and PWA deployment checks.
10. **Proportional later qualification.** Source/AST contract checks for every exported entity/root, private fields and mounted registry declarations; transport-level mutation retry guard; deterministic hook tests for CREATE/DELETE/filter/sort/count, parent propagation/reparent and status invalidation; full-window retention past >60/>100 rows, complete snapshots >500 statuses, event bursts and refresh/append races; subscription handshake/reconnect gap, logout/ACL epoch, newly created progress membership, pause/resume counts and admin/member listener admission. These are future acceptance checks, not results from this research turn.

## Responsive-media follow-ups: frontend/API integration contracts

All gaps explicitly identified in `docs/responsive-media-audit.md` are accounted for here. Parent audit owns exhaustive backend implementation design; this section identifies the frontend query/event contract so those features do not add polling or separate REST CRUD.

| Gap / priority | Required typed API and client behavior | Invalidation / acceptance dependencies |
| --- | --- | --- |
| Saved filters / curated playlists / smart collections (P1) | New owned saved-rule/playlist entities with validated typed predicates, stable server sort/pagination and explicit manual vs smart membership; editor uses generated documents | Rule/membership/media changes notify collection and affected home shelf counts; refresh loaded window, never filter one fetched page as entire library. ACL revision invalidates rules with inaccessible libraries. |
| Pin/hide/reorder home shelves / favourite libraries (P1) | Per-user preference entity and server-resolved shelf definitions; current HomePage.tsx:23 fixed queries become typed shelf configuration + stable item queries | PreferenceChanged updates order/visibility across clients; item events update affected shelves. Keep Continue Watching's personal progress semantics. Local appearance prefs are separate device settings. |
| Versions / editions (P1) | Explicit edition/version identity and multiple linked files; selection by compatibility/quality/user choice; per-edition progress identity, stable chooser before playback | Version/link/edition/file/quality events update available choices and resume. Existing single `mediaFile` selections (`media.graphql:10`, `:81`) must migrate without destructive merging or duplicate progress. |
| Managed offline media (P1) | Separate device-owned downloads/manifest/expiry/storage limits from server torrent acquisition; generate GraphQL for persistent metadata, media bytes via media endpoints; explicit sync/conflict API | Device queue events + local download progress; local clock/progress writes allowed, not server polling. Reconnect reconciles offline progress with explicit conflict semantics; logout/revoke removes access/private manifests as policy requires. PWA shell cache is not a media library. |
| Intro/credits markers (P2) | Typed marker and optional analysis-job entities, confidence/source/manual correction, media/version identity; player's skip action uses markers | Marker/job changes invalidate timeline only; analysis completion updates marker connection via MediaFile parent. No fake detected offset or recurring analysis status query. |
| Audiobook bookmarks + sleep timer (P2) | Owned bookmark entity with note/position/chapter/version; generated CRUD/list. Sleep state initially local player state, chapter-boundary behavior explicit; server entity only if cross-device persistence desired | BookmarkChanged owner stream; chapter/end/sleep timer events operate player, not data reload. Preserve book/chapter resume and rates. |
| Source-search continuation (explicit backend follow-up) | Extend SearchSourcesInput/results with per-source cursor/completeness/partial failure/stable release key; current SearchSources is bounded, no offset/hasMore | Query when user searches/continues; source config events mark eligibility stale without re-running network search. Explain partial results, dedupe stable identities. |
| Explain wanted/blocked/missing/mismatch/quality + next action | Extend typed ContentStatus or explanatory companion projection with reason and eligible actions, using real pipeline state | PendingFileMatch/Torrent/MediaFile/quality changes invalidate reasons as well as labels; avoid divergent UI business logic. |
| Large-library filtering / retry predictability | Stable server predicates/order and cursor or qualified offset snapshots; remove silent completeness assumptions in guide, pickers and nested results | Shared invalidation adapter retains loaded windows, filters/selections and complete error snapshots. Current guide has 100-episode and 1,000-schedule caps (`schedule.graphql:5`, `TvGuide.tsx:53`) that need an explicit bounded product contract or continuation. |
| Personal Watch Later distinct from admin Wanted | New owned watchlist/membership entity with no acquisition side effect | WatchLater/membershipChanged updates personal shelves only. Keep auto-download mutations separate and admin-authorized. |
| Understand direct-play/transcode and chosen tracks/device suitability | Typed stable media/version/stream fields + qualified runtime playback decision; expose reasons and selected audio/subtitle/device | Stream/device/version events update choices; actual playback source resolves once/on explicit playback action, not repeatedly on any cache write. MediaInfo runtime negotiation remains media exception until equivalent typed API is available. |
| Personal library focus / no promotional services | Owned shelf/rule/preferences contracts above; static UI composition retains artwork and connected media workflows | No unrelated provider/feed polling or global external account subscriptions added as part of this work. |

## Completion boundaries

This audit establishes frontend static wiring, available generated operation coverage, polling sites and a concrete migration strategy. It does **not** establish event emission for every backend write path, successful runtime ACL enforcement, a current exported SDL equal to source, absence of backend domain SQL, or passing tests/builds. Those belong to the parent audit and subsequent implementation/qualification. Existing work in the dirty checkout was treated as authorized prior work and not reverted.

The appendices below are generated read-only inventory evidence written only into this document. File entries marked `scan` were exhaustively pattern-scanned; operation entries are AST-derived. Detailed behavior claims above were manually traced at the cited files/lines. A source file with no operation match may still render data supplied by a parent; the per-feature matrix supplies that dependency context.

## Appendix A — Available subscriptions versus source-mounted consumers

Each row is one of all 50 subscription definitions. A listed call site wires the document when its containing feature/hook is mounted and any skip condition permits it. Query Observable.subscribe calls are excluded.

| Document / definition | Root | `useSubscription` call sites |
| --- | --- | --- |
| FilesystemChanged (web/src/graphql/documents/files.graphql:51) | filesystemChanged | **None** |
| TorrentProgress (web/src/graphql/documents/torrents.graphql:40) | torrentProgress | web/src/features/downloads/useLiveTorrents.ts:26 |
| TorrentAdded (web/src/graphql/documents/torrents.graphql:52) | torrentAdded | web/src/components/shell/useShellCounts.ts:22<br>web/src/features/downloads/useLiveTorrents.ts:46 |
| TorrentCompleted (web/src/graphql/documents/torrents.graphql:60) | torrentCompleted | web/src/components/shell/useShellCounts.ts:23<br>web/src/features/downloads/useLiveTorrents.ts:47 |
| TorrentRemoved (web/src/graphql/documents/torrents.graphql:68) | torrentRemoved | web/src/components/shell/useShellCounts.ts:24<br>web/src/features/downloads/useLiveTorrents.ts:48 |
| EntityAlbumChanged (web/src/graphql/entities/album.graphql:94) | albumChanged | **None** |
| EntityAppLogChanged (web/src/graphql/entities/app-log.graphql:78) | appLogChanged | web/src/features/settings/LogsSettings.tsx:48 |
| EntityAppSettingChanged (web/src/graphql/entities/app-setting.graphql:76) | appSettingChanged | **None** |
| EntityArtistChanged (web/src/graphql/entities/artist.graphql:83) | artistChanged | **None** |
| EntityAudioStreamChanged (web/src/graphql/entities/audio-stream.graphql:85) | audioStreamChanged | **None** |
| EntityAudiobookChanged (web/src/graphql/entities/audiobook.graphql:96) | audiobookChanged | **None** |
| EntityCastDeviceChanged (web/src/graphql/entities/cast-device.graphql:86) | castDeviceChanged | web/src/features/settings/CastingSettings.tsx:39 |
| EntityCastSessionChanged (web/src/graphql/entities/cast-session.graphql:92) | castSessionChanged | **None** |
| EntityCastSettingChanged (web/src/graphql/entities/cast-setting.graphql:77) | castSettingChanged | **None** |
| EntityChapterChanged (web/src/graphql/entities/chapter.graphql:81) | chapterChanged | web/src/features/wanted/useWanted.ts:193 |
| EntityCollectionChanged (web/src/graphql/entities/collection.graphql:81) | collectionChanged | **None** |
| EntityEpisodeChanged (web/src/graphql/entities/episode.graphql:87) | episodeChanged | web/src/features/wanted/useWanted.ts:190 |
| EntityInviteTokenChanged (web/src/graphql/entities/invite-token.graphql:81) | inviteTokenChanged | **None** |
| EntityLibraryScanIssueChanged (web/src/graphql/entities/library-scan-issue.graphql:86) | libraryScanIssueChanged | web/src/features/libraries/ScansPanel.tsx:62 |
| EntityLibraryScanRunChanged (web/src/graphql/entities/library-scan-run.graphql:92) | libraryScanRunChanged | web/src/features/libraries/ScansPanel.tsx:61<br>web/src/features/libraries/useLibraryScan.ts:10 |
| EntityLibraryChanged (web/src/graphql/entities/library.graphql:86) | libraryChanged | **None** |
| EntityMediaChapterChanged (web/src/graphql/entities/media-chapter.graphql:76) | mediaChapterChanged | **None** |
| EntityMediaFileChanged (web/src/graphql/entities/media-file.graphql:99) | mediaFileChanged | web/src/features/wanted/useWanted.ts:194 |
| EntityMetadataCacheChanged (web/src/graphql/entities/metadata-cache.graphql:78) | metadataCacheChanged | **None** |
| EntityMovieCastCreditChanged (web/src/graphql/entities/movie-cast-credit.graphql:76) | movieCastCreditChanged | **None** |
| EntityMovieChanged (web/src/graphql/entities/movie.graphql:103) | movieChanged | web/src/features/wanted/useWanted.ts:191 |
| EntityNamingPatternChanged (web/src/graphql/entities/naming-pattern.graphql:79) | namingPatternChanged | **None** |
| EntityNotificationChanged (web/src/graphql/entities/notification.graphql:86) | notificationChanged | web/src/components/shell/useShellCounts.ts:25<br>web/src/features/activity/ActivityPage.tsx:52 |
| EntityPendingFileMatchChanged (web/src/graphql/entities/pending-file-match.graphql:95) | pendingFileMatchChanged | **None** |
| EntityPersonChanged (web/src/graphql/entities/person.graphql:75) | personChanged | **None** |
| EntityPlaybackProgressChanged (web/src/graphql/entities/playback-progress.graphql:79) | playbackProgressChanged | **None** |
| EntityPlaybackSessionChanged (web/src/graphql/entities/playback-session.graphql:89) | playbackSessionChanged | **None** |
| EntityQualityProfileChanged (web/src/graphql/entities/quality-profile.graphql:95) | qualityProfileChanged | **None** |
| EntityRefreshTokenChanged (web/src/graphql/entities/refresh-token.graphql:80) | refreshTokenChanged | **None** |
| EntityReleaseBlocklistChanged (web/src/graphql/entities/release-blocklist.graphql:81) | releaseBlocklistChanged | **None** |
| EntityRssFeedItemChanged (web/src/graphql/entities/rss-feed-item.graphql:90) | rssFeedItemChanged | **None** |
| EntityRssFeedChanged (web/src/graphql/entities/rss-feed.graphql:83) | rssFeedChanged | **None** |
| EntityScheduleCacheChanged (web/src/graphql/entities/schedule-cache.graphql:89) | scheduleCacheChanged | **None** |
| EntityScheduleSyncStateChanged (web/src/graphql/entities/schedule-sync-state.graphql:76) | scheduleSyncStateChanged | **None** |
| EntityShowChanged (web/src/graphql/entities/show.graphql:92) | showChanged | **None** |
| EntitySourcePriorityRuleChanged (web/src/graphql/entities/source-priority-rule.graphql:78) | sourcePriorityRuleChanged | **None** |
| EntitySourceChanged (web/src/graphql/entities/source.graphql:89) | sourceChanged | **None** |
| EntitySubtitleChanged (web/src/graphql/entities/subtitle.graphql:86) | subtitleChanged | **None** |
| EntityTorrentFileChanged (web/src/graphql/entities/torrent-file.graphql:81) | torrentFileChanged | **None** |
| EntityTorznabCategoryChanged (web/src/graphql/entities/torznab-category.graphql:73) | torznabCategoryChanged | **None** |
| EntityTrackChanged (web/src/graphql/entities/track.graphql:86) | trackChanged | web/src/features/wanted/useWanted.ts:192 |
| EntityUsenetDownloadChanged (web/src/graphql/entities/usenet-download.graphql:94) | usenetDownloadChanged | **None** |
| EntityUsenetServerChanged (web/src/graphql/entities/usenet-server.graphql:85) | usenetServerChanged | **None** |
| EntityUserChanged (web/src/graphql/entities/user.graphql:79) | userChanged | **None** |
| EntityVideoStreamChanged (web/src/graphql/entities/video-stream.graphql:91) | videoStreamChanged | **None** |

## Appendix B — Every feature query and its selected output dependencies

82 feature-query definitions; generated entity query families are recorded in Appendix C. Dependencies are selected object output types expanded through fragments, not inferred event propagation. The feature matrix adds computed and ownership dependencies absent from the result AST. Locations include imports and actual uses, so a bare import is not proof of an executed operation.

| Operation / source | Root fields | Selected object dependencies | Production references |
| --- | --- | --- | --- |
| PendingUpgrades (web/src/graphql/documents/activity.graphql:1) | pendingUpgrades | Notification | No production reference |
| UnresolvedScanIssues (web/src/graphql/documents/activity.graphql:14) | libraryScanIssues | LibraryScanIssue | No production reference |
| NeedsSetup (web/src/graphql/documents/auth.graphql:1) | needsSetup | scalar only | web/src/routes/_auth/login.tsx:5<br>web/src/routes/_auth/login.tsx:17<br>web/src/routes/_auth/register.tsx:5<br>web/src/routes/_auth/register.tsx:15 |
| CurrentUser (web/src/graphql/documents/auth.graphql:5) | me | MeUser | web/src/lib/auth/session.ts:15<br>web/src/lib/auth/session.ts:97 |
| BackupOverview (web/src/graphql/documents/backup.graphql:1) | backupCapabilities, backupSnapshots | BackupCapabilities, BackupSnapshotSummary | web/src/features/settings/BackupSettings.tsx:7<br>web/src/features/settings/BackupSettings.tsx:16 |
| ActiveCastSessions (web/src/graphql/documents/cast.graphql:123) | castSessions | CastSession | No production reference |
| ContentStatuses (web/src/graphql/documents/content-status.graphql:1) | contentStatuses | ContentStatusResult | web/src/hooks/useContentStatuses.ts:4<br>web/src/hooks/useContentStatuses.ts:46 |
| ContinueWatching (web/src/graphql/documents/home.graphql:3) | playbackProgresses | PlaybackProgress | web/src/features/home/useContinueWatching.ts:6<br>web/src/features/home/useContinueWatching.ts:50 |
| MediaFilesByIds (web/src/graphql/documents/home.graphql:17) | mediaFiles | MediaFile | web/src/features/home/useContinueWatching.ts:8<br>web/src/features/home/useContinueWatching.ts:54 |
| MoviesByIds (web/src/graphql/documents/home.graphql:27) | movies | Movie | web/src/features/home/useContinueWatching.ts:9<br>web/src/features/home/useContinueWatching.ts:61 |
| EpisodesByIds (web/src/graphql/documents/home.graphql:37) | episodes | Episode, Show | web/src/features/home/useContinueWatching.ts:7<br>web/src/features/home/useContinueWatching.ts:62 |
| TracksByIds (web/src/graphql/documents/home.graphql:53) | tracks | Album, Track | web/src/features/home/useContinueWatching.ts:10<br>web/src/features/home/useContinueWatching.ts:63 |
| ChaptersByIds (web/src/graphql/documents/home.graphql:68) | chapters | Audiobook, Chapter | web/src/features/home/useContinueWatching.ts:5<br>web/src/features/home/useContinueWatching.ts:64 |
| RecentMovies (web/src/graphql/documents/home.graphql:84) | movies | Movie | web/src/features/home/HomePage.tsx:9<br>web/src/features/home/HomePage.tsx:24 |
| RecentShows (web/src/graphql/documents/home.graphql:94) | shows | Show | web/src/features/home/HomePage.tsx:9<br>web/src/features/home/HomePage.tsx:25 |
| RecentAlbums (web/src/graphql/documents/home.graphql:104) | albums | Album | web/src/features/home/HomePage.tsx:9<br>web/src/features/home/HomePage.tsx:26 |
| RecentAudiobooks (web/src/graphql/documents/home.graphql:114) | audiobooks | Audiobook | web/src/features/home/HomePage.tsx:9<br>web/src/features/home/HomePage.tsx:27 |
| RecentEpisodes (web/src/graphql/documents/home.graphql:124) | episodes | Episode, Show | No production reference |
| WantedMovies (web/src/graphql/documents/home.graphql:140) | movies | Movie | No production reference |
| MovieReleaseGuide (web/src/graphql/documents/home.graphql:155) | movieReleases | MovieSearchResult | web/src/features/home/MovieReleases.tsx:9<br>web/src/features/home/MovieReleases.tsx:42<br>web/src/features/home/MovieReleases.tsx:43 |
| LibraryMoviesByTmdbIds (web/src/graphql/documents/home.graphql:172) | movies | Movie | web/src/features/home/MovieReleases.tsx:9<br>web/src/features/home/MovieReleases.tsx:56<br>web/src/features/home/MovieReleases.tsx:66 |
| LibrariesOverview (web/src/graphql/documents/libraries.graphql:1) | libraries | Album, Audiobook, Library, Movie, Show | web/src/features/home/HomePage.tsx:9<br>web/src/features/home/HomePage.tsx:23<br>web/src/features/home/MovieReleases.tsx:9<br>web/src/features/home/MovieReleases.tsx:60<br>web/src/features/libraries/LibrariesPage.tsx:6<br>web/src/features/libraries/LibrariesPage.tsx:12<br>web/src/features/search/SearchPage.tsx:10<br>web/src/features/search/SearchPage.tsx:36<br>web/src/features/settings/LibrariesSettings.tsx:13<br>web/src/features/settings/LibrariesSettings.tsx:33 |
| LibraryDetail (web/src/graphql/documents/libraries.graphql:63) | library | Library | web/src/routes/_app/libraries/$libraryId.tsx:4<br>web/src/routes/_app/libraries/$libraryId.tsx:9<br>web/src/routes/_app/libraries/$libraryId/index.tsx:4<br>web/src/routes/_app/libraries/$libraryId/index.tsx:9 |
| LibraryUnmatchedCount (web/src/graphql/documents/libraries.graphql:99) | mediaFiles | scalar only | No production reference |
| LibraryScanState (web/src/graphql/documents/libraries.graphql:110) | libraryScanRuns, libraryScanIssues | LibraryScanRun | web/src/features/libraries/useLibraryScan.ts:4<br>web/src/features/libraries/useLibraryScan.ts:9 |
| LibraryPathAvailability (web/src/graphql/documents/libraries.graphql:134) | libraryPathAvailability | LibraryPathAvailability | No production reference |
| BrowseDirectory (web/src/graphql/documents/libraries.graphql:147) | browseDirectory | BrowseDirectoryEntry, BrowseDirectoryResult, BrowseQuickPath | web/src/features/files/FolderPickerDialog.tsx:7<br>web/src/features/files/FolderPickerDialog.tsx:24 |
| FilesystemRuntimeInfo (web/src/graphql/documents/libraries.graphql:171) | filesystemRuntimeInfo | FilesystemRuntimeInfo | No production reference |
| LibraryMovies (web/src/graphql/documents/library-lists.graphql:18) | movies | MediaFile, Movie | web/src/features/libraries/browser/MoviesBrowser.tsx:9<br>web/src/features/libraries/browser/MoviesBrowser.tsx:56 |
| LibraryShows (web/src/graphql/documents/library-lists.graphql:35) | shows | Show | web/src/features/libraries/browser/ShowsBrowser.tsx:8<br>web/src/features/libraries/browser/ShowsBrowser.tsx:49 |
| LibraryCollections (web/src/graphql/documents/library-lists.graphql:59) | collections | Collection | web/src/features/libraries/browser/CollectionsBrowser.tsx:8<br>web/src/features/libraries/browser/CollectionsBrowser.tsx:38 |
| LibraryAlbums (web/src/graphql/documents/library-lists.graphql:78) | albums | Album | web/src/features/libraries/browser/AlbumsBrowser.tsx:8<br>web/src/features/libraries/browser/AlbumsBrowser.tsx:40 |
| LibraryArtists (web/src/graphql/documents/library-lists.graphql:97) | artists | Artist | web/src/features/libraries/browser/ArtistsBrowser.tsx:6<br>web/src/features/libraries/browser/ArtistsBrowser.tsx:32 |
| LibraryTracks (web/src/graphql/documents/library-lists.graphql:111) | tracks | Album, MediaFile, Track | web/src/features/libraries/browser/TracksBrowser.tsx:7<br>web/src/features/libraries/browser/TracksBrowser.tsx:60 |
| LibraryAudiobooks (web/src/graphql/documents/library-lists.graphql:133) | audiobooks | Audiobook | web/src/features/libraries/browser/AudiobooksBrowser.tsx:8<br>web/src/features/libraries/browser/AudiobooksBrowser.tsx:40 |
| LibraryMediaFiles (web/src/graphql/documents/library-lists.graphql:152) | mediaFiles | MediaFile | web/src/features/libraries/browser/FilesBrowser.tsx:10<br>web/src/features/libraries/browser/FilesBrowser.tsx:66 |
| LibraryScanRuns (web/src/graphql/documents/library-lists.graphql:166) | libraryScanRuns | LibraryScanRun | No production reference |
| LibraryScanIssues (web/src/graphql/documents/library-lists.graphql:180) | libraryScanIssues | LibraryScanIssue | No production reference |
| MovieDetail (web/src/graphql/documents/media.graphql:2) | movie | Library, MediaFile, Movie | web/src/features/media/MoviePage.tsx:19<br>web/src/features/media/MoviePage.tsx:43<br>web/src/features/player/WatchPage.tsx:5<br>web/src/features/player/WatchPage.tsx:29<br>web/src/routes/_app/movies/$movieId.tsx:4<br>web/src/routes/_app/movies/$movieId.tsx:9 |
| MovieCastCredits (web/src/graphql/documents/media.graphql:16) | movieCastCredits | MovieCastCredit, Person | web/src/features/media/MoviePage.tsx:17<br>web/src/features/media/MoviePage.tsx:46 |
| MediaFileStreams (web/src/graphql/documents/media.graphql:34) | videoStreams, audioStreams, subtitles, mediaChapters | AudioStream, MediaChapter, Subtitle, VideoStream | web/src/features/media/FileDetails.tsx:7<br>web/src/features/media/FileDetails.tsx:16<br>web/src/features/player/VideoPlayer.tsx:20<br>web/src/features/player/VideoPlayer.tsx:54 |
| ShowDetail (web/src/graphql/documents/media.graphql:65) | show | Library, Show | web/src/features/media/ShowPage.tsx:13<br>web/src/features/media/ShowPage.tsx:35<br>web/src/routes/_app/shows/$showId.tsx:4<br>web/src/routes/_app/shows/$showId.tsx:9 |
| ShowEpisodes (web/src/graphql/documents/media.graphql:76) | episodes | Episode, MediaFile | web/src/features/media/useShowEpisodes.ts:4<br>web/src/features/media/useShowEpisodes.ts:10<br>web/src/features/media/useShowEpisodes.ts:13<br>web/src/features/media/useShowEpisodes.ts:64<br>web/src/features/media/useShowEpisodes.ts:80 |
| ShowEpisodesPage (web/src/graphql/documents/media.graphql:99) | episodes | Episode, MediaFile | web/src/features/media/useShowEpisodes.ts:4<br>web/src/features/media/useShowEpisodes.ts:34<br>web/src/features/media/useShowEpisodes.ts:70 |
| EpisodeDetail (web/src/graphql/documents/media.graphql:122) | episode | Episode, MediaFile, Show | web/src/features/player/WatchPage.tsx:5<br>web/src/features/player/WatchPage.tsx:30 |
| AlbumDetail (web/src/graphql/documents/media.graphql:134) | album | Album, Library, MediaFile, Track | web/src/features/media/AlbumPage.tsx:19<br>web/src/features/media/AlbumPage.tsx:95<br>web/src/routes/_app/albums/$albumId.tsx:4<br>web/src/routes/_app/albums/$albumId.tsx:9 |
| AlbumTracksPage (web/src/graphql/documents/media.graphql:164) | album | Album, MediaFile, Track | web/src/features/media/AlbumPage.tsx:19<br>web/src/features/media/AlbumPage.tsx:99 |
| ArtistDetail (web/src/graphql/documents/media.graphql:188) | artist | Album, Artist, Library | web/src/features/media/ArtistPage.tsx:7<br>web/src/features/media/ArtistPage.tsx:17<br>web/src/features/media/ArtistPage.tsx:21<br>web/src/routes/_app/artists/$artistId.tsx:4<br>web/src/routes/_app/artists/$artistId.tsx:9 |
| AudiobookDetail (web/src/graphql/documents/media.graphql:207) | audiobook | Audiobook, Chapter, Library, MediaFile | web/src/features/media/AudiobookPage.tsx:18<br>web/src/features/media/AudiobookPage.tsx:96<br>web/src/routes/_app/audiobooks/$audiobookId.tsx:4<br>web/src/routes/_app/audiobooks/$audiobookId.tsx:9 |
| AudiobookChaptersPage (web/src/graphql/documents/media.graphql:235) | audiobook | Audiobook, Chapter, MediaFile | web/src/features/media/AudiobookPage.tsx:18<br>web/src/features/media/AudiobookPage.tsx:100 |
| CollectionDetail (web/src/graphql/documents/media.graphql:258) | collection | Collection, Library, Movie | web/src/features/media/CollectionPage.tsx:7<br>web/src/features/media/CollectionPage.tsx:17<br>web/src/features/media/CollectionPage.tsx:21<br>web/src/routes/_app/collections/$collectionId.tsx:4<br>web/src/routes/_app/collections/$collectionId.tsx:9 |
| MovieCollectionDetails (web/src/graphql/documents/media.graphql:277) | movieCollectionDetails | MovieCollectionDetails, MovieCollectionMovieDetails | web/src/features/media/CollectionPage.tsx:7<br>web/src/features/media/CollectionPage.tsx:26<br>web/src/features/media/MoviePage.tsx:18<br>web/src/features/media/MoviePage.tsx:47 |
| PlaybackProgressForFile (web/src/graphql/documents/playback.graphql:1) | playbackProgresses | PlaybackProgress | web/src/features/media/MoviePage.tsx:20<br>web/src/features/media/MoviePage.tsx:48<br>web/src/features/player/useProgressSync.ts:9<br>web/src/features/player/useProgressSync.ts:28 |
| PlaybackProgressForFiles (web/src/graphql/documents/playback.graphql:11) | playbackProgresses | PlaybackProgress | web/src/features/media/useMediaProgress.ts:4<br>web/src/features/media/useMediaProgress.ts:24 |
| PlaybackSyncInterval (web/src/graphql/documents/playback.graphql:21) | appSettings | AppSetting | web/src/features/player/useProgressSync.ts:10<br>web/src/features/player/useProgressSync.ts:33 |
| QualityProfilesList (web/src/graphql/documents/quality.graphql:1) | qualityProfiles | QualityProfile | web/src/features/acquisition/AcquisitionDialog.tsx:14<br>web/src/features/acquisition/AcquisitionDialog.tsx:57<br>web/src/features/libraries/LibrarySettingsForm.tsx:16<br>web/src/features/libraries/LibrarySettingsForm.tsx:40 |
| NamingPatternsForType (web/src/graphql/documents/quality.graphql:11) | namingPatterns | NamingPattern | web/src/features/libraries/LibrarySettingsForm.tsx:15<br>web/src/features/libraries/LibrarySettingsForm.tsx:39 |
| UpcomingLibraryEpisodes (web/src/graphql/documents/schedule.graphql:4) | episodes | Episode, Show | web/src/features/home/TvGuide.tsx:7<br>web/src/features/home/TvGuide.tsx:87 |
| ScheduleWeek (web/src/graphql/documents/schedule.graphql:32) | scheduleCaches | ScheduleCache | web/src/features/home/TvGuide.tsx:7<br>web/src/features/home/TvGuide.tsx:49 |
| ScheduleCountries (web/src/graphql/documents/schedule.graphql:49) | scheduleSyncStates | ScheduleSyncState | web/src/features/home/TvGuide.tsx:7<br>web/src/features/home/TvGuide.tsx:88 |
| NextLibraryEpisodes (web/src/graphql/documents/schedule.graphql:62) | episodes | Episode, Show | web/src/features/home/TvGuide.tsx:7<br>web/src/features/home/TvGuide.tsx:98 |
| SearchMovies (web/src/graphql/documents/search.graphql:3) | searchMovies | MovieSearchResult | web/src/features/library-add/ProviderSearchDialog.tsx:18<br>web/src/features/library-add/ProviderSearchDialog.tsx:62<br>web/src/features/search/ProviderResults.tsx:15<br>web/src/features/search/ProviderResults.tsx:46 |
| SearchTvShows (web/src/graphql/documents/search.graphql:19) | searchTvShows | TvShowSearchResult | web/src/features/library-add/ProviderSearchDialog.tsx:19<br>web/src/features/library-add/ProviderSearchDialog.tsx:63<br>web/src/features/search/ProviderResults.tsx:16<br>web/src/features/search/ProviderResults.tsx:47 |
| SearchAlbums (web/src/graphql/documents/search.graphql:35) | searchAlbums | AlbumSearchResult | web/src/features/library-add/ProviderSearchDialog.tsx:15<br>web/src/features/library-add/ProviderSearchDialog.tsx:64<br>web/src/features/search/ProviderResults.tsx:13<br>web/src/features/search/ProviderResults.tsx:48 |
| SearchAudiobooks (web/src/graphql/documents/search.graphql:55) | searchAudiobooks | AudiobookSearchResult | web/src/features/library-add/ProviderSearchDialog.tsx:16<br>web/src/features/library-add/ProviderSearchDialog.tsx:65<br>web/src/features/search/ProviderResults.tsx:14<br>web/src/features/search/ProviderResults.tsx:49 |
| SearchMovieCollections (web/src/graphql/documents/search.graphql:68) | searchMovieCollections | MovieCollectionSearchResult | web/src/features/library-add/ProviderSearchDialog.tsx:17<br>web/src/features/library-add/ProviderSearchDialog.tsx:66 |
| SearchSources (web/src/graphql/documents/search.graphql:79) | searchSources | ParsedRelease, SourceReleaseInfo, SourceSearchResultItem, SourceSearchResultSet | web/src/features/downloads/ReleaseSearchDialog.tsx:10<br>web/src/features/downloads/ReleaseSearchDialog.tsx:81 |
| LibrarySearch (web/src/graphql/documents/search.graphql:134) | movies, shows, albums, artists, audiobooks, episodes | Album, Artist, Audiobook, Episode, Movie, Show | web/src/features/search/SearchPage.tsx:10<br>web/src/features/search/SearchPage.tsx:38 |
| AppSettingsByCategory (web/src/graphql/documents/settings.graphql:1) | appSettings | AppSetting | web/src/features/settings/useAppSettings.ts:4<br>web/src/features/settings/useAppSettings.ts:12 |
| AppSettingByKey (web/src/graphql/documents/settings.graphql:11) | appSettings | AppSetting | No production reference |
| ActiveDownloadCount (web/src/graphql/documents/shell.graphql:1) | activeDownloadCount | scalar only | web/src/components/shell/useShellCounts.ts:4<br>web/src/components/shell/useShellCounts.ts:18 |
| UnreadNotificationCount (web/src/graphql/documents/shell.graphql:5) | notifications | scalar only | web/src/components/shell/useShellCounts.ts:9<br>web/src/components/shell/useShellCounts.ts:19 |
| NavLibraries (web/src/graphql/documents/shell.graphql:13) | libraries | Library | web/src/components/shell/NavRail.tsx:9<br>web/src/components/shell/NavRail.tsx:28<br>web/src/features/downloads/AddTorrentDialog.tsx:8<br>web/src/features/downloads/AddTorrentDialog.tsx:24 |
| AvailableSourceDefinitions (web/src/graphql/documents/sources.graphql:1) | availableSourceDefinitions | SourceDefinitionInfo | web/src/features/settings/SourceDialog.tsx:10<br>web/src/features/settings/SourceDialog.tsx:39 |
| SourceSettingDefinitions (web/src/graphql/documents/sources.graphql:14) | sourceSettingDefinitions | SourceSettingDefinition, SourceSettingOption | web/src/features/settings/SourceDialog.tsx:13<br>web/src/features/settings/SourceDialog.tsx:42 |
| LiveTorrents (web/src/graphql/documents/torrents.graphql:16) | liveTorrents | LiveTorrent, LiveTorrentFile | web/src/features/downloads/useLiveTorrents.ts:6<br>web/src/features/downloads/useLiveTorrents.ts:22 |
| LiveTorrent (web/src/graphql/documents/torrents.graphql:28) | liveTorrent | LiveTorrent, LiveTorrentFile | No production reference |
| WantedListEpisodes (web/src/graphql/documents/wanted.graphql:4) | episodes | Episode, Show | web/src/features/wanted/useWanted.ts:15<br>web/src/features/wanted/useWanted.ts:50<br>web/src/features/wanted/useWanted.ts:80 |
| WantedListMovies (web/src/graphql/documents/wanted.graphql:25) | movies | Movie | web/src/features/wanted/useWanted.ts:16<br>web/src/features/wanted/useWanted.ts:56<br>web/src/features/wanted/useWanted.ts:81 |
| WantedListTracks (web/src/graphql/documents/wanted.graphql:39) | tracks | Album, Track | web/src/features/wanted/useWanted.ts:17<br>web/src/features/wanted/useWanted.ts:62<br>web/src/features/wanted/useWanted.ts:82 |
| WantedListChapters (web/src/graphql/documents/wanted.graphql:60) | chapters | Audiobook, Chapter | web/src/features/wanted/useWanted.ts:14<br>web/src/features/wanted/useWanted.ts:68<br>web/src/features/wanted/useWanted.ts:83 |
| UpgradableFiles (web/src/graphql/documents/wanted.graphql:83) | mediaFiles | MediaFile | web/src/features/wanted/useWanted.ts:8<br>web/src/features/wanted/useWanted.ts:73 |

## Appendix C — All 46 discovered entity document families

Actual operation names/roots are read from each file. Every row has two queries and five CRUD mutation definitions; subscription absence is explicit. These are SDL-available/generated documents, not proof that a feature queries or listens to them.

| Entity document | Query operations | Mutation operation roots | Changed root |
| --- | --- | --- | --- |
| web/src/graphql/entities/album.graphql:1 | EntityAlbumList@31, EntityAlbumGet@46 | createAlbum@52, updateAlbum@62, updateAlbums@72, deleteAlbum@80, deleteAlbums@87 | albumChanged@94 |
| web/src/graphql/entities/app-log.graphql:1 | EntityAppLogList@15, EntityAppLogGet@30 | createAppLog@36, updateAppLog@46, updateAppLogs@56, deleteAppLog@64, deleteAppLogs@71 | appLogChanged@78 |
| web/src/graphql/entities/app-setting.graphql:1 | EntityAppSettingList@13, EntityAppSettingGet@28 | createAppSetting@34, updateAppSetting@44, updateAppSettings@54, deleteAppSetting@62, deleteAppSettings@69 | appSettingChanged@76 |
| web/src/graphql/entities/artist.graphql:1 | EntityArtistList@20, EntityArtistGet@35 | createArtist@41, updateArtist@51, updateArtists@61, deleteArtist@69, deleteArtists@76 | artistChanged@83 |
| web/src/graphql/entities/audio-stream.graphql:1 | EntityAudioStreamList@22, EntityAudioStreamGet@37 | createAudioStream@43, updateAudioStream@53, updateAudioStreams@63, deleteAudioStream@71, deleteAudioStreams@78 | audioStreamChanged@85 |
| web/src/graphql/entities/audiobook.graphql:1 | EntityAudiobookList@33, EntityAudiobookGet@48 | createAudiobook@54, updateAudiobook@64, updateAudiobooks@74, deleteAudiobook@82, deleteAudiobooks@89 | audiobookChanged@96 |
| web/src/graphql/entities/cast-device.graphql:1 | EntityCastDeviceList@23, EntityCastDeviceGet@38 | createCastDevice@44, updateCastDevice@54, updateCastDevices@64, deleteCastDevice@72, deleteCastDevices@79 | castDeviceChanged@86 |
| web/src/graphql/entities/cast-session.graphql:1 | EntityCastSessionList@29, EntityCastSessionGet@44 | createCastSession@50, updateCastSession@60, updateCastSessions@70, deleteCastSession@78, deleteCastSessions@85 | castSessionChanged@92 |
| web/src/graphql/entities/cast-setting.graphql:1 | EntityCastSettingList@14, EntityCastSettingGet@29 | createCastSetting@35, updateCastSetting@45, updateCastSettings@55, deleteCastSetting@63, deleteCastSettings@70 | castSettingChanged@77 |
| web/src/graphql/entities/chapter.graphql:1 | EntityChapterList@18, EntityChapterGet@33 | createChapter@39, updateChapter@49, updateChapters@59, deleteChapter@67, deleteChapters@74 | chapterChanged@81 |
| web/src/graphql/entities/collection.graphql:1 | EntityCollectionList@18, EntityCollectionGet@33 | createCollection@39, updateCollection@49, updateCollections@59, deleteCollection@67, deleteCollections@74 | collectionChanged@81 |
| web/src/graphql/entities/episode.graphql:1 | EntityEpisodeList@24, EntityEpisodeGet@39 | createEpisode@45, updateEpisode@55, updateEpisodes@65, deleteEpisode@73, deleteEpisodes@80 | episodeChanged@87 |
| web/src/graphql/entities/invite-token.graphql:1 | EntityInviteTokenList@18, EntityInviteTokenGet@33 | createInviteToken@39, updateInviteToken@49, updateInviteTokens@59, deleteInviteToken@67, deleteInviteTokens@74 | inviteTokenChanged@81 |
| web/src/graphql/entities/library-scan-issue.graphql:1 | EntityLibraryScanIssueList@23, EntityLibraryScanIssueGet@38 | createLibraryScanIssue@44, updateLibraryScanIssue@54, updateLibraryScanIssues@64, deleteLibraryScanIssue@72, deleteLibraryScanIssues@79 | libraryScanIssueChanged@86 |
| web/src/graphql/entities/library-scan-run.graphql:1 | EntityLibraryScanRunList@29, EntityLibraryScanRunGet@44 | createLibraryScanRun@50, updateLibraryScanRun@60, updateLibraryScanRuns@70, deleteLibraryScanRun@78, deleteLibraryScanRuns@85 | libraryScanRunChanged@92 |
| web/src/graphql/entities/library.graphql:1 | EntityLibraryList@23, EntityLibraryGet@38 | createLibrary@44, updateLibrary@54, updateLibraries@64, deleteLibrary@72, deleteLibraries@79 | libraryChanged@86 |
| web/src/graphql/entities/media-chapter.graphql:1 | EntityMediaChapterList@13, EntityMediaChapterGet@28 | createMediaChapter@34, updateMediaChapter@44, updateMediaChapters@54, deleteMediaChapter@62, deleteMediaChapters@69 | mediaChapterChanged@76 |
| web/src/graphql/entities/media-file.graphql:1 | EntityMediaFileList@36, EntityMediaFileGet@51 | createMediaFile@57, updateMediaFile@67, updateMediaFiles@77, deleteMediaFile@85, deleteMediaFiles@92 | mediaFileChanged@99 |
| web/src/graphql/entities/metadata-cache.graphql:1 | EntityMetadataCacheList@15, EntityMetadataCacheGet@30 | createMetadataCache@36, updateMetadataCache@46, updateMetadataCaches@56, deleteMetadataCache@64, deleteMetadataCaches@71 | metadataCacheChanged@78 |
| web/src/graphql/entities/movie-cast-credit.graphql:1 | EntityMovieCastCreditList@13, EntityMovieCastCreditGet@28 | createMovieCastCredit@34, updateMovieCastCredit@44, updateMovieCastCredits@54, deleteMovieCastCredit@62, deleteMovieCastCredits@69 | movieCastCreditChanged@76 |
| web/src/graphql/entities/movie.graphql:1 | EntityMovieList@40, EntityMovieGet@55 | createMovie@61, updateMovie@71, updateMovies@81, deleteMovie@89, deleteMovies@96 | movieChanged@103 |
| web/src/graphql/entities/naming-pattern.graphql:1 | EntityNamingPatternList@16, EntityNamingPatternGet@31 | createNamingPattern@37, updateNamingPattern@47, updateNamingPatterns@57, deleteNamingPattern@65, deleteNamingPatterns@72 | namingPatternChanged@79 |
| web/src/graphql/entities/notification.graphql:1 | EntityNotificationList@23, EntityNotificationGet@38 | createNotification@44, updateNotification@54, updateNotifications@64, deleteNotification@72, deleteNotifications@79 | notificationChanged@86 |
| web/src/graphql/entities/pending-file-match.graphql:1 | EntityPendingFileMatchList@32, EntityPendingFileMatchGet@47 | createPendingFileMatch@53, updatePendingFileMatch@63, updatePendingFileMatches@73, deletePendingFileMatch@81, deletePendingFileMatches@88 | pendingFileMatchChanged@95 |
| web/src/graphql/entities/person.graphql:1 | EntityPersonList@12, EntityPersonGet@27 | createPerson@33, updatePerson@43, updatePeople@53, deletePerson@61, deletePeople@68 | personChanged@75 |
| web/src/graphql/entities/playback-progress.graphql:1 | EntityPlaybackProgressList@16, EntityPlaybackProgressGet@31 | createPlaybackProgress@37, updatePlaybackProgress@47, updatePlaybackProgresses@57, deletePlaybackProgress@65, deletePlaybackProgresses@72 | playbackProgressChanged@79 |
| web/src/graphql/entities/playback-session.graphql:1 | EntityPlaybackSessionList@26, EntityPlaybackSessionGet@41 | createPlaybackSession@47, updatePlaybackSession@57, updatePlaybackSessions@67, deletePlaybackSession@75, deletePlaybackSessions@82 | playbackSessionChanged@89 |
| web/src/graphql/entities/quality-profile.graphql:1 | EntityQualityProfileList@32, EntityQualityProfileGet@47 | createQualityProfile@53, updateQualityProfile@63, updateQualityProfiles@73, deleteQualityProfile@81, deleteQualityProfiles@88 | qualityProfileChanged@95 |
| web/src/graphql/entities/refresh-token.graphql:1 | EntityRefreshTokenList@17, EntityRefreshTokenGet@32 | createRefreshToken@38, updateRefreshToken@48, updateRefreshTokens@58, deleteRefreshToken@66, deleteRefreshTokens@73 | refreshTokenChanged@80 |
| web/src/graphql/entities/release-blocklist.graphql:1 | EntityReleaseBlocklistList@18, EntityReleaseBlocklistGet@33 | createReleaseBlocklist@39, updateReleaseBlocklist@49, updateReleaseBlocklists@59, deleteReleaseBlocklist@67, deleteReleaseBlocklists@74 | releaseBlocklistChanged@81 |
| web/src/graphql/entities/rss-feed-item.graphql:1 | EntityRssFeedItemList@27, EntityRssFeedItemGet@42 | createRssFeedItem@48, updateRssFeedItem@58, updateRssFeedItems@68, deleteRssFeedItem@76, deleteRssFeedItems@83 | rssFeedItemChanged@90 |
| web/src/graphql/entities/rss-feed.graphql:1 | EntityRssFeedList@20, EntityRssFeedGet@35 | createRssFeed@41, updateRssFeed@51, updateRssFeeds@61, deleteRssFeed@69, deleteRssFeeds@76 | rssFeedChanged@83 |
| web/src/graphql/entities/schedule-cache.graphql:1 | EntityScheduleCacheList@26, EntityScheduleCacheGet@41 | createScheduleCache@47, updateScheduleCache@57, updateScheduleCaches@67, deleteScheduleCache@75, deleteScheduleCaches@82 | scheduleCacheChanged@89 |
| web/src/graphql/entities/schedule-sync-state.graphql:1 | EntityScheduleSyncStateList@13, EntityScheduleSyncStateGet@28 | createScheduleSyncState@34, updateScheduleSyncState@44, updateScheduleSyncStates@54, deleteScheduleSyncState@62, deleteScheduleSyncStates@69 | scheduleSyncStateChanged@76 |
| web/src/graphql/entities/show.graphql:1 | EntityShowList@29, EntityShowGet@44 | createShow@50, updateShow@60, updateShows@70, deleteShow@78, deleteShows@85 | showChanged@92 |
| web/src/graphql/entities/source-priority-rule.graphql:1 | EntitySourcePriorityRuleList@15, EntitySourcePriorityRuleGet@30 | createSourcePriorityRule@36, updateSourcePriorityRule@46, updateSourcePriorityRules@56, deleteSourcePriorityRule@64, deleteSourcePriorityRules@71 | sourcePriorityRuleChanged@78 |
| web/src/graphql/entities/source.graphql:1 | EntitySourceList@26, EntitySourceGet@41 | createSource@47, updateSource@57, updateSources@67, deleteSource@75, deleteSources@82 | sourceChanged@89 |
| web/src/graphql/entities/subtitle.graphql:1 | EntitySubtitleList@23, EntitySubtitleGet@38 | createSubtitle@44, updateSubtitle@54, updateSubtitles@64, deleteSubtitle@72, deleteSubtitles@79 | subtitleChanged@86 |
| web/src/graphql/entities/torrent-file.graphql:1 | EntityTorrentFileList@18, EntityTorrentFileGet@33 | createTorrentFile@39, updateTorrentFile@49, updateTorrentFiles@59, deleteTorrentFile@67, deleteTorrentFiles@74 | torrentFileChanged@81 |
| web/src/graphql/entities/torrent.graphql:1 | EntityTorrentList@41, EntityTorrentGet@56 | createTorrent@62, updateTorrent@72, updateTorrents@82, deleteTorrent@90, deleteTorrents@97 | **Absent (Torrent)** |
| web/src/graphql/entities/torznab-category.graphql:1 | EntityTorznabCategoryList@10, EntityTorznabCategoryGet@25 | createTorznabCategory@31, updateTorznabCategory@41, updateTorznabCategories@51, deleteTorznabCategory@59, deleteTorznabCategories@66 | torznabCategoryChanged@73 |
| web/src/graphql/entities/track.graphql:1 | EntityTrackList@23, EntityTrackGet@38 | createTrack@44, updateTrack@54, updateTracks@64, deleteTrack@72, deleteTracks@79 | trackChanged@86 |
| web/src/graphql/entities/usenet-download.graphql:1 | EntityUsenetDownloadList@31, EntityUsenetDownloadGet@46 | createUsenetDownload@52, updateUsenetDownload@62, updateUsenetDownloads@72, deleteUsenetDownload@80, deleteUsenetDownloads@87 | usenetDownloadChanged@94 |
| web/src/graphql/entities/usenet-server.graphql:1 | EntityUsenetServerList@22, EntityUsenetServerGet@37 | createUsenetServer@43, updateUsenetServer@53, updateUsenetServers@63, deleteUsenetServer@71, deleteUsenetServers@78 | usenetServerChanged@85 |
| web/src/graphql/entities/user.graphql:1 | EntityUserList@16, EntityUserGet@31 | createUser@37, updateUser@47, updateUsers@57, deleteUser@65, deleteUsers@72 | userChanged@79 |
| web/src/graphql/entities/video-stream.graphql:1 | EntityVideoStreamList@28, EntityVideoStreamGet@43 | createVideoStream@49, updateVideoStream@59, updateVideoStreams@69, deleteVideoStream@77, deleteVideoStreams@84 | videoStreamChanged@91 |

## Appendix D — Complete production TS/TSX file scan manifest

All 217 files were read by the static scanner. `scan` alone means no data/timer/transport pattern found, not a manually verified absence of inherited data. This manifest includes route wrappers and static presentation files so the scan boundary is reproducible. Matched line numbers cover generated-document references, query/subscription APIs, cache/refetch operations, timers and media transports.

| Source file | Lines | Matched evidence line numbers / classification |
| --- | --- | --- |
| web/src/app/providers.tsx:1 | 37 | scan; presentation/local composition/no direct data match |
| web/src/app/router.tsx:1 | 23 | scan; presentation/local composition/no direct data match |
| web/src/components/constellation/ConstellationBackdrop.tsx:1 | 23 | scan; presentation/local composition/no direct data match |
| web/src/components/constellation/FilmConstellation.tsx:1 | 317 | scan; 262, 275, 297, 301 |
| web/src/components/constellation/graph.ts:1 | 57 | scan; presentation/local composition/no direct data match |
| web/src/components/constellation/simulation.ts:1 | 327 | scan; presentation/local composition/no direct data match |
| web/src/components/shell/AppShell.tsx:1 | 61 | scan; presentation/local composition/no direct data match |
| web/src/components/shell/BottomTabs.tsx:1 | 62 | scan; 15 |
| web/src/components/shell/Breadcrumbs.tsx:1 | 60 | scan; presentation/local composition/no direct data match |
| web/src/components/shell/ConnectionBanner.tsx:1 | 36 | scan; 4, 17, 18, 19 |
| web/src/components/shell/NavRail.tsx:1 | 131 | scan; 9, 25, 28 |
| web/src/components/shell/NotFound.tsx:1 | 18 | scan; presentation/local composition/no direct data match |
| web/src/components/shell/RouteError.tsx:1 | 42 | scan; presentation/local composition/no direct data match |
| web/src/components/shell/ShellCommands.tsx:1 | 22 | scan; presentation/local composition/no direct data match |
| web/src/components/shell/TopBar.tsx:1 | 67 | scan; 27 |
| web/src/components/shell/UpdatePrompt.tsx:1 | 24 | scan; presentation/local composition/no direct data match |
| web/src/components/shell/UserMenu.tsx:1 | 41 | scan; presentation/local composition/no direct data match |
| web/src/components/shell/nav.ts:1 | 30 | scan; presentation/local composition/no direct data match |
| web/src/components/shell/useShellCounts.ts:1 | 32 | scan; 4, 5, 6, 7, 8, 9, 14, 18, 19, 21, 22, 23, 24, 25 |
| web/src/components/three/AmbientScene.tsx:1 | 144 | scan; presentation/local composition/no direct data match |
| web/src/components/three/SceneBackdrop.tsx:1 | 40 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/AlphabetRail.tsx:1 | 49 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/Artwork.tsx:1 | 71 | scan; 4 |
| web/src/components/ui/BackdropCard.tsx:1 | 48 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/BrandMark.tsx:1 | 11 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/Button.tsx:1 | 69 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/ConfirmDialog.tsx:1 | 64 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/DataTable.tsx:1 | 394 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/Dialog.tsx:1 | 49 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/EmptyState.tsx:1 | 25 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/ErrorState.tsx:1 | 28 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/Field.tsx:1 | 219 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/GlassButton.tsx:1 | 18 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/GlassControls.tsx:1 | 49 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/HeroBanner.tsx:1 | 73 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/KeyValue.tsx:1 | 17 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/LoadedDataTable.tsx:1 | 48 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/MediaRow.tsx:1 | 70 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/MetaDot.tsx:1 | 19 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/PageHeader.tsx:1 | 29 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/Panel.tsx:1 | 22 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/PosterCard.tsx:1 | 70 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/PosterMarquee.tsx:1 | 33 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/Section.tsx:1 | 27 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/Skeletons.tsx:1 | 55 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/Spinner.tsx:1 | 7 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/StatTile.tsx:1 | 21 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/StatusChip.tsx:1 | 24 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/Tabs.tsx:1 | 77 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/ThemePicker.tsx:1 | 32 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/Toolbar.tsx:1 | 47 | scan; presentation/local composition/no direct data match |
| web/src/components/ui/index.ts:1 | 30 | scan; presentation/local composition/no direct data match |
| web/src/features/acquisition/AcquisitionChip.tsx:1 | 12 | scan; presentation/local composition/no direct data match |
| web/src/features/acquisition/AcquisitionDialog.tsx:1 | 179 | scan; 9, 10, 11, 12, 13, 14, 15, 56, 57, 58, 59, 60, 61, 62 |
| web/src/features/acquisition/mode.ts:1 | 37 | scan; presentation/local composition/no direct data match |
| web/src/features/activity/ActivityPage.tsx:1 | 161 | scan; 11, 12, 13, 14, 15, 16, 51, 52, 54, 55, 56, 57, 63, 155 |
| web/src/features/auth/LoginForm.tsx:1 | 70 | scan; 9, 22 |
| web/src/features/auth/RegisterForm.tsx:1 | 82 | scan; 9, 23 |
| web/src/features/auth/schemas.ts:1 | 19 | scan; presentation/local composition/no direct data match |
| web/src/features/downloads/AddTorrentDialog.tsx:1 | 75 | scan; 8, 24, 25 |
| web/src/features/downloads/DownloadsPage.tsx:1 | 197 | scan; 8, 50, 51, 52, 53, 61 |
| web/src/features/downloads/LinkTorrentDialog.tsx:1 | 72 | scan; 7, 22, 24, 25, 67 |
| web/src/features/downloads/ReleaseSearchDialog.tsx:1 | 328 | scan; 10, 81, 82, 91 |
| web/src/features/downloads/TorrentDetail.tsx:1 | 69 | scan; presentation/local composition/no direct data match |
| web/src/features/downloads/useLiveTorrents.ts:1 | 57 | scan; 6, 22, 24, 26, 30, 43, 44, 46, 47, 48 |
| web/src/features/files/FolderPickerDialog.tsx:1 | 104 | scan; 7, 24 |
| web/src/features/home/HomePage.tsx:1 | 153 | scan; 9, 23, 24, 25, 26, 27 |
| web/src/features/home/MovieReleases.tsx:1 | 173 | scan; 9, 42, 43, 56, 60, 66, 143 |
| web/src/features/home/TvGuide.tsx:1 | 257 | scan; 7, 49, 87, 88, 98 |
| web/src/features/home/useContinueWatching.ts:1 | 133 | scan; 5, 6, 7, 8, 9, 10, 50, 54, 61, 62, 63, 64 |
| web/src/features/libraries/LibrariesPage.tsx:1 | 49 | scan; 6, 12, 30 |
| web/src/features/libraries/LibraryCard.tsx:1 | 98 | scan; presentation/local composition/no direct data match |
| web/src/features/libraries/LibraryLayout.tsx:1 | 93 | scan; 43 |
| web/src/features/libraries/LibrarySettingsForm.tsx:1 | 161 | scan; 13, 14, 15, 16, 37, 38, 39, 40 |
| web/src/features/libraries/ManualMatchDialog.tsx:1 | 92 | scan; 7, 24 |
| web/src/features/libraries/ScansPanel.tsx:1 | 202 | scan; 10, 11, 12, 13, 14, 15, 16, 45, 59, 60, 61, 62, 64, 65, 66, 73, 171, 195 |
| web/src/features/libraries/browser/AlbumsBrowser.tsx:1 | 128 | scan; 8, 40, 106, 124 |
| web/src/features/libraries/browser/ArtistsBrowser.tsx:1 | 97 | scan; 6, 32, 85 |
| web/src/features/libraries/browser/AudiobooksBrowser.tsx:1 | 127 | scan; 8, 40, 106, 123 |
| web/src/features/libraries/browser/BrowserToolbar.tsx:1 | 62 | scan; presentation/local composition/no direct data match |
| web/src/features/libraries/browser/CollectionsBrowser.tsx:1 | 121 | scan; 8, 38, 100, 117 |
| web/src/features/libraries/browser/FilesBrowser.tsx:1 | 217 | scan; 9, 10, 11, 12, 13, 66, 68, 69, 70, 71, 78, 210, 213 |
| web/src/features/libraries/browser/MoviesBrowser.tsx:1 | 180 | scan; 9, 10, 56, 67, 143, 161 |
| web/src/features/libraries/browser/ShowsBrowser.tsx:1 | 147 | scan; 8, 49, 116, 143 |
| web/src/features/libraries/browser/TracksBrowser.tsx:1 | 149 | scan; 7, 8, 60, 62, 116 |
| web/src/features/libraries/browser/useBrowserFilters.ts:1 | 38 | scan; presentation/local composition/no direct data match |
| web/src/features/libraries/browser/useCompleteConnection.ts:1 | 44 | scan; 1, 12, 17, 34 |
| web/src/features/libraries/browser/useConnection.ts:1 | 33 | scan; 2, 15, 19 |
| web/src/features/libraries/browser/useInfiniteConnection.ts:1 | 117 | scan; 3, 39, 51, 99 |
| web/src/features/libraries/browser/useLetterJump.ts:1 | 73 | scan; 43, 56 |
| web/src/features/libraries/useLibraryScan.ts:1 | 33 | scan; 4, 8, 9, 10, 13, 15, 23 |
| web/src/features/library-add/ProviderSearchDialog.tsx:1 | 240 | scan; 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 62, 63, 64, 65, 66, 68, 69, 70, 71, 72 |
| web/src/features/media/AlbumPage.tsx:1 | 212 | scan; 19, 20, 38, 95, 99, 103, 106, 107, 108, 117, 137, 199, 206 |
| web/src/features/media/ArtistPage.tsx:1 | 58 | scan; 7, 17, 21, 25, 26 |
| web/src/features/media/AudiobookPage.tsx:1 | 234 | scan; 18, 19, 38, 96, 100, 104, 107, 109, 111, 112, 121, 167, 219, 221, 228 |
| web/src/features/media/CollectionPage.tsx:1 | 69 | scan; 7, 17, 21, 25, 26, 27 |
| web/src/features/media/DetailShell.tsx:1 | 19 | scan; presentation/local composition/no direct data match |
| web/src/features/media/EpisodeList.tsx:1 | 259 | scan; 12, 131, 132 |
| web/src/features/media/FileDetails.tsx:1 | 83 | scan; 7, 16, 17 |
| web/src/features/media/MediaActions.tsx:1 | 35 | scan; presentation/local composition/no direct data match |
| web/src/features/media/MoviePage.tsx:1 | 193 | scan; 15, 16, 17, 18, 19, 20, 21, 22, 24, 43, 46, 47, 48, 49, 51, 52, 53, 54, 59, 188 |
| web/src/features/media/ShowPage.tsx:1 | 154 | scan; 13, 14, 35, 38, 47, 49, 52, 53, 58, 75, 129, 130, 149 |
| web/src/features/media/seasons.ts:1 | 60 | scan; presentation/local composition/no direct data match |
| web/src/features/media/useAudioPages.ts:1 | 83 | scan; presentation/local composition/no direct data match |
| web/src/features/media/useMediaProgress.ts:1 | 41 | scan; 4, 7, 24, 25, 26 |
| web/src/features/media/useShowEpisodes.ts:1 | 99 | scan; 4, 7, 10, 13, 34, 61, 64, 70, 80 |
| web/src/features/player/NowPlayingSheet.tsx:1 | 96 | scan; presentation/local composition/no direct data match |
| web/src/features/player/PlayerDock.tsx:1 | 181 | scan; presentation/local composition/no direct data match |
| web/src/features/player/VideoPlayer.tsx:1 | 410 | scan; 20, 49, 54, 76 |
| web/src/features/player/WatchPage.tsx:1 | 94 | scan; 5, 26, 29, 30, 63 |
| web/src/features/player/controls.tsx:1 | 172 | scan; presentation/local composition/no direct data match |
| web/src/features/player/store.ts:1 | 119 | scan; presentation/local composition/no direct data match |
| web/src/features/player/useMediaSource.ts:1 | 101 | scan; 3 |
| web/src/features/player/usePlayer.ts:1 | 40 | scan; presentation/local composition/no direct data match |
| web/src/features/player/useProgressSync.ts:1 | 133 | scan; 5, 6, 7, 8, 9, 10, 28, 33, 64, 65, 69, 70, 76, 77, 96, 97, 107, 120 |
| web/src/features/search/ProviderResults.tsx:1 | 131 | scan; 9, 10, 11, 12, 13, 14, 15, 16, 46, 47, 48, 49, 50, 51, 52, 53 |
| web/src/features/search/SearchPage.tsx:1 | 157 | scan; 10, 36, 38 |
| web/src/features/settings/AboutSettings.tsx:1 | 46 | scan; presentation/local composition/no direct data match |
| web/src/features/settings/BackupSettings.tsx:1 | 63 | scan; 7, 16, 20, 21, 22, 29, 56 |
| web/src/features/settings/BlockedReleases.tsx:1 | 88 | scan; 8, 9, 33, 35, 41 |
| web/src/features/settings/CastingSettings.tsx:1 | 138 | scan; 11, 12, 13, 14, 15, 16, 17, 18, 19, 38, 39, 41, 44, 45, 46, 47, 48, 49, 63, 75, 90, 91, 110, 134 |
| web/src/features/settings/DownloadsSettings.tsx:1 | 116 | scan; 10, 35, 36, 37 |
| web/src/features/settings/GeneralSettings.tsx:1 | 76 | scan; 26, 27, 28 |
| web/src/features/settings/LibrariesSettings.tsx:1 | 148 | scan; 13, 33, 37, 50 |
| web/src/features/settings/LogsSettings.tsx:1 | 127 | scan; 11, 47, 48, 49, 82, 90, 104 |
| web/src/features/settings/MetadataSettings.tsx:1 | 184 | scan; 12, 41, 42, 43, 44, 45 |
| web/src/features/settings/OrderedChips.tsx:1 | 122 | scan; presentation/local composition/no direct data match |
| web/src/features/settings/OrganizationSettings.tsx:1 | 105 | scan; 10, 29, 35, 39, 40, 41, 57, 69, 101 |
| web/src/features/settings/QualitySettings.tsx:1 | 240 | scan; 11, 12, 13, 14, 82, 86, 87, 88, 113, 125, 231 |
| web/src/features/settings/SettingsForm.tsx:1 | 40 | scan; presentation/local composition/no direct data match |
| web/src/features/settings/SettingsLayout.tsx:1 | 53 | scan; presentation/local composition/no direct data match |
| web/src/features/settings/SettingsLoadError.tsx:1 | 11 | scan; presentation/local composition/no direct data match |
| web/src/features/settings/SourceDialog.tsx:1 | 219 | scan; 10, 11, 12, 13, 39, 42, 75, 76 |
| web/src/features/settings/SourcesSettings.tsx:1 | 137 | scan; 7, 20, 24, 25, 26, 27, 34, 48, 57, 113, 128 |
| web/src/features/settings/UsersSettings.tsx:1 | 116 | scan; 11, 12, 13, 14, 15, 16, 17, 33, 34, 35, 39, 40, 41, 42, 61, 75, 76, 87, 112 |
| web/src/features/settings/useAppSettings.ts:1 | 74 | scan; 4, 10, 12, 27, 29, 32 |
| web/src/features/settings/useSettingsConnection.ts:1 | 126 | scan; 1, 25, 46, 85 |
| web/src/features/wanted/WantedPage.tsx:1 | 240 | scan; 13, 14, 15, 16, 17, 18, 55, 56, 57, 58, 59, 60, 71, 95, 117, 181, 193, 206 |
| web/src/features/wanted/grouping.ts:1 | 73 | scan; presentation/local composition/no direct data match |
| web/src/features/wanted/useWanted.ts:1 | 220 | scan; 5, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 49, 50, 55, 56, 61, 62, 67, 68, 73, 80, 81, 82, 83, 90, 91, 92, 93, 187, 188, 190, 191, 192, 193, 194 |
| web/src/hooks/useContentStatuses.ts:1 | 82 | scan; 4, 18, 45, 46, 53, 79, 80 |
| web/src/hooks/useDebouncedValue.ts:1 | 11 | scan; 6 |
| web/src/hooks/useMediaQuery.ts:1 | 19 | scan; presentation/local composition/no direct data match |
| web/src/hooks/useServerTable.ts:1 | 67 | scan; presentation/local composition/no direct data match |
| web/src/lib/api/urls.ts:1 | 75 | scan; 10, 14, 23, 27, 61 |
| web/src/lib/apollo/client.ts:1 | 80 | scan; 27, 37, 78 |
| web/src/lib/apollo/typePolicies.ts:1 | 31 | scan; presentation/local composition/no direct data match |
| web/src/lib/artwork.ts:1 | 26 | scan; 1 |
| web/src/lib/auth/session.ts:1 | 210 | scan; 15, 39, 97, 117, 146, 186 |
| web/src/lib/auth/useSession.ts:1 | 14 | scan; presentation/local composition/no direct data match |
| web/src/lib/format.ts:1 | 111 | scan; presentation/local composition/no direct data match |
| web/src/lib/glass/appearance.ts:1 | 38 | scan; presentation/local composition/no direct data match |
| web/src/lib/glass/brand.ts:1 | 16 | scan; presentation/local composition/no direct data match |
| web/src/lib/graphql/content-status.ts:1 | 11 | scan; presentation/local composition/no direct data match |
| web/src/lib/graphql/errors.ts:1 | 41 | scan; presentation/local composition/no direct data match |
| web/src/lib/graphql/glass-table.ts:1 | 73 | scan; presentation/local composition/no direct data match |
| web/src/lib/input-mode.tsx:1 | 191 | scan; presentation/local composition/no direct data match |
| web/src/lib/languages.ts:1 | 52 | scan; presentation/local composition/no direct data match |
| web/src/lib/library-types.ts:1 | 81 | scan; presentation/local composition/no direct data match |
| web/src/lib/prefs.ts:1 | 67 | scan; presentation/local composition/no direct data match |
| web/src/lib/pwa.ts:1 | 26 | scan; 12 |
| web/src/lib/status.ts:1 | 87 | scan; presentation/local composition/no direct data match |
| web/src/lib/theme.tsx:1 | 69 | scan; presentation/local composition/no direct data match |
| web/src/lib/utils.ts:1 | 21 | scan; presentation/local composition/no direct data match |
| web/src/main.tsx:1 | 44 | scan; presentation/local composition/no direct data match |
| web/src/routeTree.gen.ts:1 | 1152 | scan; presentation/local composition/no direct data match |
| web/src/routes/__root.tsx:1 | 32 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app.tsx:1 | 19 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/activity.tsx:1 | 7 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/activity/index.tsx:1 | 8 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/albums/$albumId.tsx:1 | 14 | scan; 4, 9 |
| web/src/routes/_app/artists/$artistId.tsx:1 | 14 | scan; 4, 9 |
| web/src/routes/_app/audiobooks/$audiobookId.tsx:1 | 14 | scan; 4, 9 |
| web/src/routes/_app/collections/$collectionId.tsx:1 | 14 | scan; 4, 9 |
| web/src/routes/_app/downloads.tsx:1 | 7 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/downloads/index.tsx:1 | 8 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/index.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries.tsx:1 | 7 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/$libraryId.tsx:1 | 18 | scan; 4, 9 |
| web/src/routes/_app/libraries/$libraryId/albums.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/$libraryId/artists.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/$libraryId/audiobooks.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/$libraryId/collections.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/$libraryId/files.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/$libraryId/index.tsx:1 | 15 | scan; 4, 9 |
| web/src/routes/_app/libraries/$libraryId/movies.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/$libraryId/scans.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/$libraryId/settings.tsx:1 | 15 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/$libraryId/shows.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/$libraryId/tracks.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/libraries/index.tsx:1 | 8 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/movies/$movieId.tsx:1 | 14 | scan; 4, 9 |
| web/src/routes/_app/search.tsx:1 | 7 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/search/index.tsx:1 | 8 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/about.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/backup.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/casting.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/downloads.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/general.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/index.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/libraries.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/logs.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/metadata.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/organization.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/quality.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/sources.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/settings/users.tsx:1 | 9 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/shows/$showId.tsx:1 | 14 | scan; 4, 9 |
| web/src/routes/_app/wanted.tsx:1 | 7 | scan; presentation/local composition/no direct data match |
| web/src/routes/_app/wanted/index.tsx:1 | 8 | scan; presentation/local composition/no direct data match |
| web/src/routes/_auth.tsx:1 | 34 | scan; presentation/local composition/no direct data match |
| web/src/routes/_auth/login.tsx:1 | 27 | scan; 5, 17 |
| web/src/routes/_auth/register.tsx:1 | 27 | scan; 5, 15 |
| web/src/routes/_player.tsx:1 | 15 | scan; presentation/local composition/no direct data match |
| web/src/routes/_player/watch/$mediaFileId.tsx:1 | 12 | scan; presentation/local composition/no direct data match |
| web/src/vite-env.d.ts:1 | 11 | scan; presentation/local composition/no direct data match |

## Appendix E — Tooling, document and non-production scan boundaries

Production manifest excludes 53 colocated TS/TSX test/helper files, 3 codegen artifacts, and test setup/styles/assets. Tests were **not run**. Test-only gql literals are intentionally outside the production ban. Codegen artifacts were inspected by exported-name inventory, not treated as handwritten API usage.

Static generated-output count: 511 exported Document constants. Source operation names absent from those exports: none. This checks symbol presence only, not artifact freshness or operation validation.

Feature document files (all 19):

- web/src/graphql/documents/activity.graphql:1 — 2 queries / 1 mutations / 0 subscriptions.
- web/src/graphql/documents/auth.graphql:1 — 2 queries / 4 mutations / 0 subscriptions.
- web/src/graphql/documents/backup.graphql:1 — 1 queries / 3 mutations / 0 subscriptions.
- web/src/graphql/documents/cast.graphql:1 — 1 queries / 10 mutations / 0 subscriptions.
- web/src/graphql/documents/content-status.graphql:1 — 1 queries / 0 mutations / 0 subscriptions.
- web/src/graphql/documents/files.graphql:1 — 0 queries / 5 mutations / 1 subscriptions.
- web/src/graphql/documents/home.graphql:1 — 14 queries / 0 mutations / 0 subscriptions.
- web/src/graphql/documents/libraries.graphql:1 — 7 queries / 6 mutations / 0 subscriptions.
- web/src/graphql/documents/library-lists.graphql:1 — 10 queries / 0 mutations / 0 subscriptions.
- web/src/graphql/documents/media.graphql:1 — 14 queries / 8 mutations / 0 subscriptions.
- web/src/graphql/documents/playback.graphql:1 — 3 queries / 0 mutations / 0 subscriptions.
- web/src/graphql/documents/quality.graphql:1 — 2 queries / 0 mutations / 0 subscriptions.
- web/src/graphql/documents/schedule.graphql:1 — 4 queries / 0 mutations / 0 subscriptions.
- web/src/graphql/documents/search.graphql:1 — 7 queries / 7 mutations / 0 subscriptions.
- web/src/graphql/documents/settings.graphql:1 — 2 queries / 5 mutations / 0 subscriptions.
- web/src/graphql/documents/shell.graphql:1 — 3 queries / 0 mutations / 0 subscriptions.
- web/src/graphql/documents/sources.graphql:1 — 2 queries / 2 mutations / 0 subscriptions.
- web/src/graphql/documents/torrents.graphql:1 — 2 queries / 6 mutations / 4 subscriptions.
- web/src/graphql/documents/wanted.graphql:1 — 5 queries / 0 mutations / 0 subscriptions.

All remaining web scripts/config/workflows inventoried for transport, schema and generation entry points (static scan only):

- web/scripts/generate-entity-documents.mjs:1 — relevant matches at 3, 4, 12, 14, 15, 29, 32, 33, 35, 36, 37, 38, 111, 119, 260, 261, 265.
- web/scripts/generate-film-graph.mjs:1 — relevant matches at 28, 30, 45, 48, 73, 92, 114, 189.
- web/scripts/generate-icons.mjs:1 — no GraphQL/polling/generation match.
- web/scripts/generate-route-tree.mjs:1 — relevant matches at 15, 22.
- web/scripts/pull-schema.mjs:1 — relevant matches at 3, 4, 6, 11, 13, 14, 16.
- .github/workflows/ci.yml:1 — relevant matches at 58, 59, 77, 79, 80, 83, 84, 86, 89, 90.
- .github/workflows/release.yml:1 — relevant matches at 196.
- web/.gitignore:1 — relevant matches at 9.
- web/README.md:1 — relevant matches at 13, 14, 19, 34, 35, 36, 42, 43, 85, 86, 87, 92, 93, 94, 106.
- web/codegen.ts:1 — relevant matches at 1, 5, 8, 9, 11, 12.
- web/package.json:1 — relevant matches at 12, 14, 15, 16, 17, 19, 22, 23, 24, 31, 40, 41, 55, 56, 57.
- web/playwright.config.ts:1 — no GraphQL/polling/generation match.
- web/tsconfig.build.json:1 — no GraphQL/polling/generation match.
- web/tsconfig.json:1 — relevant matches at 24.
- web/vite.config.ts:1 — relevant matches at 18, 37, 67, 106, 150.
- web/vitest.setup.ts:1 — no GraphQL/polling/generation match.

Colocated test/helper file inventory (paths scanned, no execution):

- web/src/components/constellation/__tests__/graph.test.ts:1
- web/src/components/constellation/__tests__/simulation.test.ts:1
- web/src/components/shell/__tests__/Breadcrumbs.test.tsx:1
- web/src/components/shell/__tests__/RouteError.test.tsx:1
- web/src/components/shell/__tests__/nav.test.ts:1
- web/src/components/ui/__tests__/AlphabetRail.test.tsx:1
- web/src/components/ui/__tests__/Button.test.tsx:1
- web/src/components/ui/__tests__/DataTable.test.tsx:1
- web/src/components/ui/__tests__/Dialog.test.tsx:1
- web/src/components/ui/__tests__/GlassButton.test.tsx:1
- web/src/components/ui/__tests__/GlassControls.test.tsx:1
- web/src/components/ui/__tests__/LoadedDataTable.test.tsx:1
- web/src/components/ui/__tests__/MediaCards.regressions.test.tsx:1
- web/src/components/ui/__tests__/PosterCard.test.tsx:1
- web/src/components/ui/__tests__/Tabs.test.tsx:1
- web/src/components/ui/__tests__/states.test.tsx:1
- web/src/features/acquisition/__tests__/AcquisitionDialog.test.tsx:1
- web/src/features/downloads/__tests__/ReleaseSearchDialog.test.tsx:1
- web/src/features/home/__tests__/MovieReleases.test.tsx:1
- web/src/features/home/__tests__/TvGuide.test.tsx:1
- web/src/features/home/__tests__/continue-watching.test.ts:1
- web/src/features/libraries/browser/__tests__/useInfiniteConnection.test.tsx:1
- web/src/features/media/__tests__/AudioPages.test.tsx:1
- web/src/features/media/__tests__/EpisodeList.test.tsx:1
- web/src/features/media/__tests__/MediaActions.test.tsx:1
- web/src/features/media/__tests__/mediaHookHarness.tsx:1
- web/src/features/media/__tests__/seasons.test.ts:1
- web/src/features/media/__tests__/useMediaProgress.test.tsx:1
- web/src/features/media/__tests__/useShowEpisodes.test.tsx:1
- web/src/features/player/__tests__/WatchPage.test.tsx:1
- web/src/features/settings/__tests__/DownloadsSettings.test.tsx:1
- web/src/features/settings/__tests__/LogsSettings.test.tsx:1
- web/src/features/settings/__tests__/OrderedChips.test.tsx:1
- web/src/features/settings/__tests__/QualitySettings.test.tsx:1
- web/src/features/settings/__tests__/useAppSettings.test.ts:1
- web/src/features/settings/__tests__/useSettingsConnection.test.tsx:1
- web/src/features/wanted/__tests__/grouping.test.ts:1
- web/src/features/wanted/__tests__/useWanted.test.tsx:1
- web/src/hooks/__tests__/useContentStatuses.test.tsx:1
- web/src/lib/__tests__/artwork.test.ts:1
- web/src/lib/__tests__/entity-documents.test.ts:1
- web/src/lib/__tests__/format.test.ts:1
- web/src/lib/__tests__/input-mode.test.tsx:1
- web/src/lib/__tests__/library-types.test.ts:1
- web/src/lib/__tests__/prefs.test.tsx:1
- web/src/lib/__tests__/status.test.ts:1
- web/src/lib/__tests__/theme.test.tsx:1
- web/src/lib/apollo/__tests__/client.test.ts:1
- web/src/lib/auth/__tests__/session.test.ts:1
- web/src/lib/graphql/__tests__/content-status.test.ts:1
- web/src/lib/graphql/__tests__/entity-subscriptions.test.ts:1
- web/src/lib/graphql/__tests__/errors.test.ts:1
- web/src/lib/graphql/__tests__/glass-table.test.ts:1
