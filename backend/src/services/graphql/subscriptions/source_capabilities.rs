use async_graphql::{Context, Result, Subscription};
use futures::{Stream, StreamExt};
use graphql_orm::graphql::subscriptions::SubscriptionEvent;
use std::pin::Pin;

use crate::db::Database;
use crate::graphql::entities::SourceChangedEvent;
use crate::services::graphql::auth::AuthExt;
use crate::services::graphql::queries::source_capabilities::has_enabled_sources;

#[derive(Default)]
pub struct SourceCapabilitySubscriptions;

#[Subscription]
impl SourceCapabilitySubscriptions {
    /// Member-safe enabled-state invalidation; never releases source IDs,
    /// names, settings, credentials or raw entity change witnesses.
    async fn enabled_sources_changed(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<bool>> + Send>>> {
        ctx.require_member()?;
        let db = ctx.data::<Database>()?.clone();
        let receiver = db
            .ensure_event_sender::<SubscriptionEvent<SourceChangedEvent>>()
            .subscribe();
        Ok(futures::stream::unfold(
            (receiver, db, true, None),
            |(mut receiver, db, first, mut previous)| async move {
                let mut force = first;
                loop {
                    if !force {
                        match receiver.recv().await {
                            Ok(_) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                force = true
                            }
                            Err(_) => return None,
                        }
                    }
                    let value = has_enabled_sources(&db).await;
                    if let Ok(current) = value.as_ref() {
                        if !force && previous == Some(*current) {
                            continue;
                        }
                        previous = Some(*current);
                    }
                    return Some((value, (receiver, db, false, previous)));
                }
            },
        )
        .boxed())
    }
}
