use super::prelude::*;
use crate::services::NotificationService;

#[derive(Default)]
pub struct NotificationMutations;

#[Object]
impl NotificationMutations {
    /// Mark a notification as read
    async fn mark_notification_read(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Notification ID")] id: String,
    ) -> Result<NotificationResult> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<NotificationService>>();

        let uuid = match Uuid::parse_str(&id) {
            Ok(id) => id,
            Err(_) => {
                return Ok(NotificationResult {
                    success: false,
                    error: Some("Invalid notification ID".to_string()),
                    notification: None,
                })
            }
        };

        match service.mark_read(uuid).await {
            Ok(Some(notification)) => Ok(NotificationResult {
                success: true,
                error: None,
                notification: Some(Notification::from(notification)),
            }),
            Ok(None) => Ok(NotificationResult {
                success: false,
                error: Some("Notification not found".to_string()),
                notification: None,
            }),
            Err(e) => Ok(NotificationResult {
                success: false,
                error: Some(e.to_string()),
                notification: None,
            }),
        }
    }

    /// Mark all notifications as read
    async fn mark_all_notifications_read(&self, ctx: &Context<'_>) -> Result<MarkAllReadResult> {
        let user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<NotificationService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match service.mark_all_read(user_id).await {
            Ok(count) => Ok(MarkAllReadResult {
                success: true,
                count,
                error: None,
            }),
            Err(e) => Ok(MarkAllReadResult {
                success: false,
                count: 0,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Resolve a notification with an action
    async fn resolve_notification(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Resolution input")] input: ResolveNotificationInput,
    ) -> Result<NotificationResult> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<NotificationService>>();

        let uuid = match Uuid::parse_str(&input.id) {
            Ok(id) => id,
            Err(_) => {
                return Ok(NotificationResult {
                    success: false,
                    error: Some("Invalid notification ID".to_string()),
                    notification: None,
                })
            }
        };

        let resolution: crate::db::Resolution = input.resolution.into();

        match service.resolve(uuid, resolution).await {
            Ok(Some(notification)) => Ok(NotificationResult {
                success: true,
                error: None,
                notification: Some(Notification::from(notification)),
            }),
            Ok(None) => Ok(NotificationResult {
                success: false,
                error: Some("Notification not found or already resolved".to_string()),
                notification: None,
            }),
            Err(e) => Ok(NotificationResult {
                success: false,
                error: Some(e.to_string()),
                notification: None,
            }),
        }
    }

    /// Delete a notification
    async fn delete_notification(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Notification ID")] id: String,
    ) -> Result<MutationResult> {
        let user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<NotificationService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let uuid = match Uuid::parse_str(&id) {
            Ok(id) => id,
            Err(_) => {
                return Ok(MutationResult {
                    success: false,
                    error: Some("Invalid notification ID".to_string()),
                })
            }
        };

        match service.delete(uuid, user_id).await {
            Ok(true) => Ok(MutationResult {
                success: true,
                error: None,
            }),
            Ok(false) => Ok(MutationResult {
                success: false,
                error: Some("Notification not found".to_string()),
            }),
            Err(e) => Ok(MutationResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }
}
