GraphQL Type Consistency Refactor Plan
======================================

Purpose
-------
Single source of truth: define each table once in a Rust model with macros/traits that
generate SQLite schema/migrations, GraphQL types/queries/mutations/subscriptions, and
frontend types via introspection. Adding/removing a field should only require editing
the model and regenerating artifacts.

Goals
-----
- Single source of truth per table: one annotated Rust struct drives GraphQL types, table queries,
  filters/sorts/pagination, relations, and SQLite query generation.
- Macro/trait-generated boilerplate (queries/mutations/subscriptions) for every table.
- Consistent filtering, sorting, pagination, nested relations, and cursor-based infinite load.
- Minimal manual work: only write view models and utility operations (e.g. `ScanLibrary`).
- Generate frontend TypeScript types from GraphQL schema introspection to prevent drift.

Analysis of /home/toby/dev/gema-2026/crates/jim-service/src/graphql.rs (adapted for Librarian)
--------------------------------------------------------------------------------------------
- Entity structs derive `GraphQLFilters`, `Relations`, and `SimpleObject`. Per-field attributes declare:
  - `#[graphql(name = "...")]` for GraphQL naming.
  - `#[filterable(type = "string|number|date|boolean")]` to expose filter inputs.
  - `#[sortable]` for `orderBy` support.
  - `#[relation(...)]` for joins and nested selection loading.
  - `#[primary_key]`, `#[db_column]`, `#[date_field]`, `#[boolean_field]` for SQL mapping.
- `resolve_entities` is a shared resolver that:
  - Converts `WhereInput` to a filter struct via `convert_*_where_to_filter`.
  - Applies `limit` and `orderBy` to the filter via `ConfigurableFilter`.
  - Executes SQL via `graphql_orm::fetch_with_filter_and_client`.
  - Loads relations only when requested using `look_ahead` + relation metadata.
- Query root exposes table collections and a single entity lookup with relation loading.
- For Librarian, this pattern must be adapted to SQLite (sqlx + parameter binding).
- This is the starting point we should expand: the same macro-driven, single-source-of-truth pattern,
  extended for SQLite and more filter types.

Dependencies and Macro/Traits Summary (from graphql-orm + graphql-orm-macros)
----------------------------------------------------------------------------
- Traits and helpers (graphql-orm):
  - `DatabaseEntity`: table name, columns list, default sort, primary keys, row conversion.
  - `DatabaseFilter`: `apply_to_where_builder`, sort, limit, column metadata.
  - `FromSqlRow`: converts a DB row to the entity struct.
  - `RelationshipLoader`: loads relations (single and bulk, plus "selective" versions).
  - `RelationMetadataProvider` + registry: enumerates relation names for lookahead traversal.
  - `WhereBuilder`: string-based SQL builder for eq/ne/in/contains/date/bool filters.
  - `OrderByInput` + `ConfigurableFilter`: tie GraphQL inputs to SQL ordering/limits.
- Procedural macros (graphql-orm-macros):
  - `GraphQLFilters` generates:
    - `StructFilter`, `StructWhereInput`, `StructOrderByInput`.
    - `convert_struct_where_to_filter` function.
    - `DatabaseEntity`, `DatabaseFilter`, `OrderByInput`, `ConfigurableFilter` impls.
    - `FromSqlRow` for row decoding, including date/bool helpers.
  - `Relations` generates:
    - `RelationshipLoader` with selective loading based on requested fields.
    - `RelationMetadataProvider` and a registration hook for nested lookahead.
- Current limitations relative to desired behavior:
  - Number filters are limited to eq/ne/in/not_in (no gt/gte/lt/lte).
  - No decimal filter type (only i32 for number filters).
  - SQL generation is string-concatenated (risk of injection if extended).
  - No cursor-based pagination in the ORM layer (only limit + order).
  - Columns are selected wholesale; no field-level SQL projection.

Target Architecture for Librarian
---------------------------------
- One annotated Rust struct per table (macro/trait driven):
  - `#[derive(GraphQLFilters, Relations, SimpleObject, Clone)]`
  - `#[table_name = "...", plural = "..."]`
  - `#[default_sort = "..."]`
  - Field-level attributes define filterable/sortable/relations.
- Macros generate:
  - GraphQL type + input types (where/order).
  - Queries: `<table>` (by id) and `<tables>` (list) with F/S/P/C.
  - Mutations: insert/update/delete (or table-specific CRUD).
  - Subscriptions: table change events.
  - SQL/SQLite query plumbing with parameter binding.
- Query capabilities:
  - Full field selection, nested joins, filtering, sorting.
  - Cursor-based pagination (Relay-style connections) for infinite load.
  - Filters with eq/neq/gt/gte/lt/lte/contains/in/not_in for string, int, decimal, date, boolean.
- Utility and view-model operations are the only manual surface:
  - File operations like `ScanLibrary`, `DeleteFile`, etc.
  - Aggregates/views built on top of table modules and relations.
- GraphQL naming must follow `.cursor/rules/graphql-naming-convention.mdc`
  (PascalCase for all GraphQL-exposed names).
- Single source of truth lifecycle:
  - Add/remove fields in Rust model definition.
  - Macros update SQLite schema/migrations and GraphQL schema.
  - Frontend types update via introspection/codegen.

SQLite-specific macro requirements
----------------------------------
- Generate SQL with parameter binding via sqlx (no string concatenation).
- Emit SQLite-safe column/identifier handling (quoted identifiers where needed).
- Centralize row decoding helpers for dates, booleans, decimals, JSON.
- Keep filter rendering SQLite-compatible (e.g., `LIKE` for contains, `IS NULL` checks).
- Cursor pagination should use stable sort columns and deterministic tie-breakers.
- Cursor pagination format follows Relay Connection (Edges/Node).

Real Schema Example - Libraries
-------------------------------
Based on `backend/migrations_sqlite/001_initial_schema.sql`, the `libraries` table and
its real `library_id` relationships (e.g. `movies`, `shows`, `artists`, `albums`,
`tracks`, `audiobooks`, `media_files`, `torrents`, `rss_feeds`, `user_library_access`).

Example model (macro-first, single source of truth):

```rust
use crate::graphql::prelude::*;
use chrono::NaiveDateTime;

#[derive(GraphQLFilters, Relations, SimpleObject, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[graphql(name = "Library")]
#[serde(rename_all = "PascalCase")]
#[table_name = "libraries", plural = "Libraries", default_sort = "Name"]
pub struct Library {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    #[sortable]
    pub user_id: String,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "Path")]
    #[filterable(type = "string")]
    pub path: String,

    #[graphql(name = "LibraryType")]
    #[filterable(type = "enum")]
    #[sortable]
    pub library_type: LibraryType,

    #[graphql(name = "Icon")]
    #[filterable(type = "string")]
    pub icon: Option<String>,

    #[graphql(name = "Color")]
    #[filterable(type = "string")]
    pub color: Option<String>,

    #[graphql(name = "AutoScan")]
    #[filterable(type = "bool")]
    pub auto_scan: bool,

    #[graphql(name = "ScanIntervalMinutes")]
    #[filterable(type = "number")]
    pub scan_interval_minutes: i64,

    #[graphql(name = "WatchForChanges")]
    #[filterable(type = "bool")]
    pub watch_for_changes: bool,

    #[graphql(name = "LastScannedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_scanned_at: Option<NaiveDateTime>,

    #[graphql(name = "Scanning")]
    #[filterable(type = "bool")]
    #[sortable]
    pub scanning: bool,

    #[graphql(name = "PostDownloadAction")]
    #[filterable(type = "enum")]
    pub post_download_action: PostDownloadAction,

    #[graphql(name = "OrganizeFiles")]
    #[filterable(type = "bool")]
    pub organize_files: bool,

    #[graphql(name = "RenameStyle")]
    #[filterable(type = "enum")]
    pub rename_style: RenameStyle,

    #[graphql(name = "NamingPattern")]
    #[filterable(type = "string")]
    pub naming_pattern: Option<String>,

    #[graphql(name = "AutoDownload")]
    #[filterable(type = "bool")]
    pub auto_download: bool,

    #[graphql(name = "AutoHunt")]
    #[filterable(type = "bool")]
    pub auto_hunt: bool,

    #[graphql(name = "AutoAddDiscovered")]
    #[filterable(type = "bool")]
    pub auto_add_discovered: bool,

    #[graphql(name = "AllowedResolutions")]
    pub allowed_resolutions: Vec<String>,

    #[graphql(name = "AllowedVideoCodecs")]
    pub allowed_video_codecs: Vec<String>,

    #[graphql(name = "AllowedAudioFormats")]
    pub allowed_audio_formats: Vec<String>,

    #[graphql(name = "RequireHdr")]
    #[filterable(type = "bool")]
    pub require_hdr: bool,

    #[graphql(name = "AllowedHdrTypes")]
    pub allowed_hdr_types: Vec<String>,

    #[graphql(name = "AllowedSources")]
    pub allowed_sources: Vec<String>,

    #[graphql(name = "ReleaseGroupBlacklist")]
    pub release_group_blacklist: Vec<String>,

    #[graphql(name = "ReleaseGroupWhitelist")]
    pub release_group_whitelist: Vec<String>,

    #[graphql(name = "AutoDownloadSubtitles")]
    #[filterable(type = "bool")]
    pub auto_download_subtitles: bool,

    #[graphql(name = "PreferredSubtitleLanguages")]
    pub preferred_subtitle_languages: Vec<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: NaiveDateTime,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: NaiveDateTime,

    #[relation(name = "Movies", from = "Id", to = "LibraryId", multiple)]
    pub movies: Vec<Movie>,

    #[relation(name = "TvShows", from = "Id", to = "LibraryId", multiple)]
    pub tv_shows: Vec<TvShow>,

    #[relation(name = "Artists", from = "Id", to = "LibraryId", multiple)]
    pub artists: Vec<Artist>,

    #[relation(name = "Albums", from = "Id", to = "LibraryId", multiple)]
    pub albums: Vec<Album>,

    #[relation(name = "Tracks", from = "Id", to = "LibraryId", multiple)]
    pub tracks: Vec<Track>,

    #[relation(name = "Audiobooks", from = "Id", to = "LibraryId", multiple)]
    pub audiobooks: Vec<Audiobook>,

    #[relation(name = "MediaFiles", from = "Id", to = "LibraryId", multiple)]
    pub media_files: Vec<MediaFile>,

    #[relation(name = "Torrents", from = "Id", to = "LibraryId", multiple)]
    pub torrents: Vec<Torrent>,

    #[relation(name = "RssFeeds", from = "Id", to = "LibraryId", multiple)]
    pub rss_feeds: Vec<RssFeed>,

    #[relation(name = "UserAccess", from = "Id", to = "LibraryId", multiple)]
    pub user_access: Vec<UserLibraryAccess>,
}
```

Example GraphQL operations (Relay cursor connections):

Simple query:

```graphql
query GetLibrary {
  Library(Id: "lib_123") {
    Id
    Name
    Path
    LibraryType
  }
}
```

Filtered list with sort + pagination:

```graphql
query ListLibraries {
  Libraries(
    Where: {
      LibraryType: { Eq: Movies }
      Name: { Contains: "4K" }
      AutoScan: { Eq: true }
    }
    Sort: [{ Name: Asc }, { UpdatedAt: Desc }]
    Page: { Limit: 25, Offset: 0 }
  ) {
    Nodes {
      Id
      Name
      LibraryType
      UpdatedAt
    }
    PageInfo {
      TotalCount
      HasNextPage
    }
  }
}
```

Cursor-based infinite load with nested joins:

```graphql
query LibrariesWithMovies {
  Libraries(
    Where: { LibraryType: { Eq: Movies } }
    Sort: [{ Name: Asc }]
    Cursor: { First: 50, After: "cursor:abc123" }
  ) {
    Edges {
      Cursor
      Node {
        Id
        Name
        Movies(
          Where: { Year: { Gte: 2020 } }
          Sort: [{ Year: Desc }, { Title: Asc }]
          Page: { Limit: 10, Offset: 0 }
        ) {
          Nodes {
            Id
            Title
            Year
            MediaFile {
              Id
              Path
              SizeBytes
            }
          }
        }
      }
    }
    PageInfo {
      EndCursor
      HasNextPage
    }
  }
}
```

Mutation (simple create):

```graphql
mutation CreateLibrary {
  CreateLibrary(
    Input: {
      Name: "Movies"
      Path: "/media/movies"
      LibraryType: Movies
      AutoScan: true
      ScanIntervalMinutes: 60
      WatchForChanges: true
    }
  ) {
    Id
    Name
    Path
    LibraryType
  }
}
```

Mutation (complex update):

```graphql
mutation UpdateLibrarySettings {
  UpdateLibrary(
    Id: "lib_123"
    Patch: {
      AutoDownload: true
      AutoHunt: true
      AllowedResolutions: ["1080p", "2160p"]
      AllowedVideoCodecs: ["H264", "HEVC"]
      RenameStyle: Clean
      NamingPattern: "{title} ({year})/{title} ({year}).{ext}"
    }
  ) {
    Id
    Name
    AutoDownload
    AutoHunt
    RenameStyle
    NamingPattern
  }
}
```

Subscription (table change):

```graphql
subscription LibraryChanges {
  LibraryChanged(Where: { LibraryType: { Eq: Movies } }) {
    Action
    Library {
      Id
      Name
      LibraryType
      UpdatedAt
    }
  }
}
```

Subscription (scan progress for a single library):

```graphql
subscription LibraryScanProgress {
  LibraryScanProgress(LibraryId: "lib_123") {
    LibraryId
    Phase
    Current
    Total
    Message
  }
}
```

Frontend Migration Hints (Macro-Generated Table APIs)
-----------------------------------------------------
- Prefer table queries for any table-backed data; they will consistently support filtering,
  sorting, and cursor pagination via Relay Connections.
- Use view-model/workflow queries only when the data is derived, aggregated, or service-backed
  (non-table).
- For infinite load, use cursor-based connections (`First` + `After`) and `PageInfo.EndCursor`.

Utility (Non-Table) GraphQL Operations
--------------------------------------
- Keep non-table operations manual and grouped by domain (query/mutation/subscription together).
- Non-table list queries should mirror the same GraphQL operation naming and input shapes
  (filters, sorts, pagination, cursor connections) as table-backed queries.
- Prefer server-side filtering/sorting/pagination when the upstream source supports it
  (e.g., third-party APIs). When it does not, fetch once and apply the same behavior
  in memory so clients can use identical inputs across table and non-table queries.
- Example utility domains (current/expected):
  - Filesystem: browse/validate paths, file copy/move/rename, directory creation.
  - Scanner: `ScanLibrary` and scan progress updates.
  - Other service-backed operations that do not map to a single table.

Session Notes for Future Agents
-------------------------------
- The branch `refactor/graphql-consistency` contains a large set of manual/domain-oriented
  GraphQL refactors that do not align with the macro-first, single-source-of-truth approach.
  Treat it as a scratch/reference branch and prefer restarting from `main/master`.
- A previous ops map (`docs/graphql-operations-map.md`) was removed because it documented
  legacy manual domain queries/mutations and conflicted with the macro-first direction.

Implementation Plan (Phased)
----------------------------
Phase 1 - Inventory and Model Mapping
- Inventory tables and columns from migrations:
  - `/home/toby/dev/librarian/backend/migrations_sqlite/*.sql`
  - `/home/toby/dev/librarian/backend/migrations/*.sql` (if present in future)
- Map each table to an existing DB module in `/home/toby/dev/librarian/backend/src/db/*.rs`
  or create a new module when missing.
- Identify where the existing GraphQL schema already covers table data in:
  - `/home/toby/dev/librarian/backend/src/graphql/schema.rs`
  - `/home/toby/dev/librarian/backend/src/graphql/tables/*.rs`
  - `/home/toby/dev/librarian/backend/src/graphql/utility/*.rs`
  - `/home/toby/dev/librarian/backend/src/graphql/subscriptions.rs`
  - `/home/toby/dev/librarian/backend/src/graphql/subscriptions.rs`

Phase 2 - ORM and Macro Foundation
- Add a lightweight internal ORM layer modeled on `graphql-orm`:
  - New module location suggestion: `/home/toby/dev/librarian/backend/src/graphql/orm/`
  - Traits: `DatabaseEntity`, `DatabaseFilter`, `FromSqlRow`, `RelationshipLoader`,
    `RelationMetadataProvider`, `OrderByInput`, `ConfigurableFilter`.
  - Query builder: SQLx `QueryBuilder` for SQLite with parameter binding.
  - Extend filter types to include decimal (e.g. `f64` or `rust_decimal::Decimal`).
- Extend `librarian-macros` to include derives similar to `GraphQLFilters` + `Relations`:
  - New macros in `/home/toby/dev/librarian/librarian-macros/src/lib.rs`.
  - Ensure generated code is SQLite-safe and uses parameter binding.
  - The macro should also generate GraphQL root fields (`episode`, `episodes`, etc)
    and the default CRUD/subscribe surface to avoid manual boilerplate.

Phase 3 - Table Modules and GraphQL Surface
- Table modules should be macro-driven and minimal:
  - Provide the annotated struct and let macros generate everything else.
  - Keep manual code limited to view models and utilities.
- Update GraphQL root wiring to expose the macro-generated table operations.
- Add a utility module for non-table file/system actions:
  - Example location: `/home/toby/dev/librarian/backend/src/graphql/utility/`
  - Group operations by domain (filesystem/media/scanning) and keep the
    query/mutation/subscription trio together when they operate on the same
    domain objects.

Phase 4 - Filtering, Sorting, Pagination, and Joins
- Implement standardized filter handling for string/int/date/decimal/boolean:
  - Extend filter inputs in `/home/toby/dev/librarian/backend/src/graphql/filters.rs`
    or move into the new ORM module and re-export.
- Add cursor-based pagination per table:
  - Use `/home/toby/dev/librarian/backend/src/graphql/pagination.rs` for connections.
  - Integrate cursor handling into the query builder and GraphQL resolvers.
- Implement selective relation loading:
  - Use lookahead to load only requested relations.
  - Support nested joins by reusing `RelationshipLoader` metadata.

Phase 5 - Frontend Type Generation
- Introduce GraphQL schema introspection + codegen:
  - Add a `pnpm` script in `/home/toby/dev/librarian/frontend/package.json`.
  - Generate types into `/home/toby/dev/librarian/frontend/src/lib/graphql/generated/`.
  - Replace manual types in `/home/toby/dev/librarian/frontend/src/lib/graphql/types.ts`
    with generated types where feasible.
- Update GraphQL client wrappers:
  - `/home/toby/dev/librarian/frontend/src/lib/graphql/client.ts`
  - `/home/toby/dev/librarian/frontend/src/lib/graphql/queries.ts`
  - `/home/toby/dev/librarian/frontend/src/lib/graphql/mutations.ts`
  - `/home/toby/dev/librarian/frontend/src/lib/graphql/subscriptions.ts`

Phase 6 - Migration and Safety
- Incrementally migrate tables to the new table-centric GraphQL modules.
- Keep existing APIs stable while introducing new endpoints to avoid breaking clients.
- Add tests for:
  - Filter semantics and SQL generation.
  - Pagination edge cases and cursor integrity.
  - Relation loading correctness and N+1 prevention.

Risk Notes
----------
- The `graphql-orm` approach uses raw SQL strings; for Librarian we should use
  parameterized SQLx queries to avoid injection risks when adding gt/lt/contains.
- SQLite column types are flexible; ensure strict Rust typing with conversion
  helpers in `/home/toby/dev/librarian/backend/src/db/sqlite_helpers.rs`.
- Ensure GraphQL schema changes are reflected in frontend codegen to avoid stale
  manual types.

Open Questions
--------------
- Do we want one-to-one GraphQL types per DB table, or table types plus
  "view models" for existing domain aggregates?
- Should subscriptions be table-level (row changes) or higher-level domain events?
- Preferred decimal type (`f64` vs `rust_decimal::Decimal`) for money fields?

Resolved Direction (from discussion)
------------------------------------
- Keep both table-backed GraphQL types and view models.
- View models should be built on top of the table modules (filters, joins,
  lookahead, pagination) so that relational data can be reshaped without
  duplicating SQL or business logic.

Implementation Status (current branch)
--------------------------------------
**Completed phases:**

Phase 1: Macro Foundation (DONE)
- Created `librarian-macros` crate with three derive macros:
  - `#[derive(GraphQLEntity)]` - Generates WhereInput, OrderByInput, DatabaseEntity, DatabaseFilter, FromSqlRow impls
  - `#[derive(GraphQLRelations)]` - Generates RelationLoader impl and ComplexObject resolvers for relations
  - `#[derive(GraphQLOperations)]` - Generates Query/Mutation/Subscription structs, Connection/Edge types, Input types

Phase 2: Query Builder (DONE)
- Created `backend/src/graphql/orm/` module with:
  - `EntityQuery<E>` builder for parameterized SQL queries
  - Traits: `DatabaseEntity`, `DatabaseFilter`, `DatabaseOrderBy`, `FromSqlRow`, `RelationLoader`
  - Pagination types: `PageInput`, `CursorInput`, `PageInfo`
  - Filter types with full PascalCase naming

Phase 3: Entity Definitions (DONE)
- Created macro-driven entities in `backend/src/graphql/entities/`:
  - `LibraryEntity` with relations to Movies, TvShows, MediaFiles
  - `MovieEntity` with relation to MediaFile
  - `TvShowEntity` with relation to Episodes
  - `EpisodeEntity` with relation to MediaFile
  - `MediaFileEntity`
- Each entity generates: WhereInput, OrderByInput, Connection, Edge, Queries, Mutations, Subscriptions

Phase 4: Nested Relations with Filtering/Sorting/Pagination (DONE)
- Relations are exposed as ComplexObject resolver methods
- Each relation accepts: Where (filter), OrderBy (sort), Page (pagination)
- Example query now supported:
  ```graphql
  query LibrariesWithMovies {
    LibraryEntities(Where: { LibraryType: { Eq: "movies" } }) {
      Edges {
        Cursor
        Node {
          Id
          Name
          Movies(Where: { Year: { Gte: 2020 } }, OrderBy: [{ Year: Desc }], Page: { Limit: 10 }) {
            Edges {
              Node { Id, Title, Year, MediaFile { Id, Path, Size } }
            }
          }
        }
      }
      PageInfo { EndCursor, HasNextPage }
    }
  }
  ```

Phase 5: Frontend Type Generation (DONE)
- Added `@graphql-codegen/cli` and related packages to frontend
- Created `codegen.ts` configuration file
- Added `pnpm codegen` script to generate types from schema
- Types are generated to `frontend/src/lib/graphql/generated/types.ts`
- Introspection schema saved to `frontend/src/lib/graphql/generated/schema.json`

Phase 6: Schema Integration (DONE)
- Wired generated Query/Mutation/Subscription structs into schema MergedObject
- Added all 15 entity modules to QueryRoot and MutationRoot
- Increased recursion limit to 512 to handle large MergedObject types

Phase 7: Full Entity Coverage (DONE)
- Created all remaining entity definitions:
  - `ArtistEntity` with relation to Albums
  - `AlbumEntity` with relation to Tracks
  - `TrackEntity` with relation to MediaFile
  - `AudiobookEntity` with relation to Chapters
  - `ChapterEntity` with relation to MediaFile
  - `TorrentEntity` with relation to TorrentFiles
  - `TorrentFileEntity`
  - `UserEntity`
  - `RssFeedEntity`
  - `IndexerConfigEntity`
- Updated `LibraryEntity` with relations to Artists, Albums, Audiobooks, Torrents, RssFeeds

Phase 8: CRUD Mutations (DONE)
- Implemented CREATE mutation with SQL INSERT generation
- Implemented UPDATE mutation with dynamic SQL UPDATE generation
- Uses `execute_with_binds` helper to handle sqlx query lifetimes
- All fields properly converted to SqlValue for binding

Phase 9: Subscriptions (DONE)
- Generated subscription resolvers for all entities
- Subscriptions gracefully return empty streams when broadcast channels not configured
- Filter support for action types (Created, Updated, Deleted)

Phase 10: ORM-like Auto-Migration (DONE)
- Added `DatabaseSchema` trait to entities with column definitions
- Entities define their columns with: name, SQL type, nullability, primary key, defaults
- Created `schema_sync` module that:
  - Queries SQLite `sqlite_master` and `PRAGMA table_info` for current schema
  - Compares to entity definitions
  - Creates missing tables automatically
  - Adds missing columns automatically (using ALTER TABLE ADD COLUMN)
- `db.sync_entity_schemas()` called at startup after manual migrations
- Entity changes now auto-sync to database without manual .sql files

**Note on coexistence:**
- Manual migration files (`migrations_sqlite/*.sql`) still needed for:
  - Non-entity tables (app_settings, torznab_categories, etc.)
  - Seed data inserts
  - Complex constraints and triggers
- Domain queries (`LibraryQueries`, etc.) coexist with entity queries:
  - Domain queries: business logic, computed stats, view models
  - Entity queries: raw CRUD with filtering/sorting/pagination

Phase 11: Repository Pattern for Internal Use (DONE)
- Added internal query API to entity structs for service-layer use:
  - `Entity::query(&pool)` - Returns `FindQuery` builder with filter/order/pagination
  - `Entity::get(&pool, id)` - Find single entity by ID
  - `Entity::count_query(&pool)` - Count entities with optional filter
  - `Entity::search_similar(&pool, field, query, threshold, filter, limit)` - Fuzzy text search
- Added helper constructors to filter types:
  - `StringFilter::eq()`, `ne()`, `contains()`, `is_null()`, `similar()`
  - `IntFilter::eq()`, `gte()`, `lte()`, `is_null()`
  - `BoolFilter::is_true()`, `is_false()`, `is_null()`
  - `DateFilter::recent_days()`, `within_days()`, `in_past()`, `in_future()`
- Enables internal services to use the same generated queries as GraphQL

Phase 12: Extended Filter Operators (DONE)
- Added IsNull/IsNotNull to all filter types (StringFilter, IntFilter, BoolFilter, DateFilter)
- Added fuzzy/similar text matching:
  - `SimilarFilter` with value and threshold
  - Uses strsim crate for Jaro-Winkler + Levenshtein scoring
  - `FuzzyMatcher` class with normalization (handles media naming patterns)
  - Normalization handles: dots, underscores, articles (the/a/an), brackets
- Added date arithmetic operators:
  - `InPast`, `InFuture`, `IsToday` - relative to today
  - `RecentDays(n)` - within last N days
  - `WithinDays(n)` - within next N days
  - `GteRelative`, `LteRelative` - using RelativeDate input

Phase 13: db/ Folder Migration (IN PROGRESS - ~10% complete)
- Goal: Remove duplicate record types and manual SQL from db/ folder
- Pattern: Replace manual SQL with generated entity queries via `Entity::query()`

### Files to KEEP (essential infrastructure):
| File | Reason |
|------|--------|
| `mod.rs` | Database connection, pool, migration runner (simplified) |
| `schema_sync.rs` | Auto-migration logic for entity tables |
| `sqlite_helpers.rs` | UUID/datetime/JSON conversion helpers |

### Files to REMOVE (fully replaced by entities):
| File | Replacement |
|------|-------------|
| `libraries.rs` | `LibraryEntity::query()`, `::get()`, mutations |
| `movies.rs` | `MovieEntity::query()`, `::get()`, mutations |
| `tv_shows.rs` | `TvShowEntity::query()`, `::get()`, mutations |
| `episodes.rs` | `EpisodeEntity::query()`, `::get()`, mutations |
| `albums.rs` | `AlbumEntity::query()`, `ArtistEntity::query()` |
| `tracks.rs` | `TrackEntity::query()`, `::get()`, mutations |
| `audiobooks.rs` | `AudiobookEntity::query()`, `ChapterEntity::query()` |
| `torrents.rs` | `TorrentEntity::query()`, `::get()`, mutations |
| `torrent_files.rs` | `TorrentFileEntity::query()` |
| `media_files.rs` | `MediaFileEntity::query()`, `::get()` |
| `pending_file_matches.rs` | `PendingFileMatchEntity::query()` |
| `rss_feeds.rs` | `RssFeedEntity::query()`, `RssFeedItemEntity::query()` |
| `logs.rs` | `AppLogEntity::query()` (with date filters) |
| `users.rs` | `UserEntity::query()`, related entities |
| `watch_progress.rs` | `WatchProgressEntity::query()` |
| `playback.rs` | `PlaybackSessionEntity::query()` |
| `cast.rs` | Cast entity queries |
| `schedule.rs` | Schedule entity queries |
| `notifications.rs` | `NotificationEntity::query()` |
| `usenet_servers.rs` | `UsenetServerEntity::query()` |
| `usenet_downloads.rs` | `UsenetDownloadEntity::query()` |
| `indexers.rs` | Indexer entity queries |
| `naming_patterns.rs` | `NamingPatternEntity::query()` |
| `priority_rules.rs` | `SourcePriorityRuleEntity::query()` |
| `subtitles.rs` | Subtitle/Stream entity queries |
| `artwork.rs` | `ArtworkCacheEntity::query()` |

### Operations requiring custom code (keep as utilities):
Some operations need raw SQL because they involve:
- **Aggregates**: COUNT with GROUP BY, SUM, AVG
- **Complex JOINs**: Until JOIN filters are implemented
- **Batch operations**: Multi-row INSERT with transactions
- **Cleanup**: DELETE with date arithmetic, VACUUM
- **Statistics**: Library stats, log counts by level

These should be moved to a new `db/operations.rs` module or kept as helper functions within the entity's impl block.

### Migration pattern for services:
```rust
// BEFORE (using db/ repository)
let movies = db.movies().list_by_library(library_id).await?;

// AFTER (using entity query)
let movies = MovieEntity::query(db.pool())
    .filter(MovieEntityWhereInput {
        library_id: Some(StringFilter::eq(library_id.to_string())),
        ..Default::default()
    })
    .fetch_all()
    .await?;
```

### Simplified mod.rs after migration:
```rust
pub mod schema_sync;
pub mod sqlite_helpers;
pub mod operations; // Custom aggregate/batch operations

pub struct Database {
    pool: DbPool,
}

impl Database {
    pub async fn connect(url: &str) -> Result<Self> { ... }
    pub fn pool(&self) -> &DbPool { &self.pool }
    pub async fn sync_entity_schemas(&self) -> ... { ... }
    pub async fn migrate(&self) -> Result<()> { ... }
}
```

**Pending:**
- Complete db/ folder migration to entity queries
- Add JOIN/nested filter support for cross-entity queries (see Future Improvements)
- Configure broadcast channels for entities that need real-time updates
- Migrate frontend to use generated types instead of manual types
- Review and normalize list sorting/filtering/pagination across workflow queries

Current Migration Status (Phase 13)
-----------------------------------

## Key Insight: Most Manual Queries Are Redundant

The generated entity queries (`MovieEntities`, `LibraryEntities`, etc.) already provide:
- Filtering via `Where` input
- Sorting via `OrderBy` input  
- Pagination via `Page` input
- Get by ID via singular query (`MovieEntity(Id: "...")`)

**Therefore, most manual queries in `graphql/queries/*.rs` are REDUNDANT and should be REMOVED.**

### What to KEEP vs REMOVE

**KEEP (not redundant):**
- External API calls (e.g., `search_movies` calls TMDB API)
- Complex aggregations that require custom SQL (use `db/operations.rs`)
- Workflow operations (e.g., `ScanLibrary`, `ProcessTorrent`)

**REMOVE (redundant with entity queries):**
- `movies(library_id)` → use `MovieEntities(Where: {LibraryId: {Eq: "..."}})`
- `movies_connection(...)` → use `MovieEntities(Where: ..., Page: ...)`
- `movie(id)` → use `MovieEntity(Id: "...")`
- `all_movies` → use `MovieEntities(Where: {UserId: {Eq: "..."}})`
- Similar patterns for libraries, tv_shows, episodes, etc.

**COMPUTED FIELDS: Add to Entity via ComplexObject**

Some manual queries add computed fields (e.g., `download_progress`). These should be added directly to the entity:

```rust
// In movie.rs entity - add ComplexObject resolver
#[async_graphql::ComplexObject]
impl MovieEntity {
    #[graphql(name = "DownloadProgress")]
    async fn download_progress(&self, ctx: &Context<'_>) -> Option<f64> {
        if self.media_file_id.is_some() { return None; }
        let db = ctx.data_unchecked::<Database>();
        if let Ok(id) = Uuid::parse_str(&self.id) {
            db.torrent_files().get_download_progress_for_movie(id).await.ok().flatten()
        } else { None }
    }
}
```

## Architecture: Consolidated Entity Files (Standard Practice)

Each entity file (`graphql/entities/*.rs`) is the **single source of truth** containing everything related to that entity. This is the standard pattern all entities MUST follow.

### Three-Part Entity File Structure

Every entity file has exactly three parts:

```
┌─────────────────────────────────────────────────────────────────┐
│ PART 1: Entity Struct                                           │
│ - #[derive(GraphQLEntity, GraphQLOperations, SimpleObject)]     │
│ - Generates: WhereInput, OrderByInput, Connection, CRUD         │
├─────────────────────────────────────────────────────────────────┤
│ PART 2: ComplexObject impl                                      │
│ - Computed fields (ItemCount, DownloadProgress)                 │
│ - Relations with Where/OrderBy/Page args                        │
├─────────────────────────────────────────────────────────────────┤
│ PART 3: CustomOperations struct                                 │
│ - External API calls (Search*, AddFromProvider)                 │
│ - Service triggers (ScanLibrary, ProcessTorrent)                │
│ - Custom result types defined here                              │
└─────────────────────────────────────────────────────────────────┘
```

### Complete Template

```rust
//! {Entity} Entity
//!
//! This module contains:
//! - {Entity}Entity with computed fields and relations
//! - {Entity}CustomOperations for non-CRUD operations
//!
//! CRUD operations are auto-generated by GraphQLOperations macro.

use std::sync::Arc;

use async_graphql::{Context, Object, Result, SimpleObject};
use librarian_macros::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::Database;
use crate::graphql::auth::AuthExt;
use crate::graphql::orm::{EntityQuery, SqlValue};

// ============================================================================
// PART 1: Entity Definition
// ============================================================================

/// {Entity} Entity
///
/// Note: We don't use GraphQLRelations here because we need custom ComplexObject.
#[derive(GraphQLEntity, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "{Entity}Entity", complex)]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "{table}", plural = "{Entity}Entities", default_sort = "{field}")]
pub struct {Entity}Entity {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,

    // ... fields with #[filterable], #[sortable] ...

    // Relations (skipped from DB, resolved via ComplexObject)
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    pub related_items: Vec<RelatedEntity>,
}

// ============================================================================
// PART 2: ComplexObject Resolvers
// ============================================================================

#[async_graphql::ComplexObject]
impl {Entity}Entity {
    /// Computed field
    #[graphql(name = "ItemCount")]
    async fn item_count(&self, ctx: &async_graphql::Context<'_>) -> i64 {
        let db = ctx.data_unchecked::<Database>();
        // ... compute value using db/operations.rs helpers
        0
    }

    /// Relation with filtering/sorting/pagination
    #[graphql(name = "RelatedItems")]
    async fn related_items_resolver(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(name = "Where")] where_input: Option<RelatedEntityWhereInput>,
        #[graphql(name = "OrderBy")] order_by: Option<Vec<RelatedEntityOrderByInput>>,
        #[graphql(name = "Page")] page: Option<crate::graphql::orm::PageInput>,
    ) -> async_graphql::Result<RelatedEntityConnection> {
        let db = ctx.data_unchecked::<Database>();
        let mut query = EntityQuery::<RelatedEntity>::new()
            .where_clause("{entity}_id = ?", SqlValue::String(self.id.clone()));

        if let Some(ref f) = where_input { query = query.filter(f); }
        if let Some(ref o) = order_by { for ord in o { query = query.order_by(ord); } }
        if query.order_clauses.is_empty() { query = query.default_order(); }
        if let Some(ref p) = page { query = query.paginate(p); }

        let conn = query.fetch_connection(db.pool()).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(RelatedEntityConnection::from_generic(conn))
    }
}

// ============================================================================
// PART 3: Custom Operations
// ============================================================================

/// Result type for operations
#[derive(Debug, SimpleObject)]
pub struct {Entity}OperationResult {
    pub success: bool,
    pub {entity}: Option<ViewModelType>,
    pub error: Option<String>,
}

/// Custom operations that CAN'T be replaced by generated CRUD
#[derive(Default)]
pub struct {Entity}CustomOperations;

#[Object]
impl {Entity}CustomOperations {
    /// Search external API
    #[graphql(name = "Search{Entity}s")]
    async fn search(&self, ctx: &Context<'_>, query: String) -> Result<Vec<SearchResult>> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<ExternalService>>();
        // ... call external API
        Ok(vec![])
    }

    /// Trigger service action
    #[graphql(name = "Scan{Entity}")]
    async fn scan(&self, ctx: &Context<'_>, id: String) -> Result<ScanResult> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<ScannerService>>();
        // ... trigger scan
        Ok(ScanResult { ... })
    }
}
```

### Key Decisions

**When to use `GraphQLRelations` macro vs manual `ComplexObject`:**
| Scenario | Use |
|----------|-----|
| Entity has ONLY simple relations | `GraphQLRelations` macro |
| Entity needs computed fields | Manual `ComplexObject` |
| Entity needs custom relation logic | Manual `ComplexObject` |

**When to add `*CustomOperations`:**
- External API calls (TMDB, TVMaze, etc.)
- Service triggers (ScanLibrary, ProcessTorrent)
- Complex business logic that isn't simple CRUD

**Naming conventions:**
| Type | Pattern | Example |
|------|---------|---------|
| Entity struct | `{Entity}Entity` | `MovieEntity`, `LibraryEntity` |
| Custom ops | `{Entity}CustomOperations` | `MovieCustomOperations` |
| Result types | `{Entity}OperationResult` or specific | `ScanResult`, `ConsolidateResult` |

### What Goes Where (Quick Reference)

| Operation Type | Location | Example |
|----------------|----------|---------|
| List/filter/paginate | Generated by `GraphQLOperations` | `MovieEntities(Where: ...)` |
| Get by ID | Generated by `GraphQLOperations` | `MovieEntity(Id: "...")` |
| Create/Update/Delete | Generated by `GraphQLOperations` | `CreateMovieEntity` |
| Relations | `ComplexObject` impl | `library.Movies(Where: ...)` |
| Computed fields | `ComplexObject` impl | `library.ItemCount` |
| External API calls | `*CustomOperations` struct | `SearchMovies`, `SearchTvShows` |
| Service triggers | `*CustomOperations` struct | `ScanLibrary`, `ConsolidateLibrary` |

### Schema Integration

Export CustomOperations from `graphql/entities/mod.rs`:
```rust
pub use library::{LibraryEntity, ..., LibraryCustomOperations};
pub use movie::{MovieEntity, ..., MovieCustomOperations};
```

Merge into schema root in `graphql/schema.rs`:
```rust
#[derive(MergedObject, Default)]
pub struct QueryRoot(
    // Generated entity queries
    MovieEntityQueries, TvShowEntityQueries, LibraryEntityQueries, ...
    
    // Custom operations (includes both queries AND mutations)
    MovieCustomOperations, TvShowCustomOperations, LibraryCustomOperations, ...
);

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    // Generated entity mutations
    MovieEntityMutations, TvShowEntityMutations, LibraryEntityMutations, ...
    
    // Custom operations (same structs, Object can have query+mutation methods)
    MovieCustomOperations, TvShowCustomOperations, LibraryCustomOperations, ...
);
```

### Reference Implementation

**See `graphql/entities/library.rs`** for the canonical example with:
- Entity with 20+ fields and proper attributes
- ComplexObject with computed fields (`ItemCount`, `TotalSizeBytes`)
- ComplexObject with paginated relations (`Movies`, `TvShows`, `MediaFiles`)
- CustomOperations with service triggers (`ScanLibrary`, `ConsolidateLibrary`)
- Proper result types (`ScanResult`, `ConsolidateResult`)

## Migration Strategy (Revised)

### Step 1: Consolidate into entity files
For each domain, move custom operations from `graphql/queries/*.rs` and `graphql/mutations/*.rs` into the entity file:

```
graphql/queries/movies.rs (search_movies)     → entities/movie.rs (MovieCustomOperations)
graphql/mutations/movies.rs (addMovie, etc.)  → entities/movie.rs (MovieCustomOperations)
graphql/queries/tv_shows.rs (search_tv_shows) → entities/tv_show.rs (TvShowCustomOperations)
graphql/mutations/tv_shows.rs (addTvShow)     → entities/tv_show.rs (TvShowCustomOperations)
```

### Step 2: Add computed fields to ComplexObject
For each entity that needs computed fields:
- `MovieEntity`: `DownloadProgress`, `MediaFile` relation
- `TvShowEntity`: `Episodes` relation, `EpisodeCount`, `DownloadedEpisodeCount`
- `LibraryEntity`: `ItemCount`, `TotalSizeBytes`, content relations
- `EpisodeEntity`: `DownloadProgress`, `MediaFile` relation

### Step 3: Remove redundant query/mutation files
Once operations are consolidated, delete or gut the old files:
- `graphql/queries/movies.rs` → DELETE (now in MovieCustomOperations)
- `graphql/queries/tv_shows.rs` → DELETE (now in TvShowCustomOperations)
- `graphql/queries/libraries.rs` → DELETE (now in LibraryCustomOperations)
- `graphql/mutations/movies.rs` → DELETE (CRUD generated, custom in entity)
- etc.

### Step 4: Update frontend to use entity queries
```graphql
# Old manual query
movies(libraryId: "...") { id, title }

# New entity query
MovieEntities(Where: {LibraryId: {Eq: "..."}}) { 
  Edges { Node { Id, Title, DownloadProgress, MediaFile { Path } } }
}

# Custom operations unchanged
SearchMovies(Query: "inception") { Title, Year, ProviderId }
```

### Step 5: Update internal services
Services should use entity query pattern:
```rust
// Instead of: db.movies().list_by_library(lib_id)
MovieEntity::query(db.pool())
    .filter(MovieEntityWhereInput { library_id: Some(StringFilter::eq(&id)), ..Default::default() })
    .fetch_all().await
```

### Step 6: Remove db/ repository files
Once all callers are migrated, remove:
- `db/movies.rs` (MovieRecord, MovieRepository)
- `db/libraries.rs` (LibraryRecord, LibraryRepository)
- `db/episodes.rs`, `db/tv_shows.rs`, etc.

Keep only:
- `db/mod.rs` (Database struct, connection)
- `db/schema_sync.rs` (auto-migration)
- `db/sqlite_helpers.rs` (type conversions)
- `db/operations.rs` (aggregates, JOINs)
- `db/settings.rs` (app_settings key-value store)

## Files Status

### Entity Consolidation Progress

| Entity File | ComplexObject | CustomOperations | Status |
|-------------|---------------|------------------|--------|
| `library.rs` | ✅ ItemCount, TotalSizeBytes, Movies, TvShows, MediaFiles | ✅ ScanLibrary, ConsolidateLibrary | **DONE (Reference)** |
| `movie.rs` | ✅ DownloadProgress, MediaFile | ✅ SearchMovies, AddMovieFromTmdb, RefreshMovieMetadata | DONE |
| `tv_show.rs` | ✅ Episodes | ✅ SearchTvShows, AddTvShowFromProvider, RefreshTvShowMetadata | DONE |
| `torrent.rs` | Uses GraphQLRelations | - (uses existing TorrentMutations) | DONE |
| `episode.rs` | ? DownloadProgress, MediaFile | - | TODO |
| `album.rs` | ? Tracks, DownloadedTrackCount | - | TODO |
| `audiobook.rs` | ? Chapters, ChapterCount | - | TODO |

### Old Query/Mutation Files to Remove

After consolidation, these files become obsolete:

| File | Current Content | Migrate To | Status |
|------|-----------------|------------|--------|
| `queries/movies.rs` | search_movies, movies, movie, all_movies | MovieCustomOperations | TODO - remove redundant |
| `queries/tv_shows.rs` | search_tv_shows, tv_shows, tv_show | TvShowCustomOperations | TODO - remove redundant |
| `queries/libraries.rs` | libraries, library, library_stats | LibraryCustomOperations | TODO |
| `queries/episodes.rs` | episodes, episode | DELETE (use EpisodeEntities) | TODO |
| `queries/upcoming.rs` | upcoming_episodes | LibraryCustomOperations or keep | EVALUATE |
| `queries/music.rs` | artists, albums, tracks | DELETE (use entity queries) | TODO |
| `queries/audiobooks.rs` | audiobooks, chapters | DELETE (use entity queries) | TODO |
| `mutations/movies.rs` | addMovie, updateMovie, deleteMovie | DELETE (CRUD generated) | TODO |
| `mutations/tv_shows.rs` | addTvShow, updateTvShow, deleteTvShow | TvShowCustomOperations | MIGRATED |
| `mutations/libraries.rs` | createLibrary, updateLibrary, deleteLibrary, scanLibrary | LibraryCustomOperations | TODO |

### Internal services to update (use Entity::query instead of db.*)

- [ ] `services/file_matcher.rs` - uses db.movies(), db.episodes(), db.libraries()
- [ ] `services/file_processor.rs` - uses db.movies().set_media_file()
- [ ] `services/scanner.rs` - uses db.libraries(), db.movies(), db.episodes()
- [ ] `services/organizer.rs` - uses db.libraries(), db.episodes()
- [ ] `services/queues.rs` - uses db.movies(), db.libraries()
- [ ] `services/metadata.rs` - uses db.movies(), db.episodes()
- [ ] `jobs/content_progress.rs` - uses db.movies(), db.episodes()
- [ ] `jobs/rss_poller.rs` - uses db.libraries(), db.episodes()

Future Improvements (TODO)
--------------------------
### JOIN/Nested Relation Filters
Enable filtering entities by their related entity fields without fetching the relation:
```graphql
# Filter episodes by their TV show's library
Episodes(Where: { 
  TvShow: { LibraryId: { Eq: "..." }, Monitored: { Eq: true } }
  MediaFileId: { IsNull: true }
})
```
Would generate:
```sql
SELECT e.* FROM episodes e 
JOIN tv_shows ts ON ts.id = e.tv_show_id 
WHERE ts.library_id = ? AND ts.monitored = 1 AND e.media_file_id IS NULL
```
Use cases:
- `list_wanted_by_library` - episodes without files, filtered by tv_show.library_id
- `list_upcoming_by_user` - episodes airing soon, filtered through tv_show.library.user_id
- `list_needing_files` - any content type without associated media files

Legacy Code Still in Place (by design for now)
----------------------------------------------
- CoreSubscriptions remain service-backed (torrent progress, notifications, etc).
- Workflow/view-model operations (previously legacy domain endpoints) remain active
  alongside table operations for now.

Legacy Migration Checklist (short)
----------------------------------
- Inventory frontend usage of workflow operations (see `docs/graphql-operations-map.md`).
- Decide which workflow endpoints can be replaced with table queries + client-side shaping.
- Normalize list sort/filter/pagination for non-table list endpoints.
  - Deprecated now: indexers/indexer, rss_feeds, usenet_servers/usenet_server,
    usenet_downloads/usenet_download, notifications/notification.

Examples from current codebase (view models)
-------------------------------------------
- `TorrentProgress` and `ActiveDownloadCount` are subscription/event view models
  built from service state rather than a single table.
  - Types: `backend/src/graphql/types.rs`
  - Subscriptions: `backend/src/graphql/subscriptions.rs`

How these map to the new model
------------------------------
- Keep table-backed modules as the “source of truth” for data access, filters,
  and relations.
- View model resolvers should compose from table modules (or their repositories)
  to shape the response, reusing filter/order/pagination/relationship helpers
  where applicable.

References (Source Files)
-------------------------
- `/home/toby/dev/gema-2026/crates/jim-service/src/graphql.rs`
- `/home/toby/dev/gema-2026/crates/graphql-orm/src/lib.rs`
- `/home/toby/dev/gema-2026/crates/graphql-orm-macros/src/lib.rs`
- `/home/toby/dev/librarian/backend/src/graphql/filters.rs`
- `/home/toby/dev/librarian/backend/src/graphql/pagination.rs`
- `/home/toby/dev/librarian/backend/src/graphql/schema.rs`
- `/home/toby/dev/librarian/backend/src/db/mod.rs`
- `/home/toby/dev/librarian/backend/migrations_sqlite/*.sql`
- `/home/toby/dev/librarian/frontend/src/lib/graphql/types.ts`
