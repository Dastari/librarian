# Librarian — Design Document

## Purpose
This document is the **single source of truth** for what Librarian is, what it supports, the architecture it runs on, and the rules agents must follow when building code. It describes the **intended completed system**, not a partial or transitional state.

## Product Summary
Librarian is a locally hostable media library management system with a built-in torrent client. It automates acquisition, organization, discovery, and playback of personal media libraries while remaining privacy‑preserving and local‑first.

### Supported Library Types
- **Movies** (standalone entries) with **Movie Collections**
- **TV Shows** with **Episodes**
- **Music** with **Tracks** and **Artists**
- **Audiobooks** with **Chapters**

### Core Capabilities
- Built-in torrent client with DHT, UPNP, and seeding control
- Multiple sources: public/private torrent indexers and RSS feeds
- Auto-download for missing items (episodes/chapters/tracks/movies)
- Automatic organization into a configurable naming structure
- Persistent video player (YouTube-like) and music player (Spotify-like)
- Casting support: **Chromecast/Google Cast**; DLNA discovery is informational only
- Local-first storage and offline usage

### Movie Collections Behavior
- Collection entities store provider collection metadata within a library; movies retain `CollectionId`, `CollectionName` and `CollectionPosterUrl`.
- Generated `Collection.movies` membership matches both the library ID and provider collection ID. A collection must never include another library's matching provider IDs.
- Movie add and movie metadata refresh must both update collection fields from TMDB so collection membership stays current.
- Collection poster URLs should be cached through the artwork service and persisted as local `/api/artwork/...` URLs when available.

### Personal library organization

- **Watch Later is viewing intent**, independent of Wanted, monitoring, quality profiles and automatic acquisition. Saving a title never imports metadata or starts a download.
- `PersonalList` supports manual lists/playlists, smart collections and one canonical `Watch Later` list per user. `ListEntry` has one typed Movie, Show, Episode, Album, Track, Audiobook or Chapter target and a nonnegative ordering position, so a playlist holds any media. Positions are spaced by 1024 and a move writes one position between its neighbours; when a move lands on an occupied position (no gap left, or two clients appending at once) the `ListEntry` lifecycle hook settles the tie inside the same transaction, placing a moved entry before and a new one after what it tied with, and respaces the list. Playlists are an entity in the UI: `PlaylistIdentity` (a mosaic of up to four covers and state badges), `usePlaylistActions`, `PlaylistSettingsDialog` and the `/lists` pages. "Add to playlist…" is an action of every media entity and of table bulk bars (`useAddToPlaylist`). Playing a playlist expands it with `playlistPlayItems` (tracks and chapters as audio, movies and episodes as video; albums, shows and audiobooks to their playable children in order) and puts the result in the player's queue in order or shuffled.
- Personal rows belong to the actual signed-in user, including administrator accounts. Administrator access to shared management does not grant access to another person's lists, saved rules, shelf preferences or favourites.
- Version-one `SavedView` rules contain a media type, optional owned library, title substring, availability and sort/direction. They compile to generated entity Where/order inputs. Shows support unrestricted availability only until a canonical generated show-availability predicate exists. Empty library scopes must never become an omitted IN predicate.
- Deleting a scoped library removes its saved rules; it must not broaden them to every library. Personal association writes are validated inside the generated mutation transaction, with generated subscriptions and surviving-parent invalidation.
- `HomeShelf` stores visibility, order and optional list/library references per user. Built-in shelves work without preference rows; personal lists can be pinned. `LibraryPreference` stores favourites independently of the shared Library record.
- Browser routes `/lists` and `/lists/:listId` manage these features. Title actions and track menus save entries; Home customization controls shelves; library cards control favourites. Complete lists filter before incremental rendering; smart collections filter and load through server queries.
- These models are implemented in the working tree. Runtime and migration qualification is recorded separately; schema generation alone does not qualify their authorization or transaction behavior.

---

## Core Rules and Conventions
These rules govern all implementation decisions.

### Agent Rule Source of Truth
When multiple agents are working in this repository, they must follow the same rule stack and precedence:

1. `docs/design.md` (this document) for product + architecture + system constraints.
2. `AGENTS.md` for repository workflow requirements.
3. `.cursor/rules/*.mdc` for implementation rules (frontend, naming, entity system, logging, media pipeline, quick reference, development).

For frontend work, `.cursor/rules/frontend-ui.mdc` is mandatory and should be treated as the canonical UI implementation guide.
For backend entity/GraphQL work, `.cursor/rules/entity-single-source-of-truth.mdc` and `.cursor/rules/graphql-naming-convention.mdc` are mandatory.

### GraphQL and the Entity System
- **GraphQL is the only data API**. All CRUD should go through the GraphQL entity system.
- The custom entity system is the **single source of truth** for **both** database schema and GraphQL types.
- Entities live in the backend GraphQL services layer and generate:
  - GraphQL schema and resolvers
  - Database schema and migrations
  - Subscriptions and change events
- Resolver macros support **Where**, filtering, sorting, pagination, and infinite scroll
- Entity macros support **field-level transform hooks** and **input-only fields** (see Entity Field Transform Hooks below)
- **GraphQL resolvers are the primary code path** for reading and writing data.
- The only non-GraphQL HTTP endpoints are under `/api` (file uploads, artwork, static asset serving, and media streaming).
- Subscriptions use WebSockets for real-time updates.
- Ordinary persisted reads use generated roots and nested relations. Computed provider/runtime reads remain explicit exceptions; Queries perform no imports, repairs or reconnect effects.
- Related writes use the originating generated mutation transaction. Generated direct and parent notifications publish after commit; deletion, reparenting and nested-only dependencies require explicit surviving-parent invalidation.
- Upward change delivery follows `propagate_change = "up"` on the child's to-one relation (single or composite key, `emit_fk = false` when no foreign key exists). A child that changes often, such as a `Torrent` synced every 10 seconds, must propagate rather than write its parents: propagation emits an event and writes no parent row. It covers creates and updates from the post-update row only, ignores `source_condition` (so `PendingFileMatch.torrent` stays a hand-written lookup), and needs the subscriber to be able to read the source. A delete or a move leaves no surviving source, so those cases keep an explicit parent write in `AppMutationHook::maintain_summaries_and_parent_events`.
- A relation's `on_delete` states what really happens. With `emit_fk = false` the ORM enforces nothing, so the declared policy is whatever `AppMutationHook` does (`cascade`: it deletes the rows; `set_null`: `detach_deleted_references` or `cleanup_media_file_dependents` clears the id; `restrict`: a hook refuses the delete). Physical cascades skip hooks and events, so a child that needs events, hooks or artwork cleanup is deleted by a hook before the foreign key can act.
- Relations are not indexed for you: every column a relation joins on, or a hook filters by when it clears or deletes children, must lead a declared or `#[filterable]` index (`every_relation_join_column_leads_an_index`).
- `DatabaseService::from_config` sets `EVENT_CHANNEL_CAPACITY` for the per-entity `Changed` channels before any subscription or write. Propagated events share the parent's channel, so a library delete or first scan can lag every subscriber of one entity at once; the capacity is sized from the live table row counts.
- Active frontend reads register scoped subscription dependencies, preserve complete loaded windows, and reconcile after reconnect or missed events. HTTP retry may repeat reads, never action mutations.
- Current session authority is checked before socket operation dispatch and before subscription delivery. Signed connection credentials alone do not authorize a revoked or demoted session.
- Session role claims may be narrower than the current account's roles; validation must not promote them. Logout revokes the identified session family, including a successor issued by refresh rotation. Credential changes, role changes and account disablement revoke persisted sessions in the originating transaction.
- Server filesystem roots and runtime torrent identities require administrator/internal authority. Checked action workflows preserve the authorized owner when using internal generated mutations; system actors never become arbitrary record owners.

### Generated Resolver Contract (Macro-Generated)
GraphQL resolvers for entity CRUD are generated by derive macros and are part of the API contract. Agents must assume these generated operations exist for every entity (subject to schema generation):

- Queries:
  - `<plural>(where, orderBy, page)`
  - `<single>(id)`
- Mutations:
  - `create<Single>(input)`
  - `update<Single>(id, input)` (single-row update)
  - `update<Plural>(where, input)` (bulk update by filter)
  - `delete<Single>(id)`
  - `delete<Plural>(where)` (bulk delete by filter)
- Subscriptions:
  - `<single>Changed(filter)`

Bulk update and bulk delete mutations must require a non-empty `Where` filter.
Single-id update/delete mutations remain valid and should not be removed when adding bulk operations.

### Naming Conventions
- **GraphQL resolver fields**: **camelCase** for all query, mutation, subscription, object, input, and payload fields. PascalCase resolver fields are legacy and should be migrated away from when touched.
- **GraphQL type names**: `PascalCase` for object, input, enum, interface, union, and scalar type names.
- **Rust**: `snake_case` for functions/modules/variables, `PascalCase` for types/traits.
- **Database**: `snake_case` for tables/columns; plural table names.
- **Serde**: Serde casing applies only to JSON payloads; it is not a substitute for GraphQL resolver naming.

### Entity Field Transform Hooks

The derive macros (`GraphQLEntity`, `GraphQLOperations`) support per-field transform hooks for data manipulation during reads and writes. These are used when a field requires processing (encryption, formatting, etc.) that is tightly coupled to the entity lifecycle.

#### `#[transform(write = "path::to::fn", read = "path::to::fn")]`

- **Write transform**: An async, context-aware function called before INSERT or UPDATE. Has access to the full GraphQL `Context`, enabling service lookups (e.g., encryption keys).
  - Signature: `async fn(&async_graphql::Context<'_>, String) -> async_graphql::Result<String>`
  - Injected into all relevant `insert_binds` and `update_field_checks` branches in the macro-generated mutations.
- **Read transform**: A synchronous function applied when reading a row from the database.
  - Signature: `fn(String) -> String`
  - Injected into `generate_row_field_assignment()` and `generate_option_row_field_assignment()`.
- Both are optional and independent; a field can have write-only, read-only, or both.

#### `#[input_only]`

Overrides `#[graphql(skip)]` for mutation inputs. The field remains hidden from GraphQL query output but appears in the macro-generated `CreateXInput` and `UpdateXInput` types. This enables **write-only fields** — data that can be submitted via mutations but is never returned in queries.

#### Example: Encrypted credentials on Source entity

```rust
#[graphql(skip)]           // Hidden from query output
#[input_only]              // But included in CreateSourceInput / UpdateSourceInput
#[transform(write = "crate::services::sources::encryption::encrypt_credentials_ctx")]
pub credentials: String,
```

The frontend sends plaintext JSON credentials in a single `CreateSource` mutation. The write transform encrypts them before the INSERT. The field is never exposed in query results.

#### Limitations

- Write transforms only apply to `String`-typed fields (and `Option<String>`). Boolean, JSON, and numeric fields are not transformed.
- Write transforms are async and require `&Context`, so they can only be used in the GraphQL mutation path (not in direct SQL calls outside of GraphQL).
- Read transforms are synchronous and do not have access to the GraphQL context or services.
- There is no built-in rollback if a transform succeeds but the subsequent database write fails.
- Transform functions must be importable as a fully-qualified Rust path from the macro expansion site.

### Media Pipeline Rules
- **Source-agnostic matching boundary**: Matching accepts normalized files from the built-in torrent
  client, manual imports, and library scans. Future acquisition adapters may reuse that boundary only
  after they implement a real, qualified download/import path.
- **Source-defined file handling**: Each source decides **copy vs move** and **when deletion is allowed** (e.g., after seeding for X days). The library pipeline follows the source policy.
- **Library ownership**: Once in the library, files are owned by the library; unlinking a download source never removes library files.
- **Quality is verified**: FFprobe is authoritative; quality is not assumed.
- **No auto-delete conflicts**: Conflicts require explicit user resolution.
- **Status reflects reality**: the computed status derives from `wanted` and file linkage (e.g., pending matches imply “downloading”, linked files imply “downloaded”).

---

## System Architecture

### Backend
- **Rust** service using **Axum** and **async-graphql**
- Jobs and background services managed via a service manager
- Custom entity system generating GraphQL + DB schema
- `/api` endpoints for file upload, artwork, static assets, and media streaming

### Frontend
- **Vite SPA/PWA** with **TanStack Router**
- **Glass UX + Tailwind** for the UI system; existing HeroUI/custom presentation migrates in the slices defined in [the migration plan](glass-ux-migration-plan.md). Librarian retains media-specific compositions, navigation, playback and data policy.
- **Apollo Client** for GraphQL queries/mutations/subscriptions
- GraphQL operations and generated types are the primary data contract

### Storage
- **SQLite** for structured metadata only
- **graphql-orm-storage** for object bytes that should not live in the database
- **Artwork cache** metadata is stored in SQLite; cached artwork bytes are stored outside the database through object storage
- Artwork is always served locally. `/api/artwork/{entity}/{id}/{kind}` is cache-through: a miss downloads the entity's provider URL (allowlisted hosts, public addresses only) into object storage, then serves it. Entities carry provider URLs only as the cache's source; clients never hotlink them. Kinds: movie/show poster and backdrop, season poster, episode thumbnail (`stillUrl`), album/audiobook cover, artist poster, person profile, collection poster/backdrop (by TMDB collection id), TV guide poster (by TVMaze show id)
- `?w=` serves a resized copy at one of 185/342/500/780/1280/1920 px, generated once from the original and stored beside it (never upscaled). A provider changing its image replaces the cached original and drops its resized copies on the next metadata refresh. Concurrent misses for one image share a single download; responses carry an ETag and answer `If-None-Match` with 304
- Cached artwork lives and dies with its entity: deleting a movie, show, season, episode, album, artist, audiobook or person deletes its artwork rows (originals and resized copies) in the same transaction, and the storage hooks remove the bytes after commit. Deleting a show deletes its seasons through the entity layer so their hooks run. Collection artwork goes with the last library collection of that TMDB id; TV guide posters go when the guide no longer lists the show. The hourly guide job also sweeps artwork whose entity no longer exists
- Seasons (`seasons`) hold TVMaze season names, dates, episode counts and poster sources; they sync with episodes on add/refresh. The TV guide job also backfills seasons, backdrops (TVMaze background images) and episode stills for shows added earlier, a few shows per hour
- Artist pictures (`Artist.imageUrl`, served as `artist/poster`) need no API key. An artist with a MusicBrainz id gets the Wikimedia Commons photo of its Wikidata item (`P18`, via the MusicBrainz `url-rels` Wikidata link), sized with `Special:FilePath?width=1000`. Without one, Deezer's public artist search supplies `picture_xl`, accepted only when the result's name equals the artist's after normalising case, punctuation and typographic apostrophes, and never Deezer's empty-picture placeholder. Found pictures are remembered in the metadata cache (`GetArtistImage` by MusicBrainz id, `GetArtistImageByName` by normalised name); misses are not. The picture is looked up, best effort and time-bounded, when an artist is created with an id, and a music library scan backfills up to 10 artists per scan that still have none (those with an id first). A picture already on the artist is never replaced. Commons and Deezer image hosts are in the artwork host allowlist
- **Filesystem** for media library files owned by library roots
- Optional external dependency: **ffprobe/ffmpeg** for analysis/transcoding

### Settings Loading
- Settings are stored in the `app_settings` table as key-value pairs.
- String settings should be loaded via `get_setting_string()` (raw read) rather than `get_setting::<String>()` (JSON parse) to avoid failures on non-JSON-quoted values.
- Settings checks for individual features (e.g., "is TMDB configured?") must query only the relevant key, not load all settings at once. A parse failure in an unrelated setting must not block feature availability.
- Parse errors in optional settings should log a warning and fall back to defaults, not propagate as fatal errors.

---

## Acquisition and Sources
Librarian's currently supported acquisition paths converge into the same pipeline:

- **Torrent** (built-in client) with DHT, UPNP, seeding control
- **RSS feeds** (`RssFeed`, admin-managed in Settings › Sources beside the indexers) are polled by the
  auto-download worker, kept as a catalogue of what is available (Q90–Q92) and matched exactly against wanted shows
  and movies (Q82); source definitions remain separate
- **Usenet/IRC:** not currently supported acquisition paths; settings or entities must not be presented
  as a working downloader
- **Manual Download** and direct add
- **Library Scanning** (existing files on disk)

### Guide discovery and air times

The `schedule_sync` service refreshes seven days of TVmaze country broadcast/local-streaming and global-streaming schedules daily, with stale-on-startup population and hourly retries after errors. Country partitions default to US and are configurable with `LIBRARIAN_SCHEDULE_COUNTRIES`. Provider failures preserve the previous cache; successful windows are deduplicated by country/provider episode ID and cancelled entries are removed. Fourteen days of past schedule entries are retained. Generated `scheduleCaches` supports `airStamp` sorting. Schedule discovery never imports shows or starts downloads.

`movieReleases(kind, region, page)` returns one day-cached TMDB now-playing/upcoming page using `MovieSearchResult`, including `releaseDate`. Cache identity includes kind, normalized region, and page. `Episode.airStamp` preserves TVmaze's timestamp on imports/refreshes; existing linked episodes with missing timestamps are backfilled without changing wanted/file/playback state. Unnumbered specials are represented in the public guide with number zero and are not assigned an ambiguous library season/episode slot.

`showcaseArtwork(limit)` is public and reads only medium-sized posters from the public schedule cache, not personal libraries. See [Guide backend API](guide-backend-api.md) for exact queries, defaults, and add-action integration.

### The auto-download hunt

`jobs::auto_download` is the single search-and-grab path (scheduled loop, `triggerAutoDownload`, and `searchMissing` all run it):

- **What is hunted** is decided by `autoDownloadMode` on the parent Show/Album/Audiobook; movies keep `monitored` + `wanted`. `NONE` never hunts, `WANTED` hunts only children flagged `wanted`, and `ALL` treats every missing, non-ignored, already-aired/released child as wanted — flipping `wanted = true` on those rows so status reflects reality (a future episode becomes wanted on its own once its air date passes). `autoDownload` stays the master switch and is normalised against the mode on read: mode `NONE` implies `autoDownload = false`, any other mode implies `true`.
- **Season packs** are hunted first when a season has at least 3 missing episodes or at least half its aired episodes missing *and* the profile allows packs. Both "Show S02" and "Show Season 2" are queried; a grabbed pack sets `season` and `showId` on the `Torrent` row and suppresses per-episode grabs for that season until it resolves. Only if no pack is grabbed does the job fall back to per-episode searches.
- **Search backoff**: a target is not re-searched more often than every 6 hours. The per-target timestamps live in the `auto_download.search_state` app setting so a restart does not reset them. `searchMissing` ignores the backoff.
- **Seeding obligations**: a grab records the source's `minimumRatio` and `minimumSeedTime` (seconds, stored as `Torrent.minimumSeedTimeMinutes`) so the seeding-policy loop can honour a private tracker's floor. Share ratio is computed from `Torrent.uploadedBytesTotal`, a cumulative counter the db-sync loop keeps monotonic across restarts (librqbit's `uploadedBytes` is per-session and resets), never from the session counter alone.
- **Release blocklist**: `ReleaseBlocklist` rows (matched on `infoHash` or `guid`) are never grabbed. The retry sweep adds an entry for every grabbed torrent whose import ended `failed` or that made zero progress for over 24 hours, then immediately re-hunts that target so a different release is tried.

### Sources

Librarian uses a unified **Sources** system for implemented torrent search/download endpoints. Source
types reserved for future protocols are schema vocabulary, not a claim that those downloaders exist.

#### Source Types
- **Torrent Indexer** — Private/public torrent trackers (e.g., IPTorrents)
- **Usenet Indexer** — reserved schema value; unavailable without an NNTP/NZB acquisition client
- **RSS Feed** — an `RssFeed` row polled on its own interval (Q82); only admins may create or edit one, because
  the server fetches its address. Shown and added in Settings › Sources as a kind of source, but stored as its own
  entity (Q90). Separate from the `RssFeed` source-type vocabulary

#### Source Architecture
- **`SourcesService`** — Lifecycle service registered with `ServicesManager`, depends on the database service. Manages encryption key creation and source loading.
- **`SourcesManager`** — Holds loaded `Source` trait objects, handles priority-ordered search across all enabled sources, and provides credential encryption.
- **`Source` trait** — Common interface that all source implementations must implement (`search`, `test_connection`, capabilities).
- **Source Definitions** — A static registry (`definitions/mod.rs`) describing available source types, their required credentials, and optional settings.

#### Credential Encryption
- Source credentials (cookies, API keys, passwords) are encrypted at rest using **AES-256-GCM**.
- The active key must come from an external environment/file key source and must be exactly 32 bytes;
  the database never contains the key needed to decrypt its own credentials.
- Credential ciphertext uses a versioned envelope carrying its key ID. Legacy database-key envelopes are
  migrated through the documented backup/dry-run/verify rotation workflow.
- Encryption happens transparently via the entity macro's `#[transform(write = "...")]` hook during `CreateSource` and `UpdateSource` mutations.
- Credentials are **never** exposed through GraphQL queries (`#[graphql(skip)]` + `#[input_only]`).
- The `SourcesService` decrypts credentials internally when loading sources for search operations.

#### Frontend Integration
- Sources are managed via `/settings/sources` with a table for listing, add/edit modals, and test connection functionality.
- Adding a source is a single step: the frontend sends credential JSON alongside all other fields in one `CreateSource` mutation.
- The `SearchSources` custom query searches across all enabled sources ordered by priority.

#### Key Files
| Purpose | File |
|---------|------|
| Source entity + custom ops | `backend/src/services/graphql/entities/source.rs` |
| Sources service | `backend/src/services/sources/service.rs` |
| Sources manager | `backend/src/services/sources/manager.rs` |
| Credential encryption | `backend/src/services/sources/encryption.rs` |
| Source definitions | `backend/src/services/sources/definitions/` |
| Web documents | `web/src/graphql/documents/sources.graphql` |
| Web settings page | `web/src/features/settings/SourcesSettings.tsx` |

---

## Unified Media Pipeline
All content flows through one unified pipeline regardless of source:
1. **Acquire**: Auto-download, RSS, manual download, direct add, or library scan
2. **Match**: Source-agnostic matching against wanted items
3. **Process**: Apply source-defined copy/move policy, create media records
4. **Analyze**: FFprobe extracts quality and metadata
5. **Organize**: Rename into library naming pattern
6. **Update**: Status transitions and subscriptions
This pipeline is the single code path for database and library changes.

### Expected User Outcomes (North Star)
The system must optimize for these real user expectations:

1. Existing library onboarding:
- User points Librarian at an already-populated folder.
- Scan discovers files, identifies media from filename/path first, then probed metadata.
- Missing catalog entries (shows, movies, albums, audiobooks) are created and enriched from metadata providers where available.
- Files are linked to created/found items so UI reflects actual file presence.
- Library scan must only invoke provider metadata fallback (TMDB/TVMaze/MusicBrainz/OpenLibrary create flows) for newly discovered media files. Previously known files should not repeatedly trigger provider fallback on subsequent scans.

2. Manual library curation:
- User can add media entries manually even before files exist.
- The system tracks `wanted` state separately from file presence.
- When matching files appear later (scan or downloads), they auto-link to those wanted items.

3. Source ingestion (torrent now, usenet/others later):
- Downloaded files are evaluated file-by-file through the same matcher path as scanner.
- Matching should prioritize explicit wanted gaps over unrelated candidates.
- On match, file is linked, analyzed, and organized under library rules.

4. Organization consistency:
- If auto-organize is enabled, files are moved/renamed into the active library naming pattern.
- Organization must remain deterministic across scanner/import/download flows.
- Folder cleanup must not remove protected expected library structure folders.

---

## Playback and Casting
- **Persistent video player** (YouTube-like)
- **Persistent music player** (Spotify-like)
- **Casting**: Chromecast/Google Cast playback. DLNA/UPnP renderers are discovery-only until a separately
  qualified AVTransport adapter exists. Librarian has no first-party AirPlay controller; Safari's native
  media-element AirPlay affordance is browser behavior, not a Librarian casting guarantee.
- **Streaming**: direct play when possible, otherwise HLS via FFmpeg
- **Track selection**: analyzed audio streams use their absolute FFprobe index. Selecting another
  language prepares an independent HLS variant and preserves the current position and pause state.
  Embedded, sidecar and downloaded text subtitles are delivered as authorized WebVTT with an
  optional timing offset; picture subtitles (PGS, VobSub) are burned into an on-demand transcode
  (Q118–Q120). Lyrics: Q121.
- **Resume**: checkpoints are personal and tied to the media file. Playback sessions retain the
  actual episode or audiobook parent identity; switching sources must not copy another file's position.
- **Receiver access**: Cast grants bind the issuing login session, media owner and receiver
  address/device/port. Every receiver fetch and checkpoint rechecks current authority. Grants last
  the known duration plus two hours, bounded to 4–24 hours; longer use requires recasting.

---

## Transcoding & Streaming
- **FFprobe** is the authoritative source for media metadata and quality
- **FFmpeg** is used for on-demand transcoding and HLS packaging
- Direct play is used when the client supports the container and codecs

---

## Organization and Naming
- Automatic organization per library with configurable patterns
- Multiple pre-existing naming patterns supported per media type
- Default pattern controlled by Librarian
- Organizer runs only within library folders

---

## Archive Extraction
Archive extraction is supported for **ZIP**, **RAR** (including multi-volume `.rar`+`.rNN` and
`.partNN.rar` sets) and **7z**, implemented in `backend/src/services/extract.rs`. No external
binary is required: `zip` and `sevenz-rust2` are pure Rust, and `unrar` statically links the
vendored RARLAB unrar C++ source. Extraction only ever runs inside the torrent import path
(`process_torrent_source_files`); the library scanner never unpacks anything.

Every gate below is mandatory; an archive is only unpacked when all of them pass.

- **Bounded staging.** Output goes to `<archive directory>/.librarian-extract/<info hash>/<archive>/`
  and nowhere else. Every entry's destination is recomputed and re-validated against that root.
- **Entry safety.** Absolute paths, `..` traversal, Windows drive/UNC prefixes, empty names,
  symlinks, hardlinks, reparse points and device/FIFO/socket entries abort the whole extraction.
- **Unpacked-byte budget.** `extract.max_unpacked_gb` (default 60) is checked against the archive's
  declared sizes *and* against bytes actually written, because headers can lie.
- **Entry-count limit.** `extract.max_entries` (default 20000).
- **Free-space check.** The declared unpacked size plus a 1 GB margin must fit on the staging
  filesystem, otherwise extraction is refused before any byte is written.
- **The archive is never deleted or modified** — the torrent has to keep seeding. Only the first
  volume of a multi-volume RAR set is opened; continuation volumes are skipped, not re-extracted.
- **Idempotent.** A completed extraction writes a marker file listing its output; re-running reuses
  it instead of unpacking again. A failed extraction deletes its partial staging output.
- **Durable, explanatory failure.** Any failure sets `Torrent.post_process_status = "failed"` with a
  `post_process_error` that names the cause: password protected, unsupported format, budget
  exceeded, too many entries, unsafe entry, insufficient space, or corrupt/missing volume.
- **Encrypted archives are never guessed at.** Librarian stores no archive passwords; an encrypted
  archive is a durable failure, not a silent skip.

Extracted media files are then imported through exactly the same path as plain torrent files. They
are Librarian's own copies rather than seeding payload, so they are *moved* out of staging into the
library (a hardlink to a staged file would be pointless) and the staging directory is removed after
the import pass finishes. Nested archives are not unpacked recursively.

---

## Quality Profiles
- Separate **video** and **audio** quality profiles
- Rules evaluate resolution, codec, HDR, source type, and audio specs
- Suboptimal media is detected and tracked for upgrades

### Release-selection rules
A profile also carries acquisition-side rules, used only when the hunt picks a release (`services/quality/profile.rs`):

| Field | Kind | Behavior |
|-------|------|----------|
| `preferredLanguages` | soft (hard with `requireLanguageMatch`) | Ordered ISO 639-1 codes. A release with no language tag counts as `en`; a `MULTi`/`DUAL` release satisfies any preference. |
| `requireLanguageMatch` | hard | Reject releases that do not declare a preferred language. |
| `minSizeMb` / `maxSizeMb` | hard | Per release for movies/books; **per episode** for TV (season-pack size divided by the season's episode count). |
| `minSeeders` | hard | Default 1. Sources that do not report seeders are not filtered. |
| `maxReleaseAgeDays` | hard | Null = any age. |
| `preferredReleaseGroups` | soft | Ordered; the existing white/blacklists remain the hard filters. |
| `allowSeasonPacks` | hard | Default true; false disables both pack hunting and pack grabs. |
| `preferProperRepack` | soft + upgrade | PROPER/REPACK wins ties, and counts as an upgrade over a non-PROPER file at the same resolution. |
| `resolutionPreference` | soft | Ordered; when empty the highest allowed resolution wins. Entries must still be inside `allowedResolutions`. |

Ranking precedence for the surviving candidates is strict and deterministic: resolution (preference order when set, otherwise highest parsed resolution) → PROPER/REPACK → language preference order → release-group preference order → seeders → title similarity → release title. The last key exists only so two otherwise identical releases always order the same way.

The seeded default profile ("Any Quality") leaves every list empty and sets `minSeeders: 1`, `allowSeasonPacks: true`, `preferProperRepack: true`.

---

## Library Scanning & Discovery
- Scheduled scans discover existing media
- Autoscan is driven by library settings (`auto_scan`, `scan_interval_minutes`)
- New files are inserted as `MediaFile` records, analyzed via queued ffprobe workers, then matched and optionally organized
- Missing files are reconciled by unlinking related entities and deleting stale `MediaFile` records
- Scanner/import matching and download matching should converge on the same matching logic and decision model

---

## Background Services
Long-running services include:
- Download monitor and completion processing
- Library scanning and file matching
- Organization and file processing
- Metadata fetching and extraction
- Streaming/transcoding services
- Indexer polling

---

## Auth Model
- Local authentication uses backend-managed HttpOnly access and rotating refresh cookies.
- Refresh credentials are never returned to JavaScript or GraphQL payloads.
- Cookie-authenticated mutations require an allowed same-origin/configured Origin; forwarded transport
  headers are honored only from configured trusted proxies.
- No external SSO or OAuth integration

---

## Distribution & Packaging
Librarian is distributed as a **single binary** with embedded frontend assets.

### Distribution goals (not all qualified)
- Windows and Linux binaries
- Windows installer/MSI are future packaging goals
- Optional bundled `ffprobe`/`ffmpeg`
- Embedded frontend served from a virtual filesystem
- No nginx in production deployments
- Service registration/integration remains deployment-specific; Windows service/tray mode is unavailable

### Frontend Asset Bundling
- Frontend `dist/` is embedded into the binary during release builds
- Assets are served from an in-memory virtual filesystem
- Optional build flag allows serving from disk if needed

### TV client prototype
- `apps/android-tv/` builds a sideloadable Android TV / Google TV debug client that loads the
  existing frontend from a user-selected server origin and preserves browser cookie authentication.
- This hosted WebView prototype reuses the browser player. Native playback, bundled TV frontend
  assets, other TV platforms, and physical-device compatibility remain unqualified.
- See the [TV application plan](tv-applications-plan.md) and [device testing guide](../apps/android-tv/README.md).

### Windows Packaging
- Installer includes `librarian.exe` and optional `ffprobe.exe`
- Windows service/tray installation remains a packaging target, not a currently qualified capability.

### Service Modes
- The currently qualified runtime is server mode.
- Tray mode and Windows `service install|uninstall|start|stop|status` remain disabled until Windows CI and
  VM lifecycle qualification exist; placeholder flags must return an explicit unsupported result.

### Linux Packaging
- Tarball distribution with optional install scripts
- `systemd` service unit support

### Docker
- Docker image is optional but supported for advanced users

---

## Frontend Refactor Targets
The intended final frontend architecture includes:
- **Apollo Client** for all GraphQL operations
- **Generated GraphQL types** are mandatory; no custom type extensions
- **Glass UX useAppForm + Zod** for new bound forms; existing react-hook-form bridges use Glass controls and retain one validation/submission owner per form
- Removal of deprecated library-level AutoDownload settings
- Consolidated page-level data fetching (avoid manual loading states)
- Prefer `previousData` patterns to avoid UI flicker on refetch
- Use public Glass UX components for UI primitives; media artwork, buffered seeking and acquisition/playback policy remain Librarian compositions

### Frontend Data Layer Standards (Mandatory)
- Use `useQuery`, `useMutation`, and `useSubscription` from Apollo as the default pattern for route pages and feature components.
- Avoid imperative `await graphqlClient...toPromise()` in routes/components. Treat this as legacy migration-only code.
- Keep local React state minimal:
  - Do not duplicate server state in `useState` when Apollo cache/query data can be used directly.
  - Do not create manual loading/error flags for GraphQL operations when hook state already provides `loading`/`error`.
  - Specifically: when using Apollo hooks, do not introduce custom `isLoading` state for the same operation.
- Use `previousData` on detail/list pages to prevent flicker during refetch, subscription-driven updates, and variable changes.
- Favor schema/codegen types over custom frontend GraphQL result interfaces:
  - Prefer `*Document`, `*Query`, `*Mutation`, and variable types from generated code.
  - Custom local types are only acceptable for temporary compatibility when a backend field/query is not yet in schema/codegen.
- Subscription-first UI behavior:
  - Library and media routes should subscribe to relevant change events (`libraryChanged`, `movieChanged`, `showChanged`, `albumChanged`, `trackChanged`, `audiobookChanged`, etc.).
  - On change events, refresh using query refetch/cache updates instead of bespoke polling loops.
- GraphQL naming consistency in frontend documents must follow backend schema conventions:
  - Resolver fields and payload keys should use camelCase where defined by schema.
  - Remove/replace legacy PascalCase resolver fields as backend migration completes.
- Prefer nested GraphQL relations over frontend post-processing transforms when the schema can provide needed relational data directly.
- New hooks in `web/src` must be justified:
  - Do not add wrapper hooks that only proxy Apollo hooks without adding clear reusable behavior.
  - Periodically decommission obsolete hooks after route migrations.

---

## Media Pipeline Decision Guide (Q&A)
The following Q&A is authoritative for pipeline behavior.

This guide defines the decision points in the media processing pipeline and the intended behavior for each scenario. Reference this guide when making changes to any media pipeline components.

### Current Backend Contract (Scanner/Matcher/Organizer)
When this section conflicts with older Q&A answers below, this section wins.

- Service: `backend/src/services/library_scan.rs`
- GraphQL mutations:
  - `scanLibrary(id)`
  - `analyzeMediaFile(mediaFileId, path)`
  - `matchMediaFile(input: { mediaFileId, libraryId?, episodeId?, movieId?, trackId?, chapterId?, methods? })`
  - `organizeMediaFile(input: { mediaFileId })`
- Matching methods currently supported:
  - `Filename`
  - `Metadata`
  - `Ollama` (optional method slot; pipeline must work without it)
- Scan flow:
  - Guard against concurrent scan per library (`scanning` flag + in-process lock)
  - Walk library path and insert missing media files
  - Queue ffprobe analysis jobs (worker-limited)
  - Run `matchMediaFile` for each discovered file
  - If no local wanted/entity match is found, attempt provider lookup/create (movies via TMDB, shows via TVMaze), then re-match/link
  - Matching should prefer explicit `wanted` gaps when multiple candidates are plausible
  - If `auto_organize=true`, run `organizeMediaFile` for matched files
  - If `auto_organize=true`, also organize files the scan skipped as unchanged that are not where the naming pattern puts them (Q78); nothing is re-matched or re-read
  - Reconcile deleted files from DB relations
  - For TV libraries, ensure expected show/season folders exist
  - If `auto_organize=true`, clean up empty non-protected folders (Q80)
- Organization rules:
  - Uses `library.naming_pattern` as primary naming template
  - If empty, resolves default from `naming_patterns` by `library_type`
  - If DB default missing, uses hardcoded safe fallback pattern
  - Renames/moves file on disk, then updates `MediaFile.Path` via GraphQL mutation
- Torrent completion import:
  - Uses `TorrentService::process_completed_torrent` as the single entrypoint for completion events, `processSource`, and `rematchSource`
  - Archives (`.zip`/`.rar`/`.7z`) are unpacked first into a bounded staging directory (see Archive Extraction) and their media files join the same per-file import pass
  - Sample/proof/extras and undersized video files are dropped before matching (Q72)
  - Wanted-target linkage recorded on the `Torrent` row (`movieId`/`episodeId`/`showId`+`season`/`albumId`/`audiobookId`/`trackId`/`chapterId`) decides the target when present; the generic matcher only runs for files with no linkage (Q71)
  - If `Torrent.LibraryId` is set, only that library is considered and auto-match requires confidence `>= 0.70`
  - If `Torrent.LibraryId` is null, all compatible libraries for the user are considered and auto-match requires confidence `>= 0.90` with a `>= 0.10` margin over the next candidate
  - Imports hardlink first, copy fallback, never move/delete torrent source files
  - Final `MediaFile.Path`, `RelativePath`, `LibraryId`, and target link are updated only after hardlink/copy succeeds
  - Copy conflicts never overwrite existing different files; they leave a `PendingFileMatch` copy error and create a warning notification
  - `Torrent.post_process_status` is `completed`, `partial`, `unmatched`, or `failed` based on actual processed/failed counts, and `post_process_error` always explains a non-`completed` result (Q73)
- TV folder protection and creation:
  - Derived from active naming pattern, not hardcoded `Season XX` assumptions
  - Protected folders are preserved during cleanup even when empty

---

## Core Principles

1. **Source-agnostic matching boundary** - Supported torrent, manual-import, and library-scan files use
   the same matcher; future adapters may join only after implementing a qualified import path
2. **Source-defined file handling** - Each source defines copy vs move and when deletion is allowed; library processing follows that policy
3. **Library owns files** - Unlinking a download source never affects library files
4. **Quality is verified, not assumed** - Every file is analyzed with FFprobe to determine true quality
5. **No auto-delete files** - Conflicts require user resolution; never overwrite or auto-delete library files
6. **Partial fulfillment is OK** - Downloading 8 of 12 album tracks is valid; remaining 4 stay "wanted"
7. **Status reflects reality** - computed from `wanted` + pending matches + linked files
8. **Use what exists now** - If a file exists NOW which is a better match, use it rather than waiting for a download

---

## Phase 1: File Matching

### Single Matching Entry Point (Mandatory)

All file matching must go through a single GraphQL mutation-driven code path:

- `matchMediaFile(input: MatchMediaFileInput!)`
- This mutation is the canonical entry point for:
  - automatic scanner matching
  - manual matching from UI
  - rematch/reverification flows
  - download/import pipeline matching

`MatchMediaFileInput` must support optional matching methods so the pipeline can evolve without forking code paths:

- `Filename` (default, always available)
- `Metadata` (ffprobe/embedded tags when available)
- `Ollama` (optional AI-assisted parsing; may be disabled at runtime)

`Ollama` is a fallback parser only, separate from Jev (Q52b), which decides among known candidates rather than parsing names. Deterministic filename/metadata matching runs first. If the best deterministic candidate is missing or below `llm.confidence_threshold`, and the caller explicitly requested the `Ollama` method, the Ollama parser may produce structured hints which are fed back into the same candidate collectors. Routine library scans default to deterministic methods only; torrent import may request Ollama as part of its fallback path. Ollama never directly chooses a target and never plans final paths. When `Ollama` is disabled or unavailable, matching must still proceed with non-AI methods in the same mutation flow.

Related pipeline mutations that must remain reusable across all library types:

- `analyzeMediaFile` for queued ffprobe analysis
- `organizeMediaFile` for final naming/folder placement

### Q1: What do we do when we find a file match for a media file that is in a download state but hasn't finished downloading yet?
**Answer:** We use what we have now. If a file exists NOW which is a better match for any potential future match (even one that may be downloading), we should use what we have. The scanner processes files that exist on disk; it doesn't wait for downloads.

### Q2: What happens when a file matches multiple items across different libraries?
**Answer:** Current matcher links one concrete target per `MediaFile` (`EpisodeId`, `MovieId`, `TrackId`, or `ChapterId`) through `matchMediaFile`. Multi-target fanout is not an active path in the current scanner service.

### Q3: What happens when a file matches an item that's already downloaded?
**Answer:** For tracks, we skip if a `media_file_id` already exists. For albums/movies/audiobooks with known targets from auto-download, we use 100% confidence and proceed even if `wanted = false`.

### Q4: How do we handle sample files (trailers, previews)?
**Answer:** If filename contains "sample", "trailer", or "preview" → mark as `Sample` type and don't process further. These are excluded from matching.

### Q5: How does the weighted fuzzy matching work?
**Answer:** Matching uses filename/metadata heuristics and fuzzy similarity scoring in the `library_scan` pipeline. Threshold behavior remains:

**Music (100 points max):**
| Field | Weight | Scoring |
|-------|--------|---------|
| Artist | 30 | 80% match = 24 points |
| Album | 25 | 90% match = 22.5 points |
| Track Title | 25 | Proportional to similarity |
| Track Number | 15 | Exact match only (0 or 15), requires disc match |
| Year | 5 | ±1 year tolerance (0.8 for ±1) |

**TV Shows (100 points max):**
| Field | Weight | Scoring |
|-------|--------|---------|
| Show Name | 30 | Proportional to similarity |
| Season | 25 | Exact match only |
| Episode | 25 | Exact match only |
| Episode Title | 15 | Bonus if matches |
| Year | 5 | ±1 year tolerance |

**Movies (100 points max):**
| Field | Weight | Scoring |
|-------|--------|---------|
| Title | 50 | Proportional to similarity |
| Year | 30 | Exact or ±1 year (0.8 for ±1) |
| Director | 20 | Bonus if matches |

**Audiobooks (100 points max):**
| Field | Weight | Scoring |
|-------|--------|---------|
| Author | 30 | Proportional to similarity |
| Book Title | 30 | Proportional to similarity |
| Chapter Title | 25 | Proportional to similarity |
| Chapter Number | 15 | Exact match only |

**Thresholds:**
- Score ≥ 70 → Auto-link (high confidence)
- Score ≥ 40 → Suggest for manual review
- Score < 40 → No match

### Q6: What happens when embedded metadata conflicts with filename parsing?
**Answer:** The code uses a 3-tier priority system:
1. **Embedded metadata first** - ID3/Vorbis tags, container metadata from FFprobe
2. **Original filename second** - If file was renamed, try stored `original_name`
3. **Current filename last** - Parse current path as fallback

If metadata exists and produces a match, filename parsing is skipped entirely. This prevents mismatches when files are renamed incorrectly.

### Q7: What do we do when the original filename differs from the current filename?
**Answer:** We try matching in order:
1. Embedded metadata (if extracted)
2. Original filename (stored in `media_files.original_name`)
3. Current filename

This 3-tier approach handles cases where files were incorrectly renamed. The `original_name` is preserved in the database and used as a fallback when current filename matching fails.

### Q8: What happens when no library matches are found?
**Answer:** Return a non-success `matchMediaFile` result with reason text. The `media_files` row remains unmatched and can be manually matched later.

### Q9: How do we determine if a file is video vs audio?
**Answer:** File extension check:
- Video extensions: `.mp4`, `.mkv`, `.avi`, `.mov`, `.wmv`, `.flv`, `.webm`, `.m4v`, `.ts`
- Audio extensions: `.mp3`, `.flac`, `.m4a`, `.aac`, `.ogg`, `.opus`, `.wav`, `.wma`

### Q9b: How do manual matches work?
**Answer:** Users can manually match files to library items via the UI. Manual matches:
1. Set `match_type = 'manual'` on the `media_files` record
2. Store `matched_by_user_id` and `match_confirmed_at` for audit
3. **CRITICAL:** Manual matches are NEVER overwritten by automatic matching or scanner verification
4. Scanner skips files with `match_type = 'manual'` during verification/correction
5. The `update_match()` function checks for manual matches and returns `false` if file is manually matched
6. To change a manual match, user must explicitly unmatch first, then rematch

---

## Phase 2: Download Processing

### Q10: When do we copy files vs move files?
**Answer:** The source defines its post-download policy. The pipeline must honor per-source rules for copy vs move and for when a source file may be deleted (for example, after seeding for N days). Within the library folder, we can still rename/organize as needed.

### Q11: What happens if the source file doesn't exist when we try to process a match?
**Answer:** Return an error "Source file does not exist". The match is marked as failed with `copy_error` set in the database.

### Q12: What happens if the destination file already exists?
**Answer:** Treat it as a conflict that requires user resolution. The pipeline never overwrites existing library files. Create an action-required notification with options to:
1. Use the best quality version (replace)
2. Keep the existing file
3. Rematch the file to a different item

### Q13: What happens if creating the destination directory fails?
**Answer:** The error propagates and the match processing fails. No partial state is created.

### Q14: How do we handle cross-filesystem copies?
**Answer:** Cross-filesystem operations must follow the source policy. If a source requests a move across filesystems, fall back to copy+delete once the source allows deletion.

### Q15: What happens when a hardlink operation fails?
**Answer:** Hardlinking is optional per source policy. If hardlinking is not supported or fails, fall back to copy.

### Q16: How do we represent torrent post-processing state?
**Answer:**
- `post_process_status = completed` if `files_failed == 0` AND `files_processed > 0`
- `post_process_status = partial` if `files_processed > 0` but some failed
- `post_process_status = unmatched` if `files_processed == 0`

---

## Phase 3: Library Scanning

### Q17: What happens if we try to scan a library that's already being scanned?
**Answer:** Return early without starting a duplicate scan. The service checks persisted `library.scanning` and an in-memory in-progress set before processing.

### Q18: What happens if the library path doesn't exist on disk?
**Answer:** Log a warning and return. No error is thrown; the scan just doesn't happen.

### Q19: How do we handle auto_add_discovered when scanning?
**Answer:** Scanner adds discovered files to `media_files`, queues analysis, then attempts `matchMediaFile`. If no local match is found, scanner can provider-enrich by searching metadata providers and creating the missing entity (currently movie/show), then linking the file.

### Q20: What happens when metadata lookup fails (TVMaze/TMDB/etc.)?
**Answer:** Provider lookup is best-effort. Failures are logged with file/library context, but scan continues and the file remains unmatched for manual/later rematch.

### Q21: What happens when a file is already in the database during a scan?
**Answer:** Check if it needs linking to an item. If already linked → skip. If not linked but matches an item → link it. This prevents duplicate processing.

### Q22: Do we trigger auto-download after scanning?
**Answer:** Not in the current scanner service. Scanning handles discovery, analysis, matching, organizing, and reconciliation.

---

## Phase 4: Quality Evaluation

### Q23: How do we determine if a file meets quality requirements?
**Answer:** Compare against library/show/movie quality settings:
- Check resolution against allowed list
- Check if HDR is required
- Check HDR type against allowed list
- Check source type against allowed list

If all pass → `optimal`. If any fail → `suboptimal` with specific reasons.

### Q24: What happens when a file is suboptimal?
**Answer:** Set `quality_status = "suboptimal"` on the media_file and derive computed status from `wanted` + linkage. Suboptimal is reflected via quality status, not a status column.

The system does NOT automatically trigger an upgrade download.

### Q25: How do we determine if a new file is an upgrade over an existing one?
**Answer:**
- If new resolution rank > existing rank → upgrade
- If same resolution but new has HDR and existing doesn't → upgrade  
- If new rank < existing rank → NOT an upgrade
- If existing quality is unknown → always consider it an upgrade

### Q26: What happens when no quality settings are configured?
**Answer:** If all restrictions are empty (`allows_any()` returns true) → everything is considered optimal.

---

## Phase 5: Organization

### Q27: When do we organize files automatically?
**Answer:** During scan, after a successful match, when `library.auto_organize = true`. `organizeMediaFile` is also available for explicit/manual invocation.

### Q28: What if a show has organize_files_override = false?
**Answer:** Per-show organize overrides are not part of the current scanner service contract. Organization is controlled at library level via `auto_organize`.

### Q29: What naming pattern do we use if none is configured?
**Answer:** Use `library.naming_pattern` first. If empty, resolve the default from `naming_patterns` for the library type. If that is unavailable, use built-in fallback patterns.

### Q30: How do we handle files that are already organized correctly?
**Answer:** If `original_path == new_path` → skip organization (no-op). Don't move file to itself.

---

## Phase 6: Torrent/Download Events

### Q31: What happens when a torrent is added?
**Answer:** 
1. Spawn async task to match files immediately
2. Run matching and processing through GraphQL mutation-driven pipeline entry points
3. Keep status derived from entity linkage and wanted flags (no dedicated status column)

### Q32: What happens when a torrent completes?
**Answer:**
1. Completion event calls `TorrentService::process_completed_torrent`
2. Source media files are created/reused with `LibraryId = null`
3. Candidate library scope is either the torrent's explicit `LibraryId` or all compatible user libraries
4. `matchMediaFile(autoMatch: false)` ranks candidates; explicit-library imports use `>= 0.70`, all-library imports use `>= 0.90` plus a `>= 0.10` margin
5. The shared organization path planner computes the target path
6. The source file is hardlinked into the library path, or copied if hardlink fails; the torrent source file is never moved/deleted
7. Only after hardlink/copy succeeds, `MediaFile` is updated with `LibraryId`, target id, `Path`, and `RelativePath`, analysis is queued, and `PendingFileMatch.copied_at` is set
8. `Torrent.post_process_status` is updated from actual processed/failed counts

### Q33: What happens if torrent completion processing fails?
**Answer:** Pipeline-level failures set `Torrent.post_process_status = failed` and keep error details. Per-file failures leave or update `PendingFileMatch` with the copy/match error and produce `partial` or `unmatched` status rather than throwing away successful imports.

### Q34: How often does the download monitor retry unmatched torrents?
**Answer:** Filters for torrents created within last 7 days that have no pending matches. This effectively limits retry window to 7 days.

---

## Phase 7: Match Verification

### Q35: When do we verify matches after download?
**Answer:** Verification should happen before final link/organize updates, using the same matching pipeline (`matchMediaFile`) with metadata-aware methods.

### Q36: What happens if verification finds a mismatch?
**Answer:**
- If better match found with score ≥ 70 → auto-correct the match
- If mismatch detected but no high-confidence alternative → flag for review (set `verification_status`)
- If file doesn't exist → skip verification

### Q36b: How does the scanner detect and fix mismatched files?
**Answer:** During library scan, for each linked file:
1. Extract embedded metadata (if not already done)
2. Compare linked item's artist/album/show with metadata
3. If artist similarity < 50% → flag as `ARTIST MISMATCH`
4. Re-run matching using `matchMediaFile`
5. If new match found with score ≥ 70 → auto-correct:
   - Update `media_files.track_id` to new track
   - Update old track's `media_file_id` to NULL and set `wanted = true`
   - Update new track's `media_file_id` to file and set `wanted = false`
6. Clear any stale bidirectional references (other tracks pointing to same file)

### Q36c: How do we ensure bidirectional link consistency?
**Answer:** After successful verification, explicitly clean up and align `media_file_id` and `wanted`:
```sql
-- Clear any OTHER tracks incorrectly pointing to this file
UPDATE tracks SET media_file_id = NULL, wanted = true
WHERE media_file_id = $file_id AND id != $correct_track_id;

-- Ensure correct track points to this file
UPDATE tracks SET media_file_id = $file_id, wanted = false
WHERE id = $correct_track_id;
```

**Note:** The user-visible `status` is computed from `wanted` + presence of a linked `media_file_id`. There is no status column.

## Phase 7b: Metadata Extraction

### Q36d: When is metadata extracted from files?
**Answer:**
1. **After download completion** - Queued as background job via `queues.rs`
2. **During library scan** - For files without `metadata_extracted_at` timestamp
3. **Manual trigger** - Via `analyzeMediaFile` GraphQL mutation

### Q36e: What metadata is extracted and stored?
**Answer:** Stored in `media_files` table:
- **Audio**: Artist, album, title, track number, disc number, year, genre
- **Video**: Show name, season, episode (from container metadata)
- **Album art**: Base64-encoded cover image with MIME type
- **Lyrics**: Read on demand from an `.lrc` sidecar, the tags (`lofty`) or LRCLIB, and cached as `Lyric` rows (Q121)
- **Timestamps**: `ffprobe_analyzed_at`, `metadata_extracted_at`, `matched_at`

### Q36f: What tools are used for metadata extraction?
**Answer:**
- **FFprobe** - Video analysis (resolution, codecs, chapters, container metadata)
- **lofty** - Lyrics tags (ID3 USLT/SYLT, Vorbis/FLAC LYRICS, MP4 ©lyr), read through the storage layer (Q121). Other audio tags still come from ffprobe
- Extraction happens in `queues.rs` via `extract_audio_metadata_with_art()`

---

## Phase 8: Edge Cases & Resolved Decisions

These scenarios have been explicitly decided and should be implemented accordingly.

### Q37: What happens when an item has multiple pending downloads?
**Answer:** Prefer the newer/better quality download. When a second download matches the same item:
1. Compare quality of both downloads (parsed from filename)
2. Keep both pending matches, but mark the higher quality as preferred in processing
3. Let both downloads complete
4. When processing, use the better quality file

### Q38: What do we do when a better quality file is found for an already-downloaded item?
**Answer:** Notify the user and let them decide. The system should:
1. Detect when a new file would be an upgrade
2. Create a user notification with options: "Upgrade" or "Keep Current"
3. Do NOT auto-replace files
4. User must explicitly approve the upgrade

### Q39: How do we handle partial album/season downloads?
**Answer:** Process only the matched files; remaining items stay "wanted". After processing a partial download:
1. Set a flag on the show/album/audiobook: `download_individual_items = true`
2. When auto-download runs next, search for individual missing episodes/tracks/chapters
3. Do NOT search for the complete album/show again (avoid re-downloading the same partial release)
4. This flag is set when: download completes AND some items matched AND some items still "wanted"

### Q40: What happens when matching fails repeatedly for the same file?
**Answer:** Retain the unmatched file but notify the user. The system should:
1. Keep unmatched `media_files` records until manually matched, re-scanned, or deleted
2. After N failed match attempts (configurable, default 3 via `match_attempts`), create a user notification
3. Notification should link to the file for manual matching
4. Do NOT auto-delete unmatched files

### Q41: What do we do with files that fail processing multiple times?
**Answer:** Alert the user after X failures. The system should:
1. Track repeated processing failures in pipeline state/telemetry
2. Retry on each download monitor run
3. After X failures (configurable, default 3), create a user notification
4. Notification should include the error message and options to retry or dismiss

### Q42: How do we handle archive extraction failures?
**Answer:** Fail durably with an explanation and leave the archive alone. The system:
1. Logs the extraction error with the archive name and the specific cause
2. Sets `Torrent.post_process_status = "failed"` and writes the cause into `post_process_error`
   (password protected / unsupported / budget exceeded / too many entries / unsafe entry /
   insufficient space / corrupt or missing volume) — see the Archive Extraction section
3. Records a `PendingFileMatch` for the archive carrying the same message
4. Deletes the partial staging output but **never** the archive itself (seeding continues)
5. Does not retry automatically. `processSource`/`rematchSource` re-runs it on demand, and a
   successful extraction is idempotent, so retrying is always safe
6. No external tool install is ever required: ZIP/RAR/7z are handled in-process

### Q43: What happens when library storage is full?
**Answer:** Track free space and alert proactively. The system should:
1. Track disk free space in server status (exposed via GraphQL for frontend)
2. Create user notification when space drops below threshold (configurable, default 10GB)
3. If copy fails due to ENOSPC, create urgent notification (implemented on copy failure)
4. Clean up any partial files on copy failure

**TODO:** Implement free space tracking in server status. Already exposed via GraphQL for TUI backend, extend for frontend use.

### Q44: How do we handle duplicate torrents for the same item?
**Answer:** Let both complete and pick best quality. The system should:
1. Allow multiple torrents to match the same item
2. When first torrent completes, process normally
3. When second torrent completes, compare quality to existing file
4. If second is better quality → process and replace (with user's upgrade approval per Q38)
5. If second is same/worse quality → skip processing, mark as duplicate

### Q45: What do we do when a file's metadata says one thing but filename says another?
**Answer:** Use 3-tier priority system:
1. **Trust metadata first** - If embedded tags produce a match with score ≥ 70, use it
2. **Try original filename** - If metadata fails, try stored `original_name` if different from current
3. **Fall back to current filename** - Last resort parsing
4. If best score is 40-70 → suggest for manual review
5. If best score < 40 → leave unmatched for manual intervention
6. Do NOT notify unless file remains unmatched after all tiers

### Q46: How do we handle shows/movies that get renamed/merged in metadata providers?
**Answer:** Periodically re-sync metadata and detect changes during library scan. The system should:
1. During library scan, re-fetch metadata for existing items
2. Compare external IDs (tvmaze_id, tmdb_id, etc.)
3. If external ID changes or item is marked "merged/redirected" by provider → flag item
4. Create notification for user to review affected items
5. Provide UI to merge/migrate affected items

### Q47: What happens when a library is deleted while files are downloading?
**Answer:** Remove all links that torrents/files have to that library. On library delete:
1. CASCADE delete all library items (shows, movies, albums, etc.)
2. Related `media_files`/entity links are removed by schema constraints and scanner reconciliation logic
3. Torrents remain but become "unlinked" (no library association)
4. Downloaded files in the library folder are NOT deleted (library owns files, user must delete manually)

### Q48: How do we handle files with non-ASCII characters in names?
**Answer:** During organization, filesystem-invalid characters are sanitized in generated names. If organization is disabled (`auto_organize=false`), files are not renamed by scanner.

### Q49: What's the behavior when FFprobe analysis fails?
**Answer:** Persist a bounded actionable scan issue, log as warning, and continue. The system should:
1. Classify missing executable, timeout, malformed output, oversized output, permission error, and
   unsupported/corrupt media with stable issue codes
2. Persist bounded diagnostic context and attempt count on the scan issue
2. Mark file as having unknown quality (`quality_status = null`)
3. Continue with filename-parsed quality info for matching decisions
4. Matching is independent of FFprobe; FFprobe just provides quality verification hints
5. Aggregate failure counts on the durable scan run and expose a per-file Retry analysis action

### Q50: How do we handle season packs vs individual episodes?
**Answer:** Prefer a pack when most of a season is missing, otherwise search per episode. (This supersedes the earlier "never download packs" rule.) The system:
1. Hunts a season as a pack when the profile's `allowSeasonPacks` is true **and** the season has at least 3 missing episodes or at least 50% of its aired episodes missing.
2. Queries both title styles for the pack ("Show S02" and "Show Season 2") and merges the results before ranking.
3. Stamps `showId` + `season` on the grabbed `Torrent`, and skips per-episode grabs for that season while that torrent is unresolved.
4. Falls back to per-episode searches for that season only when no acceptable pack was grabbed.
5. Still matches a pack's contents file-by-file on import — a pack download is not itself "the match". Partial packs work fine (matched episodes get processed, unmatched stay as files).
6. A profile with `allowSeasonPacks: false` neither hunts nor accepts pack releases.

### Q52a: Where does audiobook metadata come from?
**Answer:** Audible's catalogue for search and details, and Audnexus for chapters, because Open Library lists every printed book and has no chapters. Neither API is officially public, so results are cached and Open Library remains the fallback when Audible fails or finds nothing. A book's provider id is stored in `audibleId` whichever provider it came from, and its form (a ten-character ASIN or an `OL…` id) says which provider it belongs to.

### Q52: How does auto-download handle audiobooks, where chapters aren't individually released?
**Answer:** Search and grab at the whole-audiobook level, not per-chapter. Unlike TV episodes and music tracks, audiobook releases are essentially never published as one-file-per-chapter torrents — a real indexer release is "the whole book" (often as a single file or a small number of parts). So:
1. Auto-download considers an audiobook a candidate when `Audiobook.auto_download = true` and at least one of its chapters is `wanted = true` with no `media_file_id`.
2. The `SourceQuery` is built from the audiobook's title/author (a `BookSearch`), never a per-chapter query.
3. The grabbed `Torrent` row gets `audiobookId` set (from the new wanted-linkage columns — see the Auto-Download Loop section of `docs/tier1-features-plan.md` §1); `chapterId` is left null.
4. Chapter-level linkage happens during import matching, the same way a manually-downloaded TV season pack gets matched file-by-file to episodes (Q50 point 2) — the whole-book download is not itself "the match," its extracted files are.

### Q51: What happens when a user manually changes a file on disk?
**Answer:** Require manual re-scan to detect changes. The system should:
1. Scanner is the source of truth for detecting out-of-band file changes
2. User triggers library scan to detect changes
3. Scanner compares discovered file paths to database records
4. Missing files → unlink from items, set `wanted = true`
5. New files → queue analyze, then match and optionally organize
6. Existing files can be re-analyzed through `analyzeMediaFile`

### Q52b: How is a release matched to a book when its files don't look like the book's chapters?
**Answer:** A provider's chapter list describes the provider's edition, so it is a reference, never slots to fill by position. `services/release_match.rs` holds the rules as pure code, and a preview and an import share one answer.

1. **Which files are the book.** Files are grouped by album tag, then folder, then the pattern of their names. A group is the book if an ASIN tag matches, or its name or tags match the title. A lone group whose name doesn't match needs a length within 3%, and tags naming another book always stop it. ffprobe reads ID3, MP4 atoms, APE and Vorbis comments, and `tags_from_pairs` reads them all the same way.
2. **Where chapters come from**, first match wins:
   1. Chapter markers inside the file. If their names are only numbers and the counts agree, the provider's names are kept with the file's times.
   2. File names that match the provider's chapter titles, even if some chapters are missing (partial fulfilment, Q39).
   3. The same recording, within 3% of the book's length: the provider's chapters stay, placed on the files by time. Each chapter gets `mediaFileId` and `fileOffsetSecs`. This covers one whole file, one file per chapter, and parts that ignore chapter breaks.
   4. Files with real names of their own: each file becomes a chapter.
   5. No chapter information, within 15% of the length: the files in order, as numbered parts.
3. **Anything else needs a person.** The torrent gets `postProcessStatus = needs_review`, nothing moves, and it isn't retried. They can accept the fallback from the torrent's Identified tab (`importIdentifiedItem`).
4. **Replacing chapters is reversible.** The provider's list is kept in `Audiobook.providerChapters`, rows are reused so references survive, and `restoreAudiobookChapters` puts it back.
5. **Packs.** Other books in a release are never imported into this one. Their files are reported as belonging to another book, the torrent ends `partial`, and `importIdentifiedItem` imports them as a library book or adds them from the provider.
6. **A release with no audio** is refused while still paused when the file list is known, before anything downloads. If that is missed, the import fails with `failed`, which blocklists it so a hunt can try another release. Hunts never grab ebooks or multi-book packs.
7. **Previews.** `torrentMatchPreview` runs the same plan from names and sizes while downloading, and from tags and lengths once files finish. Nothing moves on an estimate.
8. **Jev** (`services/jev.rs`) is optional and off by default, enabled in Settings → Metadata. Its key lives in `MatchingAssistant.api_key`, encrypted by the write transform like `Source.credentials`, writable but never readable through GraphQL. It is asked only what rules couldn't settle, which group is this book and which track is this file, and only among candidates the rules found. A book answer needs at least 85% and a 10-point lead over the next group; a track answer needs 80%. Lengths still have to agree. File and title names are sent to TypeSafe when it is on.

Files in parts are named `<book> - Part NN` and keep that name through organizing, because a chapter's name would be wrong on them. Music uses tags, names and lengths together through `match_tracks`.

### Q52c: How does a download get identified?
**Answer:** Every torrent goes through one recorded pipeline, `services/library_scan/identify.rs`, with its rules as pure code in `services/identify_rules.rs`. It runs when a torrent's files are first seen, 30 seconds after more files finish, when the torrent finishes, and on request (`identifyTorrent`). Runs are repeatable: results are kept by what they describe, a step that fails is recorded and passed over, and a torrent with no finished run is picked up after a restart.

1. **Request.** The Torrent row's link, or its library's type, is a hint: what it was grabbed for is tried first. A request can't make files into something they can't be.
2. **Reading.** Tags, lengths and chapter markers from finished files, and `.txt`, `.nfo`, `.cue` and `.md` files up to 256 KB. Text files are evidence from a stranger, compared with other sources and never acted on directly.
3. **Kind.** Audiobook, music, TV, movie or ebook, from the request, formats, genre tags, names, text files and track lengths. Long albums are books only if their tracks are too, or if they run past seven hours.
4. **Components.** Books and albums by `group_files`, TV by show and season, movies by file.
5. **Which thing.** The library first, then providers. For books the title alone is never enough: a listing must have the right author and a matching length, which is how another author's book of the same name is kept out. An Audible id in a name settles it if the length agrees. Otherwise, in order: searches with each author and then the plain title, then what's heard in the recording's opening, then Jev among the listings that remain. Two readings of the same length are left to a person. Music compares track lengths with MusicBrainz, and movies compare runtime with TMDB.
6. **Results.** `IdentifiedItem` holds each thing found, with its candidates and its evidence, each piece recording its source (`files`, `heard`, `listing`, `library`, `jev` or `other`), whether it is a warning, and a short label for warnings shown beside the title. Statuses are `imported`, `wanted`, `in_library`, `identified`, `needs_choice` and `unknown`. `IdentificationEvent` holds each step with its timing, for the last three runs. Both are shown on the torrent, and both go when it's removed.
7. **Imports.** A complete item matching something wanted imports at once, through the same paths as a linked download. A run from merely seeing a torrent, such as after a restart, never imports. A restored torrent's files seem to finish while the client checks them; that never imports either, and a first look replaced that way while queued still runs as a first look. A pending "file finished" run stops waiting as soon as the torrent finishes. Everything else waits for `importIdentifiedItem`. Imported stays imported across runs.
8. **Files that can't be read** don't replace results. A run that can't read a file retries once, and if there are results already, it keeps them and says so. Mid-verification a restored torrent reads as empty, so `completedAt` decides whether its files are there.
9. **Imports of one torrent run one at a time**, because finishing, identification and a person can each start one.
10. **Unlinked audiobooks** skip the generic per-file matcher, whose view of one file at a time can place a pack's parts on the wrong book.

**Listening** (`services/whisper.rs`, `library_scan/listen.rs`) is one more source of evidence, on by default where installed, and it runs on the server only. It uses `whisper-cli` from whisper.cpp as a separate program, found by `LIBRARIAN_WHISPER_CLI` and `LIBRARIAN_WHISPER_MODEL` or in `data/models`. After a book imports it listens at each listed chapter's start for that chapter being announced, such as "Chapter Ten" or "Prologue". The listing keeps its names and order, and only times move. A heard chapter starts at the end of the last real pause before its announcement, since word timings are out by a second or so. One not heard falls between the heard chapters around it, then on the longest pause nearby. Credits and titles of a single ordinary word aren't listened for. Unless at least half of the chapters that can be heard are, nothing changes and the book says so. `restoreAudiobookChapters` undoes it. `changeAudiobookEdition` makes a book a different listing and keeps its files.

### Q53: Which `QualityProfile` applies to a given media file/wanted item?
**Answer:** Precedence, resolved by `services::quality::profile::resolve_profile_for_media_file`/`resolve_profile_for_library`:
1. A per-entity override (`Show`/`Movie`/`Album`/`Audiobook`.`qualityProfileId`), if set.
2. Else the owning `Library.qualityProfileId`, if set.
3. Else the seeded default profile (`QualityProfile.isDefault = true`, seeded as "Any Quality" by `bootstrap_defaults.rs`).
4. If even the default lookup fails (should not happen in practice), an in-memory `builtin_any_profile()` — a profile with every allow-list empty — is used so the pipeline never hard-fails over a missing settings row.

`jobs::auto_download` resolves a candidate's profile the same way, but without a `MediaFile` (nothing downloaded yet): each `WantedCandidate` carries the parent entity's override id directly, and `profile::resolve_profile(db, override_id, library_id)` applies steps 2-4 above.

### Q54: When is `MediaFile.quality_status` computed, and from what?
**Answer:** Computed by `services::quality::profile::recompute_and_persist`, called from the `evaluateMediaFileQuality` mutation and from `AppMutationHook` (`services::quality::recompute`), which keeps it current after commit: when a file is analysed or its link/properties change (`requalify_media_file`, inline, so an analysis job ends with the status set), when a `QualityProfile` is edited in a way that changes evaluation or deleted, and when a library or title is assigned a different `qualityProfileId` (these run as spawned background passes, since `commit_and_emit` awaits deferred work inline). Only analysed files are evaluated, an unchanged status is not rewritten, and the status write itself never triggers another pass. There is no `recomputeLibraryQualityStatus` mutation. Input is `profile::parsed_release_for_media_file`: ffprobe-verified columns (resolution/codec/HDR/audio) take priority per "quality is verified, not assumed"; `source_type` and `release_group` (which ffprobe cannot determine) are supplemented by parsing `MediaFile.originalName` (falling back to `path`) through the same `scoring::parse_release` used for search-result titles. A failure here (e.g. transient DB error) is logged as a warning and does not fail the analysis job (Q49).

### Q55: How does a Q44 duplicate-quality upgrade actually get surfaced and approved?
**Answer:** An existing-target conflict may produce an `ACTION_REQUIRED`/`QUALITY`
notification when scoring identifies a better candidate. The conflict remains
skipped until explicit approval. Only trusted workflows write the command
envelope. Approval requires a current administrator and rederives the target from
the owned Library/MediaFile and the candidate from the qualified pending match
and persisted complete torrent file or extraction proof. Captured filesystem
identity is checked again before copying and replacement; arbitrary JSON paths
are not execution authority. The candidate is copied into a new inode in the
target directory and renamed over the verified target, preserving the seeding
source. Generated entity writes record the result and analysis refreshes the
media facts. Old path-only or stale notifications fail closed; retrying import
can create a fresh verified request. Owner acknowledgement, dismissal and deletion
remain ordinary generated notification mutations.

### Q56: How does the generic Torznab/Newznab client source (`sources/definitions/torznab.rs`) resolve categories and capabilities before it has ever queried the indexer?
**Answer:** `TorznabSource` is constructed with a conservative, generic `SourceCapabilities` (search + TV/movie/music/book search with common ID params; categories = the six round-thousands Torznab buckets, or the configured `DefaultCategories` setting if present) computed synchronously at construction time, since the `Source::capabilities()` trait method is synchronous (`&self -> &SourceCapabilities`) and can't await a network call. `test_connection()` calls `t=caps`, parses `<categories>`/`<searching>` into a real `SourceCapabilities` (an *identity* `CategoryMapping` table: `tracker_id == torznab_cat` as strings, since a genuine Torznab endpoint already speaks the standard numbering), and caches it once via `OnceLock` — it is never re-fetched for the lifetime of that source instance (a fresh instance is created on every `SourcesService` reload, so this isn't a "permanently stale" problem). If `t=caps` errors, returns a Newznab `<error>` other than an auth failure (code 100/101), or fails to parse, the source fails **soft**: it keeps the constructor-time fallback capabilities and `test_connection()` still returns `Ok(true)` (only an actual HTTP failure or an auth-error code returns `Ok(false)`). One practical consequence: category requests made *before* a successful `t=caps` call get mapped to their parent round-thousands bucket (e.g. requesting Torznab cat `5030` maps to `"5000"` in the outbound `cat=` param) rather than the indexer's real subcat id, via the existing `SourceCapabilities::map_torznab_to_tracker` parent-bucket fallback (reused, not reimplemented) — this is intentional and covered by `torznab::tests::build_query_params_includes_core_fields`.

`TorznabSource::tracker_type()` always returns `TrackerType::Private` (there's no protocol-level signal to distinguish public/private Torznab-proxied indexers, and requiring an `ApiKey` credential is the closer semantic fit of the three variants). Scope is intentionally `SourceType::TorrentIndexer` only — real Newznab/usenet (NZB download + NNTP posting) is an explicit Tier-2 follow-on per `docs/tier1-features-plan.md` §3, since no NNTP client exists anywhere in this codebase.

### Q57: How does the backend decide whether a media file needs HLS remuxing or transcoding for browser playback?
**Answer:** `api/media.rs`'s `needs_transcode(container, video_codec, audio_codec) -> TranscodeDecision` (`None`/`Remux`/`Transcode`), computed from three independent predicates:

| Container OK? | Video codec OK? | Audio codec OK? | Decision |
|---|---|---|---|
| yes | yes | yes | `None` — direct play via `/stream` |
| no | yes | yes | `Remux` — stream-copy repackage into HLS (`-c copy`), no re-encode |
| — | no | — | `Transcode` — full re-encode (`-c:v libx264 -c:a aac`) |
| — | — | no | `Transcode` |

"Container OK" means ffprobe's `format_name` contains `mp4`/`webm`/`mp3` **and does not contain `matroska`** — ffprobe reports the same `format_name` (`"matroska,webm"`) for both genuine WebM and Matroska/MKV files (WebM is a constrained Matroska profile using the same demuxer), so a real MKV file cannot be distinguished from a real WebM file by container name alone; treating any `matroska`-containing name as incompatible correctly catches the common MKV-rip case at the cost of conservatively remuxing rare standalone `.webm` files too (still fully correct, just not the fastest path). Unknown video/audio codec (`None`, e.g. an audio-only or video-only stream) defaults to "assume compatible"; unknown container defaults to "assume incompatible" — so a file with no probed data at all resolves to `Remux`, not `Transcode`.

`GET /api/media/{id}/info` exposes this as `needs_hls: bool`, `transcode_decision: "none"|"remux"|"transcode"`, and `playback_url` (either `/stream` or `/hls/playlist.m3u8`), so the frontend never re-implements the codec logic.

### Q58: Does casting use the same transcode decision as browser playback?
**Answer:** No — deliberately separate. `CastService::decide_playback` produces `DIRECT`, `REMUX`,
`TRANSCODE`, or `UNSUPPORTED` from the receiver type/model, analyzed container/codecs/HDR, preferred
quality, and the Cast transcode setting. Direct decisions use the scoped `/stream` URL; remux/transcode
decisions use a Cast-owned HLS lifecycle and carry the session/media-bound Cast grant onto playlist
segments. Browser playback continues to use `api/media.rs::needs_transcode`. The selected Cast decision
and reason are persisted as diagnostics, while the transient grant is never persisted or returned.

### Q59: What should a movie-library scan do when TMDB is not configured?
**Answer:** Keep every discovered file as an unmatched `MediaFile`, skip the redundant per-file TMDB requests, and complete the scan with an explicit warning summary rather than a clean-success message. Create one unresolved, library-scoped `CONFIGURATION` notification that links the user to **Settings → Metadata**; do not create another notification on every scan while that one remains unresolved. Once a TMDB key is configured, the next scan retries provider matching normally. This is a configuration block, not file loss: the files remain visible in the library's unmatched-files view.

### Q60: Which database schema changes may run during ordinary application startup?
**Answer:** Only fully additive plans. Startup must validate and plan before applying anything. New tables, nullable/defaulted columns, and new indexes may be applied automatically; data-transforming or destructive steps are blocked. All SQL type conversions are treated as destructive until a migration-specific transform is implemented, and an unannotated field rename is treated as add-plus-drop. A non-additive plan requires `librarian schema apply` with the exact reviewed plan hash and a checksum-verified full backup whose source schema hash matches the live database. There is no environment-variable bypass. The operator workflow is documented in `docs/schema-migrations.md`.

### Q61: What happens when automatic organization cannot prove a move is safe or cannot complete the move?
**Answer:** Fail closed and leave the file associated with its current database path. A safety-guard error is never permission to organize. A returned unsuccessful outcome or operation error is not discarded or reported as success. Log the full internal error with library/media/path context and create one unresolved, scoped `ORGANIZATION` notification per failure kind containing the library ID, media file ID, current path, attempted operation, and a sanitized reason. Preserve the no-clobber rule; do not automatically retry a filesystem move whose outcome may be ambiguous.

### Q62: How are library-scan outcomes retained and remediated?
**Answer:** A scan is a durable `LibraryScanRun` created at queue time. Discovery, matching, provider
fallback, organization, analysis settlement, reconciliation, and completion update the same run and
persist bounded `LibraryScanIssue` rows with stable codes. A provider/configuration block, analysis
failure, organization failure, or verified duplicate produces `COMPLETED_WITH_ISSUES`, never a clean
success. The UI shows the latest run and unresolved issues, can retry an analysis issue, and can mark a
reviewed issue resolved. The service retains the latest 50 runs per library and removes resolved issues
after 30 days.

### Q63: How are byte-identical files handled?
**Answer:** Equal size is only a candidate signal. The scanner streams a full SHA-256 for each same-size
candidate and creates a review issue only when hashes match. Scans never delete duplicates. The issue
stores bounded machine-readable path, size, keeper-path, and digest context. An administrator may
explicitly choose “Move to trash”; immediately before acting, Librarian verifies both paths are still
inside the library root, both still hash to the recorded digest, moves only the selected duplicate into
the library's `.librarian-trash` directory, and then removes its entity row. If the entity update fails,
the filesystem move is compensated. Permanent deletion is outside this workflow.

### Q64: What is the authoritative content-status precedence?
**Answer:** Status is a pure, non-persisted reduction of entity/runtime facts, in this order:
`PLAYING`, `PAUSED`, `DOWNLOADING`, `FAILED`, `PROCESSING`, `UPGRADABLE`, `AVAILABLE`, `IGNORED`,
`UPCOMING`, `WANTED`, `MISSING`. `ignored` is an explicit nullable entity flag whose legacy/null value
means false. Status reads batch media-analysis, active-playback, pending-download, and unresolved
analysis-issue facts and the frontend consumes the generated `ContentStatus` contract rather than
reimplementing the reducer.

### Q65: Which unfinished platform/source capabilities may the product advertise?
**Answer:** Only qualified capabilities. Usenet/NNTP acquisition, automated archive extraction,
first-party AirPlay control, DLNA AVTransport playback, and Windows service/tray lifecycle are not
currently supported and must be labelled unavailable or omitted from product promises. DLNA devices may
remain visible as `discoveryOnly`; Safari may expose its own media-element AirPlay control, but this is
not a Librarian protocol adapter. These capabilities become supported only after their automated
fixtures plus the provider/device/VM qualification gates in the remediation plan pass.

### Q66: How are automatically discovered Cast devices retained and removed?
**Answer:** Manual and favorite devices are never removed by discovery cleanup. Network-discovered,
non-favorite, non-manual records are marked disconnected when their last observation exceeds the
connectivity window and are removed only after `CAST_DISCOVERY_RETENTION_DAYS` (30 days by default,
validated to 1–3650 days). Discovery upserts refresh `lastSeenAt`, `lastProbeAt`, and probe status through
the entity layer before cleanup runs. Device capabilities are recomputed from the current persisted
device metadata for each playback decision rather than cached, so a model/type update takes effect
without cache invalidation or stale capability state.

### Q67: When may a rescan reuse prior matching and media analysis?
**Answer:** Each discovered `MediaFile` stores the filesystem modification timestamp observed alongside
its byte size. An existing, already-linked file with the same size and modification timestamp reuses its
match and successful analysis; no provider or matching work is repeated. An unmatched file is always
eligible for matching again because provider configuration or catalog state may have changed. If size or
modification time changes, Librarian clears stale ffprobe/quality fields, records the new version marker,
and queues fresh analysis before continuing matching. Legacy rows without a marker are invalidated once
when the filesystem can provide one. If a filesystem cannot provide modification time, size is the
fallback signal.

### Q68: What does `autoDownloadMode` mean for a Show/Album/Audiobook, and how does it relate to `autoDownload`?
**Answer:** `autoDownloadMode` is authoritative and `autoDownload` is the normalised master switch. `NONE` = never hunt (implies `autoDownload = false`); `WANTED` = hunt only children already flagged `wanted`; `ALL` = every child that is missing (no `mediaFileId`), not `ignored`, and already aired/released is automatically treated as wanted — `jobs::auto_download` writes `wanted = true` on those rows through the entity layer so the stored status matches what the hunt is doing, which also means a future episode becomes wanted by itself once it has aired and its grace period (Q81) has passed. Items with no air/release date at all are treated as released (back-catalogue entries frequently have none). Movies are unchanged: `monitored` + `wanted`. The job normalises the pair on read (`normalize_auto_download`); the frontend sets both fields together.

### Q69: How often may the same wanted item be searched for?
**Answer:** At most once every `auto_download.search_backoff_minutes` (default 360, i.e. 6 hours) per target, or every `auto_download.air_retry_minutes` (default 60) while an episode is inside its post-air window (Q81). `jobs::auto_download` keeps a `target key -> last searched at` map in the `auto_download.search_state` app setting (written through the `AppSetting` entity, so it survives restarts and is visible/clearable in settings). Keys are `episode:<id>`, `movie:<id>`, `track:<id>`, `audiobook:<id>` and `show:<id>:s<season>` for season packs; entries older than 30 days are pruned on write. The `searchMissing` mutation and the retry sweep's re-hunt ignore the backoff, so a user pressing "search" always gets a real search.

### Q70: What happens to a release that fails to import or never downloads?
**Answer:** It is blocklisted and the target is re-hunted. The 15-minute retry sweep classifies every grabbed torrent inside the retry window: `postProcessStatus = "failed"` (reason `import_failed`) or zero bytes downloaded for more than 24 hours (reason `stalled`) creates a `ReleaseBlocklist` row carrying the info hash, guid, title, source and wanted-target ids, then immediately runs a scoped hunt for that target with the backoff disabled. `jobs::auto_download` drops any candidate whose info hash or guid is blocklisted, and treats a blocklisted in-flight torrent as resolved so the re-hunt is not blocked by the grab it just blocked. Entries block forever unless `expiresAt` is set. Everything else that merely never resolved is re-processed as before (Q34).

### Q71: How does import use the linkage recorded on the Torrent row?
**Answer:** Recorded linkage wins over re-deriving the target from the filename. `jobs::auto_download`
(and manual `addTorrent`) stamp `movieId`/`episodeId`/`trackId`/`chapterId`/`showId`/`albumId`/
`audiobookId`/`season` on the `Torrent` row at grab time. `process_torrent_source_files` reads those
before any generic matching and builds a per-file plan:

1. `movieId` → the largest video file in the release is that movie. Other video files are reported as
   "not the main feature file".
2. `episodeId` with exactly one video file → that file is that episode, whatever its name claims.
3. `showId` (season pack / multi-episode release, with or without `episodeId`) → each video file's
   season/episode is parsed from its filename; a missing season falls back to `Torrent.season`, then
   to the linked episode's season, then to a season parsed from the release name. Files are matched
   against the show's own episode rows; a file that parses to an episode the show does not have, or
   that another file already claimed, is reported individually.
4. `albumId` → audio files are matched to that album's tracks by exact unique track number first,
   then by title similarity (≥ 0.90), then — only when the file count equals the track count — by
   release order. Audio fingerprinting would be the ideal final tiebreaker, but `services/fingerprint.rs`
   needs the Chromaprint `fpcalc` binary plus an AcoustID key and is not wired into the service graph,
   so it is deliberately not used here.
5. `audiobookId` → audio files are sorted in natural filename order and mapped onto the audiobook's
   chapters in chapter-number order (Q52: releases are whole-book, not per-chapter).
6. Bare `trackId`/`chapterId` with no parent → only used when the release has exactly one audio file.

Only when no linkage is present (or it cannot be resolved) does the generic filename/metadata/Ollama
matcher run with its 0.70/0.90 confidence floors. Linkage-based matches are **not** privileged in any
other respect: manual matches are still protected (Q9b) and an existing library file is still never
overwritten — a better file raises a quality-upgrade notification instead (Q38/Q44). The decision is
auditable through `MediaFile.match_type` / `PendingFileMatch.match_type`: `torrent_link` when the
torrent named the exact target, `torrent_link_parsed` when it named the parent (show/album/audiobook)
and the individual file was resolved by parsing or ordering.

### Q72: Which files does import skip before matching?
**Answer:** Samples, proofs, extras and undersized video, decided from the release's own contents:
1. Any file whose *path* contains a `sample`/`samples`/`proof`/`extra(s)`/`featurette(s)`/`bonus`/
   `screens`/`screenshot(s)`/`trailer(s)` directory is skipped.
2. Any file whose name contains `sample`, `proof` or `trailer` as a standalone token is skipped when a
   larger file of the same kind (video/audio) exists in the release.
3. Video files below 50 MB are skipped when the release also contains a video file at or above 50 MB.
   If *every* video in the release is small the release is a legitimate small-format one and nothing
   is dropped, so webisodes and short-form content still import.
4. There is no size floor for audio: a 3 MB track is a perfectly normal track.

Skipped files are logged individually and, when nothing at all was imported, listed in
`Torrent.post_process_error` so the UI can explain the empty result. This complements Q4, which
governs the scanner's `Sample` classification.

### Q73: What does an "unmatched" torrent tell the user?
**Answer:** Always a reason, never a bare status. `Torrent.post_process_error` is composed from the
import pass and can combine several segments:
- `archive extraction failed: <exact cause per archive>` (Q42)
- `N file(s) did not match <target>: <names> (<distinct reasons>)` — e.g. `3 file(s) did not match any
  item of 'The Expanse': a.mkv, b.mkv, c.mkv ('The Expanse' has no episode S02E11)`
- `N file(s) skipped as sample/extras/undersized: <names>` (only when nothing was imported)
- `no supported media files: this torrent only contains .nfo, .srt`
- `N file(s) failed during torrent post-processing`

A fully successful import writes no error at all. Per-file detail also stays on the `PendingFileMatch`
row for that file (`unmatchedReason`, `copyError`, `matchAttempts`), which is what the UI drills into.

### Q74: Can naming customization or a library edit choose arbitrary server paths?
**Answer:** An administrator approves the library root. Owners may customize
relative naming within that root, but absolute, drive/UNC and parent-traversal
patterns are rejected. The expanded destination and existing parent components
must remain inside the canonical approved root, including through symlinks,
before directory creation, copy or move. An invalid destination leaves the source
and existing library file intact and records an actionable failure. Naming
customization does not grant access to additional filesystem roots.

### Q75: What happens when a source-processing request is not authorized?
**Answer:** It fails before processing or status writes. A torrent import binds
its persisted hash, owner and optional destination library before entering the
processing/error boundary; later status updates recheck that identity. A rejected
request cannot mark another owner's acquisition failed or initiate its retry
policy. Trusted jobs derive ownership from the persisted target and never choose
the first available user.

### Q76: Which facts drive stored media summaries and parent updates?
**Answer:** Linked canonical files determine Album/Audiobook availability and
deduplicated byte totals. Explicit same-library Album/Track artist links determine
Artist counts and known track duration; a missing track artist is not inferred
from its album. Membership/duration changes repair summaries in the same generated
transaction. Metadata-only changes still notify the surviving Artist without a
full recount. Old and new parents are invalidated on reparenting and deletion.
Personal playback checkpoints retain their own user-scoped events. Existing
summary drift is repaired by the explicit administrator reconciliation action,
which defaults to dry-run; it never runs implicitly at startup.

### Q78: Does a scan organize a file it skips as unchanged?
**Answer:** Yes. A file that is linked to a library item and has the size and modification time recorded
for it is not re-matched, re-probed or re-hashed, but when `auto_organize=true` the scan still checks
where it is. Where each item's file belongs is worked out for the whole library from a few paged reads of
its movies/episodes/tracks/chapters and the library's naming pattern, with no per-file query. Only a file
that is somewhere else goes through `organize_media_file`, which re-plans it, never overwrites an existing
file, and records a failure as an `ORGANIZATION` notification. So a file matched while organizing was off,
or left behind by a changed pattern or a renamed title, is moved by the next scan, and a library whose files
are all in place pays for the reads only. A movie whose name and matched title disagree on the year keeps
the same guard as a new match (Q61) but gives no second notification. Audiobook chapters are compared by
folder only, because a book in parts or in one file is named after its parts.

### Q79: What names a music file's artist folder?
**Answer:** The Artist record of the track's album, and nothing a track says about itself: a per-track
credit (which differs by release and may be empty) never picks a folder, and the library's Artist rows are
the only source. Every folder or file name goes through one sanitiser, which also turns typographic
apostrophes (`’`, `‘`, `ʼ`) into `'`, so `Guns N’ Roses` and `Guns N' Roses` are one folder.
An album whose artist has no real name (`Unknown Artist`, empty) has no artist folder: its files are left
where they are, never filed under a placeholder, and a provider album that credits no artist is not added
under a made-up one (its files' own tags name the album's artist instead). Files an earlier version put in
`Unknown Artist/` or in a second spelling's folder are moved to the right one by the next scan (Q78).

### Q80: Which folders does organizing remove?
**Answer:** Folders it leaves empty, a whole empty tree at a time, up to but never including the library
root. After `organizeMediaFile` moves a file, its old folder and each parent that is empty as a result are
removed; a scan with `auto_organize=true` then removes every remaining empty folder no library item expects
(TV show/season folders and the planned folders of known items are kept). A folder with anything in it stays,
hidden files such as `.DS_Store` included, and so does a hidden folder (names starting with `.`): the
library's own `.librarian-trash` and a sync tool's markers are not Librarian's to judge. Scans do not walk
into the trash, so what is in it is never recorded as library files or organized back out. Removal is a
plain `rmdir`, which cannot remove a folder a file has just been placed in.

---

## Key Files Reference

| Component | File |
|-----------|------|
| Library Scan Service | `backend/src/services/library_scan.rs` |
| Archive Extraction | `backend/src/services/extract.rs` |
| Scan/Match/Organize GraphQL Mutations | `backend/src/services/graphql/mutations/library_scan.rs` |
| Library Scan Schema Wiring | `backend/src/services/graphql/schema.rs` |
| Service Registration | `backend/src/services/manager.rs` |
| Download Monitor | `backend/src/jobs/download_monitor.rs` |
| Source Services | `backend/src/services/sources/` |
| DB Schema Planning and Application | `backend/src/services/database.rs` |
| Quality Profiles (parsing, evaluation, resolution) | `backend/src/services/quality/` |
| Quality Profile Entity | `backend/src/services/graphql/entities/quality_profile.rs` |
| Quality Custom GraphQL Ops | `backend/src/services/graphql/mutations/quality.rs`, `backend/src/services/graphql/queries/quality.rs` |

---

## Computed Status Reference

The status shown in the UI is computed from `wanted`, pending matches, linked files, and quality status. There is no status column.

| Computed Status | Meaning | Derived From |
|-----------------|---------|--------------|
| `missing` | No file exists, hasn't aired yet (for episodes) | `wanted = false`, no file, future air date |
| `wanted` | Aired/released, no file, actively looking | `wanted = true`, no file |
| `downloading` | Matched to a pending download | pending match exists |
| `downloaded` | File exists in library folder | linked `media_file_id` |
| `suboptimal` | Has file but below quality target | linked file + `quality_status = suboptimal` |
| `ignored` | User explicitly skipped | explicit user flag |

---

## UI Progress Display

### Library Item Progress

Instead of showing status chips ("Downloaded", "Wanted"), the UI displays progress fractions:

| Media Type | Display | Example | Color |
|------------|---------|---------|-------|
| Album | downloaded/total tracks | `9/15` | Green if complete, Yellow otherwise |
| TV Show | downloaded/total episodes | `25/30` | Green if complete, Yellow otherwise |
| Audiobook | downloaded/total chapters | `8/12` | Green if complete, Yellow otherwise |

This applies to:
- Library card views (top-left badge)
- Library table views (PROGRESS column)

### Computed Fields

Progress counts come from authorized generated child connections or their typed
entity-layer count APIs: Album tracks, Show episodes and Audiobook chapters.
Filter `mediaFileId` to non-null for the linked count, and use the unfiltered
connection's `pageInfo.totalCount` for the catalogue total. A loaded page length
is not a total. Service implementations must use the generated/typed query
capability, preserving ownership before counts and pagination; do not introduce
application SQL subqueries. Stored availability and byte summaries follow Q76.

### Notification acknowledgement and scan history

“Mark all read” acknowledges the authenticated user's entire feed with one
`markAllNotificationsRead` mutation, independent of pagination. Both notifications
and scan issues persist `readAt`; acknowledging an issue never sets `resolvedAt`
or changes media files. Members can acknowledge their own scan issues through
this narrow operation, while generic scan issue writes remain admin-only.
The operation uses generated entity bulk updates and explicitly filters by the
caller, including for administrators. Repeating a partially completed request is
safe because only unread records are updated.

Each scan retains its own issue history. An unchanged, unresolved issue inherits
the preceding occurrence's acknowledgement so an hourly scan does not make it
unread again. A changed issue, or a recurrence after resolution, is unread.
Repeated identical unresolved organization conflicts reuse their existing
notification and preserve its read state. Frontend subscriptions coalesce row
change events and keep existing content visible during background refreshes.
Acknowledged unresolved issues remain reviewable in Notifications → Scan Issues.

### Q77: How are Cast devices and Cast settings written, and how do saved settings reach the running service?
**Answer:** Only through the generated `createCastDevice`/`updateCastDevice(s)`/`deleteCastDevice` and
`createCastSetting`/`updateCastSetting(s)` mutations; there is no hand-written twin, and the Settings screen
adds and removes devices with those (`addCastDevice`, `removeCastDevice`, `updateCastSettings(input)` and a
second `updateCastDevice` were deleted). Content rules live in the write input transform
(`services/cast/write_rules.rs`, applied by `AppWriteInputTransform`), so every write path obeys them:
target must be a literal LAN IP with a valid port, device type one of `CHROMECAST`, `CHROMECAST_AUDIO`,
`DLNA_RENDERER` or `UNKNOWN` (a discovered receiver whose model was not recognised), a blank device name
defaults to `Cast Device (<address>)`, discovery interval 5–86400 seconds, volume 0–1, preferred quality
`original`/`2160p`/`1080p`/`720p`/`480p`. A device update is checked against the LAN rules only when it sets the
address or port, so toggling a favourite never fails on a legacy row. Refusals are returned to the caller; they
are never left to a hook, because deferred post-commit failures are only logged.
Side effects are lifecycle hooks: saving a `CastSetting` whose auto-discovery switch or interval changed applies
them to the running `CastService` after commit; deleting a `CastDevice` ends its session rows in the deleting
transaction and, after commit, stops the receiver. A replaced or removed session ends `IDLE` with its position
kept (the `CastSession` create hook is the single owner of replacement); only a receiver that stops responding or
loses authority ends `DISCONNECTED` with an error.
**Settings versus environment:** the saved `CastSetting` row is what the Settings screen shows, so it wins at
startup. `CAST_AUTO_DISCOVERY` and `CAST_DISCOVERY_INTERVAL_SECONDS` only choose what that row starts as: they seed a
new row, and override an existing row that still holds the untouched seed values. A row an admin has saved is never
overwritten by the environment. Discovery timeout and retention stay environment-only.
**Guard:** `services::graphql::schema::tests::no_root_field_is_defined_by_more_than_one_chunk` fails when a root
field is defined by more than one chunk of the merged root (async-graphql silently keeps only one, which is how a
hand-written `updateCastDevice` went unnoticed). It reads the same chunk list that `schema_roots!` is given.

### Q80: What turns automatic downloading on, and what runs when it is?
**Answer:** One setting, `auto_download.enabled`, default **off**, edited on Settings › Downloads. There is no environment
fallback, so the switch always shows what the server does. While it is off nothing searches, grabs, or
re-searches after blocklisting a dead grab (feeds are still read into their catalogue, Q91). `triggerAutoDownload` ("Run now") and `searchMissing` ("Search now") still run: a
person pressed a button. When it is on, `AutoDownloadService` runs three workers, each re-reading the settings every minute so
a change needs no restart: the **scheduler** (a full sweep of every wanted title every `auto_download.interval_minutes`,
default 60, starting on the first tick after the switch is turned on; plus the air-time pass, Q81, every five minutes), the
**RSS worker** (Q82), and the **retry sweep** (Q70). Feeds are sources, so their catalogue (Q91) is filled and sources are
health-checked (Q96) whether or not the switch is on; only *grabbing* waits for it. All settings are `auto_download.*` app settings; values the scheduler
cannot honour are clamped (`jobs::auto_settings`). What each pass did last is written to `auto_download.status` and shown on the
Downloads screen. Per title: `NONE` never, `WANTED` only rows flagged wanted, `ALL` every missing row that has aired or been
released (Q68); movies are downloaded when `monitored` and `wanted` and their release date has passed.

### Q81: When is an episode searched for after it airs?
**Answer:** Not before `air_stamp` (or, with only a date, the start of that day in UTC) plus `auto_download.air_grace_minutes`
(default 60), because releases take time to appear. From then on, for `auto_download.air_window_days` (default 7) it is on the
quick schedule: searched again every `auto_download.air_retry_minutes` (default 60) until a release is grabbed. After the
window it is back-catalogue and follows the ordinary `search_backoff_minutes` (Q69) for as long as it stays missing in `ALL`
mode, or while it stays flagged wanted in `WANTED` mode. The five-minute **air-time pass** looks only at episodes inside the
window so a new episode is searched within minutes of becoming searchable instead of at the next hourly sweep; it uses the same
backoff map as the sweep, so the two never search the same episode twice in a row. Episodes with no date at all are
back-catalogue. Movies have only TMDB's primary release date (no digital release date is stored), so they get neither grace nor
the quick window: they are searched once that date has passed, on the ordinary backoff. A season pack is searched on the
ordinary backoff; its per-episode fallbacks use the episode's own schedule, and a pack that is still inside its backoff does
not hold the episode searches back.

### Q82: How are RSS feeds used?
**Answer:** `jobs::rss_poll` polls each enabled `RssFeed` on its own `pollIntervalMinutes`, never faster than 15 minutes, with the
wait doubling for every consecutive failure (up to a day) and a `Retry-After` from the host obeyed. Feeds are read one after
another with a pause between them and a 4 MB limit on the response; only RSS 2.0 (including Torznab attributes and magnet
links) is understood, and only `http`/`https`. Feeds are created with the generated `createRssFeed`/`updateRssFeed`/`deleteRssFeed` mutations (admin only; a write rule refuses a name-less feed or an address that is not `http`/`https` and raises an interval below 15 minutes to 15). Items already seen (a `RssFeedItem` row per item, kept 30 days) are skipped. New
items are matched against what the hunt would search for (same modes, quality profile and blocklist; the grace period does not
apply, since a release that exists is a release) **in the feed owner's libraries only**, and in the feed's library when it names
one. A match must name the show or movie exactly after normalising case and punctuation ("The Office" is not "The Office US"),
must agree with any TVDB/TVmaze/TMDB id the item carries, and must be the right episode, season pack or year; a title that does
not say which episode it is for is refused. Among releases of one target the quality profile's ranking decides, and a grabbed
season pack supersedes the episodes inside it. The grab goes through the same path as a search (`grab_release`), with
`sourceFeedId` set. Music and audiobook targets are not matched by feeds: they are per-track and whole-book searches, and an
album or book release cannot be tied to one of them safely. A grab that failed, or one skipped because the cap (Q83) was
reached, is not remembered, so the next poll tries again.

### Q83: How are double and runaway downloads prevented?
**Answer:** (1) One process-wide lock covers every search-and-grab (sweep, air-time pass, manual search, retry, RSS), so two
cannot both choose a release for one episode; a target with a non-failed, non-blocklisted torrent is not a candidate at all,
and the torrent service refuses a second record for an info hash. (2) At most `auto_download.max_concurrent_grabs` (default 5)
automatic grabs may be downloading (linked to a wanted target, not completed, not failed); beyond that, targets are not searched
(so no backoff is spent) and wait for a free slot. A person pressing Search now on one title is not held to the cap. (3) Every
candidate carries what a release must be to count as it (`Expectation`): the right episode, season pack or movie year. A search
narrowed by season and episode may keep a title that does not say which episode it is; one naming a different episode is never
taken. (4) A release with no magnet is fetched through its source's login before it reaches the torrent client, exactly as the
manual add does, because some trackers hand a `.torrent` only to a logged-in client.

### Q84: What trail does automatic downloading leave?
**Answer:** Notifications of category `auto_download` (Activity), owned by the owner of the target's library, never by a user the
job picked. A **grab** gets its own `SUCCESS` notification (what, from where, which profile, what triggered it: sweep, air-time,
manual, retry or a named feed) linked to the torrent. A target that was searched with no result keeps **one** open "Waiting for …"
notification, rewritten each time with why nothing was taken and when it is next searched: no release found (or the sources that
failed), releases found but below the quality profile (with the commonest reasons), blocklisted, for another episode, season or
year, or a title too different. It resolves when the target is grabbed. Back-catalogue episodes (past their quick window) and individual music tracks only keep an already-open one up to date and never open a new one, so a first sweep of a show with two hundred missing old episodes does not open two hundred notifications; manual searches always open one. A failed grab ("Could not grab …"), the cap being reached
("waiting for a free slot") and a blocklisted dead grab are likewise single notifications. Skipping a target because it is inside
its backoff is not recorded per tick; it is visible as the "next search after" time on the waiting notification and in the debug
log. Every search, grab and skip is also logged with the target, trigger and reason, per `logging.mdc`.

### Q85: Where can a library's files live, and how does Librarian reach them?
**Answer:** A library root is **a folder on this machine (the default) or a folder on an SMB share**, chosen per library when it is
added; a network share is an optional alternative, and libraries on both can exist side by side. Cloud buckets (S3 and similar) come
later and the model below is built so they fit. SMB is read by Librarian itself, with no operating-system mount, because the
backend runs in unprivileged containers (an unprivileged LXC has no `mount -t cifs`, and Docker is the same).

*The location.* Everything that names a library file keeps one string, as it does today: `Library.path`, `MediaFile.path`, the folder
paths on shows, albums and audiobooks, sidecar subtitle paths. A local path is unchanged (`/data/media/Movies/Film (2020)/Film.mkv`).
A share is `smb://host[:port]/share/folder/file.mkv`: host and share lower-cased, no user name or password in the string, folder names
case-preserved, `.` and `..` refused. `services/library_storage` parses it (`StorageLocation`, `StoragePath`; nothing else splits
these strings). `MediaFile.relative_path` is the part below the library root, so a root can be swapped without re-deriving it.
*Credentials* are per share, not per library: the sign-in for `smb://host/share` is saved once, encrypted with the key that
already protects network-path passwords, in the same app-setting record (`network_mount.<digest of the share address>`) with
`platform = smb-native`. Libraries on one share find it through `CredentialRef`; entries of that platform are never mounted,
reconnected or listed as network paths, and the password is never logged or shown.

*The interface.* Every read or write of a library file goes through `Storage` (`services/library_storage`): `stat`/`lstat`,
`list_dir`, `walk`, `open_read` (a reader that seeks, so a player can scrub), `read_all`, `put_from_local`, `rename`,
`remove_file`, `create_dir_all`, `remove_dir` (only if empty), `is_dir_empty`, `tool_input`. `LibraryStorage` routes by path: local
paths to `LocalStorage`, an `smb://` path to the backend registered for its share, and a share that is not connected is
`Unavailable`, never "file missing" (a down NAS must not make a scan delete every file's record). Callers never assume what a
filesystem gives and ask `StorageCapabilities` instead, so a bucket can say no:
- **No hard links.** "Place without overwriting" is `PlaceMode::NoClobber` on `put_from_local` and `rename`. Locally it is the
  existing hard-link-then-unlink (with copy-to-temp across devices); on SMB it is a create-or-fail or a rename without replace,
  both atomic on the server; on a bucket it is a conditional put. `atomic_no_clobber` says which one can be relied on.
- **No real directories.** On a bucket a folder is only a prefix: `create_dir_all` does nothing, an empty folder cannot exist, and
  empty-folder tidying is skipped (`real_directories`).
- **No atomic rename.** `rename` on a bucket is copy then delete and can stop half way; callers that move a file check
  `atomic_rename` and verify the copy before the delete, and never rename between two storages (the registry refuses it: a
  move across storages is `put_from_local`, check, `remove_file`).
- **No file identity.** `EntryMeta.identity` (device and inode locally, the file id on SMB) is optional; "are these the same
  file" falls back to path equality, size and time.
- **No path for other programs.** `tools_need_url` says external tools cannot open the file (see below).
Torrent downloads stayed local at first (`torrent.download_dir`; since Q103 they can be on a share too), as do extraction staging, HLS scratch output and database backups. An
import from a download **copies onto the library's storage** (`put_from_local`, `NoClobber`) and never moves out of the download
folder, so a seeding source is never touched, exactly as with a local library.

*Choice of SMB client* (spike on 2026-11-08 against `//truenas.localdomain/media`, SMB 3.1.1 over 10 GbE, release builds, other agents
loading the machine): **`smb2` (vdavid, 0.43)**. Both crates connect, sign in, list, stat, read at any offset, write, rename without
replacement, delete and make folders. The deciding difference is latency. `smb2` sets `TCP_NODELAY`; `smb` (smb-rs 0.12) does not, so
every request waits out the 40 ms Nagle/delayed-ACK stall: `smb2` against `smb` was 0.3 ms against 176 ms for a stat, 1,000 small
files created in 2.4 s against 134 s, a 1,000-file walk in 29 ms against 2.8 s, a random 1 MiB read in 4.4 ms against 48 ms, an
open-read-close in 1.7 ms against 132 ms, and a sequential read 308 to 329 MiB/s against 21 MiB/s with one request outstanding
(`smb` can only keep up if the caller builds the pipelining; its parallel read hung twice, and eight parallel 4 MiB writes on one
handle hung once, though 1 MiB writes ran at 203 to 342 MiB/s). Scrubbing a video is a few random reads and a library scan is tens of
thousands of stats, so only `smb2` fits. It also reconnects a dead session itself for reads, listings and stat, and deliberately not
for writes, renames and deletes (they may already have happened), so the backend retries only the idempotent calls and reports the
rest after one explicit reconnect (measured: a stat after a dropped connection took 19 ms; a write failed at once, then worked after
reconnecting). CHANGE_NOTIFY on a folder tree works with `smb2` (an event arrived 0.5 s after a file was created), which the later
"watch for changes" work can use instead of polling. The live backend is a debug build, where the crates' signing and encryption run
unoptimised: the same `smb2` spike built in debug managed 33 MiB/s writes and 38 MiB/s reads against 285 and 308 MiB/s in release, so
the dev profile raises `opt-level` for `smb2` and its crypto dependencies (`[profile.dev.package.*]` in `backend/Cargo.toml`).

*External tools* (ffprobe, ffmpeg for HLS, subtitles and clips, fpcalc) open a path. For a file on a remote storage the backend
serves it to them on its own loopback address: `http://127.0.0.1:<port>/internal/tool/<token>/<file name>`. The token is random,
bound to one resolved `StoragePath` (never a path a client supplies), short-lived (extended while a read is in flight, revoked when the
HLS session or scan step ends) and accepted only from a loopback peer; the URL ends in the real file name so ffmpeg still
guesses the container from the extension. The route answers `HEAD`, `Range` and `Accept-Ranges: bytes` from `open_read` with a
read-ahead buffer, and the tools get `-protocol_whitelist file,pipe,http,tcp` (the subtitle extractor today allows only
`file,pipe`). `Storage::tool_input` returns `ToolInput::Path` for local files, so local behaviour does not change at all, and
`ToolInput::Url` for remote ones. A tool that cannot take a URL (to be checked per binary, `fpcalc` first) is given a staged
copy in the scratch folder instead. Whisper reads only the WAV clips ffmpeg cuts, which are already local.

*Moving a library* (`moveLibrary`, an action mutation, not generated CRUD; as built, Q88): changing a library's
root must rewrite its records, not rescan, so nothing is re-matched and every `MediaFile` id, with its movie, episode, track,
progress and playlist links, survives. Librarian does not copy library content in this step; the person copies the folder first
(rsync or cp, preserving times). (1) *Plan* (read-only query): connect to the new root, walk it once, and for each `MediaFile`
compute `new path = new root + relative_path`; report matched, missing (first 50 named) and extra files, and refuse to proceed
with any missing unless told to. Reject a new root inside the old one or the reverse on the same storage. (2) *Apply*: mark the
library as moving (an app-setting record via the entity layer; scans, organize and imports skip a library with the marker, and
it is resumed on startup), then rewrite in chunks through generated mutations in one `MutationContext` each: `MediaFile.path`
(and `size` and `file_modified_at` from the new storage, so a copy that changed times is not read as a changed file and
re-analysed), `Show.path`, `Album.path`, `Audiobook.path`, sidecar `Subtitle.file_path`, and last `Library.path`. Rows are
rewritten from `relative_path`, so a stopped move resumes by running it again. Downloads, pending matches and torrent paths are
not library paths and are not touched; open scan issues for the library are closed (they hold old paths). (3) *Verify*: one
scan must report no new, removed or changed files. Moving back is the same action with the old root. A bulk "set path prefix"
capability would be added to the entity layer first if chunked mutations prove too slow for tens of thousands of files; direct
SQL is not an option.

*What moved in the first step.* The interface and the local backend carried the library discovery walk, empty-folder tidying,
duplicate hashing, duplicate trash moves, same-file checks, the stream endpoint, and the existence and size checks of the music
and identification passes, with no behaviour change. The SMB backend, the share-aware pipeline, the tool URL service and the move
are Q86 to Q88.

### Q86: How does a library on a network share behave?
**Answer:** The same as a local one, through `Storage`, with these differences. **Adding it:** in Add library and in a library's
settings the location is either a folder on this server (the default) or a network share: server (a name or an IP address),
share, a user name and password, and a folder browsed over the share. *Test connection* signs in, lists the share's top level
and, if it works, saves the sign-in encrypted for the share (`connectShare`); a server name that does not resolve from the
machine Librarian runs on is reported as such ("try its IP address"). Saving the library normalises the address (`smb://`
host and share in lower case) and refuses a root that cannot be read or is not a folder. **Connections:** `SmbStorage` keeps up to
three signed-in sessions per share, connected on first use from the saved sign-in, and replaces a session known to be dead before
anything is sent on it. A read, listing or stat that fails because the connection dropped is repeated once; a write, rename or
delete is never repeated, because it may have happened: it fails as "unavailable", says so, and the next call starts afresh.
**Unavailable shares:** a scan of a library whose share cannot be reached records a `LIBRARY_STORAGE_UNAVAILABLE` issue and stops
without touching the catalogue. A share that stops answering mid-scan, any unreadable folder, or a walk that finds nothing in a
library that had files, keeps every file record: nothing is removed or marked missing on the strength of a walk that was not
complete, and a check that cannot be answered counts as "present". The failed connection is remembered for ten seconds so a scan
does not wait out a connection timeout per file. **Imports:** a download is *copied* to the share (to a name no scan takes for
media, then renamed into place without replacing), and the download stays where it is so its torrent keeps seeding; extraction
output stays next to the download. **Organizing** renames on the share (rename without replace, atomic on the server);
emptied folders are removed only when the share says they are empty. **Contained paths:** on a share a path is contained by its
name alone (under the root, no `..`), as there are no symlinks. **Refused on a share, clearly:** quality upgrades (they replace a
file after proving its device, inode and change time, which a share cannot give; the better copy stays in the downloads and the
log says why), copying between two shares, and copying within one.

### Q87: How do external tools read a file on a share?
**Answer:** Through `/internal/tool/<token>/<file name>` on the backend's own port (`api::tool`, `services::tool_url`). The token is
random (two UUIDs), bound to one resolved path, expires after 30 minutes without a request (each request counts), and the route
answers only a loopback peer (or the single address the server is bound to). It serves `GET` and `HEAD` with `Range` and
`Accept-Ranges: bytes` from `open_read`, so ffprobe and ffmpeg can seek; it sits outside `/api`, takes no login, and refuses
anything else. Tool callers ask `tool_input`/`tool_argument`: a local file gets its path, unchanged, and a file on a share a URL.
ffprobe (`-protocol_whitelist file,pipe,http,tcp` added for a URL), the HLS remux, transcode and on-demand segment ffmpeg runs,
the WebVTT subtitle conversion (whose whitelist stays `file,pipe` for local input) and the whisper audio clips all go through it.
`fpcalc` is not run by the product today (`services/fingerprint.rs` is not part of the build, see the note in the music matcher), but
the helper it will use is in place and checked: Chromaprint 1.5.1's `fpcalc` reads the tool URL directly (its input is opened with
ffmpeg's libraries, which speak http), and `run_on_input` repeats a failed run on a copy staged in scratch space (at most 1 GiB, removed
afterwards) for a build that refuses it. Residual risk, as for
any ffmpeg input: a file disguised as a playlist could make ffmpeg open other http addresses; a library's files are the
administrator's own.

### Q88: How is a library moved?
**Answer:** Changing a library's root rewrites its records; it never copies files, and the person copies the folder first
(keeping file dates). *Library settings > Move*, or the library's *Move library…* action: choose the new location (a folder or a
share), **Check the files** (`libraryMovePlan`, read only) and read the result, then **Move library** (`moveLibrary`); a
verifying scan starts afterwards. The plan walks the new root once and compares it with every file record by the part of its path
below the root: files found with the same size (`matched`), not found (`missing`), found at another size (`changed`, a copy that did
not finish) and media files the library does not hold (`extra`, found by the next scan). It refuses a new root inside the old one or
the reverse, one that cannot be read, and a library with files outside its own folder. Applying refuses while anything is missing or
changed unless *Move anyway* is chosen, and while a scan is running. It then marks the library as moving (an app-setting record
`library_move.<id>`; scans, organizing and imports refuse a library with the mark), rewrites in batches through the generated update
mutations (`MediaFile` path, relative path, size and modified time, taken from the new storage so a copy that changed times is not read
as a changed file; show, album and audiobook folders; sidecar subtitle paths), closes the library's open scan issues (they name old
paths), changes `Library.path` last, and clears the mark. Every record is rewritten from the part of its path below the root, so a
move that stopped is finished by running it again, and the server resumes marked moves at start. The records keep their ids, so
matches, progress and playlists survive, and nothing is re-matched or re-analysed. The originals are never deleted. Moving back is the
same action with the old location.

### Q90: Is an RSS feed a `Source`?
**Answer:** In the product, yes: Settings › Sources lists indexers and feeds in one table, "Add source" starts by choosing
*Indexer* or *RSS feed*, and a feed's row has the same identity, state, "Test now" (for a feed, "Poll now") and priority-free
actions as an indexer's. In storage, no: a feed stays its own `RssFeed` entity with `RssFeedItem` rows. Merging them into
`Source` was weighed and refused. A `Source` is a *search adapter* (loaded into `SourcesManager`, queried on demand,
admin-only, no owner); a feed is a *pull catalogue* with an owner (`userId`), an optional library, per-item rows kept 30 days,
and `Torrent.sourceFeedId` pointing at it. Folding it in would have meant migrating `RssFeedItem.feedId`,
`Torrent.sourceFeedId`, the ownership policies and the poller for no change the user sees. Where the two must behave alike
they share code instead: the same credential encryption, the same health vocabulary (Q96) and the same Notification trail.

**Secrets.** A feed address carries the tracker passkey (`...?u=123456;tp=<passkey>`). `RssFeed.url` is therefore an
encrypted envelope written by the same write transform as `Source.credentials` (`v2:` AES-256-GCM, input-only, absent from
the GraphQL object); `urlDisplay` holds the redacted form (host, path, parameter names, every value hidden) for the UI. Rows
written before this are encrypted the next time the poller runs. A feed item's download link is stored the same way
(`RssFeedItem.link`) and is never sent to a client. `services::redact` removes keys, passkeys, userinfo and magnet
trackers from every address in an error, a log line, a notification and a grab; HTTP client errors are stripped of their
URL before they are formatted.

### Q91: What does Librarian keep of a feed's items?
**Answer:** Every item, as a catalogue of torrents that exist right now (30 days, Q82), fully read by
`services::feed_catalogue` with the parsers the hunt already trusts: `quality::scoring::parse_release` for title facts
(year, season, episode, pack, resolution, source, codec, HDR, audio, group), the description's `12.1 GB; Movie/HD/Bluray`
for size and category, Torznab numbers for the kind, and `read_music_title` for `Artist - Album (Year) [FLAC 24bit]`
(artist, album, format, bitrate; audiobooks the same with author and title). A pack is one of `episode`, `season_pack`,
`series_pack` (a multi-season range or "Complete Series"), `movie`, `album`, `book`.

Each item is **pre-matched** to the owner's libraries with a confidence and a reason, or kept unmatched with the reason it did
not match. The exactness rules are the hunt's: a show is found by `rss_poll::acceptable_names` ("The Office" is not "The
Office US"; a bare name that fits two shows matches neither), a movie must agree on the year to within one, and an item
that does not say which season or episode it is stays unmatched. Albums need the same artist and album
(`music_rules::same_artist`/`same_album_title`); a book needs title and author and is never a multi-book pack. A match is a
label (`matchKind`, `matchLabel`, `matchConfidence`, `matchReason`, and the show/movie/album/audiobook ids): it never
authorises a grab, which still checks the target's `Expectation`. Unmatched items are matched again on later polls (a
title may have been added), so the catalogue improves as the library grows.

Feeds are read whether or not automatic downloading is on (Q80); only matching to *wanted* targets and grabbing need the
switch. Music and audiobook items are matched for display and recommendations, never auto-grabbed (Q82).

### Q92: Where do feed items show up?
**Answer:** In **Find releases**: the catalogue answers a release search for the same target without a request to the
tracker, as one result per feed (marked "From feed", source name `<feed> (feed)`), graded by the quality profile like any
other result. A feed release has no link a client may see: it carries a `feedItemId`, and `addTorrent` reads the link
server-side (the caller must own the feed or be an admin). In the **feed's table** (Settings › Sources › "What it has
listed"): a bounded, server-side, batch-loaded table of its newest items with quality, size and what each one is in the
library, filterable to matched or unmatched. Auto-grab from feeds is unchanged (Q82).

### Q93: What is a Recommended release, and how is it chosen?
**Answer:** For anything missing, the best release found ahead of time, shown as **Recommended** with a Download button on
missing seasons (and their episodes), the series, movies, albums and audiobooks, and pinned at the top of Find releases. It
is stored as a `ReleaseRecommendation` (target, scope `episode`/`season_pack`/`series_pack`/`movie`/`album`/`track`/`book`,
the release, which missing items it covers, short reasons, `foundAt`, `expiresAt`).

**One brain with auto-download.** `jobs::recommend` builds candidates with the hunt's own discovery
(`discover_candidates`, run non-grabbing and, for on-demand lookups, with `include_unwanted`), the season-pack rule
(`should_hunt_season_pack`), the per-episode candidates and `Expectation`; searches with the hunt's
`collect_releases_from`; drops blocklisted releases; and ranks with `quality::profile::filter_and_rank` through
`rank_for_candidate`, which is what `pick_for_candidate` (the hunt's pick) takes the first of. Two scopes are added
because the hunt grabs neither: a complete-series pack (`Expectation::SeriesPack`) and a whole album
(`Expectation::Album`). One deliberate difference: within the profile's best quality tier the recommendation prefers the
**smallest** release (one with at least half the top release's seeders), so a recommendation can differ from the hunt's pick
between two releases of equal quality.

**What covers the gap.** For a season, one search (packs and episodes come back together) decides: a season pack if the hunt
would look for one and an acceptable one exists, else the episodes that have acceptable releases. A show with two or more
such seasons also gets a series-pack lookup. Albums are looked up whole; a track is looked up on request. Audiobooks are
whole books. A row says how many missing items it covers ("covers 8 of 10 missing").

**Inputs, cheapest first.** The feed catalogue (Q91) is tried first and settles a lookup when it has an acceptable release
for every missing item or a pack; indexers are searched only if it did not.

**Grabbing.** `grabRecommendation` goes through `grab_release_with`, the hunt's grab path, carrying the same wanted-target
linkage (so it counts as downloading for the item), after taking the hunt lock. The download link is stored encrypted (an
indexer release) or re-read from its catalogue item (a feed release) and is never readable.

### Q94: How are recommendations kept from hammering indexers?
**Answer:** Indexer requests for recommendations are counted against `auto_download.hourly_search_budget` (default **30** per
rolling hour, 0 turns them off; Settings › Downloads); one request is one `search_all` round, so a season costs two (its
"S02" and "Season 2" variants) and a feed-settled lookup costs none. **Background** lookups cover only items flagged wanted
(or in `ALL` mode), every 30 minutes, at most 8 targets a pass. **Everything else is on demand**: opening a season, movie,
album, audiobook or the show queues a lookup (`lookupRecommendations`), a worker answers it within the budget, and the page
says "Looking for the best release…" meanwhile. An answer, and a "nothing found", is kept for a day (`expiresAt`), so it is
not searched again. A row disappears when its release is blocklisted or gone from its feed, when what it covered is no
longer missing, when anything for the target is grabbed (by anyone), or when it expires. The sources' own rate limiters and
the hunt's backoff apply underneath, and the budget is in memory (a restart forgets it, which at worst repeats one hour's
searches).

### Q95: Does a pack download everything?
**Answer:** Not by default. `auto_download.only_missing_files` (default **on**, Settings › Downloads): when a pack or album is
grabbed from a recommendation, the torrent is added paused, its file names are read, and only the files that fill what was
missing are selected (`torrent::partial`, librqbit's `update_only_files`): episodes by season and number with their
subtitles, tracks by number or title, release-wide artwork and sheets. A selection that matches nothing or cannot tell
files apart downloads everything, never fewer than were asked for. **Ratio:** on a private tracker a partial download never
completes the torrent, so it cannot be seeded to 100%, and some trackers count an unfinished torrent against the account;
the setting says so. **Import:** the client's totals count only the selected files, so the torrent completes when they do,
and the import already skips files that are not fully downloaded. Not verified end to end against a real partial pack.

### Q96: How is a source's health checked, and who is told?
**Answer:** `jobs::source_health` tests every enabled source at least once an hour, one at a time with a random 5–45 second
pause between (never a burst), with `SourcesManager::test_source`. A real search that succeeded since the last check is the
check and nothing is requested; one that failed counts as a failed check. A feed's poll is its check. The result is
`Source.healthStatus` (`ok`, `failing`, `auth_expired`), `healthKind` (`auth`, `unreachable`, `rate_limited`, `other`),
`lastCheckedAt`, `nextCheckAt` and the existing `lastError`/`errorCount`/`lastSuccessAt`/`lastErrorAt` (feeds the same on
`RssFeed`). Failures are classified from HTTP status and message (`sources::health`): 401/403 and "not logged in" are
authentication; DNS, timeout and 5xx are unreachable; 429 is rate limiting. A source that keeps failing is tested hourly,
then every 2, 4, 8, 16 and 24 hours; **Test now** (and Poll now for a feed) ignores that.

**Alerts.** One `source_health` Notification per failing source or feed, for every admin and, for a feed, its owner (the same
open-and-update, never-duplicate approach as the auto-download trail): "IPTorrents can't sign in: its login has expired.
Update its credentials in Settings › Sources." A sign-in problem alerts at once; any other failure after two checks in a row.
The notification is rewritten if the reason changes and resolved when the source recovers. Messages never contain
credentials, cookies or addresses. In Sources, a failing row is red with the reason and when it last worked.

### Q97: How does IPTorrents sign in?
**Answer:** With a username and password (the default) or a pasted cookie (advanced). With a password, `iptorrents_login`
reads the login page, finds its form (the one with a password field, with its action and hidden fields), posts the
credentials and keeps the `uid`/`pass` session cookies from the answer, stored encrypted with the credentials. When a request
finds the session ended, the source asks `SourceSessions` for a new one and retries once. **A sign-in is never attempted more
than once an hour per source** (`Source.loginAttemptedAt`, written before the attempt; two requests in the same minute share
one), so a wrong password cannot lock the account. A CAPTCHA, a Cloudflare challenge, a two-factor step, a missing form or an
unrecognised answer is detected and reported as a failure that tells the user to use the cookie method, which raises the
auth alert (Q96). Nothing logs the username, password or cookies. If signed in, "Create RSS feed from this account" reads the
site's RSS page (`getrss.php`), takes the personal link, and creates a feed from it (Q90) whose address is redacted
everywhere.

**Unverified.** No request has been made to the real site. The flow (`login.php`, the `username`/`password` fields,
`take_login.php`, the cookie names, the RSS page printing the link) is inferred from Jackett's and Prowlarr's definitions and
reports; the parsers read the form from the page instead of assuming it, and are tested against hand-written fixtures in
`backend/tests/fixtures/iptorrents/`.

### Q98: How does the hunt tell that a release is for the wanted title?
**Answer:** By the release's **title part**, word for word (`quality::title_match`), not by a similarity score. Until
November 2026 `filter_and_rank` dropped a release only when the Jaro-Winkler similarity of the *whole* normalised release
title (quality tags included) against the wanted title fell under 0.70. A shared prefix carries Jaro-Winkler a long way:
`Babylon.Berlin.S01.1080p.10bit.WEBRip.6CH.x265.HEVC-PSA` passed for **Babylon 5**, was recommended as "covers 22 of 22
missing", and the same gate guards automatic grabs. `Expectation` checked only season, episode and year, so nothing caught it.

Every hunt target now carries a `WantedTitle` (`WantedCandidate.match_title`), and `filter_and_rank` rejects any release
whose `WantedTitle::mismatch` is not `None`, before quality is looked at:
- **Shows and movies** (`Work`, the show name or movie title plus aliases, today a movie's original title): the words of the
  release are walked from the start. Every significant word of the name must come, in order, and the first word after the
  name must be a *marker* (SxxEyy, `2x05`, `Season`, `Complete`, a year, a resolution, a number of two or more digits as
  anime numbering uses, a source/codec/edition tag) or the end. So `Babylon.Berlin.S01` is refused (`berlin` where `5`
  was wanted) and `Silo.Rising.S01` is refused for Silo. Tolerated: `the`/`a`/`an`/`and`/`&`, punctuation, apostrophes,
  accents, `S.H.I.E.L.D` as `SHIELD`, `SpiderMan` as `Spider Man`, a leading `[Group]` or `www.site.org -`, an `aka` alias in
  the release, and a year or a country code (`US`, `UK`, `GB`, `AU`, `CA`, `NZ`) after the name unless the wanted name says
  another (`The Office (US)` refuses `The.Office.UK`; `Doctor Who (2005)` refuses `Doctor.Who.1963`). Two years in a row
  mean the first is part of a longer name (`Blade.Runner.2049.2017` is not *Blade Runner*).
- **Music** (`Music`: artist, album, track): every word of the artist and every word of the album or the track, as whole
  words (`Queen` is not `Queens of the Stone Age`). A whole album is still also checked by `Expectation::Album`.
- **Audiobooks** (`Book`): every word of the short title present and `release_match::title_score` at least 0.75 (author and
  narrator words are ignored, so extra words are fine). Other books of a series are still dropped by `release_relevance`.

The similarity score remains only as the last ranking tiebreak. A release turned down this way counts as "named after a
different title" in the no-release explanation, with one example ("'Babylon.Berlin.S01…' names 'babylon berlin', not 'Babylon
5' (names 'berlin' where '5' was wanted)"). Feed matching keeps its own exact-name rule (`rss_poll::names_match`) for
labelling catalogue items.

**Stale answers.** Every `ReleaseRecommendation` row carries `rulesVersion` (`jobs::recommend::RULES_VERSION`, now 2). A row
from older rules (including every row written before the column existed) does not count as an answer: lookups ignore it, it
cannot be grabbed, and the next pass removes it, so the Babylon Berlin row is replaced within one worker tick rather than
after its day. Raise the version whenever a change means stored answers may be wrong.

### Q99: Can a torrent be looked into before it is grabbed?
**Answer:** Yes: its file list (names, sizes, piece layout) is read without downloading anything, by
`services::torrent::inspect`, and kept as a `TorrentContents` row keyed by info hash.
- **A `.torrent` link** (Torznab, IPTorrents, a feed item): fetched the way a grab fetches it, through the source's login
  when the link is on the source's own site (`fetch_torrent_from_source`), else by the client in list-only mode, and parsed
  locally. No peer or tracker is asked, so a private tracker never sees an announce, and the passkey in its announce
  address is never stored (only the info dictionary is read, and for a private torrent not even that is kept).
- **A magnet** (The Pirate Bay and the like): `TorrentService::read_metadata_only` asks librqbit 9.1 for it in list-only
  mode (`AddTorrentOptions { list_only: true, paused: true }`). librqbit resolves the info dictionary from peers (DHT,
  the magnet's trackers) and returns `AddTorrentResponse::ListOnly` before creating storage, a handle or a session entry,
  so nothing is downloaded, written or left in the session. A hard 40-second timeout drops the attempt (and its peer
  connections). A torrent the session already holds answers from its handle.
- **A feed release** (catalogue item): its link is revealed from the item, as a grab reveals it, and read as above.

Each file is described once (`ContentFile`): video, audio, subtitle, sample, extra (featurettes, behind the scenes,
`Extras/`), info (`.nfo`, `.txt`, `.cue`...), image, ebook, archive, other; with the season and episodes a video's name or
`Season N/` folder gives, and an audio file's track and disc numbers. Padding files are left out but indexes stay the
torrent's own, so a selection can be passed to `only_files`.

**Cache and budget.** Metadata is immutable, so a row is never refreshed; the same hash is never read twice. A `.torrent`
link with no info hash in its listing is also found again by `linkKey`, a SHA-256 of its source and guid (never the link
itself, which can hold a passkey). `.torrent` downloads count against `auto_download.hourly_search_budget` per rolling hour;
at most two magnets are resolved at once; a release that could not be read is not tried again for six hours. Callers
inspect only the top few candidates of a target (Q100). Errors are redacted before they are logged or shown.

`infoBytes` keeps a public torrent's info dictionary (base64) for a later grab that skips magnet resolution; grabs do not
use it yet, because a torrent added from a file loses the magnet its `Torrent` row keeps for restoring.

**Tested** with two hand-made `.torrent` fixtures (`backend/tests/fixtures/torrents/`, built by `make_fixtures.py`: a
private season pack whose announce holds a fake passkey, and a public album) and with a magnet resolved from a librqbit
seeder on loopback, which checks that the reading session keeps no torrent and writes no file, and that a magnet nobody
seeds gives up within its timeout. Never run against real indexers or the public DHT.

### Q100: How do a release's files change whether it is taken?
**Answer:** Its files beat its name. `services::torrent::judge` maps an inspected torrent (Q99) to what is wanted:
- **Episodes** (a season or the whole show): video files that are not samples or extras, numbered by their names
  (`S01E05`, `2x05`) or by a `Season N/` folder and a leading number (`Season 2/05 - Title.mkv`), and, for a single-season
  pack, the pack's season for a file that gives only an episode. A pack whose numbered files mostly name another show is
  **rejected** ("its files are 'babylon berlin', not 'Babylon 5'"); one that holds none of the missing episodes is rejected
  ("holds none of the missing episodes (it has S01E01–E18)"); otherwise it counts for exactly what it holds ("contains 18 of
  22 missing episodes"), which is how a vaguely named pack ("Babylon 5 Complete DVD Rip") covers the seasons its files show.
- **Tracks**: audio files, by title first (numbers repeat across discs) and then by a unique track number.
- **A book or a movie**: it must have audio (or video) files; an ebook-only release is rejected.
- Files packed in archives, or videos that say no episode, cannot be checked: the release is judged by its name as before.

The files that fill what is missing are named exactly: the matching videos or tracks, their subtitles (same name or same
episode) and release-wide `.nfo`/artwork at the top level, never samples or extras. With `auto_download.only_missing_files`
on they are passed to the grab as `WantedFiles::exactly` (indexes into the torrent), which `select_wanted_files` takes as
they are when they fit the torrent, before falling back to matching names (Q95).

**In the hunt** only season packs are looked into, the best three ranked releases at most (`INSPECT_PER_TARGET`): among
those not rejected, the one holding most of what is missing is grabbed, with its files selected; a release that could not
be read is taken by its name only when none of them could be read; if all are rejected nothing is grabbed, the reason
("2 release(s) turned down by their files: ...") goes to the trail, and the per-episode fallbacks are searched as when no
pack is found. Single episodes, movies, tracks and books are grabbed by name, as a look costs a request or a magnet
resolution per grab. **Recommendations** use the same judge for their plans (Q101).

### Q101: What does Recommended show for a show, and how is it chosen?
**Answer:** A **coverage plan**: the smallest set of releases that covers every missing episode of the show, mixing a
complete-series pack, season packs and single episodes for the gaps, shown as "Recommended: 3 releases cover 24 of 44
missing" with the releases listed, a **Download all**, a Download on each release, and what is still uncovered ("Still
missing: S02E03–E22 (no usable release found)"). It replaces the per-season answers of Q93 for shows; movies, albums and books
still get one release each.

**Building it** (`jobs::recommend::look_up_show`, pure rules in `jobs::recommend_plan`):
1. The missing episodes come from the hunt's discovery, as before (wanted only in the background, any missing on demand).
   Opening any season, episode or the show asks for the same lookup: the whole show.
2. Releases are gathered cheapest first: the feed catalogue, then a complete-series search (two or more seasons missing),
   then each season's search, **only for seasons the plan does not yet cover**, stopping as soon as everything is covered
   or the hourly budget (Q94) runs out.
3. Each release must name the show (Q98) and pass the profile (`filter_and_rank`, size limits per covered episode). Its name
   says what it claims (`recommend_plan::claim_of`): episodes, a season or a range, the complete series, or a vague
   "Complete" pack. The four most promising packs are looked into (Q99) and their files override the name (Q100): another
   show's pack or one holding none of the missing episodes is dropped; a short pack counts for what it holds; a vague pack
   counts only once its files are read.
4. `plan_cover` picks the releases, **quality first**: the best resolution tier of the profile is planned first and a lower
   tier only fills what it leaves; within a tier, greedily the release adding the most still-uncovered episodes (so one
   complete series beats three season packs, and packs beat episodes), ties to the better-seeded (in bands) and then the
   smaller; a release the others of its tier or better cover is dropped. Each release is credited only with what it adds.

**Stored** as `ReleaseRecommendation` rows (additive columns `planId`, `planPosition`, `coversSeasons`, `fileIndices`): one
row of scope `plan` (counts, uncovered note, summary reasons; status `ready`, `none` with the why, or `searching`) and one per
release (`episode`, `season_pack` or `series_pack`) pointing at it, with what it adds (`coversIds`, `coversCount`), its
seasons, and the exact files to download when its files were read. At most 60 releases per plan.

**Downloading.** `grabRecommendation` downloads one release; `grabRecommendationPlan` downloads every release of the plan in
order through the same path (reported per release, so a failure does not stop the rest). A pack takes only the files for
what it adds when "Download only the missing files" is on (Q95, exactly by index when inspected). A grab removes only what
it makes pointless: the release itself, the same episode, or another pack of the same season, never the plan's other
releases. The plan row goes when it has no releases left; a release goes when what it adds is filled or a torrent was
grabbed for exactly it. Rules version 3 retires every earlier per-season answer.

**In the UI** (`RecommendedRelease`): Find a release shows the show's whole plan whichever season is searched. A show page
shows the whole plan above the seasons when two or more seasons are missing something, and each season lists its part of it
(the releases covering that season); with one season missing, that season shows the whole plan.

**Not built:** plans for an artist's discography or an album set (albums are still recommended one at a time, looked into
for their tracks), and plans that weigh a smaller lower-quality pack against several better releases (quality always wins).

### Q102: How are file servers and their shares saved?
**Answer:** As a **server**, the `NetworkServer` entity (Settings › Network shares): name, host (lower case, a name or an IP
address), port (empty for 445), user name, and a password that the generated mutations accept and the write transform
encrypts (the `Source.credentials` envelope); it is never readable through GraphQL, and `hasPassword` says whether one is
stored. A server is saved once, and every share on it uses its sign-in: SMB signs in per server, not per share, and the
user's own setup (one TrueNAS, one user, several libraries on one share) means one entry. **Shares are listed, not typed:**
`smb2` speaks srvsvc `NetShareEnumAll` (`SmbClient::list_shares`, the list Windows shows for a server), so choosing a
location lists what the server offers (`networkServerShares`, administrative `$` shares left out). A server that signs the
user in but refuses the listing still works: the share is typed once and kept in `knownShares`, and every share that was
used or listed is remembered there, so a picker can offer them while the server is down. A location stays one
`smb://host[:port]/share/folder` string (Q85); the sign-in for it is the saved server with that host and port
(`network_servers::find_server`, used by `NetworkShareProvider`).

*Lifecycle hooks, no hand resolvers:* creating or updating normalises the host, port and share names, encrypts a new
password (an already encrypted envelope is kept as it is) and refuses a second server for the same host and port. Removing
a server, or changing its host or port, is refused while a library, the download folder or a stored torrent is on it, with
the reason ("TrueNAS cannot be removed while it is used by the libraries Movies, TV Shows and the download folder. Move
them to another location first"), shown to the person as it is (the refusals carry `BAD_USER_INPUT`). After any change the registry forgets that server's connections, so the next use signs in with
the new details. `testNetworkServer` (an action: sign in, list shares, optionally open one share) and `networkServerShares`
are the only hand-written operations; a test of a saved server records `status`, `lastError` and `lastCheckedAt` on it.

*Where it is chosen:* every location picker (Add library, Library settings, a library's Move tab, the download folder) is
"This server | Network share"; a share is a saved server, then one of its shares, then a folder (typed or browsed). "Add
network share…" in the server list saves a new server without leaving the picker and chooses it. The server has its own
identity (glyph, a dot for whether it answered, address and user), actions (Server settings, Test connection, Remove) and
settings dialog (Connection, Shares, Remove).

*Carried over:* sign-ins saved before servers existed (`network_mount.<digest>` app settings, platform `smb-native`) become
servers at startup (`carry_over_saved_sign_ins`), once each, with the stored envelope copied as it is: the password is
never decrypted for this, and the old record is left in place as a fallback until a server exists for that host.
`connectShare` now saves into the server too.

### Q103: Can downloads be on a share, and what reads them there?
**Answer:** Yes. `torrent.download_dir` is a folder on this machine or an `smb://` folder on a saved server, and torrents are
written there by Librarian itself (Q104). Everything that reads a download goes through `library_storage`, as library files
do (Q85): identification reads text files and ffprobe facts through `Storage` and the tool URL (Q87), and the stream and
analysis paths already did. What had assumed a local download path, and does now:
- *Import into a library* never moves a download. A download on a share is copied: into a library on the same share with an
  SMB server-side copy (`FSCTL_SRV_COPYCHUNK`, `Storage::copy_within`: no bytes cross the network, to a temporary name and
  then renamed without replacing), into a library on another share by streaming it through Librarian, and into a local
  library by copying it down to a temporary name beside the target and linking it into place without replacing
  (`LibraryStorage::copy`). A missing-file check that cannot reach the share says so instead of "does not exist".
- *Archives* on a share are unpacked on this machine: the entry volume and its own other volumes (`extract::archive_set`)
  are copied to `CACHE_PATH/share-extract/<hash>/archive` (refused with a clear message when local space is short),
  unpacked into `…/unpacked`, and the copies removed; the output is imported from there as before. An archive unpacked
  completely once is not copied again.
- *Quality upgrades* are not offered from a download on a share (they prove the better copy by device, inode and change time,
  as they do for a library on a share), and the log says why; the copy stays in the downloads.
- *Creating a folder* (`createDirectory`) for an `smb://` path makes it on the share.
- *The startup check* of the download folder probes a share by making the folder and writing and removing a probe file
  through it. A share that is down at start is not a broken setting: the client starts, its torrents wait (Q104), and adding a
  torrent is refused while the share stays down ("The download folder is on a share that cannot be reached now").
- *Partial packs* (`only_files`) and deleting a torrent with its files go through the torrent storage, so they work on a share.
Torrents already in `/data/downloads` keep their folder until they are moved (Q105).

### Q104: How does the torrent client read and write a share, and what happens when the share goes away?
**Answer:** librqbit 9.1 has pluggable, synchronous storage (`StorageFactory`/`TorrentStorage`: positioned reads and writes,
file length, removal, called from blocking threads). The session's `default_storage_factory` is `LibrarianStorageFactory`
(`services::torrent::share_storage`): a torrent whose folder is `smb://` gets a `ShareStorage`, every other one librqbit's
filesystem storage. Two things librqbit does not offer are worked around: the factory is not shown the torrent's folder, so
it is worked out as the session does (the folder its JSON persistence stored for that info hash, read from `session.json` at
start and registered on every add; else the download folder plus the torrent's name when it has several files), and
persistence only saves torrents of the filesystem factory, so the factory reports the filesystem type it wraps, as
librqbit's own middlewares do (its files are laid out the same way).

*I/O:* each file is opened once (`SmbStorage::open_file`, read-write, created if missing) on its own clone of a pooled
connection, so reads and writes from several threads are pipelined over the one signed-in session. Reads and writes are raw
SMB2 READ/WRITE requests at an offset (`SmbFile`, up to 1 MiB each), and the file length is set with SET_INFO
end-of-file. *Writes are gathered:* peers send 16 KiB blocks; they are kept in memory per file, joined while contiguous,
and written as one run when it reaches 4 MiB, when the torrent holds more than 32 MiB unwritten, when a read touches them, and
when the torrent pauses. The read that checks a finished piece writes that piece's blocks first and is answered from them, so
a piece is only marked complete once the server has acknowledged it; unchecked blocks lost to a crash are downloaded again,
as they would be anyway. Handles are closed when a torrent pauses or goes, so the server can delete or rename the files. The
dev profile's optimised `smb2` and crypto crates (Q85) keep signing from being the limit. Measured against a local Samba
(loopback, debug build): a 9.7 MB two-file torrent in 256 KiB pieces downloaded onto the share in 0.21 s, checked after a
pause and resume, re-added over the files with nothing downloaded again, and deleted with its files
(`tests/smb_torrent.rs`).

*Outages:* a storage call that cannot reach the share fails with "The download share is unavailable: …" and reports the
share to the watcher (`share_watch`). Every 20 s the watcher pauses the running torrents on a share that is down (their
status says "Waiting for the download share …, unreachable since …"; `LiveTorrent.statusDetail`) and tries the share again;
when it answers, the torrents it paused are resumed and any that librqbit had stopped with that error are started again.
librqbit then checks what is on the share, so nothing is downloaded twice, and nothing is ever deleted. The torrents it
paused are kept in `torrent.share_paused`, so a restart during an outage still resumes them, and a torrent a person paused
is left alone. Imports of a finished download wait too: a copy that cannot read the share fails without touching the
library, and library scans of a share never mark files missing on a walk that did not finish (Q86).

### Q105: How is the download folder moved?
**Answer:** Like a library (Q88): the person copies the folder first (`rsync -rt`, keeping dates), Librarian checks the copy,
then moves. Settings › Downloads › Download folder › *Move…* picks the new folder (here or on a saved share) and *Check
the files* (`downloadsMovePlan`, read only): every file on disk of every torrent stored under the old folder is looked for at
the same place under the new one, by size (`matched`, `missing`, `changed`, the first 50 named). It refuses the same folder,
one inside the other, and a folder that cannot be read. *Move downloads* (`moveDownloads`) refuses missing or changed files
unless *Move anyway* is chosen (the client then downloads them again), writes a marker app setting
(`torrent.downloads_move`), and for each torrent under the old folder takes it out of the client keeping its files and adds it
back from its own metadata with its folder under the new one, its wanted files (`only_files`) and paused state kept; the
client checks the copied files, so nothing that is there is downloaded again. It then rewrites the records through the
generated update mutations (`Torrent.savePath` and `downloadPath`, `TorrentFile.filePath`, `PendingFileMatch.sourcePath`, and
the file records of downloads), sets `torrent.download_dir`, clears the marker and restarts the client so new downloads go
to the new folder. Every step skips what is done, so a stopped move is finished by the server at its next start. *New
downloads only* changes the folder for new torrents and leaves the existing ones seeding where they are. The old files are
never deleted.

### Q106: Can the torrent session folder be on a share?
**Answer:** No, and it is not worth building. In librqbit 9.1 the session store is chosen by `SessionPersistenceConfig`,
which is `Json { folder }` or `Postgres { connection_string }` only; the `SessionPersistenceStore` trait is public but
`SessionOptions` has no way to pass an implementation (`persistence_factory` in `session.rs` matches the enum), and the JSON
store writes with `tokio::fs` (`session.json` through a temporary file and a rename, the `.torrent` files, the fast-resume
bitfields) to a local path. DHT state (`dht.json`) is a local file name too. A share path would need a fork of librqbit,
and the client must start while the share is down, which a store on the share cannot do; a local copy mirrored to the share
would only be a backup of a few small files that Librarian can already rebuild (every torrent is in the database with its
magnet link and save path, and `restore_from_database` re-adds what the session lost). So the session folder stays on this
machine (`torrent.session_dir`), and only payloads go to a share.

### Q129: How are credits and people kept, for every kind of title?
**Answer:** As `Credit` rows: one person's part in one title. A credit names its title through exactly one of `movieId`,
`showId`, `seasonId`, `episodeId`, `albumId`, `trackId` or `audiobookId` (`targetKind` says which), and carries `kind`
(`cast` or `crew`), `guest` (a guest star of one episode, a cast credit), the provider's `department` and `job`
("Directing"/"Director", "Writing"/"Screenplay", "Production"/"Executive Producer"), `characterName`, `creditOrder` (billing
order; crew gets an order too, directors and writers first), `episodeCount` (show and season credits), `source` (`tmdb`,
`musicbrainz` or `audiobook`) and the provider's credit id. Each credit copies its title owner's `userId`, so the row policy
scopes it directly (owner class) rather than through seven possible parents. The physical foreign keys cascade when a title is
deleted; deleting a person deletes their credits through the hook, so each delete has its event.

*Replacing, not adding:* a refresh calls `CreditStore::replace_credits(title, source, credits)`, which reads the title's
credits from that source and matches each new credit on (person, kind, guest, job, character): a match is updated only if
something changed, a new credit is created and anything left over is deleted. Running it twice changes nothing; credits from
another source are a separate set. Writes are the generated `createCredit`/`updateCredit`/`deleteCredit` mutations, batched
as aliased fields of one document (40 a request), each with its own hooks.

*People* are `Person` rows, extended additively with `imdbId`, `musicbrainzArtistId`, `wikidataId`, `nameKey` (the name
folded for comparison), `biography`, `birthday`, `deathday`, `placeOfBirth`, `knownForDepartment`, `alsoKnownAs`,
`imageUrls` and `detailsFetchedAt`. A person is matched across providers by id, strongest first: TMDB, IMDb, MusicBrainz,
Wikidata. A match gains the ids it lacked (and a real TMDB id replaces a stand-in); TMDB's name and picture win, another
source only fills a gap. A name alone never matches a person who has ids; two people known by name only with the same
`nameKey` are one person (a narrator of two books). `tmdbPersonId` was required and unique before people came from anywhere
but TMDB, and making it nullable is not an additive step, so a person without a TMDB id holds a negative stand-in derived
from their key (`mb:…`, `imdb:…`, `wd:…` or `name:…`; FNV-1a, retried on a collision). Zero and below mean "no TMDB id".

*`MovieCastCredit` is frozen, not removed.* The schema can only grow, so the table and its generated API stay, but nothing
writes it any more and the web reads `Credit`. Writing both would keep two copies of the cast that can disagree, for no reader.
A movie whose TMDB document cannot be read (no API key) has its old cast copied into `Credit` once by the credits job, so it
still shows.

### Q130: What is fetched and kept for each title?
**Answer:** Everything the provider has, in as few requests as before, whether a page shows it yet (`TitleDetail`, one row per
title, beside its credits):
- *Movies:* one TMDB request, `/movie/{id}?append_to_response=credits,release_dates,keywords,alternative_titles,videos,external_ids`,
  which also replaces the two separate credits and release-date requests the details used to make. The parsed result rides on
  the cached `MovieDetails` (`extras`), so adding and refreshing a movie save it without asking again; details cached before
  that are read fresh. Kept: cast and crew (up to 150 each), production companies, origin countries, certifications for every
  country (theatrical first), keywords, alternative titles, videos, external ids and the homepage.
- *Shows:* TMDB `/tv/{id}` with aggregate credits, content ratings, keywords, alternative titles, videos and external ids; the
  show's TMDB id is its own or found from its TheTVDB or IMDb id (`/find`), and is kept in the details, not written to the
  show. Then one request per season it has here (`/tv/{id}/season/{n}` with aggregate credits) gives the season's credits and
  every episode's crew and guest stars. Creators lead the crew. A refresh of the show reads them inline; adding a show wakes
  the credits job (Q132) instead, so adding stays quick.
- *Albums and tracks:* the release read that builds the track listing now asks MusicBrainz for its relationships too
  (`inc=recordings+artist-rels+recording-level-rels+work-rels+work-level-rels+labels`, still one request). Release
  relationships become album credits (producer, executive producer, engineers, mixing, mastering), recording relationships
  track credits (performers, instruments and vocals as cast, credited names kept as `characterName`), and the recording's work
  gives the writers (composer, lyricist). Labels are the album's studios. The client remembers the last 16 releases' credits so
  adding an album saves them without asking again.
- *Audiobooks:* the authors (every author the provider lists, `AudiobookDetails.authors`) are crew ("Writing"/"Author") and the
  narrators cast ("Narration"/"Narrator"), as people known by name. `authorName`, `narratorName` and `narrators` stay filled.
  The publisher and the ASIN, ISBN and Goodreads ids are the book's details.
- *People:* `/person/{id}?append_to_response=external_ids,images` (biography, dates, place of birth, other names, ids and
  every profile picture), read by the credits job and by the `refreshPersonDetails` action.

`TitleDetail` keeps name lists as JSON string lists (`studios`, `networks`, `keywords`, `originCountries`) and structured
lists as JSON text in a documented shape (`contentRatings`, `alternativeTitles`, `videos`, `externalIds`), read defensively by
the web app's `lib/titleDetails.ts`. Videos are links only (YouTube, Vimeo); nothing is downloaded or embedded.

*Locks:* a title whose `lockedFields` has `credits` or `castNames` keeps its credits on every refresh, and a locked
`studios`, `networks`, `keywords`, `originCountries`, `contentRatings`, `alternativeTitles`, `videos`, `externalIds` or
`homepage` keeps that detail. Nothing here writes the title's own fields.

### Q131: Where are credits shown?
**Answer:** Modestly, since keeping the data is the point. A movie page has **Cast** (portraits in billing order, each opening
the person) and **Crew** (Directed by, Written by, Produced by, Music, with the rest of the crew behind *Full crew*), each person
a `PersonIdentity` chip with the person's actions on right-click. Its hero facts add the studio and the trailer link, and the
certification falls back to the stored ratings. A show page has the show's cast (with episode counts) and crew after the
episodes, then **Guest stars in season N** for the chosen season; the focused episode's summary gains one reserved line,
"Directed by … · Written by … · Guest stars …". Its facts add the network when the show has none, the studio and the trailer.
Albums show **Performers** and **Credits**, audiobooks **Narrated by** and **Authors**. All of these read live root queries
(`credits(where: …)`, `titleDetails(where: …)`), so a refresh or the credits job shows up without reloading.

*A person* has their own page, `/people/{id}`: a hero with their portrait, what they are known for, when and where they were
born, their biography and their actions, then shelves of what credits them in the viewer's libraries (Movies, TV, Music,
Audiobooks, each title with their roles; a guest spot names its episode). The person is an entity like any other: its
`EntityIdentity` (`PersonIdentity`: a portrait or initials, known-for and years as facts, a state badge for a death or for
details not read yet) and its action set (`usePersonActions`: open, search the library, open on TMDB or IMDb, and for
administrators *Refresh details*). Cast tiles and person shelves follow the media page conventions: `data-nav-section` per
section, `data-nav-group` on each row and chip list, and portraits from the artwork cache.

### Q132: How do existing titles get their credits?
**Answer:** The `credit_fill` service. It waits 90 seconds after start, then runs a pass every 10 minutes, or 5 seconds after
a show is added (`request_credit_fill`). A pass takes titles that have no `TitleDetail` yet (which is how a finished title is
known) and people whose `detailsFetchedAt` is empty, within a budget: 50 audiobooks (no provider request), 30 movies (one
TMDB request each), 3 shows (one plus one per season), 4 albums (two MusicBrainz requests each, at its one a second) and 120
people (one TMDB request each). TMDB requests go through the client's 12-a-second limit and its back-off on HTTP 429. A title
or person that fails is not tried again until the next start, so a missing provider record does not cost a request every
pass. Titles are never written; locked credits and details stay as they are (Q130). Without a TMDB key the job still fills
audiobooks and albums, and copies movies' old cast (Q129).

### Q115: How does Librarian notice that a library's files changed?
**Answer:** A library with *Watch the folder* (`Library.watchForChanges`) is told by its storage, instead of waiting for the
next scheduled scan. *What ran before:* the setting was saved but nothing read it. Every library was scanned whole by the
scheduler (a 60 s timer that only reads the database and queues libraries whose `scanIntervalMinutes`, 60 for all four live
libraries, had passed since `lastScannedAt`). Over SMB each of those scans lists every folder and stats every file of the
library: on 2026-11-09 the four libraries on `smb://10.0.1.190/media` (about 320 files) took 1.3 to 1.6 s, 0.5 to 1.5 s, 0.2 s
and 0.05 s an hour, so the cost was small but a new file could wait up to an hour to show up. *Now:* `Storage::watch` hands out
a `FolderWatch` of changed paths below a folder.
- *On a share:* SMB2 CHANGE_NOTIFY with `SMB2_WATCH_TREE` on the library root, through `smb2`'s own `Watcher` (0.43 already
  has one: it keeps a request armed at all times, including across its ten-minute refresh, so no event falls between two
  requests, and bounds the wait by connection silence, which its ECHO keepalive makes mean something). The watch has a
  signed-in session of its own, so the long-held request never takes one of the three pooled connections from scans and
  playback. A server that runs out of room for the list (`STATUS_NOTIFY_ENUM_DIR`) is a "rescan everything".
- *On this machine:* inotify through the `notify` crate (8.2, recursive, a watch per folder added as folders appear). A
  queue overflow is a "rescan everything"; running out of `fs.inotify.max_user_watches` is a failed watch that says so.
- *The fallback the request allowed for* (re-listing only folders whose time changed) was not needed: both storages notify.
  A future bucket storage, which cannot, says `change_notifications: false` and refuses `watch`, and its libraries keep the
  schedule.

*The share drops:* the watch ends (`WatchSignal::Lost`, within about 15 s on a silent server, at once on a reset), the
library says "Share not reachable, retrying" (Q117), and the watch is tried again after 5 s, doubling to a minute. When it is
armed again one whole-library scan catches up with whatever changed meanwhile, because nothing told it. That scan keeps the
rules of Q86: a share that stops answering removes nothing. A watch armed for the first time (the backend started, or
watching was just turned on) also starts with one whole-library scan if the library's last one is more than ten minutes old.

*The safety net:* a watched library is still scanned whole at least hourly (its own schedule if shorter; hourly even with
*Scan on a schedule* off), for whatever no notification told. Only these whole-library scans set `lastScannedAt`.

### Q116: What does a change notification lead to?
**Answer:** A scan of just what changed (`ScanScope::Changes`), never of the whole library, unless more than 50 places changed
at once or the storage lost count.
- *Ignored:* any path with a hidden part (the library's `.librarian-trash`, `.DS_Store`, AppleDouble `._` files, rsync's
  `.name.XXXXXX` temporaries), Librarian's own `.librarian-tmp-` copies, names ending `.part`, `.partial`, `.crdownload`,
  `.!qB`, `.tmp` or `~`, and **Librarian's own writes**: every change made through `LibraryStorage` (copies, imports,
  organize renames, trash moves, folders made and removed) and the scanner's local-only placements are noted in
  `library_storage::own_writes` while they run and for 30 s after; a notification for such a path, or for a folder above one
  (its time changes when something in it does), is dropped. So an import is never scanned a second time and an organize
  rename never looks like a file that went away.
- *Gathered per path:* a path below one already waiting joins it (a film copied into a new folder is one scan of that
  folder); a folder whose contents changed is kept apart and dropped if, when looked at, it is a folder that only had its
  own times changed, as its files are reported on their own.
- *Settled first:* a path is looked at once it has had no notification for 3 s: its size and time, or for a folder its
  number of files, bytes and newest time. Two looks 4 s apart must agree, and every new notification starts the looks over,
  so a large copy is never scanned half written (a gone path is two looks of "gone").
- *Queued:* `queue_change_scan` drops paths that cannot matter (a changed `.nfo`, or a gone file nobody catalogued) and
  queues the rest as one scan. A change scan is queued even while the library is being scanned, as the running scan may
  have walked past it; scans of one library run one after another.
- *The scan:* walks each changed folder, stats each changed file, and reconciles only the records at or under those paths
  (a gone path's records are checked one by one, and one that cannot be checked is kept). It matches, analyses and
  organizes what it finds as any scan does, but leaves library-wide tidying (TV season folders, empty folders, collections)
  to whole-library scans and does not set `lastScannedAt`. Its scan run says what it looked at ("Changes checked in Film A
  (2020): 1 found, 0 matched, 0 removed").

A rename by someone else is a gone name and a new one, as it always was for scans: the old record goes and the new file is
matched afresh.

### Q117: Where is a library's watch state shown?
**Answer:** On the library, from three fields the watch writes through the generated `updateLibrary` mutation:
`watchStatus` (`starting`, `watching`, `unavailable`, `failed`, or empty while the library is not watched), `watchDetail`
(why, for `failed` and `unavailable`) and `watchStatusSince` (additive, nullable columns). They are written only when the
state changes, so the generated change events keep every open view current without polling. Library settings › Scanning
shows a *Folder watch* row under *Watch the folder* ("Watching for changes", "Share not reachable, retrying" with since
when, "Can't watch the folder" with the reason); Settings › Libraries has a *Watch* column; and a library's identity shows
"Share not reachable, retrying" as a warning state (after scanning and open issues). The watch supervisor follows the
setting at once: a change to a library's folder or its watch setting nudges it from the `Library` lifecycle hook, and it
also looks again every minute.

### Q107: What do the arrow keys do in a number field on a TV?
**Answer:** Up and Down move focus to the next row, like every other single-line field; they never change the value. Glass
UX is an external package (`glass-ux` from a release tarball), and its `NumberField` is Base UI's: the input steps on
ArrowUp/ArrowDown and stops the event, so on a page (not in a dialog, where the app's capture handler already won) the
remote's Down changed the value instead of moving on. The app now takes Up/Down on a number input in TV mode in the capture
phase of `useSpatialNavigation` (`handleTvNumberKey` in `lib/input-mode.tsx`). The −/+ steppers are `tabIndex=-1`, so a remote
cannot reach them; to change a value without typing it, OK (Enter) puts the focused field into an explicit *adjusting*
state (`data-tv-adjusting`, drawn as a focus ring on the field group), where Up/Down step it as Base UI does; OK, Back or
leaving the field ends it, and Back there does not leave the page. Left/Right keep the text-field rule (they leave once the
caret is at that end). OK on a number field in TV mode therefore no longer submits its form; use the Save button.
Upstream suggestion for Glass UX: a `NumberField` prop (or a TV-mode default) that lets ArrowUp/ArrowDown propagate unless the field
is in an explicit edit state, and focusable steppers in `data-input-mode="tv"`.

**TV app chrome (same entry):** In TV mode (the input-mode flag, which the Android TV app forces) the top bar keeps only the breadcrumbs: the
search field, the Commands button, the Cast button, the Back button and the account are not rendered, so nothing hidden
can take focus. The row stays because the brand mark shares it and the shell grid needs it. Search is the rail's own
Search destination (it was already there); the Search page keeps its type-to-search. Casting stays in the players: the
video player's cast control and the audio dock's Cast button; what is lost is the top bar's list of active cast sessions.
The account becomes the last item of the side rail, a rail item with the unread count, opening the same account menu
beside the rail; Escape returns focus to it. In TV mode the rail scrolls instead of folding items into a More menu, so
the account item is always in the column. The rail list is inset by the focus reach so the first and last items' rings are
never clipped. Outside TV mode nothing changes.

### Q108: What does a playlist entry's menu offer?
**Answer:** The entry's own actions plus its target's. A playlist row (a movie, show, album or book entry) shows the
playlist's actions (Play from here, Shuffle from here, Play next, Add to queue, Add to playlist, Move up and down, Remove from
playlist, Copy titles) and then the target's own action set from `useMovieEntity`, `useShowEntity`, `useAlbumEntity`
and `useAudiobookEntity`: Find releases, Refresh metadata, Add to or Remove from Wanted, its settings dialog and the
rest. `liftActions` (`features/personal/entryActions.ts`) turns a target's set into entry actions: each runs on the
entries' targets, shows only when every selected entry points at one of that kind, and its label, confirm,
disabled reason and success message resolve for those targets. Left out because the entry already does them its own way:
Play (the entry plays from its place in the list), Open, the queue pair, Add to playlist, and Remove from the library, which
should not sit next to Remove from playlist. Episodes, tracks and chapters have no action set of their own yet, so their
entries offer the playlist's actions and Open; giving those three entities sets would put theirs here with no change to
this page.

### Q109: What does a lone chapter do when it ends, and how does a list card draw its picture?
**Answer:** A chapter queued on its own (from a playlist or the queue) stops where it ends. When the file holds more than the
chapter (a whole-book file, or a chapter placed inside a part), its queue entry carries `endPosition` (its start plus its
length, `chapterEnd` in `features/personal/playlistPlayback.ts`), and the audio dock treats reaching it as the entry
ending: it pauses and calls `playerStore.advance()`, which moves to the next queue item or asks the continuation resolver.
A chapter that runs to its file's end, and every per-chapter file, carries none. Whole-book and by-part entries that
play a book from the page still run through, with their chapter marks. The seek bar of a bounded entry still spans the
whole file. On `/lists` a card shows up to four distinct covers from the list's first entries in one square tile
(`mosaic` on `PosterCard`: two side by side, three with the third spanning the bottom, four in a 2×2 grid, the layout the
identity's picture already uses); entries from one album or show make one tile, so a list built from a single album shows
that cover alone, and with fewer than two covers, or when covers fail to load, the card falls back to the single cover and
then to the playlist glyph.

### Q118: Which subtitle files belong to a video, and how do they move with it?
**Answer:** Files next to the video that are named after it, or kept in a `Subs` folder beside it
(`services::subtitles::sidecar`). Each one becomes a `Subtitle` row with `sourceType` `sidecar` and its full path. Paths
are strings, so local paths and `smb://` addresses work the same way.
- **Where to look:** `Film (2020).srt`, `Film (2020).en.forced.srt` and `Show.S01E01.eng.sdh.srt` in the video's own
  folder. In `Subs/` (or `Subtitles/`, `Sub/`), files named after the video, or named only by language (`Subs/2_English.srt`)
  when that folder's parent holds one video. Season packs use `Subs/<video name>/3_French.srt`. When two videos' names both
  start a subtitle's name (`Film.mkv` and `Film.Extended.mkv`), the longer name wins. `Aliens.en.srt` does not belong to
  `Alien.mkv`, because the video's name must end at a separator.
- **What a name says** (`parse_name_tokens`): a language as a code (`en`, `eng`, `fre`, `pt-BR`) or a name (`English`,
  `Français`), stored as ISO 639-2/B like ffprobe's `eng`. Flags: `forced`/`foreign`, `sdh`/`hi`/`cc` for hearing
  impaired, and `default`. Leading numbers are ignored. Any other words become the title (`Commentary`). Two-letter codes
  that are also common words (`it`, `no`, `is`) only count when nothing else in the name is a language.
- **Formats:** `.srt`, `.vtt`, `.ass`/`.ssa`, and `.sub`. A `.sub` with an `.idx` beside it is one VobSub picture
  subtitle (codec `dvd_subtitle`), listed by its `.idx`. A `.sub` on its own is MicroDVD text.
- **When:** the library scan walk now reports subtitle extensions for movie and TV libraries. They are never made into
  media files. After a complete walk (no unreadable entries, share not down), each video's rows are made to match what
  was found: new ones are created, renamed flags are updated, and rows for files that are gone are removed. A walk that did
  not finish changes nothing. After ffprobe analyses a video (a new file, an import, a changed file), its folder is looked
  at again for that one video. A `downloaded` row keeps its kind and is never listed a second time as a sidecar.
- **Imports** (`materialize_torrent_file`): a downloaded video's sidecars are copied next to its new library path and
  renamed after it, like the video. They are never moved, so the torrent keeps seeding. The rows come from the analysis
  that follows.
- **Organize** (`move_media_file_to_pattern`): the video's folder is looked at first, so sidecars no scan has seen yet are
  included. After the video's move is recorded, each sidecar is moved with no-clobber placement to
  `<new name><what followed the old name>` (`film.en.forced.srt` becomes `Film (2020).en.forced.srt`). One not named after
  the video takes `<new name>.<lang>[.forced][.sdh].<ext>`, with `.2` added when the name is already taken. A VobSub's
  `.sub` goes with its `.idx`. The row is then pointed at the new path. If a target already exists, that sidecar is left
  where it was.

### Q119: How are subtitles downloaded?
**Answer:** From OpenSubtitles.com's REST API (`services::subtitles::opensubtitles`, `provider`), in the player
("Find subtitles…"), or automatically after an import when that is turned on.
- **Settings** (Settings › Metadata, *Subtitles · OpenSubtitles*, next to the other metadata providers): the API key
  (`subtitles.opensubtitles.api_key`), and an optional username and password. The key and password are encrypted as they
  are written: the `AppSetting` write hook encrypts the keys in `SECRET_SETTING_KEYS` with the source-credential envelope.
  They are write-only: the page shows whether each is set, sends one only when it is typed, and *Forget key* clears both.
  Also here: preferred subtitle languages (the existing `subtitles.preferred_languages`, moved from Settings › General,
  English when empty), hearing-impaired preference (`subtitles.hearing_impaired`: either, prefer, avoid) and the automatic
  fetch (`subtitles.auto_download`, off by default). Nothing is seeded: the key is saved through the normal admin setting
  path (`createAppSetting`/`updateAppSetting`, or this page). *Test* signs in when an account is given, and otherwise
  checks the key with one small search.
- **Matching:** one search per request, using the file's OpenSubtitles hash (the size plus the 64-bit words of the first
  and last 64 KiB, read through storage, so a share costs 128 KiB) together with the movie's IMDb or TMDB id, or the show's
  ids plus season and episode. When there is no id, the title is searched. Parameters are sent sorted and in lower case,
  as the API asks.
- **Ranking** (`opensubtitles::rank`): a hash match first, then the preferred languages in order, full subtitles before
  forced-only ones, the hearing-impaired preference, human over machine or AI translation, and trusted uploaders. After
  that comes `log10(downloads) + rating / 2`, where the rating only counts with three or more votes.
- **Downloading:** with an account saved, Librarian signs in (`/login`), keeps the token for 12 hours, uses the host the
  API names for VIP accounts, and downloads on that account's allowance. With only a key, it calls `/download` with the
  key alone. If the API wants a signed-in user for that key, the person is told to add the account. The file is asked for
  as SubRip, checked to parse, and saved as UTF-8 next to the video as `<video>.<lang>[.forced][.sdh].srt`, adding `.2`
  when the name is taken. A `downloaded` row records the provider's file id (the `opensubtitlesId` column, unused until
  now), `downloadedAt`, and the release, uploader and remaining allowance in `metadata`. Because the file sits next to the
  video, it moves when the video is organized and works in other players.
- **Limits:** requests are capped at 4 per second, shared by every user. A 429 says "slow down, try again in N seconds".
  The daily download allowance (406 from `/download`) is reported as "OpenSubtitles download limit reached: You have
  downloaded your allowed 20 subtitles for 24h. Your quota will be renewed in …". Neither is retried in a loop. After each
  download the player shows how many downloads are left today.
- **Automatic, only for files without subtitles.** Many videos come with subtitles, so after a movie's or episode's video
  is analysed (when the automatic fetch is on and a key is saved), it is left alone if it already has any of these
  (`provider::auto_skip_reason`):
  - a subtitle in a preferred language, embedded (text or picture), sidecar or downloaded;
  - an embedded track with no language marked (`und`);
  - closed captions inside the video stream (EIA-608/708). Analysis records these in the additive `MediaFile.closedCaptions`
    column, from ffprobe's `closed_captions=1` or "Closed Captions" in the stream description.

  Forced-only tracks (just the foreign parts) do not count. A file that is not skipped gets one search and one download:
  the best human-made, full subtitle in the most preferred language that has one. This happens once per file per server
  run. "Find subtitles…" in the player stays available for every file, and shows why the automatic fetch skipped it
  ("Has English subtitles built in", from `searchSubtitles.autoSkipReason`).
- **Closed captions in the player: not built.** ffmpeg can only pull EIA-608 out by decoding the picture
  (`movie=…[out0+subcc]`), which takes about as long as the film for a whole file. That is far beyond the 20-second
  subtitle route. It would need a background extraction job caching a `.vtt`, like the read-along transcript below.
  Untested lead: libx264 passes A53 captions through (`a53cc`, on by default), and hls.js shows CEA-608 in HLS
  segments, so transcoded playback may already carry them.
- **Subtitles bundled in a torrent** are handled on import (Q118): the same placement rules find a video's files
  (`Movie.en.srt` next to it; `Subs/English.srt`, `Subs/2_English.srt` for a single-video torrent; `Subs/<episode>/2_English.srt`
  in a season pack; `.idx`+`.sub`; `.ass`/`.ssa`). They are copied next to the imported video with library names
  (`sidecar::carry_names`: `<video>.en.srt`, `<video>.en.forced.srt`, `<video>.en.2.srt` for a second English file). The
  names are deterministic and copies are no-clobber, so importing the torrent again adds nothing. They then count as
  present for the rule above.
- **Access:** `searchSubtitles`, `downloadSubtitle` and `fetchLyrics` are actions for members who can read the file
  (`authorized_media_file`). `testOpenSubtitles` is for administrators. The search is a mutation rather than a query
  because each call spends the provider's allowance. Credentials are never logged, and `Credentials` debug-prints with
  them redacted.
- **Tests** use a recorded search response (`tests/fixtures/opensubtitles/search.json`) and a loopback server that copies
  the API's login, download, quota and error answers. The real API is never called in tests.

### Q120: How does the player show subtitles, and what about picture subtitles?
**Answer:** Glass UX's captions menu lists every text subtitle: embedded, sidecar and downloaded. Each label gives the
language, its flags and where it came from ("English · SDH · Downloaded"). SDH tracks are given `kind: captions`.
- **One way to WebVTT** (`/api/media/{file}/subtitles/{id}.vtt`, `services::subtitles::cues`): sidecar and downloaded
  files are read through storage (a share needs no ffmpeg) and converted without ffmpeg. UTF-8 and UTF-16 are read by their
  byte order mark; other files are guessed by `chardetng`, since old SubRip files use Windows code pages. SubRip, ASS/SSA
  (`Dialogue:` lines read by the `Format:` line; italic, bold and underline kept; `\an7–9` becomes `line:0`; drawings
  dropped; other styling lost), MicroDVD (at its declared or the video's frame rate) and WebVTT are supported. Embedded
  text streams still go through ffmpeg, and its WebVTT is parsed again by the same code. Cue text keeps only balanced
  `<i>`, `<b>` and `<u>`. Everything else is escaped, so a subtitle file cannot inject markup or forge a timing line.
- **Timing offset:** Glass's player (0.19.1) has no subtitle delay, so the route takes `?offsetMs=` (±10 minutes) and
  shifts every cue. Cues that end before zero are dropped. The player's *Subtitle options* menu moves the timing ¼ s
  earlier or later and can reset it. Changing the offset changes the track URLs, and the chosen track is shown again.
  The offset lasts for the file being played.
- **Remembered per show:** the language and flags chosen (or Off, or a burned-in picture subtitle) are saved for the
  signed-in user and the show, and applied to the show's next episode (`pickSubtitle`: same language, then same flags,
  then text over pictures). They are stored on the device (`librarian.pref.subtitles.show.<user>.<show>`), not on the
  server: a per-user entity would need its own personal-policy wiring for one small preference. Movies start with
  subtitles off, as before.
- **Picture subtitles (PGS, VobSub, DVB): burned in.** OCR would need a new external tool and gets text wrong. Listing
  them as "needs transcoding" would still leave them unwatchable. The on-demand HLS encoder already re-encodes every
  frame with libx264, so an overlay adds little cost: `burnSubtitle=<subtitle id>` on `/info`, the playlist and the
  segments makes the variant an on-demand transcode, with `-filter_complex [0:v:0][0:<index>]overlay=eof_action=pass`. A
  local VobSub sidecar is added as a second, equally seeked input (`[1:s:0]`). The burn-in is part of the session key, so
  it is its own variant, and seeking and resuming work as with any other transcode. Limits: only for files with a known
  length (others answer 422). Not for casting (a receiver plays what its session chose). Not for a VobSub sidecar on a
  share, because ffmpeg opens its `.sub` by name next to the `.idx`, which a loopback URL cannot provide. Embedded picture
  streams on a share do work. The player lists picture subtitles under "Burned into the picture", and choosing one
  restarts the stream from the same position.

### Q121: Where do lyrics come from, and how are they shown?
**Answer:** From an `.lrc` file beside the track, the track's tags, or LRCLIB, in that order of trust. They are cached in
one `Lyric` row per media file (an additive entity, table `lyrics`). The row holds: `mediaFileId`, `trackId`, `source`
(`sidecar`, `embedded`, `lrclib` or `none`), `synced`, `syncedLyrics`, `plainLyrics`, `language`, `instrumental`,
`providerId` and `fetchedAt`. It is readable by whoever can read the file and written only by the server, through the
generated mutations. It is deleted with its file (`cleanup_media_file_dependents`).
- **Tags** (`services::lyrics::embedded`): read with `lofty` 0.25, now a real dependency (this document claimed it before
  it was). The file is opened through `library_storage` and read through a blocking bridge with a 64 KiB buffer, so a
  track on a share is read in place, and only the tag blocks are fetched (no audio properties, no pictures). Supported:
  ID3v2 `SYLT` in milliseconds (one entry per line, or word pieces joined at their line breaks) and `USLT`; Vorbis/FLAC
  `LYRICS`/`UNSYNCEDLYRICS`; APE `Lyrics`; MP4 `©lyr`. A tag holding LRC counts as synced.
- **Sidecar:** `<audio name>.lrc` (or `.LRC`) next to the file, up to 1 MiB. `[offset:]` is applied, `[la:]` is kept, and
  enhanced-LRC word times are dropped.
- **LRCLIB** (`lrclib.net`, free and keyless, setting `lyrics.lrclib_enabled`, on by default): `/api/get` is called with
  artist, title, album and duration, then `/api/search` by artist and title. A search result is only taken within two
  seconds of the track's length, or, when the length is unknown, when the names match exactly. Requests are limited to 2
  per second. Only those four details leave the server.
- **Choosing:** this server's synced lyrics, then LRCLIB's synced lyrics (keeping this server's plain words when it has
  them), then this server's plain lyrics, then LRCLIB's plain lyrics or its "instrumental". With nothing, a `none` row is
  kept and the track is not looked up again for 30 days, unless someone presses *Look again* (`fetchLyrics(refresh:
  true)`). If LRCLIB is down, that is not recorded as a miss.
- **Stored form:** synced lyrics are normalised to one `[mm:ss.cc]text` line per time, sorted, so the web reads them by
  splitting lines (`features/player/lyrics.ts`).
- **In the player** (`LyricsPanel`, beside the artwork in Now playing, chosen with *Up next | Lyrics*): the line being sung
  is highlighted and kept centred. Scrolling by hand holds the view for 4 s. Every line is a button that seeks to it, so a
  remote walks the lines with its arrows (`data-nav-group`, the sung line is the group's default) and presses OK to jump
  there. Plain lyrics are shown as stanzas. The source is credited ("Lyrics from LRCLIB").
- **Read-along for audiobooks (not built).** `services::whisper` already transcribes the opening of a book and chapter
  boundaries. A transcript would mean running whisper.cpp over the whole book, which takes tens of minutes of CPU for a
  long book, as a queued low-priority job like analysis, and writing its segments (`-osrt`/`-oj` timestamps) as synced
  lines. The `Lyric` row cannot hold this as it is: a book's text is long, and per-file rows break for a chapter that
  spans files. A `Transcript` entity (audiobook, file, start, end, text, per segment, written in batches) would feed the
  same `LyricsPanel` (lines, tap to seek, auto-scroll) with the chapter's segments. It should be opt-in per book, because
  of the cost and because a book's text is copyrighted material kept on the server.

### Q122: How are seek-bar previews made and served?
**Answer:** As JPEG sprite sheets made after analysis by the preview queue (Q124), from one keyframe per tile. A tile is
taken every 10 seconds, or less often for a long file so no file needs more than 360 (a two-hour film gets one every 20 s);
tiles are 240 px wide (at most 180 px tall), shaped like the picture inside any black bars (Q123), which are cropped away
first, and use the stream's display aspect ratio, so anamorphic files are not squashed. Ten by ten tiles make a sheet, so a
film has a few sheets and the last may have fewer rows. Each frame is one ffmpeg run with a fast seek (`-ss` before `-i`,
`-noaccurate_seek`) that decodes one keyframe (`-skip_frame nokey`) near the middle of its tile's stretch, single-threaded at
the lowest CPU priority, so a film on a share is never read end to end; HDR frames are tone-mapped when ffmpeg has `zscale`.
The raw frames are drawn into sheets in the backend (`image`); a missing frame repeats the one before, and more than a quarter
missing gives no storyboard. Sheets are cached at `<CACHE_PATH>/storyboards/<file>/<key>/<n>.jpg`, where the key hashes the
file's id, size, modification time, layout and crop, and older sets for the file are removed. `MediaFile.storyboard`
(`key`, `interval`, tile size, `columns`, `rows`, `frameCount`, `sheetCount`) describes them, and
`GET /api/media/{file}/storyboard/{key}/{n}.jpg` serves them to the file's owner or an admin, like the other media routes,
with `Cache-Control: private, max-age=31536000, immutable` because a key never names other pictures. The player gives Glass's
`storyboard` for one sheet and its `thumbnail` callback, which also picks the sheet, for several. If the cache is cleared,
the sheets 404 until the file changes or its `previewsCheckedAt` is cleared.

### Q123: How are black bars baked into a video found and used?
**Answer:** With ffmpeg `cropdetect` in the preview job (Q124), at ten points from 10% to 90% of the file (intros, logos and
credits are often full frame or black), six consecutive frames each after a fast seek, with a fractional `limit=0.1` so 8-bit
and 10-bit video read alike, `skip=0` and `reset=0`, so each point gives the union of its frames. The edges (`x1 x2 y1 y2`) are
read, not the rounded `crop=`, and black frames (inverted edges) are skipped. A reading smaller than a quarter of the frame
either way is a dark scene and is dropped. At least three readings must remain; the biggest group that agrees within 1% (at
least 8 px) must hold 60% of them, and if 20% or more show a bigger picture than the group (IMAX scenes, a long full-frame
intro) nothing is kept, because zooming would cut real picture. The group's union, snapped outwards to even pixels, is the
picture; bars thinner than 2% of the frame are ignored. It is stored as `MediaFile.pictureCrop` (`left`, `top`, `width`,
`height`, and the `frameWidth`/`frameHeight` it was measured in), and is `null` for a full frame or an inconclusive file.
**In the player:** a *Picture size* menu (only for a file with a crop) offers *Fit the whole frame* (as before) or *Fill the
screen*, remembered per device (`librarian.pref.player.pictureMode`). Fill scales the `<video>` with a CSS transform, about its
centre, until the picture meets the box on one side, and re-centres it; the box clips the rest. It is worked out from the
crop as a share of the frame, so direct play and HLS renditions of any size zoom alike, and it follows resizes and view
modes. On a box no wider (letterbox) or taller (pillarbox) than the picture it changes nothing. Nothing is re-encoded.

### Q124: When does the preview job run, and what does it cost?
**Answer:** One file at a time, on its own queue (`services::video_preview`), every ffmpeg run single-threaded at nice 19.
A job reads the record, skips anything not a video (content type `video`, a video codec that is not cover art, a length and a
size), runs crop detection (Q123) then the storyboard (Q122), and records both with `previewsCheckedAt` through the generated
`updateMediaFile` mutation, unless the file's size or modification time changed meanwhile. Files on a share are read through
one loopback tool URL per job (Q87). Jobs come from: **playback**, as the stream or HLS playlist of a file without previews is
first requested, at the front of the queue (a file is looked up at most every ten minutes); **analysis**, at the back, after
each successful analysis; and **backfill**, when the queue has been empty for two minutes, five files at a time that were
analysed but never done, with a 20-second pause after each. Backfill is controlled by `playback.video_preview_backfill`
(Settings › General › Playback, *Make previews for existing videos*, on by default); playback and analysis queue files
either way. A failed job (a share offline, ffmpeg missing) is not tried again for six hours. When a scan sees a file change,
its `pictureCrop`, `storyboard` and `previewsCheckedAt` are cleared with its analysis, so it is done again. Schema: the three
`media_files` columns are additive (`picture_crop` and `storyboard` JSON text, `previews_checked_at`).

### Q125: Why not generate previews in one ffmpeg pass, or re-encode for fill?
**Answer:** A single pass (`fps=1/10,tile=10x10` or a `select` on keyframes) reads the whole file, which over a share is the
whole film across the network, and decoding every frame of a 4K HEVC film costs far more CPU than a few hundred keyframes.
Per-tile seeks read a few hundred kilobytes each. The keyframe found may be a few seconds before the tile's moment, which a
preview tolerates. Fill is a view of the same stream: cropping in the transcode would need a second rendition per file and
would break direct play, while a CSS transform costs nothing and can be turned off at once.

**Added later to Q125 — remote control of the players.** *Media keys* are handled in one place
(`web/src/features/player/mediaKeys.ts`), a window capture listener, so they work without focusing
anything: the video player while it holds the screen, else the audio player; the Android TV shell
dispatches these same `KeyboardEvent.key` values, and the audio Media Session handlers (play, pause,
next/previous track, seek, stop) call the same commands; Glass publishes the video's. *Scrubbing*
(`scrub.ts`) moves a target, not the playhead: 10 s a press, 30 s from the fourth press in a row
(presses within half a second, or a held key's repeats, throttled to one step per 150 ms), then a
minute (two for items over an hour); a lone tap is 10 s. The player seeks once, on OK or 0.7 s after the
last press, so HLS sees one seek; Back cancels. The video player scrubs from its picture, its seek bar,
or anywhere while its controls are hidden (TV mode; elsewhere Glass's own arrows stay), showing the
target time, the delta, the storyboard tile (Q122) and the chapter; it takes the keys before Glass
does rather than reconfiguring it, since Glass has no option for step size or deferred seeking. The
audio seek bar scrubs the same way and names the audiobook chapter. *Now playing*
(`nowPlaying.tsx`): on a TV a rail entry at the top while anything plays, the Info key and N open it;
starting audio on a TV opens the drawer (Q125's maximised form) with Play/Pause focused, counted by
`audioStarts`, which only a new queue increments, so Add to queue, Play next into a playing queue and
the queue moving on never take over the screen. Back collapses it and focus returns to the opener.

### Q137: How does Librarian run on an Android TV?
**Answer:** As a thin native shell, `android-tv/`. It is a Kotlin app with one Activity and one
full-screen WebView, and it loads the web app from the person's own server. A TV without Chrome
cannot install the PWA, and a WebView shell is the smallest thing that can. The web app already has
TV mode and remote focus (STYLE_GUIDE "Remote focus"), so the shell adds only what a browser tab
cannot do.

*Why not Capacitor:*
- Capacitor packages a bundled web app with a plugin bridge. Here the page comes from the server, so
  the bundle and most of the runtime would go unused, and the AndroidX dependency tree would add
  megabytes on a TV with about 390 MB free.
- The stage 2 player (Q140) is one `addJavascriptInterface` object either way.
- The shell uses platform APIs only: no AndroidX and no native libraries. The user's TV is 32-bit ARM
  only, and a pure Java/Kotlin APK runs on every ABI.
- The release build is minified with R8.

*Manifest:*
- `LEANBACK_LAUNCHER` and `LAUNCHER`, with a 320 × 180 banner. The banner and the launcher icons are
  drawn from the brand art by `web/scripts/generate-pwa-assets.mjs --android-tv`; the adaptive icon
  uses the mark as a vector drawable.
- leanback and touchscreen are both `required="false"`.
- minSdk 24, target 37.
- Cleartext http is allowed (network security config), with system trust anchors only.
- `enableOnBackInvokedCallback="false"`, so Back still reaches the Activity as a key on Android 16+.

*The WebView:*
- JavaScript and DOM storage are on. IndexedDB comes with JavaScript.
- `mediaPlaybackRequiresUserGesture=false`.
- `LOAD_DEFAULT` cache: the server's headers decide.
- Cookies are accepted and flushed on pause, and third-party cookies are refused.
- No zoom, `textZoom` 100, no file or content access, no extra windows.
- User agent suffix `LibrarianTV/<versionName>`.
- Renderer priority is `IMPORTANT`, waived when the app is hidden.
- `onRenderProcessGone` (a crash, or Android reclaiming the renderer) destroys the WebView and builds
  a new one at the page's last address (`doUpdateVisitedHistory` follows client-side navigation).
  When it happens in the background, the rebuild waits for the app to come back.
- `onTrimMemory`: while running, it asks the page to give memory back (the page garbage-collects
  the Apollo cache). In the background, it releases the WebView.
- Links to another origin leave the app (`ACTION_VIEW`) and never load in the WebView.
- The bridge also ignores calls from any page that is not the server's.
- A main-frame load error shows a native "Reconnecting" screen (below).

*The 10-foot canvas.* The user's TV is a TCL G07: Android 11, 1920 × 1080 at density 320. At that
density the page's `width=device-width` gives a 960 × 540 CSS viewport, a phone layout blown up.
TV mode's 1.18× root font is flat, and text zoom and font scale are 1, so neither was the cause.

The shell sets the page's viewport meta to `width=<W>, viewport-fit=cover`, where
`W = 1920 × 100 / displaySize` (`TvCanvas`):
- It runs in `onPageStarted` (less flash) and again in `onPageFinished`, and the WebView has
  `useWideViewPort` on.
- Chromium applies a changed viewport at once. Measured on the TV: 1920 × 1080 at 100 % (Home shows
  six cards and the Libraries row), and exactly 1600 × 900 at 120 %.
- `devicePixelRatio` still reports 2; only the layout width changes.
- The canvas is a fixed 1920 rather than the panel's pixel width. A TV whose app window really is
  3840 px wide (density 640) then gets the same 10-foot layout instead of a quarter-size one.
- The web app never writes the viewport meta (nothing in `web/src` does), so nothing undoes the
  shell's.

What failed on the G07: building the WebView on a context with a lower `densityDpi`. A
`ContextWrapper` overriding `getResources()` was ignored (still 960 at 2×). A bare
`createConfigurationContext()` left the WebView at 0 × 0, blank, because it could no longer find its
Activity. The WebView stays on the Activity.

*Display size* (80, 90, 100, 110 or 120 %) is the one appearance knob added since appearance was
locked:
- TVs differ in size and viewing distance, so it is offered in the TV app only (Settings ›
  General), next to Blur.
- The shell keeps it (`DisplayPrefs`) per TV. The page reads and sets it through the bridge
  (`displaySize()`, `setDisplaySize()`).
- A change re-runs the viewport script without a reload, keeping the scroll position.

*Reconnecting.* A main-frame load error marks only that navigation as failed: every
`onPageStarted` clears the mark, so a later successful load always removes the screen. (A sample
shell's 30 s timeout latched its error screen until a manual retry, on every backend restart.)
While failed, the shell shows "Reconnecting to <server>. Trying again in N s" with *Try again* and
*Change server*. It retries after 2, 4, 8 and 16 s, then every 30 s.

*Remote keys* go through `dispatchKeyEvent` (`RemoteKeys`, `PressTracker`). The arrows, OK, Enter
and the volume keys are not intercepted. The rest map as follows:

| Remote button (Android key code) | Reaches the page as |
|---|---|
| Back, gamepad B | `window.librarianTv.back()` (below). Held 700 ms: the server screen |
| Menu, TV contents menu | `librarianTv.menu()`: a `ContextMenu` keydown, then a `contextmenu` event at the focused element if nothing took it. Held: the server screen |
| Play/pause, headset hook | `MediaPlayPause` |
| Play / Pause / Stop | `MediaPlay` / `MediaPause` / `MediaStop` |
| Next / Previous | `MediaTrackNext` / `MediaTrackPrevious` |
| Fast-forward, skip forward, step forward | `MediaFastForward`, repeating while held |
| Rewind, skip backward, step backward | `MediaRewind`, repeating while held |
| Info | `Info` |
| Guide | `Guide` |
| Channel up / down | `PageUp` / `PageDown`, repeating while held |

- Each button arrives as a standard `KeyboardEvent` at the focused element
  (`librarianTv.keyEvent(key, "keydown" | "keyup", repeat)`, see web/src/lib/tv-app.ts): one keydown,
  then `repeat: true` keydowns while held for the repeating keys, then one keyup.
- What the page does with the media keys, Info and Guide belongs to the player and the "Now playing"
  work (scrubbing previews), not to the shell.
- `back()` does, in order: an open dialog or menu gets Escape; fullscreen is left; on `/` it returns
  `"exit"` and the app closes; otherwise a `GoBack` key goes to the focused element, and the shell's
  remote focus (or the video player) takes it and goes back in-app history.
- A page that does not take Back, or an older page without the bridge, falls back to
  `WebView.goBack()`, then finishing the app.

*Keep the screen on:* the video player and the audio dock hold `LibrarianTV.setKeepScreenOn`
while playing (`holdScreenOn`, one token each), which sets `FLAG_KEEP_SCREEN_ON`.

*The TV's volume.* The player's volume control is the TV's media volume (`STREAM_MUSIC`,
`VolumeSync`):
- **Bridge:** `volume()` returns `{level, step, max, muted, fixed, hdmiArc}`; `setVolume(0–1)`
  sets the nearest whole step without the system UI; `setMuted(bool)` returns whether the TV's mute
  now matches.
- **Changes from anywhere** (the remote's volume keys, which the shell never intercepts so Android
  shows its own volume UI; the TV's menu; CEC) arrive through the `VOLUME_CHANGED_ACTION` and
  `STREAM_MUTE_CHANGED_ACTION` broadcasts and a `Settings.System` observer. They are coalesced for
  60 ms and sent to the page as a `librarian-tv-volume` event, and once more after each page load.
- **Steps:** a step is `level = step / max`, and a level goes to the nearest step, so a round trip
  never drifts. A 0.05 nudge still moves one step with 15 steps.
- **In the page** (`bindTvVolume`):
  - The media element stays at volume 1 and unmuted, so the two levels never multiply.
  - The element's own `volume` and `muted` read and write the TV's, and a TV change fires
    `volumechange`, so the Glass video player's slider, its mute button and its keys follow the
    remote live.
  - The audio dock's `VolumeControl` uses the same store.
- **Fixed volume:** reported when `isVolumeFixed()` is true, or when a `setStreamVolume` did not
  change the stream (volume handed to a sound system). The shell then tries one `adjustStreamVolume`
  raise or lower with the system UI, which some TVs forward over HDMI-CEC. The page shows "Volume is
  controlled by your TV or sound system" instead of the slider, and mute falls back to muting the
  player when the TV will not mute.
- **Not yet measured:** whether the G07 reports fixed volume with a soundbar on ARC or CEC, and
  whether `adjustStreamVolume` reaches one. The event's `hdmiArc` flag shows whether an ARC or eARC
  output is present.

*The web side*, only when the user agent says `LibrarianTV`:
- TV mode is always on.
- The `data-tv-app` attribute is set.
- No service worker is registered, and any old one is retired. The page always comes from the
  server and cannot work offline anyway; `version.json` polling still offers updates.
- Route preloading on intent is off: every remote move would preload a page and its data.
- View transitions are off.
- Settings › About shows the app version, the server and *Change server*.

*Signing:*
- The release key is `/root/.config/librarian/android-tv-release.jks`, outside the repository,
  root-only, with its passwords in a root-only properties file.
- Gradle signs with it when it can read that file, and with the debug key otherwise.
- `versionCode` derives from `version.properties` (major × 10000 + minor × 100 + patch), so every
  release installs over the last. The certificate fingerprint is in `android-tv/README.md`.

### Q138: How does the TV app find the server?
**Answer:** The backend announces itself over mDNS as `_librarian._tcp` (`services/lan_advertise.rs`).
It reuses the `mdns-sd` crate that Cast discovery uses, with its own responder.

*The record:*
- The instance is "Librarian on <host name>".
- The host is `<host>.local.`, with addresses filled in and kept current by the responder (`addr_auto`).
- The SRV port is the port of the announced address.
- The TXT keys are `txtvers=1`, `url` (the origin to open), `name`, `version` and `path=/`.

*The announced address* is the `discovery.web_url` app setting when it is set. Otherwise it is
`http://<LAN address>:<port>`:
- the `HOST` bind address when that names one interface, else the default route's address;
- the backend's own port in a build with the embedded web app;
- 3000 in a development build, whose web app is the Vite server.

*The settings:*
- `discovery.web_url` and `discovery.advertise` are seeded from `LIBRARIAN_PUBLIC_WEB_URL` and
  `LIBRARIAN_LAN_ADVERTISE`. An environment value only seeds a row or replaces an untouched seed
  (Q77's rule).
- Settings › General › Network shows both to admins: *Announce on the local network* and *Address
  for TV apps*.
- The `AppSetting` write transform checks the values: an http(s) origin without credentials, query
  or fragment, stored as a JSON string of the origin, or `true`/`false`. A person's bad value is
  refused. A bad value from the environment is logged and stored as automatic, so a typo cannot stop
  startup.
- Saving either setting restarts the service through the `AppSetting` after-hook.

*On a site with `Secure` cookies*, the address must be the https one: an http page cannot keep the
sign-in cookie. The live site is one of these, so its *Address for TV apps* should be its public
https address.

*Failures* (no multicast, no LAN address) leave the service degraded with the reason in its health,
and never stop the server.

*On the TV:*
- The server screen browses with `NsdManager`, resolving one service at a time (older Android refuses
  parallel resolves), and holds a Wi-Fi multicast lock while it looks.
- It lists each server once per address. The announced `url` wins; without one, the SRV address and
  port are used.
- A typed address is checked before it is saved. A bare host is tried on 3001, then 3000, then plain
  http; any HTTP answer below 500 counts as reachable. *Use it anyway* saves an address that did not
  answer.
- The server is kept in SharedPreferences, and the sign-in in the WebView's cookie store.

### Q139: What does the app do differently on a budget TV?
**Answer:** The user's TV is a TCL G07: Android 11, 32-bit ARM only, 2 GB with about 390 MB
available, Chromium 156 WebView. Measured with the shell's predecessor (a plain WebView):
- the sign-in page, with its animated backdrop, at 342 MB PSS, 235 MB of it graphics;
- Home at 212 MB PSS, 96 MB of it graphics.
`isConstrainedDevice()` (web/src/lib/tv-app.ts) is true in the TV app and wherever
`navigator.deviceMemory <= 2`, and these follow from it.

- **Animated backgrounds:** these are Home's WebGL scene (`SceneBackdrop`: three, fiber and drei)
  and sign-in's film constellation (`ConstellationBackdrop`, a full-screen 2D canvas redrawn every
  frame at up to 2× density, with its thumbnails). The constellation, not WebGL, is what ran on the
  measured sign-in page.
  - Neither mounts, and neither lazy chunk loads, in the TV app or at 2 GB or less
    (`useAnimatedBackdrop`). The static `canvas-ambient` gradient stays.
  - The WebGL scene also skips reduced motion and screens under 640 px, as before. The constellation
    keeps its own still drawing for reduced motion and still runs on phones.
  - A device setting next to Blur, *Animated backgrounds*, turns both off anywhere. It is shown
    locked off in the TV app and on 2 GB devices. It is stored under
    `librarian.animatedBackgrounds.v1`, like `librarian.blur.v1`.
  - A test pins that neither renders for the TV user agent or for `deviceMemory = 2`.
- **Blur** defaults off there: an unset `librarian.blur.v1` follows `blurDefault()`, and a device's
  own "on" or "off" still wins.
- **Composited layers** (`styles/tv-app.css`, `[data-tv-app]`):
  - The page backdrop loses its `blur(4.5rem)` wash and its masks (each a full-screen offscreen
    pass), and its scale-in.
  - Focused tiles keep their scale and ring but drop the large shadow.
  - Row entrance animations are off.
  - The guide's sticky headers lose `backdrop-filter`.
  - Nothing in `src` sets `will-change`. The only other canvas is the constellation; the only WebGL
    is the Home scene.
  - Glass surfaces stop blurring with Blur off (`glassRendering: "none"`).
- **Artwork:** `sizedArtwork` asks for one image pixel per CSS pixel, at most 1280 wide, and gives
  unsized artwork (backdrops) the same cap.
- **hls.js:** 30 s ahead (60 s at most), 10 s behind and a 30 MB buffer, instead of 60 s ahead and
  90 s behind.
- **Routes:** no preloading on intent and no view transitions, which snapshot whole views as
  textures. The router keeps only the current route mounted.
- **The shell:** one WebView, no AndroidX, no native code, no splash bitmap. The renderer can be
  reclaimed in the background, and memory warnings reach the page.

The user's agent will measure again after sign-in, with a library grid up, which is where artwork
memory lands.

### Q140: How will the TV app play files natively (stage 2)?
**Answer:** Not built yet. The plan is a Media3 ExoPlayer in the shell, driven from the page over the
same bridge, so the web app keeps its UI, queue and progress, and the TV's decoders play what the
WebView cannot. That covers MKV, HEVC, HDR10/HLG/Dolby Vision where the panel has it, and AC3/EAC3/DTS
passthrough over HDMI. Media3 is pure Java (`media3-exoplayer`, `-exoplayer-hls`, `-ui`), so it runs on
the 32-bit TV.

*Choosing native:*
- The page asks `LibrarianTV.player.canPlay(json)`, passing the `/api/media/{id}/info` fields
  (container, codecs, HDR, resolution).
- The shell answers from `MediaCodecList` and `AudioCapabilities` (`{"direct":true,"passthrough":["ac3","eac3"]}`).
- When `direct` is true, the page plays `/api/media/{id}/stream` natively. Otherwise it keeps its
  current path (the HLS remux or transcode in the WebView, or natively, below).
- *Settings › About* gets a "Use the TV's player" switch, on by default, to compare.

*Bridge, page to shell:* a `window.LibrarianTV.player` object with JSON-string arguments, as
`addJavascriptInterface` passes only primitives.
- `load({url, startSeconds, title, subtitle, artwork, autoplay, subtitles:[{id, url, language, label, format:"vtt"}], audioStreamIndex})`:
  - `url` is same-origin and relative: `/api/media/{id}/stream` for direct play, or
    `/api/media/{id}/hls/playlist.m3u8[?audioStream=n]`.
  - The shell resolves it against the server.
  - It adds the WebView's cookies for that origin (`CookieManager.getCookie`) to every request
    through a `DefaultHttpDataSource.Factory` whose request properties are refreshed per request.
    The page keeps refreshing the access token while it is open, so a long film keeps working.
  - Direct play relies on the stream's byte ranges, which `stream_media` already serves.
- `play()`, `pause()`, `seek(seconds)`, `stop()` (releases the player and its surface).
- `setAudioTrack(id)` and `setSubtitleTrack(id | null)`:
  - the ids come from the `tracks` event;
  - embedded tracks are selected with `TrackSelectionParameters` overrides;
  - external WebVTT subtitles are `SubtitleConfiguration`s on the `MediaItem`.
- `setVolume(0..1)`, `setRate(r)`.

*Events, shell to page:* `window.dispatchEvent(new CustomEvent("librariantv:player", {detail}))`
from `evaluateJavascript`.
- `state`: `{state:"buffering"|"ready"|"ended"|"idle", playing}`.
- `time`: `{position, duration, buffered}`, every 1 s while playing and after every seek.
- `tracks`: `{audio:[{id, language, label, codec, channels, passthrough}], text:[{id, language, label, external}]}`.
- `error`: `{code, message, fatal}`, from the `PlaybackException` error code name.
- `closed`: the person left the native player with Back.

*The page:*
- A `NativeMediaElement` adapter in `features/player/` gives the subset of `HTMLMediaElement` that
  the Glass `VideoPlayer` controller and `useProgressSync` use: `currentTime`, `duration`, `paused`,
  `buffered`, `play()`, `pause()` and the `play`, `pause`, `timeupdate`, `ended`, `waiting`,
  `playing` and `error` events, built from the bridge events.
- `VideoPlayer` attaches it instead of `<video>` when `canPlay` says so, so progress sync,
  continue-watching, the queue's next episode and the session save all work unchanged.
- The page keeps drawing its controls over a transparent WebView: the shell puts Media3's
  `SurfaceView` behind the WebView and makes the WebView background transparent while native
  playback runs.
- The remote keys and the page's controls drive the bridge. Back closes the player through the page
  as today. Media buttons keep arriving as the standard key events of Q137, so the page's key
  handling is the same for both players.
- Volume needs nothing new: Media3 plays on `STREAM_MUSIC`, the stream `VolumeSync` already ties to
  the page's volume control (Q137). The player's own volume stays at 1.

*The server:* nothing new is needed for direct play.
- `/info` already gives codecs and HDR, and `/stream` serves ranges with cookie auth.
- Subtitles are `/api/media/{id}/subtitles/{sid}.vtt`; embedded ones are read from the file itself.
- Two later additions:
  - an `/info` field listing the file's embedded subtitle and audio streams with their stream
    indexes, so the page can show them before Media3 has parsed the file;
  - a capability-aware decision: the shell sends its `canPlay` answer as `?client=tv&decoders=…`, so
    the server remuxes only what the TV cannot decode instead of following the browser matrix.

### Q126: How is a title's artwork changed, and where do the choices come from?
**Answer:** Every picture is a *slot* (`services/artwork_choice`): a movie's poster, backdrop and logo, a show's poster and
backdrop, a season's poster, an episode's still, an album's cover, an artist's picture, an audiobook's cover and a
playlist's cover. *Change artwork…* is an action in each entity's action set (a season's from its tile's menu on the show
page) and an Artwork tab in its settings dialog (a show's lists its seasons too); both open one picker. The picker asks for
the slot's provider pictures when it opens (`artworkOptions`), and keeps a list for ten minutes in memory (a list with a
provider that failed is not kept): TMDB's posters, backdrops, logos and stills in every language with size and votes
(`tmdb::images`, movies always, shows, seasons and episodes when the show has a TMDB id); TVmaze's posters and backgrounds,
its season poster and episode still; Cover Art Archive for the release group and its first eight editions (official first),
front, back and the rest, labelled by side, year, country and MusicBrainz's note; for artists every Commons photo
Wikidata's `P18` names and Deezer's pictures for artists of exactly that name (there is no Fanart.tv key, so it is not
asked); for audiobooks the provider record's cover, the Open Library work's covers and the covers of Open Library's
editions found by title and author. Each option says which picture a refresh would choose (`isDefault`) and which is shown
now (`isCurrent`, by `same_image`, which treats one TMDB file at any size and Cover Art Archive over http or https as one
picture). `chooseArtwork` takes only a picture the list holds, downloads it through the artwork cache's guarded fetch
(allowed hosts, size and type checks), and writes the slot's field and its lock in one update. Previews in the grid are
the providers' own small copies, like other provider results not in a library. Changing artwork is for administrators,
as other metadata edits are; a playlist's owner changes its cover. A movie's logo has no field: only a choice fills it, so
its artwork cache row is the whole record, and the hero shows it in place of the title (`artworkSlots`).

### Q127: How do uploads work?
**Answer:** `PUT /api/artwork/{entity}/{id}/{kind}` with the picture as the body and its `image/*` type (which a
cross-site form cannot send), for whoever may change the slot. JPEG, PNG and WebP up to 20 MiB, at least 100 pixels on
each side and at most 100 megapixels decoded, are decoded and encoded again (JPEG at quality 90, or PNG when it has
transparency), which drops camera and location metadata and anything that only claims to be an image; wider than 3840
pixels is scaled down to that. The original goes into object storage as the slot's artwork cache row, replacing what was
there (its resized copies are dropped by the existing hook), and the sizes the app shows below its width (185 to 1920) are
made straight away. The slot's field becomes `upload:<first 16 hex of its SHA-256>`: a marker, never a URL, so nothing
downloads it and no provider URL can equal it. A playlist gains `coverUrl` for it, which members cannot write through
GraphQL; its identity shows the upload instead of the mosaic of its entries.

### Q128: How do artwork locks hold, and how are they undone?
**Answer:** Chosen and uploaded artwork is locked through the entity's `lockedFields` with the field's GraphQL name
(`posterUrl`, `backdropUrl`, `stillUrl`, `coverUrl`, `imageUrl`). Season, Episode, Album and Artist gained `locked_fields`
(additive JSON columns) for it. Two rules keep a lock without changing any provider code: the write transform
(`keep_locked_artwork`, for generated and repository updates alike) leaves a locked artwork field out of any update that
does not also write `lockedFields`, which is every refresh and backfill; and `ArtworkService::cache_image`, which every
refresh path calls, keeps the cached picture of a locked slot unless the requested source is the chosen one. Unlocking is
the entity's own generated update of `lockedFields` without the field: the picture stays until the next refresh replaces
it. *Revert to provider* (`revertArtwork`) unlocks and puts back the option marked as the refresh's choice now; with
nothing to go back to (a playlist, a logo, a provider without a picture) it empties the slot, and when a provider could
not be asked it changes nothing, so a choice is never lost to an outage. A metadata form saving its own locks keeps the
entity's current artwork locks (`withArtworkLocks`), so a picture chosen while the form was open is not unlocked by it.
Browsers kept a picture for a day under one address, so a changed picture would not show: the device that changed it adds
a revision to the address (`?v=`, kept for a week in local storage), and an address without one is reused for five
minutes and then checked again in the background (`stale-while-revalidate`), so other devices follow within one view.

### Q133: How is a device identified, and what can be done to it?
**Answer:** Each browser, installed app or TV keeps a stable id of its own (`librarian.device.v1` in local storage) and
checks in with it (`checkInDevice`) when the app starts, on sign-in, when it comes back to the foreground, when TV mode
changes and every ten minutes. It says what it can tell about itself: a suggested name ("Safari on iPad", "Chrome on
Windows", "webOS TV", or "TV" in TV mode), its form factor, platform, browser and app version. The server records what it
saw of the request: the user agent and the address (a forwarded address only from a configured trusted proxy), the time,
and the login session (`session_id`, private, never a GraphQL field). `Device` is one row per person and client id
(`unique (user_id, client_id)`), owner scope: members see their own, administrators everyone's. Clients cannot create
devices through the generated create, and the generated update changes the name only; renaming sets `nameIsCustom`, after
which check-ins keep the name. *Sign out* (`signOutDevice`, the owner or an administrator) revokes every live refresh token
of the device's login session in one transaction, ends its open playback sessions and marks it signed out; its access
tokens stop working at their next request because session authority is rechecked. *Forget* is the generated
`deleteDevice`, whose Before hook does the same and detaches its sessions; plays keep the device's name. A playback
session names its device (`deviceId`, an owned link, so another person's device is dropped), and starting one ends the
device's other open sessions. UI: `DeviceIdentity` (TV, phone, tablet or computer glyph; "This device", "Active now",
"Signed out", last seen), `useDeviceActions` and `DeviceSettingsDialog` (General, Details, Forget), listed in your
profile's Devices tab and, for administrators, under Settings › Statistics › Devices (the dashboard is gone, Q147).

### Q134: How is play history recorded?
**Answer:** From the existing progress pipeline, server-side. The player already updates its `PlaybackSession` every few
seconds (now also with its device, chapter, state, play method, quality, bitrate and `endedAt` on close; the server
replaces the client's timestamps with its own). The session's After Updated hook compares each report with the one before
and credits only time spent playing to a `PlayHistory` row in the same transaction: moving forward by about the wall time
between reports counts the smaller of the two (so 2x speech counts real minutes); a seek counts the wall time between the
reports, at most a minute, never the distance jumped; paused or stalled time counts nothing. Nothing is written until
something has played. A play continues while it is resumed within 30 minutes of its last activity on the same device:
pausing and resuming, or closing and reopening the same title, is one play; a longer gap starts a new one, even in the same
session. A play is complete by the watched rule (the session's `completedAt`, or 92% of the duration), and a finished play
is never continued: watching it again is a new play. Each play keeps title, parent (show, artist, author) and subtitle
snapshots and the content ids as plain references, not relations, so history outlives deleted titles and files. Plays are
read and deleted through the generated roots (owner and administrators); the generated create and update refuse public
callers. Clearing your history is `deletePlayHistories(where: {userId})`. Administrators set
`play_history.retention_days` (Settings › General › Playback, 0 keeps everything); a service prunes older plays every six
hours. Existing watch progress seeds history once at startup (`play_history.imported_at`) and on demand
(`importPlayHistory`): one completed play per watched checkpoint, with no time claimed, marked `imported` and keyed by
`sourceProgressId` so it never imports twice. Deleting a user removes their plays, devices and shares.

### Q135: What do statistics show, and how are they computed?
**Answer:** `playStatistics` is a computed read: after authorization it reads the period's plays through the typed entity
layer (at most 50,000; `complete` says when more were left out) and sums them in Rust (`play_history::stats`). Local days
follow the viewer's `utcOffsetMinutes`; a play counts on the day it started; weeks start on Monday. It returns time watched
(movies, episodes) and listened (tracks, audiobooks) per day and per week, by hour of day, totals (plays, finished,
imported, different titles, active days), the current and longest streak of consecutive days within the period, top shows,
movies, artists, albums, audiobooks, authors and genres (genres from the movie, show or album, ranked by time, then plays),
devices used, the 20 most recent plays without device addresses, and, in an administrator's everyone view, time per person.
Imported plays count as plays with no time. Your profile's Statistics tab (and Settings › Statistics for everyone, Q150) shows them with Glass charts: metrics with sparklines,
stacked columns per day (per week past three months), ranked bars and time of day, with a period choice (7, 30, 90 days, a
year) and a person choice. Settings › Statistics › Now playing lists open sessions that reported in the last three minutes
(members their own, administrators everyone's) with poster, title, person, device, progress, play method, quality and
state, live over the session subscription; the owner or an administrator can *Stop* one (`stopRequestedAt`), and the
player pauses as soon as its session changes.

### Q136: Who can see someone else's statistics?
**Answer:** Statistics are private by default. A person shares them with chosen people or with everyone on this server
(`StatsShare`: `audience` `user` with a `viewerUserId`, or `everyone`); the share is the owner's row, validated in the
create hook (an active other user, once) and never updated, only removed. `playStatistics(userId)` answers the owner, an
administrator, or a viewer granted by a share while both accounts are active; anyone else gets the same "not found or not
shared with you" as for an unknown user. Viewers read the computed statistics only: they cannot list the shares, the raw
`PlayHistory` rows or the devices. `statisticsPeople` lists whose statistics the caller can open and
`statisticsShareCandidates` the other active people to share with (names only). Sharing between servers is out of scope.

### Q110: What does Recommended offer for music?
**Answer:** An **artist's coverage plan**, built like a show's (Q101) and stored the same way (a `plan` row with `artistId`
and one row per release with `planId`). Opening an album, a track or the artist asks for the same lookup: the whole artist.
The missing tracks are every missing track of the artist's albums in the library (wanted only in the background).
Releases are gathered cheapest first: the feed catalogue for each album, a discography search (`<artist> discography`,
when two or more albums miss something), each album's search for albums still uncovered, and at most six single-track
searches (the hunt's own per-track query) for tracks still uncovered, stopping as soon as everything is covered or the
hourly budget (Q94) runs out. A release must name the artist (Q98) and pass the profile. Its name says what it claims
(`recommend_music::music_claim`): a **discography** (`Discography`, `Complete Studio Albums`, `Anthology`), one of the
artist's **albums**, a **single** named after a missing track, or **another release** (an EP, a compilation, a live
album). Up to six releases are looked into (Q99), discographies first, then albums, then other releases, then singles;
their files are judged track by track (`WantedItems::ArtistTracks`: in the album's own folder by title and then by a
unique number, else anywhere by title, since numbers repeat across albums). A discography or another release counts only
once its files are read; an album or a single counts by its name until then. `plan_cover` then picks quality first and,
within a tier, the release adding the most tracks, so a discography beats albums and an album beats its tracks; between
releases adding the same (one missing track on an album, a compilation and a single), the healthier and then the
**smallest** wins. Rules version 4 retires the per-album answers.

**Single tracks.** A track's release downloads only that track's files (and the release's own artwork and sheets) with
"Download only the missing files" on: exactly by index when it was looked into (`WantedFiles::exactly`), else by title. Its
row says so (`downloadNote`, with `selectedSize`): "Downloads 1 track (8 MB) from a 600 MB album", "Downloads only the 2
missing tracks from a 4.0 GB discography pack", or, with the setting off, "Downloads the whole 600 MB album for 1 missing
track". Grabbing a discography from a plan links the torrent to nothing narrower than its library; import identifies the
albums by their tags, as for any music download.

### Q111: Is a torrent read before it is grabbed read again at grab time?
**Answer:** Not when it is public. `inspect` keeps a public torrent's info dictionary (`infoBytes`) and, since this
answer, its tracker addresses (`TorrentContents.trackers`, public torrents only, and only addresses with nothing
secret-looking in them: no passkey in the path or query). Every grab (`grab_release_with`: the hunt, recommendations,
plans, feeds) first asks `inspect::stored_torrent`: if the release was read before, its `.torrent` is rebuilt from the kept
info dictionary and trackers (`torrent_file_from_info`; the info hash is checked against the row before it is used) and
added from those bytes, so no peer is asked for the metadata and no `.torrent` is fetched again. The `Torrent` row still
keeps a magnet for restoring: the release's own magnet when it had one, else one built from the info hash, the name and the
kept trackers (`magnet_for`). **Private torrents** keep no info dictionary and no trackers; they are fetched through their
source at grab time as before, so the passkey stays only where it already lived (the encrypted source credentials, feed
item link and recommendation link) and is never logged.

### Q112: Which subtitle files does a torrent bring, and how are they kept with their video?
**Answer:** Bundled `.srt`, `.ass`/`.ssa`, `.vtt`, `.sup` and VobSub `.idx`+`.sub` files are recognised by
`torrent::subtitles` when a torrent is inspected (and again for readings kept from before). Each is given its **video**
(`ContentFile.belongsTo`): by a shared name (`Video.en.srt`), by a `Subs/<video stem>/` folder (season packs:
`Subs/Show.S01E02.1080p/2_English.srt`), by the same episode, by a `Subs/` folder beside a single video
(`Movie/Subs/2_English.srt`), or, with one video in the torrent, that one; and a **language** (`ContentFile.language`)
from a code or name in its file name or folder (`2_English`, `.en.forced`, `Subs/ger/`). Whenever only some files are
downloaded (`judge` selections for inspected packs, exact `WantedFiles::exactly` indexes and the name-based fallback in
`partial`), a chosen video's subtitles come with it, and one half of a VobSub pair brings the other. Importing subtitles
into the library is not part of this; it belongs to the subtitles work.

### Q113: When does automatic downloading search for new episodes and movies, and how does it stay quiet?
**Answer:** The air-time pass (Q81) now looks in the owner's **feed catalogue first** and asks the indexers only when the
catalogue has nothing acceptable for that episode or movie, and only **inside `auto_download.hourly_search_budget`**,
shared with recommendations (a target the budget cannot cover is not searched and keeps its place for the next tick; its
backoff is not spent). Retries **back off**: every `air_retry_minutes` at first, then a quarter of the time since it aired,
up to the ordinary `search_backoff_minutes`, for `air_window_days` (`auto_schedule::backoff_at`), about thirty-five
searches over a week instead of one an hour. **Movies** now keep TMDB's earliest digital and physical release dates
(`Movie.digitalReleaseDate`, `Movie.physicalReleaseDate`, release types 4 and 5, any country); a wanted movie with either
is searched from the earlier of them like a new episode (grace, quick and growing retries, then the ordinary schedule), and
not before; without one it is searched from its release date on the ordinary schedule, as before. Everything else of the
hunt applies unchanged: modes, the profile, the blocklist, the cap, contents judging, the per-target backoff.

**Quiet.** A fresh episode or movie that has not turned up opens no "Waiting for …" notification in its first day
(`auto_trail::QUIET_AFTER_RELEASE`): releases often take that long. After that it opens one, which later searches only
rewrite. A notification that was opened and then dismissed or deleted is not opened again by automatic searches
(`auto_download.waiting_opened` remembers it for thirty days, and forgets it when the target is grabbed); a manual search
still opens one. `auto_download.enabled` stays off by default, and Settings › Downloads shows the server's value and says
that turning it on starts the hourly sweep and the searches after air time.

### Q114: Do bundled subtitles change which release is recommended?
**Answer:** Only as a tiebreak, never as a reason to turn a release down. A judged release reports the languages of the
subtitles that come with what it is taken for (`ContentsVerdict.subtitleLanguages`, English first), and its row lists
"Includes English subtitles" (or the languages found) among its reasons. In a coverage plan, between releases of the same
tier adding the same and in the same health band, one with subtitles goes before the smaller one; in the hunt, between two
inspected packs holding as much of what is missing, the one with subtitles is taken.

**Where recommendations show.** Only in Find a release: its button carries a count badge (Glass `CountBadge`, the count
also in the button's name) wherever it appears (a show and each season header, a movie, an album, an artist, an
audiobook), and the dialog's table lists the recommended releases as ordinary rows pinned at the top, with "Recommended"
in the profile column (its reasons on the badge's tooltip), what each adds in the quality column ("S02 · adds 6"), and a
release the live search also returned shown once. "Download recommended" in the table's toolbar downloads the whole plan
after a confirmation; what the plan leaves uncovered is a quiet line under the table.

### Q141: What can a TV remote stop on?
**Answer:** One definition, `isRemoteStop` in `web/src/lib/input-mode.tsx`, used by the arrow keys, a page's first focus
and focus restoration alike. A stop is something to act on: a card, a row, a control. Never a stop:
- **Hidden things:** anything hidden, `inert` or `aria-hidden`, and anything with no box (a collapsed panel keeps its width
  but has no height).
- **Disabled controls:** `disabled` or `aria-disabled="true"`, including Glass buttons with a `disabledReason`. A card that
  can't act right now but is worth reading (an episode still with no file) is not disabled: it says why instead.
- **Pointer-only shortcuts:** `data-nav-skip` always, `data-tv-skip` on a TV.
- **A data table's header:** sort buttons, column menus, resize handles and select-all (`.fa-dt-head`, `columnheader`,
  `separator`). On a TV the tables are given `DataTableRemoteContext` (an addition to our Glass patch), so the table's
  toolbar carries the same sort menu its card view has. Columns and density stay a pointer matter.
- **A field's own buttons that Glass keeps out of the tab order**, such as a number field's −/+ (OK adjusts instead, Q107).
- **Inside a table line:** a row or card of a Glass `DataTable` is one stop. Its checkbox, ⋯, expander and in-cell
  buttons are the pointer's; its actions open with the remote's Menu key, which is the table's context menu. A detail
  panel's own controls are stops.
- **A region that only contains controls:** focus lands on the controls.

**Tables.** The table moves between its lines with Up/Down, and across a grid of cards with Left/Right, but it clamps at
its edges. So the remote leaves it there instead (`leavesTable`): Up from the first row, Down from the last once
everything is loaded (not while it is busy), and Left/Right from the end of a row of cards. Left from a table row reaches
the rail.

**Composites.** A select's trigger and a horizontal slider let Up/Down move on in TV mode, as ⋯ menu buttons already did.
OK opens the select, and Left/Right change the slider. Walking down a settings page never opens or nudges what it passes.
A `role="grid"` marked `data-nav-group` (the guide's month of days) is walked spatially; other grids own their arrows.

**Reading.** When nothing lies further down but the page scrolls on (charts, a summary under the last control), Down
scrolls the page a step instead of doing nothing.

**Tests.** `web/src/test/remote.ts` lays a jsdom page out as a column (`layoutColumn`), lists its stops (`remoteStops`)
and walks it with the arrow keys (`walkRemote`). Pages that use it: Find a release, Statistics and General settings.

### Q142: What does the remote's Back do, and in what order?
**Answer:** Back arrives as `Escape` (a keyboard, or the Android TV app aiming it at an overlay), as `GoBack` or
`BrowserBack`, or only as key code 461 (webOS) or 10009 (Tizen). In TV mode, Backspace outside a text field is Back too.
`web/src/lib/remote-back.ts` has the one `isBackKey`. The first step that applies takes the press:
1. **The nearest thing that handles it itself:**
   - the scrub preview (Back puts the position back);
   - a number field that is being adjusted;
   - the video player;
   - the now playing drawer.
2. **The topmost overlay:** a menu, popover, select list, dialog or drawer. Dialogs and menus close on Escape themselves.
   Another Back key, or focus outside the overlay (the drawer, with focus on the page), is re-sent to it as Escape.
3. **An edit state:** a search field with text is cleared.
4. **One navigation step:** in-app history, else Home. On Home there is no step, so the key is left unhandled and the
   Android TV app exits (`handleBack` sends `GoBack` first and exits only if nothing took it).

**Tooltips.** A tooltip shown on a focused icon button closes on Escape and lets it carry on. It still prevents the
default (Glass's tooltip host on the capture phase, then Base UI's dismiss). So every "already handled?" check that read
`defaultPrevented` ignored Back while a tooltip showed. `backHandled(event)` replaces those checks. A watcher on the
window's capture phase records who prevents a Back key's default. Document-level listeners are dismiss layers and don't
count; a control, page or player does. The checks in the shell, the player, the dock and the drawer use it.

### Q143: Where does focus go when something closes, or after going back?
**Answer:** Focus never rests on the page body in TV mode.
- **Dialogs and menus** return focus to their trigger (Base UI). If that fails, for example because a menu item opened
  the dialog, or because the action removed its opener, the shell steps in. While an overlay holds focus, and when focus
  drops with no successor, the shell looks every 100 ms. Once the overlay is gone and focus is on the body, it moves
  focus back, in this order:
  - the last control focused outside any overlay (its table row, when that control was inside one);
  - the last content control;
  - the page's `data-tv-autofocus`;
  - its first stop.
- **The now playing drawer** returns focus to the control that opened it.
- **Navigating back:** route focus remembers the last control used on each page by its link, table row id, label, id or
  button text, and restores it when the page opens again. While the page loads it waits up to 1.5 s for that control
  before it falls back to the page's start. This includes closing the player, which is a route.
- **Row controls:** when focus is handed back to one of a row's own controls (its ⋯, after its menu closes), focus moves
  to the row.

### Q144: What happens when keyboard focus and mouse hover are on different items?

Only one item is highlighted: whichever input was used last owns it (`web/src/lib/pointer-focus.ts`).

- **Tracking the last input:** `data-last-input` on `<html>` becomes `keys` after a navigation key
  (arrows, Tab, Enter, Back, media keys) and `pointer` after the mouse really moves.
- **Pointer last:** a keyboard focus ring left on another tile hides.
- **Keys last:** hover stops lifting tiles, so a resting cursor can't leave a second highlight.
- **TV mode:** hovering a tile focuses it, so the tile under the mouse and the focused tile are the
  same, and OK opens what the mouse is on. Tiles that slide under a resting cursor while the remote
  scrolls a row don't take focus; that needs a real mouse move after the last key. The focused tile
  is drawn over its neighbours, so a lifted ring never collides with the next tile's edge.
- **Persistent selection** (a chosen tile, ticked rows) is a separate state from focus and is marked
  inside the tile, so it can sit next to the focused tile.

### Q145: How is a TV show's folder named, and which show folders exist?

**Answer:** One function names it, a folder exists only while it holds something, and empty folders are tidied.

- **One name:** `apply_tv_naming_pattern` (`backend/src/services/library_scan.rs`) is the only place an episode's path
  is made. Organizing, imports and the scan's misplaced-file check all reach it through `TargetFacts::Episode`, so they
  can't disagree about a show's folder. There used to be a second spelling: the scan's folder step built
  `"{name} ({year})"` itself, while organize used the pattern's plain `{show}`. That left an empty `Fallout (2024)/`
  next to the real `Fallout/`.
- **`{year}` in TV patterns:** `{show}` is the plain sanitised name and `{year}` is the show's year. A show without a
  year (or with year 0) loses the token, any brackets around it, and the separator before it. So `{show} ({year})`
  becomes `{show}`, never `{show} ()`. Between two other parts one separator stays: `{show} - {year} - S01E01` becomes
  `{show} - S01E01`. A folder named only by `{year}` is left out. A name that already ends in ` (Year)` isn't given
  the year twice.
- **The default:** `{show} ({year})/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}`, which is
  the user's choice. At startup, every TV library whose stored pattern is exactly the previous default
  (`{show}/Season {season:02}/…`), and the built-in "Standard" row if it still has it, moves to the new default once.
  This goes through the generated update mutations and is logged. Any other pattern, including a person's own pattern
  with the same text, is never changed. The episodes then move on the library's next full scan, through the existing
  check for misplaced files. On a share that is a no-clobber rename, with the media file's path, relative path and
  subtitles updated.
- **No folders ahead of files:** the scan no longer creates `Show (Year)/Season NN` for every show and season it knows
  about. A show's folders are made when its first episode is imported or organized into them.
- **Tidying:** for TV, only the library root is protected; a show or season folder is kept only by what is in it.
  After a full scan with organize on, `cleanup_empty_folders` removes every empty folder tree through the library's
  storage, so local folders and `smb://` shares are both covered. (It used to check `Path::exists`, which is always
  false for a share, so shares were never tidied.) It never removes:
  - the root;
  - a hidden folder (`.librarian-trash`, sync markers);
  - a folder holding any file, including artwork, `.nfo` files and hidden files, because `remove_dir` refuses a
    non-empty folder.

  It removes nothing at all when the storage can't be reached, or when any folder in the walk couldn't be listed.
- **Movies and music** are unchanged: their patterns already carry `{year}`, and their protected folders work as before.

### Q151: How does the server learn about new releases, and who is told?

**Answer:** A background check of GitHub's release list, every 6 hours and on "Check now", tells every admin once
per version (`backend/src/services/updates`).

- **The request.** `GET https://api.github.com/repos/<repo>/releases?per_page=30`, through the project's outbound
  client (User-Agent `Librarian/<version>`, nothing personal), with `If-None-Match`, so an unchanged list is a free
  `304`. `403`/`429` with `x-ratelimit-remaining: 0` or `retry-after` set a time before which nothing is asked
  again; "Check now" then says when. The first check runs 2 minutes after startup. `<repo>` is
  `LIBRARIAN_UPDATE_REPOSITORY`, default `Dastari/librarian`.
- **Channels** (`updates.channel`): **Stable** takes plain `v*` tags that GitHub does not flag as pre-releases;
  **Dev** takes every `v*` tag. Unset, the channel follows the running version: Dev on a pre-release, else
  Stable. Versions compare by semver (`0.6.0-dev.1 < 0.6.0-dev.2 < 0.6.0-rc.1 < 0.6.0 < 0.6.1-dev.1`). Drafts and
  tags that are not versions are ignored; `tv-v*` tags are the Android TV app.
- **The running version.** The release workflow builds with `LIBRARIAN_RELEASE_TAG` set, so a `v0.6.1-dev.2`
  binary reports `0.6.1-dev.2`; any other build reports the Cargo version. Binaries built before this change
  (v0.6.0-dev.1 and older) report the Cargo version.
- **The record.** `updates.state` holds the last check, its error, the ETag, the rate-limit wait, the newest
  stable, newest overall and newest TV release, and the versions already announced. `updates.auto_check`
  (default on) turns the background check off.
- **The notification.** Category and action type `server_update`, to every active admin, once per version:
  the announced versions are remembered, so dismissing or deleting it does not bring it back. `actionData` is
  `{version, tag, url, canUpgrade}`; Activity offers "Release notes" and, when this install can upgrade
  itself, "Upgrade to …", which opens Settings › Updates. An open one is resolved as `superseded` when a newer
  version is announced and as `installed` once the server runs that version.
- **Settings › Updates** (admins): version, build kind (release, debug, Docker, dev, source) and platform; the
  channel and the automatic check; the last and next check; the newest release on the channel with its notes;
  "Check now" and "Upgrade to vX"; the upgrade history; the newest TV app. Release notes are Markdown from
  GitHub, drawn by Glass's `RichTextReader`, which ignores raw HTML, sanitises links and here loads no images.
- **The TV app.** `tvAppRelease` (any signed-in user) returns the newest `tv-v*` version and its APK link, for
  the shell to offer. The shell does not use it yet.

### Q152: How does the server upgrade itself, and what happens when the new version fails?

**Answer:** An admin confirms "Upgrade to vX". The server downloads and verifies the release, takes a verified
backup, swaps its binary, and restarts into it in place. A new binary that doesn't get ready is swapped back by
itself.

- **Who and where.** Admins only (`upgradeServer(version)`, which must name the newest release on the channel),
  on release builds for linux-x86_64 and windows-x86_64. Refused, with what to do instead:
  - in Docker (`/.dockerenv`, `/run/.containerenv`, or the image's `LIBRARIAN_IN_DOCKER`): pull the new image;
  - on debug builds, with `LIBRARIAN_DEV`, or on a binary without the embedded web app or run from a cargo
    `target/` folder: pull and rebuild;
  - with the terminal dashboard;
  - when the folder holding the binary isn't writable.

  An LXC system container counts as a host.
- **Integrity.** The release workflow publishes `SHA256SUMS` (`sha256sum` format, one line per uploaded file,
  by base name). The server downloads `SHA256SUMS` and `librarian-<version>-linux-x86_64.tar.gz` (or the
  Windows zip) from `https://github.com/<repo>/releases/download/…` only. Redirects are followed only to
  `github.com`, `api.github.com`, `objects.githubusercontent.com` and `release-assets.githubusercontent.com`,
  over HTTPS, five at most. The archive must be the size the API lists, at most 400 MB. A missing entry or a
  wrong checksum refuses the upgrade. Releases published before this change have no `SHA256SUMS`, so the page
  says to install them by hand. The checksums guard against corruption and tampering in transit, not against
  a compromised release; a signature (minisign) would need a key held in the repository's secrets.
- **Before anything changes.** Only the one binary is taken from the archive (a regular file at its top
  level). It is run with `--version` alone, which prints `librarian <version>` and touches nothing, and must
  report the expected version. This catches a build that cannot load on this system. Then a full backup is taken
  with the backup service and verified. If the backup fails, the upgrade stops.
- **The swap**, beside the real file the process runs from (symlinks resolved, so `current -> v0.6.0/` writes
  into `v0.6.0/`):
  - On Unix, the current binary is hard-linked as `<exe>.previous`, then `<exe>.new` is renamed over `<exe>`.
    The path always holds a whole binary.
  - On Windows, the running `.exe` is renamed aside first, because Windows can't replace a running file.

  `<exe>.upgrade.json` (the marker: from, to, who, when, attempts, backup id) is written before the swap.
- **The restart.** The server shuts down as for SIGTERM, then on Unix re-executes `<exe>` in place: same PID,
  arguments and environment. So it works under any systemd `Restart=` setting, including the live unit's
  `Restart=on-failure`, and when run by hand. If exec fails, or `LIBRARIAN_UPGRADE_RESTART=exit` is set, it
  exits with 75 (`EX_TEMPFAIL`), which `Restart=on-failure` or `always` restarts. Windows has no exec: it always
  exits with 75, and the portable server must run under a service manager that restarts it (NSSM, WinSW, or a
  scheduled task set to restart on failure).
- **Started ok, or rolled back.**
  - The new binary's first start counts itself in the marker (attempts 1), before it reads its configuration.
  - When its HTTP server is listening, that start is "started ok": the marker is deleted, the history records
    `upgraded`, and the admins are told.
  - If a start finds the marker already at attempts 1, the previous start never got ready: it crashed, or it
    exited on an error and systemd restarted it. That start puts `.previous` back, keeps the new binary as
    `<exe>.failed`, marks the marker `rolled_back` and re-executes the old binary.
  - A start that hangs is rolled back the same way by a watchdog after 5 minutes.
  - The old binary records `rolled_back` in the history and tells the admins.
- **Migrations** run on startup as always. Only additive schema changes are allowed (the startup migration
  refuses others), so the old binary runs against a database that the new one migrated. The verified backup
  from before the swap is there for anything else.
- **The record.** `updates.history` keeps the last 20 attempts: from, to, when, who (username), outcome
  (`upgraded`, `rolled_back`, `failed`), detail and the backup snapshot id. Progress (downloading, verifying,
  backing up, installing, restarting, failed) lives in memory; the page polls it every 2 seconds while an
  upgrade runs.

### Q146: How does Librarian watch free disk space, and warn when a drive or share is low?

One reading per volume, kept in a small `StorageVolume` entity that libraries point at, refreshed by
a light job and turned into one alert per volume and level.

- **Reading.** `Storage::capacity` in `services/library_storage`: `statvfs` for a folder on this
  machine, SMB2 QUERY_INFO `FileFsFullSizeInformation` (smb2's `fs_info`) for a share. Both report
  what the caller may still write (`f_bavail`, caller-available allocation units), so quotas and
  reserved blocks count; the volume-wide free figure is kept beside it. A folder that does not exist
  yet is measured at its nearest existing parent. A failed or slow reading (20 s) marks the volume
  `unavailable`; it never fails a scan, an import or a move.
- **One reading per volume.** `Storage::volume_key` names a volume: the share root
  (`smb://10.0.1.190/media`, worked out without connecting, so a share that is down still groups) or
  `local:<device id>`. Every library folder, the torrent download folder and Librarian's data and
  cache folders (`DATABASE_PATH`'s folder, `CACHE_PATH`) are grouped by key and each volume is read
  once. All four live libraries sit on one share, so they share one row.
- **Why an entity, not Library columns.** The download folder and the data folder have no library
  row, alerts are per volume rather than per library, and four libraries on one share would mean
  four identical rows rewritten each time. `StorageVolume` (`storage_volumes`: key, kind, label,
  status, level, total/available/free bytes, `holdsDownloads`, `holdsAppData`, library names,
  `checkedAt`) holds the reading; `Library.storageVolumeId` points at it. Both are additive. The
  entity is admin-read and has no generated mutation roots: only the job writes it.
- **Keeping it current, quietly.** `jobs::storage_capacity` runs beside the source-health loop in
  the auto-download service (whether or not automatic downloading is on): every 12 minutes, and 30
  seconds after a nudge. The entity layer nudges it when a media file is made, removed or moved, a
  library is made, removed or moved, or the download folder setting changes; the job's own
  `storageVolumeId` writes do not. A row is written only when the available space moves by more
  than 0.5 % of the volume, the level or status changes, what lives on it changes, or the stored
  reading is a day old, so entity subscriptions stay quiet. The Libraries table updates through
  the `StorageVolume` change subscription.
- **Levels.** Settings › General › Storage: warning below 10 % or 50 GB free, whichever is larger;
  critical below 5 % or 20 GB (`storage.warning_free_fraction`, `storage.warning_free_bytes`,
  `storage.critical_free_fraction`, `storage.critical_free_bytes`; critical is never looser than
  warning). A level is left only when the space is back above its threshold by a tenth of the
  threshold, at least 1 GB (`services::storage_capacity::level_for`), so a volume on the line does
  not flap. An unreadable volume keeps its level.
- **Alerts.** One notification (category `storage_space`, `actionData` `<volume id>:<level>`) per
  volume and level, for the admins and the owners of the libraries on it, naming the libraries and
  whether the download folder or Librarian's own data is there. Nothing is raised twice; going
  critical resolves the warning (`escalated`) and raises a critical one; recovering resolves it
  (`recovered`), coming down from critical to warning resolves the critical one (`eased`). A volume
  nothing lives on any more is removed and its alerts resolved. Unreachable shares are left to the
  folder watch and the scan, which already say so.
- **Libraries table.** A Free space column with Glass `MeterCell` (`tone="data"`): used against
  total, "1.2 TB free of 8 TB", warning and danger following the server's level (the thresholds
  move only inside the hysteresis band so the colour never disagrees with the alert). A share that
  cannot be read shows "Unavailable", never 0. Settings › Downloads shows the same meter for the
  download folder's volume.
- **Grabs onto a full disk.** When the download folder's volume is critical, a release whose size
  is at least the available space is refused in `grab_release_with` with a clear reason ("Not
  enough disk space: the download folder has 10 GB free and this release is 30 GB …"), which covers
  the hunt, feeds and "Download recommended". The hunt checks first and records the candidate as
  waiting rather than as a failed grab. A release of unknown size is let through, and a partial
  pack is judged by its whole size (conservative). Only the stored reading is consulted, so a grab
  never waits on a share.

### Q147: Where is a person's profile, and how does two-factor sign-in work?

**The profile is the User entity's settings dialog** (`UserSettingsDialog`), not a page. It opens from the account
menu (Profile, Statistics; in TV mode the menu sits at the foot of the rail), from the command palette, from a
person's identity wherever it appears (`UserIdentity`, `useUserActions`), and from any address with `?profile=<tab>`,
so Back closes it and old links can open a tab. Tabs: Profile (photo, display name, email, phone, account facts),
Security (password, two-factor sign-in), Devices, Preferences (genres, Q149), Statistics and History. An administrator
opening someone else's dialog from Settings › Users sees their profile, account recovery (Q148), devices, statistics
and history. The `/dashboard` route is gone: its personal views are profile tabs and its everyone views are Settings ›
Statistics (Q150); `/dashboard…` addresses redirect there (`dashboardRedirect`).

**Who edits what.** `User` is now owner-scoped like other personal rows: a member's `users` and `user` reads see only
their own row (row policy on `id`), and the generated `updateUser` edits it. Username, role, sign-in access and the
photo marker need an administrator (`admin.write` fields); the security facts (`mustChangePassword`,
`twoFactorEnabledAt`, `recoveryCodesRemaining`, `passwordChangedAt`, `lastLoginAt`) have no GraphQL input at all and are
written only by the auth actions. Only administrators create or delete accounts through the generated roots (the
mutation hook refuses everyone else). Display name, email and phone are trimmed and checked in the write rules, and an
email can belong to one account only, since sign-in accepts it. The photo is an artwork slot (`USER`/`AVATAR`): it is
uploaded through the artwork route (Q127: decoded, re-encoded, size-checked), served from `/api/artwork/user/{id}/avatar`
to its owner and administrators, and removed with `revertArtwork`. There is no notification email preference: nothing
sends mail yet.

**Two-factor sign-in (TOTP).** `agql-auth` already provides secret generation, `otpauth://` provisioning and
replay-safe verification with its own per-principal abuse limits, so Librarian uses those (no `totp-rs`). Enrolling:
`beginTwoFactorSetup` (needs the password) makes a 20-byte secret, stores it encrypted with the credential key
(`CredentialEncryption` envelope, like source credentials) as pending, and returns the key and a server-drawn QR code
(SVG, shown as an image). `confirmTwoFactorSetup` with a code from the app moves it to the confirmed secret and returns
ten recovery codes once: 10 characters from a 31-letter alphabet without look-alikes (about 50 bits), stored only as
SHA-256 of the account id and the code, each spent once under a lock. `regenerateRecoveryCodes` and
`disableTwoFactor` need the password. TOTP secrets are encrypted with the same key as source credentials; rotating
that key (`rotate_source_credentials`) does not yet re-encrypt them, so rotation turns two-factor sign-in off in effect
until people enrol again.

**Signing in in steps.** `login` checks the password; an account with a confirmed secret or a temporary password gets
`challenge { token, stage, expiresIn }` instead of a session. The token is 32 random bytes, held in memory only as its
SHA-256, for five minutes and five wrong codes; it is not a JWT, sets no cookie and is accepted only by
`completeTwoFactorSignIn` (a TOTP code or a recovery code) and `completePasswordChange`. A password change or a
disabled account meanwhile voids it. Only the last step issues a session (a host grant, refreshed with the account's
current role). Every wrong password or code counts in the existing login rate limiter under what was typed to sign in
(10 per 15 minutes), on top of `agql-auth`'s TOTP limiter; wrong current passwords in the profile count under the
account. Accounts without a second step sign in exactly as before.

### Q148: How are passwords reset without email?

- **Change your own:** `changePassword` with the current one. The password-hash change revokes every session of the
  account (the existing User update hook), so this device gets a new session in the answer.
- **Temporary password:** an administrator's `setTemporaryPassword` sets it and `mustChangePassword`; the person's
  sessions end, and their next sign-in (after two-factor sign-in, when on) asks for a new password before any session
  exists.
- **Reset link:** `createPasswordResetLink` (administrators) issues `agql-auth`'s signed reset token for a day and records
  its id as a `PasswordResetToken` row (new admin-read entity, generated mutations closed); an earlier link for the
  account is deleted. The link (`/reset-password?token=…`) is shown to the administrator to pass on and is never
  stored or logged. `resetPasswordWithLink` consumes the id with a conditional update (unused and unexpired), so it
  works once; it sets the password, clears `mustChangePassword` and ends every session. Two-factor sign-in still
  applies afterwards.
- **Recovery of two-factor sign-in:** an administrator's `resetUserTwoFactor` clears the secret and codes and logs who
  did it to whom (warning level). Temporary passwords and reset links are logged the same way, without the secret.
- **Mail later:** the reset flow already has the shape a mailer needs. An SMTP setting (host, port, sender, password
  stored like other secrets) and a `send_reset_link(user, link)` step after `createPasswordResetLink` would let people
  ask for their own link from the sign-in page (`LoginFlow` has the views); `agql-auth`'s
  `should_process_password_reset_request` gives the silent, rate-limited answer that needs. Not built.

### Q149: How do genre interests change Home?

`User.genreInterests` (a JSON list, at most twelve, trimmed and without repeats) is picked in the profile's Preferences
tab from `genreChoices`: every genre of the movies and shows the person can see, with counts. `genreInterestShelves`
returns one shelf per interest, in the person's order, with the newest movies and shows of that genre they can see;
Home shows them as "Because you like Sci-Fi" rows under its new `interests` shelf (right after Continue watching by
default, movable and hideable in Customize home like the others). Genres with nothing to show are left out. Nothing
else is re-ranked. There were no per-person preferences in Settings › General to move: theme and TV mode are per
device and the subtitle languages are the server's.

### Q150: Where are everyone's statistics, and how will plays from shared libraries fit?

Settings › Statistics (administrators; others are sent to their profile) has Now playing for everyone with Stop,
Statistics for everyone or anyone, People (each person's watched and listened time, plays, finished and last play for the
period; a row opens their statistics), History (everyone's plays: who, what, when, on which device, how and for how long)
and Devices (everyone's). The tables are bounded and frameless; History pages on the server.

`PlayHistory` gains `origin` (null for this server) and `originName`. When libraries can be shared between servers, a
play of a remote library records the server it came from; until then every play is local. `playStatistics` returns
`origins` (this server first, then others, with time and plays) and takes `origin` (`local` or an origin's id) to
narrow everything. The statistics origin choice and the history's Server column and filter appear only when more than
one origin exists, so nothing changes on a server without sharing.

### Q154: Why is transcoded audio stereo?

On-demand transcodes and full re-encodes mix audio down to stereo (`aformat=channel_layouts=stereo|mono`,
ffmpeg's standard downmix). Until 0.6.0-dev.1 they kept 5.1. The user's budget Android TV (TCL G07, WebView)
then played Fallout's E-AC-3 5.1 track with music and effects but no dialogue. It kept only front left and
right of the 5.1 AAC it was sent and lost the centre channel, which carries almost all speech. A stereo mix
folds the centre and the surrounds into both sides, so every player gets the dialogue.

Direct play is unchanged: a player that can decode a file's own surround track still gets it. Surround over
a transcode, and passthrough to a soundbar, belong with the native Android player (Q140, stage 2). There,
the device can say what it can output and the server can choose to keep 5.1.

### Q155: Why is nothing over artwork frosted, and how does the page backdrop stay cheap?

Measured on the user's GT 710 driving an ultrawide, a library page with a full-bleed backdrop scrolled sluggishly, worst in its first seconds. The cost was GPU work redone every scroll frame:
- the backdrop layer's opacity and transform came from a CSS variable set on every frame, which its subtree inherited, so frames went through style and paint;
- the backdrop stacked full-screen masks;
- each poster badge, poster action, card corner and hero button ran its own backdrop blur over moving artwork;
- the blurred rail re-blurred the content scrolling under it.

Now:
- **Backdrop layer:** `.shell-art` is its own GPU layer (`contain: strict; will-change: opacity, transform`). AppShell sets its opacity and transform directly, so a frame is compositing only.
- **Masks:** the backdrop image drops its own mask; the layer's feather already shapes it.
- **Opening zoom:** shortened to the fade's length, so it ends before the first scroll.
- **Over artwork:** nothing is frosted (`.over-imagery { --_frost: none }`). Controls there take a solid canvas tint.
- **Rail and top bar:** they keep their frosting. A pause-while-scrolling toggle (Glass `data-moving`) was tried and dropped. The strips are thin, so their blur is cheap, and switching the material at the start and end of each scroll risked a hitch and a visible pop.

Second pass, the same kind of cost elsewhere:
- **Colour wash:** the wash behind media pages was a 320 px image stretched full-screen under a 4.5rem `filter: blur` and its own mask. It is now drawn once into a canvas 48 px wide, lightly blurred, with its fade baked in, and the browser stretches the canvas. No filter, no mask, same look.
- **Guide headings:** the sticky day and service cells were frosted, so each re-blurred its patch on every scroll frame. They are now an opaque band a shade off the canvas, on every device.
- **Poster shadows:** a tile's hover or focus shadow is drawn once on `.poster-card::before`, shaped by `--poster-ratio`, and only faded and scaled. Animating the art's `box-shadow` repainted it on every frame.
- **Home's 3D scene:** renders on demand at 30 fps, only while on screen and the tab is visible. It runs at 1× density without antialiasing, and its panes use a plain translucent material, because a physical "transmission" material renders the scene a second time. A software renderer, or a GPU that manages under 60% of the frames in its first seconds, falls back to the gradient for the rest of the tab.
- **Sign-in constellation:** 1× density and 30 fps; hidden tabs already get no animation frames.
- **Poster rows:** a row checks its scroll arrows once per frame, not once per scroll event.

### Q156: How are cast receivers named, and what does "How it was found" show?
**Answer:** By what each receiver says about itself, read at discovery (`services/cast/describe.rs`).
- **Google Cast (mDNS `_googlecast._tcp`):** the TXT record's `fn` is the name and `md` the model; without `fn` the
  service instance is tidied (`Chromecast-Ultra-<id>` → "Chromecast Ultra"). Anything answering on that service is a
  Cast receiver, whatever its model says (TVs with Chromecast built in report their own model), so it is
  `CHROMECAST` and can play, or `CHROMECAST_AUDIO` when its `ca` bitmask has audio out but no video out. One row per
  receiver: its IPv4 address when it has one.
- **DLNA (SSDP `MediaRenderer`):** the reply's `LOCATION` (already pinned to the replying device) is fetched, at most
  256 KB within 3 s and without redirects, and the root `<device>` gives `friendlyName`, `manufacturer`, `modelName`,
  `UDN` and the service list. Samsung's bracketed category is dropped ("[TV] Samsung 8 Series (55)" → "Samsung 8
  Series (55)"). A description that can't be read leaves the renderer listed as "DLNA renderer (<address>)" with
  the reason kept. The raw `USN` is never a name.
- **Every network:** the SSDP search goes out of each private IPv4 interface (one socket each, `IP_MULTICAST_IF`),
  skipping container bridges. A server on two subnets used to search only its default route's, which is why a
  Samsung on the other subnet went unseen. mDNS already listens on every interface.
- **Stored:** `manufacturer`, `receiverId` (the Cast `id` or the UDN), `nameIsCustom` and `discoveryDetails` (JSON:
  protocol, the name as reported, TXT record or description URL, `SERVER`, `USN`, services, capabilities). All are
  nullable, so the migration is additive.
- **Upserts:** found by address and port, then by `receiverId`, so a receiver whose address changed keeps its row,
  name, favourite and sessions. A name an administrator chose (`nameIsCustom`) is kept; a reply that says less never
  erases what is known. Renaming back to the reported name clears `nameIsCustom`.
- **UI:** a `CastDevice` identity (TV, speaker or cast glyph; Available / Not responding / Turned off), actions
  (How it was found, settings, rename, favourite, turn off, copy address, remove) and a settings dialog with
  General, How it was found and Remove tabs. The cast picker lists playable receivers first, favourites on top, and
  folds the rest under "Can't play from Librarian" with the reason.

**Casting from another network** (the question behind this): Librarian casts from the server. The server opens the
Cast control channel to the receiver's LAN address, and the receiver fetches media from the server, so a receiver
must be on (or routed to) the server's network. A cloud-hosted server cannot reach a TV behind a home router. Ways
to cover that, none built yet: (1) the browser casts: the Google Cast Web Sender SDK in Chrome discovers and controls
the TV from the viewer's own network, and the receiver streams from the server's public HTTPS address with a grant
in the URL (Chrome on desktop and Android only); (2) "Play on" a Librarian app: the TV app (or any signed-in
browser) holds a live connection to the server, so a phone anywhere can tell it what to play, as Plex and Spotify
Connect do; (3) a small helper on the home network that relays discovery and control for DLNA, which browsers
cannot speak.
