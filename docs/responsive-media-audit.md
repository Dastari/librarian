# Responsive media and table audit

2026-10-24. Glass UX 0.9.10. Follow-up to the
[media quality pass](glass-ux/media-quality-pass.md).

## Product direction

Librarian should open as a personal media library: artwork, Continue Watching,
recent additions, seasons and playback. Management remains available without
turning the home screen into a dashboard of empty administrative panels.
Full-bleed show/movie artwork and foreground scrims remain part of the detail
design. Posters also appear on tablet layouts; phones retain the background art
and give the title, metadata and primary action the available width.

## Changes in this pass

- Phone navigation has four primary destinations and a More menu for Wanted,
  Activity and Settings. All destinations remain available without six crowded
  labels. Page content reserves space for both the bottom navigation and an
  active audio mini-player. Tablet audio controls allocate more space to seeking.
- Continue Watching and local-library shelves precede the TV guide and external
  release discovery. Libraries use a compact horizontal shelf on Home; their
  dedicated route retains the full grid. Empty guide copy is compact. The
  shelf keeps the latest resume target per show, book or album instead of
  repeating multiple episodes/chapters from the same title.
- Detail sections use tighter phone spacing, titles can wrap, and poster grids
  retain useful cover density at narrow widths. Missing detail records display
  a readable state. Artist and collection relations load beyond their first
  100 entries and expose continuation failures with Retry.
- The compatibility DataTable now composes the public Glass runtime. It retains
  Librarian poster galleries separately from responsive record cards. Narrow
  tables fold secondary fields into accessible details rather than clipping them.
  Column/cell component identities stay stable during row/action/status updates.
- Complete collections use Glass search/filter contracts before revealing
  successive batches. Settings load complete snapshots before enabling local
  filtering; a failed refresh retains the previous complete snapshot. Media
  season/track/chapter tables use the same complete-collection filtering rule.
- Server-backed library/activity/scan/log lists use accumulated batches and
  dataset-wide server predicates. A changed query identifies a new dataset;
  appended batches preserve its selection and focus. Refreshes suspend load
  triggers. No table offers Next/Previous or a page selector.
- Status queries are batched to respect the backend's 500-item limit while
  continuing to observe Apollo updates. Library pickers, torrent metadata and
  grouped Wanted data no longer silently stop at their former fixed limits.

## Data-loading contract

The table must know whether its rows are a complete collection or an incomplete
server result. Local filtering of one downloaded page is prohibited. A complete
snapshot can be filtered first and incrementally rendered afterwards. A server
list sends search/filter/sort inputs to its query, resets its accumulated result
when those inputs change, and appends subsequent offsets as the end comes into
view. Selection applies to the current dataset, never a previous filter scope.

Source searches are an explicit exception to dataset exhaustiveness: the current
`SearchSourcesInput` has `limit`, but no cursor, offset, `hasMore` or completeness
metadata. Removing the client's 100-result limit does not make every indexer
exhaustive. The release dialog explains that results may be partial and filters
apply to returned releases. Genuine remote infinite search needs a per-source
continuation contract, stable deduplication keys, and explicit partial/failure
metadata. This is a backend/API follow-up; the UI does not invent a continuation.

## Route and device coverage

The final route audits cover **38 concrete routes at five sizes**: 390×844,
834×1194, 1194×834, 375×812 and 768×1024 (**190 route/viewport checks**).
They include Home, Libraries, Search, Wanted,
Downloads, Activity, every settings section, all nine library content/admin views,
all six media detail families, populated and missing collections, redirects and
the player failure state. There are no pagination controls or document-width
overflow. A transient development HMR error on Search was rechecked successfully
at 375/768/834 after the source edits settled; both the original result and its
successful recheck are retained in the artifacts.

**11 additional interaction checks pass**: phone navigation/More, fitting table
search/filter controls, long file paths, filtering beyond the first rendered
track/chapter batch and loading again after clearing search, phone mini-player
content reservation, tablet seeking, library server search plus continued loading,
a 320px media action menu, and login/register. Record cards and folded fields are
readable on narrow settings/library panes; gallery mode remains a media grid.

The disposable sandbox contains 103 episodes, 101 tracks and 101 chapters, linked
media, saved progress and missing/wanted/ignored states. Artwork is synthetic.
Screenshots and browser results are ignored artifacts under
`web/screenshots/responsive-audit/` and `tmp/glass-responsive-*`.
External TMDB release-discovery requests can fail in the sandbox and display
their existing retry states. Those provider results, full-length media playback,
real casting and physical iOS/iPadOS devices are not qualified by this audit.
The sandbox servers are stopped after checks; the live server is left running.

## Automated verification

- Full Vitest suite: **52 files, 432 passed, 1 pre-existing skipped tooltip test**
  (433 total). The two formerly skipped legacy table-selection cases have working
  Glass checkbox/keyboard/bulk-action regressions now.
- `tsc --noEmit`: passes, including test sources.
- `glass-ux check-layout src`: passes; CSS variables resolve in eight stylesheets.
- `pnpm run build`: passes, including GraphQL generation, production TypeScript,
  bundle output and PWA service-worker generation. Vite reports a chunk-size advisory.
- `git diff --check`: passes. Tests and builds use the shared 80% aggregate CPU cap.

Tests cover complete filtering beyond the rendered batch, continuing after a
search is cleared, equal-sized replacement heads, refresh/append races, selection
scope, retained Wanted/settings snapshots, duplicate offsets, batching more than
1,000 computed statuses, cache mutation invalidation and principal isolation.
Logs are `/tmp/librarian-responsive-{suite,types,layout,build}.log`.

## Plex comparison and priorities

These are identified product gaps and proposals, not features implemented by
this responsive pass. The comparison uses current official Plex documentation;
absence claims about Librarian refer to the inspected implementation.

| Priority | Gap in the current Librarian experience | Useful reference and proposed direction |
| --- | --- | --- |
| P1 | Filters cannot be saved as user-curated collections or playlists. Existing collections have no smart-rule editor. | Plex supports smart collections derived from filtering/sorting, including home rows. Add saved typed predicates and ordering, with server-wide results and user ownership. [Plex recommendations](https://support.plex.tv/articles/manage-recommendations/) |
| P1 | Home shelves are fixed in code; users cannot pin, hide or reorder them or choose favourite libraries. | Plex exposes per-library home/recommendation visibility and ordering. Add per-user shelf preferences, keep Continue Watching prominent, and provide a compact management view. [Plex recommendations](https://support.plex.tv/articles/manage-recommendations/) |
| P1 | Playback selects one linked file; there is no library-facing versions/editions chooser or separately tracked edition identity. Release-name parsing alone is insufficient. | Plex groups alternate encodes as versions and distinguishes editions. Add explicit version identity, playback suitability and per-edition progress without destructive automatic merging. [Plex versions](https://support.plex.tv/articles/200381043-multi-version-movies/), [Plex editions](https://support.plex.tv/articles/multiple-editions/) |
| P1 | Offline readiness currently refers to the PWA app shell, not a managed offline media collection. | Plex has explicit device Downloads for offline media. Define storage limits, download queues, expiry, playable offline manifests and later progress reconciliation. Keep this distinct from server acquisition downloads. [Plex Downloads](https://support.plex.tv/articles/downloads-overview/) |
| P2 | There is no detected intro/credits marker pipeline or corresponding skip affordance. | Plex detects season intros and offers Skip Intro. Introduce typed markers, optional background analysis, manual correction and a player action; avoid pretending arbitrary seek offsets are detected markers. [Plex intro detection](https://support.plex.tv/articles/skip-content/) |
| P2 | Audiobooks have chapter resume and rate controls but no listening bookmarks or sleep timer. | Treat audiobooks as a first-class listening workflow: sleep at a chapter boundary, bookmarks with notes, and clear per-book progress. This is a Librarian proposal, not a claim about Plex's supported audiobook feature set. |

## Where Librarian can improve the experience

- Keep discovery, acquisition, missing items, quality evaluation and playback
  connected through the same title. Show why an item is wanted, blocked, missing,
  mismatched or unsuitable, with the next useful action beside that reason.
- Make large-library filtering predictable: searchable complete datasets, no
  silent record caps, no lost filters or selections, and readable retry states.
- Distinguish viewing intent from acquisition policy: a personal Watch Later
  list should not silently request files or change an administrator's Wanted rules.
- Keep local-server management and personal playback usable together. Surface
  direct-play/transcode reasons, selected audio/subtitle tracks and device
  suitability as decisions users can understand.
- Offer personal shelves and collection rules without adding unrelated streaming
  services, social feeds or promotional content to a privately hosted library.

These opportunities express Librarian's product direction. They are not a claim
that Plex lacks every corresponding capability.
