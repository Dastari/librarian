# RSS Feeds Feature Contract

## Status

Planned. The repository currently has `RssFeed` and `RssFeedItem` entities in the GraphQL ORM schema, but it does not yet have a complete RSS polling, digesting, matching, or auto-download implementation.

A sanitized IPTorrents-style RSS XML fixture is checked in at `docs/sample-data/movie-sample.xml`. The old `legacy/sample-data/movie-sample.xml` path must not be reintroduced.

## Purpose

RSS feeds let Librarian ingest release listings from trackers that either:

- only expose recent releases through an RSS feed;
- do not provide a searchable API or scrapeable search page;
- provide RSS as a faster discovery channel than live search.

RSS feeds are not intended to replace live source search for trackers that support reliable search. They are a complementary source: Librarian polls feeds frequently, stores every seen release locally, and then treats the stored feed items as searchable source results for auto-download and manual search flows.

## User Outcomes

- A user can add one or more RSS feeds.
- Librarian periodically polls enabled feeds.
- Each feed poll upserts newly seen items without duplicating previously seen releases.
- If a tracker feed only exposes the latest N items, Librarian keeps older seen items locally so missed UI sessions do not lose release history.
- Missing or wanted media can be matched against the local RSS item cache during automatic acquisition.
- RSS-only trackers can still participate in the same source search and download workflow as normal searchable sources.
- Users can test a feed before saving it and can manually trigger a poll.
- Failed polls are visible and retried without permanently disabling the feed unless the user disables it.

## Product Model

### RSS Feed

`RssFeed` is the user-configured source endpoint.

Required behavior:

- Owned by `UserId`.
- May optionally be scoped to a `LibraryId`.
- Has a display `Name`.
- Has a `Url`.
- Can be enabled or disabled.
- Has a `PollIntervalMinutes`.
- Tracks `LastPolledAt`, `LastSuccessfulAt`, `LastError`, and `ConsecutiveFailures`.
- Has a `PostDownloadAction` for future tracker-specific handling if needed.

### RSS Feed Item

`RssFeedItem` is the locally retained release listing from a feed.

Required behavior:

- Belongs to one `FeedId`.
- Stores raw identity fields: `Guid`, `Link`, `LinkHash`, `TitleHash`.
- Stores display/search fields: `Title`, `Description`, `PubDate`.
- Stores parsed release fields where available:
  - show name, season, episode;
  - resolution, codec, source, audio, HDR;
  - future extensions: year, movie title, album/artist, book/audiobook title.
- Stores processing state:
  - `Processed`;
  - optional `TorrentId`;
  - optional `SkippedReason`;
  - `SeenAt`.

## Architecture

RSS should be implemented as a persisted source cache plus a source adapter.

```text
RSS feed URL
  -> RssFeedService poller
  -> parse RSS/Atom items
  -> normalize + hash + upsert RssFeedItem
  -> RssFeedItemSource searches local cache
  -> SourceRelease candidates
  -> AutoDownloadService chooses release
  -> AddTorrent / source authenticated download
  -> media pipeline links downloaded files
```

### Services

#### RssFeedService

Lifecycle service owned by `ServicesManager`.

Responsibilities:

- Load enabled `RssFeed` entities.
- Poll feeds according to `PollIntervalMinutes`.
- Allow manual polling by feed id.
- Allow feed testing without saving.
- Parse and upsert feed items.
- Update feed health fields.
- Emit enough logs to identify feed id, feed name, URL, item count, inserted count, skipped count, and failure reason.

The service must use GraphQL ORM generated operations for all entity reads and writes. It must not add direct SQL.

#### RssFeedItemSource

Source adapter that presents stored RSS items as `SourceRelease` results.

Responsibilities:

- Search `RssFeedItem` rows by title and parsed metadata.
- Respect optional library scoping from the parent `RssFeed`.
- Return releases in the same shape as normal source searches.
- Prefer unprocessed items, but allow manual search to see processed items when requested later.
- Support TV and movie search first. Music/audiobook matching can be added after the core workflow is proven.

#### AutoDownloadService

New acquisition service that finds wanted or missing media and searches sources.

Responsibilities:

- Run event-driven when media is added or marked wanted.
- Run scheduled catch-up scans.
- Query missing/wanted movies, episodes, tracks, and chapters through GraphQL ORM/generated resolvers.
- Search live sources via `SourcesManager`.
- Search RSS cache via `RssFeedItemSource`.
- Rank candidates.
- Add the selected torrent through the torrent service.
- Mark related RSS items `Processed=true` and store `TorrentId` when a release is selected.
- Mark skipped RSS items with `SkippedReason` only when the item was considered and rejected for a durable reason, such as wrong library type, unsupported link, or already downloaded.

## Feed Polling Contract

### Poll Frequency

- Each feed has `PollIntervalMinutes`.
- Minimum interval should be clamped to a safe lower bound, initially 5 minutes.
- Failed feeds should still be retried, but with backoff based on `ConsecutiveFailures`.
- Manual poll should bypass schedule timing but still use rate limiting.

### HTTP Behavior

- Use the shared rate-limited HTTP client pattern where possible.
- Timeout each request.
- Follow redirects.
- Accept RSS 2.0 and Atom where practical.
- Store HTTP/network/parser errors in `LastError`.
- Never panic on malformed feeds.

### Parsing

Required RSS fields:

- `title`
- `link`
- `guid`
- `pubDate`
- `description`

Required Atom equivalents:

- `title`
- `link href`
- `id`
- `updated` or `published`
- `summary` or `content`

Optional tracker fields:

- enclosure URL;
- torrent link;
- magnet link;
- category;
- size;
- seeders/leechers if exposed through Torznab-like extensions.

### Identity and Dedupe

Feeds are deduped per feed.

Identity priority:

1. `Guid`, when present and stable.
2. Normalized `LinkHash`.
3. `TitleHash + PubDate`.
4. `TitleHash` only as a last resort.

Upsert behavior:

- Existing item found: update mutable metadata, do not reset `Processed`, `TorrentId`, or durable `SkippedReason` unless the link/guid changed.
- New item: insert with `Processed=false`, `SeenAt=now`.
- If a feed item has no usable link, insert it with a `SkippedReason` explaining why it cannot currently be downloaded.

### Retention

Default retention should keep RSS items indefinitely until a retention policy is implemented. The whole point of RSS ingestion is to retain items after they disappear from the upstream feed.

Future retention may safely remove:

- processed items older than a configured period;
- skipped items with durable reasons older than a configured period;
- unprocessed items only if the user opts into cleanup.

## Matching and Auto-Download Contract

RSS feed items are not downloads by themselves. They are release candidates.

For TV:

- Match parsed show name, season, and episode to wanted/missing episodes.
- Use title fallback parsing when parsed fields are missing.
- Prefer exact season/episode matches.
- Do not download specials or packs unless pack handling is explicitly implemented.

For movies:

- Match parsed movie title and year where available.
- Prefer exact year matches.
- Use metadata IDs if exposed by the feed.

For music and audiobooks:

- Treat as phase two unless a tracker feed format already provides reliable artist/album/book metadata.

Candidate ranking should use the same source-agnostic criteria as live source search:

- media type match;
- title/show similarity;
- season/episode/year match;
- quality preferences;
- seeders/availability where present;
- source priority;
- freeleech or tracker rules where present;
- whether the item is already processed.

## Download Contract

RSS item links may be:

- magnet links;
- direct `.torrent` links;
- authenticated tracker download links;
- details pages that require source-specific extraction.

The first implementation should support:

- magnet links directly through `AddTorrent`;
- direct `.torrent` links through the matching source authentication path if the feed is tied to a configured source;
- direct unauthenticated `.torrent` links as a fallback.

Unsupported link types must not be silently ignored. Store `SkippedReason` with enough detail to debug the feed item.

## GraphQL Contract

Standard generated GraphQL ORM CRUD exists for `RssFeed` and `RssFeedItem`. Custom operations should be added for RSS-specific workflows.

Required custom operations:

- `TestRssFeed(Input)`:
  - fetches a URL without saving;
  - parses sample items;
  - returns item count, sample items, and parser errors.
- `PollRssFeed(Id)`:
  - manually polls one saved feed;
  - returns poll summary and updated feed state.
- `PollRssFeeds(Input)`:
  - optionally polls all due feeds, all enabled feeds, or feeds for a library.
- `SearchRssFeedItems(Input)`:
  - searches the local RSS item cache directly for debugging/manual inspection.

Operation and field names should follow the active `graphql-orm` generated schema and frontend codegen output. Do not hand-maintain stale frontend GraphQL strings.

## Frontend Contract

RSS UI should live under source settings and library acquisition views.

Settings UI:

- list feeds;
- add/edit/delete feed;
- enable/disable feed;
- show last poll status;
- show consecutive failures;
- test URL;
- manual poll;
- view recent items.

Library/acquisition UI:

- show RSS candidates in source search results;
- show whether a candidate came from a live source search or RSS cache;
- allow manual download of a candidate;
- surface skipped reason when an RSS item cannot be used.

Frontend must consume generated GraphQL types after backend codegen is refreshed.

## Open Decisions

- Should RSS feeds be standalone `RssFeed` entities only, or should they also be a `Source` definition type?
- Should a feed be tied to a configured authenticated source for download, or should credentials live on the feed itself?
- What is the minimum allowed polling interval?
- Should processed RSS items be hidden from default search results?
- Should feed item parsing be library-type aware at poll time, search time, or both?
- How should multi-episode releases and season packs be handled?
- Should RSS feed items be considered for auto-download before or after live source search?
- Should feed retention be indefinite by default, or configurable from the start?

Recommended initial decisions:

- Keep RSS feeds as first-class `RssFeed` entities and expose them through an `RssFeedItemSource` adapter.
- Allow optional linking to a configured `Source` for authenticated downloads later; do not block initial unauthenticated/magnet support on that.
- Set minimum poll interval to 5 minutes.
- Keep all items indefinitely for the first implementation.
- Search live sources and RSS cache together, then rank candidates globally.

## Implementation Plan

### Phase 1: Contract and Fixtures

- Add this feature contract.
- Use the checked-in RSS fixture at `docs/sample-data/movie-sample.xml`.
- Add smaller focused RSS fixtures under `backend/tests/fixtures/rss/` when parser tests need narrow cases.
- Add at least one RSS 2.0 sample with torrent-like titles, guid, link, pubDate, and description if the existing sample does not cover a parser case.
- Add at least one malformed/partial feed fixture.
- When real user feeds are available, add sanitized snapshots as fixtures if licensing/privacy allows.

### Phase 2: Parser

- Implement RSS/Atom parser module.
- Normalize items into an internal parsed item struct.
- Parse release metadata using existing filename/title parsing where possible.
- Hash guid/link/title consistently.
- Add unit tests for:
  - RSS 2.0 parsing;
  - Atom parsing if supported in phase one;
  - missing optional fields;
  - malformed XML;
  - magnet links;
  - direct torrent links;
  - duplicate identity behavior.

### Phase 3: Feed Poller

- Implement `RssFeedService`.
- Register it in `ServicesManager`.
- Implement manual `PollRssFeed`.
- Implement scheduled due-feed polling.
- Use generated ORM operations for reads/writes.
- Add tests around:
  - due feed selection;
  - success status updates;
  - failure status updates;
  - duplicate upsert behavior;
  - preserving processed/skipped state on re-poll.

### Phase 4: RSS Source Adapter

- Implement `RssFeedItemSource`.
- Convert `RssFeedItem` rows to `SourceRelease`.
- Add it to source search orchestration without requiring network access during search.
- Add tests around:
  - TV query matching;
  - movie query matching;
  - library scoped feeds;
  - processed item filtering;
  - ranking metadata passthrough.

### Phase 5: Auto-Download Service

- Implement `AutoDownloadService`.
- Query wanted/missing entities through generated GraphQL ORM operations.
- Search live sources plus RSS cache.
- Rank candidates.
- Add torrents through the existing torrent service.
- Update related `RssFeedItem` processing state.
- Add tests around:
  - wanted movie search;
  - wanted episode search;
  - no candidate found;
  - duplicate torrent prevention;
  - RSS candidate selected;
  - live source candidate selected;
  - unsupported RSS link skipped with reason.

### Phase 6: Frontend and Codegen

- Regenerate frontend GraphQL types from the updated backend schema.
- Replace stale RSS GraphQL strings with generated documents/operations.
- Build RSS settings UI.
- Build recent feed items/debug view.
- Add frontend tests around:
  - feed form validation;
  - test feed modal result rendering;
  - manual poll action;
  - recent item table;
  - candidate source label in search results.

## Testing Strategy

### Static Contract Tests

- Keep the existing backend no-direct-SQL contract test.
- Add contract coverage ensuring RSS custom operations exist once implemented.
- Add contract coverage ensuring the RSS poller uses generated ORM operations, not raw SQL.

### Parser Unit Tests

- Use checked-in fixtures.
- Do not rely on live RSS URLs in unit tests.
- Cover valid, duplicate, partial, malformed, and tracker-specific feed shapes.

### Service Tests

- Use an in-memory or temporary SQLite database once the test harness supports schema setup through `graphql-orm`.
- Mock HTTP responses for feed polling.
- Assert database state through generated entity queries.

### Integration Tests

- Run `PollRssFeed` through GraphQL.
- Run `SearchRssFeedItems` through GraphQL.
- Run auto-download in a mocked torrent service context.
- Assert `RssFeedItem.Processed`, `TorrentId`, and `SkippedReason` transitions.

### Manual Test Checklist

- Add a feed.
- Test the URL.
- Poll manually.
- Verify items appear.
- Mark an episode/movie wanted.
- Run auto-download.
- Verify the candidate is found from RSS cache.
- Verify torrent is added.
- Verify feed item is marked processed.
- Disable feed and verify it is not polled.
- Break feed URL and verify `LastError` / `ConsecutiveFailures`.
