# Librarian → Glass UX migration plan

Reviewed 2026-10-26. Target: **glass-ux 0.9.10**. The source migration now includes
the remaining controls and removes HeroUI/React Aria dependencies and recipes.
The inventory and ordered slices below preserve the original migration baseline;
the latest verification and qualification limits are recorded at the end.

Replace Librarian's generic controls, surfaces, tables, navigation, forms and
feedback with Glass UX. Keep its cinematic media layouts and playback behavior,
composed from the packaged primitives. Preserve the router, generated GraphQL
contract, Apollo cache/subscriptions, cookie sessions, authorization and PWA.

## Review and preparation

- Downloaded the library source and the release's runtime and offline docs
  archives. Both release archives passed the publisher's SHA256 checksums.
- Installed the exact release URL and lockfile, all six project agent skills,
  shared design/layout rules and the consuming-project rule pack. See
  [reference installation](glass-ux/README.md).
- Aligned TanStack Router with the library's exact 1.170.43 runtime dependency
  and Zod at 4.6.7 to avoid duplicate router contexts/schema runtimes.
- Reviewed all **50 route files**, the shared UI/shell, feature compositions and
  **24 DataTable consumers**. There are **50 direct HeroUI source consumers**,
  including 33 toast consumers. See the [site inventory](glass-ux/site-inventory.md).
- Reviewed the complete library API directory: **353 component pages**,
  **95 documented families**, **130 package export keys**. Every family has a
  replacement/use/defer decision in the [catalogue](glass-ux/component-catalogue.md).
- Browsed 35 authenticated desktop destinations with synthetic sandbox records,
  five representative phone destinations, and the live sign-in screen. Browser
  evidence and its limits are recorded in the site inventory. These are baseline
  screens, not migrated screenshots or evidence that all product flows work.
- Updated graphql-orm/auth before UI implementation; see
  [dependency upgrade notes](glass-ux-dependency-upgrade.md) for pins, migration,
  validation and remaining deployment gates.

The source now wires the Glass UX stylesheet, Vite plugin, appearance, tooltip,
action/toast/confirmation and command-palette providers. The old root theme and
refraction engine are removed. See [foundation implementation](glass-ux/foundation-progress.md)
for the precise scope, verification and remaining compatibility boundary.
The [populated media quality pass](glass-ux/media-quality-pass.md) adds season,
track and chapter tables, shared actions, visible playback and complete paging.
The [responsive media audit](responsive-media-audit.md) migrates the remaining
shared table runtime to Glass, removes table pagination UI, and adds complete
filtering/infinite contracts plus phone/tablet layout verification.
Backend runtime/deployment qualification remains a release gate; the UI work is
verified against synthetic sandbox data using the existing debug backend.

### data-table-pro

It is already absent from the active manifest, lockfile and source. The retired
`frontend/` is only in history. Remaining mentions are historical upgrade notes;
retain those as history. The implementation to replace is
`web/src/components/ui/DataTable.tsx`, which has its own table/grid, sorting,
selection, actions and pager. No data-table-pro compatibility layer is needed.

## Target architecture

| Layer | Responsibility |
| --- | --- |
| Glass UX | Controls, field chrome/typed inputs, Surface/Panel, overlays, navigation primitives, tables/cells, actions, feedback, appearance, formatting |
| Librarian adapters | Generated GraphQL inputs/connection mapping, Apollo integration, action handlers, form serialization, preferences, router/breadcrumb mapping, input-mode integration |
| Librarian compositions | Posters/backdrops/heroes, media shelves, alphabet jump, TV guide, metadata/provider result views, acquisition, file/scan views, persistent player and casting |
| Existing services | Auth/authorization, metadata, media matching/import, storage/backup, torrents and streaming |

Use public feature entries. Do not copy Glass UX source, introduce a second
generic design system, or replace the SPA with the docs site's windowed workspace.
Scope new adapters under `web/src/lib/glass/` (appearance/navigation) and
`web/src/lib/graphql/` (typed data boundaries). Group retained media compositions
under `web/src/components/media/` as imports migrate. Keep temporary compatibility
exports discoverable in `components/ui/`; delete each after its final consumer moves.

## Critical integration decisions

### One appearance and style runtime

1. Inventory overlapping root custom properties, `.glass-*`, utility classes,
   document defaults and HeroUI component recipes before the stylesheet switch.
2. Register `glassUx()` beside React/Tailwind in `web/vite.config.ts`. Use the
   documented Tailwind v4 layers/preflight/utilities and import shared styles once.
   For `web/src/styles/app.css`, the package source path is
   `../../node_modules/glass-ux/dist`, not the README example's relative path.
3. Register Librarian's product marks and named palettes with the documented
   brand/appearance APIs. Preserve Cinema's warm accent and the named device
   preferences; do not silently turn every existing device into Graphite.
4. Translate `librarian.theme`: dark → Cinema/dark, light → Cinema/light,
   midnight/ember/forest → corresponding host palettes/dark, system →
   Cinema/system. Write the new namespaced appearance preference only when absent.
   Keep the old value available during rollback. AppearanceProvider becomes the
   sole owner of palette/mode attributes. `data-input-mode` remains host-owned.
5. Add a bounded compatibility token sheet only for still-used layout/typography
   names. It reads Glass roles and cannot redefine Glass root geometry/materials.
   Rename conflicting old material classes before cutover. Remove old refraction
   filters, LiquidGlass effects and HeroUI overrides as their consumers move.
6. Replace shared controls and every toast host at root cutover. Remaining direct
   HeroUI slices import only their individual component recipes through
   `legacy-components.css`, with aliases derived from Glass roles. Never import
   HeroUI theme/base or its full stylesheet: there is one root appearance system.
   This explicit temporary boundary replaces the original all-controls-at-once
   proposal; remove each recipe and alias as its final consumer migrates.
7. Configure the needed AppearanceProvider, TooltipProvider, ActionProvider,
   PaletteProvider once; ActionProvider mounts Toaster and ConfirmHost. AppProviders currently sits outside the
   RouterProvider: either move router-dependent configuration beneath router
   context or pass the documented navigation callback explicitly. Keep Apollo
   and session/input-mode providers stable.

Use library density, UI scale, radius, line weight, material and motion. Preserve
Librarian safe-area/dock offsets as host layout. Reserve card geometry and use card
material for repeated artwork; anything over art uses the library's imagery rules.
Test live versus baked glass for this media canvas; do not assume baked chrome
can reproduce a live artwork backdrop. Keep reduced-transparency/Solid behavior.

### Typed DataTable and Apollo boundary

The bundled `glass-ux/adapters/graphql-orm` targets a different schema casing:
`toWhere` builds `And`, `Or`, `Contains`, `InList`, `Between.Start/End`; connection
helpers expect `Edges.Node`, `PageInfo.TotalCount`, and offset helpers return
`Limit`/`Offset`. Librarian uses `and`, `or`, `contains`, `inList`,
`between.start/end`, `edges.node`, `pageInfo.totalCount`, `limit`/`offset`.
**Do not wire those helpers straight into generated operations.**

- Build a small typed host adapter from table state to each generated Where,
  OrderBy and Page input. Use explicit field/operator maps. A generic recursive
  lowercasing function can corrupt enum values, JSON payloads and field names.
  Reuse bundled filter semantics only behind this boundary. An upstream configurable
  casing enhancement is an optional follow-up, not a dependency of the migration.
- Keep `useConnection`, `useInfiniteConnection` and generated documents initially.
  Adapt their camelCase connection result to `rows`, `rowCount`, `hasMore` and
  `endCursor` without cloning stable row arrays every render.
- Use offset connections for freely sortable views. Keyset is only suitable
  where the entity declares a fixed order. Reset loaded rows/cursors on scope,
  filters, query or sort changes. Preserve head refetch, deduplication and
  subscription invalidation rather than installing a second Apollo merge policy.
- Control state once. Existing `q`/`show` links and persisted
  `browser.<key>.view`/`table.<key>.pageSize` preferences get explicit migration.
  Do not run nuqs and `useTableUrlState` as competing writers of the same keys.
- Give each table a user/library/view-scoped stable `id` and query identity.
  Reset selection on incompatible scope changes. Bulk actions operate on explicit
  selected IDs/loaded records; a header checkbox cannot imply every matching
  server row. Preserve deliberate selection across pages only when IDs and host
  action resolution support it.
- Translate column `cell` to the exact exported column renderer contract;
  `hideBelow` becomes container priority/default-hidden/card-role decisions,
  not a mechanical rename. Use shared identity, numeric, time, state and meter cells.
- Set `rowOpen="click"` where current rows navigate. Action/menu/checkbox clicks
  must not navigate. Keep typed ActionSets stable; confirm and toast once.
- Use `infinite={{ hasMore, onLoadMore, loadingMore }}`; the old callback is
  named `loadMore`. Retry must preserve already loaded rows.
- The library's cards below `cardBreakpoint` are compact data records. Its API
  has **no `renderCard` prop**. Keep a host `MediaCollectionView` choosing
  Glass DataTable versus a cinematic poster grid, using shared row/action/filter
  state. Do not discard the gallery view or force poster markup into table cells.
- Define the scroll owner. Use `height="auto"` for a table inside page scrolling,
  or a fill table in a bounded `min-height: 0` pane. Keep status/toolbars stable.

Contract cases: two filters on one field, nested AND/OR, null, boolean, UUID/string
IDs, enum options, inclusive date ranges, local-day/time-zone boundaries, numeric
units, multiple sort keys, forward pages, refetch after deletion and empty results.
Validate the variables with the generated schema/types and an actual backend,
including a member whose rows differ from an administrator's.

### Actions, forms and authentication

Define one action set per media/library/torrent/source/device kind for header,
row, context and bulk surfaces. Adapt existing mutations and `assertSuccess`;
library action completion and host `toast` must not both report the same outcome.
Keep destructive effects and admin/member restrictions as currently implemented.

Glass UX's bound forms use TanStack Form; this app uses react-hook-form and Zod.
Use a temporary Controller-to-bare-control bridge during primitive conversion,
then migrate whole forms to `useAppForm`, `formBehaviour` and typed bound fields.
Keep one validation owner per form. Remove react-hook-form/resolvers only when
their last usage is gone. Zod remains useful for shared domain validation.

Typed controls store meaningful values: seconds, bytes, calendar dates or a
wall-clock plus IANA zone. Settings currently include minutes, KB/s and sentinel
zero/unlimited values. Write explicit round-trip load/save conversions. Do not
interpret a seed ratio as a percent. Typed schemas must preserve existing backend
ranges and password/email policy rather than silently adopting new defaults.

Use LoginFlow for `/login` and `/register`, with host callbacks establishing the
existing cookie session and performing the existing redirect. Preserve first-run
administrator setup, invite-only later registration, display name and invite URLs.
0.9.10 has no arbitrary sign-up field slot: collect/validate an invite code in a
small host pre-step when the URL does not supply one, then capture it in onSignUp.
Do not hide it in a footer and submit an unvalidated value. Unsupported reset,
provider and MFA callbacks stay absent. Keep the constellation as optional
product artwork with a motion/transparency fallback, not another glass engine.

### Keyboard, TV and playback

`useSpatialNavigation` listens globally to arrow keys. Glass UX/Base UI composites
also own arrows, including portaled grids, trees, menus, sliders, tabs, toggle
groups and Select popup triggers. Respect `defaultPrevented` and native composite
ownership. Ordinary dialog buttons still need remote movement: scope candidates
to the nearest dialog/alertdialog and use a modal-only capture fallback because
Base UI stops bubbling keyboard events. `closest('[data-spatial-ignore]')` on the page
alone does not cover portals. Focus return must target the original row/card when
an overlay closes; removed rows need a sensible neighbor.

Keep larger TV targets/type as a host density/input policy that cooperates with
appearance. Verify arrow navigation, Enter, Back/Escape, media keys and scrolling
with keyboard emulation, then physical TV/webview devices. Supported modern
desktop browsers do not establish compatibility with old webOS/Tizen engines.
Coordinate with the separate [TV applications plan](tv-applications-plan.md).

Keep the player store, HLS/native media lifecycle, Media Session, progress sync,
queue, chapter data, casting and mini-player continuity. Replace presentation
buttons/menus/sheets/volume sliders. A buffered seekbar with chapter ticks and
hover preview needs a host composition or a generic upstream slider extension;
plain Slider marks do not provide the existing buffered overlay API.

## Shared-component replacement map

Paths below are relative to `web/src/components/ui/`. The inventory appendix lists
all route bindings and feature consumers.

| Existing component(s) | Replacement / retained composition |
| --- | --- |
| Button, GlassButton | Glass Button/ButtonLink/ActionButton. Translate supported variants, pending/disabled and icon labels; retire local recipes. |
| GlassSegmented, ViewToggle | Segmented with controlled view/filter state. |
| GlassSwitch, FormSwitchField | Switch/SettingRow; bound field after form migration. |
| Dialog, ConfirmDialog | Dialog/AlertDialog/ConfirmHost. Preserve pending-close policy and focus restoration. |
| Panel | Glass Panel and Header/Body/Footer slots; group headings on canvas. |
| LiquidGlass, GlassFilters | Remove after shared material cutover; no duplicate SVG refraction/blur engine. |
| DataTable | Glass DataTable plus typed host adapter and separate MediaCollectionView grid seam. |
| FormTextField, FormNumberField, FormSelectField, SelectList, FieldGroup | Field/FormStack/FormGroup and typed controls. Interim RHF bridge, then bound forms. |
| EmptyState, ErrorState | EmptyState and Notice with retry/action. No full-page replacement of shell. |
| Spinner | Packaged pending/progress/skeleton states; retain a tiny host loading boundary only where a spinner is actually required. |
| StatusChip | StatusBadge/StatusDot; host status vocabulary and computed progress remain. |
| StatTile | Metric/DetailTile; supply a definite height and real values/series. |
| KeyValueList, MetaLine | FactList/DetailGrid, semantic metadata and ChipOverflow where appropriate. |
| SegmentTabs, SideTabs | RouteTabs/RouteVerticalTabs for routes; Tabs/Segmented for controlled local state. |
| ThemePicker | SwatchPicker plus appearance controls. |
| SkeletonBlock/Text/Card/Row/Grid/Hero | Packaged skeletons plus host media-shaped skeleton compositions. |
| BrandMark | Configure product brand assets; retain existing artwork, PWA icons and wordmark. |
| PageHeader, Section, Toolbar | Prefer EntityHeader/ActionBar where meaningful, otherwise simple host semantic layout. Remove duplicate visible route headings. |
| Artwork | Keep cached URL/error/placeholder/loading policy; compose AspectRatio, skeleton and image fallback. |
| PosterCard, BackdropCard, HeroBanner | Recreate as media compositions over Surface(card)/AspectRatio/Progress/StatusBadge/ActionBar. |
| MediaRow, PosterMarquee | Keep media shelf/landing composition; ScrollArea, keyboard controls and reduced-motion behavior. |
| AlphabetRail | Keep alphabet-jump domain behavior; use Glass buttons/scrolling, preserve loaded/unloaded letter jumps. |

Shell migration uses Glass AppShell, Rail, TopBar, TabBar, AccountButton,
RouteBreadcrumb, SearchTrigger and appropriate notices. Keep `nav.ts`, counts,
admin rules, deep links, hero top-bar behavior, connection status and UpdatePrompt's
PWA lifecycle. A library window-manager Dock is not the persistent media PlayerDock.

## Missing compositions and extension backlog

| Work item | Owner / public shape | Acceptance |
| --- | --- | --- |
| MediaTile + Artwork | Host. Stable ID, artwork/aspect/alt, title/meta, status/count/progress, link, actions and play callback. | 2:3 and square, missing/broken/slow art, touch actions, keyboard/TV, no nested buttons/links, image geometry stays fixed. |
| MediaHero / BackdropCard | Host. Backdrop/poster/title/facts/action slots; media-type tint belongs to host palette. | Full bleed, readable over art, narrow stacking, accessible actions, Solid/motion modes. |
| MediaCollectionView | Host. One row/filter/selection/action model, table or poster grid. | View preference survives; changing view keeps filter/sort identity and does not change action semantics. |
| MediaShelf + alphabet jump | Host. Stable row identity, page loader and letter navigation. | Keyboard scrolling and fetch-on-jump work; no lost focus or duplicate/inconsistent pages. |
| TV guide / release shelves | Host. Existing date/scope/country/provider models into Segmented, ListRow, country selection and media tiles. | Local-day boundaries, specials, in-library/add/play distinctions and error/empty states remain correct. |
| FolderPicker | Host adapter around Tree/Dialog/SplitPane. Full-path IDs, lazy loader, selection and validation callbacks. | Root restrictions, lazy errors/retry, network paths, manual entry, cancellation and folder selection preserved. |
| OrderedPreferenceList | Host initially. Ordered unique values, options/custom entry, drag/drop and move actions. | Order survives save/load; keyboard/touch reordering, duplicates and empty options behave correctly. Candidate for generic upstream extraction after two real consumers. |
| BufferedSeekBar | Host around documented slider seams where possible. Position, duration, buffered range, chapters, preview and seek-commit. | 5-second/large-step keys, no duplicate seek calls, scrub/preview/buffer parity; propose upstream generic track slot if needed. |
| Media icon coverage | Reuse packaged IconPlay/IconPause and other semantic equivalents. Movie/TV/music/book, casting, volume, skip and subtitle glyphs need a semantic audit; matching Tabler export names are absent in 0.9.10. | Add genuinely missing reusable glyphs upstream following the bundled icon specification, or keep explicit transitional Tabler imports until a released replacement exists. No invented custom SVG icon kit. |
| Registration invite pre-step | Host, using typed field/Dialog or routed pre-step. | First-run bypass only when needsSetup; invite URL/manual code carried unchanged to register mutation. |
| CamelCase ORM adapter | Host typed boundary; optional upstream configurable schema support. | Generated-query variable validation and real server tests; no PascalCase input/connection assumptions leak into Apollo. |

These are compositions or adapters, not reasons to fork Glass UX. A genuinely
generic upstream component should follow its author skill: typed public entry,
API docs, examples, state/keyboard contracts and focused checks. Upstream changes
and publication require separate implementation work; this plan does not claim
they already exist in release 0.9.10.

## Ordered implementation slices

Each slice has a narrow reviewable outcome and a deletion check. Keep the
application working between slices; backend qualification precedes release.

| Order | Scope | Exit gate |
| --- | --- | --- |
| 0 | ORM/auth update, private refresh grant storage, finite residual authorization scans, schema/client regeneration if SDL changes. | Auth/ownership/list/subscription/backup contracts pass; disposable upgrade rehearsal and deployment notes ready. |
| 1 | Versioned library/docs/skills/rules and complete source/browser inventory. | Installed now; reproduce package/docs, preserve prior local work, approve this implementation sequence. |
| 2 | Appearance/style collision inventory; primitive compatibility bridge, actions/toasts and provider topology. | All direct HeroUI controls have a cutover path; no duplicate root token owner. Isolated foundation specimen passes geometry/focus checks. |
| 3 | Atomic root CSS/Vite/provider switch plus shell/common primitives. | Existing routes still work; remaining HeroUI recipes explicitly bounded; five legacy themes mapped, PWA assets/chunks checked. |
| 4 | `/settings/logs` as the table pilot. Keep read-only browsing/filter/paging first; then clear-log confirmation. | Typed camelCase variables, server totals/sort/filter, live updates, row details, long messages, phone cards and scroll ownership verified. |
| 5 | `/settings/general` as the form/appearance pilot, then `/login` and `/register`. | Appearance persistence/conversion, TanStack Form contract, dirty-save/failure, auth cookies/invite/setup and keyboard focus verified. |
| 6 | `/libraries` and movie library grid/list, including shared MediaTile/Hero/CollectionView and alphabet jump. | Gallery/table parity, URL filters, infinite load, selection and add/search workflows verified; old renderCard implementation removed. |
| 7 | Remaining seven library browsers, file picker/manual matching, scans and library settings. | Music square art, tracks, collections/books/TV, safe file actions and scan-detail states verified. |
| 8 | Six media-detail routes, EpisodeList, acquisition and release/provider dialogs. | Same action set across hero/table/menu; seasons/chapters/files/metadata, confirmations and pending/error states verified. |
| 9 | `/downloads`, TorrentDetail/Add/Link/ReleaseSearch, `/wanted`, `/activity`. | Real subscription updates, loaded selection, release restrictions, upgrade approvals and no live-update target movement. |
| 10 | Remaining settings: libraries, metadata, sources, quality, organization, downloads/blocklist, casting, users/invites, backup, about. | Typed units/inheritance/secret handling/order semantics and admin/member route visibility preserved; backup/restore confirmations unchanged. |
| 11 | Home guide/releases/continue watching, global search/provider results and landing constellation. | Library/provider result boundaries, country/date scope, artwork failures and motion/Solid states verified. |
| 12 | PlayerDock, NowPlayingSheet, VideoPlayer/WatchPage, transport/seek/volume/menu presentation. | Persistent playback across navigation, native/HLS, queue/chapters/subtitles/audio/cast and remote controls qualified with synthetic playable fixtures. |
| 13 | Delete compatibility components/styles/providers and unused dependencies; final browser/PWA/device qualification. | No obsolete HeroUI, React Aria or Tabler consumers; no custom generic table/glass/control implementation; all required gates pass. |

Implemented source slices: 1–3 (reference installation, root appearance/style
boundary, shell/shared primitives), plus the slice 4 Logs DataTable pilot. Slice 0
has compile/schema contract preparation but retains its runtime/migration gate.
Slice 5 LoginFlow is the next feature migration. Broader settings actions/forms
and direct component API cleanup for the 21 compatibility-table consumers follow
their listed slices; their shared runtime now uses Glass and infinite loading.
The later media slice now includes season/track/chapter tables and header/card
quality fixes; other media, library and acquisition work is still pending.

Slice 3 is broader than a single screen because root CSS/appearance are global.
Prepare its component bridge before merging that switch; use the pilot to prove
the data/form adapters before migrating feature layouts widely. Do not accumulate
a permanent legacy facade that recreates every old prop and prevents use of
Glass UX's actions, cells, typed fields and responsive behavior.

## Route-specific requirements

| Area | Preserve and improve through library components |
| --- | --- |
| Home | Keep guide first, release rows, libraries, recently added and continue watching; use concise headings, stable placeholders and one glass layer. No new marketing framing. |
| Libraries + all browsers | Scope/type-aware navigation, counts, URL availability/search, card/list mode, server sort, infinite pages, letter jump, acquisition/add and playlist actions. |
| Library scans/files/settings | Tree/path handling, issues/detail drill-in, selected files/manual matching, cancellation and destructive-action naming. |
| Movie/show/album/artist/audiobook/collection | Existing hero artwork, watched/file/quality fractions, casts, seasons/episodes/tracks/chapters, missing items, metadata refresh and library navigation. |
| Search | Local results and provider results keep distinct query/pagination/add semantics; query identity resets selection and loaded pages. |
| Downloads | Current live torrent model, pause/resume/remove/link/process, statistics and file/peer/detail inspection. Charts only if actual samples exist. |
| Wanted | Existing entity grouping and target rules; no client-only page filtering masquerading as dataset filtering. |
| Activity | Unread/action/all, read/resolve/delete, mark-all and quality-upgrade approval; actual action policy remains in backend. |
| General | Device appearance/input mode plus admin playback/language settings. Render appearance with library controls, keep mode detection independent. |
| Libraries settings | Create/edit/delete library, folder choice and per-type fields. |
| Metadata settings | Current provider keys/configuration/test/cache; masked secrets and unavailable-provider state. |
| Sources settings | Typed source configuration, dynamic credential fields, test and enable; do not advertise unimplemented acquisition protocols. |
| Quality settings | Profiles, library inheritance, ordered codecs/resolutions/languages/groups, defaults and ranges. |
| Organization settings | Naming patterns, preview, library-type defaults and path consequences. |
| Downloads settings | Download/session directories, listen port, limits, automatic scheduling, seeding policy and release blocklist; explicit unit conversions. |
| Casting settings | Discovery versus playback support, manual devices, enable/remove and runtime errors; no fabricated connectivity. |
| Users settings | Roles/enable state/invites, copyable URLs and actual permissions. |
| Backup settings | Snapshot/table views, download/restore/delete, capability/credential state and consequential confirmations. |
| Logs + About | Live/filterable technical data and build/system facts; no fake history or metrics. |
| Watch/player | Immersive controls plus missing/unsupported/error states, safe-area/fullscreen, focus and playback continuity. |

## Verification and completion

Run builds/tests under the shared aggregate 80% CPU cgroup. Do not start backend
or web dev commands directly; use the documented sandbox/live lifecycle where
needed. UI tests that mutate data run only against an isolated fixture sandbox.

- Baseline and per-slice TypeScript checks, focused Vitest behavior tests, then
  existing suites and production build. Validate generated GraphQL documents
  against the newly exported backend schema.
- Test host adapters at meaningful boundaries rather than mirroring component
  implementation. Keep real table interaction tests; do not mock away the library.
- Browser matrix: desktop, narrow 390px phone and narrow container, tablet, TV
  keyboard mode; light/dark, Glass/Frosted/Solid, reduced motion/transparency,
  comfortable/compact, non-default scale/radius/line weight.
- Run `pnpm exec glass-ux check-layout src` after foundation CSS is migrated.
  Check actual computed insets/corners, focus rings and clipping at scroll edges.
  This checker alone is not proof of visual or accessibility correctness.
- Delayed font/artwork/network, validation, reconnect and torrent/notification
  updates must not move active buttons. Verify menu/dialog focus return and
  keyboard arrow ownership, including portals.
- Check the built artifact: lazy component chunks, bundled fonts/assets under
  the configured origin/base, offline/PWA update behavior, embedded frontend and
  cache version transition. Installing the large library must not eagerly load
  maps, topology, AI, rich text or charts on every route.
- Confirm the host and Glass UX resolve the same Router, React and Zod modules.
  This preparation deduplicates Router 1.170.43 and Zod 4.6.7; repeat the check
  after future upgrades because the library pins its Router runtime dependency.
- Rehearse member/admin differences and backend authorization with separate
  users. Presentation filters and hidden menu items never replace authorization.
- For final cleanup, search `web/src`, manifests and lockfile for HeroUI,
  React Aria, Tabler, obsolete custom table/material classes and unused RHF
  dependencies. Keep `motion`, fonts or other packages only while real consumers
  remain; do not remove Three/HLS/Apollo/router because the UI package is installed.

Done means all inventoried routes/dialogs use Glass primitives where available,
the retained media components use its design contracts, the adapter/gap backlog
is resolved, legacy generic UI is deleted, and browser/data/playback/device gates
have evidence. Package installation or a successful static typecheck alone is
not completion of the UI migration.

## Remaining-control cutover — 2026-10-26

The remaining generic controls now use public Glass APIs: LoginFlow with the existing
cookie/invite callbacks, provider/source/acquisition selects and search, player menus,
now-playing Sheet, transport buttons and volume Slider, folder rows, guide day tabs and
programme rows, alphabet buttons, and download confirmation checkboxes. HeroUI/React Aria
imports, providers, recipes and package dependencies are removed. Host `cn` uses clsx and
tailwind-merge. Existing form bridges still own their validation; new personal forms use
Glass useAppForm.

Librarian retains full-art navigation targets and the specialized buffered seek bar. The
installed generic Slider has no buffered-track/preview slot; replacing that composition
must preserve chapter ticks, preview, pointer commit and 5/30-second seek keys. This is an
intentional media composition, not a second generic component library. Media glyphs with
no public packaged equivalent may still use Tabler.

The personal-library routes use generated private CRUD/subscriptions for Watch Later,
manual lists/playlists, versioned smart rules, Home shelf preferences and favourites.
Saving personal viewing intent does not change Wanted or acquisition policy.

Verification passes: 71 Vitest files, 525 tests passed and one unchanged skip; full
TypeScript; actual tooling SDL export and codegen for all 51 public entities; production
Vite/PWA build; CSS variable/layout validation in seven stylesheets; Rust formatting and
whitespace checks. The existing large-chunk advisory remains.

Browser fixtures pass 120 interactions at 375/390/768/834 in light/dark Solid, including
touch-sized phone layouts, list/rule creation, saving, reordering, pinning, favourites,
menus, modal focus and authentication. Evidence is under
`web/tmp/screenshots/personal-browser/`; its GraphQL operations execute against isolated,
schema-validated fixtures. It does not qualify the running backend, real subscription
delivery, media decoding, Safari or physical devices. The backend/recovery VM gate is
recorded in `docs/schema-migrations.md`.
