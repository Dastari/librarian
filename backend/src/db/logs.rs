//! Application logs database operations

use anyhow::Result;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// A log record in the database
#[derive(Debug, Clone, sqlx::FromRow)]
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

/// Result for paginated log queries
#[derive(Debug, Clone)]
pub struct PaginatedLogs {
    pub logs: Vec<LogRecord>,
    pub total_count: i64,
    pub has_more: bool,
}

/// Logs repository for database operations
pub struct LogsRepository {
    pool: PgPool,
}

impl LogsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new log entry
    pub async fn create(&self, log: CreateLog) -> Result<LogRecord> {
        let record = sqlx::query_as::<_, LogRecord>(
            r#"
            INSERT INTO app_logs (level, target, message, fields, span_name, span_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&log.level)
        .bind(&log.target)
        .bind(&log.message)
        .bind(&log.fields)
        .bind(&log.span_name)
        .bind(&log.span_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Insert multiple log entries in a batch (for efficiency)
    pub async fn create_batch(&self, logs: Vec<CreateLog>) -> Result<usize> {
        if logs.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        let mut count = 0;

        for log in logs {
            sqlx::query(
                r#"
                INSERT INTO app_logs (level, target, message, fields, span_name, span_id)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(&log.level)
            .bind(&log.target)
            .bind(&log.message)
            .bind(&log.fields)
            .bind(&log.span_name)
            .bind(&log.span_id)
            .execute(&mut *tx)
            .await?;
            count += 1;
        }

        tx.commit().await?;
        Ok(count)
    }

    /// Get logs with filtering and pagination
    pub async fn list(
        &self,
        filter: LogFilter,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedLogs> {
        // Build the WHERE clause dynamically
        let mut conditions = Vec::new();
        let mut params_count = 0;

        // Build conditions based on filters
        if filter.levels.is_some() {
            params_count += 1;
            conditions.push(format!("level = ANY(${})", params_count));
        }

        if filter.target.is_some() {
            params_count += 1;
            conditions.push(format!("target ILIKE ${}  || '%'", params_count));
        }

        if filter.keyword.is_some() {
            params_count += 1;
            conditions.push(format!("message ILIKE '%' || ${} || '%'", params_count));
        }

        if filter.from_timestamp.is_some() {
            params_count += 1;
            conditions.push(format!("timestamp >= ${}", params_count));
        }

        if filter.to_timestamp.is_some() {
            params_count += 1;
            conditions.push(format!("timestamp <= ${}", params_count));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count query
        let count_sql = format!("SELECT COUNT(*) as count FROM app_logs {}", where_clause);
        
        // Data query with limit/offset
        let data_sql = format!(
            "SELECT * FROM app_logs {} ORDER BY timestamp DESC LIMIT ${} OFFSET ${}",
            where_clause,
            params_count + 1,
            params_count + 2
        );

        // Execute count query
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);

        if let Some(ref levels) = filter.levels {
            count_query = count_query.bind(levels);
        }
        if let Some(ref target) = filter.target {
            count_query = count_query.bind(target);
        }
        if let Some(ref keyword) = filter.keyword {
            count_query = count_query.bind(keyword);
        }
        if let Some(ref from_ts) = filter.from_timestamp {
            count_query = count_query.bind(from_ts);
        }
        if let Some(ref to_ts) = filter.to_timestamp {
            count_query = count_query.bind(to_ts);
        }

        let total_count = count_query.fetch_one(&self.pool).await?;

        // Execute data query
        let mut data_query = sqlx::query_as::<_, LogRecord>(&data_sql);

        if let Some(ref levels) = filter.levels {
            data_query = data_query.bind(levels);
        }
        if let Some(ref target) = filter.target {
            data_query = data_query.bind(target);
        }
        if let Some(ref keyword) = filter.keyword {
            data_query = data_query.bind(keyword);
        }
        if let Some(ref from_ts) = filter.from_timestamp {
            data_query = data_query.bind(from_ts);
        }
        if let Some(ref to_ts) = filter.to_timestamp {
            data_query = data_query.bind(to_ts);
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
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<LogRecord>> {
        let record = sqlx::query_as::<_, LogRecord>(
            "SELECT * FROM app_logs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get distinct target values for filtering (top N most common)
    pub async fn get_distinct_targets(&self, limit: i64) -> Result<Vec<String>> {
        let targets = sqlx::query_scalar::<_, String>(
            r#"
            SELECT target
            FROM app_logs
            GROUP BY target
            ORDER BY COUNT(*) DESC
            LIMIT $1
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
    pub async fn delete_before(&self, before: OffsetDateTime) -> Result<u64> {
        let result = sqlx::query("DELETE FROM app_logs WHERE timestamp < $1")
            .bind(before)
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
