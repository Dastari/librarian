//! SQL Query Builder for GraphQL ORM
//!
//! Provides a type-safe query builder that works with `DatabaseEntity` types
//! and uses parameterized queries via sqlx to prevent SQL injection.

use sqlx::{Row, SqlitePool};

use super::traits::{
    CursorInput, DatabaseEntity, DatabaseFilter, DatabaseOrderBy, FromSqlRow, PageInput, SqlValue,
};
use super::{Connection, Edge, PageInfo, encode_cursor};

/// A query builder for database entities.
///
/// Builds parameterized SQL queries for SELECT operations with
/// filtering, sorting, and pagination support.
pub struct EntityQuery<E: DatabaseEntity> {
    _phantom: std::marker::PhantomData<E>,
    where_clauses: Vec<String>,
    values: Vec<SqlValue>,
    order_by: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    param_counter: usize,
    /// Track order clauses added (for checking if any sorting was applied)
    pub order_clauses: Vec<String>,
}

impl<E: DatabaseEntity + FromSqlRow> EntityQuery<E> {
    /// Create a new query builder for the entity type.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            where_clauses: Vec::new(),
            values: Vec::new(),
            order_by: None,
            limit: None,
            offset: None,
            param_counter: 0,
            order_clauses: Vec::new(),
        }
    }

    /// Add a filter to the query.
    pub fn filter<F: DatabaseFilter>(mut self, filter: &F) -> Self {
        if !filter.is_empty() {
            let (conditions, values) = filter.to_sql_conditions();
            // Rewrite parameter placeholders to use correct indices
            for condition in conditions {
                let rewritten = self.rewrite_params(&condition, values.len());
                self.where_clauses.push(rewritten);
            }
            self.values.extend(values);
        }
        self
    }

    /// Add a raw WHERE clause condition.
    pub fn where_clause(mut self, condition: &str, value: SqlValue) -> Self {
        self.param_counter += 1;
        let rewritten = condition.replace("?", &format!("?{}", self.param_counter));
        self.where_clauses.push(rewritten);
        self.values.push(value);
        self
    }

    /// Add sorting to the query.
    pub fn order_by<O: DatabaseOrderBy>(mut self, order: &O) -> Self {
        if let Some(order_sql) = order.to_sql_order() {
            self.order_clauses.push(order_sql.clone());
            self.order_by = Some(order_sql);
        }
        self
    }

    /// Add default sorting if no order is specified.
    pub fn default_order(mut self) -> Self {
        if self.order_by.is_none() {
            self.order_by = Some(format!("{} {}", E::DEFAULT_SORT, E::DEFAULT_SORT_DIR));
        }
        self
    }

    /// Apply offset-based pagination.
    pub fn paginate(mut self, page: &PageInput) -> Self {
        self.limit = Some(page.limit());
        self.offset = Some(page.offset());
        self
    }

    /// Apply cursor-based pagination.
    pub fn cursor_paginate(mut self, cursor: &CursorInput) -> Result<Self, &'static str> {
        let (offset, limit) = cursor.to_offset_limit()?;
        self.limit = Some(limit);
        self.offset = Some(offset);
        Ok(self)
    }

    /// Set limit directly.
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set offset directly.
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Rewrite parameter placeholders to use sequential indices.
    fn rewrite_params(&mut self, condition: &str, num_new_params: usize) -> String {
        let mut result = condition.to_string();
        // Replace each ? with ?N where N is the parameter index
        for _i in 0..num_new_params {
            self.param_counter += 1;
            // Replace first occurrence of ? with ?N
            if let Some(pos) = result.find('?') {
                // Check if it's not already numbered (e.g., ?1)
                let next_char = result.chars().nth(pos + 1);
                if next_char.is_none() || !next_char.unwrap().is_ascii_digit() {
                    result = format!(
                        "{}?{}{}",
                        &result[..pos],
                        self.param_counter,
                        &result[pos + 1..]
                    );
                }
            }
        }
        result
    }

    /// Build the SQL query string.
    fn build_sql(&self) -> String {
        let mut sql = E::select_sql();

        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }

        if let Some(ref order) = self.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(order);
        }

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            if offset > 0 {
                sql.push_str(&format!(" OFFSET {}", offset));
            }
        }

        sql
    }

    /// Build a COUNT query string.
    fn build_count_sql(&self) -> String {
        let mut sql = format!("SELECT COUNT(*) FROM {}", E::TABLE_NAME);

        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }

        sql
    }

    /// Build a DELETE query string and bind values for bulk delete by filter.
    /// Returns `(sql, values)` so the caller can run it with `execute_with_binds`.
    pub fn build_delete_sql(&self) -> (String, Vec<SqlValue>) {
        let mut sql = format!("DELETE FROM {}", E::TABLE_NAME);
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }
        (sql, self.values.clone())
    }

    /// Execute the query and return all matching entities.
    pub async fn fetch_all(self, pool: &SqlitePool) -> Result<Vec<E>, sqlx::Error> {
        let sql = self.build_sql();
        tracing::debug!(sql = %sql, "Executing entity query");

        let mut query = sqlx::query(&sql);
        for value in &self.values {
            query = value.bind_to_query(query);
        }

        let rows = query.fetch_all(pool).await?;
        rows.iter().map(E::from_row).collect()
    }

    /// Execute the query and return a single entity.
    pub async fn fetch_one(self, pool: &SqlitePool) -> Result<Option<E>, sqlx::Error> {
        let sql = self.build_sql();
        tracing::debug!(sql = %sql, "Executing entity query (one)");

        let mut query = sqlx::query(&sql);
        for value in &self.values {
            query = value.bind_to_query(query);
        }

        match query.fetch_optional(pool).await? {
            Some(row) => Ok(Some(E::from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// Execute a COUNT query.
    pub async fn count(&self, pool: &SqlitePool) -> Result<i64, sqlx::Error> {
        let sql = self.build_count_sql();
        tracing::debug!(sql = %sql, "Executing count query");

        let mut query = sqlx::query_scalar::<_, i64>(&sql);
        for value in &self.values {
            // Re-bind values for count query
            query = match value {
                SqlValue::String(s) => query.bind(s.as_str()),
                SqlValue::Int(i) => query.bind(*i),
                SqlValue::Float(f) => query.bind(*f),
                SqlValue::Bool(b) => query.bind(if *b { 1i32 } else { 0i32 }),
                SqlValue::Null => query.bind(None::<String>),
            };
        }

        query.fetch_one(pool).await
    }

    /// Execute the query and return a Relay-style connection.
    pub async fn fetch_connection(self, pool: &SqlitePool) -> Result<Connection<E>, sqlx::Error> {
        // Get total count first (before limit/offset)
        let total = self.count(pool).await?;

        let offset = self.offset.unwrap_or(0);
        let _limit = self.limit.unwrap_or(25);

        // Fetch the actual items
        let items = self.fetch_all(pool).await?;

        let has_next_page = (offset + items.len() as i64) < total;
        let has_previous_page = offset > 0;

        let edges: Vec<Edge<E>> = items
            .into_iter()
            .enumerate()
            .map(|(i, node)| Edge {
                cursor: encode_cursor(offset + i as i64),
                node,
            })
            .collect();

        let page_info = PageInfo {
            has_next_page,
            has_previous_page,
            start_cursor: edges.first().map(|e| e.cursor.clone()),
            end_cursor: edges.last().map(|e| e.cursor.clone()),
            total_count: Some(total),
        };

        Ok(Connection { edges, page_info })
    }
}

impl<E: DatabaseEntity + FromSqlRow> Default for EntityQuery<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to find a field in the selection set by name.
pub fn has_field<'a>(
    selection: &'a [async_graphql::context::SelectionField<'a>],
    name: &str,
) -> bool {
    selection.iter().any(|f| f.name() == name)
}

/// Helper to get a field's sub-selection from the selection set.
pub fn get_field_selection<'a>(
    selection: &'a [async_graphql::context::SelectionField<'a>],
    name: &str,
) -> Option<Vec<async_graphql::context::SelectionField<'a>>> {
    selection
        .iter()
        .find(|f| f.name() == name)
        .map(|f| f.selection_set().collect())
}

/// Execute an INSERT/UPDATE query with bound values.
/// This helper properly handles the sqlx query lifetime requirements.
pub async fn execute_with_binds(
    sql: &str,
    values: &[SqlValue],
    pool: &SqlitePool,
) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    // We need to build the query and bind all values in one chain
    // to satisfy lifetime requirements

    match values.len() {
        0 => sqlx::query(sql).execute(pool).await,
        1 => {
            let mut q = sqlx::query(sql);
            q = values[0].bind_to_query(q);
            q.execute(pool).await
        }
        _ => {
            // For multiple values, we need to use a different approach
            // Build all binds in a single expression
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
            q.execute(pool).await
        }
    }
}
