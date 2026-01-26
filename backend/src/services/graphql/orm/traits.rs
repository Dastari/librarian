//! Core traits for the GraphQL ORM layer
//!
//! These traits are implemented by the `#[derive(GraphQLEntity)]` and
//! `#[derive(GraphQLRelations)]` macros from `librarian-macros`.

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

/// Column definition for schema generation.
#[derive(Debug, Clone)]
pub struct ColumnDef {
    /// Column name in the database
    pub name: &'static str,
    /// SQLite column type (TEXT, INTEGER, REAL, BLOB)
    pub sql_type: &'static str,
    /// Whether the column can be NULL
    pub nullable: bool,
    /// Whether this is the primary key
    pub is_primary_key: bool,
    /// Default value expression (e.g., "datetime('now')")
    pub default: Option<&'static str>,
}

impl ColumnDef {
    /// Generate the column definition SQL
    pub fn to_sql(&self) -> String {
        let mut sql = format!("{} {}", self.name, self.sql_type);

        if self.is_primary_key {
            sql.push_str(" PRIMARY KEY");
        }

        if !self.nullable && !self.is_primary_key {
            sql.push_str(" NOT NULL");
        }

        if let Some(default) = self.default {
            sql.push_str(&format!(" DEFAULT {}", default));
        }

        sql
    }
}

/// Trait for database schema generation and migration.
///
/// Implemented by `#[derive(GraphQLEntity)]` macro.
pub trait DatabaseSchema: DatabaseEntity {
    /// Get all column definitions for this entity's table
    fn columns() -> &'static [ColumnDef];

    /// Generate CREATE TABLE IF NOT EXISTS SQL
    fn create_table_sql() -> String {
        let column_defs: Vec<String> = Self::columns().iter().map(|c| c.to_sql()).collect();

        format!(
            "CREATE TABLE IF NOT EXISTS {} (\n  {}\n)",
            Self::TABLE_NAME,
            column_defs.join(",\n  ")
        )
    }

    /// Get column names that exist in the entity definition
    fn defined_column_names() -> Vec<&'static str> {
        Self::columns().iter().map(|c| c.name).collect()
    }
}

/// Metadata about a database entity (table).
///
/// Implemented by `#[derive(GraphQLEntity)]` macro.
pub trait DatabaseEntity: Sized + Send + Sync {
    /// The SQL table name (e.g., "libraries")
    const TABLE_NAME: &'static str;

    /// The GraphQL plural name (e.g., "Libraries")
    const PLURAL_NAME: &'static str;

    /// The primary key column name (e.g., "id")
    const PRIMARY_KEY: &'static str;

    /// Default sort column for list queries (e.g., "name")
    const DEFAULT_SORT: &'static str;

    /// Default sort direction
    const DEFAULT_SORT_DIR: &'static str = "ASC";

    /// List of all column names in the table
    fn column_names() -> &'static [&'static str];

    /// Build a SELECT query for all columns
    fn select_sql() -> String {
        let columns = Self::column_names().join(", ");
        format!("SELECT {} FROM {}", columns, Self::TABLE_NAME)
    }
}

/// Trait for applying filters to a SQL query.
///
/// Implemented by the generated `*WhereInput` structs.
pub trait DatabaseFilter: Send + Sync {
    /// Apply this filter to a query builder, returning the WHERE clause fragments
    /// and the values to bind.
    fn to_sql_conditions(&self) -> (Vec<String>, Vec<SqlValue>);

    /// Check if the filter has any conditions
    fn is_empty(&self) -> bool;
}

/// Trait for applying sort order to a SQL query.
///
/// Implemented by the generated `*OrderByInput` structs.
pub trait DatabaseOrderBy: Send + Sync {
    /// Get the ORDER BY clause fragment (e.g., "name ASC, created_at DESC")
    fn to_sql_order(&self) -> Option<String>;
}

/// Trait for decoding a database row into an entity.
///
/// Implemented by `#[derive(GraphQLEntity)]` macro with SQLite-specific
/// type conversions (TEXT→UUID, INTEGER→bool, TEXT→DateTime, etc.)
pub trait FromSqlRow: Sized {
    /// Decode a SQLite row into this entity type
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error>;
}

/// Trait for loading relations based on GraphQL look_ahead.
///
/// Implemented by `#[derive(GraphQLRelations)]` macro.
/// Uses look_ahead to only load relations that are actually requested.
#[allow(async_fn_in_trait)]
pub trait RelationLoader: Sized + Send + Sync {
    /// Load relations for a single entity based on the selection set.
    ///
    /// The `selection` parameter comes from async-graphql's look_ahead,
    /// allowing us to only load relations that are actually requested.
    async fn load_relations(
        &mut self,
        pool: &SqlitePool,
        selection: &[async_graphql::context::SelectionField<'_>],
    ) -> Result<(), sqlx::Error>;

    /// Load relations for multiple entities in bulk to avoid N+1 queries.
    ///
    /// This collects all parent IDs and loads relations in a single query per relation type.
    async fn bulk_load_relations(
        entities: &mut [Self],
        pool: &SqlitePool,
        selection: &[async_graphql::context::SelectionField<'_>],
    ) -> Result<(), sqlx::Error>;
}

/// Relation metadata for look_ahead traversal.
#[derive(Debug, Clone)]
pub struct RelationMetadata {
    pub field_name: &'static str,
    pub target_type: &'static str,
    pub is_multiple: bool,
}

/// Sort direction for ORDER BY clauses.
#[derive(async_graphql::Enum, Copy, Clone, Debug, Default, Eq, PartialEq)]
#[graphql(name = "SortDirection")]
pub enum OrderDirection {
    /// Ascending order (A-Z, 1-9, oldest-newest)
    #[default]
    #[graphql(name = "Asc")]
    Asc,
    /// Descending order (Z-A, 9-1, newest-oldest)
    #[graphql(name = "Desc")]
    Desc,
}

impl OrderDirection {
    /// Convert to SQL order string
    pub fn to_sql(&self) -> &'static str {
        match self {
            OrderDirection::Asc => "ASC",
            OrderDirection::Desc => "DESC",
        }
    }
}

/// Represents a SQL value that can be bound to a query.
///
/// Used by filters to collect values for parameterized queries.
#[derive(Debug, Clone)]
pub enum SqlValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

impl SqlValue {
    /// Bind this value to a sqlx query builder at the given parameter index
    pub fn bind_to_query<'q>(
        &'q self,
        query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    ) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
        match self {
            SqlValue::String(s) => query.bind(s.as_str()),
            SqlValue::Int(i) => query.bind(*i),
            SqlValue::Float(f) => query.bind(*f),
            SqlValue::Bool(b) => query.bind(if *b { 1i32 } else { 0i32 }),
            SqlValue::Null => query.bind(None::<String>),
        }
    }
}

/// Pagination input for offset-based pagination.
#[derive(async_graphql::InputObject, Default, Clone, Debug)]
#[graphql(name = "PageInput")]
pub struct PageInput {
    /// Maximum number of items to return (default: 25, max: 100)
    #[graphql(name = "Limit")]
    pub limit: Option<i32>,

    /// Number of items to skip
    #[graphql(name = "Offset")]
    pub offset: Option<i64>,
}

impl PageInput {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(25).min(100) as i64
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0)
    }
}

/// Pagination input for cursor-based (Relay-style) pagination.
#[derive(async_graphql::InputObject, Default, Clone, Debug)]
#[graphql(name = "CursorInput")]
pub struct CursorInput {
    /// Return the first N items
    #[graphql(name = "First")]
    pub first: Option<i32>,

    /// Return items after this cursor
    #[graphql(name = "After")]
    pub after: Option<String>,

    /// Return the last N items (not commonly used)
    #[graphql(name = "Last")]
    pub last: Option<i32>,

    /// Return items before this cursor (not commonly used)
    #[graphql(name = "Before")]
    pub before: Option<String>,
}

impl CursorInput {
    /// Parse cursor input into offset and limit
    pub fn to_offset_limit(&self) -> Result<(i64, i64), &'static str> {
        let limit = self.first.unwrap_or(25).min(100) as i64;

        let offset = if let Some(ref cursor) = self.after {
            super::decode_cursor(cursor)? + 1
        } else {
            0
        };

        Ok((offset, limit))
    }
}

/// Input for entity change subscriptions.
#[derive(async_graphql::InputObject, Default, Clone, Debug)]
#[graphql(name = "SubscriptionFilterInput")]
pub struct SubscriptionFilterInput {
    /// Only receive events for entities matching this ID
    #[graphql(name = "Id")]
    pub id: Option<String>,

    /// Only receive events of these types
    #[graphql(name = "Actions")]
    pub actions: Option<Vec<ChangeAction>>,
}

/// Type of change for subscription events.
#[derive(
    async_graphql::Enum, Copy, Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize,
)]
#[graphql(name = "ChangeAction")]
pub enum ChangeAction {
    #[graphql(name = "Created")]
    Created,
    #[graphql(name = "Updated")]
    Updated,
    #[graphql(name = "Deleted")]
    Deleted,
}

/// Float filter for decimal/float fields.
#[derive(async_graphql::InputObject, Default, Clone, Debug)]
#[graphql(name = "FloatFilter")]
pub struct FloatFilter {
    /// Equals
    #[graphql(name = "Eq")]
    pub eq: Option<f64>,
    /// Not equals
    #[graphql(name = "Ne")]
    pub ne: Option<f64>,
    /// Less than
    #[graphql(name = "Lt")]
    pub lt: Option<f64>,
    /// Less than or equal
    #[graphql(name = "Lte")]
    pub lte: Option<f64>,
    /// Greater than
    #[graphql(name = "Gt")]
    pub gt: Option<f64>,
    /// Greater than or equal
    #[graphql(name = "Gte")]
    pub gte: Option<f64>,
}

impl FloatFilter {
    pub fn is_empty(&self) -> bool {
        self.eq.is_none()
            && self.ne.is_none()
            && self.lt.is_none()
            && self.lte.is_none()
            && self.gt.is_none()
            && self.gte.is_none()
    }
}
