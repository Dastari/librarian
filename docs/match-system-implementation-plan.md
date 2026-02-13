# Match System Implementation Plan

Last updated: 2026-02-11

## Purpose
Implement a unified, score-based media-file matching service that can:
- match a `MediaFile` to `Episode`/`Movie`/`Track`/`Chapter`
- support user-guided constraints (`LibraryType`, `LibraryId`, explicit target IDs)
- return ranked contenders (top-N) even when auto-match is enabled
- support force rematch behavior
- reuse one backend matching engine across scan/manual/source rematch workflows

This plan is based on:
- `docs/design.md`
- current active implementation in `backend/src/services/library_scan.rs`
- legacy matcher/scorer references in `legacy/backend/src/services/legacy/`

## Current State
- `MatchMediaFile` already exists in `backend/src/services/graphql/mutations/library_scan.rs`.
- Main matching code already exists in `backend/src/services/library_scan.rs` (`match_media_file`, `find_*_match`, provider fallback methods).
- `ScanLibrary` already invokes matching per discovered file.
- Manual matching by explicit IDs already works.
- Matching is currently single-best-result oriented, not contender-list oriented.

## Gaps Against Target
- No ranked top contenders returned in API.
- No `Force` rematch input handling.
- No explicit `AutoMatch` vs preview mode.
- No explicit library-type scoped matching input.
- No multi-library scope input in one request.
- ffprobe analysis is queued, but matching does not wait for analyze completion.
- Provider fallback is currently coupled into same path, not cleanly separated.

## Architecture Direction
- Keep matching logic centralized in one engine/module and called from:
  - `ScanLibrary`
  - `MatchMediaFile`
  - future `RematchSource`
- Keep GraphQL entity operations as the required data path (`execute_graphql` / `execute_mutation`), no new direct SQL for domain matching logic.
- Keep matching deterministic and score-based.

## Data Model Note: `Chapter` vs `MediaChapter`
- `Chapter` (`backend/src/services/graphql/entities/chapter.rs`) is the domain audiobook chapter entity and has `MediaFileId` for file linkage.
- `MediaChapter` (`backend/src/services/graphql/entities/media_chapter.rs`) stores ffprobe-discovered chapter markers inside a media container (`media_chapters` table). It is not the same domain concept as audiobook `Chapter`.

Conclusion:
- `MediaChapter` is not a duplicate of `Chapter`.
- It is correct to keep both entities.

## Recommendation: `MediaFile.ChapterId`
Current model has reverse pointer fields on `MediaFile` for `EpisodeId`/`MovieId`/`TrackId`, but not `ChapterId`.

Recommended for consistency with existing pattern:
- add `MediaFile.ChapterId` so unmatched queries and generic matching logic can behave uniformly across all 4 content types
- keep `Chapter.MediaFileId` as well (existing)
- enforce synchronization in linker/unlinker logic to avoid drift

Alternative (normalized-only) option:
- do not add `MediaFile.ChapterId`, and always resolve chapter linkage through `Chapter.Where(MediaFileId=...)`
- this avoids duplicate linkage state, but diverges from existing media_file pattern and complicates generic unmatched filters

Given current codebase conventions, the first option is lower-friction.

## API Changes
Extend `MatchMediaFileInput` (backward compatible):
- `MediaFileId: String!`
- `LibraryId: String` (existing)
- `LibraryIds: [String!]`
- `LibraryTypes: [String!]` (or enum)
- `EpisodeId: String` (existing)
- `MovieId: String` (existing)
- `TrackId: String` (existing)
- `ChapterId: String` (existing)
- `Force: Boolean = false`
- `AutoMatch: Boolean = true`
- `CandidateLimit: Int = 10`
- `RequireAnalyzed: Boolean = true`
- `AllowProviderFallback: Boolean = false`
- `Methods: [MatchMethod!]` (existing)

Extend `MatchMediaFileResult`:
- `Success: Boolean!`
- `AutoMatched: Boolean!`
- `AlreadyMatched: Boolean!`
- `MatchedType: String`
- `MatchedId: String`
- `Confidence: Float!`
- `Reason: String`
- `Candidates: [MatchCandidate!]!`
- `AnalysisQueued: Boolean!`
- `AnalysisWaited: Boolean!`
- `AnalysisTimedOut: Boolean!`

Add `MatchCandidate` type:
- `TargetType`, `TargetId`, `LibraryId`, `LibraryType`
- `Score` (0..1)
- `Reason`
- optional `ScoreBreakdown` payload for UI/debug

## Matching Engine Design
- Build all candidates in scope for relevant content type(s).
- Score candidates using weighted strategy inspired by legacy `match_scorer`:
  - music: artist/album/title/track/disc/year
  - tv: show/season/episode/title/year
  - movie: title/year/(optional director)
  - audiobook: author/book/chapter title/chapter number
- Include filename-derived hints and analyzed metadata when available.
- Sort candidates by score descending.
- Return top-N candidates.
- Auto-link only when `AutoMatch=true` and score policy passes.

## Score Policy
- Keep score in 0..1 externally.
- Internally allow weighted points and normalize.
- Use two thresholds:
  - suggest threshold (candidate appears in list)
  - auto-link threshold (can link automatically)
- Add tie/margin guard (avoid auto-link when top two are too close).

## Force Rematch Behavior
- If existing link exists and `Force=false`:
  - do not mutate link
  - return `AlreadyMatched=true` and ranked candidates
- If `Force=true`:
  - clear old link(s)
  - apply selected/auto link if available
  - keep status fields (`Wanted`, `HasFile`) synchronized via mutation path

## ffprobe/Analyze Behavior
- If `RequireAnalyzed=true` and file not analyzed:
  - queue analyze
  - wait for completion until timeout
  - proceed with best available data if timeout
  - set analysis flags in result

## Provider Fallback Separation
- Keep metadata provider create-and-link logic behind `AllowProviderFallback`.
- Default off for direct `MatchMediaFile`.
- Keep scan flow configurable for provider fallback as a distinct stage.

## Implementation Phases
1. Extract matcher internals from `LibraryScanService` into a dedicated engine module.
2. Add candidate model and scoring pipeline, return top-N without changing old fields.
3. Add new input flags (`Force`, `AutoMatch`, scope filters, analyze options).
4. Add `MediaFile.ChapterId` (if adopting consistency option), wire link/unlink sync.
5. Update unmatched queries/UI to include chapter-linked files correctly.
6. Add tests (unit + integration).
7. Wire `RematchSource` to shared matcher engine.

## Testing Plan
- Unit tests:
  - parsing hints (movie/tv/music/audiobook)
  - weighted scoring and threshold behavior
  - tie-break behavior
- Integration tests:
  - explicit target ID override
  - force vs non-force
  - scope by `LibraryId`/`LibraryTypes`/multi-library
  - chapter matching parity with movie/episode/track
  - top-N ordering stability
  - analyze-required path with timeout

## Migration/Compatibility Notes
- Keep existing fields in `MatchMediaFileResult` to avoid frontend break.
- Add new fields as additive GraphQL changes.
- If `MediaFile.ChapterId` is added, include schema sync/migration and backfill job from `Chapter.MediaFileId`.

## Open Decisions
- Final enum set for `LibraryTypes` input (string vs enum in GraphQL).
- Exact thresholds per media type.
- Whether `AllowProviderFallback` should ever default true in scan context.
- Whether to persist candidate history for audit/debug (optional, later).

## Implemented (Current Session)
- Added explicit wanted matching policy control to backend matcher:
  - service enum: `MatchWantedPolicy` with `PreferWanted` (default), `WantedOnly`, `All`
  - GraphQL enum/input field: `MatchWantedPolicy`, `WantedPolicy`
- Wired policy through `MatchMediaFileInput -> MatchRequest -> candidate collection`.
- Enforced policy consistently for `Movie`, `Episode`, `Track`, and `Chapter` candidate collection:
  - `WantedOnly`: exclude non-wanted candidates entirely
  - `PreferWanted`: include all, but boost wanted and penalize non-wanted-with-existing-file
  - `All`: include all without wanted bias
- Scan flow default kept at `PreferWanted` so scanner still prioritizes wanted gaps but can match non-wanted media.
- Added matcher unit tests for wanted-policy include/filter and score-adjust behavior.

## Product Logic Decision (Current Default)
- Matching should run against all candidates by default (`PreferWanted`) rather than only wanted items.
- `wanted` should influence prioritization, not hard eligibility, unless explicitly requested via `WantedOnly`.
- Organization and "adopt into library" decisions remain a separate step/policy from pure match detection.
