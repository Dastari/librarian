use async_graphql::{Context, Object, Result, SimpleObject};
use graphql_orm::graphql::filters::{DateFilter, StringFilter};

use crate::db::Database;
use crate::graphql::entities::{
    LibraryScanIssue, LibraryScanIssueWhereInput, Notification, NotificationWhereInput,
    UpdateLibraryScanIssueInput, UpdateNotificationInput,
};
use crate::services::graphql::auth::AuthExt;

#[derive(SimpleObject)]
pub struct MarkAllNotificationsReadResult {
    pub notification_count: i64,
    pub scan_issue_count: i64,
}

#[derive(Default)]
pub struct NotificationFeedMutations;

#[Object]
impl NotificationFeedMutations {
    /// Acknowledge the caller's complete feed, including scan issues on other pages.
    async fn mark_all_notifications_read(
        &self,
        ctx: &Context<'_>,
    ) -> Result<MarkAllNotificationsReadResult> {
        let user = ctx.require_member()?;
        let db = ctx.data::<Database>()?;
        let owner = Some(StringFilter {
            eq: Some(user.user_id.clone()),
            ..Default::default()
        });
        let unread = Some(DateFilter {
            is_null: Some(true),
            ..Default::default()
        });
        let now = chrono::Utc::now().to_rfc3339();

        // Trusted generated entity operations, explicitly scoped to the authenticated
        // owner. Members may acknowledge their own issues, but the generic issue
        // write policy remains admin-only. No caller-supplied owner or update fields.
        let notification_count = Notification::update_where(
            db,
            NotificationWhereInput {
                user_id: owner.clone(),
                read_at: unread.clone(),
                ..Default::default()
            },
            UpdateNotificationInput {
                read_at: Some(Some(now.clone())),
                ..Default::default()
            },
        )
        .await?;
        let scan_issue_count = LibraryScanIssue::update_where(
            db,
            LibraryScanIssueWhereInput {
                user_id: owner,
                read_at: unread,
                ..Default::default()
            },
            UpdateLibraryScanIssueInput {
                read_at: Some(Some(now)),
                ..Default::default()
            },
        )
        .await?;
        Ok(MarkAllNotificationsReadResult {
            notification_count,
            scan_issue_count,
        })
    }
}
