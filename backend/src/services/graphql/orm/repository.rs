//! Repository pattern for entity data access
//!
//! This module provides a unified interface for querying entities
//! that can be used by both GraphQL resolvers and internal service code.
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use crate::graphql::entities::{MovieEntity, MovieEntityWhereInput};
//! use crate::graphql::filters::StringFilter;
//!
//! // Find all movies in a library
//! let movies = MovieEntity::query(&pool)
//!     .filter(MovieEntityWhereInput {
//!         library_id: Some(StringFilter::eq(library_id)),
//!         ..Default::default()
//!     })
//!     .fetch_all()
//!     .await?;
//!
//! // Find one movie by ID
//! let movie = MovieEntity::get(&pool, "some-id").await?;
//!
//! // Count movies
//! let count = MovieEntity::count_query(&pool)
//!     .filter(MovieEntityWhereInput {
//!         monitored: Some(BoolFilter::is_true()),
//!         ..Default::default()
//!     })
//!     .execute()
//!     .await?;
//! ```

use crate::services::graphql::orm::{
    DatabaseEntity, DatabaseFilter, DatabaseOrderBy, EntityQuery, FromSqlRow, SqlValue,
};
use sqlx::SqlitePool;

/// Query builder for finding entities
///
/// Wraps EntityQuery and provides a fluent interface for building queries.
pub struct FindQuery<'a, E, F, O>
where
    E: DatabaseEntity + FromSqlRow,
    F: DatabaseFilter + Default,
    O: DatabaseOrderBy + Default,
{
    pool: &'a SqlitePool,
    filter: Option<F>,
    order_by: Vec<O>,
    limit: Option<i64>,
    offset: Option<i64>,
    _marker: std::marker::PhantomData<E>,
}

impl<'a, E, F, O> FindQuery<'a, E, F, O>
where
    E: DatabaseEntity + FromSqlRow,
    F: DatabaseFilter + Default,
    O: DatabaseOrderBy + Default,
{
    /// Create a new find query
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            pool,
            filter: None,
            order_by: Vec::new(),
            limit: None,
            offset: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Set the filter
    pub fn filter(mut self, filter: F) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Add ordering
    pub fn order_by(mut self, order_by: Vec<O>) -> Self {
        self.order_by = order_by;
        self
    }

    /// Set limit
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set offset
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Set pagination (limit and offset)
    pub fn paginate(mut self, limit: i64, offset: i64) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }

    /// Execute and fetch all results
    pub async fn fetch_all(self) -> Result<Vec<E>, sqlx::Error> {
        let mut query = EntityQuery::<E>::new();

        // Apply filter using EntityQuery's filter method
        if let Some(ref filter) = self.filter {
            query = query.filter(filter);
        }

        // Apply ordering using EntityQuery's order_by method
        for order in &self.order_by {
            query = query.order_by(order);
        }

        // Apply default ordering if none specified
        if self.order_by.is_empty() {
            query = query.default_order();
        }

        // Apply pagination
        if let Some(limit) = self.limit {
            query = query.limit(limit);
        }
        if let Some(offset) = self.offset {
            query = query.offset(offset);
        }

        query.fetch_all(self.pool).await
    }

    /// Execute and fetch one optional result
    pub async fn fetch_optional(self) -> Result<Option<E>, sqlx::Error> {
        let results = self.limit(1).fetch_all().await?;
        Ok(results.into_iter().next())
    }

    /// Execute and fetch exactly one result (errors if not found)
    pub async fn fetch_one(self) -> Result<E, sqlx::Error> {
        self.fetch_optional()
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }
}

/// Query builder for counting entities
pub struct CountQuery<'a, F>
where
    F: DatabaseFilter + Default,
{
    pool: &'a SqlitePool,
    table_name: &'static str,
    filter: Option<F>,
}

impl<'a, F> CountQuery<'a, F>
where
    F: DatabaseFilter + Default,
{
    /// Create a new count query
    pub fn new(pool: &'a SqlitePool, table_name: &'static str) -> Self {
        Self {
            pool,
            table_name,
            filter: None,
        }
    }

    /// Set the filter
    pub fn filter(mut self, filter: F) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Execute the count query
    pub async fn execute(self) -> Result<i64, sqlx::Error> {
        let mut sql = format!("SELECT COUNT(*) FROM {}", self.table_name);
        let mut values: Vec<SqlValue> = Vec::new();

        if let Some(ref filter) = self.filter {
            if !filter.is_empty() {
                let (conditions, filter_values) = filter.to_sql_conditions();
                if !conditions.is_empty() {
                    // Join conditions with AND and renumber parameters
                    let mut param_counter = 0;
                    let mut renumbered_conditions: Vec<String> = Vec::new();
                    for condition in conditions {
                        let mut c = condition;
                        while c.contains('?') {
                            param_counter += 1;
                            c = c.replacen('?', &format!("?{}", param_counter), 1);
                        }
                        renumbered_conditions.push(c);
                    }
                    sql.push_str(" WHERE ");
                    sql.push_str(&renumbered_conditions.join(" AND "));
                    values = filter_values;
                }
            }
        }

        // Execute count query
        let count = execute_count_with_binds(&sql, &values, self.pool).await?;
        Ok(count)
    }
}

/// Execute a count query with bindings
async fn execute_count_with_binds(
    sql: &str,
    values: &[SqlValue],
    pool: &SqlitePool,
) -> Result<i64, sqlx::Error> {
    use sqlx::Row;

    match values.len() {
        0 => {
            let row: (i64,) = sqlx::query_as(sql).fetch_one(pool).await?;
            Ok(row.0)
        }
        _ => {
            let mut q = sqlx::query(sql);
            for v in values {
                q = match v {
                    SqlValue::String(s) => q.bind(s.as_str()),
                    SqlValue::Int(i) => q.bind(*i),
                    SqlValue::Float(f) => q.bind(*f),
                    SqlValue::Bool(b) => q.bind(if *b { 1i32 } else { 0i32 }),
                    SqlValue::Null => q.bind(None::<String>),
                };
            }
            let row = q.fetch_one(pool).await?;
            let count: i64 = row.try_get(0)?;
            Ok(count)
        }
    }
}

// Note: Repository methods are generated directly on each entity type
// by the GraphQLOperations macro. Example usage:
//
// let movies = MovieEntity::query(&pool)
//     .filter(MovieEntityWhereInput { ... })
//     .fetch_all()
//     .await?;
//
// let movie = MovieEntity::get(&pool, "some-id").await?;
//
// let count = MovieEntity::count_query(&pool)
//     .filter(MovieEntityWhereInput { ... })
//     .execute()
//     .await?;

/// Order direction for sorting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending (A-Z, 0-9, oldest-newest)
    Asc,
    /// Descending (Z-A, 9-0, newest-oldest)
    Desc,
}

/// Helper to add simple order by without constructing the full OrderBy type
pub fn order_by_clause(field: &str, direction: SortDirection) -> String {
    match direction {
        SortDirection::Asc => format!("{} ASC", field),
        SortDirection::Desc => format!("{} DESC", field),
    }
}
