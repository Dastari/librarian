use super::prelude::*;
use crate::db::priority_rules::{CreatePriorityRule, SourceRef, SourceType};

#[derive(Default)]
pub struct PriorityRuleMutations;

#[Object]
impl PriorityRuleMutations {
    /// Set a source priority rule (creates or updates)
    async fn set_source_priority_rule(
        &self,
        ctx: &Context<'_>,
        input: SetPriorityRuleInput,
    ) -> Result<PriorityRuleResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        // Parse library_id if provided
        let library_id = input
            .library_id
            .as_ref()
            .and_then(|id| Uuid::parse_str(id).ok());

        // Convert priority order from input to internal format
        let priority_order: Vec<SourceRef> = input
            .priority_order
            .iter()
            .filter_map(|ref_input| {
                let source_type = match ref_input.source_type.to_uppercase().as_str() {
                    "TORRENT_INDEXER" => SourceType::TorrentIndexer,
                    "USENET_INDEXER" => SourceType::UsenetIndexer,
                    _ => return None,
                };
                Some(SourceRef {
                    source_type,
                    id: ref_input.id.clone(),
                })
            })
            .collect();

        // Upsert the rule
        let record = db
            .priority_rules()
            .upsert(CreatePriorityRule {
                user_id,
                library_type: input.library_type.clone(),
                library_id,
                priority_order,
                search_all_sources: input.search_all_sources.unwrap_or(false),
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(PriorityRuleResult {
            success: true,
            error: None,
            rule: Some(SourcePriorityRule::from(record)),
        })
    }

    /// Delete a source priority rule
    async fn delete_source_priority_rule(
        &self,
        ctx: &Context<'_>,
        library_type: Option<String>,
        library_id: Option<String>,
    ) -> Result<MutationResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let lib_id = library_id
            .as_ref()
            .and_then(|id| Uuid::parse_str(id).ok());

        let deleted = db
            .priority_rules()
            .delete_by_scope(user_id, library_type.as_deref(), lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if deleted {
            Ok(MutationResult {
                success: true,
                error: None,
            })
        } else {
            Ok(MutationResult {
                success: false,
                error: Some("Rule not found".to_string()),
            })
        }
    }
}
