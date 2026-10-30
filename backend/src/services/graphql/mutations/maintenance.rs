use async_graphql::{Context, Object, Result, SimpleObject};

use crate::db::Database;
use crate::services::database::reconcile_stored_media_summaries;
use crate::services::graphql::auth::AuthExt;

#[derive(Default)]
pub struct MaintenanceMutations;

#[derive(SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct StoredSummaryReconciliationResult {
    pub dry_run: bool,
    pub artists_examined: u64,
    pub artists_changed: u64,
    pub albums_examined: u64,
    pub albums_changed: u64,
    pub audiobooks_examined: u64,
    pub audiobooks_changed: u64,
}

#[Object]
impl MaintenanceMutations {
    /// Explicit repair of existing stored summaries. Defaults to inspection;
    /// applying a repair is one generated transaction and publishes on commit.
    async fn reconcile_stored_media_summaries(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = true)] dry_run: bool,
    ) -> Result<StoredSummaryReconciliationResult> {
        ctx.require_admin()?;
        let report = reconcile_stored_media_summaries(ctx.data::<Database>()?, dry_run)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        Ok(StoredSummaryReconciliationResult {
            dry_run: report.dry_run,
            artists_examined: report.artists_examined,
            artists_changed: report.artists_changed,
            albums_examined: report.albums_examined,
            albums_changed: report.albums_changed,
            audiobooks_examined: report.audiobooks_examined,
            audiobooks_changed: report.audiobooks_changed,
        })
    }
}
