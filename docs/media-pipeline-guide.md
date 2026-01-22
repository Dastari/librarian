# Media Pipeline Decision Guide

This document defines the decision points in the media processing pipeline and what the code currently does (or should do) for each scenario. Reference this guide when making changes to any media pipeline components.

---

## Core Principles

1. **Source-agnostic matching** - The same matching logic handles files from torrents, usenet, IRC, FTP, or library scans
2. **Always COPY, never move** - Files are always copied from download folders to library folders
3. **Library owns files** - Unlinking a download source never affects library files
4. **Quality is verified, not assumed** - Every file is analyzed with FFprobe to determine true quality
5. **No auto-delete files** - Move conflicts to a designated folder, never auto-delete user files
6. **Partial fulfillment is OK** - Downloading 8 of 12 album tracks is valid; remaining 4 stay "wanted"
7. **Status reflects reality** - "downloading" means in download queue, "downloaded" means file in library folder
8. **Use what exists now** - If a file exists NOW which is a better match, use it rather than waiting for a download

---

## Phase 1: File Matching

### Q1: What do we do when we find a file match for a media file that is in a download state but hasn't finished downloading yet?
**Answer:** We use what we have now. If a file exists NOW which is a better match for any potential future match (even one that may be downloading), we should use what we have. The scanner processes files that exist on disk; it doesn't wait for downloads.

### Q2: What happens when a file matches multiple items across different libraries?
**Answer:** The FileMatcher returns all matches (can match to items in multiple libraries). Each match is saved as a separate `pending_file_matches` record. When processed, the file is copied to each matching library.

### Q3: What happens when a file matches an item that's already in `downloaded` status?
**Answer:** For tracks, we check if status is `wanted` or `missing` and skip if already `downloaded`. For albums/movies/audiobooks with known targets from auto-hunt, we use 100% confidence and proceed even if status isn't wanted.

### Q4: How do we handle sample files (trailers, previews)?
**Answer:** If filename contains "sample", "trailer", or "preview" → mark as `Sample` type and don't process further. These are excluded from matching.

### Q5: How does the weighted fuzzy matching work?
**Answer:** Uses `match_scorer.rs` with field-specific weights and proportional scoring (not thresholds). Each field contributes proportionally based on fuzzy similarity:

**Music (100 points max):**
| Field | Weight | Scoring |
|-------|--------|---------|
| Artist | 30 | 80% match = 24 points |
| Album | 25 | 90% match = 22.5 points |
| Track Title | 25 | Proportional to similarity |
| Track Number | 15 | Exact match only (0 or 15) |
| Year | 5 | ±1 year tolerance |

**TV Shows (100 points max):**
| Field | Weight | Scoring |
|-------|--------|---------|
| Show Name | 35 | Proportional to similarity |
| Season | 25 | Exact match only |
| Episode | 25 | Exact match only |
| Episode Title | 15 | Bonus if matches |

**Movies (100 points max):**
| Field | Weight | Scoring |
|-------|--------|---------|
| Title | 50 | Proportional to similarity |
| Year | 40 | Exact or ±1 year |
| Director | 10 | Bonus if matches |

**Audiobooks (100 points max):**
| Field | Weight | Scoring |
|-------|--------|---------|
| Author | 30 | Proportional to similarity |
| Book Title | 30 | Proportional to similarity |
| Chapter Title | 20 | Proportional to similarity |
| Chapter Number | 20 | Exact match only |

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
**Answer:** Return an `Unmatched` result with a reason string. The file is saved as an unmatched `pending_file_match` which can be manually matched later via the UI.

### Q9: How do we determine if a file is video vs audio?
**Answer:** File extension check:
- Video extensions: `.mp4`, `.mkv`, `.avi`, `.mov`, `.wmv`, `.flv`, `.webm`, `.m4v`, `.ts`
- Audio extensions: `.mp3`, `.flac`, `.m4a`, `.aac`, `.ogg`, `.opus`, `.wav`, `.wma`

---

## Phase 2: Download Processing

### Q10: When do we copy files vs move files?
**Answer:** Always COPY from download folder to library folder (never move). This preserves source files for seeding. Within the library folder, we can move/rename for organization.

### Q11: What happens if the source file doesn't exist when we try to process a match?
**Answer:** Return an error "Source file does not exist". The match is marked as failed with `copy_error` set in the database.

### Q12: What happens if the destination file already exists?
**Answer:** 
- If existing file has different size → conflict detected → move existing to conflicts folder
- If existing file has same size → check if another DB record points to it → if yes, delete source (duplicate) → if no, update DB record to new path

### Q13: What happens if creating the destination directory fails?
**Answer:** The error propagates and the match processing fails. No partial state is created.

### Q14: How do we handle cross-filesystem copies?
**Answer:** Try `rename()` first (fast). If it fails with cross-device error, fall back to `copy()` then `delete()`.

### Q15: What happens when a hardlink operation fails?
**Answer:** Fall back to regular copy. Hardlinks only work on Unix and require same filesystem.

### Q16: What status do we set after processing a torrent's files?
**Answer:**
- If `files_failed == 0` AND `files_processed > 0` → `completed`
- If `files_processed > 0` but some failed → `partial`
- If `files_processed == 0` → `unmatched`

---

## Phase 3: Library Scanning

### Q17: What happens if we try to scan a library that's already being scanned?
**Answer:** Return early without starting a new scan. The `library.scanning` flag is checked first.

### Q18: What happens if the library path doesn't exist on disk?
**Answer:** Log a warning and return. No error is thrown; the scan just doesn't happen.

### Q19: How do we handle auto_add_discovered when scanning?
**Answer:** 
- If enabled and file matches no existing item → create new item from file metadata (search TVMaze/TMDB/MusicBrainz)
- If disabled → just add files without creating new library entries

### Q20: What happens when metadata lookup fails (TVMaze/TMDB/etc.)?
**Answer:** For movies, retry search without the year. If still no results, the file remains unmatched. The item is NOT created if we can't find metadata.

### Q21: What happens when a file is already in the database during a scan?
**Answer:** Check if it needs linking to an item. If already linked → skip. If not linked but matches an item → link it. This prevents duplicate processing.

### Q22: Do we trigger auto-hunt after scanning?
**Answer:** Yes, if the library has `auto_hunt` enabled OR if TV library has shows with `auto_hunt_override=true`. Runs in background after scan completes.

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
**Answer:** Set `quality_status = "suboptimal"` on the media_file AND update the item status:
- Episode → status = `suboptimal`
- Movie → `download_status = "suboptimal"`

The system does NOT automatically trigger an upgrade hunt.

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
**Answer:** After library scan completes, if `organize_files` is enabled on the library. Also after downloads complete if configured.

### Q28: What if a show has organize_files_override = false?
**Answer:** Skip organizing that show's episodes even if the library has organization enabled.

### Q29: What naming pattern do we use if none is configured?
**Answer:** Fall back to default patterns:
- TV: `Show Name - S01E01 - Episode Title.ext`
- Movies: Keep original filename
- Music: Uses music naming pattern from library

### Q30: How do we handle files that are already organized correctly?
**Answer:** If `original_path == new_path` → skip organization (no-op). Don't move file to itself.

---

## Phase 6: Torrent/Download Events

### Q31: What happens when a torrent is added?
**Answer:** 
1. Spawn async task to match files immediately
2. Skip if matches already exist (created by auto-hunt to avoid duplicates)
3. Save matches to `pending_file_matches`
4. Update matched items to `downloading` status
5. Set `active_download_id` on items

### Q32: What happens when a torrent completes?
**Answer:**
1. Acquire semaphore permit (max 3 concurrent)
2. If permit unavailable → skip (will be processed by scheduled job later)
3. Run match verification to correct mismatches
4. Process all uncopied matches (copy files to library)
5. Update torrent `post_process_status`

### Q33: What happens if torrent completion processing fails?
**Answer:** Log warning and continue. The scheduled download_monitor job will retry processing later.

### Q34: How often does the download monitor retry unmatched torrents?
**Answer:** Filters for torrents created within last 7 days that have no pending matches. This effectively limits retry window to 7 days.

---

## Phase 7: Match Verification

### Q35: When do we verify matches after download?
**Answer:** After torrent completes but before processing. Uses `verify_matches_with_metadata()` to check if embedded metadata contradicts the match.

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
4. Re-run matching using `FileMatcher.match_media_file()`
5. If new match found with score ≥ 70 → auto-correct:
   - Update `media_files.track_id` to new track
   - Update old track's `media_file_id` to NULL, status to 'wanted'
   - Update new track's `media_file_id` to file, status to 'downloaded'
6. Clear any stale bidirectional references (other tracks pointing to same file)

### Q36c: How do we ensure bidirectional link consistency?
**Answer:** After successful verification, explicitly clean up:
```sql
-- Clear any OTHER tracks incorrectly pointing to this file
UPDATE tracks SET media_file_id = NULL, status = 'wanted'
WHERE media_file_id = $file_id AND id != $correct_track_id;

-- Ensure correct track points to this file
UPDATE tracks SET media_file_id = $file_id, status = 'downloaded'
WHERE id = $correct_track_id;
```

## Phase 7b: Metadata Extraction

### Q36d: When is metadata extracted from files?
**Answer:**
1. **After download completion** - Queued as background job via `queues.rs`
2. **During library scan** - For files without `metadata_extracted_at` timestamp
3. **Manual trigger** - Via `extractMediaFileMetadata` GraphQL mutation

### Q36e: What metadata is extracted and stored?
**Answer:** Stored in `media_files` table:
- **Audio**: Artist, album, title, track number, disc number, year, genre
- **Video**: Show name, season, episode (from container metadata)
- **Album art**: Base64-encoded cover image with MIME type
- **Lyrics**: Extracted from FLAC/MP3 tags if present
- **Timestamps**: `ffprobe_analyzed_at`, `metadata_extracted_at`, `matched_at`

### Q36f: What tools are used for metadata extraction?
**Answer:**
- **FFprobe** - Video analysis (resolution, codecs, chapters, container metadata)
- **lofty** - Audio tag reading (ID3, Vorbis, FLAC, MP3, etc.) including album art and lyrics
- Extraction happens in `queues.rs` via `extract_audio_metadata_with_art()`

---

## Phase 8: Edge Cases & Resolved Decisions

These scenarios have been explicitly decided and should be implemented accordingly.

### Q37: What happens when an item has multiple pending downloads?
**Answer:** Prefer the newer/better quality download. When a second download matches the same item:
1. Compare quality of both downloads (parsed from filename)
2. Keep the higher quality one as `active_download_id`
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
1. Set a flag on the show/album/audiobook: `hunt_individual_items = true`
2. When auto-hunt runs next, search for individual missing episodes/tracks/chapters
3. Do NOT search for the complete album/show again (avoid re-downloading the same partial release)
4. This flag is set when: download completes AND some items matched AND some items still "wanted"

### Q40: What happens when matching fails repeatedly for the same file?
**Answer:** Retain the unmatched file but notify the user. The system should:
1. Keep unmatched files in `pending_file_matches` indefinitely
2. After N failed match attempts (configurable, default 3), create a user notification
3. Notification should link to the file for manual matching
4. Do NOT auto-delete unmatched files

### Q41: What do we do with files that fail processing multiple times?
**Answer:** Alert the user after X failures. The system should:
1. Track `copy_attempts` count on `pending_file_matches`
2. Retry on each download monitor run
3. After X failures (configurable, default 3), create a user notification
4. Notification should include the error message and options to retry or dismiss

### Q42: How do we handle archive extraction failures?
**Answer:** Alert the user and wait for manual intervention. The system should:
1. Log the extraction error
2. Mark the torrent/download with `extraction_failed = true`
3. Create a user notification explaining the failure
4. Do NOT retry automatically (user may need to install `unrar`/`7z`)
5. User can trigger manual retry after fixing the issue

### Q43: What happens when library storage is full?
**Answer:** Track free space and alert proactively. The system should:
1. Track disk free space in server status (exposed via GraphQL for frontend)
2. Create user notification when space drops below threshold (configurable, default 10GB)
3. If copy fails due to ENOSPC, create urgent notification
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
2. This cascades to delete `pending_file_matches` via FK
3. This clears `active_download_id` on items (already handled by CASCADE)
4. Torrents remain but become "unlinked" (no library association)
5. Downloaded files in the library folder are NOT deleted (library owns files, user must delete manually)

### Q48: How do we handle files with non-ASCII characters in names?
**Answer:** Sanitize to ASCII-safe names when organizing, but only if library has `organize_files` enabled. The system should:
1. If `organize_files = false` → preserve original filename exactly (we can't modify torrent files anyway)
2. If `organize_files = true` → sanitize non-ASCII to closest ASCII equivalent or underscore
3. Use a transliteration library (e.g., `unidecode`) for intelligent conversion
4. Preserve the original filename in `media_files.original_name` for reference

### Q49: What's the behavior when FFprobe analysis fails?
**Answer:** Log as warning and continue. The system should:
1. Log the FFprobe error as a warning (not error, not notification)
2. Mark file as having unknown quality (`quality_status = null`)
3. Continue with filename-parsed quality info for matching decisions
4. Matching is independent of FFprobe; FFprobe just provides quality verification hints
5. Do NOT create user notification for FFprobe failures (too noisy)

### Q50: How do we handle season packs vs individual episodes?
**Answer:** Never hunt for TV show packs, but support them if downloaded. The system should:
1. Auto-hunt should search for individual episodes only
2. If user manually downloads a season pack, file matching handles it automatically
3. Each file in the pack is matched individually to episodes
4. Partial packs work fine (matched episodes get processed, unmatched stay as files)
5. No special "pack scoring" logic needed

### Q51: What happens when a user manually changes a file on disk?
**Answer:** Require manual re-scan to detect changes. The system should:
1. No real-time file watching (too resource intensive)
2. User triggers library scan to detect changes
3. Scanner compares file paths and sizes to database
4. Missing files → unlink from items, update status to "wanted"
5. New files → match and link as usual
6. Modified files (same path, different size) → re-analyze with FFprobe

---

## Key Files Reference

| Component | File |
|-----------|------|
| File Matching | `backend/src/services/file_matcher.rs` |
| Weighted Scoring | `backend/src/services/match_scorer.rs` |
| File Processing | `backend/src/services/file_processor.rs` |
| Library Scanning | `backend/src/services/scanner.rs` |
| Quality Evaluation | `backend/src/services/quality_evaluator.rs` |
| Organization | `backend/src/services/organizer.rs` |
| Download Monitor | `backend/src/jobs/download_monitor.rs` |
| Torrent Events | `backend/src/services/torrent_completion_handler.rs` |
| Filename Parsing | `backend/src/services/filename_parser.rs` |
| Background Jobs | `backend/src/services/queues.rs` |
| Media Files DB | `backend/src/db/media_files.rs` |

---

## Status Values Reference

| Status | Meaning |
|--------|---------|
| `missing` | No file exists, hasn't aired yet (for episodes) |
| `wanted` | Aired/released, no file, actively looking |
| `downloading` | Matched to a pending download |
| `downloaded` | File exists in library folder |
| `suboptimal` | Has file but below quality target |
| `ignored` | User explicitly skipped |

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

Progress counts are calculated dynamically via SQL subqueries:

```sql
-- Albums
(SELECT COUNT(*)::int FROM tracks t WHERE t.album_id = a.id AND t.status = 'downloaded') as downloaded_track_count

-- TV Shows (already existed)
episode_file_count, episode_count

-- Audiobooks
(SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id AND c.status = 'downloaded') as downloaded_chapter_count
```
