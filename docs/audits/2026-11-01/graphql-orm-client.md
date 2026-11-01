# Should Librarian adopt graphql-orm-client?

Date: 2026-11-01. Audited against graphql-orm 0.89.0 (`workspace-2026.11.01.1`) and
graphql-orm-client `main` at `4fca071`, two merges past `0.1.0-alpha.1`.

## Verdict

**Not yet.** The client's ideas fit Librarian, but today it would cost a rewrite of the data layer
for a gain limited to part of the site. It describes itself as an early alpha that has not run in
production, and several of its automatic behaviours do not reach Librarian without backend or
upstream work.

The upgrade itself already delivered the most valuable part of the contract: lost events now end a
stream with an error the frontend acts on. Stay on Apollo, take the Apollo-side wins below, and
revisit once the blockers are closed.

## What was tested

Every document in `web/src/graphql` was run through the client's real compiler, against the schema
exported from this upgrade. This proves compilation only. No screen was migrated, and the store and
live layer were not run against a Librarian server.

| Check | Result |
| --- | --- |
| 635 documents (578 operations, 57 fragments) | All compile; camelCase `id` fields are accepted |
| Live subscriptions through the `generate` CLI, all fragments `@live` | **0 of 54 types.** It looks for `LibraryChanged`; ours is `libraryChanged`. The `liveField` override is a function, so the JSON config cannot set it |
| Same, calling `compile()` with `liveField` and 115 `keys` entries | **52 of 54 types**, all in `keys` mode. `LegacyCastSession` and `LiveTorrent` have no `Changed` stream |
| Generated module size | 3.2 MB (224 KB gzipped) for all documents, against 1.7 MB (82 KB) for today's codegen output. Both include unused entity operations, so treat this as a rough comparison |
| Client runtime | 66 KB (16 KB gzipped) before minifying |

## Blockers

1. **Subscription names.** The CLI cannot be pointed at camelCase subscriptions. Small upstream fix.
2. **Record identity.** The client defaults to an `Id` field. Librarian uses `id`, so it needs either
   a `keys` entry per type or `node_id`, which the ORM refuses on `#[graphql(complex)]` entities.
   28 of our entity files use that form, including Movie, Library and Torrent.
3. **Record ordering.** `RecordVersion` needs `version = "managed"` and is not added to
   `#[graphql(complex)]` entities. No Librarian entity declares a managed version, so a slow query
   result could overwrite a newer event.
4. **Live payloads are scalars only.** A `Changed` payload rejects relation and computed fields, yet
   43 of the 90 hand-written queries traverse a relation. Those screens need a separate `@live`
   fragment for each record, plus a refetch for the related data.
5. **List membership still needs a refetch.** Events cannot say whether a row now belongs in a
   filtered or sorted list, so `LiveRegistry` deliberately re-reads snapshots. The client does the
   same for inserts and deletes. In-place updates are the only saving.
6. **Session handling.** Apollo's links retry reads and renew an expired session once before
   replaying. `createHttpNetwork` does neither, so this needs a custom network.
7. **Two stores during migration.** Migrating one route at a time means a mutation through Apollo
   does not update the client's store, and the reverse. Cross-route data such as the rail, library
   lists and content status would need both to agree.
8. **No router.** `@refetchable` needs `node(NodeId)`, which only `graphql-orm-router` serves.

## Site survey

Query and mutation counts are hand-written documents referenced from each area, so a shared
document counts once per area. Generated entity documents are not included, which is why Personal
lists shows one query while using 11 read hooks. "Fields" is the average scalar selections per query.

| Area | Queries | Mutations | Live types | Relations | Fields | Benefit from the client |
| --- | --- | --- | --- | --- | --- | --- |
| Libraries and browsers | 16 | 8 | 15 | 21 | 30 | Low: filtered lists re-read anyway |
| Media detail | 15 | 4 | 18 | 16 | 33 | Medium: most queries per page, but relation heavy |
| Route loaders | 8 | 0 | 10 | 19 | 39 | Medium: one root per route would simplify these |
| Home | 8 | 1 | 10 | 14 | 25 | Medium: 10 read hooks on one page |
| Search | 8 | 4 | 9 | 7 | 40 | Low: results are short lived |
| Downloads | 6 | 6 | 7 | 11 | 22 | Medium: in-place updates, though `TorrentProgress` already patches the cache |
| Settings | 5 | 13 | 6 | 6 | 13 | Low: forms, rarely live |
| Library add | 5 | 5 | 0 | 0 | 9 | None: provider results, not entities |
| Wanted | 5 | 1 | 8 | 3 | 24 | Low |
| Player | 4 | 7 | 7 | 4 | 39 | Low: progress writes, not lists |
| Guide | 3 | 0 | 4 | 1 | 13 | Low |
| Activity | 2 | 1 | 9 | 8 | 25 | Medium: rows update in place; best pilot |
| Personal lists | 1 | 0 | 7 | 7 | 41 | Low |
| Files, acquisition, shell, auth | 4 | 3 | 1 | 2 | 11 | None |

The migration surface is about 100 read hooks and 117 mutation hooks across the site, with 44 files
using live hooks. Only 8 queries select more than one root field, so waterfalls come from several
hooks on a page rather than from over-sized queries.

## What to do instead

1. **Use `keys` on `Changed` from Apollo.** `LiveRegistry` already knows the ids a detail page
   depends on, but it opens one stream per type and filters in the browser. Passing `keys` when every
   consumer of a topic has ids cuts server work on detail pages. This is new in the upgrade and
   needs no new client.
2. **Consider one root query per route** on Home and Media detail, where several hooks load one page.
   This is the client's roadmap phase 2, and it pays off on Apollo too.
3. **Remove unused documents.** 15 queries and 10 mutations in `web/src/graphql/documents` are not
   referenced outside tests.
4. **Ask upstream for** a `liveField` config option or automatic camelCase detection, `id` as a
   default key, `node_id` and `RecordVersion` on `#[graphql(complex)]` entities, and retry and session
   renewal hooks in `createHttpNetwork`.

## When to look again

Run a pilot on Activity when the client has a beta release and items 1 and 2 of the blockers are
closed. Its rows are flat, update in place and have only two queries, so it would show the real
saving at low risk.
