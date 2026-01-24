use super::prelude::*;
use crate::graphql::types::PaginatedNotifications;
use crate::services::NotificationService;

#[derive(Default)]
pub struct NotificationQueries;

#[Object]
impl NotificationQueries {
    /// Get notifications with optional filtering and pagination
    async fn notifications(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Filter options")] filter: Option<NotificationFilterInput>,
        #[graphql(default = 50, desc = "Number of notifications to return")] limit: i32,
        #[graphql(default = 0, desc = "Offset for pagination")] offset: i32,
    ) -> Result<PaginatedNotifications> {
        let user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<NotificationService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let db_filter = filter.map(|f| crate::db::NotificationFilter {
            unread_only: f.unread_only.unwrap_or(false),
            unresolved_only: f.unresolved_only.unwrap_or(false),
            category: f.category.map(|c| c.into()),
            notification_type: f.notification_type.map(|t| t.into()),
        }).unwrap_or_default();

        let result = service
            .list(user_id, db_filter, limit as i64, offset as i64)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(PaginatedNotifications {
            notifications: result.notifications.into_iter().map(Notification::from).collect(),
            total_count: result.total_count,
            has_more: result.has_more,
        })
    }

    /// Get a single notification by ID
    async fn notification(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Notification ID")] id: String,
    ) -> Result<Option<Notification>> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<NotificationService>>();

        let uuid = Uuid::parse_str(&id)
            .map_err(|_| async_graphql::Error::new("Invalid notification ID"))?;

        let notification = service
            .get(uuid)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(notification.map(Notification::from))
    }

    /// Get recent notifications for popover display
    ///
    /// When `unreadOnly` is true, only unread notifications are returned (for navbar badge).
    async fn recent_notifications(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 10, desc = "Number of recent notifications")] limit: i32,
        #[graphql(default = false, desc = "Only return unread notifications")] unread_only: bool,
    ) -> Result<Vec<Notification>> {
        let user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<NotificationService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let notifications = if unread_only {
            service
                .get_recent_unread(user_id, limit as i64)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
        } else {
            service
                .get_recent(user_id, limit as i64)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
        };

        Ok(notifications.into_iter().map(Notification::from).collect())
    }

    /// Get unread notification count for badge display
    async fn unread_notification_count(&self, ctx: &Context<'_>) -> Result<i64> {
        let user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<NotificationService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let count = service
            .get_unread_count(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(count)
    }

    /// Get notification counts for badge display
    async fn notification_counts(&self, ctx: &Context<'_>) -> Result<NotificationCounts> {
        let user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<NotificationService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let unread_count = service
            .get_unread_count(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let action_required_count = service
            .get_action_required_count(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(NotificationCounts {
            unread_count,
            action_required_count,
        })
    }
}
