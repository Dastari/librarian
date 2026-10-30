# Populated media quality pass

2026-10-24, Glass UX 0.9.10. This follows the
[foundation migration](foundation-progress.md) and addresses the reported media
control visibility, icon alignment and season presentation defects.

## Changes

- The temporary Button bridge now handles actual Tabler runtime components,
  conditional children and fragments using the library's icon/iconEnd slots.
  Media headers use explicit Glass icons through `defineActions` and `ActionBar`,
  rather than relying on compatibility inference. Play/Resume, Want and Find are
  promoted; settings, refresh and removal use the responsive overflow menu. The
  same action definitions register in the command palette. Removal still uses
  the existing confirmation; rejected mutations stay rejected for action feedback.
- Shows use real Glass DataTables separated by season, with title/code, status,
  air date, runtime, quality/file, saved progress and visible actions. The shared
  ActionSet supplies row, context and overflow actions. Admin gates, missing-only
  bulk predicates, ignore/wanted rules and release-search targets are retained.
  Media mutations refresh active backend-computed status queries separately, so
  Want/Unwant updates the status without refetching or collapsing complete collections.
  Initial loading and empty states use the table. Watch-state loading disables
  playback with a reason rather than starting from an incorrect saved position.
- Album tracks and audiobook chapters use Glass DataTable, shared cells,
  responsive row cards and visible Play/Resume/Want controls. Action columns keep
  enough width for Resume instead of collapsing the primary action into More.
  Table height is auto; the media detail page remains the scroll owner. Stable
  cell component identities and reactive presentation contexts preserve focused
  controls when mutations, computed statuses or playback state update.
- Complete paging replaces the previous 100-row truncation for shows, albums and
  books. Track queues preserve disc/track ordering, and chapter queues preserve
  chapter order. Unfinished chapter one is labelled Resume; watched chapters
  restart at zero. An incomplete audio collection allows individual playback but
  does not advertise a complete album/book queue.
- Saved progress queries use disjoint batches of at most 100 file IDs and keep
  observing Apollo cache updates. Show next-up selection uses the full candidate
  list, independently of the selected season. Player Next also uses the complete
  show and never wraps to the first episode when the current ID is absent.
- Show refresh/revisit revalidation retains the complete loaded connection until
  a fresh complete snapshot can replace it. Failed continuations keep later
  seasons readable and retryable. Scoped generations, non-advancing-page checks,
  deduplication and explicit request isolation prevent stale in-flight pages from
  restoring deleted rows or stalling a newly selected show. Cached shows override
  the app's cache-and-network policy and revalidate through this atomic path.
- Posters and continue-watching tiles use Glass Surface(card), AspectRatio,
  Progress, ToneDot and play controls. Poster navigation and playback/actions are
  siblings, so clicking or pressing Enter on Play cannot activate the detail link.
  Play/actions no longer depend on hover; shelf arrows are visible when usable.
  Missing collection entries render as passive cards rather than no-op buttons.
- Foreground hero content uses the documented imagery context. Acquisition status
  stays passive for non-admins, while admin editing uses a Glass control.

## Verification

The pass uses a disposable sandbox containing **103 episodes**, **101 tracks**,
and **101 chapters**, with real GraphQL relationships, watched/resume progress,
missing/wanted/ignored records and a short generated media file. Artwork in the
browser fixtures is synthetic. These fixtures specifically exercise the state
that the earlier mostly empty inventory did not cover.

Regression tests use actual Glass controls and Apollo cache behavior. They cover
icon slots, fragment/conditional children, card keyboard targets, member gates,
season predicates, saved progress, 100/101/201 row boundaries, ordered playback,
refresh failure/retry, delayed old-show requests, cache-and-network revisits,
request deduplication and Next beyond episode 100.

The full Vitest suite passes: **46 files, 391 passed, 3 unchanged existing skips**
(394 total). Browser validation passes **17 populated interaction/geometry
checks**, additional real episode/track/chapter mutation and focus checks, and a
**39-route smoke audit**, with no new page errors or document-width overflow.

Final checks also pass: `tsc --noEmit`, `glass-ux check-layout src` (CSS variables
resolve in eight stylesheets), `pnpm run build` including GraphQL generation and
PWA service-worker generation, and `git diff --check`. Vite reports a chunk-size
advisory; the build succeeds. Tests and builds ran serially under the shared
80% aggregate CPU quota. Logs are `/tmp/librarian-glass-quality-{suite,types,layout,build}.log`.

Browser artifacts are under `web/screenshots/glass-quality/`; QA scripts and
results are ignored files under `tmp/glass-quality-*`. Checks inspect control
opacity without hovering and icon/label centres, season switching and mutation
state, desktop/phone tables, a real coarse-pointer context, TV mode, and
light/dark × Full/Solid with non-default scale/radius/borders. Unit render success
is not treated as proof of visibility or alignment.

The disposable sandbox servers are stopped after verification.

Physical TV/webview devices, real casting and full-length playback qualification
remain separate work. The existing sandbox startup WebSocket error is tracked
as baseline behavior. No live database migration, backend deployment, commit or
PR is part of this pass. The backend release gates remain in the
[dependency upgrade record](../glass-ux-dependency-upgrade.md).
