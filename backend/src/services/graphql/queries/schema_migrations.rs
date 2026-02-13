//! GraphQL query for in-code schema migration history.

use async_graphql::{Context, Object, Result, SimpleObject};
use sqlx::FromRow;

use crate::db::Database;
use crate::services::graphql::auth::{AuthExt, AuthGuard};

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "SchemaMigrationEntry")]
pub struct SchemaMigrationEntry {
    #[graphql(name = "Id")]
    pub id: String,
    #[graphql(name = "Description")]
    pub description: Option<String>,
    #[graphql(name = "AppliedAt")]
    pub applied_at: String,
}

#[derive(Debug, FromRow)]
struct SchemaMigrationRow {
    id: String,
    description: Option<String>,
    applied_at: String,
}

#[derive(Default)]
pub struct SchemaMigrationsQueries;

#[Object]
impl SchemaMigrationsQueries {
    /// List in-code schema migrations applied by backend schema sync.
    #[graphql(name = "SchemaMigrations", guard = "AuthGuard")]
    async fn schema_migrations(&self, ctx: &Context<'_>) -> Result<Vec<SchemaMigrationEntry>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data::<Database>()?;

        let rows: Vec<SchemaMigrationRow> = sqlx::query_as(
            "SELECT id, description, applied_at FROM schema_migrations ORDER BY applied_at DESC, id DESC",
        )
        .fetch_all(db)
        .await
        .map_err(|e| async_graphql::Error::new(format!("Failed to load schema migrations: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|row| SchemaMigrationEntry {
                id: row.id,
                description: row.description,
                applied_at: row.applied_at,
            })
            .collect())
    }
}
