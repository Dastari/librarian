# Guide backend API

The backend owns schedule and release discovery. Frontend GraphQL code generation should be rerun against the updated schema; neither frontend is modified by this backend change.

## TV schedule

The `schedule_sync` service populates the existing `scheduleCaches` query at startup when stale, then daily. It fetches today through today + 6 days in UTC, using each configured country's TVmaze broadcast/local-streaming schedule plus the global streaming schedule. Set `LIBRARIAN_SCHEDULE_COUNTRIES=US,GB` in the backend environment to change the default `US` (maximum eight countries). UK is `GB` in this API.

```graphql
query Guide($from: String!, $to: String!, $country: String!, $offset: Int!) {
  scheduleCaches(
    where: { countryCode: { eq: $country }, airDate: { gte: $from, lte: $to } }
    orderBy: [{ airStamp: ASC }, { showName: ASC }]
    page: { limit: 100, offset: $offset }
  ) {
    edges {
      node {
        id
        tvmazeEpisodeId
        tvmazeShowId
        showName
        episodeName
        season
        episodeNumber
        episodeType
        airDate
        airTime
        airStamp
        runtime
        showNetwork
        showPosterUrl
        episodeImageUrl
        showGenres
        countryCode
      }
    }
    pageInfo { totalCount }
  }
}
```

`airStamp` is newly sortable. `airDate` remains the provider's calendar date. A null timestamp means no precise time was provided; don't invent midnight. Unnumbered specials have `episodeNumber: 0`. Use `tvmazeEpisodeId`, rather than season/number, as their provider identity.

A country partition includes that country's broadcasts/local streaming and global streaming. An episode appearing in both feeds is stored once per country. When multiple countries are configured, select a country or deduplicate by `tvmazeEpisodeId` in an aggregate view. Pagination matters: a seven-day window commonly exceeds 100 rows.

The job retains 14 days of past entries and removes cancelled/rescheduled entries from each successfully refreshed window. If a provider fetch fails, the previous cache is preserved. Failed passes retry hourly. `scheduleSyncStates` (admin read) exposes `countryCode`, `lastSyncedAt`, `lastSyncDays`, and `syncError`; failure does not advance the last successful timestamp. No show is automatically added to a library by this sync. For the existing add action, call `addTvShow(libraryId: ..., input: { tvmazeId: ..., autoDownloadMode: ... })`, passing `tvmazeShowId` as `tvmazeId`. Choose the monitoring mode explicitly (`NONE`, `WANTED`, or `ALL`); omitting it defaults to `ALL`.

TVmaze documents the [broadcast schedule](https://www.tvmaze.com/api#schedule) and [global/local streaming distinction](https://www.tvmaze.com/api#web-schedule). The integration supports both `show` and `_embedded.show` response shapes.

## Movie releases outside the library

```graphql
query MovieReleaseGuide($kind: MovieReleaseKind!, $region: String, $page: Int!) {
  movieReleases(kind: $kind, region: $region, page: $page) {
    provider
    providerId
    title
    originalTitle
    year
    releaseDate
    overview
    posterUrl
    backdropUrl
    imdbId
    voteAverage
    popularity
  }
}
```

- `MovieReleaseKind`: `NOW_PLAYING` or `UPCOMING`.
- `region`: optional two-letter ISO country code, normalized to uppercase. Omit to use TMDB's default list.
- `page`: defaults to `1`; accepted range `1..500`. The result is one TMDB page, not the entire catalogue. An empty page ends pagination.
- Returns existing `MovieSearchResult` objects, now with nullable `releaseDate` (`YYYY-MM-DD`). `provider` is `tmdb`; `providerId` is the movie ID for the existing `addMovie(libraryId: ..., input: { tmdbId: ..., monitored: ... })` action.
- Requires a signed-in member and the configured TMDB API key. Provider/configuration failures are GraphQL errors, not false empty lists.
- Cached for one day, separately by list kind, normalized region, and page. Adult entries are excluded.

The lists use TMDB's [now playing](https://developer.themoviedb.org/reference/movie-now-playing-list) and [upcoming](https://developer.themoviedb.org/reference/movie-upcoming-list) endpoints. These are theatrical release lists, not a guarantee of streaming or downloadable availability. `releaseDate` is the movie release date TMDB supplies in that list. Re-releases can retain an older original release date, so use list membership for the row category rather than discarding upcoming entries whose `releaseDate` is in the past.

## Library episode air times

`Episode.airStamp: String` is a new nullable, filterable, sortable field. Include it in library episode queries and use `orderBy: [{ airStamp: ASC }]` when ordering known air times. Continue to use `airDate` for calendar-day grouping and place unknown times in a separate “time to be announced” group.

TVmaze import/refresh saves the timestamp on both episode creation and update. The schedule worker also backfills missing timestamps on existing TVmaze-linked library episodes at startup/daily. It only updates `airStamp`; wanted flags, file links, and playback data are preserved. Episodes without a known TVmaze identity or provider timestamp remain null.

## Public sign-in artwork

```graphql
query SignInArtwork {
  showcaseArtwork(limit: 12) {
    title
    posterUrl
  }
}
```

This query is public. It returns up to 12 posters by default (limit clamped to 1..24), deduplicated from the public TV schedule cache. URLs are restricted to TVmaze's HTTPS medium-sized poster images. It does not read users' libraries, reveal ownership, or require a TMDB key. Before the initial schedule sync succeeds it can return `[]`, so retain the existing fallback scene.

## Schema/storage change

`Episode.airStamp` adds one nullable column through the normal entity schema sync on backend startup. No manual SQL migration is required. Existing schedule entities and their generated CRUD/subscriptions are retained. All job writes use generated GraphQL entity mutations; reads use the existing typed entity query layer.

## Verification on 2026-09-06

The deployed backend populated 690 US/global-streaming schedule entries for September 6–12 and backfilled air times for 89 TVmaze-linked library episodes. Both movie list kinds returned 20 results for region US; a repeated upcoming request succeeded from cache. Anonymous `showcaseArtwork` returned 12 posters, while anonymous schedule access was rejected.

Six focused guide integration tests, the schema smoke test, and two existing TMDB tests passed. All GraphQL examples above validate against the generated schema. The server executable built under the aggregate 80% CPU quota and passed its readiness check after restart.
