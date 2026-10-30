# Glass UX 0.11.5 upgrade and site audit

Date: 2026-10-28. Release: [v0.11.5](https://github.com/Dastari/glass-ux/releases/tag/v0.11.5).

The upgrade and shared fixes are verified. This is not a claim that every route now
fully satisfies every design rule. Source coverage spans the 52 route files and
their feature families; browser checks cover the representative paths listed below.
The delegated source inspection was consolidated here by the parent after report
generation stalled. Findings distinguish observed source patterns from visual risks.

## Installation and shared fixes completed

- Runtime archive and docs archive verified against publisher SHA256SUMS.
- Package/lockfile upgraded; all six skills installed with the release CLI.
- Shared UI rules, layout contract, icon specification and host rule references refreshed.
- Focus-capture patch rebased to 0.11.5: native row focus updates the index retained
  by table virtualization. Upstream still lacks this behavior; retain the pnpm patch.
- Main table/galleries reserve focus reach inside their bounded results/layout.
- Poster navigation paints an inset focus outline; horizontal shelves reserve focus space.
- Custom shell/navigation now supply print handling and forced-colour focus/current states.
- Shelf movement follows reading direction; navigation offsets use logical properties.
- Narrow settings navigation uses the release's horizontal tabs with More overflow.
- Explicit product requirements preserved: whole-card A4 portrait ratio, frameless
  main tables, internal infinite loading and footers above mobile navigation/player.

## Route-family coverage

| Family | Source assessment / current composition | Follow-up |
| --- | --- | --- |
| Home and guide | Glass-backed media compositions, personal shelves, artwork heroes | Focus at shelf boundaries; avoid replacing media compositions with unrelated widgets |
| Library overview | Domain library cards and artwork marquee | Review remaining manual material/geometry recipes against Surface roles |
| Eight library browsers | Bounded fill tables/galleries, generated server filters/infinite loading, scoped alphabet jumps | Nested clipping contract and URL sorting state |
| Movie/show/collection/artist details | Artwork heroes, Glass actions/panels, complete connections | Embedded-table print coverage; retained manual presentation recipes |
| Episodes, albums and audiobooks | Glass tables/cells and visible playback actions | Large loaded collections in print; embedded auto-height tables intentionally scroll with detail pages |
| Downloads | Bounded shared table, responsive detail dialog, persistent footer | Duplicate route heading; populated runtime-download visual acceptance remains separate |
| Wanted and Activity | Complete/server-filtered infinite datasets, shared table/actions | Duplicate headings; local filter/sort URL state |
| Lists and smart collections | Generated personal entities, bounded full results, preserved Home previews | Form-bridge cleanup; URL state for full result tables |
| General/metadata/download settings | Public Glass controls, appearance runtime, existing form bindings | Migrate remaining generic form bridges in coherent slices |
| Libraries/sources/quality/organization settings | Generated collections, Glass tables and dialogs | Dense forms, dirty-close consistency and nested focus boundaries need targeted checks |
| Users/casting/backup settings | Glass table/action compositions, real authorization retained | Populated large settings collections and form-state recovery coverage |
| Logs | Bounded Glass infinite table, keyset/default order, live subscriptions | Shareable filter/sort state; multi-page print acceptance |
| Login/register | Glass LoginFlow; application-owned invite/session handling | Physical mobile keyboard/autofill verification |
| Video/audio/casting | Native media engine with Glass controls, menus and dialogs | Physical Safari/Google TV acceptance; specialized seek behavior remains host-owned |
| Root/error/redirect routes | Application router, shell and feedback compositions | No reason to introduce a Glass window manager or new product area |

## Remaining alignment work, in priority order

1. **P2 — Repeated route headings (R6.5).** `WantedPage.tsx`, `ActivityPage.tsx`
   and `DownloadsPage.tsx` render PageHeader titles already supplied by their route
   crumbs in TopBar. Remove the repeated visible title while retaining an accessible
   heading, action placement and useful counts. Assess other headings individually:
   a library-context title is not automatically the same as its active section.
2. **P2 — Nested focus-room contracts (R11.10).** `LibraryLayout.tsx` and
   `SettingsLayout.tsx` still have host outlet clipping/padding independent of
   `--focus-reach`. Shared table/poster fixes protect their own controls, but do not
   prove every ancestor safe, especially with stronger TV rings. Check first/last
   controls and toolbar edges, then use token-based room where clipping is observed.
3. **P2 — Nested print qualification (R4.10).** The custom shell grows and hides
   chrome in browser print emulation. That does not prove full output through all
   nested library/settings/detail wrappers. Test collections exceeding one printed
   page, verify every loaded row/card survives, and extend host print rules if needed.
   Printing must not silently fetch unloaded server pages.
4. **P3 — Table state and URLs.** `DataTable.tsx`, `useServerTable.ts` and
   `LogsSettings.tsx` retain local view state. Library text/availability filters
   already use URL state. Add typed router adapters for shareable sorting/search/
   column filters where appropriate, preserving generated camelCase query mappings
   and resetting infinite windows correctly. Selection should remain local.
5. **Accepted migration debt — forms and generic bridges.** `SettingsForm.tsx`,
   shared field/Button/Dialog bridges and some feature dialogs retain compatibility
   APIs. Project rules explicitly permit the current RHF bridge; it is not an upgrade
   regression. Prefer the installed bound forms, entity settings and shared ActionSets
   for subsequent coherent migrations; preserve validation, dirty-close behavior and
   permission gates rather than replacing all forms mechanically.
6. **Qualification — full appearance/accessibility matrix.** Representative Solid,
   scaled/compact and forced-colour checks passed. Full Glass/Frosted, Still, actual
   TV navigation, RTL layouts, physical iOS and receiver behavior remain separate
   acceptance work. Native buffered seeking and artwork URLs remain Librarian-owned.

## Verification and evidence

- Production TypeScript: passed. Production Vite build: passed (chunk-size advisory).
- Full Vitest: **597 passed, 1 existing skip, 77 files**.
- `glass-ux check-layout web/src`: variables resolve in all seven host stylesheets.
- **38 browser audit checks:** 390/834 widths, light/dark Solid, compact density,
  scale 1.1, radius/line weight 1.2; Home, Wanted, Downloads, movie browser, General,
  Users and Logs; active narrow settings link, high-contrast focus, print shell/chrome.
- **17 real-media browser checks:** decode, seeking, subtitles/audio replacement,
  resume/next, audio queues, failure/Retry, responsive controls and Cast empty state.
- Browser used production assets and a disposable real backend. Built-in preview
  could not start its root sandbox; the documented Playwright workflow was used.
- No production database or running production service changed. Fixture shut down.
- Evidence: `/tmp/librarian-glass-0115-validation.json`; detailed logs/results paths
  are recorded there. These checks do not certify all routes visually or real devices.
