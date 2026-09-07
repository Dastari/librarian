# Torrent & Sources Reliability — Phase 4b Progress

Owner scope: `backend/src/services/torrent/**`, `backend/src/services/sources/**`
(plus additive-only `entities/torrent.rs` for item 4, per instructions — turned out
unnecessary, see notes).
Do NOT touch `services/graphql/**`, `services/auth.rs`, `services/database.rs`.

Baseline: tree compiles, `cargo test` 64/64 passes (verified at start).

## Checklist

- [x] Item 1 — Completion events must never be lost
  - [x] 1a. Decouple: spawn import task on Completed event instead of sync inline await; dedupe in-flight by torrent id
  - [x] 1b. Reconciliation sweep loop (every 5 min) using existing loop/cancellation pattern; idempotent via post_process_status / existing guards
- [x] Item 2 — Timeouts everywhere in sources
  - [x] iptorrents.rs client timeout/connect_timeout
  - [x] 1337x.rs client timeout/connect_timeout
  - [x] limetorrents.rs client timeout/connect_timeout
  - [x] thepiratebay.rs client timeout/connect_timeout
  - [x] yts.rs client timeout/connect_timeout
  - [x] manager.rs search_all (+ search_sources): wrap each spawned search with tokio::time::timeout(45s), warn+empty on timeout
- [x] Item 3 — UPnP lease renewal
  - [x] Add renewal loop task (every 40 min) with cancellation, state-change-only logging
- [x] Item 4 — Stop the 10-second write storm
  - [x] find_torrent_by_info_hash / find_setting: filtered entity queries (info_hash and key were ALREADY #[filterable] — no entity edit needed)
  - [x] upsert_torrent_files: change detection instead of delete+reinsert every tick
  - [x] ensure_bulk_delete_success discarded-result bug: eliminated by removing the bulk delete+reinsert pattern entirely (replaced with checked per-row create/update/delete)
  - [x] per-torrent sync skips update when nothing changed (upsert_from_session change-detection)

## Verify
- [x] `cargo check` clean (whole crate, not just owned files)
- [x] `cargo test` all pass — 74 lib tests (baseline 52 + 5 new `plan_torrent_file_sync` tests + others added by parallel agent) + 12 contract tests = 86 total, 0 failed
- [x] `cargo clippy` — zero warnings in any touched file (all warnings present are pre-existing, in files outside this task's scope)

## Notes / findings log

- Entity check: `Torrent.info_hash`, `Torrent.progress`, `Torrent.post_process_status`,
  `AppSetting.key`, `TorrentFile.torrent_id` were all already `#[filterable]` in
  `services/graphql/entities/{torrent,app_setting,torrent_file}.rs`. Item 4's conditional
  entity edit was therefore not needed, and `services/graphql/**` was left untouched
  entirely (per file-ownership rule 5).
- `mark_completed()` and the old bulk `ensure_bulk_delete_success` helper became dead
  code after the refactor and were deleted (both had exactly one call site, both in
  this file).
- `TorrentService::process_completed_torrent` (manual reprocess API) now also fixes
  the audit's "reports false success on missing torrent" note as a side effect of
  sharing `process_completed_torrent_core` with the event handler and sweep — it
  returns an explicit `success: false` summary with a clear message instead of
  silently running with an empty file list.
- Compile note: partway through, `cargo check` failed on `src/services/graphql/mutations/auth.rs`
  (missing `invite_token` field on `RegisterInput`) — unrelated to any file in this
  task's scope (parallel agent's area). It resolved itself by the time of the final
  verification pass; not touched.
