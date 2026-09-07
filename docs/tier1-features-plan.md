# Tier-1 Feature Implementation Plan

Plan for closing the four Tier-1 gaps identified in `docs/audit-2026-07.md` §6: **auto-download loop**, **quality profiles + upgrade automation**, **Torznab/Newznab client source**, **HLS/FFmpeg transcoding**. Written against the codebase as of 2026-07-05.

**Mandatory reading before touching code**, per `docs/design.md` and `AGENTS.md`:
- `docs/design.md` — Media Pipeline Decision Guide (Q1–Q51), Quality Profiles section, Acquisition/Sources section
- `.cursor/rules/entity-single-source-of-truth.mdc` — no raw SQL for domain entities; everything through `graphql_entity` macro CRUD or GraphQL-mutation-driven internal calls
- `.cursor/rules/graphql-naming-convention.mdc` — camelCase resolver fields, `#[graphql(name = "...")]` on every new field
- `.cursor/rules/media-pipeline.mdc` — references `src/jobs/download_monitor.rs`, `src/jobs/auto_download.rs`, `src/services/quality_evaluator.rs`, `src/services/torrent_completion_handler.rs`. **RESOLVED for the first two** as of the phase-A auto-download implementation: `backend/src/jobs/download_monitor.rs` and `backend/src/jobs/auto_download.rs` (plus a new `backend/src/jobs/scoring.rs`) now exist, so the rule pack's globs match real files. `quality_evaluator.rs`/`torrent_completion_handler.rs` remain aspirational, pending §2 (Quality Profiles).

Schema is macro-driven: **all new columns/tables are `#[graphql_entity]` struct fields**, not `.sql` migrations. There is no `db/schema_sync.rs` in this repo (it lives inside the external `graphql-orm` crate pinned in `Cargo.toml:58`) — `AGENTS.md`'s reference to it is stale; do not create it.

---

## 0. Cross-cutting groundwork (read first, applies to all four features)

### 0.1 Service lifecycle pattern (from `backend/src/services/manager.rs`)
Every long-running feature is a `Service` impl registered via `ServicesManagerBuilder`:
- Define a `FooServiceConfig` struct + `impl IntoServiceRegistration for FooServiceConfig` (pattern at `backend/src/services/manager.rs:170-234`)
- Add a `ServiceRegistrationKind::Foo(FooServiceConfig)` arm, instantiate in `ServicesManagerBuilder::build()` (`manager.rs:297-373`)
- Add `foo: RwLock<Option<Arc<FooService>>>` + `register_foo`/`get_foo` accessors to `ServicesManager` (mirrors `library_scan`/`sources` at `manager.rs:390-406`, `732-755`)
- Wire into `backend/src/main.rs:136-172` `.add_service(FooServiceConfig{..})` in dependency order
- Background loops follow the torrent service's cancellation-token pattern: spawn a `tokio::task` in `start()`, store the `JoinHandle` + a `CancellationToken` in an internal `Runtime`/`Inner` struct behind `RwLock<Option<...>>`, await-join in `stop()`. See `backend/src/services/torrent/service.rs:672-721` (`completion_handle`, `reconcile_handle`) as the concrete template for a periodic reconciliation/retry loop — this is the shape the auto-download monitor and the HLS session-reaper should copy.
- GraphQL access pattern for a service to run mutations internally: build an `async_graphql::Request`, execute against the schema obtained via `self.manager.get_graphql().await?.schema().await?`, exactly as `LibraryScanService::execute_graphql` does at `backend/src/services/library_scan.rs:333-377`. Reuse this rather than inventing a second internal-GraphQL-call helper.

### 0.2 Entity/schema convention
New entities: copy the shape of `backend/src/services/graphql/entities/pending_file_match.rs` or `torrent.rs` — `#[derive(GraphQLEntity, GraphQLRelations, GraphQLOperations, ...)]`, `#[graphql_entity(table = "...", plural = "...", ...)]`, every field gets `#[graphql(name = "camelCase")]`. Register the entity in `schema_roots!` in `backend/src/services/graphql/schema.rs:20-66` (`entities: [...]` list) to get generated CRUD; add any custom `#[Object]` query/mutation types to `extra_query_types`/`extra_mutation_types` there too, following `SourceCustomQueries`/`SourceCustomMutations` (`backend/src/services/graphql/entities/source.rs:330-696`).

### 0.3 Known cross-feature dependency
**Quality scoring is a prerequisite for auto-download's release selection** (design.md Q37: "prefer the newer/better quality download"; Q25 upgrade-rank logic needs to exist before auto-download can pick between multiple search results). However, quality profiles as a *user-facing configurable entity* is a bigger, more novel piece of work than a minimal "parse + rank" module. Recommended sequencing (detailed in §5): build a **minimal internal release-scoring module first** (shared by both features), ship auto-download against it with hardcoded sane defaults, then build the full `QualityProfile` entity + settings UI on top, wiring auto-download's scorer to read from it once it exists. This avoids a hard blocking dependency while still respecting the logical ordering.

---

## 1. Auto-Download Loop

### What exists today
| Concern | File:line | State |
|---|---|---|
| `wanted` flags | `episode.rs:93-95`, `track.rs:89-91`, `chapter.rs:64-66`, `movie.rs:151-153` | Present, but nothing reads them to trigger search |
| `auto_download`/`auto_download_mode` | `show.rs:127-130`, `album.rs:110-113`, `audiobook.rs:116-119` | Present (enum `None`/`All`/`Wanted` in `common.rs:74-87`), unconsumed |
| Torrent grab primitive | `backend/src/services/torrent/service.rs:112-193` `add_magnet_with_metadata(magnet, user_id, TorrentAddMetadata{library_id, source_url, source_indexer_id, source_feed_id})` | Fully working, just needs a caller |
| Source search | `backend/src/services/sources/manager.rs:177-257` `search_all`/`search_sources` | Fully working, only called from the manual `searchSources` GraphQL query today (`source.rs:404-504`) |
| Download completion → import | `TorrentService::process_completed_torrent` (`torrent/service.rs:199-241`) → `process_completed_torrent_core` in `library_scan.rs` | Fully working end-to-end; this is the "grab→import" back half already built |
| `src/jobs/` | — | **Does not exist.** Referenced by `docs/design.md` and `.cursor/rules/media-pipeline.mdc` but absent (audit confirms) |
| Partial-fulfillment flag (`download_individual_items`, Q39) | — | **Missing entirely** — no column anywhere in the entity tree |
| "Downloading" status pre-completion | — | **Gap**: `PendingFileMatch` rows are created only inside `process_completed_torrent_core`/file-level processing (`library_scan.rs:6880-6965`), i.e. *after* the torrent finishes. Between grab and completion there is currently no DB record that a wanted item has an active download. |

### New modules
- `backend/src/jobs/mod.rs` — new top-level module, declared in `main.rs` next to `mod services;`
- `backend/src/jobs/auto_download.rs` — candidate discovery: for each library type, query wanted items whose parent enables auto-download (`Show.auto_download`/`auto_download_mode`, `Album`/`Audiobook` equivalents; movies use the library-level "wanted" concept per `movie.rs:448-450` `wantedMissing`). Build a `SourceQuery` (reuse `backend/src/services/sources/types.rs:167-283`) per wanted item and call `SourcesManager::search_all`/`search_sources`. Hands results to the scoring module (§3 dependency) to pick a release, then calls `TorrentService::add_magnet_with_metadata`.
- `backend/src/jobs/download_monitor.rs` — the periodic background service (`AutoDownloadService: Service`) that:
  - runs `auto_download::run_once()` on a timer (config knob, e.g. `auto_download_interval_minutes` in `app_settings`, loaded via `get_setting_string()` per design.md's settings-loading rule)
  - runs a **retry/reconciliation sweep** modeled directly on `torrent/service.rs:697-721`'s `reconciliation_sweep_loop` — re-checks torrents whose `post_process_status` is still null/`unmatched` after N minutes, and separately (per design.md Q34) limits "no pending match yet" retries to torrents added within the last 7 days
  - is the concrete file design.md's Key Files table already points at (`backend/src/jobs/download_monitor.rs`)
- `backend/src/services/graphql/mutations/auto_download.rs` — exposes `triggerAutoDownload(libraryId: ID)` for manual "search now", and a query `autoDownloadCandidates` for a settings/debug view. Wire into `schema.rs` `extra_mutation_types`/`extra_query_types`.

### Entity/schema additions
1. **Torrent wanted-linkage fields** (new nullable columns on `Torrent`, `torrent.rs:29-128`): `episode_id`, `movie_id`, `track_id`, `chapter_id`, `show_id`, `album_id`, `audiobook_id` (all `Option<String>`, `#[filterable(type = "string")]`). Set by the auto-download job at grab time. Closes the "downloading" status gap — computed-status resolvers (incl. `entities/common.rs:51-70` stub) should check "is there a non-terminal torrent referencing this item" in addition to `PendingFileMatch`.
2. **`download_individual_items`** boolean column on `Show`/`Album`/`Audiobook` (design.md Q39).
3. **Match-attempt / failure counters already exist** on `PendingFileMatch` (`match_attempts`, `copy_attempts`) — reuse for Q40/Q41 notification triggers.
4. Optional: a lightweight `SearchAttempt`/`GrabLog` entity if per-run auditability is wanted (skip for MVP).

### Integration points
- **Grab**: `TorrentService::add_magnet_with_metadata` — pass the new wanted-target id(s) through an extended `TorrentAddMetadata`.
- **Import**: no changes needed — `process_completed_torrent_core` already handles matching/organizing; auto-download is purely upstream (search → grab).
- **Release selection**: depends on quality scoring (§2) — must not grab the first result; rank by seeders/freeleech *and* parsed quality, falling back to "highest seeders, reasonable resolution" pre-quality-profiles.
- **Season packs (Q50)**: only ever build per-episode `SourceQuery`s, never season-pack queries.
- **Notifications**: reuse `Notification` entity for Q40/Q41; Q43 disk-space is a natural follow-on once the monitor loop exists.

### Risks / unknowns
- **Sequencing risk (critical)**: without at least a minimal scorer, v1 auto-download grabs garbage. Minimum: "reject <70% fuzzy title match" + "prefer higher seeders + parseable resolution".
- The 0.65/0.70 episode auto-link floor bug (`library_scan.rs:4738,4192,5488,2888`) sits directly in the completion path auto-download flows through — a **blocking correctness risk**: shipping auto-download before it's fixed means automated, unattended wrong-show links.
- Broadcast channel lag on torrent completion (`torrent/service.rs:915-932`) makes the reconciliation sweep load-bearing, not optional.
- No "search budget"/backoff across many wanted items hitting rate-limited trackers — the monitor must throttle per-source and add its own outer concurrency cap.

### Effort estimate
**Large.** Grab/import halves are done; net-new is candidate discovery across 4 media types, the monitor loop + retry/reconciliation, the new Torrent linkage columns (and every read-path needing them for "downloading" status), and enough scoring (§2) to not grab garbage. Highest-risk, highest-value item.

### Implemented (phase A)

Shipped:
- `backend/src/jobs/mod.rs`, `backend/src/jobs/auto_download.rs` (candidate discovery + grab), `backend/src/jobs/download_monitor.rs` (`AutoDownloadService: Service`), `backend/src/jobs/scoring.rs` (phase-A inline scorer). `mod jobs;` declared in `main.rs` next to `mod services;`, so the rule pack's globs in `.cursor/rules/media-pipeline.mdc` now match real files.
- `AutoDownloadService` registered in `services/manager.rs` (`ServiceRegistrationKind::AutoDownload`, `register_auto_download`/`get_auto_download`/`get_auto_download_unchecked`) and wired in `main.rs` in dependency order (`["database","graphql","sources","torrent"]`).
- Candidate discovery across all 4 media types, always per-item queries (per-episode, per-track; never season-pack/whole-album — Q50). Audiobooks are the one exception by design: searched/grabbed at the whole-audiobook level when any chapter is wanted, since audiobook releases are essentially never published per-chapter (new **Q52** in design.md).
- Movies: candidates are `monitored = true && wanted = true` (no separate per-movie auto-download toggle exists on `Movie`, unlike Show/Album/Audiobook).
- Minimal inline scorer (`jobs::scoring`): rejects releases below 70% `strsim::jaro_winkler` title similarity against the wanted item's search title; survivors ranked by parsed resolution rank (2160p/4K > 1080p > 720p > 480p > unknown) first, then seeders, then similarity. Marked with `// TODO(quality-profiles): replace with services::quality::scoring` at both the module doc and the `run_once` call site for the next agent.
- `Torrent` gained 7 nullable wanted-linkage columns (`episodeId`, `movieId`, `trackId`, `chapterId`, `showId`, `albumId`, `audiobookId`), all `#[filterable(type = "string")]`. `TorrentAddMetadata` gained matching fields; `torrent::database::create_torrent` takes a new `WantedLinkage<'a>` parameter that populates them via the existing `CreateTorrentInput` GraphQL mutation path (no raw SQL). Auto-download stamps these at grab time.
- Retry/reconciliation sweep (`jobs::download_monitor::run_retry_sweep`) modeled on `torrent/service.rs`'s `reconciliation_sweep_loop`: re-checks torrents with a wanted-linkage id set whose `post_process_status` is still null/`"unmatched"` after a configurable delay (default 15 min), bounded to torrents added within the last 7 days (Q34). Re-processing re-uses `TorrentService::process_completed_torrent`, the same entrypoint the torrent service's own completion handler and manual `processSource`/`rematchSource` use.
- `triggerAutoDownload(libraryId: ID)` mutation (`services/graphql/mutations/auto_download.rs`, wired into `schema.rs`'s `extra_mutation_types`) always runs one pass immediately regardless of the enabled gate — the safe way to exercise the loop.
- Config: `auto_download.enabled` (app_setting, JSON bool) checked each tick, falling back to `LIBRARIAN_AUTO_DOWNLOAD_ENABLED` env var, **default `false`**. `auto_download.interval_minutes` (default 60), `auto_download.retry_sweep_interval_minutes` (default 15), `auto_download.retry_after_minutes` (default 15), `auto_download.retry_window_days` (default 7) are all live-reloaded per tick — no restart needed to change them.
- Unit tests: 7 for the scorer (title-similarity threshold, resolution-rank ordering, resolution-over-seeders preference, seeder tiebreaker) + 7 for the retry-window/cutoff predicate (`needs_retry`), all DB-free. `cargo test` total went from 92 to 106 (94 unit + 12 integration); `backend_source_does_not_contain_direct_sql_access` still passes.
- **Confirmed the 0.65/0.70 episode auto-link floor bug is already fixed**, not naive: `library_scan.rs:6793-6798` implements exactly design.md Q32 — explicit-`Torrent.LibraryId` imports require confidence `>= 0.70`; all-library imports require `>= 0.90` with a `>= 0.10` margin over the second-best candidate. This was the plan's top blocking-risk item for enabling auto-download; it does not need further work from this agent, only the default-disabled posture below.

Deferred to later agents:
- **Quality-profiles retrofit** (§2/§5): `jobs::scoring` is intentionally a placeholder: no `is_upgrade`, no `QualityProfile`-aware ranking, no HDR/codec/source-type comparison. Replace wholesale per the `TODO(quality-profiles)` markers.
- `download_individual_items` boolean column on Show/Album/Audiobook (Q39) — not added. Not needed for phase A's correctness because auto-download only ever issues per-episode/per-track queries (never whole-show/whole-album), so the "don't re-download the same partial release" problem this flag guards against doesn't arise yet; it becomes relevant once a season/album-pack search path is added.
- Wiring the new `Torrent` linkage columns into the computed-status resolvers (`entities/common.rs`'s `calculate_content_status`) so `Status` reflects "downloading" before completion. That function (and the `ContentStatus` enum) is currently dead code — imported by `movie.rs`/`track.rs`/`episode.rs`/`chapter.rs` but never called from an actual resolver — so wiring it up is a separate, pre-existing gap, not something phase A regressed. The data needed to close it (this PR's linkage columns) now exists.
- `SearchAttempt`/`GrabLog` auditability entity — explicitly out of scope per the plan's MVP note.
- Per-source/global search budget beyond `SourcesManager`'s existing 45s-per-source timeout and 2-concurrent-searches-per-source semaphore — no additional auto-download-specific throttling was added in phase A.

---

## 2. Quality Profiles + Upgrade Automation

### What exists today
| Concern | File:line | State |
|---|---|---|
| `MediaFile` resolution/codec/HDR columns | `media_file.rs:58-94` | Present, ffprobe-populated |
| `quality_status` column | — | **Does not exist anywhere** — design.md Q24 describes it as authoritative but unimplemented |
| Filename quality-tag extraction | `library_scan.rs:8234-8246` `extract_quality_info`; release-tag regexes at `:2408,2540,8795,8927` | Present but purely string-matching for matcher/naming, not a structured parsed-release object, not reused for scoring |
| Parsed release metadata storage | `PendingFileMatch.parsed_resolution/parsed_codec/parsed_source/parsed_audio` (`pending_file_match.rs:85-99`) | Columns exist, populated during import — reusable as the "what we got" side of upgrade comparison |
| Frontend quality UI | `frontend/.../QualitySettingsCard.tsx` (483 lines) + `ShowSettingsModal.tsx` | **Fully built UI expecting per-show override fields that DO NOT exist on the backend and are not wired into any route** (`ShowSettingsModal` exported but never imported — orphaned). The frontend was built ahead of a backend that never shipped. |
| Upgrade decision logic (Q25) | — | **Does not exist** — no `is_upgrade()` anywhere |

### Design decision (call out to the implementing agent)
Two shapes: (1) bolt 8 override columns onto `Show` to match the orphaned frontend, or (2) a proper reusable `QualityProfile` entity (what design.md Phase 4 and the audit actually call for; what Sonarr/Radarr do).

**Recommendation: (2).** Only shape supporting "a profile per library/show" as a reusable, nameable thing, supports cutoff/upgrade-until cleanly, avoids duplicating 8+ columns across four entity types. Reuse the existing `QualitySettingsCard.tsx` field *shape* as `QualityProfile`'s columns; repoint the component at `QualityProfile` create/edit + a `qualityProfileId` selector on `Library`/`Show`/`Movie`/`Album`/`Audiobook`. This is a "frontend adaptation, not a rewrite".

### New modules
- `backend/src/services/quality/mod.rs` — new module (keep separable/unit-testable; `library_scan.rs` is already 9,138 lines)
  - `quality/profile.rs` — evaluation: given `MediaFile`/parsed fields/`SourceRelease` title + a resolved `QualityProfile`, return `Optimal`/`Suboptimal(reasons)`. Implements Q23/Q26 (`allows_any()` when empty).
  - `quality/scoring.rs` — shared release-ranking module (referenced in §0.3/§1): parses `SourceRelease.title` into `ParsedRelease{resolution, codec, hdr, source_type, audio, release_group}` (refactor `extract_quality_info`/regex sets from `library_scan.rs` into a shared parser) and implements `is_upgrade(new, existing) -> bool` per Q25 exactly.
- `backend/src/services/graphql/entities/quality_profile.rs` — new entity.
- `backend/src/services/graphql/mutations/quality.rs` — `evaluateMediaFileQuality(mediaFileId)`, `checkForUpgrade(mediaFileId)` (Q38).

### Entity/schema additions
1. **`QualityProfile` entity** (`quality_profiles`): `id`, `name`, `media_kind` (`video`/`audio`), `allowed_resolutions` (`#[json_field] Vec<String>`), `allowed_video_codecs`, `allowed_audio_formats`, `require_hdr: bool`, `allowed_hdr_types`, `allowed_sources`, `release_group_blacklist`, `release_group_whitelist`, `cutoff_resolution: Option<String>`, `upgrade_until_cutoff: bool`, `is_default: bool`, timestamps. Use the existing `#[json_field]` pattern (e.g. `Show.genres` at `show.rs:112-114`).
2. **`quality_profile_id`** nullable FK on `Library` (primary) and optionally `Show`/`Movie`/`Album`/`Audiobook` (override).
3. **`quality_status`** on `MediaFile` (`Option<String>`, `optimal`/`suboptimal`/null) — the column the audit calls out as missing. Populated after ffprobe analysis (hook into `analyzeMediaFile`/`ffprobe_analyze` in `library_scan.rs:7959`), re-evaluated when the assigned profile changes.

### Public GraphQL surface
- Generated CRUD on `QualityProfile` (free via macro once registered in `schema.rs`).
- `updateLibrary`/`updateShow`/etc. gain `qualityProfileId: Option<String>`.
- Custom: `evaluateMediaFileQuality(...)`, `pendingUpgrades(libraryId)` (Q38 UI), `approveUpgrade`/`dismissUpgrade` backing Q38 notification actions.

### Integration points
- **Reused by auto-download (§1)**: `scoring::is_upgrade` + `ParsedRelease` parser are the shared substrate; auto-download filters candidates against the wanted item's resolved profile before ranking.
- **Reused by torrent import**: `process_completed_torrent_core`'s parsed_* fields become input to `quality::profile::evaluate`; duplicate-torrent handling (Q44) calls `scoring::is_upgrade`.
- **FFprobe → `quality_status` write-back**: hook recompute into `analyzeMediaFile`; add a bulk `recomputeLibraryQualityStatus(libraryId)` for profile reassignment.

### Risks / unknowns
- Profile-resolution precedence: per-entity `quality_profile_id` wins, else `Library.quality_profile_id`, else a seeded "Any Quality" default (seed via `bootstrap_defaults.rs:808` `seed_defaults`).
- Verify `#[filterable]` on a `#[json_field]` is supported by the pinned `graphql-orm` rev before promising filter support (likely not needed for v1).
- Trap: don't reflexively "just add the override columns the frontend expects" — build the entity properly.

### Effort estimate
**Medium-Large.** Entity + evaluator + upgrade-detection is self-contained and testable. Frontend adaptation is a separate chunk.

### Implemented

Shipped:
- `backend/src/services/quality/` module: `scoring.rs` (`ParsedRelease` parser — resolution/codec/HDR/source/audio/release-group; `title_similarity`/`normalize_title`/`MIN_TITLE_SIMILARITY` moved here; `is_upgrade(new, existing: Option<&ParsedRelease>)` per Q25 exactly — unknown existing quality or a higher resolution is always an upgrade, same-resolution-plus-new-HDR is an upgrade, lower resolution never is; `rank_candidates`/`pick_best` retained as the profile-agnostic title-similarity+resolution+seeders ranking) and `profile.rs` (`QualityEvaluation::{Optimal, Suboptimal(reasons)}`, `evaluate()` implementing Q23/Q26 `allows_any()`, `parsed_release_for_media_file()` merging ffprobe-verified `MediaFile` columns with filename-parsed source-type/release-group, `resolve_profile_for_media_file`/`resolve_profile_for_library`/`resolve_profile` implementing the per-entity-override > `Library.qualityProfileId` > seeded-default precedence, `recompute_and_persist()` as the single evaluate+write entrypoint shared by the ffprobe hook and the GraphQL mutations, `should_seek_upgrade()` for a future upgrade-sweep job, `builtin_any_profile()` fallback).
- `QualityProfile` entity (`backend/src/services/graphql/entities/quality_profile.rs`, table `quality_profiles`) with `MediaKind` (VIDEO/AUDIO) enum and all fields from the plan (`allowedResolutions`/`allowedVideoCodecs`/`allowedAudioFormats`/`allowedHdrTypes`/`allowedSources`/`releaseGroupBlacklist`/`releaseGroupWhitelist` as `#[json_field] Vec<String>` — no `#[filterable]` on any of them, per the plan's caution; `requireHdr`, `cutoffResolution`, `upgradeUntilCutoff`, `isDefault`, timestamps). Registered in `schema.rs` `schema_roots!` (free CRUD) **and** in `database.rs`'s separate `entity_metadata()` list (this is what actually drives schema-sync table creation — a second, easy-to-miss registration point the plan didn't call out; also needed a `batch_load.rs` entry to satisfy the `batch_loader_supports_every_graphql_orm_entity_export` contract test).
- `quality_profile_id: Option<String>` FK added to `Library` (primary), `Show`, `Movie`, `Album`, `Audiobook` (overrides). `quality_status: Option<String>` added to `MediaFile`.
- Seeded default "Any Quality" profile (`bootstrap_defaults.rs::seed_quality_profiles`, idempotent on `is_default`).
- GraphQL surface: generated CRUD on `QualityProfile` (free); `qualityProfileId` now appears in `updateLibrary`/`updateShow`/`updateMovie`/`updateAlbum`/`updateAudiobook` inputs automatically. Custom (`mutations/quality.rs` + `queries/quality.rs`): `evaluateMediaFileQuality(mediaFileId)`, `recomputeLibraryQualityStatus(libraryId)`, `pendingUpgrades(libraryId)` (unresolved `quality_upgrade` notifications), `approveQualityUpgrade(notificationId)`. **No separate `dismissUpgrade` mutation** — dismissal is the already-existing generic `updateNotification(id, { resolution: DISMISSED })` mutation, which the frontend notifications UI already called for every other notification category; only *approving* an upgrade has a real side effect (file replace) that needed a dedicated mutation.
- **Retrofit**: `jobs/scoring.rs` **deleted** (was explicitly "meant to be swapped wholesale"). `jobs/auto_download.rs` now imports `services::quality::{profile, scoring}`; each `WantedCandidate` carries a `quality_profile_override_id` (the show/movie/album/audiobook's override, populated at discovery time), and `run_once` resolves the effective profile per candidate (`profile::resolve_profile`, falling back to `builtin_any_profile()` with a warning on any resolution error — auto-download must never hard-fail because of a missing profile row), **filters** search results to only those whose parsed release passes `profile::evaluate` (Q23), then ranks survivors with `scoring::rank_candidates`/`pick_best`. `jobs::auto_download`'s own tests (candidate discovery, linkage) were unaffected; the phase-A scorer's 7 unit tests were superseded by `quality::scoring`'s 20.
- **Torrent-import integration (Q44/Q38)**: `LibraryScanService::materialize_torrent_file`'s existing "target path already exists → skip" branch now also calls a new `maybe_notify_quality_upgrade` helper: it looks up the existing `MediaFile` at the target path, parses the new file's filename and the existing file's ffprobe-verified+filename-parsed fields into `ParsedRelease`, and if `scoring::is_upgrade` says the new one is better, creates an `ACTION_REQUIRED`/`QUALITY` notification (`actionType: "quality_upgrade"`, `actionData` JSON carrying `sourcePath`/`targetPath`/`mediaFileId`/both parsed resolutions) via a new `create_action_notification` helper (extends the existing `create_notification` with `libraryId`/`mediaFileId`/`actionType`/`actionData`). The file is **never** replaced automatically (Q38) — the existing "conflict skipped" warning/notification still fires unconditionally afterward, so behavior is strictly additive.
- **FFprobe → `quality_status` write-back**: `LibraryScanService::analyze_media_file_inner` calls `profile::recompute_and_persist` right after the ffprobe-derived columns are written; failures are logged as warnings, not propagated (Q49 — quality gaps must not fail the analysis job). `extract_quality_info` (used for the `{quality}` naming-pattern token) now delegates to `scoring::parse_release(..).resolution` instead of duplicating the 4-tag substring match — the one part of the plan's "refactor library_scan.rs's regexes into the shared parser" that was safe to do as a small, isolated, single-call-site change.
- Unit tests: 20 in `quality::scoring` (parser coverage per field, `is_upgrade` per Q25 branch, ranking) + 9 in `quality::profile` (`allows_any`, per-field rejection/acceptance, HDR-required, release-group blacklist/whitelist, cutoff-based `should_seek_upgrade`). Backend `cargo test` went from 106 to **126** (114 unit + 12 integration, up from 94+12); `backend_source_does_not_contain_direct_sql_access` and `batch_loader_supports_every_graphql_orm_entity_export` both still pass.
- **Frontend**: `QualitySettingsCard.tsx` is no longer orphaned — it's now the field editor inside a new `QualityProfileEditorModal.tsx` (create/edit a `QualityProfile`, plus name/mediaKind/cutoffResolution/upgradeUntilCutoff/isDefault), wired into a new routed page `/settings/quality-profiles` (list + create/edit/delete, added to the settings nav in `routes/settings.tsx`). A new `QualityProfileSelector.tsx` (mirrors `NamingPatternSelector.tsx`) is wired into `LibrarySettingsForm.tsx`/`LibrarySettingsTab.tsx` (library's primary `qualityProfileId`) and into `ShowSettingsModal.tsx`, which had its 8 nonexistent override fields (`allowedResolutionsOverride` etc. — never backed by any backend column) replaced with a single `qualityProfileId` override + inherit option. The notifications UI (`NotificationDetailModal.tsx` + its two callers `NotificationPopover.tsx`/`routes/notifications.tsx`) gained an "Approve Upgrade" action for `quality_upgrade` notifications that calls the new `approveQualityUpgrade` mutation instead of the generic resolve path.
  - GraphQL codegen has since been run successfully against the current schema snapshot. `src/lib/graphql/qualityProfiles.ts` is now only a thin feature-facing alias layer over generated documents and types; it no longer contains hand-authored `DocumentNode` ASTs.
  - `LibraryDetailRoute`/`UpdateLibraryRoute`, `qualityProfileId`, and all quality-profile operations now come from the generated client.
  - The production build regenerated and validated `routeTree.gen.ts`, including `/settings/quality-profiles`.

Deferred to later agents:
- **`should_seek_upgrade` has no caller**: there is no background "actively sweep the library for upgrades toward `cutoffResolution`" job. `upgrade_until_cutoff`/`cutoffResolution` are stored, editable in the UI, and covered by unit tests, but only passively relevant today (via the Q44 duplicate-torrent path). A real upgrade-sweep job (parallel to `jobs::auto_download`, searching sources for already-downloaded-but-suboptimal items) is a natural follow-on, not built here.
- **`approveQualityUpgrade`'s file replace is best-effort and doesn't reconcile `MediaFile.size`/other stale columns**: it hardlinks/copies the new file over the old path and queues `analyzeMediaFile` (which refreshes codec/resolution/HDR/audio/bitrate/duration), but `size` isn't part of that ffprobe-driven update path anywhere in the existing pipeline — a pre-existing gap this change doesn't introduce but also doesn't fix.
- **`ShowSettingsModal.tsx` is still not wired into any route.** Only its quality section was fixed (real `qualityProfileId` instead of 8 fictional override fields); its automation section (`autoHuntOverride`, `monitorType` values `ALL`/`FUTURE`/`NONE` that don't match `Show.autoDownloadMode`'s actual `NONE`/`ALL`/`WANTED`) and organization section (`organizeFilesOverride`, `renameStyleOverride`) reference concepts that don't exist on the `Show` entity at all — a separate, larger, unrelated pre-existing gap out of scope for quality profiles.
- Bulk "recompute all libraries" convenience (only per-library `recomputeLibraryQualityStatus` exists; a global sweep would just loop libraries client-side today).
- `#[filterable]` on `QualityProfile`'s `#[json_field]` Vec<String> columns was not attempted (per the plan's own caution) — filtering profiles by "contains resolution X" isn't exposed.

---

## 3. Torznab/Newznab Client Source

### What exists today
| Concern | File:line | State |
|---|---|---|
| `Source` trait | `sources/mod.rs:103-150` | Stable: `search`, `download`, `test_connection`, `capabilities` |
| 5 hardcoded scrapers | `sources/definitions/{1337x,iptorrents,limetorrents,thepiratebay,yts}.rs` | Working but scraper-fragile; 1337x/LimeTorrents produce no downloadable link |
| Torznab-shaped types already | `sources/types.rs` (`QueryType`, `TvSearchParam`, `MovieSearchParam`, `SourceCapabilities`, `SourceRelease` — "modeled after the Torznab specification") | Internal types are already Torznab-shaped — mapping is ~1:1, not a redesign |
| Category mapping | `sources/categories.rs` (`CategoryMapping`, `cats::*` matching the Newznab numbering) | Ready to use for parsing `<newznab:attr name="category">` |
| `TorznabCategory` entity | `entities/torznab_category.rs` (registered `schema.rs:65`) | Lookup table, usable for a category picker |
| Server-side Torznab (provider) code | — | **Already deleted** — client is genuinely greenfield |
| XML parsing dep | `Cargo.toml:100` `quick-xml = "0.37"` | Available, no new crate |
| Source loading/registration | `sources/manager.rs:66-138` `load_source()` match on `definition_id` | New arm needed |
| Definition registry | `sources/definitions/mod.rs:70-154` `AVAILABLE_DEFINITIONS` | New entry needed |

### New modules
- `backend/src/services/sources/definitions/torznab.rs` — `TorznabSource: Source`:
  - Constructed from `site_url` + a required `ApiKey` credential — add `"torznab"` to `AVAILABLE_DEFINITIONS` with `required_credentials: &["ApiKey"]` + optional default-categories setting (mirror the `iptorrents` `optional_settings` shape).
  - `search()`: build `{site_url}/api?t={search|tvsearch|movie|music|book}&q=...&cat=...&apikey=...` → parse RSS 2.0 + `<torznab:attr>`/`<newznab:attr>` XML with `quick_xml` → map each `<item>` to `SourceRelease` (title, guid/link/enclosure→`link`, `pubDate`→`publish_date`, size from `<enclosure length>` or `<torznab:attr name="size">`, seeders/peers/infohash/imdb/tmdb from attrs).
  - `test_connection()`: hit `{site_url}/api?t=caps&apikey=...`, parse `<caps>` to populate `SourceCapabilities` dynamically; cache parsed capabilities on the struct.
  - `download()`: fetch the `link`/`enclosure` URL (reuse `yts.rs:236-243` client pattern).
- No new GraphQL surface required — `availableSourceDefinitions`, `sourceSettingDefinitions`, `createSource`, `testSource` all work generically once `"torznab"` is a valid `definition_id`.
- Confirm `createSource` supports multiple `Source` rows with the same `definition_id` but different `site_url`/credentials (it should — `load_source` keys off `source_id`).

### Entity/schema additions
- **None on `Source`** — `site_url`, encrypted `credentials` (`ApiKey`), and `settings` JSON cover everything.
- Optional `default_categories` stored in the existing `settings` field (not a column).

### Integration points
- Auto-download (§1) and manual `searchSources` both call `SourcesManager::search_all` — Torznab slots in with zero caller changes once registered.
- Quality scoring (§2): prefer parsing `<torznab:attr name="resolution">` directly into `ParsedRelease` when present.

### Risks / unknowns
- **Set `.timeout()`/`.connect_timeout()` on this source's own `reqwest::Client`** (copy `yts.rs:74-78`, the good example — don't repeat the other four scrapers' missing-timeout mistake). A generic client talks to arbitrary third-party instances.
- `t=caps` adds a round-trip; cache aggressively, fail soft with a conservative default if unimplemented.
- Category edge cases: reuse `map_tracker_to_torznab`/`map_torznab_to_tracker` (`types.rs:141-163`) parent-bucket fallback, don't reimplement.
- Scope to **torrent** sources only (`SourceType::TorrentIndexer`); usenet/NZB is a Tier-2 follow-on (no NNTP client exists yet).

### Effort estimate
**Small-Medium.** Cheapest of the four — types already Torznab-shaped, XML dep present, no schema changes, no dead code. Bulk of work is XML→`SourceRelease` mapping and `t=caps` parsing.

### Implemented

Shipped:
- `backend/src/services/sources/definitions/torznab.rs` (new, ~700 lines incl. tests) — `TorznabSource: Source`, `SourceType::TorrentIndexer` only. Registered as `"torznab"` in `AVAILABLE_DEFINITIONS` (`definitions/mod.rs`) with `required_credentials: &["ApiKey"]` and one optional setting, `DefaultCategories` (comma-separated Torznab category IDs, mirrors the `iptorrents` `optional_settings` shape). `SourcesManager::load_source` (`manager.rs`) gained a `"torznab"` match arm pulling `credentials.get("ApiKey")` and constructing the source from `site_url` + `settings`.
- **HTTP client**: `reqwest::Client` built with `.gzip(true).timeout(Duration::from_secs(30)).connect_timeout(Duration::from_secs(10))` — copied from `yts.rs:74-78` per the plan's explicit instruction, not the other four scrapers' missing-timeout pattern. Uses `RateLimitedClient::for_indexer()` (1 req/s) like `iptorrents`/`limetorrents`.
- **`search()`**: builds `{site_url}/api?t={search|tvsearch|movie|music|book}&apikey=...&extended=1` (query-type string comes straight from `QueryType`'s existing `Display` impl, which already emits the exact Torznab tokens) plus `q`, `cat`, `season`, `ep`, `imdbid` (via the existing `SourceQuery::imdb_id_short()` helper), `tvdbid`, `tmdbid`, `tvmazeid`, `year`, `genre`, `album`, `artist`, `title`, `author`, `limit`, `offset` as applicable. Parses the RSS 2.0 response with `quick_xml`'s low-level `Reader`/`Event` API (mirrors the existing test-only usage pattern at `library_scan.rs:8681-8724` — no new dependency/feature needed; `quick_xml::de`/serde parsing was deliberately **not** used since it's only incidentally enabled via feature unification with an unrelated dependency, not declared by this crate's own `Cargo.toml`). `local_name()` is used throughout so both `torznab:attr` and `newznab:attr` (older items) are handled uniformly regardless of namespace prefix. Maps title/guid/link-or-enclosure/pubDate/size(`<size>` or `<enclosure length>` or `torznab:attr size`)/seeders/peers/leechers/infohash/magnet/imdb/tmdb/tvdb/tvmaze/year/download-upload-volume-factor/minimum-ratio/minimum-seed-time/poster into `SourceRelease` 1:1, confirming the plan's "types already Torznab-shaped" claim. Resolution/codec/audio-format `torznab:attr`s (no dedicated `SourceRelease` field exists for them) are folded into the release title if not already present, so `services::quality::scoring::parse_release`'s title parser picks them up downstream, per the plan's "feed `services::quality::scoring`" note.
- **`test_connection()`**: calls `t=caps`, detects Newznab `<error code="..."/>` responses (auth-failure codes 100/101 → `Ok(false)`; any other error, e.g. "function not available" for indexers without `t=caps` → soft-fail, `Ok(true)`, keep fallback caps), else parses `<searching>`/`<categories>` into a real `SourceCapabilities` and caches it once via `OnceLock` (never re-fetched for that instance's lifetime — a fresh instance is created on every `SourcesService` reload). See design.md Q56 for the full fail-soft/fallback-capabilities writeup.
- **Category handling**: reuses `SourceCapabilities::map_torznab_to_tracker`/`map_tracker_to_torznab` (`types.rs:141-163`) exactly as instructed, not reimplemented — `<caps><categories>` is parsed into an *identity* `CategoryMapping` table (`tracker_id == torznab_cat` as strings, since a real Torznab endpoint already speaks the standard numbering), which gets the existing parent-bucket fallback "for free" (querying category `5030` before `t=caps` succeeds falls back to the round-thousands bucket `5000` from the generic default capabilities — covered by a unit test and called out in design.md Q56 as an intentional, documented behavior).
- **`download()`**: same pattern as `yts.rs` — rejects `magnet:` links (not directly downloadable), otherwise GETs the link and returns bytes.
- **Multiple same-`definition_id` sources**: confirmed via a new test, `services::sources::manager::tests::same_definition_id_supports_multiple_independent_instances` — `SourcesManager::load_source` keys everything (`sources`/`priorities`/`rate_limiters` maps) by `source_id`, not `definition_id`, so two differently-configured `"torznab"` rows (different `site_url`/`ApiKey`/priority) coexist and are independently retrievable/searchable. No schema change was needed, confirming the plan's expectation.
- **No new GraphQL surface** — verified end-to-end structurally: `availableSourceDefinitions`/`sourceSettingDefinitions`/`createSource`/`testSource` all read `AVAILABLE_DEFINITIONS` and `SourcesManager` generically with no per-`definition_id` whitelist anywhere in `services/graphql/` (the one other place a string-matches-on-indexer-type pattern exists, `graphql/helpers.rs:161`'s `download_torrent_file_authenticated`, is dead code with zero callers, and its `_ =>` branch is already API-key-aware anyway). `"torznab"` becomes usable the moment it's a valid `definition_id`, per the plan.
- Unit tests (10 new, all DB-free): 8 in `torznab.rs` (RSS→`SourceRelease` mapping against a realistic 2-item fixture incl. `torznab:attr` seeders/size/category/infohash/imdb/leechers/magnet/resolution-hint, `<caps>` parsing incl. TV/movie/music/book `supportedParams` and category/subcat→identity-mapping incl. parent-bucket fallback, Newznab `<error>` detection, `site_url` requirement, `/api` suffix resolution, outbound query-param building, `DefaultCategories` setting, `is_configured`) + 2 in `manager.rs` (multiple-same-`definition_id` coexistence; missing-`site_url` load failure). `cargo test` went from 126 (114 unit + 12 integration) to **136** (124 unit + 12 integration); `backend_source_does_not_contain_direct_sql_access` still passes (this feature added no SQL of any kind — pure HTTP + XML parsing).
- `docs/design.md` gained **Q56** (capabilities/category resolution before the first `t=caps` succeeds, fail-soft semantics, `TrackerType::Private` default, torrent-only scope).

Deferred to later agents:
- **Real Newznab/usenet support** (NZB download over HTTP is already covered structurally by `download()`, but there's no NNTP posting client, no `.nzb`-specific parsing beyond what the generic `SourceRelease` mapping already gives, and `SourceType::UsenetIndexer` is never selected) — explicitly out of scope per the plan, Tier-2 follow-on.
- **No per-indexer category override UI** beyond the single `DefaultCategories` optional setting — a real per-source category-picker (using the now-parsed `<caps><categories>` data) would need a new custom GraphQL query exposing a loaded source's live `SourceCapabilities.categories`, which doesn't exist today (`availableSourceDefinitions` only exposes the *static* `AVAILABLE_DEFINITIONS` metadata, not a configured instance's dynamically-fetched caps).
- **`t=caps` capabilities are fetched only from `test_connection()`**, i.e. only when a user clicks "test" or `SourcesService::reload()` happens to call it — there's no proactive caps warm-up at source-load time, so a freshly created Torznab source searches with only the generic fallback capabilities/categories until someone tests it. Given `search_all`/auto-download would otherwise incur an extra round-trip per fresh instance, this was left as explicitly fail-soft/lazy rather than eager, matching the plan's "cache aggressively, fail soft" framing, but a proactive caps warm-up on load is a reasonable small follow-on.
- **`<caps>`'s `<limits>` element (max/default page size) is not parsed or enforced** — `search()`'s own `limit`/`offset` params pass through from `SourceQuery` unchanged; no clamping against an indexer-advertised max was added.

---

## 4. HLS/FFmpeg Transcoding

### What exists today
| Concern | File:line | State |
|---|---|---|
| Direct-play streaming | `api/media.rs:71-205` `stream_media` (full Range support) | Working, stays as-is for compatible files |
| Transcode/quality query params | `api/media.rs:36-46` `StreamParams{transcode, quality}` | Parsed but only logged "not yet implemented" (`:82-95`) |
| Direct-play compatibility check | `api/media.rs:337-364` `is_chromecast_compatible(container, video_codec, audio_codec)` | Working, reusable as the "needs transcoding?" gate |
| `ffprobe` invocation | `library_scan.rs:7959-7975`; availability check `:380-395` `check_ffprobe_available()` | Working; no `ffmpeg` invocation/availability check anywhere |
| Config hook for output storage | `config/mod.rs:38` `cache_path` (env `CACHE_PATH`, `./data/cache`) | **Defined but entirely unused** — ready-made segment-output dir |
| Frontend player | `frontend/.../VideoPlayer.tsx` — imports `hls.js`, detects `.m3u8`, initializes `Hls`/native Safari HLS (`:47-67`) | **Already fully wired for HLS** — only backend gap is that no `.m3u8` URL is ever produced |
| Frontend src selection | `getMediaStreamUrl` | Always builds direct `/api/media/{id}/stream` — no HLS decision logic |

The best-understood gap: player done, config knob exists, ffprobe invocation is a direct template for ffmpeg — only the transcode/segment/serve pipeline is missing.

### New modules
- `backend/src/services/transcode/mod.rs` (parallel to `services/quality/`):
  - `transcode/ffmpeg.rs` — `ffmpeg_available()` (copy `check_ffprobe_available`), and `spawn_hls_remux(input, output_dir, session_id) -> Result<Child>` running `ffmpeg -i {input} -c copy -f hls -hls_time 6 -hls_playlist_type vod -hls_segment_filename {out}/seg_%03d.ts {out}/playlist.m3u8` (stream-copy remux — covers the common incompatible-*container* case). A second `spawn_hls_transcode` (`-c:v libx264 -c:a aac`) handles incompatible-*codec* (HEVC/DV) — sequence remux first (order of magnitude simpler), transcode as "if time permits" but don't fully punt (HEVC/DV rips need it).
  - `transcode/session.rs` — `TranscodeSession` bookkeeping (media_file_id, session_id, output dir under `cache_path`, ffmpeg `Child`, last-accessed) in an in-memory `RwLock<HashMap<String, TranscodeSession>>` (ephemeral runtime state, not a DB entity).
  - `transcode/service.rs` — `TranscodeService: Service`, depends on `["database"]`. Background: (1) reaper killing ffmpeg / deleting segment dirs for idle sessions, (2) startup cleanup of orphaned segment dirs in `cache_path`.
- `backend/src/api/media.rs` additions (extend the router at `:28-32`):
  - `GET /media/{file_id}/hls/playlist.m3u8` — decides direct-vs-HLS via a generalized `needs_transcode(container, video_codec, audio_codec) -> {None, Remux, Transcode}` (extract from `is_chromecast_compatible`), starts a session, waits for first segments, serves the playlist.
  - `GET /media/{file_id}/hls/{segment}` — serves `.ts` segments from the session dir with the **same auth guard** (`require_authenticated_user`) as `stream_media` (`:77,214`) — must not regress the audit's auth-on-/api/media fix.
  - `media_info` gains a `needsHls`/`playbackUrl` hint so the frontend decides without duplicating codec logic.

### Entity/schema additions
- **None.** Existing `MediaFile` `container`/`video_codec`/`audio_codec`/`resolution`/`is_hdr` columns suffice. Transcode sessions are intentionally ephemeral/in-memory.

### Public GraphQL surface
- None required for MVP (pure `/api` REST). Optional `mediaFile.needsTranscode: Boolean`/`transcodeReason: String` computed field for a "will transcode" badge.

### Integration points
- **Frontend**: `getMediaStreamUrl` gets a decision branch — call `/api/media/{id}/info` (already returns `chromecast_compatible`) and point `VideoPlayer`'s `src` at `/hls/playlist.m3u8` when incompatible. `VideoPlayer.tsx` needs zero changes (already branches on `.m3u8`).
- **Casting**: cast devices often have real HEVC support — keep the transcode decision endpoint/client-specific (browser vs cast), don't bake one global answer into `media_info`.
- **ffprobe**: read codec/resolution from `MediaFile` columns already populated by `ffprobe_analyze`; don't re-invoke ffprobe at playback time.

### Risks / unknowns
- No `ffmpeg`/`ffprobe` path config exists (bare `"ffprobe"` on `$PATH`) — read an optional `FFMPEG_PATH`/`ffmpeg_path` setting (mirror `cache_path`) so Windows-bundled-binary distribution isn't blocked.
- Process lifecycle: no `.unwrap()` on `Child` ops (`panic='abort'` makes the server fragile); verify ffmpeg children are killed on graceful shutdown (register the service so its `stop()` runs).
- Disk usage: segments accumulate under `cache_path`; the idle-session reaper is required for correctness, not polish.
- Remux-only requires a browser-decodable codec inside — genuine HEVC/10-bit still fails in remux mode; only the deferred transcode path fixes that. Shipping remux-only closes "most" of the gap per the audit's phrasing, not all.

### Effort estimate
**Medium** (remux-only MVP; +extra if full re-encode included). Strong existing scaffolding; main net-new work is process lifecycle (spawn/monitor/reap) + two HTTP routes.

### Implemented

Shipped:
- `backend/src/services/transcode/` module (parallel to `services/quality/`), registered in `services/mod.rs`:
  - `transcode/ffmpeg.rs` — `ffmpeg_available(ffmpeg_path)` (copies `check_ffprobe_available`'s exact shape: `-version` invocation, `warn!` with stderr/spawn-error on failure). `build_remux_args`/`build_transcode_args` are pure functions (unit-tested without spawning ffmpeg) building the argument vectors described in the plan (`-c copy` stream-copy remux; `-c:v libx264 -preset veryfast -c:a aac` transcode), both writing `{out}/seg_%03d.ts` segments + `{out}/playlist.m3u8` via `-hls_time 6 -hls_playlist_type vod`. `spawn_hls_remux`/`spawn_hls_transcode` spawn via `tokio::process::Command` with `Stdio::null()` for stdout/stderr (avoids a pipe-buffer deadlock from ffmpeg's continuous progress output) and `kill_on_drop(true)` as a last-resort safety net; no `.unwrap()` on any `Child` operation anywhere in the module.
  - `transcode/session.rs` — `TranscodeSession` (media_file_id, session_id, output dir, `TranscodeKind::{Remux,Transcode}`, the `Child` handle, `last_accessed: DateTime<Utc>`) and `SessionRegistry` (`RwLock<HashMap<String, TranscodeSession>>` keyed by `media_file_id` — intentionally ephemeral in-memory state, not a DB entity, per the plan). `session_is_idle`/`sanitize_segment_name` are pulled out as pure functions for unit testing: the former backs the reaper's idle cutoff, the latter is the only thing standing between `GET /hls/{segment}`'s attacker-controlled `segment` path param and a path-traversal/arbitrary-file-read bug (rejects `..`, `/`, `\`, anything not ending in `.ts`).
  - `transcode/service.rs` — `TranscodeService: Service`, `dependencies() = ["database"]`. `start()` runs the `ffmpeg_available` check once (stored for `health()`, mirroring `LibraryScanService`'s `ffprobe_available` pattern), wipes+recreates `{cache_path}/hls` (safe because sessions are in-memory only — nothing on disk at process start can have a live `Child`, so everything under it is definitionally orphaned from a prior run), then spawns the reaper loop (torrent-service cancellation-token pattern: `CancellationToken` + `tokio::select!` between `cancelled()` and a sleep, `JoinHandle` stored and awaited on stop). `stop()` cancels+joins the reaper **and** calls `SessionRegistry::kill_all()`, which kills every live ffmpeg child (bounded 5s wait after the kill signal, never blocks shutdown indefinitely on a wedged process) and deletes its segment directory — this is what's registered with `ServicesManager` (`ServiceRegistrationKind::Transcode`, `register_transcode`/`get_transcode`/`get_transcode_unchecked`) so graceful shutdown (`main.rs`'s `services.stop_all()` on SIGTERM/Ctrl-C) never leaves an orphaned ffmpeg process running.
- `backend/src/api/media.rs` additions (router extended, `stream_media`/`media_info` untouched apart from `media_info`'s new fields below):
  - `needs_transcode(container, video_codec, audio_codec) -> TranscodeDecision{None, Remux, Transcode}` generalizes `is_chromecast_compatible` onto three shared predicates (`compatible_container`/`compatible_video_codec`/`compatible_audio_codec`): all three OK → `None` (direct play); codecs OK but container isn't → `Remux`; either codec itself isn't browser-decodable → `Transcode`. **Bug found and fixed while writing this**: a naive `container.contains("webm")` substring check (inherited from the original `is_chromecast_compatible`) wrongly matched real MKV rips, since ffprobe's `format_name` for both genuine WebM *and* Matroska/MKV is literally `"matroska,webm"` (WebM is a constrained Matroska profile sharing the same demuxer name) — every MKV file was being misclassified as direct-playable. Fixed by treating any `"matroska"`-containing format name as container-incompatible (conservatively remuxes real standalone `.webm` too, which is still fully correct output, just not the fastest path for that rarer case). Covered by a regression test (`compatible_container_treats_any_matroska_format_name_as_incompatible`).
  - `GET /media/{file_id}/hls/playlist.m3u8` — same `require_authenticated_user` guard as `stream_media`, resolves the `MediaFile`, computes `needs_transcode` (a `None` decision here still gets remuxed — the client already decided via `media_info`'s hint that it wants this endpoint), starts or reuses a `TranscodeSession` via `TranscodeService::get_or_start_session`, bounded-polls (250ms/20s) for the playlist file to be non-empty, serves it as `application/vnd.apple.mpegurl`.
  - `GET /media/{file_id}/hls/{segment}` — same auth guard, validates `segment` via `sanitize_segment_name`, requires an already-started session (`TranscodeService::touch_session`, 404 if absent — segment requests never start a session on their own), bounded-polls (200ms/15s) for the segment file, serves as `video/mp2t`.
  - `media_info` gains `needs_hls: bool`, `transcode_decision: "none"|"remux"|"transcode"`, and `playback_url: string` (`/stream` or `/hls/playlist.m3u8`). Kept **snake_case** to match every other field already in this endpoint's JSON body (the plan's `needsHls`/`playbackUrl` naming was GraphQL-convention shorthand; this is a plain REST/JSON endpoint, and `docs/design.md`'s "Serde casing applies only to JSON payloads; it is not a substitute for GraphQL resolver naming" rule means the GraphQL camelCase convention doesn't apply here — consistency with the endpoint's existing fields wins).
- `Config` (`config/mod.rs`) gained `ffmpeg_path: Option<String>` (env `FFMPEG_PATH`), following `cache_path`'s exact shape; defaults to `"ffmpeg"` on `$PATH` when unset (resolved in `main.rs`'s `TranscodeServiceConfig` construction, not in `Config` itself, so the config struct keeps storing the raw optional override).
- Unit tests (23 new, all DB-free, no real ffmpeg/media spawned): 3 in `transcode::ffmpeg` (remux/transcode arg vectors, playlist/segment path helpers), 6 in `transcode::session` (idle-cutoff boundary cases, segment-name sanitization incl. path-traversal rejection), 10 in `api::media` (the full `needs_transcode` decision matrix, the MKV/WebM container regression, `TranscodeDecision::as_str`, `is_chromecast_compatible`/`needs_transcode` cross-consistency, range-header parsing). Backend `cargo test` went from 136 (124 unit + 12 integration) to **155** (143 unit + 12 integration); `backend_source_does_not_contain_direct_sql_access` still passes (this feature added no SQL — sessions are in-memory, and the only DB read in the HLS routes reuses `stream_media`'s existing `MediaFile::get` entity call).
- **Frontend**: correcting the plan's premise here — `VideoPlayer.tsx`'s hls.js-wired `<video>` component was **not actually used** by either persistent player; `PersistentPlayer.tsx`/`PersistentAudioPlayer.tsx` only imported `VideoPlayer.tsx`'s `getMediaStreamUrl` helper and rendered their own raw `<video>`/`<audio>` tags with a static direct-stream `src`, so assigning an `.m3u8` URL there would have silently failed to play in any non-Safari browser. Fixed as part of this feature:
  - Extracted the hls.js-attach logic out of `VideoPlayer.tsx` into a new reusable hook, `frontend/src/hooks/useHlsMediaSource.ts` (`useHlsMediaSource(mediaRef, src, onError)` — works for both `HTMLVideoElement` and `HTMLAudioElement` since hls.js attaches to any `HTMLMediaElement`), and refactored `VideoPlayer.tsx` to use it (net simplification, no behavior change there).
  - Added `resolveMediaPlaybackUrl(mediaFileId)` to `VideoPlayer.tsx`: fetches `/api/media/{id}/info`, returns the `/hls/playlist.m3u8` URL when `needs_hls` is true, else the direct `/stream` URL; falls back to direct-stream on any fetch/parse error so a transient `/info` failure never blocks playback.
  - Wired both into `PersistentPlayer.tsx` (video) and `PersistentAudioPlayer.tsx` (audio): each now seeds a `playbackSrc` state to the direct stream URL synchronously (so the common non-HLS case isn't delayed by the `/info` round trip), resolves the real decision asynchronously and swaps `playbackSrc` if it differs, and calls `useHlsMediaSource(ref, playbackSrc, ...)` instead of a JSX `src` attribute (mixing a React-controlled `src` prop with hls.js's own internal blob-URL assignment risked a spurious decode-error event firing before hls.js attaches — matching `VideoPlayer.tsx`'s already-imperative, JSX-`src`-free pattern avoids that).
  - **Casting is untouched by design**: `CastButton`/the cast pipeline keeps calling `getMediaStreamUrl` directly, per the plan's "keep the transcode decision client-specific, not global" instruction — cast receivers often have hardware HEVC decoders a browser tab lacks, so they should not be forced through the browser's transcode decision. New design.md Q entry documents this split.
  - `pnpm exec tsc --noEmit` is clean for all touched files.

Deferred to later agents:
- **The re-encode (`Transcode`) path is unexercised beyond its unit-tested arg vector.** Both `spawn_hls_remux` and `spawn_hls_transcode` are implemented and wired into `hls_playlist`'s decision branch, but per this environment's hard constraint (no spawning real ffmpeg against live media in this session), neither path has been run end-to-end against an actual file. Remux is the overwhelmingly common case (incompatible container, compatible codecs — most MKV rips); transcode (HEVC/DV) needs a real smoke test with `LIBRARIAN_DEV`/manual QA before being trusted in production.
- **No `mediaFile.needsTranscode`/`transcodeReason` GraphQL computed field** — the plan called this optional for MVP; the REST `media_info` JSON hint covers the only current consumer (the frontend players).
- **No proactive warm-up / pre-transcode** — a session only starts on the first `hls/playlist.m3u8` request, so the very first playback attempt for a file needing HLS pays the "wait for first segments" latency (bounded-polled, up to 20s) instead of an instant start. A future "pre-warm on demand" (e.g. kick off a session as soon as `media_info` reports `needsHls`, before the player actually requests the playlist) is a reasonable follow-on.
- **No per-file concurrency cap / global ffmpeg process limit** — if many different files are requested for HLS concurrently, each gets its own `ffmpeg` child with no ceiling on how many run at once. Fine for the expected single/few-concurrent-viewer local-first use case; a real multi-tenant deployment would want a semaphore.
- **Segment/playlist responses have no `Cache-Control`/ETag-based conditional GET support** beyond a flat `no-cache` (playlist) / `max-age=3600` (segments) — adequate for a VOD-style HLS session (segments never change once written) but not optimized.
- **Casting decision (`chromecast_compatible`) and the browser HLS decision (`needs_transcode`) share their underlying container/codec predicates but are computed independently** — if a future agent wants cast devices to *also* get HLS remux/transcode (today they only get direct-play via `chromecast_compatible`), that's new work, not something this change does.

---

## 5. Sequencing — within and across features

Implemented by separate agents in order: **auto-download → quality profiles → torznab → HLS**.

1. **Auto-download, phase A (foundation + minimal scorer)** — `backend/src/jobs/` skeleton, `AutoDownloadService`, candidate discovery across 4 media types, Torrent wanted-linkage columns, and a **minimal inline** parser/ranker (title-similarity + seeders + basic resolution compare). Ship with hardcoded reasonable defaults, don't block on user-configurable profiles.
2. **Quality profiles** — real `services/quality` module (parser + `is_upgrade` + `evaluate`), `QualityProfile` entity, `quality_status` column, settings UI (adapt the orphaned `QualitySettingsCard`). **Then retrofit auto-download's scorer** to the new `QualityProfile`-aware module, replacing the phase-A heuristic. Deliberate two-pass approach.
3. **Torznab client** — functionally independent; land after the quality parser so `<torznab:attr>` metadata can feed `ParsedRelease` (small win, not a hard dependency).
4. **HLS/FFmpeg** — fully independent subsystem; sequenced last per directive, could be parallel.

### Recommended build order
**Auto-download (minimal scorer) → Quality profiles (+ retrofit auto-download's scorer) → Torznab client → HLS transcoding.**

### Top cross-cutting risks
1. **The 0.65/0.70 episode auto-link confidence floor bug** (`library_scan.rs:4738,4192,5488,2888`) sits in the import path auto-download flows through. Confirm it's fixed before enabling the download monitor by default, or unattended automation mass-produces wrong-show links.
2. **No per-source HTTP timeouts / no `search_all` deadline** — the manager has a 45s per-source timeout, but the 5 scrapers' own clients lack timeouts. Auto-download (frequent) and Torznab (arbitrary third-party) amplify this.
3. **Unbounded resource growth without reaper loops**: HLS segment cache and auto-download retry state need bounded periodic cleanup from day one.
4. **Frontend/backend drift**: grep the frontend for expected-but-missing GraphQL fields before designing new entities (the `ShowSettingsModal`/`QualitySettingsCard` orphan is a concrete instance).
5. **Schema-sync destructiveness** (startup sync can `DROP` on rename, `graphql-orm migrations.rs:169,198`): name new columns carefully; treat renames as delete+re-add data-loss events, not casual refactors.

### Critical files
- `backend/src/services/manager.rs` — service registration/lifecycle every new background service follows
- `backend/src/services/library_scan.rs` — matching/import/organize pipeline grabs flow into; source of filename/quality regexes to refactor into the shared scorer
- `backend/src/services/sources/manager.rs` + `definitions/mod.rs` — where Torznab registers and how search dispatch works
- `backend/src/services/torrent/service.rs` — `add_magnet_with_metadata`/`process_completed_torrent` grab+import primitives
- `backend/src/services/graphql/entities/media_file.rs` (+ `torrent.rs`, `pending_file_match.rs`) — entities gaining columns
- `backend/src/api/media.rs` — direct-play streaming + new HLS routes
- `docs/design.md` — authoritative Q&A; every new decision point gets a new Q entry per `.cursor/rules/media-pipeline.mdc`
