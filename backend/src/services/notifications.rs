//! Notification service for user alerts and action requests
//!
//! This service manages creating, broadcasting, and resolving user notifications.
//! It provides real-time updates via GraphQL subscriptions.

use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::{
    ActionType, CreateNotification, Database, NotificationCategory, NotificationFilter,
    NotificationRecord, NotificationType, PaginatedNotifications, Resolution,
};

/// Event broadcast when a notification is created or updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
    pub notification: NotificationRecord,
    pub event_type: NotificationEventType,
}

/// Type of notification event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEventType {
    Created,
    Read,
    Resolved,
    Deleted,
}

/// Count update event for badge display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationCountEvent {
    pub user_id: Uuid,
    pub unread_count: i64,
    pub action_required_count: i64,
}

/// Notification service configuration
#[derive(Debug, Clone)]
pub struct NotificationServiceConfig {
    /// Broadcast channel capacity
    pub channel_capacity: usize,
}

impl Default for NotificationServiceConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 256,
        }
    }
}

/// Service for managing user notifications
pub struct NotificationService {
    db: Database,
    /// Broadcast channel for notification events
    event_tx: broadcast::Sender<NotificationEvent>,
    /// Broadcast channel for count updates
    count_tx: broadcast::Sender<NotificationCountEvent>,
}

impl NotificationService {
    /// Create a new notification service
    pub fn new(db: Database, config: NotificationServiceConfig) -> Self {
        let (event_tx, _) = broadcast::channel(config.channel_capacity);
        let (count_tx, _) = broadcast::channel(config.channel_capacity);

        Self {
            db,
            event_tx,
            count_tx,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(db: Database) -> Self {
        Self::new(db, NotificationServiceConfig::default())
    }

    /// Subscribe to notification events
    pub fn subscribe(&self) -> broadcast::Receiver<NotificationEvent> {
        self.event_tx.subscribe()
    }

    /// Subscribe to count update events
    pub fn subscribe_counts(&self) -> broadcast::Receiver<NotificationCountEvent> {
        self.count_tx.subscribe()
    }

    /// Create a new notification
    pub async fn create(
        &self,
        user_id: Uuid,
        title: String,
        message: String,
        notification_type: NotificationType,
        category: NotificationCategory,
    ) -> Result<NotificationRecord> {
        self.create_full(CreateNotification {
            user_id,
            title,
            message,
            notification_type,
            category,
            library_id: None,
            torrent_id: None,
            media_file_id: None,
            pending_match_id: None,
            action_type: None,
            action_data: None,
        })
        .await
    }

    /// Create a notification with all options
    pub async fn create_full(&self, notification: CreateNotification) -> Result<NotificationRecord> {
        let user_id = notification.user_id;
        let title = notification.title.clone();
        let category = notification.category;

        // Check for duplicate notification
        if self
            .db
            .notifications()
            .exists_similar(
                user_id,
                notification.category,
                notification.library_id,
                notification.torrent_id,
                notification.media_file_id,
            )
            .await?
        {
            info!(
                user_id = %user_id,
                category = ?category,
                title = %title,
                "Returning existing notification (duplicate prevention)"
            );
            // Return the existing record (or just skip)
            let existing = self
                .db
                .notifications()
                .list(
                    user_id,
                    NotificationFilter {
                        category: Some(notification.category),
                        unresolved_only: true,
                        ..Default::default()
                    },
                    1,
                    0,
                )
                .await?;
            if let Some(record) = existing.notifications.into_iter().next() {
                return Ok(record);
            }
        }

        let record = self.db.notifications().create(notification).await?;

        info!(
            "Created notification for user {}: {}",
            user_id, record.title
        );

        // Broadcast the event
        let _ = self.event_tx.send(NotificationEvent {
            notification: record.clone(),
            event_type: NotificationEventType::Created,
        });

        // Broadcast updated count
        self.broadcast_count_update(user_id).await;

        Ok(record)
    }

    /// Create an action-required notification
    pub async fn create_action_required(
        &self,
        user_id: Uuid,
        title: String,
        message: String,
        category: NotificationCategory,
        action_type: ActionType,
        action_data: Option<JsonValue>,
        library_id: Option<Uuid>,
        torrent_id: Option<Uuid>,
        media_file_id: Option<Uuid>,
        pending_match_id: Option<Uuid>,
    ) -> Result<NotificationRecord> {
        self.create_full(CreateNotification {
            user_id,
            title,
            message,
            notification_type: NotificationType::ActionRequired,
            category,
            library_id,
            torrent_id,
            media_file_id,
            pending_match_id,
            action_type: Some(action_type),
            action_data,
        })
        .await
    }

    /// Create a warning notification
    pub async fn create_warning(
        &self,
        user_id: Uuid,
        title: String,
        message: String,
        category: NotificationCategory,
    ) -> Result<NotificationRecord> {
        self.create(user_id, title, message, NotificationType::Warning, category)
            .await
    }

    /// Create an error notification
    pub async fn create_error(
        &self,
        user_id: Uuid,
        title: String,
        message: String,
        category: NotificationCategory,
    ) -> Result<NotificationRecord> {
        self.create(user_id, title, message, NotificationType::Error, category)
            .await
    }

    /// Create an info notification
    pub async fn create_info(
        &self,
        user_id: Uuid,
        title: String,
        message: String,
        category: NotificationCategory,
    ) -> Result<NotificationRecord> {
        self.create(user_id, title, message, NotificationType::Info, category)
            .await
    }

    /// Create a system-wide notification for all users
    ///
    /// Used for configuration issues, system warnings, etc.
    pub async fn create_system_warning(
        &self,
        title: String,
        message: String,
        category: NotificationCategory,
    ) -> Result<Vec<NotificationRecord>> {
        let users = self.db.users().list_all().await?;
        let mut records = Vec::new();

        for user in users {
            let user_id = match Uuid::parse_str(&user.id) {
                Ok(id) => id,
                Err(e) => {
                    warn!("Invalid user ID {}: {}", user.id, e);
                    continue;
                }
            };
            match self
                .create_warning(user_id, title.clone(), message.clone(), category)
                .await
            {
                Ok(record) => records.push(record),
                Err(e) => {
                    warn!(
                        "Failed to create system notification for user {}: {}",
                        user.id, e
                    );
                }
            }
        }

        if records.is_empty() {
            debug!("No users found to send system notification to");
        } else {
            info!(
                "Created system notification '{}' for {} users",
                title,
                records.len()
            );
        }

        Ok(records)
    }

    /// Create a system-wide action-required notification for all users
    pub async fn create_system_action_required(
        &self,
        title: String,
        message: String,
        category: NotificationCategory,
        action_type: ActionType,
        action_data: Option<JsonValue>,
    ) -> Result<Vec<NotificationRecord>> {
        let users = self.db.users().list_all().await?;
        let mut records = Vec::new();

        for user in users {
            let user_id = match Uuid::parse_str(&user.id) {
                Ok(id) => id,
                Err(e) => {
                    warn!("Invalid user ID {}: {}", user.id, e);
                    continue;
                }
            };
            match self
                .create_action_required(
                    user_id,
                    title.clone(),
                    message.clone(),
                    category,
                    action_type,
                    action_data.clone(),
                    None,
                    None,
                    None,
                    None,
                )
                .await
            {
                Ok(record) => records.push(record),
                Err(e) => {
                    warn!(
                        "Failed to create system notification for user {}: {}",
                        user.id, e
                    );
                }
            }
        }

        Ok(records)
    }

    /// Get a notification by ID
    pub async fn get(&self, id: Uuid) -> Result<Option<NotificationRecord>> {
        self.db.notifications().get_by_id(id).await
    }

    /// Get unread count for a user
    pub async fn get_unread_count(&self, user_id: Uuid) -> Result<i64> {
        self.db.notifications().get_unread_count(user_id).await
    }

    /// Get action-required count for a user
    pub async fn get_action_required_count(&self, user_id: Uuid) -> Result<i64> {
        self.db
            .notifications()
            .get_action_required_count(user_id)
            .await
    }

    /// List notifications with filtering and pagination
    pub async fn list(
        &self,
        user_id: Uuid,
        filter: NotificationFilter,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedNotifications> {
        self.db
            .notifications()
            .list(user_id, filter, limit, offset)
            .await
    }

    /// Get recent notifications for popover display
    pub async fn get_recent(&self, user_id: Uuid, limit: i64) -> Result<Vec<NotificationRecord>> {
        self.db.notifications().get_recent(user_id, limit).await
    }

    /// Get recent unread notifications (for navbar popover)
    pub async fn get_recent_unread(&self, user_id: Uuid, limit: i64) -> Result<Vec<NotificationRecord>> {
        self.db.notifications().get_recent_unread(user_id, limit).await
    }

    /// Mark a notification as read
    pub async fn mark_read(&self, id: Uuid) -> Result<Option<NotificationRecord>> {
        let record = self.db.notifications().mark_read(id).await?;

        if let Some(ref record) = record {
            let _ = self.event_tx.send(NotificationEvent {
                notification: record.clone(),
                event_type: NotificationEventType::Read,
            });

            self.broadcast_count_update(record.user_id).await;
        }

        Ok(record)
    }

    /// Mark all notifications as read for a user
    pub async fn mark_all_read(&self, user_id: Uuid) -> Result<i64> {
        let count = self.db.notifications().mark_all_read(user_id).await?;

        if count > 0 {
            self.broadcast_count_update(user_id).await;
        }

        Ok(count)
    }

    /// Resolve a notification
    pub async fn resolve(
        &self,
        id: Uuid,
        resolution: Resolution,
    ) -> Result<Option<NotificationRecord>> {
        let record = self.db.notifications().resolve(id, resolution).await?;

        if let Some(ref record) = record {
            info!(
                "Resolved notification {} with {:?}",
                id,
                resolution
            );

            let _ = self.event_tx.send(NotificationEvent {
                notification: record.clone(),
                event_type: NotificationEventType::Resolved,
            });

            self.broadcast_count_update(record.user_id).await;
        }

        Ok(record)
    }

    /// Delete a notification
    pub async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<bool> {
        let deleted = self.db.notifications().delete(id).await?;

        if deleted {
            self.broadcast_count_update(user_id).await;
        }

        Ok(deleted)
    }

    /// Broadcast count update to subscribers
    async fn broadcast_count_update(&self, user_id: Uuid) {
        let unread_count = self
            .db
            .notifications()
            .get_unread_count(user_id)
            .await
            .unwrap_or(0);

        let action_required_count = self
            .db
            .notifications()
            .get_action_required_count(user_id)
            .await
            .unwrap_or(0);

        let _ = self.count_tx.send(NotificationCountEvent {
            user_id,
            unread_count,
            action_required_count,
        });
    }

    /// Auto-resolve a notification (used when the underlying issue is fixed)
    pub async fn auto_resolve(&self, id: Uuid) -> Result<Option<NotificationRecord>> {
        self.resolve(id, Resolution::AutoResolved).await
    }

    /// Auto-resolve notifications matching criteria
    pub async fn auto_resolve_for_media_file(&self, media_file_id: Uuid) -> Result<i64> {
        // This is a simplified version - in production you might want a more specific query
        let notifications = self
            .db
            .notifications()
            .list(
                Uuid::nil(), // Would need to know user_id or query differently
                NotificationFilter {
                    unresolved_only: true,
                    ..Default::default()
                },
                100,
                0,
            )
            .await?;

        let mut resolved_count = 0;
        for notif in notifications.notifications {
            if notif.media_file_id == Some(media_file_id) && notif.resolved_at.is_none() {
                if self.auto_resolve(notif.id).await?.is_some() {
                    resolved_count += 1;
                }
            }
        }

        Ok(resolved_count)
    }
}

/// Helper trait for creating notification service
impl Clone for NotificationService {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            event_tx: self.event_tx.clone(),
            count_tx: self.count_tx.clone(),
        }
    }
}

/// Create a notification service with default config
pub fn create_notification_service(db: Database) -> Arc<NotificationService> {
    Arc::new(NotificationService::with_defaults(db))
}
