# Librarian web style guide

Glass UX 0.11.7 owns the shared UI and appearance runtime. Follow the
[migration plan](../docs/glass-ux-migration-plan.md),
[shared design rules](../docs/glass-ux/ui-rules.md) and
[layout contract](../docs/glass-ux/layout-contract.md). The
[foundation progress record](../docs/glass-ux/foundation-progress.md) identifies
implemented components and the remaining compatibility boundary.

## Product layout

- Keep the full-screen media app: rail on desktop, bottom navigation on phones,
  persistent player, artwork shelves and full-bleed heroes. Use `page-gutter` for
  page content; preserve safe areas and dock offsets.
- Artwork and media facts carry the page. Use factual labels and reserve space for
  loading, artwork failures, validation and live results. Avoid repeated headings
  when navigation already names the view.
- Remote users need visible focus and reachable actions. Preserve `data-input-mode`
  independently of Glass appearance. TV mode multiplies the chosen UI scale and
  enlarges host poster geometry and navigation focus.

## Tokens, materials and typography

Import `glass-ux/styles.css` once through `src/styles/app.css`. Glass owns mode,
palette, fonts, density, glass amount, motion and geometry. Register host palette
parameters in `palettes.css`; do not reproduce complete themes. Device choices
persist under `librarian.appearance.v1`, migrated once from `librarian.theme`.

Use `Surface` material/radius/padding props and `PanelHeader`, `PanelBody` and
`PanelFooter` slots. Raw material classes supply neither content insets nor the
appropriate geometry. Repeated media tiles use card material; content over art
uses `.over-imagery`. Respect Solid mode. The old SVG filters, LiquidGlass hook
and custom backdrop filters are retired.

Use Glass typography roles and bundled fonts. The old `text-body-sm`,
`text-title-md`, `brand`, `foreground`, `surface` and similar names are temporary
aliases to Glass roles in `tokens.css`/`typography.css`, for existing callers.
Do not add independent values to these aliases or override library geometry.

## Controls, navigation and forms

Import new components from public Glass UX feature entries. The exported host
`Button`, `Dialog`, `ConfirmDialog`, `Panel`, state/skeleton, field and tab APIs
are temporary bridges for existing callers. They render library controls; do not
extend them into another generic design system. Host Button supports legacy
`onPress`, pending/disabled spellings and leading icons in children.

Routed tabs are links. Local tabs and Segmented keep their native composite
keyboard behavior. `RouteVerticalTabs` wraps the settings outlet so it measures
the complete pane: rail when wide, horizontal tabs with More when narrow. The outlet remains
available to spatial navigation. Inside dialogs, ordinary buttons use scoped
remote navigation; composites, text inputs and sliders own their arrows.

Existing React Hook Form schemas remain the validation owner while their field
bridge uses Glass inputs/selects/numbers/switches. Preserve blur/ref behavior and
nullable numeric values. New complete form migrations should use the library's
form bindings; remove the bridge when its final consumer moves.

## Tables and media compositions

Use public Glass DataTable for migrated data views. Logs, season episodes, album tracks and audiobook chapters now use it.
Typed host adapters translate its controlled state to generated camelCase
GraphQL variables; the bundled PascalCase ORM adapter is incompatible.
Preserve server filters, infinite loading, live updates, authorization and stable row IDs.
Main dataset routes use `fixedHeight` and a bounded `min-height: 0` flex chain.
Pass `height="fill"` to their table: search, column headings and the footer remain
inside the viewport while rows/cards scroll and load more internally. Reserve the
mobile navigation and active-player space through the shell; do not size tables
with a whole-screen height or scroll their footer away with the page. Poster
galleries use a Glass ScrollArea and keep their count footer outside it. Tables
embedded in scrolling artwork/detail pages use `height="auto"`. Failed refreshes
keep readable loaded rows and offer retry.

`components/ui/DataTable.tsx` is now a compatibility adapter over public Glass
DataTable. Its 21 feature consumers share that runtime; its old pager is removed.
Main-page tables sit directly on the page canvas, without an enclosing glass
surface or frame. The adapter defaults to `frame={false}`; do not add a Panel
solely to surround a primary table.
Complete datasets filter before incrementally rendering batches. Server lists send
filters to their query and supply both dataset identity and refresh/load revision.
Reset selection on a changed dataset, preserve it on append and ordinary refresh.
`LoadedDataTable` provides the complete-collection contract for media detail lists.
Its poster `renderCard` mode is a host media gallery, separate from the
Glass table's narrow row cards. `PosterCard` shows the artwork uncropped (2:3, or
square for albums) as the card itself, with the title and one meta line on the canvas
underneath and watched progress along the art's bottom edge; never fit a caption
inside the artwork's ratio. Keep posters, heroes, shelves, alphabet jump,
acquisition and playback behavior as product compositions over Glass primitives.
See [responsive audit](../docs/responsive-media-audit.md) for route coverage and
the release-source continuation limitation. Never filter a partial server batch
while presenting it as the complete dataset.

The pinned Glass 0.11.7 dependency has a small pnpm patch at
`patches/glass-ux@0.11.7.patch`: grid focus capture synchronizes the virtualizer's
active row after native focus, retaining alphabet-jump and cell focus while
scrolling. Its additive `infinite.revision` also rearms replaced batches without
using the semantic query key or resetting the reader. See [patch notes](patches/README.md).
Keep these reproducible fixes until a library release includes them.

## Media pages

Detail pages (`DetailShell`) and Home put the title's artwork behind the whole page,
not in a boxed banner. `MediaBackdrop` draws the backdrop under the transparent top
bar, fades it into the canvas and spreads a single static blur of the same image as
a colour wash (hidden in Solid). A poster is only ever used as that wash. With wide
art the `HeroBanner` fills exactly the first screen (`--view-h`, the scroller's
visible height), so the next section starts at the fold rather than half on screen;
without it (albums, books, missing art) the hero sizes to its text.

Hero actions lead with a large Play button (`data-tv-autofocus`); the other actions
come from the same action set through `ActionBar`, which is told the remaining room.
Play keeps its menu and command-palette entries. Short facts (director, genres) sit
under the overview with `HeroFacts`; technical file details stay in their panel.
Shows put a compact Seasons row (when there is more than one season; the show poster
stands in until season artwork exists) above the chosen season's episodes as a row of
stills, with the focused episode's summary in reserved space underneath. A still with a
file plays; a missing one opens its actions (members can still browse onto it). Seasons
and episodes form one section so they share the screen; the table is the Manage view.

Movie and TV libraries open on **Recommended** (`/libraries/$id/recommended`): a
spotlight over the title's art (drawn behind the library tabs through
`LibraryArtContext`), then shelves — continue watching, recently added, not watched yet,
newest releases and collections for films; continue watching, recently added and recently
updated for series. The spotlight and first shelf share the first screen: the block is
measured to the space under the tabs and the spotlight takes what the shelf leaves.
After those come genre shelves (`libraryGenreShelves`: the library's largest genres with
their newest titles, two or more titles each). **Browse** keeps the complete filterable wall as its own tab, so filters, sorting and the
letter jump stay pinned and in the URL; shelves link to it with "See all". Browse and the
other tabs are bounded panes (`fixedHeight` on each child route); Recommended scrolls.

## Artwork

Library artwork always comes from the local cache (`lib/artwork.ts`): never pass an
entity's provider URL to an image. Give `Artwork` a `displayWidth` (CSS px) so it asks
the cache for a stored size near the rendered width (`?w=`, density-aware); posters, stills,
portraits, the hero backdrop and its blurred wash all do. Seasons use their own cached
poster, with the show poster standing in until one exists. Provider search and discover
results that are not in the library keep their remote URLs.

## Remote focus

`lib/input-mode.tsx` owns TV behaviour; mark structure, do not script focus per page:

- `data-nav-section` on each hero/row. Up/Down moves to the nearest section, then the
  nearest row inside it; a remote reveals the whole section when it fits on screen.
- `data-nav-group` on a row or grid of cards: returning to it resumes at the card
  last focused there; the first entry lands on `data-nav-default` (the selected season,
  the next episode to watch). `MediaRow` and episode rows already set it. A remote
  reveals the largest enclosing section that fits on screen.
- `data-nav-zone` + `data-nav-side` for side rails (navigation, letters): entered and
  left with Left/Right only; leaving returns to the content control left behind.
  The top bar is a zone reached by Up when content has nothing further up.
- `data-tv-autofocus` names a page's starting control; `data-tv-skip` marks pointer
  shortcuts (a tile's play button, "See all") that a remote reaches another way. The
  remote's Play key plays a focused tile.
- In TV mode single-line fields, ⋯ menu buttons and horizontal tab strips never trap
  the remote; Backspace/Escape and the webOS/Tizen Back keys go back through in-app
  history, then Home. Focused tiles grow with a transform only.

## Motion

Routes cross-fade with the View Transitions API (`defaultViewTransition`). Only the
routed view moves; the rail and top bar are named layers that stay still (R6.3). An
opened tile's artwork morphs into the detail poster (`claimMorph`). Hero text rises
in and backdrops settle once; nothing loops, and every duration multiplies by
`--motion-scale`, so Still and reduced motion stop it all.

## Feedback, icons and compatibility

Glass `ActionProvider` owns Toaster and ConfirmHost; do not mount duplicate hosts.
Use `glass-ux/toast` for mutation results, inline field validation for forms, and
reserved status space for refresh/connection feedback. Confirm destructive actions.

Use `glass-ux/icons` for migrated presentation; let controls size their icons.
Existing Tabler icons remain where product compositions or library gaps require
them until their slice migrates.

HeroUI, React Aria components and their component recipes are retired. Do not
restore their providers, dependencies or styles. Host API bridges compose public
Glass controls; media-specific artwork targets and buffered seeking remain owned
by Librarian.
