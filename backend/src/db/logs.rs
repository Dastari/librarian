//! Application logs database operations

use anyhow::Result;
use serde_json::Value as JsonValue;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{str_to_uuid, uuid_to_str};

/// A log record in the database
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub id: Uuid,
    pub timestamp: OffsetDateTime,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: Option<JsonValue>,
    pub span_name: Option<String>,
    pub span_id: Option<String>,
    pub created_at: OffsetDateTime,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for LogRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use time::format_description::well_known::Rfc3339;

        let id_str: String = row.try_get("id")?;
        let timestamp_str: String = row.try_get("timestamp")?;
        let created_str: String = row.try_get("created_at")?;
        let fields_str: Option<String> = row.try_get("fields")?;

        // Parse timestamps - try RFC3339 first, then SQLite datetime format
        let parse_timestamp = |s: &str| -> sqlx::Result<OffsetDateTime> {
            OffsetDateTime::parse(s, &Rfc3339)
                .or_else(|_| {
                    // Try SQLite datetime format: "YYYY-MM-DD HH:MM:SS"
                    let format = time::format_description::parse(
                        "[year]-[month]-[day] [hour]:[minute]:[second]",
                    )
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
                    OffsetDateTime::parse(s, &format)
                        .or_else(|_| {
                            // Assume UTC for naive datetime
                            time::PrimitiveDateTime::parse(s, &format)
                                .map(|dt| dt.assume_utc())
                        })
                        .map_err(|e| sqlx::Error::Decode(Box::new(e)))
                })
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))
        };

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            timestamp: parse_timestamp(&timestamp_str)?,
            level: row.try_get("level")?,
            target: row.try_get("target")?,
            message: row.try_get("message")?,
            fields: fields_str
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
            span_name: row.try_get("span_name")?,
            span_id: row.try_get("span_id")?,
            created_at: parse_timestamp(&created_str)?,
        })
    }
}

/// Input for creating a new log entry
#[derive(Debug, Clone)]
pub struct CreateLog {
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: Option<JsonValue>,
    pub span_name: Option<String>,
    pub span_id: Option<String>,
}

/// Filter options for querying logs
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    /// Filter by log levels (e.g., ["ERROR", "WARN"])
    pub levels: Option<Vec<String>>,
    /// Filter by target/source (e.g., "librarian_backend::jobs")
    pub target: Option<String>,
    /// Filter by keyword in message
    pub keyword: Option<String>,
    /// Filter logs after this timestamp
    pub from_timestamp: Option<OffsetDateTime>,
    /// Filter logs before this timestamp
    pub to_timestamp: Option<OffsetDateTime>,
}

/// Order by options for logs
#[derive(Debug, Clone)]
pub struct LogOrderBy {
    /// Field to sort by (timestamp, level, target)
    pub field: String,
    /// Sort direction (ASC or DESC)
    pub direction: String,
}

/// Result for paginated log queries
#[derive(Debug, Clone)]
pub struct PaginatedLogs {
    pub logs: Vec<LogRecord>,
    pub total_count: i64,
    pub has_more: bool,
}

/// Logs repository for database operations
pub struct LogsRepository {
    pool: DbPool,
}

impl LogsRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert a new log entry

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, log: CreateLog) -> Result<LogRecord> {
        let id = uuid_to_str(Uuid::new_v4());
        let fields_json = log.fields.as_ref().map(|f| serde_json::to_string(f).unwrap_or_default());

        sqlx::query(
            r#"
            INSERT INTO app_logs (id, level, target, message, fields, span_name, span_id, timestamp, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id)
        .bind(&log.level)
        .bind(&log.target)
        .bind(&log.message)
        .bind(&fields_json)
        .bind(&log.span_name)
        .bind(&log.span_id)
        .execute(&self.pool)
        .await?;

        // Fetch the record back
        let record = sqlx::query_as::<_, LogRecord>("SELECT * FROM app_logs WHERE id = ?1")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;

        Ok(record)
    }

    /// Insert multiple log entries in a batch (for efficiency)

    #[cfg(feature = "sqlite")]
    pub async fn create_batch(&self, logs: Vec<CreateLog>) -> Result<usize> {
        if logs.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        let mut count = 0;

        for log in logs {
            let id = uuid_to_str(Uuid::new_v4());
            let fields_json = log.fields.as_ref().map(|f| serde_json::to_string(f).unwrap_or_default());

            sqlx::query(
                r#"
                INSERT INTO app_logs (id, level, target, message, fields, span_name, span_id, timestamp, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))
                "#,
            )
            .bind(&id)
            .bind(&log.level)
            .bind(&log.target)
            .bind(&log.message)
            .bind(&fields_json)
            .bind(&log.span_name)
            .bind(&log.span_id)
            .execute(&mut *tx)
            .await?;
            count += 1;
        }

        tx.commit().await?;
        Ok(count)
    }

    /// Get logs with filtering, ordering, and pagination

    #[cfg(feature = "sqlite")]
    pub async fn list(
        &self,
        filter: LogFilter,
        order_by: Option<LogOrderBy>,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedLogs> {
        use time::format_description::well_known::Rfc3339;

        // Build the WHERE clause dynamically
        let mut conditions = Vec::new();
        let mut params_count = 0;

        // Build conditions based on filters
        // SQLite doesn't have ANY(), so we need to handle levels differently
        if let Some(ref levels) = filter.levels {
            if !levels.is_empty() {
                // Build IN clause with placeholders
                let placeholders: Vec<String> = levels
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", params_count + i + 1))
                    .collect();
                conditions.push(format!("level IN ({})", placeholders.join(", ")));
                params_count += levels.len();
            }
        }

        // SQLite uses LIKE instead of ILIKE (case-insensitive by default for ASCII)
        if filter.target.is_some() {
            params_count += 1;
            conditions.push(format!("target LIKE ?{} || '%'", params_count));
        }

        if filter.keyword.is_some() {
            params_count += 1;
            conditions.push(format!("message LIKE '%' || ?{} || '%'", params_count));
        }

        if filter.from_timestamp.is_some() {
            params_count += 1;
            conditions.push(format!("timestamp >= ?{}", params_count));
        }

        if filter.to_timestamp.is_some() {
            params_count += 1;
            conditions.push(format!("timestamp <= ?{}", params_count));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Build ORDER BY clause - validate field names to prevent SQL injection
        let order_clause = match &order_by {
            Some(order) => {
                let field = match order.field.as_str() {
                    "timestamp" => "timestamp",
                    "level" => "level",
                    "target" => "target",
                    _ => "timestamp",
                };
                let direction = if order.direction.to_uppercase() == "ASC" {
                    "ASC"
                } else {
                    "DESC"
                };
                format!("ORDER BY {} {}", field, direction)
            }
            None => "ORDER BY timestamp DESC".to_string(),
        };

        // Count query
        let count_sql = format!("SELECT COUNT(*) as count FROM app_logs {}", where_clause);

        // Data query with limit/offset
        let data_sql = format!(
            "SELECT * FROM app_logs {} {} LIMIT ?{} OFFSET ?{}",
            where_clause,
            order_clause,
            params_count + 1,
            params_count + 2
        );

        // Helper to bind filter params to a query
        // We need to execute both queries with the same filter bindings
        
        // Execute count query
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);

        if let Some(ref levels) = filter.levels {
            for level in levels {
                count_query = count_query.bind(level);
            }
        }
        if let Some(ref target) = filter.target {
            count_query = count_query.bind(target);
        }
        if let Some(ref keyword) = filter.keyword {
            count_query = count_query.bind(keyword);
        }
        if let Some(ref from_ts) = filter.from_timestamp {
            let ts_str = from_ts.format(&Rfc3339).unwrap_or_default();
            count_query = count_query.bind(ts_str);
        }
        if let Some(ref to_ts) = filter.to_timestamp {
            let ts_str = to_ts.format(&Rfc3339).unwrap_or_default();
            count_query = count_query.bind(ts_str);
        }

        let total_count = count_query.fetch_one(&self.pool).await?;

        // Execute data query
        let mut data_query = sqlx::query_as::<_, LogRecord>(&data_sql);

        if let Some(ref levels) = filter.levels {
            for level in levels {
                data_query = data_query.bind(level);
            }
        }
        if let Some(ref target) = filter.target {
            data_query = data_query.bind(target);
        }
        if let Some(ref keyword) = filter.keyword {
            data_query = data_query.bind(keyword);
        }
        if let Some(ref from_ts) = filter.from_timestamp {
            let ts_str = from_ts.format(&Rfc3339).unwrap_or_default();
            data_query = data_query.bind(ts_str);
        }
        if let Some(ref to_ts) = filter.to_timestamp {
            let ts_str = to_ts.format(&Rfc3339).unwrap_or_default();
            data_query = data_query.bind(ts_str);
        }

        data_query = data_query.bind(limit).bind(offset);

        let logs = data_query.fetch_all(&self.pool).await?;
        let has_more = (offset + logs.len() as i64) < total_count;

        Ok(PaginatedLogs {
            logs,
            total_count,
            has_more,
        })
    }

    /// Get a single log by ID

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<LogRecord>> {
        let record = sqlx::query_as::<_, LogRecord>("SELECT * FROM app_logs WHERE id = ?1")
            .bind(uuid_to_str(id))
            .fetch_optional(&self.pool)
            .await?;

        Ok(record)
    }

    /// Get distinct target values for filtering (top N most common)

    #[cfg(feature = "sqlite")]
    pub async fn get_distinct_targets(&self, limit: i64) -> Result<Vec<String>> {
        let targets = sqlx::query_scalar::<_, String>(
            r#"
            SELECT target
            FROM app_logs
            GROUP BY target
            ORDER BY COUNT(*) DESC
            LIMIT ?1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(targets)
    }

    /// Get log count by level (for stats)
    pub async fn get_counts_by_level(&self) -> Result<Vec<(String, i64)>> {
        let counts = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT level, COUNT(*) as count
            FROM app_logs
            GROUP BY level
            ORDER BY level
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(counts)
    }

    /// Delete logs older than a given timestamp (for cleanup)

    #[cfg(feature = "sqlite")]
    pub async fn delete_before(&self, before: OffsetDateTime) -> Result<u64> {
        use time::format_description::well_known::Rfc3339;

        let before_str = before.format(&Rfc3339).unwrap_or_default();
        let result = sqlx::query("DELETE FROM app_logs WHERE timestamp < ?1")
            .bind(before_str)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete all logs (for cleanup)
    pub async fn delete_all(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM app_logs")
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
