# Librarian site and component inventory

Reviewed 2026-10-23 against the current `web/` source. The retired `frontend/`
is not an implementation target. Route tables include layouts and redirects so
none of the router tree is silently omitted.

## Source scope

- 50 route TSX files, including root/layout/guard and redirect nodes.
- 31 shared UI TSX files under `components/ui/`.
- 24 feature components render the current custom DataTable.
- 50 non-test source components import HeroUI directly; 33 import its toast API.
- Raw presentation includes 38 button, 3 input, 1 select, 9 image and 1 table
  element occurrences in the inspected non-test TSX source. Native image/media
  elements may remain in product compositions where the library has no equivalent.

## Every route binding

`_app`, `_auth` and `_player` are pathless TanStack layout IDs, not URL segments.
`$libraryId` and the other parameters remain the host routing contract.

| Route source (`web/src/routes/`) | Router ID | Feature / behavior | Slice |
| --- | --- | --- | --- |
| [__root.tsx](../../web/src/routes/__root.tsx) | `__root` | Root / layout / outlet | 3 |
| [_app/activity/index.tsx](../../web/src/routes/_app/activity/index.tsx) | `/_app/activity/` | activity/ActivityPage | 9 |
| [_app/activity.tsx](../../web/src/routes/_app/activity.tsx) | `/_app/activity` | Root / layout / outlet | 9 |
| [_app/albums/$albumId.tsx](../../web/src/routes/_app/albums/$albumId.tsx) | `/_app/albums/$albumId` | media/AlbumPage | 8 |
| [_app/artists/$artistId.tsx](../../web/src/routes/_app/artists/$artistId.tsx) | `/_app/artists/$artistId` | media/ArtistPage | 8 |
| [_app/audiobooks/$audiobookId.tsx](../../web/src/routes/_app/audiobooks/$audiobookId.tsx) | `/_app/audiobooks/$audiobookId` | media/AudiobookPage | 8 |
| [_app/collections/$collectionId.tsx](../../web/src/routes/_app/collections/$collectionId.tsx) | `/_app/collections/$collectionId` | media/CollectionPage | 8 |
| [_app/downloads/index.tsx](../../web/src/routes/_app/downloads/index.tsx) | `/_app/downloads/` | downloads/DownloadsPage | 9 |
| [_app/downloads.tsx](../../web/src/routes/_app/downloads.tsx) | `/_app/downloads` | Root / layout / outlet | 9 |
| [_app/index.tsx](../../web/src/routes/_app/index.tsx) | `/_app/` | home/HomePage | 11 |
| [_app/libraries/$libraryId/albums.tsx](../../web/src/routes/_app/libraries/$libraryId/albums.tsx) | `/_app/libraries/$libraryId/albums` | libraries/browser/AlbumsBrowser | 6–7 |
| [_app/libraries/$libraryId/artists.tsx](../../web/src/routes/_app/libraries/$libraryId/artists.tsx) | `/_app/libraries/$libraryId/artists` | libraries/browser/ArtistsBrowser | 6–7 |
| [_app/libraries/$libraryId/audiobooks.tsx](../../web/src/routes/_app/libraries/$libraryId/audiobooks.tsx) | `/_app/libraries/$libraryId/audiobooks` | libraries/browser/AudiobooksBrowser | 6–7 |
| [_app/libraries/$libraryId/collections.tsx](../../web/src/routes/_app/libraries/$libraryId/collections.tsx) | `/_app/libraries/$libraryId/collections` | libraries/browser/CollectionsBrowser | 6–7 |
| [_app/libraries/$libraryId/files.tsx](../../web/src/routes/_app/libraries/$libraryId/files.tsx) | `/_app/libraries/$libraryId/files` | libraries/browser/FilesBrowser | 6–7 |
| [_app/libraries/$libraryId/index.tsx](../../web/src/routes/_app/libraries/$libraryId/index.tsx) | `/_app/libraries/$libraryId/` | libraries/LibraryLayout | 6–7 |
| [_app/libraries/$libraryId/movies.tsx](../../web/src/routes/_app/libraries/$libraryId/movies.tsx) | `/_app/libraries/$libraryId/movies` | libraries/browser/MoviesBrowser | 6–7 |
| [_app/libraries/$libraryId/scans.tsx](../../web/src/routes/_app/libraries/$libraryId/scans.tsx) | `/_app/libraries/$libraryId/scans` | libraries/ScansPanel | 6–7 |
| [_app/libraries/$libraryId/settings.tsx](../../web/src/routes/_app/libraries/$libraryId/settings.tsx) | `/_app/libraries/$libraryId/settings` | libraries/LibrarySettingsForm | 6–7 |
| [_app/libraries/$libraryId/shows.tsx](../../web/src/routes/_app/libraries/$libraryId/shows.tsx) | `/_app/libraries/$libraryId/shows` | libraries/browser/ShowsBrowser | 6–7 |
| [_app/libraries/$libraryId/tracks.tsx](../../web/src/routes/_app/libraries/$libraryId/tracks.tsx) | `/_app/libraries/$libraryId/tracks` | libraries/browser/TracksBrowser | 6–7 |
| [_app/libraries/$libraryId.tsx](../../web/src/routes/_app/libraries/$libraryId.tsx) | `/_app/libraries/$libraryId` | libraries/LibraryLayout | 6–7 |
| [_app/libraries/index.tsx](../../web/src/routes/_app/libraries/index.tsx) | `/_app/libraries/` | libraries/LibrariesPage | 6–7 |
| [_app/libraries.tsx](../../web/src/routes/_app/libraries.tsx) | `/_app/libraries` | Root / layout / outlet | 6–7 |
| [_app/movies/$movieId.tsx](../../web/src/routes/_app/movies/$movieId.tsx) | `/_app/movies/$movieId` | media/MoviePage | 8 |
| [_app/search/index.tsx](../../web/src/routes/_app/search/index.tsx) | `/_app/search/` | search/SearchPage | 11 |
| [_app/search.tsx](../../web/src/routes/_app/search.tsx) | `/_app/search` | Root / layout / outlet | 11 |
| [_app/settings/about.tsx](../../web/src/routes/_app/settings/about.tsx) | `/_app/settings/about` | settings/AboutSettings | 10 |
| [_app/settings/backup.tsx](../../web/src/routes/_app/settings/backup.tsx) | `/_app/settings/backup` | settings/BackupSettings | 10 |
| [_app/settings/casting.tsx](../../web/src/routes/_app/settings/casting.tsx) | `/_app/settings/casting` | settings/CastingSettings | 10 |
| [_app/settings/downloads.tsx](../../web/src/routes/_app/settings/downloads.tsx) | `/_app/settings/downloads` | settings/DownloadsSettings | 10 |
| [_app/settings/general.tsx](../../web/src/routes/_app/settings/general.tsx) | `/_app/settings/general` | settings/GeneralSettings | 5 |
| [_app/settings/index.tsx](../../web/src/routes/_app/settings/index.tsx) | `/_app/settings/` | Redirect / layout / guard | 10 |
| [_app/settings/libraries.tsx](../../web/src/routes/_app/settings/libraries.tsx) | `/_app/settings/libraries` | settings/LibrariesSettings | 10 |
| [_app/settings/logs.tsx](../../web/src/routes/_app/settings/logs.tsx) | `/_app/settings/logs` | settings/LogsSettings | 4 |
| [_app/settings/metadata.tsx](../../web/src/routes/_app/settings/metadata.tsx) | `/_app/settings/metadata` | settings/MetadataSettings | 10 |
| [_app/settings/organization.tsx](../../web/src/routes/_app/settings/organization.tsx) | `/_app/settings/organization` | settings/OrganizationSettings | 10 |
| [_app/settings/quality.tsx](../../web/src/routes/_app/settings/quality.tsx) | `/_app/settings/quality` | settings/QualitySettings | 10 |
| [_app/settings/sources.tsx](../../web/src/routes/_app/settings/sources.tsx) | `/_app/settings/sources` | settings/SourcesSettings | 10 |
| [_app/settings/users.tsx](../../web/src/routes/_app/settings/users.tsx) | `/_app/settings/users` | settings/UsersSettings | 10 |
| [_app/settings.tsx](../../web/src/routes/_app/settings.tsx) | `/_app/settings` | settings/SettingsLayout | 3 |
| [_app/shows/$showId.tsx](../../web/src/routes/_app/shows/$showId.tsx) | `/_app/shows/$showId` | media/ShowPage | 8 |
| [_app/wanted/index.tsx](../../web/src/routes/_app/wanted/index.tsx) | `/_app/wanted/` | wanted/WantedPage | 9 |
| [_app/wanted.tsx](../../web/src/routes/_app/wanted.tsx) | `/_app/wanted` | Root / layout / outlet | 9 |
| [_app.tsx](../../web/src/routes/_app.tsx) | `/_app` | Redirect / layout / guard | 3 |
| [_auth/login.tsx](../../web/src/routes/_auth/login.tsx) | `/_auth/login` | auth/LoginForm | 5 |
| [_auth/register.tsx](../../web/src/routes/_auth/register.tsx) | `/_auth/register` | auth/RegisterForm | 5 |
| [_auth.tsx](../../web/src/routes/_auth.tsx) | `/_auth` | Root / layout / outlet | 5 |
| [_player/watch/$mediaFileId.tsx](../../web/src/routes/_player/watch/$mediaFileId.tsx) | `/_player/watch/$mediaFileId` | player/WatchPage | 12 |
| [_player.tsx](../../web/src/routes/_player.tsx) | `/_player` | Redirect / layout / guard | 12 |

## Every shared UI file

The [plan](../glass-ux-migration-plan.md#shared-component-replacement-map) gives the
replacement decision, including components that remain host-owned compositions.

| File (`web/src/components/ui/`) | Exported components and contracts |
| --- | --- |
| [AlphabetRail.tsx](../../web/src/components/ui/AlphabetRail.tsx) | `AlphabetRail` |
| [Artwork.tsx](../../web/src/components/ui/Artwork.tsx) | `ArtworkAspect`, `Artwork` |
| [BackdropCard.tsx](../../web/src/components/ui/BackdropCard.tsx) | `BackdropCardProps`, `BackdropCard` |
| [BrandMark.tsx](../../web/src/components/ui/BrandMark.tsx) | `BrandMark` |
| [Button.tsx](../../web/src/components/ui/Button.tsx) | `ButtonVariant`, `ButtonSize`, `ButtonProps`, `Button` |
| [ConfirmDialog.tsx](../../web/src/components/ui/ConfirmDialog.tsx) | `ConfirmDialog` |
| [DataTable.tsx](../../web/src/components/ui/DataTable.tsx) | `Breakpoint`, `DataTableColumn`, `DataTableRowAction`, `DataTableBulkAction`, `DataTableProps`, `DataTable` |
| [Dialog.tsx](../../web/src/components/ui/Dialog.tsx) | `Dialog` |
| [EmptyState.tsx](../../web/src/components/ui/EmptyState.tsx) | `EmptyState` |
| [ErrorState.tsx](../../web/src/components/ui/ErrorState.tsx) | `ErrorState` |
| [Field.tsx](../../web/src/components/ui/Field.tsx) | `FormTextField`, `FormNumberField`, `SelectOption`, `FormSelectField`, `SelectList`, `FormSwitchField`, `FieldGroup` |
| [GlassButton.tsx](../../web/src/components/ui/GlassButton.tsx) | `GlassButton` |
| [GlassControls.tsx](../../web/src/components/ui/GlassControls.tsx) | `GlassSegment`, `GlassSegmented`, `GlassSwitch` |
| [GlassFilters.tsx](../../web/src/components/ui/GlassFilters.tsx) | `GlassFilters` |
| [HeroBanner.tsx](../../web/src/components/ui/HeroBanner.tsx) | `HeroBanner` |
| [KeyValue.tsx](../../web/src/components/ui/KeyValue.tsx) | `KeyValueItem`, `KeyValueList` |
| [LiquidGlass.tsx](../../web/src/components/ui/LiquidGlass.tsx) | `LiquidGlass` |
| [MediaRow.tsx](../../web/src/components/ui/MediaRow.tsx) | `MediaRow` |
| [MetaDot.tsx](../../web/src/components/ui/MetaDot.tsx) | `MetaLine` |
| [PageHeader.tsx](../../web/src/components/ui/PageHeader.tsx) | `PageHeader` |
| [Panel.tsx](../../web/src/components/ui/Panel.tsx) | `Panel` |
| [PosterCard.tsx](../../web/src/components/ui/PosterCard.tsx) | `PosterCardProps`, `PosterCard` |
| [PosterMarquee.tsx](../../web/src/components/ui/PosterMarquee.tsx) | `PosterMarquee` |
| [Section.tsx](../../web/src/components/ui/Section.tsx) | `Section` |
| [Skeletons.tsx](../../web/src/components/ui/Skeletons.tsx) | `SkeletonBlock`, `SkeletonText`, `SkeletonCard`, `SkeletonRow`, `SkeletonGrid`, `SkeletonHero` |
| [Spinner.tsx](../../web/src/components/ui/Spinner.tsx) | `Spinner` |
| [StatTile.tsx](../../web/src/components/ui/StatTile.tsx) | `StatTile` |
| [StatusChip.tsx](../../web/src/components/ui/StatusChip.tsx) | `StatusChip` |
| [Tabs.tsx](../../web/src/components/ui/Tabs.tsx) | `TabItem`, `SegmentTabs`, `SideTabs` |
| [ThemePicker.tsx](../../web/src/components/ui/ThemePicker.tsx) | `ThemePicker` |
| [Toolbar.tsx](../../web/src/components/ui/Toolbar.tsx) | `ViewMode`, `ViewToggle`, `Toolbar` |

## Feature composition consumers

Each row identifies the shared UI/HeroUI contract that must move with the screen
or dialog. Local helpers and data hooks stay with the owning feature.

| Feature file (`web/src/features/`) | Existing presentation dependencies |
| --- | --- |
| [acquisition/AcquisitionChip.tsx](../../web/src/features/acquisition/AcquisitionChip.tsx) | `StatusChip` |
| [acquisition/AcquisitionDialog.tsx](../../web/src/features/acquisition/AcquisitionDialog.tsx) | `Label`, `Select`, `toast`, `Button`, `Dialog`, `GlassSegmented`, `GlassSwitch`, `SelectList` |
| [activity/ActivityPage.tsx](../../web/src/features/activity/ActivityPage.tsx) | `toast`, `Button`, `DataTable`, `DataTableBulkAction`, `DataTableColumn`, `DataTableRowAction`, `EmptyState`, `ErrorState`, `PageHeader`, `SegmentTabs`, `StatusChip` |
| [auth/LoginForm.tsx](../../web/src/features/auth/LoginForm.tsx) | `Alert`, `Button`, `FieldGroup`, `FormTextField` |
| [auth/RegisterForm.tsx](../../web/src/features/auth/RegisterForm.tsx) | `Alert`, `Button`, `FieldGroup`, `FormTextField` |
| [downloads/AddTorrentDialog.tsx](../../web/src/features/downloads/AddTorrentDialog.tsx) | `toast`, `Button`, `Dialog`, `FieldGroup`, `FormSelectField`, `FormTextField` |
| [downloads/DownloadsPage.tsx](../../web/src/features/downloads/DownloadsPage.tsx) | `toast`, `Button`, `ConfirmDialog`, `DataTable`, `DataTableColumn`, `DataTableRowAction`, `EmptyState`, `ErrorState`, `GlassSegmented`, `PageHeader`, `StatTile`, `StatusChip` |
| [downloads/LinkTorrentDialog.tsx](../../web/src/features/downloads/LinkTorrentDialog.tsx) | `ListBox`, `ListBoxItem`, `Select`, `toast`, `Button`, `Dialog` |
| [downloads/ReleaseSearchDialog.tsx](../../web/src/features/downloads/ReleaseSearchDialog.tsx) | `SearchField`, `Select`, `ListBox`, `ListBoxItem`, `Tooltip`, `toast`, `Button`, `DataTable`, `DataTableColumn`, `Dialog`, `EmptyState`, `ErrorState`, `GlassSegmented`, `Spinner`, `StatusChip` |
| [downloads/TorrentDetail.tsx](../../web/src/features/downloads/TorrentDetail.tsx) | `KeyValueList`, `Panel`, `StatusChip` |
| [files/FolderPickerDialog.tsx](../../web/src/features/files/FolderPickerDialog.tsx) | `Button`, `Dialog`, `EmptyState`, `Spinner` |
| [home/HomePage.tsx](../../web/src/features/home/HomePage.tsx) | `BackdropCard`, `Button`, `EmptyState`, `MediaRow`, `PosterCard`, `Section` |
| [home/MovieReleases.tsx](../../web/src/features/home/MovieReleases.tsx) | `ListBox`, `ListBoxItem`, `Select`, `toast`, `EmptyState`, `ErrorState`, `MediaRow`, `PosterCard`, `Section`, `SkeletonRow` |
| [home/TvGuide.tsx](../../web/src/features/home/TvGuide.tsx) | `Artwork`, `EmptyState`, `GlassSegmented`, `SkeletonBlock` |
| [libraries/LibrariesPage.tsx](../../web/src/features/libraries/LibrariesPage.tsx) | `Button`, `EmptyState`, `ErrorState`, `PageHeader`, `SkeletonBlock` |
| [libraries/LibraryCard.tsx](../../web/src/features/libraries/LibraryCard.tsx) | Host layout/media controls; inspect raw DOM and local imports |
| [libraries/LibraryLayout.tsx](../../web/src/features/libraries/LibraryLayout.tsx) | `Tooltip`, `Button`, `PageHeader`, `SegmentTabs`, `TabItem` |
| [libraries/LibrarySettingsForm.tsx](../../web/src/features/libraries/LibrarySettingsForm.tsx) | `toast`, `Button`, `ConfirmDialog`, `FieldGroup`, `FormNumberField`, `FormSelectField`, `FormSwitchField`, `FormTextField`, `Panel` |
| [libraries/ManualMatchDialog.tsx](../../web/src/features/libraries/ManualMatchDialog.tsx) | `toast`, `Button`, `Dialog`, `EmptyState`, `Spinner`, `StatusChip` |
| [libraries/ScansPanel.tsx](../../web/src/features/libraries/ScansPanel.tsx) | `toast`, `Button`, `DataTable`, `DataTableColumn`, `DataTableRowAction`, `EmptyState`, `ErrorState`, `Panel`, `StatTile`, `StatusChip` |
| [libraries/browser/AlbumsBrowser.tsx](../../web/src/features/libraries/browser/AlbumsBrowser.tsx) | `Button`, `DataTable`, `DataTableColumn`, `EmptyState`, `ErrorState`, `PosterCard` |
| [libraries/browser/ArtistsBrowser.tsx](../../web/src/features/libraries/browser/ArtistsBrowser.tsx) | `DataTable`, `EmptyState`, `ErrorState`, `PosterCard`, `DataTableColumn` |
| [libraries/browser/AudiobooksBrowser.tsx](../../web/src/features/libraries/browser/AudiobooksBrowser.tsx) | `Button`, `DataTable`, `DataTableColumn`, `EmptyState`, `ErrorState`, `PosterCard` |
| [libraries/browser/BrowserToolbar.tsx](../../web/src/features/libraries/browser/BrowserToolbar.tsx) | `SearchField`, `AlphabetRail`, `GlassSegmented`, `ViewToggle`, `ViewMode` |
| [libraries/browser/CollectionsBrowser.tsx](../../web/src/features/libraries/browser/CollectionsBrowser.tsx) | `Button`, `DataTable`, `DataTableColumn`, `EmptyState`, `ErrorState`, `PosterCard` |
| [libraries/browser/FilesBrowser.tsx](../../web/src/features/libraries/browser/FilesBrowser.tsx) | `toast`, `DataTable`, `EmptyState`, `ErrorState`, `GlassSegmented`, `StatusChip`, `DataTableColumn`, `DataTableRowAction` |
| [libraries/browser/MoviesBrowser.tsx](../../web/src/features/libraries/browser/MoviesBrowser.tsx) | `Button`, `DataTable`, `DataTableColumn`, `EmptyState`, `ErrorState`, `PosterCard`, `StatusChip` |
| [libraries/browser/ShowsBrowser.tsx](../../web/src/features/libraries/browser/ShowsBrowser.tsx) | `Button`, `DataTable`, `DataTableColumn`, `EmptyState`, `ErrorState`, `PosterCard` |
| [libraries/browser/TracksBrowser.tsx](../../web/src/features/libraries/browser/TracksBrowser.tsx) | `DataTable`, `EmptyState`, `ErrorState`, `StatusChip`, `DataTableColumn` |
| [library-add/ProviderSearchDialog.tsx](../../web/src/features/library-add/ProviderSearchDialog.tsx) | `Checkbox`, `SearchField`, `toast`, `Artwork`, `Button`, `Dialog`, `EmptyState`, `Spinner` |
| [media/AlbumPage.tsx](../../web/src/features/media/AlbumPage.tsx) | `toast`, `Button`, `ConfirmDialog`, `DataTable`, `ErrorState`, `GlassButton`, `KeyValueList`, `MetaLine`, `Panel`, `SkeletonHero`, `StatusChip`, `DataTableColumn` |
| [media/ArtistPage.tsx](../../web/src/features/media/ArtistPage.tsx) | `ErrorState`, `MetaLine`, `PosterCard`, `Section`, `SkeletonHero` |
| [media/AudiobookPage.tsx](../../web/src/features/media/AudiobookPage.tsx) | `toast`, `Button`, `ConfirmDialog`, `DataTable`, `ErrorState`, `GlassButton`, `KeyValueList`, `MetaLine`, `Panel`, `SkeletonHero`, `StatusChip`, `DataTableColumn` |
| [media/CollectionPage.tsx](../../web/src/features/media/CollectionPage.tsx) | `ErrorState`, `MetaLine`, `PosterCard`, `Section`, `SkeletonHero` |
| [media/DetailShell.tsx](../../web/src/features/media/DetailShell.tsx) | `HeroBanner` |
| [media/EpisodeList.tsx](../../web/src/features/media/EpisodeList.tsx) | `toast`, `Artwork`, `Button`, `EmptyState`, `SkeletonBlock`, `StatusChip` |
| [media/FileDetails.tsx](../../web/src/features/media/FileDetails.tsx) | `toast`, `Button`, `KeyValueList`, `Panel`, `StatusChip` |
| [media/MoviePage.tsx](../../web/src/features/media/MoviePage.tsx) | `toast`, `Artwork`, `ConfirmDialog`, `ErrorState`, `KeyValueList`, `MediaRow`, `MetaLine`, `Panel`, `PosterCard`, `Section`, `SkeletonHero`, `StatusChip`, `GlassButton` |
| [media/ShowPage.tsx](../../web/src/features/media/ShowPage.tsx) | `toast`, `ConfirmDialog`, `ErrorState`, `GlassButton`, `KeyValueList`, `MetaLine`, `Panel`, `SegmentTabs`, `SkeletonHero` |
| [player/NowPlayingSheet.tsx](../../web/src/features/player/NowPlayingSheet.tsx) | `Drawer`, `Artwork` |
| [player/PlayerDock.tsx](../../web/src/features/player/PlayerDock.tsx) | `Artwork` |
| [player/VideoPlayer.tsx](../../web/src/features/player/VideoPlayer.tsx) | `Spinner`, `Dropdown` |
| [player/WatchPage.tsx](../../web/src/features/player/WatchPage.tsx) | Host layout/media controls; inspect raw DOM and local imports |
| [player/controls.tsx](../../web/src/features/player/controls.tsx) | Host layout/media controls; inspect raw DOM and local imports |
| [search/ProviderResults.tsx](../../web/src/features/search/ProviderResults.tsx) | `ListBox`, `ListBoxItem`, `Select`, `toast`, `Artwork`, `Button`, `EmptyState`, `Spinner` |
| [search/SearchPage.tsx](../../web/src/features/search/SearchPage.tsx) | `SearchField`, `Button`, `EmptyState`, `MediaRow`, `PageHeader`, `PosterCard`, `Section`, `SegmentTabs`, `SkeletonRow` |
| [settings/AboutSettings.tsx](../../web/src/features/settings/AboutSettings.tsx) | `KeyValueList`, `Panel` |
| [settings/BackupSettings.tsx](../../web/src/features/settings/BackupSettings.tsx) | `toast`, `Button`, `ConfirmDialog`, `DataTable`, `DataTableColumn`, `DataTableRowAction`, `EmptyState`, `Panel`, `StatTile` |
| [settings/BlockedReleases.tsx](../../web/src/features/settings/BlockedReleases.tsx) | `toast`, `DataTable`, `DataTableColumn`, `DataTableRowAction`, `EmptyState`, `Panel`, `StatusChip` |
| [settings/CastingSettings.tsx](../../web/src/features/settings/CastingSettings.tsx) | `toast`, `Button`, `ConfirmDialog`, `DataTable`, `DataTableColumn`, `DataTableRowAction`, `Dialog`, `EmptyState`, `FieldGroup`, `FormNumberField`, `FormSwitchField`, `FormTextField`, `Panel`, `StatusChip` |
| [settings/DownloadsSettings.tsx](../../web/src/features/settings/DownloadsSettings.tsx) | `toast`, `Button`, `FieldGroup`, `FormNumberField`, `FormSwitchField`, `FormTextField`, `Panel` |
| [settings/GeneralSettings.tsx](../../web/src/features/settings/GeneralSettings.tsx) | `FieldGroup`, `FormNumberField`, `FormTextField`, `Panel`, `ThemePicker` |
| [settings/LibrariesSettings.tsx](../../web/src/features/settings/LibrariesSettings.tsx) | `toast`, `Button`, `DataTable`, `DataTableColumn`, `Dialog`, `EmptyState`, `FieldGroup`, `FormSelectField`, `FormSwitchField`, `FormTextField`, `Panel` |
| [settings/LogsSettings.tsx](../../web/src/features/settings/LogsSettings.tsx) | `toast`, `Button`, `DataTable`, `DataTableColumn`, `EmptyState`, `ErrorState`, `Panel`, `SegmentTabs`, `StatusChip` |
| [settings/MetadataSettings.tsx](../../web/src/features/settings/MetadataSettings.tsx) | `toast`, `Button`, `FieldGroup`, `FormNumberField`, `FormSwitchField`, `FormTextField`, `KeyValueList`, `Panel` |
| [settings/OrderedChips.tsx](../../web/src/features/settings/OrderedChips.tsx) | `Input`, `Label`, `ListBox`, `ListBoxItem`, `Select`, `TextField`, `Button` |
| [settings/OrganizationSettings.tsx](../../web/src/features/settings/OrganizationSettings.tsx) | `toast`, `Button`, `ConfirmDialog`, `DataTable`, `DataTableColumn`, `DataTableRowAction`, `Dialog`, `EmptyState`, `FieldGroup`, `FormSelectField`, `FormSwitchField`, `FormTextField`, `Panel`, `StatusChip` |
| [settings/QualitySettings.tsx](../../web/src/features/settings/QualitySettings.tsx) | `toast`, `Button`, `ConfirmDialog`, `DataTable`, `DataTableColumn`, `DataTableRowAction`, `Dialog`, `EmptyState`, `FieldGroup`, `FormNumberField`, `FormSelectField`, `FormSwitchField`, `FormTextField`, `Panel`, `StatusChip` |
| [settings/SettingsForm.tsx](../../web/src/features/settings/SettingsForm.tsx) | `toast`, `Button` |
| [settings/SettingsLayout.tsx](../../web/src/features/settings/SettingsLayout.tsx) | `PageHeader`, `SideTabs`, `TabItem` |
| [settings/SourceDialog.tsx](../../web/src/features/settings/SourceDialog.tsx) | `Description`, `FieldError`, `Input`, `Label`, `ListBox`, `ListBoxItem`, `Select`, `Switch`, `TextField`, `toast`, `Button`, `Dialog`, `FieldGroup` |
| [settings/SourcesSettings.tsx](../../web/src/features/settings/SourcesSettings.tsx) | `toast`, `Button`, `ConfirmDialog`, `DataTable`, `DataTableColumn`, `DataTableRowAction`, `EmptyState`, `Panel`, `StatusChip` |
| [settings/UsersSettings.tsx](../../web/src/features/settings/UsersSettings.tsx) | `toast`, `Button`, `ConfirmDialog`, `DataTable`, `DataTableColumn`, `DataTableRowAction`, `Dialog`, `EmptyState`, `FieldGroup`, `FormNumberField`, `FormSelectField`, `Panel`, `StatusChip` |
| [wanted/WantedPage.tsx](../../web/src/features/wanted/WantedPage.tsx) | `toast`, `Artwork`, `Button`, `DataTable`, `DataTableColumn`, `DataTableRowAction`, `EmptyState`, `ErrorState`, `PageHeader`, `SegmentTabs`, `StatusChip` |

## All 24 table consumers

These include settings dialogs and media sublists, not only top-level lists.

- `activity/ActivityPage.tsx`
- `downloads/DownloadsPage.tsx`
- `downloads/ReleaseSearchDialog.tsx`
- `libraries/ScansPanel.tsx`
- `libraries/browser/AlbumsBrowser.tsx`
- `libraries/browser/ArtistsBrowser.tsx`
- `libraries/browser/AudiobooksBrowser.tsx`
- `libraries/browser/CollectionsBrowser.tsx`
- `libraries/browser/FilesBrowser.tsx`
- `libraries/browser/MoviesBrowser.tsx`
- `libraries/browser/ShowsBrowser.tsx`
- `libraries/browser/TracksBrowser.tsx`
- `media/AlbumPage.tsx`
- `media/AudiobookPage.tsx`
- `settings/BackupSettings.tsx`
- `settings/BlockedReleases.tsx`
- `settings/CastingSettings.tsx`
- `settings/LibrariesSettings.tsx`
- `settings/LogsSettings.tsx`
- `settings/OrganizationSettings.tsx`
- `settings/QualitySettings.tsx`
- `settings/SourcesSettings.tsx`
- `settings/UsersSettings.tsx`
- `wanted/WantedPage.tsx`

## Shell and runtime dependencies

- `components/shell/`: AppShell, NavRail, BottomTabs, Breadcrumbs, TopBar, UserMenu,
  ConnectionBanner, UpdatePrompt, NotFound and RouteError; preserve nav/count hooks.
- `app/providers.tsx`: Apollo, theme, input mode, React Aria router and HeroUI toast
  outlet. Glass provider/router placement and one toast owner are foundation work.
- `lib/theme.tsx`, `styles/{tokens,glass,overrides,typography,utilities,motion}.css`:
  current root theme, custom refraction/material engine and host geometry.
- `lib/input-mode.tsx`: global spatial navigation must yield to Base UI composites
  and portaled modal focus scopes.
- `features/player/`: native/HLS media source, queue/session store, progress sync and
  Media Session remain; transport/seek/volume/sheets/menus are presentation work.
- `components/constellation/` and `components/three/`: optional host ambience and
  movie/cast landing identity. Keep motion/performance/attribution requirements.
- `lib/apollo/`, `graphql/`, browser connection hooks, auth session and preferences
  are host data contracts. Glass UX does not replace them.

## Browser baseline and limitations

T3 preview could not launch: “Chrome will not start its sandbox as root”. The
documented repository Playwright workflow was used as fallback. The fixture sandbox
was created empty under `/tmp/librarian-sandbox`, not copied from the live media DB.
Synthetic libraries/media/notifications were inserted through GraphQL, with scanning,
organization and acquisition disabled for fixture libraries. No real media files
or live records were changed. The temporary sandbox services were stopped afterward.

- Desktop: 35 authenticated destinations, covering every feature/settings page,
  all eight library browsers, six media-detail kinds and a missing-media watch route.
- Phone: home, libraries, movie browser, general settings and show detail at 390×844.
- Public: sign-in screenshot from the live site. Auth forms and invite/setup
  conditions were also reviewed in source.
- Evidence: `web/screenshots/glass-audit/01.png` through `35.png`, phone captures
  in that folder, `web/screenshots/glass-audit-login.png`, and page text/
  headings/errors in ignored `tmp/glass-site-audit.json`. The local fixture/browser
  driver is `tmp/glass-site-audit.mjs`; its API origin is fixed to sandbox :3003.

The screenshots confirm current composition and fallback states, not production
qualification. The sandbox used the existing debug backend binary. Home provider
release rows showed errors without configured provider credentials; fixture art
used placeholders. Navigation recorded “WebSocket closed without opened.” page
errors, so subscription/reconnect behavior is explicitly unqualified. There was no
playable fixture: WatchPage was inspected in missing-media state and player behavior
was reviewed in source. No real Cast device, provider download, restore, TV webview
or destructive dialog action was exercised. New-backend runtime qualification is
a separate dependency-upgrade gate, not implied by this baseline.
