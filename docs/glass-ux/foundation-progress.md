# Glass UX foundation implementation

2026-10-23. Follow-up: [populated media quality pass](media-quality-pass.md),
2026-10-24. Release: **0.9.10**. This is the first implementation slice of the
[migration plan](../glass-ux-migration-plan.md), assisted by GPT 6.1-sol agents
for primitives, fields/tabs, the Logs pilot, CSS integration, tests and review.

## Implemented

- One root appearance runtime: the Glass Vite plugin, shared CSS, bundled fonts,
  registered Cinema/Midnight/Ember/Forest palettes and existing Librarian mark.
  `librarian.theme` migrates once to `librarian.appearance.v1`; an existing Glass
  preference wins. The legacy key remains available for rollback.
- Appearance, tooltip, action and command-palette providers. ActionProvider mounts
  the single Toaster and ConfirmHost. All application toast producers now use the
  public Glass API. The palette supplies authenticated navigation commands.
- Glass shell surfaces, account Menu/Avatar, tooltips and brand rendering. Settings
  uses RouteVerticalTabs with the outlet inside the measured root, so wide panes
  get a rail and narrow panes get the library's picker.
- Appearance settings use SettingRow, SwatchPicker, Segmented and Slider for
  palette, mode, glass, density, motion, corners, borders and UI scale. Device/TV
  input mode remains independent; TV enlarges the selected scale, poster sizes
  and host navigation focus.
- Existing Button/GlassButton, switches/segmented controls, Dialog/ConfirmDialog,
  Panel, status, empty/error, spinner/skeleton, stat and key-value APIs now compose
  public Glass components. Legacy leading button icons use the library icon slot.
- React Hook Form field bridges use Glass Input/Textarea/NumberField/Select/Switch,
  preserving validation ownership, blur/ref forwarding and numeric nulls. Routed
  and local tabs, ViewToggle and OrderedChips use Glass controls. SourceDialog's
  remaining boolean controls also use Glass SettingRow/Switch.
- Logs uses the actual Glass DataTable, shared cells, level/module/message
  filters, server search/sort/paging, responsive cards and row detail dialogs.
  `lib/graphql/glass-table.ts` explicitly maps table state to generated camelCase
  variables. Live arrivals wait behind the library's count while scrolled. Failed
  refreshes preserve readable rows and expose retry. Clearing requires cancel-first
  confirmation, checks mutation success and returns to page one.
- Settings gives the table a bounded height and one row scroll owner. Spatial
  navigation respects native controls, popup triggers, grids, trees, menus, tabs
  and toggle groups. Ordinary modal actions remain reachable by arrows inside the
  active dialog. A modal-only capture fallback handles Base UI's propagation
  boundary without navigating behind the modal.
- Deleted the custom GlassFilters/LiquidGlass/refraction engine and optical CSS.
  Legacy material class callers now use Glass materials; Solid mode is respected.

## Temporary boundary

`legacy-components.css` imports individual recipes for remaining HeroUI controls,
with compatibility names derived from Glass roles in `tokens.css`. It imports no
HeroUI theme/base/full stylesheet. This explicit boundary allows the root cutover
without maintaining two root themes. Delete recipes/aliases as their callers move.
`tw-animate-css` is a direct build dependency for those remaining recipes.

The follow-up [responsive audit](../responsive-media-audit.md) replaces the shared
custom table runtime with public Glass DataTable for its **21 feature consumers**.
The temporary caller API remains until each slice adopts direct typed columns;
its poster gallery is a product composition. data-table-pro is
already absent. HeroUI/React Aria, React Hook Form and Tabler remain while their
consumers migrate; their removal belongs to the final cleanup slice.

Next: complete the general-settings form pilot and LoginFlow auth screens, then
media/library galleries and the remaining table/action migrations. Do not treat
shared field bridges as completion of every feature form or action workflow.

## Verification and limits

- Full frontend typecheck, generated operations, production/PWA build and
  `glass-ux check-layout src` pass. Checks run inside the shared 80% CPU cgroup.
- Full Vitest suite: **38 files, 302 passed, 3 unchanged existing skips**.
  Focused and aggregate tests cover one-time appearance conversion and precedence,
  OS mode changes, stored geometry/material settings, field validation, dialog
  dismissal/focus return, modal remote movement, bare Glass composites and legacy
  Select popup keys, server table variables, filters, live arrivals and clear/retry.
- Browser regression audit: the same **35 desktop destinations** as the source
  inventory, plus **five phone destinations** at 390px. No new page errors or
  document-width overflow. The intentionally missing-file watch route exercises
  the error screen, not successful playback.
- Seventeen matrix/interaction checks and an additional real SourceDialog popup/boolean
  check pass. Browser checks cover bounded table scrolling, row Enter/Escape focus return,
  Cancel/Confirm arrow movement, cancel-first confirmation, held live updates,
  command navigation, persisted appearance, TV sizing and phone row cards. The
  geometry matrix uses light/dark × Full/Solid with compact density, custom UI
  scale, corners and borders. Solid panels compute no backdrop filter.
- Review fixed outlet-wide spatial exclusion, lost TV scaling, cached-row loss on
  refetch error and modal/popup arrow handling. The Select regression uses actual
  HeroUI/React Aria controls inside an actual Glass dialog.
- Screenshots are local artifacts under `web/screenshots/glass-foundation/`;
  ignored QA scripts/results are under `tmp/glass-foundation-*`. Synthetic data
  and credentials exist only in the disposable sandbox. The sandbox uses the
  existing debug backend; it does **not** qualify the new Rust dependency runtime.
  The disposable sandbox servers were stopped after verification.
- The sandbox's existing `WebSocket closed without opened.` startup error also
  occurs in the pre-migration baseline and is recorded separately. Physical TV
  devices, playable media/cast flows, service-worker update installation and all
  destructive feature workflows remain later qualification work.
- ORM/auth runtime tests, schema export and disposable database/backup migration
  rehearsal remain the release gates documented in the
  [dependency upgrade record](../glass-ux-dependency-upgrade.md). No live database
  migration, backend deployment, commit or PR was performed in this slice.
