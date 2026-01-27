//! App log batch operations (used by the logging service).

use anyhow::Result;
use sqlx::SqlitePool;

use crate::services::graphql::entities::AppLog;

/// Insert a batch of app logs (used by the logging service).
pub async fn insert_app_logs_batch(pool: &SqlitePool, logs: &[AppLog]) -> Result<()> {
    if logs.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    let sql = r#"
        INSERT INTO app_logs (id, timestamp, level, target, message, fields, span_name, span_id, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
    "#;
    for log in logs {
        sqlx::query(sql)
            .bind(&log.id)
            .bind(&log.timestamp)
            .bind(&log.level)
            .bind(&log.target)
            .bind(&log.message)
            .bind(&log.fields)
            .bind(&log.span_name)
            .bind(&log.span_id)
            .bind(&log.created_at)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
