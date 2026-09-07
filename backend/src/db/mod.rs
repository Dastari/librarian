pub type DbPool = graphql_orm::DbPool;
pub type Database = graphql_orm::db::Database;

use std::path::Path;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

/// Default size of the connection pool (shared between readers and writers).
const DEFAULT_MAX_CONNECTIONS: u32 = 10;
/// How long a caller waits for a free connection before giving up.
const DEFAULT_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);
/// How long SQLite waits on `SQLITE_BUSY` before giving up, at the connection level
/// (in addition to the pool-level `acquire_timeout` above).
const DEFAULT_BUSY_TIMEOUT: Duration = Duration::from_secs(30);

/// Ensure the parent directory of the database path exists.
/// No-op for in-memory or empty paths.
fn ensure_database_parent_dir(opts: &SqliteConnectOptions) -> anyhow::Result<()> {
    let path = opts.get_filename();
    if path.as_os_str().is_empty() || path == Path::new(":memory:") {
        return Ok(());
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!("Failed to create database directory {:?}: {}", parent, e)
        })?;
    }
    Ok(())
}

/// Whether a SQLite connection URL refers to an in-memory (or shared-cache in-memory)
/// database rather than a file on disk. Covers the `sqlite::memory:`,
/// `sqlite://:memory:` and `sqlite://?mode=memory` forms sqlx accepts.
///
/// WAL is unsupported for in-memory/temporary databases: SQLite silently falls back to
/// its "memory" journal mode instead of erroring, but we skip setting it explicitly so
/// intent in the code matches what actually happens on disk (or doesn't).
fn is_in_memory_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains(":memory:") || lower.contains("mode=memory")
}

/// Connect to the database with retries. Creates the database file and parent
/// directory if they do not exist (for file-based SQLite URLs).
///
/// Applies production-sane SQLite connection settings: WAL journaling with `NORMAL`
/// synchronous (skipped for in-memory databases, which don't support WAL), a busy
/// timeout so concurrent writers back off instead of immediately erroring with
/// `SQLITE_BUSY`, and explicit foreign-key enforcement.
pub async fn connect_with_retry(
    url: &str,
    timeout: std::time::Duration,
) -> anyhow::Result<Database> {
    let opts = url
        .parse::<SqliteConnectOptions>()
        .map_err(|e| anyhow::anyhow!("Invalid database URL: {}", e))?;
    ensure_database_parent_dir(&opts)?;
    let mut opts = opts.create_if_missing(true).foreign_keys(true);

    if !is_in_memory_url(url) {
        opts = opts
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);
    }
    opts = opts.busy_timeout(DEFAULT_BUSY_TIMEOUT);

    let pool_opts = SqlitePoolOptions::new()
        .max_connections(DEFAULT_MAX_CONNECTIONS)
        .acquire_timeout(DEFAULT_ACQUIRE_TIMEOUT);

    let start = Instant::now();
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match pool_opts.clone().connect_with(opts.clone()).await {
            Ok(pool) => return Ok(Database::new(pool)),
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
