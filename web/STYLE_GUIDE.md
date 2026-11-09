# Librarian web style guide

Glass UX 0.19.1 owns the shared UI and appearance runtime. Follow the
[migration plan](../docs/glass-ux-migration-plan.md),
[shared design rules](../docs/glass-ux/ui-rules.md) and
[layout contract](../docs/glass-ux/layout-contract.md). The
[foundation progress record](../docs/glass-ux/foundation-progress.md) identifies
implemented components and the remaining compatibility boundary.

## Product layout

- The frame is Glass `AppShell` with its `Rail`, `TopBar` and phone `TabBar`, all on the canvas:
  the bar has no material, so artwork shows through it. Use `page-gutter` for page content;
  preserve safe areas and dock offsets. Installed on iOS the app draws under the status bar
  (`black-translucent`): anything at an edge pads with `--safe-top`, `--safe-bottom` and the
  inline `--safe-inline-start`/`--safe-inline-end` (a landscape notch), and full-screen
  layers use `dvh`, never `vh`. Check portrait and landscape with emulated insets.
  `styles/native.css` holds the installed-app and touch behaviour (no double-tap zoom, no callout or
  selection on chrome, no keyboard hint on the search field). The dev server serves the web manifest
  (so the live site installs standalone) but registers no service worker; production builds add the worker.
- The document never scrolls and neither does the frame. Each page has one `PageScroller`. Routed
  tabs sit outside it, so they stay put while the view under them scrolls. A bounded pane clips
  (`overflow: clip`); `overflow: hidden` can still be scrolled from code by a focus or a reveal.
- Artwork and media facts carry the page. Use factual labels and reserve space for
  loading, artwork failures, validation and live results. Avoid repeated headings
  when navigation already names the view.
- Remote users need visible focus and reachable actions. Preserve `data-input-mode`
  independently of Glass appearance. TV mode multiplies the chosen UI scale and
  enlarges host poster geometry and navigation focus.

## Tokens, materials and typography

Import `glass-ux/styles.css` once through `src/styles/app.css`. Glass owns mode,
palette, fonts, density, glass amount, motion and geometry. Librarian's appearance is
fixed (`lib/glass/appearance.ts` locks it): the Cinema palette (`palettes.css`, the only
one), full glass, comfortable density, system motion and the default radius, line weight
and interface size. Two choices remain. Dark or Light (Dark by default, never following
the system) persists under `librarian.appearance.v1`; anything else stored there is
dropped. Blur is a device setting under `librarian.blur.v1`, for everyone on that browser:
off, Glass renders with `glassRendering: "none"`. Do not add palettes or appearance knobs.

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

## Settings pages

Routes under `/settings` are site settings: pages, with no card or panel around anything. Group with
space and a label (R15.8), and let the settings navigation name the page, so no page repeats its
title. Build them from three shared pieces in `features/settings`:

- `SettingsPage` stacks the page's groups `gap-8` apart.
- `SettingsSection` is one group. **Rows** (`width="readable"`) get the overline group label and
  are held to `--settings-measure` (52rem, the `settings-measure` utility), left-aligned with the
  page gutter. They stay left-aligned on wide screens so a table below them on the same page shares
  their left edge. **Records** (the default, `width="full"`) get a title row (one line of help,
  actions on the right) above a full-width table, the way every entity table sits on the canvas.
  A form wrapping several row groups puts `settings-measure` on the `Form` once, and its
  `FormActions` end the page in the same column.
- `SettingFields` holds the row-shaped bound fields for `useAppForm`: `NumberRow`, `PercentRow`,
  `DurationRow`, `ByteSizeRow`, `TextRow`, `UrlRow`, `SelectRow` and `TagsRow`, beside the library's
  `field.SwitchRow` and `field.SegmentedRow`. Each is a `SettingRow` (label and help left, control
  right, the validation message in the description slot) over the bare Glass control, so the control
  keeps its value contract. Pass `settingsFormBehaviour` instead of `formBehaviour`, because the
  library's focus-first-invalid only looks inside `Field`s.

Choose the control by what the value is: a switch for a boolean, a number with a `unit` for a count,
`DurationRow` for an interval stored in seconds, `PercentRow` for a ratio stored as a fraction,
`ByteSizeRow` for sizes, `UrlRow` for an address, chips for a short list, a select for a choice with
detail and segmented for two to four short ones. Keep a stored contract as it is: convert at the
form boundary, never in the schema. Secrets are `TextRow type="password" autoComplete="off"`,
because the password input would let a browser fill the account password into an API key.
Read-only facts are `SettingRow`s with the value as the control (see About).

Lists of records are tables (`LoadedDataTable`, `height="auto"` on these scrolling pages, or `fill`
for an unbounded feed such as Logs) with their `EntityIdentity` and `ActionSet`; adding or editing
one opens its dialog. A row's control wraps under its text in a narrow row and takes the full width
there (`SettingFields` sizes controls by the row's own container, not the viewport), so phones need
no separate layout. Check a new page at 390px, a laptop, 2560px, TV mode and Light.

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

Filters belong to their table. Search, column filters and saved views (Unread, Needs action) all
sit in the table's toolbar, through `search`, column `filter` and `toolbar`. Above a table go only
routed tabs, page actions and a filter that drives several things at once.

Server-backed views compose public Glass DataTable directly. Complete datasets
use `components/ui/LoadedDataTable.tsx` for local filtering and progressive reveal.
Main-page tables sit directly on the page canvas, without an enclosing glass
surface or frame; do not add a Panel solely to surround a primary table.
Complete datasets filter before incrementally rendering batches. Server lists send
filters to their query and supply both dataset identity and refresh/load revision.
Reset selection on a changed dataset, preserve it on append and ordinary refresh.
All populated tables expose selection and share one typed `ActionSet` between row,
right-click and selection menus. Set `bulk: true` on single-target actions too when
they belong in a one-row selection, retaining their `rows.length === 1` guard.
Use `TableBulkActions` to keep the footer's actions reachable in narrow panes.
Controlled server tables use `useTableSelection`; complete local tables use
`LoadedDataTable` (or `useSelectableTableState` for a fully loaded reorderable list).
Prune completed selections against the full filtered collection, before incremental
rendering. Never reset them to a literal empty object on each render or persist them
in the URL. Read-only tables offer meaningful copy/export actions; mutations retain
permission checks, protected-row reasons, one confirmation and honest partial results.
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

The pinned Glass dependency has a small pnpm patch at `patches/glass-ux@0.19.1.patch`: grid
focus capture synchronizes the virtualizer's active row after native focus, retaining
alphabet-jump and cell focus while scrolling. Its additive `infinite.revision` rearms replaced
batches without using the semantic query key or resetting the reader, and `setDuration` lets the
player show a growing playlist's real length. See [patch notes](patches/README.md).
Keep these reproducible fixes until a library release includes them.

## Media pages

Detail pages (`DetailShell`) put the title's artwork behind the whole window, not in a boxed
banner. `MediaBackdrop` renders into the layer behind the shell, so the picture reaches under the
rail and the top bar. It does not scroll: it recedes as the page scrolls over it. It fades into
the canvas and spreads a single static blur of the same image as a colour wash (hidden in Solid).
A poster is only ever used as that wash. With wide
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

The TV guide groups episodes by show, never one chip per episode. Home's guide section
(`GuideSpotlight`) puts the next of your shows to air over its feathered backdrop with a
countdown and its release dates in batches (a weekly double bill is one row), your other
upcoming shows beneath it, then "New tonight" from the TVmaze schedule (new series and
premieres first; news, talk, sports and year-numbered daily shows filtered out). It links to
`/guide`, which swaps between **Calendar** (a month of your shows with a day panel; the six
week rows share the first screen through `.guide-day`) and **Streaming grid** (services ×
days, defaulting to the subscription streamers listed that week, with the chosen services
remembered per browser). `?view=grid` keeps the choice in the URL.

Home is shelves from the first line, as in Plex: continue watching, libraries and what is new.
It does not preview one title; that belongs to a library's Recommended view.

## Titles you don't have

A movie, show or audiobook that is not in a library opens a normal page at `/discover/...`, built on
`DetailShell`. Details come from `providerMovie`, `providerShow` and `providerAudiobook`, read through
the metadata cache; pictures come from the artwork cache as `provider-*` entities, so nothing is
fetched from a provider twice. A title already in a library redirects to its real page. **Add to
library** is the one action: it adds straight away, then asks whether to choose a release, download
automatically or leave it. Search, the TV guide and the release shelves all link here. Albums have no
such page yet and still add from their search card.

## Detail dialogs

`EntityDetailDialog` is the one layout for a record's details: the record's `EntityIdentity` at its header tier with its actions on the same line (a long name truncates, the actions don't move), what matters next, and everything else in tabs that scroll under it, at a fixed height so changing tab never resizes it. A download shows its transfer as `Metric`s and a `Gauge` and its files as a `DataTable`, one row per file with what matching did with it.

## Files

A media file has one identity (`FileIdentity`), one action set (`useFileActions`) and one dialog
(`FileDialogHost`, mounted in the shell). A file row, a file chip and `openFileDialog(id)` all open
that dialog. "Delete file" takes an administrator and removes the file from disk as well as the library
(`deleteMediaFileFromDisk`). The record goes first and the file last, and a path outside the library's
root is refused before either.

## Video

`features/player/VideoPlayer.tsx` uses Glass `VideoPlayer`. Librarian supplies the stream through
`attach` (`useMediaSource`), watch progress, tracks, chapters and casting through a `CastAdapter`
(`useCastAdapter`, over the server's cast sessions). On a remote, Down reaches the controls, Left and
Right walk them and Up returns to the picture, where Left and Right seek.

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
  shortcuts (a tile's play button, "See all") that a remote reaches another way. `data-nav-skip`
  marks pointer-only controls (a row's scroll arrows) that arrow keys never stop on, in any
  input mode; give them `tabIndex={-1}` too. The
  remote's Play key plays a focused tile.
- In TV mode single-line fields, ⋯ menu buttons and horizontal tab strips never trap
  the remote; Backspace/Escape and the webOS/Tizen Back keys go back through in-app
  history, then Home. Focused tiles grow with a transform only.
- A card is one stop. Secondary buttons on it (a poster's play button, a library card's favourite
  star) are pointer-only: `tabIndex={-1}` and `data-nav-skip`, with their action also in the
  card's menu or on its Play key. Walking across cards never lands between them.
- TV mode trims the app chrome: the top bar loses search, Commands, Cast and the account (the
  rail has Search; the account is its last item, opening the same menu beside the rail; casting
  is in the players' own controls) and keeps the breadcrumbs. Every scroller that clips (a rail's
  list, a poster row) pads its ends by `--focus-reach` so the first and last item's ring is
  never cut off; pad or `scroll-padding`, do not remove the clip.
- A number field (Glass `NumberField`) never traps Up/Down in TV mode either; OK on it starts an
  adjusting state (`data-tv-adjusting`) in which Up/Down step the value, and OK or Back ends it
  (design.md Q107).
- **Media keys** (`MediaPlayPause`, `MediaPlay`, `MediaPause`, `MediaStop`, `MediaTrackNext`,
  `MediaTrackPrevious`, `MediaFastForward`, `MediaRewind`) are handled once, in
  `features/player/mediaKeys.ts`, on the window before any control: they work wherever focus is.
  They go to the video player while it holds the screen, otherwise to the audio player. Do not
  handle them in a component; a player registers what it offers (`videoKeyTarget`, `audioScrub`).
- **Now playing:** on a TV the rail's first entry is what is playing (artwork and title) while
  anything is playing or paused. It, the remote's **Info** key and **N** on a keyboard
  (`nowPlaying.tsx`) open the video player, or the now playing drawer with Play/Pause focused.
  Starting audio on a TV (an album, a track, a book, a playlist, Play next with nothing playing)
  opens the drawer the same way; Add to queue does not. Back (Escape) collapses it and returns
  focus to the control that opened it; Back or Up from the bar returns to the page control focus
  came from. The bar is never a trap.
- **Scrubbing:** on a seek bar, or the video picture with its controls hidden, Left/Right (and
  Rewind/Fast-forward from anywhere) move a target with a preview (time, delta, storyboard tile,
  chapter), faster while held: 10 s, then 30 s, then a minute or two. OK seeks, a pause of 0.7 s
  seeks, Back puts it back; it is one seek, however many presses. Up/Down leave the bar (mark a
  slider `data-owns-horizontal` for that). A non-modal dialog (`aria-modal="false"`) does not hold
  the arrows.

## Motion

Routes cross-fade with the View Transitions API (`defaultViewTransition`). Only the
routed view moves (R6.3): AppShell takes the root out of the transition, so the rail, top bar and
artwork stay live, and the view is named only while a transition runs. An
opened tile's artwork morphs into the detail poster (`claimMorph`). Hero text rises
in and backdrops settle once; nothing loops, and every duration multiplies by
`--motion-scale`, so Still and reduced motion stop it all.

## Feedback, icons and compatibility

Glass `ActionProvider` owns Toaster and ConfirmHost; do not mount duplicate hosts.
Use `glass-ux/toast` for mutation results, inline field validation for forms, and
reserved status space for refresh/connection feedback. Confirm destructive actions.

Use `glass-ux/icons` for migrated presentation; let controls size their icons.
Tabler remains only where Glass has no equivalent (film, disc, microphone, books and similar media
kinds) or where an icon is passed as a value to a host bridge that has not migrated.

HeroUI, React Aria components and their component recipes are retired. Do not
restore their providers, dependencies or styles. Host API bridges compose public
Glass controls; media-specific artwork targets and buffered seeking remain owned
by Librarian.
