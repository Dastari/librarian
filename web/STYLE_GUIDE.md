# Librarian web style guide

This document is the visual contract for `web/`. Every screen follows it; nothing here is
optional. Tokens live in `src/styles/tokens.css`; components that implement the rules live in
`src/components/ui`.

## Principles

1. **Full bleed.** No centred max-width container. Pages fill the viewport and use
   `page-gutter` for horizontal breathing room. Hero art and poster rows may bleed to the edge
   with `bleed-gutter`.
2. **Content first.** Artwork carries the design. Chrome is glass, translucent and quiet.
3. **One rhythm.** Spacing steps are 4px based. Section gaps are `gap-8` (desktop) and `gap-6`
   (mobile). Card padding is `p-4`; dense list rows are `py-2.5`.
4. **Remote friendly.** Everything focusable has a visible ring (`nav-focus`) and is reachable
   with arrow keys. Hover is an enhancement, never the only way to reach an action.
5. **Plain language.** Headings name the thing ("Movies", "Downloads"). No marketing copy, no
   explanatory sub-headings unless the content is genuinely ambiguous.

## Tokens

| Group | Tokens | Notes |
| --- | --- | --- |
| Background | `bg-background`, `bg-background-secondary`, `bg-background-tertiary` | Page canvas |
| Surface | `bg-surface`, `bg-surface-secondary`, `bg-surface-tertiary`, `bg-surface-hover` | Cards, panels |
| Overlay | `bg-overlay` | Popovers, menus, modals |
| Text | `text-foreground`, `text-muted` | Only two text colours plus status colours |
| Brand | `bg-brand`, `text-brand`, `bg-brand-soft` | Warm gold. Primary action and focus |
| Accent | `bg-accent`, `text-accent` | Blue. Links and informational state |
| Status | `success`, `warning`, `danger`, and `status-*` aliases | Mapped from `ContentStatus` in `lib/status.ts` |
| Media types | `media-movies`, `media-tv`, `media-music`, `media-audiobooks` | Tinting placeholders and icons |
| Glass | `glass-chrome`, `glass-surface`, `glass-control`, `glass-brand`, `glass-highlight` | See "Liquid glass" below |
| Radius | `rounded-lg` (controls), `rounded-card`, `rounded-poster`, `rounded-pill` | Never mix radii inside one element |
| Shadow | `shadow-surface`, `shadow-overlay`, `shadow-poster`, `shadow-poster-hover`, `shadow-dock` | |
| Motion | `duration-fast/base/slow`, `ease-fluid`, `animate-rise-in` | Respect `prefers-reduced-motion` |

## Liquid glass

Every translucent surface uses one of three tiers from `src/styles/glass.css`; nothing draws its
own backdrop blur.

| Tier | Where | Recipe |
| --- | --- | --- |
| `glass-chrome` | Navigation rail, bottom tabs, top bar, player dock | 26px blur, 180% saturation, refraction filter |
| `glass-surface` | Dialogs, drawers, popovers, panels, cards, tables | 20px blur, top reflection, bottom inner shadow, drop shadow |
| `glass-control` | Buttons, inputs, switches, segmented controls, chips | 12px blur, crisp rim, fine refraction filter |

The refraction comes from two SVG filters mounted once by `GlassFilters` (fractal-noise
displacement through `backdrop-filter: url()`); browsers without support get the same blur and
saturation without the warp. `glass-brand` tints a control with the brand colour (primary
buttons, active pills, checked switches). The page canvas is `ambient-canvas`, soft colour
fields that give the glass something to bend. Themes tune the fill, rim, reflection and ambient
colours through tokens; components never hard-code glass values.

## Typography

Use only the scale classes: `text-display-xl/lg/md`, `text-title-lg/md/sm`, `text-body-lg`,
`text-body`, `text-body-sm`, `text-label`, `text-label-sm`, `text-overline`. Numbers in tables
and stats use `text-numeric`. Display and title classes use the Outfit face; body uses Inter.

## Layout

- **Shell:** left navigation rail (`NavRail`) on tablet and desktop, bottom tab bar
  (`BottomTabs`) on phones, top bar (`TopBar`) with breadcrumbs, search and account.
- **Player dock:** persistent `PlayerDock` at the bottom; pages reserve space with
  `pb-dock` when a session is active.
- **Page header:** `PageHeader` with title, optional eyebrow, actions on the right. One per page.
- **Sections:** `Section` renders an `h2` in `text-title-lg` plus an optional trailing link.
- **Tabs:** horizontal tabs (`Tabs`) for peer views within a page; vertical tabs
  (`SideTabs`) only in settings-style pages with many sections. On phones, vertical tabs
  collapse into a horizontally scrollable strip.
- **Breadcrumbs:** derived from the route tree via `staticData.crumb` or loader data. Always
  shown in the top bar; never duplicated inside the page.

## Cards

- **Poster card** (`PosterCard`): 2:3 for movies, shows, books; 1:1 for albums and artists.
  Title and one meta line below. Progress bar at the bottom edge when partially watched.
  Status badge top-left; actions appear on hover or focus.
- **Backdrop card** (`BackdropCard`): 16:9 for episodes and continue-watching.
- **Panel** (`Panel`): flat surface card with `p-4`, used in settings and detail pages.

## Data tables

`DataTable` (our own, in `components/ui/DataTable.tsx`) is the only list component. It renders
sortable columns, responsive column hiding, selection with bulk actions, row action menus, server
pagination and a card grid through `renderCard`. Card mode reuses `PosterCard`/`BackdropCard`, so
list and grid views always agree. Filters live in the URL (`useBrowserFilters`), the view mode in
device preferences.

## Buttons

- `Button` from `components/ui` is the only button. `primary` (brand glass) for the single main
  action of a view, `secondary` (glass) for supporting actions, `ghost` for toolbar icons,
  `danger`/`danger-soft` for destructive actions after confirmation.
- `GlassButton` is the pill variant used over artwork (hero, player).
- Icon-only buttons always have `aria-label`.

## Feedback

- Loading: skeletons shaped like the final content. Never a full-page spinner.
- Empty: `EmptyState` with icon, one sentence, and a primary action when one exists.
- Errors: `ErrorState` inline where the data would have been, with a retry.
- Toasts for the result of a mutation; inline validation for forms.

## Icons

`@tabler/icons-react` only. Sizes: 16 in buttons and rows, 20 in navigation, 24 in headers,
40+ in empty states. Media-type icons use the media tint; status icons use status colours.
