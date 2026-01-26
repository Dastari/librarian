pub mod operations;
pub mod schema_sync;
pub mod seed;
pub mod sqlite_helpers;

pub type DbPool = sqlx::SqlitePool;
pub type Database = sqlx::SqlitePool;

use std::path::Path;
use std::time::Instant;
use tokio::time::sleep;

use sqlx::sqlite::SqliteConnectOptions;

/// Ensure the parent directory of the database path exists.
/// No-op for in-memory or empty paths.
fn ensure_database_parent_dir(opts: &SqliteConnectOptions) -> anyhow::Result<()> {
    let path = opts.get_filename();
    if path.as_os_str().is_empty() || path == Path::new(":memory:") {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                anyhow::anyhow!("Failed to create database directory {:?}: {}", parent, e)
            })?;
        }
    }
    Ok(())
}

/// Connect to the database with retries. Creates the database file and parent
/// directory if they do not exist (for file-based SQLite URLs).
pub async fn connect_with_retry(
    url: &str,
    timeout: std::time::Duration,
) -> anyhow::Result<Database> {
    let opts = url
        .parse::<SqliteConnectOptions>()
        .map_err(|e| anyhow::anyhow!("Invalid database URL: {}", e))?;
    ensure_database_parent_dir(&opts)?;
    let opts = opts.create_if_missing(true);

    let start = Instant::now();
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match Database::connect_with(opts.clone()).await {
            Ok(pool) => return Ok(pool),
            Err(e) => {
                if start.elapsed() >= timeout {
                    anyhow::bail!(
                        "Database connection failed after {:?} (attempt {}): {}",
                        timeout,
                        attempt,
                        e
                    );
                }
                eprintln!(
                    "Database not ready (attempt {}), retrying in 1s... {}",
                    attempt, e
                );
                sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}
