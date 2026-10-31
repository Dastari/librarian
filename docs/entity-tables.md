# Entity identity and data tables

How records are named, linked and listed across the web app. Every page that shows a
movie, show, torrent, person or notification draws it the same way and offers the same
actions, so a thing looks and behaves identically wherever it appears.

## Entity layer (`web/src/lib/entities/`)

- **Kinds** — `kinds.ts` declares every entity kind once with `defineEntityKinds`: glyph,
  singular and plural noun, and route segment. Kinds: `movie`, `show`, `season`, `episode`,
  `album`, `artist`, `track`, `audiobook`, `person`, `collection`, `library`, `torrent`,
  `mediaFile`, `notification`, `scanIssue`, `source`, `user`.
- **Identity builders** — one per kind (`torrentIdentity(row)`, `movieIdentity(movie)`, …)
  returning the props an `EntityIdentity` / `IdentityCell` needs: name, kind, glyph or
  artwork, facts (data only), href, and the kind's **state badges** (an `EntityRef` `state`
  or `status`). The same builder feeds a table's identity column, a detail header and an
  `EntityChip`.
- **Action sets** — one hook per kind (`useTorrentActions()`, …) returning an `ActionSet`
  from `defineActions`. Tables (row ⋯, right-click, bulk), detail headers (`ActionBar`), the
  command palette and chips all read that one set, so "Remove torrent" is the same action,
  with the same confirmation, everywhere.
- **Resolver** — `EntityResolverProvider` at the app root, built from the kinds, action sets
  and Apollo cache previews, so an `EntityChip` for any kind links, previews and acts.

Related records are linked with `EntityChip` (a torrent's movie, a notification's show, a
scan issue's file) rather than plain text.

## Tables

- **Bounded.** A table never scrolls off the screen. The page is a fixed-height workspace;
  the table fills the remaining space (`height="fill"`) and its rows scroll inside it. Every
  flex/grid ancestor between the table and the pane has `min-height: 0`.
- **No frame.** Main-page tables sit on the page, not inside a card or window frame.
- **Server state.** Sorting, filters and search are sent to the server (graphql-orm `where`
  / `orderBy` via `glass-ux/adapters/graphql-orm`), and rows load with `infinite` as the
  reader scrolls (`connectionPagination()` field policy, `nextOffsetPage`). Filtering one
  loaded batch in the browser is not filtering.
- **Declared filters.** Each filterable column declares its `filters.*` spec with the server
  field, option labels and units. Common views (Active, Seeding, Needs action) are presets of
  those filters, not separate code paths.
- **Rich cells.** Identity column: `IdentityCell` from the kind's builder. Numbers:
  `NumericCell`; times: `RelativeTime`; states: `StatusBadge` / entity state badges;
  progress and capacity: `MeterCell`; ordered ratings (ratio health, severity):
  `LevelGauge size="sm"`; live rates: `SparklineCell` (series sampled while the page is
  open).
- **Actions.** Rows take the kind's `ActionSet`; nothing is a dead row. `onRowOpen` opens the
  record (its page or detail pane).
- **Summary metrics without stat cards.** No strip of bordered status cards above a table.
  Live totals are compact `Metric`s (value plus sparkline) in the page's toolbar line, and
  counts belong on the filter presets they describe ("Seeding 24").

## Rollout

1. Kinds, resolver, torrent/movie/show/episode/notification/scan-issue builders and actions.
2. Downloads, Activity and library Scans rebuilt on the pattern.
3. Remaining tables (settings, browsers, Wanted, lists) migrated off the compatibility
   `DataTable` wrapper.
