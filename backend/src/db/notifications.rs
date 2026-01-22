//! User notifications database operations

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// A notification record in the database
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct NotificationRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub category: String,
    pub library_id: Option<Uuid>,
    pub torrent_id: Option<Uuid>,
    pub media_file_id: Option<Uuid>,
    pub pending_match_id: Option<Uuid>,
    pub action_type: Option<String>,
    pub action_data: Option<JsonValue>,
    pub read_at: Option<OffsetDateTime>,
    pub resolved_at: Option<OffsetDateTime>,
    pub resolution: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Input for creating a new notification
#[derive(Debug, Clone)]
pub struct CreateNotification {
    pub user_id: Uuid,
    pub title: String,
    pub message: String,
    pub notification_type: NotificationType,
    pub category: NotificationCategory,
    pub library_id: Option<Uuid>,
    pub torrent_id: Option<Uuid>,
    pub media_file_id: Option<Uuid>,
    pub pending_match_id: Option<Uuid>,
    pub action_type: Option<ActionType>,
    pub action_data: Option<JsonValue>,
}

/// Notification type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    Info,
    Warning,
    Error,
    ActionRequired,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationType::Info => "info",
            NotificationType::Warning => "warning",
            NotificationType::Error => "error",
            NotificationType::ActionRequired => "action_required",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "info" => Some(NotificationType::Info),
            "warning" => Some(NotificationType::Warning),
            "error" => Some(NotificationType::Error),
            "action_required" => Some(NotificationType::ActionRequired),
            _ => None,
        }
    }
}

/// Notification category enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    Matching,
    Processing,
    Quality,
    Storage,
    Extraction,
}

impl NotificationCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationCategory::Matching => "matching",
            NotificationCategory::Processing => "processing",
            NotificationCategory::Quality => "quality",
            NotificationCategory::Storage => "storage",
            NotificationCategory::Extraction => "extraction",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "matching" => Some(NotificationCategory::Matching),
            "processing" => Some(NotificationCategory::Processing),
            "quality" => Some(NotificationCategory::Quality),
            "storage" => Some(NotificationCategory::Storage),
            "extraction" => Some(NotificationCategory::Extraction),
            _ => None,
        }
    }
}

/// Action type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    ConfirmUpgrade,
    ManualMatch,
    Retry,
    Dismiss,
    Review,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::ConfirmUpgrade => "confirm_upgrade",
            ActionType::ManualMatch => "manual_match",
            ActionType::Retry => "retry",
            ActionType::Dismiss => "dismiss",
            ActionType::Review => "review",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "confirm_upgrade" => Some(ActionType::ConfirmUpgrade),
            "manual_match" => Some(ActionType::ManualMatch),
            "retry" => Some(ActionType::Retry),
            "dismiss" => Some(ActionType::Dismiss),
            "review" => Some(ActionType::Review),
            _ => None,
        }
    }
}

/// Resolution type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    Accepted,
    Rejected,
    Dismissed,
    AutoResolved,
}

impl Resolution {
    pub fn as_str(&self) -> &'static str {
        match self {
            Resolution::Accepted => "accepted",
            Resolution::Rejected => "rejected",
            Resolution::Dismissed => "dismissed",
            Resolution::AutoResolved => "auto_resolved",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "accepted" => Some(Resolution::Accepted),
            "rejected" => Some(Resolution::Rejected),
            "dismissed" => Some(Resolution::Dismissed),
            "auto_resolved" => Some(Resolution::AutoResolved),
            _ => None,
        }
    }
}

/// Filter options for querying notifications
#[derive(Debug, Clone, Default)]
pub struct NotificationFilter {
    pub unread_only: bool,
    pub unresolved_only: bool,
    pub category: Option<NotificationCategory>,
    pub notification_type: Option<NotificationType>,
}

/// Result for paginated notification queries
#[derive(Debug, Clone)]
pub struct PaginatedNotifications {
    pub notifications: Vec<NotificationRecord>,
    pub total_count: i64,
    pub has_more: bool,
}

/// Notifications repository for database operations
pub struct NotificationRepository {
    pool: PgPool,
}

impl NotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new notification
    pub async fn create(&self, notification: CreateNotification) -> Result<NotificationRecord> {
        let record = sqlx::query_as::<_, NotificationRecord>(
            r#"
            INSERT INTO notifications (
                user_id, title, message, notification_type, category,
                library_id, torrent_id, media_file_id, pending_match_id,
                action_type, action_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(notification.user_id)
        .bind(&notification.title)
        .bind(&notification.message)
        .bind(notification.notification_type.as_str())
        .bind(notification.category.as_str())
        .bind(notification.library_id)
        .bind(notification.torrent_id)
        .bind(notification.media_file_id)
        .bind(notification.pending_match_id)
        .bind(notification.action_type.map(|a| a.as_str()))
        .bind(&notification.action_data)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a notification by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<NotificationRecord>> {
        let record = sqlx::query_as::<_, NotificationRecord>(
            "SELECT * FROM notifications WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get unread notification count for a user
    pub async fn get_unread_count(&self, user_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND read_at IS NULL",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Get unresolved action-required count for a user
    pub async fn get_action_required_count(&self, user_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM notifications 
            WHERE user_id = $1 
            AND notification_type = 'action_required' 
            AND resolved_at IS NULL
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// List notifications with filtering and pagination
    pub async fn list(
        &self,
        user_id: Uuid,
        filter: NotificationFilter,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedNotifications> {
        let mut conditions = vec!["user_id = $1".to_string()];
        let mut param_count = 1;

        if filter.unread_only {
            conditions.push("read_at IS NULL".to_string());
        }

        if filter.unresolved_only {
            conditions.push("resolved_at IS NULL".to_string());
        }

        if filter.category.is_some() {
            param_count += 1;
            conditions.push(format!("category = ${}", param_count));
        }

        if filter.notification_type.is_some() {
            param_count += 1;
            conditions.push(format!("notification_type = ${}", param_count));
        }

        let where_clause = conditions.join(" AND ");

        // Count query
        let count_sql = format!("SELECT COUNT(*) FROM notifications WHERE {}", where_clause);

        // Data query
        let data_sql = format!(
            r#"
            SELECT * FROM notifications 
            WHERE {} 
            ORDER BY created_at DESC 
            LIMIT ${} OFFSET ${}
            "#,
            where_clause,
            param_count + 1,
            param_count + 2
        );

        // Execute count query
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql).bind(user_id);

        if let Some(ref cat) = filter.category {
            count_query = count_query.bind(cat.as_str());
        }
        if let Some(ref ntype) = filter.notification_type {
            count_query = count_query.bind(ntype.as_str());
        }

        let total_count = count_query.fetch_one(&self.pool).await?;

        // Execute data query
        let mut data_query = sqlx::query_as::<_, NotificationRecord>(&data_sql).bind(user_id);

        if let Some(ref cat) = filter.category {
            data_query = data_query.bind(cat.as_str());
        }
        if let Some(ref ntype) = filter.notification_type {
            data_query = data_query.bind(ntype.as_str());
        }

        data_query = data_query.bind(limit).bind(offset);

        let notifications = data_query.fetch_all(&self.pool).await?;
        let has_more = (offset + notifications.len() as i64) < total_count;

        Ok(PaginatedNotifications {
            notifications,
            total_count,
            has_more,
        })
    }

    /// Get recent notifications for popover display
    pub async fn get_recent(&self, user_id: Uuid, limit: i64) -> Result<Vec<NotificationRecord>> {
        let records = sqlx::query_as::<_, NotificationRecord>(
            r#"
            SELECT * FROM notifications 
            WHERE user_id = $1 
            ORDER BY created_at DESC 
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Mark a notification as read
    pub async fn mark_read(&self, id: Uuid) -> Result<Option<NotificationRecord>> {
        let record = sqlx::query_as::<_, NotificationRecord>(
            r#"
            UPDATE notifications 
            SET read_at = NOW() 
            WHERE id = $1 AND read_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Mark all notifications as read for a user
    pub async fn mark_all_read(&self, user_id: Uuid) -> Result<i64> {
        let result = sqlx::query(
            r#"
            UPDATE notifications 
            SET read_at = NOW() 
            WHERE user_id = $1 AND read_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Resolve a notification with an action
    pub async fn resolve(
        &self,
        id: Uuid,
        resolution: Resolution,
    ) -> Result<Option<NotificationRecord>> {
        let record = sqlx::query_as::<_, NotificationRecord>(
            r#"
            UPDATE notifications 
            SET resolved_at = NOW(), resolution = $2, read_at = COALESCE(read_at, NOW())
            WHERE id = $1 AND resolved_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(resolution.as_str())
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a notification
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM notifications WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete old resolved notifications (cleanup)
    pub async fn delete_old_resolved(&self, before: OffsetDateTime) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM notifications 
            WHERE resolved_at IS NOT NULL AND resolved_at < $1
            "#,
        )
        .bind(before)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Check if a similar notification already exists (to prevent duplicates)
    pub async fn exists_similar(
        &self,
        user_id: Uuid,
        category: NotificationCategory,
        library_id: Option<Uuid>,
        torrent_id: Option<Uuid>,
        media_file_id: Option<Uuid>,
    ) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM notifications 
                WHERE user_id = $1 
                AND category = $2 
                AND resolved_at IS NULL
                AND (library_id = $3 OR ($3 IS NULL AND library_id IS NULL))
                AND (torrent_id = $4 OR ($4 IS NULL AND torrent_id IS NULL))
                AND (media_file_id = $5 OR ($5 IS NULL AND media_file_id IS NULL))
            )
            "#,
        )
        .bind(user_id)
        .bind(category.as_str())
        .bind(library_id)
        .bind(torrent_id)
        .bind(media_file_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }
}
