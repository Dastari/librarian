---
name: ""
overview: ""
todos: []
isProject: false
---

# GraphQL Macro-First ORM Implementation Plan

> **Reference Document**: `docs/graphql-refactor-plan.md`

> **Naming Convention**: `.cursor/rules/graphql-naming-convention.mdc` (PascalCase for all GraphQL-exposed names)

## Goal

Single source of truth: define each table once in a Rust model with macros/traits that generate SQLite schema/migrations, GraphQL types/queries/mutations/subscriptions, and frontend types via introspection. Adding/removing a field should only require editing the model and regenerating artifacts.

---

## Current State Analysis

### What Exists

- `librarian-macros` crate with `mutation_result!` macro
- Manual GraphQL queries/mutations in `backend/src/graphql/queries/*.rs` and `mutations/*.rs`
- Manual `FromRow` implementations in `backend/src/db/*.rs`
- Basic filter types in `backend/src/graphql/filters.rs` (StringFilter, IntFilter, BoolFilter, DateFilter)
- Relay-style pagination in `backend/src/graphql/pagination.rs` with `define_connection!` macro
- 28+ database tables defined in `migrations_sqlite/001_initial_schema.sql`

### What's Missing

- `GraphQLFilters` derive macro (generates WhereInput, OrderByInput, filter structs)
- `Relations` derive macro (generates RelationshipLoader, nested loading)
- `DatabaseEntity` trait and derive (table name, columns, primary key, default sort)
- `DatabaseFilter` trait (apply filters to SQL query builder)
- `FromSqlRow` derive (automatic row decoding with SQLite type conversions)
- ORM query builder layer using `sqlx::QueryBuilder`
- Macro-generated root fields (query/mutation/subscription per table)

---

## Phase 1: ORM Foundation in `librarian-macros`

### 1.1 Add Dependencies to librarian-macros

```toml
[dependencies]
proc-macro2 = "1"
quote = "1"
syn = { version = "2", features = ["full", "parsing", "extra-traits"] }
convert_case = "0.6"  # For PascalCase conversion
```

### 1.2 Define Core Traits (in backend)

Create `backend/src/graphql/orm/mod.rs`:

```rust
// Traits that macros will implement
pub trait DatabaseEntity {
    const TABLE_NAME: &'static str;
    const PLURAL_NAME: &'static str;
    const PRIMARY_KEY: &'static str;
    const DEFAULT_SORT: &'static str;

    fn column_names() -> &'static [&'static str];
}

pub trait DatabaseFilter {
    fn apply_to_query(&self, builder: &mut QueryBuilder<'_, Sqlite>) -> Result<()>;
}

pub trait FromSqlRow: Sized {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error>;
}

pub trait OrderByInput {
    fn apply_to_query(&self, builder: &mut QueryBuilder<'_, Sqlite>);
}

pub trait RelationshipLoader {
    async fn load_relations<'c>(
        &mut self,
        pool: &SqlitePool,
        look_ahead: &Lookahead<'_>,
    ) -> Result<()>;
}
```

### 1.3 Implement `#[derive(GraphQLEntity)]` Macro

This is the main derive macro that generates:

- `SimpleObject` implementation with PascalCase field names
- `WhereInput` struct for filtering
- `OrderByInput` struct for sorting
- Filter conversion function
- `DatabaseEntity` impl
- `FromSqlRow` impl

```rust
// Usage example:
#[derive(GraphQLEntity, Clone, Debug)]
#[graphql_entity(table = "libraries", plural = "Libraries", default_sort = "Name")]
pub struct Library {
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,

    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[filterable(type = "bool")]
    pub auto_scan: bool,

    #[filterable(type = "date")]
    #[sortable]
    pub created_at: chrono::DateTime<chrono::Utc>,

    // Relations are separate (optional, loaded on demand)
}
```

**Generated code includes:**

- `#[derive(SimpleObject)]` with `#[graphql(name = "PascalCase")]` on each field
- `LibraryWhereInput` struct
- `LibraryOrderByInput` struct
- `impl DatabaseEntity for Library`
- `impl FromSqlRow for Library`
- Conversion function `convert_library_where_to_filter(...)`

### 1.4 Implement `#[derive(GraphQLRelations)]` Macro

For entities with relations:

```rust
#[derive(GraphQLRelations)]
#[graphql_entity(table = "libraries")]
pub struct Library {
    // ... fields ...

    #[relation(target = "Movie", from = "id", to = "library_id", multiple = true)]
    pub movies: Vec<Movie>,
}
```

**Generated code:**

- `impl RelationshipLoader for Library`
- Selective loading based on lookahead
- Bulk loading to avoid N+1

---

## Phase 2: Query Builder Infrastructure

### 2.1 Create ORM Module Structure

```
backend/src/graphql/orm/
├── mod.rs           # Trait definitions, re-exports
├── builder.rs       # QueryBuilder wrapper with SQLite parameterization
├── filters.rs       # Extended filter types (FloatFilter, EnumFilter)
├── pagination.rs    # Cursor pagination helpers
└── execution.rs     # Shared execution helpers (fetch_with_filter)
```

### 2.2 Implement QueryBuilder Wrapper

```rust
// backend/src/graphql/orm/builder.rs
pub struct EntityQueryBuilder<'q, E: DatabaseEntity> {
    builder: QueryBuilder<'q, Sqlite>,
    _phantom: PhantomData<E>,
}

impl<'q, E: DatabaseEntity> EntityQueryBuilder<'q, E> {
    pub fn select_all() -> Self { ... }
    pub fn with_filter<F: DatabaseFilter>(self, filter: &F) -> Self { ... }
    pub fn with_order<O: OrderByInput>(self, order: &O) -> Self { ... }
    pub fn with_pagination(self, first: Option<i32>, after: Option<String>) -> Self { ... }
    pub async fn fetch_all(self, pool: &SqlitePool) -> Result<Vec<E>> { ... }
    pub async fn fetch_one(self, pool: &SqlitePool) -> Result<Option<E>> { ... }
    pub async fn count(self, pool: &SqlitePool) -> Result<i64> { ... }
}
```

### 2.3 Extend Filter Types

Add to `backend/src/graphql/filters.rs` or new ORM module:

```rust
/// Filter for float/decimal fields
#[derive(InputObject, Default, Clone, Debug)]
pub struct FloatFilter {
    pub eq: Option<f64>,
    pub ne: Option<f64>,
    pub lt: Option<f64>,
    pub lte: Option<f64>,
    pub gt: Option<f64>,
    pub gte: Option<f64>,
}

/// Filter for enum fields (generic over enum type)
#[derive(InputObject, Default, Clone, Debug)]
pub struct EnumFilter<T: async_graphql::Enum + Clone> {
    pub eq: Option<T>,
    pub ne: Option<T>,
    #[graphql(name = "in")]
    pub in_list: Option<Vec<T>>,
    pub not_in: Option<Vec<T>>,
}
```

---

## Phase 3: Table Module Generator

### 3.1 Implement Table Root Field Generator

The macro should also generate the GraphQL root fields. Create helper macro or extend `GraphQLEntity`:

```rust
// Generates:
// - Query: Library(Id: ID!) -> Library
// - Query: Libraries(Where: LibraryWhereInput, Sort: [LibraryOrderByInput!], Page: PageInput, Cursor: CursorInput) -> LibraryConnection
// - Mutation: CreateLibrary(Input: CreateLibraryInput!) -> Library
// - Mutation: UpdateLibrary(Id: ID!, Patch: UpdateLibraryInput!) -> Library
// - Mutation: DeleteLibrary(Id: ID!) -> MutationResult
// - Subscription: LibraryChanged(Where: LibraryWhereInput) -> LibraryChangedEvent
```

### 3.2 Generate Query Structs

```rust
// Auto-generated in backend/src/graphql/tables/libraries.rs
#[derive(Default)]
pub struct LibraryTableQueries;

#[Object]
impl LibraryTableQueries {
    #[graphql(name = "Library")]
    async fn library(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Library>> {
        // Generated implementation using EntityQueryBuilder
    }

    #[graphql(name = "Libraries")]
    async fn libraries(
        &self,
        ctx: &Context<'_>,
        r#where: Option<LibraryWhereInput>,
        sort: Option<Vec<LibraryOrderByInput>>,
        page: Option<PageInput>,
        cursor: Option<CursorInput>,
    ) -> Result<LibraryConnection> {
        // Generated implementation
    }
}
```

### 3.3 Generate Mutation Structs

```rust
#[derive(Default)]
pub struct LibraryTableMutations;

#[Object]
impl LibraryTableMutations {
    #[graphql(name = "CreateLibrary")]
    async fn create_library(&self, ctx: &Context<'_>, input: CreateLibraryInput) -> Result<Library> { ... }

    #[graphql(name = "UpdateLibrary")]
    async fn update_library(&self, ctx: &Context<'_>, id: ID, patch: UpdateLibraryInput) -> Result<Library> { ... }

    #[graphql(name = "DeleteLibrary")]
    async fn delete_library(&self, ctx: &Context<'_>, id: ID) -> Result<MutationResult> { ... }
}
```

---

## Phase 4: Library Model as Reference Implementation

### 4.1 Create New Library Entity

Create `backend/src/graphql/entities/library.rs`:

```rust
use crate::graphql::orm::*;
use librarian_macros::{GraphQLEntity, GraphQLRelations};

#[derive(GraphQLEntity, GraphQLRelations, Clone, Debug)]
#[graphql_entity(table = "libraries", plural = "Libraries", default_sort = "Name")]
#[serde(rename_all = "PascalCase")]
pub struct Library {
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,

    #[filterable(type = "string")]
    pub user_id: String,

    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[filterable(type = "string")]
    pub path: String,

    #[filterable(type = "enum")]
    #[sortable]
    pub library_type: LibraryType,

    pub icon: Option<String>,
    pub color: Option<String>,

    #[filterable(type = "bool")]
    pub auto_scan: bool,

    #[filterable(type = "number")]
    pub scan_interval_minutes: i32,

    #[filterable(type = "bool")]
    pub watch_for_changes: bool,

    #[filterable(type = "date")]
    #[sortable]
    pub last_scanned_at: Option<chrono::DateTime<chrono::Utc>>,

    #[filterable(type = "bool")]
    pub scanning: bool,

    #[filterable(type = "date")]
    #[sortable]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: chrono::DateTime<chrono::Utc>,

    // Relations
    #[relation(target = "Movie", from = "id", to = "library_id", multiple = true)]
    #[graphql(skip)] // Loaded via resolver
    pub movies: Vec<Movie>,

    #[relation(target = "TvShow", from = "id", to = "library_id", multiple = true)]
    #[graphql(skip)]
    pub tv_shows: Vec<TvShow>,
}
```

### 4.2 Wire Into Schema

Update `backend/src/graphql/schema.rs`:

```rust
use crate::graphql::entities::library::{LibraryTableQueries, LibraryTableMutations};

#[derive(MergedObject, Default)]
pub struct QueryRoot(
    // ... existing queries ...
    LibraryTableQueries,  // New macro-generated queries
);

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    // ... existing mutations ...
    LibraryTableMutations,  // New macro-generated mutations
);
```

### 4.3 Verify Generated GraphQL Schema

Expected GraphQL operations:

```graphql
type Query {
  # Single entity lookup
  Library(Id: ID!): Library

  # List with filters/sort/pagination
  Libraries(
    Where: LibraryWhereInput
    Sort: [LibraryOrderByInput!]
    Page: PageInput
    Cursor: CursorInput
  ): LibraryConnection!
}

type Mutation {
  CreateLibrary(Input: CreateLibraryInput!): Library!
  UpdateLibrary(Id: ID!, Patch: UpdateLibraryInput!): Library!
  DeleteLibrary(Id: ID!): MutationResult!
}

type Subscription {
  LibraryChanged(Where: LibraryWhereInput): LibraryChangedEvent!
}

input LibraryWhereInput {
  Id: StringFilter
  UserId: StringFilter
  Name: StringFilter
  Path: StringFilter
  LibraryType: LibraryTypeFilter
  AutoScan: BoolFilter
  Scanning: BoolFilter
  CreatedAt: DateFilter
  UpdatedAt: DateFilter
  And: [LibraryWhereInput!]
  Or: [LibraryWhereInput!]
  Not: LibraryWhereInput
}

input LibraryOrderByInput {
  Id: OrderDirection
  Name: OrderDirection
  LibraryType: OrderDirection
  CreatedAt: OrderDirection
  UpdatedAt: OrderDirection
}
```

---

## Phase 5: Migrate Remaining Tables

### Priority Order (based on frontend usage and relations)

1. **Core Content Tables** (high relation count)
   - `movies` → Movie
   - `tv_shows` → TvShow
   - `episodes` → Episode
   - `media_files` → MediaFile

2. **Music Tables**
   - `artists` → Artist
   - `albums` → Album
   - `tracks` → Track

3. **Downloads & Indexers**
   - `torrents` → TorrentRecord (DB-backed, not live torrent)
   - `indexers` → Indexer
   - `rss_feeds` → RssFeed

4. **User & Settings**
   - `users` → User
   - `notifications` → Notification
   - `app_settings` → AppSetting

5. **Support Tables**
   - `pending_file_matches`, `torrent_files`, `subtitles`, etc.

### Migration Pattern Per Table

1. Create entity struct with `#[derive(GraphQLEntity)]`
2. Add relations with `#[derive(GraphQLRelations)]`
3. Wire into schema (queries, mutations, subscriptions)
4. Update frontend to use new operations
5. Deprecate old manual resolvers
6. Remove old code after frontend migration complete

---

## Phase 6: Non-Table Operations Alignment

### 6.1 Utility Operations (Manual but Consistent)

Keep manual implementations for:

- Filesystem operations (browse, copy, move, delete)
- Scanner operations (scan_library, scan progress)
- Torrent live operations (add, pause, resume, remove)
- Metadata refresh operations

### 6.2 Align Utility Query Shapes

Non-table list queries should use the same input shapes:

```graphql
# Example: Filesystem browsing should mirror table patterns
query BrowseDirectory {
  DirectoryContents(
    Path: "/media/movies"
    Where: { Name: { Contains: "movie" } }
    Sort: [{ Name: Asc }]
    Page: { Limit: 50, Offset: 0 }
  ) {
    Nodes {
      Name
      Path
      IsDirectory
      Size
    }
    PageInfo {
      TotalCount
      HasNextPage
    }
  }
}
```

---

## Phase 7: Frontend Type Generation

### 7.1 Add GraphQL Codegen

```bash
cd frontend
pnpm add -D @graphql-codegen/cli @graphql-codegen/typescript @graphql-codegen/typescript-operations
```

### 7.2 Configure Codegen

Create `frontend/codegen.ts`:

```typescript
import type { CodegenConfig } from "@graphql-codegen/cli";

const config: CodegenConfig = {
  schema: "http://localhost:3001/graphql",
  documents: ["src/**/*.tsx", "src/**/*.ts"],
  generates: {
    "./src/lib/graphql/generated/types.ts": {
      plugins: ["typescript", "typescript-operations"],
      config: {
        scalars: {
          DateTime: "string",
          ID: "string",
        },
      },
    },
  },
};

export default config;
```

### 7.3 Add NPM Script

```json
{
  "scripts": {
    "codegen": "graphql-codegen --config codegen.ts",
    "codegen:watch": "graphql-codegen --config codegen.ts --watch"
  }
}
```

---

## Implementation Tasks Checklist

### Phase 1: Macro Foundation ✅ COMPLETE

- [x] Add `convert_case` dependency to librarian-macros
- [x] Create `backend/src/graphql/orm/mod.rs` with trait definitions
- [x] Implement `GraphQLEntity` derive macro (basic version)
  - [x] Generate WhereInput struct with PascalCase names
  - [x] Generate OrderByInput struct with PascalCase names
  - [x] Generate DatabaseEntity impl
  - [x] Generate FromSqlRow impl
  - [x] Generate DatabaseFilter impl for WhereInput
  - [x] Generate DatabaseOrderBy impl for OrderByInput
- [x] Implement `GraphQLRelations` derive macro
  - [x] Generate RelationLoader impl with look_ahead support
  - [x] Support for bulk relation loading (N+1 prevention)
- [ ] Add unit tests for macro expansion

### Phase 2: Query Builder ✅ COMPLETE

- [x] Create `backend/src/graphql/orm/builder.rs`
- [x] Implement EntityQueryBuilder with filter support
- [x] Implement pagination helpers (offset + cursor)
- [x] Add FloatFilter type
- [x] Add PageInput and CursorInput types
- [ ] Test query builder with Library entity

### Phase 3: Root Field Generation (PENDING)

- [ ] Extend macro to generate Query struct for each entity
- [ ] Extend macro to generate Mutation struct for each entity
- [ ] Extend macro to generate Subscription struct for each entity
- [ ] Wire into MergedObject pattern

### Phase 4: Library Reference Implementation ✅ COMPLETE

- [x] Create `backend/src/graphql/entities/library.rs`
- [x] Migrate Library to new pattern (simplified version)
- [ ] Expand with all fields from LibraryRecord
- [ ] Update frontend to use new Library operations
- [ ] Verify filters, sorts, pagination work correctly
- [ ] Verify relation loading works

### Phase 5: Table Migration (PENDING)

- [ ] Migrate Movie entity
- [ ] Migrate TvShow entity
- [ ] Migrate Episode entity
- [ ] Migrate MediaFile entity
- [ ] Continue with remaining tables...

### Phase 6: Frontend Codegen (PENDING)

- [ ] Add codegen dependencies
- [ ] Configure codegen
- [ ] Run initial generation
- [ ] Replace manual types with generated types
- [ ] Set up CI to verify types match schema

---

## Implementation Status

### Completed (2025-01-25)

**librarian-macros enhancements:**

- Added `GraphQLEntity` derive macro
- Added `GraphQLRelations` derive macro
- Macros generate PascalCase GraphQL names via `#[graphql(name = "...")]`
- Support for filter types: string, number, boolean, date
- Support for sortable fields
- Support for db_column mapping
- Support for json_field, boolean_field, date_field conversions
- Relation loading with look_ahead support

**backend/src/graphql/orm module:**

- `traits.rs` - Core traits (DatabaseEntity, DatabaseFilter, FromSqlRow, RelationLoader, etc.)
- `builder.rs` - EntityQueryBuilder for parameterized SQL queries
- Filter types: StringFilter, IntFilter, BoolFilter, DateFilter, FloatFilter
- Pagination: PageInput, CursorInput with Relay-style connections

**backend/src/graphql/entities module:**

- `library.rs` - LibraryEntity as reference implementation

### Generated GraphQL Types (from LibraryEntity)

The macro generates:

- `LibraryEntityWhereInput` - Filter input with And/Or/Not support
- `LibraryEntityOrderByInput` - Sort input
- `impl DatabaseEntity for LibraryEntity`
- `impl DatabaseFilter for LibraryEntityWhereInput`
- `impl DatabaseOrderBy for LibraryEntityOrderByInput`
- `impl FromSqlRow for LibraryEntity`
- `impl RelationLoader for LibraryEntity`

### Completed (Phase 3) - Query/Mutation/Subscription Generation

**GraphQLOperations macro now generates:**

- `{Entity}Queries` - Query struct with list and get-by-ID operations
- `{Entity}Mutations` - Mutation struct with create/update/delete
- `{Entity}Subscriptions` - Subscription struct for real-time updates
- `{Entity}Connection` and `{Entity}Edge` - Pagination types
- `Create{Entity}Input` and `Update{Entity}Input` - Input types
- `{Entity}Result` - Mutation result type
- `{Entity}ChangedEvent` - Subscription event type

**Generated queries use:**

- `EntityQueryBuilder` for safe parameterized SQL
- Filters via `{Entity}WhereInput`
- Sorting via `{Entity}OrderByInput`
- Pagination via `PageInput`
- Connection pattern for Relay compatibility

### Next Steps

1. **Expand LibraryEntity** with all fields from the database schema
2. **Test the generated operations** by wiring into schema
3. **Add relations** (Movies, TvShows, etc.) to LibraryEntity
4. **Migrate additional entities** (Movie, TvShow, Episode)

### How to Use

To use the macro system for a new entity:

```rust
#[derive(GraphQLEntity, GraphQLRelations, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "MyEntity")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "my_table", plural = "MyEntities", default_sort = "name")]
pub struct MyEntity {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    // ... more fields
}
```

Then wire into schema:

```rust
#[derive(MergedObject, Default)]
pub struct QueryRoot(
    // ... existing queries ...
    MyEntityQueries,
);

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    // ... existing mutations ...
    MyEntityMutations,
);
```

---

## Risk Mitigation

1. **SQL Injection**: Use `sqlx::QueryBuilder` with parameterized queries exclusively
2. **Breaking Changes**: Keep old resolvers during migration, deprecate gradually
3. **Type Drift**: Generate frontend types from schema to prevent manual type staleness
4. **Performance**: Use lookahead for relation loading to prevent N+1 queries
5. **PascalCase Compliance**: Macro enforces `#[graphql(name = "PascalCase")]` on all fields

---

## Success Criteria

1. Adding a new field to Library requires only:
   - Add field to struct definition
   - Run migration (if DB change needed)
   - Frontend types auto-update via codegen

2. All table queries support:
   - Consistent Where/Sort/Page/Cursor inputs
   - Relay-style connections
   - Nested relation loading

3. GraphQL schema uses PascalCase exclusively for all exposed names

4. Frontend has zero manual type definitions for GraphQL operations
