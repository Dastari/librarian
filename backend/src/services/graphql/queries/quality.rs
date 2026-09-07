//! `pendingUpgrades` — Q38 UI query: unresolved quality-upgrade
//! notifications (created by
//! `LibraryScanService::maybe_notify_quality_upgrade`, wired from Q44's
//! duplicate-torrent handling), optionally scoped to a library. The frontend
//! resolves these via the generic `updateNotification` mutation (dismiss) or
//! the custom `approveQualityUpgrade` mutation (approve + replace file).

use std::sync::Arc;

use async_graphql::{Context, Object, Result};
use graphql_orm::graphql::filters::{DateFilter, StringFilter};

use crate::graphql::entities::{Notification, NotificationWhereInput};
use crate::services::ServicesManager;
use crate::services::graphql::auth::AuthExt;
use crate::services::graphql::mutations::quality::QUALITY_UPGRADE_ACTION_TYPE;

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

#[derive(Default)]
pub struct QualityQueries;

#[Object]
impl QualityQueries {
    /// Unresolved quality-upgrade notifications, optionally scoped to a
    /// library.
    #[graphql(name = "pendingUpgrades")]
    async fn pending_upgrades(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "libraryId")] library_id: Option<String>,
    ) -> Result<Vec<Notification>> {
        ctx.require_member()?;
        let manager = ctx.data_unchecked::<Arc<ServicesManager>>();
        let Some(db_service) = manager.get_database().await else {
            return Ok(vec![]);
        };
        let db = db_service.pool().clone();

        let mut filter = NotificationWhereInput {
            action_type: Some(string_eq(QUALITY_UPGRADE_ACTION_TYPE)),
            resolved_at: Some(DateFilter {
                is_null: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        if let Some(library_id) = &library_id {
            filter.library_id = Some(string_eq(library_id));
        }

        let notifications = Notification::query(db.pool())
            .filter(filter)
            .fetch_all()
            .await?;
        Ok(notifications)
    }
}
