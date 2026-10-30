//! Backup repository state is external to generated domain entities. Expose a
//! revision signal only; clients refresh the administrator-authorized queries.
use std::pin::Pin;
use std::sync::Arc;

use async_graphql::{Context, Result, Subscription};
use futures::Stream;

use crate::services::graphql::auth::AuthExt;
use crate::services::manager::ServicesManager;

#[derive(Default)]
pub struct BackupSubscriptions;

#[Subscription]
impl BackupSubscriptions {
    async fn backup_changed(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Pin<Box<dyn Stream<Item = String> + Send>>> {
        ctx.require_admin()?;
        let backup = ctx
            .data::<Arc<ServicesManager>>()?
            .get_backup()
            .await
            .ok_or_else(|| async_graphql::Error::new("Backup service unavailable"))?;
        let receiver = backup.subscribe_changes();
        // Initial reconciliation closes the query/subscription admission race.
        // watch retains the newest revision when a consumer falls behind.
        Ok(Box::pin(futures::stream::unfold(
            (receiver, true),
            |(mut receiver, first)| async move {
                if !first && receiver.changed().await.is_err() {
                    return None;
                }
                let revision = receiver.borrow_and_update().to_string();
                Some((revision, (receiver, false)))
            },
        )))
    }
}
