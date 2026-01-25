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
its real `library_id` relationships (e.g. `movies`, `tv_shows`, `artists`, `albums`,
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
Current direction (desired, not fully implemented yet):
- Macro-first design as described above; one struct drives GraphQL + SQL + CRUD + subscriptions.
- View models and utility operations are the only manual GraphQL surface.

Notes on current branch state:
- Existing work introduced manual table/workflow modules and extra boilerplate.
- This does not yet match the intended single-source-of-truth macro model.
- Future work should align the implementation with the `jim-service` pattern
  and remove redundant/manual schema wiring.
- Operations map documented in `docs/graphql-operations-map.md`.

In progress:
- Expand table-level mutations with per-table typed inputs (beyond shared column-value inputs).
- Reintroduce view-model queries (aggregates) on top of table modules as needed.
- Audit workflow endpoints for deprecation once table equivalents are stable.

Pending:
- Frontend GraphQL type generation wiring.
- Review and normalize list sorting/filtering/pagination across workflow queries.

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
