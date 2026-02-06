# Frontend GraphQL Types Audit

## Summary

`frontend/src/lib/graphql/types.ts` currently mixes:
1. UI view-model types (keep)
2. Schema DTO duplicates that should come from codegen (migrate)
3. Legacy exports with no practical consumers (remove)

## Keep (UI-specific, not direct schema mirrors)

- `TrackWithStatus`
- `AlbumWithTracks`
- `AudiobookWithChapters`
- `MediaFileDetails` and stream/detail shapes (`VideoStreamInfo`, `AudioStreamInfo`, etc.)
- `LibraryTypeInfo` + `LIBRARY_TYPES` + `getLibraryTypeInfo`
- Formatting/helper-oriented view-models used by table/card components

## Migrate To Codegen (high value)

These are schema-facing and should be imported from `generated/graphql` via document nodes:

- Core media entities: `TvShow`, `Movie`, `Album`, `Track`, `Audiobook`, `AudiobookChapter`, `Episode`
- Notifications domain: `Notification`, `NotificationType`, `NotificationCategory`, `NotificationResolution`, `PaginatedNotifications`
- Casting domain: `CastDevice`, `CastSession`, `CastSettings`, mutation result payloads
- RSS domain: `RssFeed`, `RssFeedResult`, `RssFeedTestResult`
- Playback domain: `PlaybackSession`, `PlaybackResult`, `StartPlaybackInput`, `UpdatePlaybackInput`, `PlaybackContentType`

## Remove Candidates (legacy/dead)

Audit identified many exports that are not referenced as active app imports and are candidates for removal once migration is complete, including examples like:

- `AuthResult`, `LogoutResult`
- `MediaItem`, `DownloadsTorrentRow`
- `SetMatchResult`
- `ParsedEpisodeInfo`, `PaginatedLogResult`, `ClearLogsResult`
- `UpcomingEpisode*`/`LibraryUpcomingEpisode*` helper interfaces
- `NotificationEvent*` legacy event interfaces

## Important constraint

Directly pruning the index type surface in one pass caused broad breakage because many files still import from `lib/graphql` instead of `generated/graphql`. Migration must be phased by domain.

## Recommended phased decommission

1. Notifications: switch all imports to generated notification types and drop notification type aliases from `types.ts`.
2. Casting: switch cast pages/hooks to generated cast types and remove custom cast DTO aliases.
3. Media details: move `TvShow`/`Movie`/`Album`/`Audiobook` imports to generated types where possible.
4. After each domain: remove old `types.ts` exports and `index.ts` re-exports for that domain.

