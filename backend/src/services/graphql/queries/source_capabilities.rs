//! Deliberate public capability projection. Members can search enabled sources
//! without receiving the admin-only Source configuration or credentials.
use async_graphql::{Context, Object, Result};
use graphql_orm::graphql::filters::BoolFilter;

use crate::db::Database;
use crate::graphql::entities::{Source, SourceWhereInput};
use crate::services::graphql::auth::AuthExt;

pub(crate) async fn has_enabled_sources(db: &Database) -> Result<bool> {
    Source::query(db.pool())
        .filter(SourceWhereInput {
            enabled: Some(BoolFilter {
                eq: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .exists()
        .await
        .map_err(|error| async_graphql::Error::new(error.to_string()))
}

#[derive(Default)]
pub struct SourceCapabilityQueries;

#[Object]
impl SourceCapabilityQueries {
    /// Whether any source is enabled; does not imply provider connectivity.
    async fn has_enabled_sources(&self, ctx: &Context<'_>) -> Result<bool> {
        ctx.require_member()?;
        has_enabled_sources(ctx.data::<Database>()?).await
    }
}
