# Phase 3a — Pipeline Safety Fixes — Progress

Owner scope: backend/src/services/library_scan.rs,
backend/src/services/graphql/entities/media_file.rs,
backend/src/services/graphql/mutations/library_scan.rs

Baseline: tree compiles, `cargo test` 56/56 passes (before this work).

## Checklist

- [x] Item 1 — Remove dangerous episode score floor (audit #3)
- [x] Item 2 — Confidence floors on provider-fallback creation (episode/track/chapter, audit HIGH)
- [x] Item 3 — Manual-match protection (audit #5, design.md Q9b)
- [x] Item 4 — Safe file operations in organize + materialize_torrent_file (audit #4/#5)
- [x] Final verify: cargo check clean; cargo test 64 pass / 0 fail (52 unit + 12 contract)

## Recovery note (session moved machines)
The Phase 3a agent's in-process state was lost when the process exited mid-Item-4.
Verified against disk on resume: Item 4 had fully landed (no_clobber_place +
no_clobber_place_blocking + cross_device_no_clobber_place helpers at ~:8557, wired into
organize move :6135, DB-failure move-back :6243, torrent materialize :6963; relativePath
staleness fixed :6180; EEXIST->copy overwrite hole closed) WITH its 3 tokio::test unit tests
(no_clobber_place_move_succeeds_and_removes_source, _copy_keeps_source,
_never_overwrites_existing_target at ~:9219). Only the checklist update + final verify were
outstanding; both now done. cargo check clean, 64/0 tests.

## Log

### Item 1 (done)
- `collect_episode_candidates` (~:4738 orig) and `find_episode_match` (~:2888 orig) both had
  `(jaro_winkler(..) * 0.9).max(0.65)`. Removed the `.max(0.65)` floor in both, factored the
  shared scoring formula into a new pure helper `score_episode_show_candidate` (near
  `normalize_for_match`, ~:2413) used by both call sites.
- `find_episode_match` previously returned `best` with no threshold at all — now filters
  `score >= 0.70` before returning, matching the auto-link threshold used in `match_media_file`.
- Audited other scorers for the same floor pattern (`grep '.max(0.6'`): only the two episode
  sites matched. Found one unrelated `.max(0.7)` at ~:3509 in `try_provider_create_and_match_movie`
  — but that's applied to an already-linked movie purely for the logged/returned confidence value
  (the actual gating already happened earlier via `candidate_score < 0.78`), so it's not the same
  bug and was left untouched per instructions ("only change episode ones unless another is clearly
  the same bug").
- Added tests (see Item 1 tests below, added after Item 2 work in the same test-writing pass).

### Item 2 (done)
- Added `candidate_score < 0.78` gates (mirroring the movie variant at ~:3447) to:
  - `try_provider_create_and_match_episode` — before `add_tv_show_from_provider`
  - `try_provider_create_and_match_track` — before `add_album_from_provider`
  - `try_provider_create_and_match_chapter` — before `add_audiobook_from_provider`
- Restructured each `max_by` closure into `.map(|c| (c, score)).max_by(|(_,a),(_,b)| ...)` so the
  winning score is available for the threshold check, same shape as the movie variant.
- Rejections log at `debug!` (tracing::debug already imported) and return `Ok(None)`, same as "no
  provider result".
- `cargo check` clean after this item (one transient error was actually from the parallel agent's
  concurrent edit to http_server.rs — resolved itself on retry, not caused by my changes).

### Item 3 (done)
- Entity: added `matchType: Option<String>`, `matchedByUserId: Option<String>`,
  `matchConfirmedAt: Option<String>` to `entities/media_file.rs`. Used `Option<String>` (not
  `chrono::DateTime<Utc>`) for the timestamp because every other timestamp field on this entity
  (`addedAt`, `analyzedAt`) is a `String` (ISO-8601, `%Y-%m-%dT%H:%M:%S%.3fZ`, formatted the same
  way `library_scan.rs` already does at `chrono::Utc::now().format(...)`), per the instruction to
  match this entity's existing timestamp convention.
- `apply_media_file_match_update` (private helper backing all of `link_movie/episode/track/chapter`)
  now takes `match_type: &str` ("manual"|"auto") and `matched_by_user_id: Option<&str>`; it sets
  `matchConfirmedAt` to now() only when `match_type == "manual"`. Threaded through all ~18
  `link_*` call sites in the file: explicit-id branches in `match_media_file` pass
  `("manual", request.requested_by_user_id.as_deref())`; every ranked-candidate
  (`apply_link_for_candidate`) and provider-fallback (`try_provider_create_and_match_*`, plus the
  grouped-sibling movie linking in the deferred-fallback path) call site passes `("auto", None)`.
  Torrent-import's direct `apply_media_file_match_update` call (bypasses `match_media_file`) also
  passes `("auto", None)`.
- Added `MatchRequest.requested_by_user_id: Option<String>` (Default-derived, so the two internal
  callers using `..Default::default()` are unaffected/None). `mutations/library_scan.rs`
  `match_media_file` resolver now captures `ctx.require_member()?.clone()` and passes
  `Some(auth_user.user_id)` — previously the auth user was checked but discarded.
- Guard: added `LibraryScanService::should_block_automatic_match(is_explicit_target, match_type)`
  pure function (`!is_explicit_target && match_type == Some("manual")`), called in
  `match_media_file` right after `existing_link`/`match_source` are computed and before the
  existing "already matched, use force" branch. When true, returns
  `MatchResult { success: false, already_matched: true, .. }` with an explanatory reason and logs
  a `warn!`. Explicit-id requests always bypass the guard (user intent may replace a manual match).
  `MediaFileRow` and both `get_media_file`/`get_media_file_by_path` GraphQL queries now fetch
  `matchType` so the guard has the data it needs.
- Torrent-import path (`process_torrent_source_files_inner`) had its own separate write path via
  `apply_media_file_match_update` that does NOT go through `match_media_file`'s guard (it only
  calls `match_media_file` earlier for *ranking*, with `auto_match:false`, so nothing is applied
  there). Added an explicit `match_type == "manual"` check right after `ensure_source_media_file`
  resolves `media_file_id` (which can resolve to a pre-existing row keyed by source path), skipping
  the file with a message into `messages` if manually matched, before doing any further work.
- `unmatch_media_file` now also nulls `matchType`/`matchedByUserId`/`matchConfirmedAt` in its
  `updateMediaFile` mutation input, so unmatch → rematch works cleanly.
- Deviation forced by the entity change: `services/database.rs` (not in my owned file list, but a
  downstream consumer) has a `#[cfg(test)]` helper `create_media_file` that builds a full
  `CreateMediaFileInput` literal without `..Default::default()`; adding the 3 entity fields made
  it fail to compile (`E0063`). Fixed with a 3-line addition (`match_type: None,
  matched_by_user_id: None, match_confirmed_at: None`) — the minimal change required to keep the
  tree compiling; nothing else in that file was touched.
- Tests added (pure-function, no DB needed, following the file's existing test style):
  `manual_match_guard_blocks_automatic_rematch_of_manual_match`,
  `manual_match_guard_allows_explicit_target_to_replace_manual_match`,
  `manual_match_guard_allows_automatic_rematch_of_auto_or_unmatched_file`.
- `cargo test` after this item: 49 unit + 12 contract = 61 passing (56 baseline + 5 new tests from
  items 1–3 so far).
