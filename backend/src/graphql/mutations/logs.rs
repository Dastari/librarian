use super::prelude::*;

#[derive(Default)]
pub struct LogMutations;

#[Object]
impl LogMutations {
    /// Clear all logs
    async fn clear_all_logs(&self, ctx: &Context<'_>) -> Result<ClearLogsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let deleted = db
            .logs()
            .delete_all()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ClearLogsResult {
            success: true,
            deleted_count: deleted as i64,
            error: None,
        })
    }

    /// Clear logs older than a specified number of days
    async fn clear_old_logs(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Delete logs older than this many days")] days: i32,
    ) -> Result<ClearLogsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let before = time::OffsetDateTime::now_utc() - time::Duration::days(days as i64);

        let deleted = db
            .logs()
            .delete_before(before)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ClearLogsResult {
            success: true,
            deleted_count: deleted as i64,
            error: None,
        })
    }
}
