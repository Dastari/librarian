# Legacy Code Archive

This directory contains code and artifacts detached from the active backend implementation.

## Purpose

- Keep the active `/backend` tree focused on current, supported code paths.
- Preserve prior implementations for reference and migration work.

## Contents

- `legacy/backend/src/services/legacy/`
  - Full legacy backend services tree moved out of `backend/src/services/legacy`.
  - Includes historical scanner/matcher/organizer/jobs/indexer implementations.

- `legacy/backend/src/services/metadata/old_providers.rs`
  - Historical metadata provider implementation that was fully commented/unused.

- `legacy/backend/commented_snapshots/`
  - Snapshots of backend files before large commented legacy blocks were removed:
    - `movie.rs`
    - `album.rs`
    - `audiobook.rs`
    - `rss_feed.rs`
    - `torrent.rs`
    - `services_mod.rs`

## Notes

- These files are intentionally **not** part of active backend module wiring.
- If a legacy implementation needs revival, copy specific logic back into active services/mutations with current architecture standards.
