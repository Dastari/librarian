# Durability & Infra Fixes — Progress (Phase 3b)

Owner scope: backend/src/db/mod.rs, backend/src/main.rs, backend/src/services/http_server.rs,
entity files EXCEPT media_file.rs, library_scan.rs, mutations/library_scan.rs (parallel agent owns those).

- [x] Item 1 — SQLite production configuration (WAL, synchronous, busy_timeout, foreign_keys, pool sizing) — db/mod.rs
- [x] Item 2 — FK/filter-column indexes on entity files (track, chapter, episode, movie, show, album, audiobook, torrent_file, playback_progress, notification, app_log)
- [x] Item 3 — SIGTERM handling in main.rs headless mode
- [x] Item 4 — Graceful HTTP shutdown in http_server.rs (with_graceful_shutdown + 10s hard timeout)

STATUS: DONE. Final verification (after the parallel agent's media_file.rs/library_scan.rs edits settled):
- `cargo check`: clean
- `cargo test`: 61/61 pass (49 unit + 12 migration-contract; baseline was 56 — the parallel agent
  added tests, zero failures). Schema lifecycle tests exercise bootstrap against the full entity list,
  which validates the new index attributes end-to-end.
- `cargo clippy`: no new warnings from these changes. The `let...else` warnings reported at the derive
  line of chapter/episode/movie/track come from the pre-existing GraphQLRelations macro expansion
  (graphql-orm-macros relations.rs:389, optional relation fields) — attributed to the derive span,
  unaffected by the added `index = ...` attributes. torrent.rs/library_scan.rs warnings are in files
  not touched by this task.

## Notes / decisions

### Item 1 (db/mod.rs)
- Added `is_in_memory_url()` helper: sqlx's `SqliteConnectOptions::get_filename()` doesn't
  literally equal ":memory:" for `sqlite::memory:` URLs (it becomes `file:sqlx-in-memory-N`
  internally, per sqlx-sqlite parse.rs), so detection is done on the raw URL string instead
  (`:memory:` / `mode=memory` substrings), before parsing.
- WAL + synchronous=NORMAL only applied for non-memory URLs. busy_timeout(30s) and
  foreign_keys(true) applied unconditionally (harmless for memory DBs).
- Switched from `DbPool::connect_with` (implicit `PoolOptions::new()` == max_connections 10,
  acquire_timeout 30s defaults already) to explicit `SqlitePoolOptions::new().max_connections(10).acquire_timeout(30s)`
  per instructions, even though the numeric values matched sqlx defaults already.
- Confirmed via grep that all existing tests connect directly with `DbPool::connect("sqlite::memory:")`,
  bypassing `connect_with_retry` entirely (services/database.rs tests, bootstrap_defaults/tests.rs,
  graphql/schema.rs) — so this change has zero effect on current test behavior, only production/from_config path.

### Item 3 (main.rs)
- Added `wait_for_shutdown_signal()` cfg-split helper; unix branch selects ctrl_c vs SIGTERM,
  non-unix (incl. windows, matching the `cfg(windows)` windows-service dep in Cargo.toml) falls
  back to ctrl_c only. Existing `services.stop_all().await` flow unchanged, called after either branch.

### Item 2 (entity index attributes)
Used the macro's plain `index = "col"` attribute (non-unique), sibling to the existing `unique_index`
(graphql-orm-macros entity.rs: `meta.path.is_ident("index") || meta.path.is_ident("unique_index")`).
Repeatable, one column or comma-separated composite per occurrence, all additive (schema bootstrap
validate->plan->apply picks these up automatically, non-destructive).

- track.rs: added `index = "album_id"`, `index = "artist_id"`, `index = "media_file_id"` (all 3 columns
  confirmed present on the struct; artist_id is `Option<String>`).
- chapter.rs: added `index = "audiobook_id"`, `index = "media_file_id"`.
- episode.rs: added `index = "media_file_id"` only. `show_id` was NOT added — it's already the leftmost
  column of 4 existing `unique_index = "show_id,..."` composites, so SQLite can use any of those as a
  covering/prefix index for show_id-only lookups; a 5th single-column index would be redundant.
- movie.rs: added `index = "library_id"`, `index = "collection_id"` (confirmed field name `collection_id: Option<i32>`
  on Movie itself — matches the audit's "CollectionId aggregation queries" note).
- show.rs: no change. `library_id` is already the leftmost column of 3 existing `unique_index` composites
  (library_id,tvmaze_id / tmdb_id / tvdb_id) — already covered, skipped per the audit's "check it isn't
  already covered" instruction.
- album.rs, audiobook.rs: added `index = "library_id"` to both — neither had any existing index/unique_index,
  so no leftmost-prefix coverage existed.
- torrent_file.rs: added `index = "torrent_id"`.
- playback_progress.rs: added `index = "user_id"`.
- notification.rs: added `index = "user_id"`.
- app_log.rs: added `index = "timestamp"`, `index = "target"`, `index = "level"`. Verified against actual
  usage in frontend/src/routes/settings/logs.tsx + frontend/src/lib/graphql/documents/logs.graphql:
  filters by `target: { eq }`, supports server-side `orderBy` on all three of timestamp/level/target, and
  `deleteAppLogs` (retention cleanup) filters by `timestamp: { lt }`.

### Item 4 (http_server.rs)
- Replaced `broadcast::channel` + manual `select!` abandon-on-shutdown with
  `axum::serve(...).with_graceful_shutdown(oneshot_rx)`. Switched shutdown_tx from
  broadcast to oneshot (matches existing style in services/logging.rs, and only one
  producer/consumer exists — no other file referenced shutdown_tx externally, confirmed by grep).
  stop() sends the oneshot, then races `h.await` against a 10s timeout; on timeout, logs
  a warning and calls `abort_handle.abort()` so the task doesn't leak in the background
  (tokio::time::timeout would otherwise just drop our await, leaving the spawned task running
  detached and the port bound).
