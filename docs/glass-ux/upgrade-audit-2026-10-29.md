# Glass UX 0.11.7 alignment pass

Release: [v0.11.7](https://github.com/Dastari/glass-ux/releases/tag/v0.11.7), checked
2026-10-29 using the GitHub release API and publisher checksums. Both runtime and
built-documentation archives are retained under ignored `tmp/glass-ux-0.11.7/`.
This record follows the [0.11.5 audit](upgrade-audit-2026-10-28.md).

## Dependency removal

There are no shadcn/Radix source imports, package declarations, lockfile entries or
reachable installed dependencies. `pnpm why` for Radix Avatar/Slot returned no
consumers; the complete dependency graph also contained none. Removed eight stale
Radix package directories and the unreferenced retired HeroUI package/namespace
links left in `node_modules` from earlier installs. These were unused install
artifacts. Glass UX uses Base UI; the existing host Button/Field/Dialog APIs render
Glass controls and are not shadcn components.

## Upgrade

- Package URL, pnpm lock and dependency patch moved from 0.11.5 to 0.11.7.
- Installed all six skills with the release CLI; verified exact release contents.
- Refreshed shared UI rules, layout contract, icon specification, license, host rule
  version, style guide, catalogue and download provenance.
- Reviewed all release additions. Signature capture, image upload/crop and schema
  canvases have no current product/backend need; they are not fabricated features.
- Retained the virtual-row native-focus correction. Extended the documented patch
  with continuation-only `infinite.revision`: refreshed snapshots rearm loading
  while keeping semantic query identity, scroll, focus and selection intact.
  See [patch contract](../../web/patches/README.md).

## Previous gaps addressed

- **Route headings:** Activity, Downloads and Wanted retain an accessible h1, counts
  and actions without repeating the visible breadcrumb title. Titles return on paper.
- **Focus boundaries:** Library and Settings outlets use public `fa-focus-room` and
  token-based bottom room. Preserve stronger TV focus and the primary scroll owner.
- **Printing:** nested library/settings/detail/personal boundaries opt into document
  flow; table/gallery adapters print all matching records already in memory, and
  restore the previous screen window afterward. Printing suppresses continuation;
  it never fetches unloaded server pages.
- **Shareable state:** a typed router adapter stores only allowed sorting, filters
  and search in a table namespace. Glass column codecs validate filter values;
  backend-specific Logs keeps its existing camelCase mapping and strict allowlist.
  Browser q/show/tab values are retained. Activity search/tabs, Downloads state
  tabs and scan query/severity/status/resolution controls also round-trip in validated
  URL parameters. Back/Forward resets reveal windows and
  re-arms continuation. Selection, offsets, cursors and batch sizes remain local.
- **Forms:** General, Downloads, Organization, Metadata and Casting use public Glass
  bound forms, including naming-pattern/manual-receiver dialogs. Preserve existing
  schemas, explicit null/number contracts, permission gates, failure feedback,
  live-draft retention and Save/Discard. Removed unused `SettingsForm.tsx`.
- **Live updates:** replacement generations no longer double as a semantic query
  change. The table reader stays in place, and Logs can continue after an interrupted
  batch followed by an equal-count refresh.

## Deliberate boundaries

RHF still serves remaining feature dialogs and library/user/quality forms through
Glass controls. The shared Button/Dialog/Field bridges still have consumers; removing
them mechanically would not improve those contracts. They can migrate as complete
feature slices. Artwork, media playback, buffered seeking, acquisition and generated
GraphQL remain application-owned.

Physical iPhone/iPad, TV remote and Google Cast receiver acceptance remains separate.
Browser appearance/RTL/TV-mode checks cannot qualify physical decoding or receiver
network behavior. No production backend or database is modified by this UI pass.

## Validation

- Full TypeScript: passed. Production build: passed (existing chunk-size advisory).
- `glass-ux check-layout web/src`: all tokens resolve in seven stylesheets.
- Six skills and copied reference documents verified against the release package.
- 106 browser alignment checks: 390/834 widths, light/dark Solid, Frosted and Glass,
  reduced motion, compact density, scale 1.1, radius/line weight 1.2; Wanted,
  Activity, Downloads, library browser, and all five migrated settings families
  plus Users/Logs. Includes forced-colour focus, RTL/TV-mode geometry, query history,
  real subscription scroll retention and route-filter hydration.
- A real generated 131-entry personal list prints all loaded rows across multiple
  A4 pages (six in this fixture) and returns to its bounded screen layout. A PDF is retained in the
  disposable run artifacts. No GraphQL continuation was requested while printing.
- Real-media browser acceptance: **17 checks passed**, including actual direct/remux/
  transcode decode, captions/audio replacement, resume/next, queues and failure/Retry.
- Full Vitest suite: **658 passed, 1 unchanged existing skip, 85 files passed**.
  Full TypeScript includes the new tests. All build/test processes used the shared
  80% aggregate CPU cgroup, one test worker; no Rust recompilation was needed.
- Evidence: `/tmp/librarian-glass-0117-validation.json`. Temporary services stopped;
  the production bundle was rebuilt without starting production services.

Browser preview returned the explicit root-sandbox error (“Chrome will not start its
sandbox as root”), so the repository's documented Playwright workflow was used.
Disposable backend and generated fixtures only; no production service or live data.
Scope is the changed implementation and representative populated routes, not every
possible device or page state.
