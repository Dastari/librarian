# Frontend audit — 5 September 2026

Audited the authenticated application at https://librarian.dastari.net using the shared admin browser, the existing Movies library, browser network observations, generated GraphQL reads, and backend logs. This complements the dependency and feature-gap reviews from the same date. It is not a claim that every planned feature is complete.

## Fixed during this audit

| Problem | Change and verification |
| --- | --- |
| Collection poster returned HTTP 500 | Storage metadata now accepts both ORM Unix-second timestamps and RFC3339 timestamps. The exact reported `/api/artwork/collection/192492/poster` returned HTTP 200, `image/jpeg`, 71,621 bytes. Three regression tests cover legacy metadata, RFC3339, and invalid values. |
| Dashboard said there was no media despite four movies | Added a generated recent-media query for movies, shows, albums, and audiobooks; merged and sorted by creation time. Versioned the dashboard cache. All four existing movies now appear on Home. |
| Table headings, filter tabs, and primary actions disappeared | The adapter now renders application headings/actions/filters independently of the upstream table toolbar. Downloads' add action, library letter filters, notification tabs, and collection headings are visible. Custom search functions are forwarded. Disabled pagination no longer silently truncates to ten rows. |
| Empty tables were too wide on narrow screens | Empty datasets render an ordinary centered empty state instead of a wide table with off-screen text. Confirmed the Downloads empty state visually on a narrow viewport. |
| Notifications showed Invalid Date and only exposed the first 50 records | Shared timestamp parsing handles Unix seconds and ISO timestamps, including notification details. Added explicit server paging and a separate unread-total query. Verified pages 1–50 and 51–100 of 400; Unread resets to page one and reports 390. The navbar's 696 includes unresolved scan issues as well as unread notifications. |
| Mobile library/settings navigation consumed the content area | Both layouts switch to horizontally scrollable section navigation on narrow screens. Library cards and settings content are visible below it. |
| Backup was missing from navigation and rendered nested settings sidebars | Added the Backup link, removed the duplicate layout, corrected snapshot column accessors, and made the lack of an in-app restore workflow explicit. |
| HLS playback never started while a movie was encoding | Changed FFmpeg playlists from VOD to EVENT so segments are published progressively; constrained encoder threads, emitted browser-compatible pixel format, and aligned transcode keyframes. A synthetic FFmpeg check confirmed a playable playlist before the encoder exits and ENDLIST after completion. |
| Player repeatedly recreated the media connection on rerender | Stabilized the HLS effect while retaining the latest error callback; added a regression test. Persistent video playback resolves the appropriate format before attaching it and displays a recoverable error instead of an indefinite spinner on fatal failure. |
| Growing HLS playlist length was used as full movie duration | Preserve the full duration from `/api/media/.../info` for display and progress persistence. The actual movie played, paused, and sought within available segments; the UI showed 2:15:07 and saved progress retained duration 8,107 seconds with `isWatched: false`. |
| Expired display metadata prevented refresh attempts after a sleeping tab resumed | Keep non-secret expiry metadata for the refresh window and allow existing user metadata to trigger refresh when the expiry cookie has disappeared. Credential cookies remain server-managed and HttpOnly. Refresh requests are shared across hooks and serialized across tabs with Web Locks to avoid rotating-token races; the concurrent-call regression test passes. |
| Misleading parser preview and unlabeled primary icons | Clarified that the filename preview does not search metadata providers. Added accessible names to add-movie/add-torrent actions and removed an internal design reference from the quality-profile form. |

FFmpeg's [HLS documentation](https://ffmpeg.org/ffmpeg-formats.html#hls-2) distinguishes appendable EVENT playlists from immutable VOD playlists. The progressive playback behavior above was also checked against the installed encoder and live application.

## Browser coverage

| Area | Checked | Limits |
| --- | --- | --- |
| Home and Libraries | Authenticated content, four movie cards, artwork, library navigation, Add Library form | Did not create a library or change its paths. |
| Library Movies and Collections | Cards, related-movie lists, collection detail, restored letter/action controls | Existing duplicate collection records remain; see below. |
| Unmatched Files and File Browser | Read-only listings and paths; four unmatched records and nine filesystem entries | Did not match, move, or delete media. |
| Movie detail and playback | Real HEVC movie through HLS on the live origin; play, pause, seek within available media, close, resume, progress persistence | Did not watch a full film, certify every codec, or test arbitrary seeking ahead of encoding. |
| Downloads | Empty state, filters, Add Torrent modal, magnet/URL/upload choices | No new downloads or uploads were submitted. |
| Notifications | Dates, totals, next page, Unread tab, restored actions | Did not bulk mark, resolve, or delete notifications. |
| Search | Existing four-movie result set; `q=Patriot` narrowed it to one matching movie | Remote acquisition search untested because no sources are configured. |
| General, library, torrent, and organization settings | Route rendering, sections, current values | Did not save settings. |
| Metadata | Provider sections and filename-preview form; Inception filename parsed year, codec, resolution, source, and release group | Preview is not a provider-match test. |
| Sources | Empty state, source selection, Torznab configuration form | No provider credentials submitted or changed. |
| Quality Profiles | Existing default profile and New Profile editor | Did not create or change a profile. |
| Casting | Device listing and settings | No physical receiver playback was initiated. |
| Backup and logs | Capabilities, empty snapshot list, navigation, system-log table | No backup restore, log clearing, or destructive recovery rehearsal. |
| Missing routes/entities | Friendly movie-not-found page and application 404 | Not an exhaustive authorization or security audit. |
| TV/music/audiobook routes | Empty route states | No corresponding fixture libraries/content are available for real playback and import tests. |
| Responsive layouts | Desktop and narrow screenshots for library, settings/backup, Downloads, and modal layouts | Browser resize automation reports timeouts despite changing the rendered layout; screenshots were inspected. This is not a cross-device or Safari certification. |

The shared browser session expired twice during the audit. After the first occurrence the server reported a missing refresh cookie and the user signed back in. Metadata recovery and concurrent refresh now have regression coverage; however, the browser session expired again before those final concurrency changes were applied. A fresh sign-in and a long-duration/cross-tab scenario are still required to verify session longevity. Both clearing-cookie headers were confirmed through the direct backend and live reverse proxy.

## Remaining gaps and follow-up priority

1. **Session longevity:** verify the final refresh changes with a fresh login across the access-token expiry window, including two tabs. This remains unverified in the browser.
2. **Data integrity and scan reliability:** two collection entities reference TMDB collection 192492 in the same library. Four additional media-file records have no movie link. Organizer conflict notifications recur, and the Shadow Recruit entry has a Failed status. Preserve the files and investigate scanner matching, idempotent collection creation, and duplicate reconciliation before any cleanup.
3. **Torrent path configuration:** runtime logs show quoted directory strings such as `"/data/session"`, producing a relative quoted path. Correct the settings deserialization/normalization and reconcile existing persistence paths deliberately. Downloads were not exercised against those paths.
4. **Playback completion:** seeking beyond the already encoded part of a growing playlist, session cancellation/resource reclamation, all codec combinations, audio queues, chapter navigation, and real Cast hardware require further tests. Closing the player stops browser playback, but server encoding is retained until session cleanup. No claim of a complete media pipeline is made here.
5. **Backup recovery:** object rehydration, complete object enumeration, and an operator restore workflow remain incomplete, as detailed in the project review. Showing a backup capability does not establish a tested disaster-recovery path.
6. **Provider/download/quality workflows:** no configured sources or active torrents exist in this fixture. Real search, acquisition, upgrade selection, extraction, organization, and error recovery need representative end-to-end fixtures.
7. **Frontend breadth and polish:** type-specific child URLs can be opened under an incompatible library type; route gating should be tightened. The installed table library emits development warnings about unstable row/column callback identities. The production build warns about large chunks. The external branding-font request was blocked in the audit browser; fallback typography renders.

## Validation and operation

- Frontend production build and TypeScript check pass; 44 frontend tests pass, including actual installed-table behavior, timestamps, dashboard data, HLS attachment stability, refresh metadata, and media source/duration selection.
- Targeted backend tests pass: three storage timestamp tests and nine transcode tests. The full backend unit suite passes: 196 passed, one ignored.
- GraphQL code generation passes against the schema snapshot. No new direct SQL was added in the backend changes.
- Rust builds ran through the shared cgroup quota: `cpu.max = 320000 100000` on four CPUs, an aggregate 80% ceiling. One build attempt was killed for memory pressure; stopping the audit encoder and retrying with the documented low-memory compiler configuration succeeded.
- Runtime logs and PID files: `/tmp/librarian-runtime/`. Detailed validation logs: `/tmp/librarian-frontend-audit-2026-09-05/`.
- Existing media, sources, libraries, and settings were preserved. Playback testing advanced one movie's saved resume position by a few seconds; it remains unwatched.

See [project review](project-review-2026-09-05.md) for the wider feature-completion plan and [dependency upgrade report](dependency-upgrade-2026-09-05.md) for package versions and upgrade constraints.

## Follow-up: persistent sessions and episode search

- Fixed the ORM timestamp mismatch that caused refresh rotation to fail and clear cookies.
- Session metadata now lasts 30 days, matching the rolling refresh window. Renewal preserves cached content, retries temporary failures, and runs before protected requests and after wake/reconnection.
- Live verification: two signed-in browser tabs renewed through exactly one successful refresh request; both remained signed in. Reloading after deleting only display metadata recovered the session from HttpOnly cookies. Browser metadata expiry measured 30 days.
- The database-backed GraphQL regression covers actual cookie rotation, 30-day TTL, near-expiry renewal, expired/replayed tokens, logout, and preserving cookies during a store outage.
- Episode and show searches now open the shared source-search modal in place, preserving show/season/episode criteria and library/show download context. Search failures end loading and show a retryable message.

Live episode-search check: Fallout S01E04 opened its search modal without changing the show URL and sent query `Fallout`, season `1`, episode `4`, and TV category `5000`. The configured IPTorrents source initially returned zero matches and no source error. Follow-up investigation traced this to the shared backend formatter sending `S01E4` instead of `S01E04`; correcting the numeric episode padding returned 35 releases for the exact same input on the live site. The episode modal displays those releases, including size, seeders, leechers, and source, both after a manual retry and after opening the episode action on a fresh page load. The backend build and all 20 source tests passed, including regression coverage for numeric episode padding and the IPTorrents search URL. Builds used the aggregate 80% CPU quota. No download was started. Frontend validation: 55 tests and production build/TypeScript passed.

### Notification and warning follow-up

The previous bulk-read handlers sent one mutation per record; the bell only
processed its ten loaded notifications. Every mutation triggered immediate
subscription refetches, and loading states replaced the visible content.
Scan issues contributed to the badge but had no acknowledgement state.

The replacement uses one owner-scoped bulk mutation across all pages and both
record types, with persistent scan-issue `readAt`, burst-coalesced refreshes, and
separate initial/background loading behavior. Unresolved issues remain accessible
on a dedicated Scan Issues tab. The table adapter retains a stable row identity
callback across renders.

Warning investigation found 404 notification records, with all 100 most recent
being repeated “Organize conflict skipped” messages, and 307 unresolved scan
issues. Of those, 104 were ffprobe parse failures. Most referenced two Jack Ryan
copies; the remaining history also included a Knight of the Seven Kingdoms
file. Valid ffprobe chapter IDs exceeded i32. The parser now ignores opaque
container IDs and stores sequential chapter positions. Identical organization
conflicts are deduplicated; duplicate-file scan records represent real copies
and must not be treated as permission to delete or overwrite media.

Validation: all 61 frontend tests pass, the production frontend build and
TypeScript check pass, and three focused backend tests pass (large chapter IDs,
bulk acknowledgement across 125 records with two owners, and complete schema
registration). Live checks exercised both bulk-read buttons, including 100 scan
issues beyond the loaded page: one bulk mutation, followed by bounded refreshes.
Subscription listeners use invalidation-only payloads and refresh once after a
WebSocket reconnect to recover events missed during session renewal.

Re-analysis succeeded for all three affected media files (15, 15, and 1 chapters;
3, 3, and 41 streams). All 104 historical ffprobe failures were resolved after
verification. The remaining unresolved scan history concerns byte-identical
copies. Read acknowledgements and original scan history are preserved; no media
cleanup was performed. The additional frontend on port 3002 was restored after
its temporary build pause. Builds retained the aggregate 80% CPU quota.

Final live rescan: 8 files discovered and matched, 0 analysis failures, and 4
byte-identical duplicate pairs. All four new scan-history records inherited
their existing acknowledgement. Notification count remained 404 and unread
count remained zero. A separate reconnect check changed a read state while the
WebSocket was disconnected; the feed recovered the correct count after
reconnection without reloading the page.


### Show controls and navigation follow-up

- Episode source searches now prefill the complete visible query (for example, `Percy Jackson and the Olympians S02E01`). Editing or deleting the episode token changes or removes the structured episode restriction. Verified the prefill and 35 results in the live modal.
- Show-level Properties now opens Show Settings for every show, including shows without playable episodes. It reads/writes the generated Show fields (`autoDownload`, `autoDownloadMode`, `qualityProfileId`) instead of opening the first episode's file details. The unused legacy modal was replaced because it described settings that do not exist on Show. Quality profiles control resolution, codec, HDR, audio, and sources; the modal links to profile management.
- Verified selecting a quality profile, saving, reopening, and clearing it back to library inheritance on Percy Jackson. The original null override, enabled automation, and WANTED mode were restored. Native quality-selector changes preserve full profile IDs.
- Library, show, movie, and collection breadcrumbs use TanStack links. Live show-to-library and library-to-Libraries clicks preserved the page document/time origin.
- The table adapter now honors explicit inline row actions instead of putting them all in the overflow menu. Episode Play/Resume and Stop appear on hover or keyboard focus (and remain accessible on touch); the active playback indicator stays visible. Paused episodes have a Resume control. File Properties remains an episode menu action. Verified available Fallout episodes have inline Play controls that become visible on row hover, while wanted rows offer Search. The background audit browser pauses CSS transitions; finishing its hover transition confirmed computed opacity changes from 0 to 1. No playback was started during this check.
- Validation: all 65 frontend tests passed; production build and TypeScript passed under the aggregate 80% CPU quota.
