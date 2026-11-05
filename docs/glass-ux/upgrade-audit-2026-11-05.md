# Glass UX 0.16.1 upgrade audit

Upgraded from 0.13.1 on 2026-11-05. The package, lockfile, patch, skills, UI rules, icon
specification and migration notes moved together. This records where each addition since 0.13.1
fits Librarian, including where it does not.

## Adopted

| Component | Where | Replaces |
| --- | --- | --- |
| `AppShell`, `ShellBrand` | `components/shell/AppShell.tsx` | `SidebarProvider` and a hand-built frame |
| `Rail` | `components/shell/NavRail.tsx` | `Sidebar` in icon mode, with its expand toggle |
| `TopBar`, `SearchTrigger`, `AccountButton` | `components/shell/TopBar.tsx`, `UserMenu.tsx` | Custom bar, separate bell and bare avatar |
| `RouteBreadcrumb` | `components/shell/Breadcrumbs.tsx` | Custom trail that hid crumbs by viewport |
| `TabBar` | `components/shell/BottomTabs.tsx` | Custom phone bar |
| `VideoPlayer`, `CastAdapter` | `features/player/VideoPlayer.tsx`, `useCastAdapter.ts` | 448 lines of custom controls |
| `EntityIdentity`, `FileIcon` | `features/files/FileIdentity.tsx` | `IdentityCell` with one generic glyph |
| Icons | 20 files moved from Tabler to `glass-ux/icons` | Tabler where an exact match exists |

## Good fit, not done yet

| Component | Where it would go | Why |
| --- | --- | --- |
| `AudioPlayer` / `useMediaPlayer` | `features/player/PlayerDock.tsx` | Same core as video. The dock's queue and phone sheet need the headless form, so it is its own change. |
| `FileDialog` over `FileProvider` | `features/files/FolderPickerDialog.tsx` | Replaces a custom folder picker with paging, path and keyboard handling. |
| `FactList` | `KeyValueList` callers (10 files) | The same label and value rows, already used by `TorrentDetail`. |
| `Rating` | Title facts and personal ratings | Replaces the hand-written star and score. |
| `ImageLightbox` / `ImageViewer` | Poster and backdrop on title pages | Zoom and pan on artwork. |
| `Carousel` | `HomeSpotlight` | Replaces custom previous, next and count controls with swipe and snap. Check TV focus first. |
| `ActivityFeed` `variant="timeline"` | Scan runs, a download's history | Those histories are event lists. |
| `NotificationPopover` | The account menu's Activity item | Reads and acts on notifications without leaving the page. |
| `StatusBar` | Table and gallery count footers | Only if footers gain more than a count. |

## No current fit

`PaneLayout`, `Menubar`, `Tree` editing, `Scheduler`, `Kanban`, `RichTextInput`, `Typeahead`, `CommentThread`,
`ReactionBar`, `EmojiPicker`, `FindBar`, `DiffView`, `ColorWheel`, `GradientPicker`, `ImageCropper`, `CompareSlider`,
`Masonry`, `SignaturePad`, `Tour`, the new chart types, and the image-editing icons. Librarian is a routed media app
with no document editing, and nothing in the product needs these today.

## Left on Tabler

Media kinds Glass has no glyph for: `IconMovie`, `IconDeviceTv`, `IconDisc`, `IconMicrophone2`, `IconBooks`,
`IconUserStar`. Also icons passed as values to host bridges such as `EmptyState`, which accept only Tabler's type until
those bridges migrate.

## Behaviour to review

- Dark Flat canvases are darker in every palette (0.16.1).
- Bare Glass `AppShell` keeps its own preferences under `workspace.shell.v1`. The old `librarian.sidebar` key is unused.
