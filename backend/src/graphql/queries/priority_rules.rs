use super::prelude::*;

#[derive(Default)]
pub struct PriorityRuleQueries;

#[Object]
impl PriorityRuleQueries {
    /// Get all source priority rules for the current user
    async fn source_priority_rules(&self, ctx: &Context<'_>) -> Result<Vec<SourcePriorityRule>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .priority_rules()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(SourcePriorityRule::from).collect())
    }

    /// Get a specific priority rule by scope
    async fn source_priority_rule(
        &self,
        ctx: &Context<'_>,
        library_type: Option<String>,
        library_id: Option<String>,
    ) -> Result<Option<SourcePriorityRule>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let lib_id = library_id
            .as_ref()
            .and_then(|id| Uuid::parse_str(id).ok());

        let record = db
            .priority_rules()
            .get_applicable_rule(user_id, library_type.as_deref(), lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(SourcePriorityRule::from))
    }

    /// Get all available sources for priority configuration
    async fn available_sources(&self, ctx: &Context<'_>) -> Result<Vec<AvailableSource>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let mut sources = Vec::new();

        // Get all indexers
        let indexers = db
            .indexers()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        for idx in indexers {
            // Determine source type based on download_type column (if we had it) or indexer_type
            let source_type = if idx.indexer_type == "newznab" {
                "USENET_INDEXER"
            } else {
                "TORRENT_INDEXER"
            };

            sources.push(AvailableSource {
                source_type: source_type.to_string(),
                id: idx.id.to_string(),
                name: idx.name,
                enabled: idx.enabled,
                is_healthy: idx.error_count == 0,
            });
        }

        // TODO: Add usenet indexers when they have separate tracking

        Ok(sources)
    }
}
