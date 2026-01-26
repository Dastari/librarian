//! Automatic schema synchronization from entity definitions
//!
//! This module provides ORM-like auto-migration capabilities:
//! - Compares entity definitions to current database schema
//! - Creates missing tables automatically
//! - Adds missing columns automatically
//! - Runs static migrations for non-entity tables (e.g. auth_secrets)
//! - Does NOT handle column renames or type changes (requires DB wipe)
//! - Pre-seeds default data (app_settings, cast_settings, naming_patterns,
//!   torznab_categories) after sync via `run_seeds`.

use sqlx::SqlitePool;
use tracing::{debug, info, warn};

use crate::services::graphql::orm::{ColumnDef, DatabaseSchema};

pub use crate::db::seed::run_seeds;

/// Result of a schema sync operation
#[derive(Debug, Default)]
pub struct SchemaSyncResult {
    pub tables_created: Vec<String>,
    pub columns_added: Vec<(String, String)>, // (table, column)
    pub errors: Vec<String>,
}

/// Check if a table exists in the database
async fn table_exists(pool: &SqlitePool, table_name: &str) -> Result<bool, sqlx::Error> {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name = ?")
            .bind(table_name)
            .fetch_optional(pool)
            .await?;

    Ok(result.is_some())
}

/// Get existing columns for a table
async fn get_table_columns(
    pool: &SqlitePool,
    table_name: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(i32, String, String, i32, Option<String>, i32)> =
        sqlx::query_as(&format!("PRAGMA table_info({})", table_name))
            .fetch_all(pool)
            .await?;

    Ok(rows.into_iter().map(|(_, name, _, _, _, _)| name).collect())
}

/// Get the SQLite type of a column (e.g. "integer", "real", "text").
async fn get_column_type(
    pool: &SqlitePool,
    table_name: &str,
    column_name: &str,
) -> Result<Option<String>, sqlx::Error> {
    let rows: Vec<(i32, String, String, i32, Option<String>, i32)> =
        sqlx::query_as(&format!("PRAGMA table_info({})", table_name))
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .find(|(_, name, _, _, _, _)| name == column_name)
        .map(|(_, _, ty, _, _, _)| ty.to_lowercase()))
}

/// Sync a single entity's table to the database
pub async fn sync_entity<E: DatabaseSchema>(
    pool: &SqlitePool,
) -> Result<SchemaSyncResult, sqlx::Error> {
    let mut result = SchemaSyncResult::default();
    let table_name = E::TABLE_NAME;

    // Check if table exists
    if !table_exists(pool, table_name).await? {
        // Create the table
        let create_sql = E::create_table_sql();
        debug!("Creating table {}: {}", table_name, create_sql);

        match sqlx::query(&create_sql).execute(pool).await {
            Ok(_) => {
                info!("Created table: {}", table_name);
                result.tables_created.push(table_name.to_string());
            }
            Err(e) => {
                let msg = format!("Failed to create table {}: {}", table_name, e);
                warn!("{}", msg);
                result.errors.push(msg);
            }
        }
    } else {
        // Table exists, check for missing columns
        let existing_columns = get_table_columns(pool, table_name).await?;
        let defined_columns = E::columns();

        for col_def in defined_columns {
            if !existing_columns.iter().any(|c| c == col_def.name) {
                // Column doesn't exist, add it
                let alter_sql = generate_add_column_sql(table_name, col_def);
                debug!("Adding column to {}: {}", table_name, alter_sql);

                match sqlx::query(&alter_sql).execute(pool).await {
                    Ok(_) => {
                        info!("Added column {}.{}", table_name, col_def.name);
                        result
                            .columns_added
                            .push((table_name.to_string(), col_def.name.to_string()));
                    }
                    Err(e) => {
                        let msg = format!(
                            "Failed to add column {}.{}: {}",
                            table_name, col_def.name, e
                        );
                        warn!("{}", msg);
                        result.errors.push(msg);
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Generate ALTER TABLE ADD COLUMN SQL
fn generate_add_column_sql(table_name: &str, col: &ColumnDef) -> String {
    let mut sql = format!(
        "ALTER TABLE {} ADD COLUMN {} {}",
        table_name, col.name, col.sql_type
    );

    // Note: SQLite has restrictions on ALTER TABLE ADD COLUMN:
    // - Cannot add PRIMARY KEY columns
    // - Cannot add NOT NULL columns without a default
    // - Cannot add UNIQUE columns

    if let Some(default) = col.default {
        sql.push_str(&format!(" DEFAULT {}", default));
    } else if !col.nullable {
        // If NOT NULL without default, we must provide a default for SQLite
        let default_val = match col.sql_type {
            "TEXT" => "''",
            "INTEGER" => "0",
            "REAL" => "0.0",
            _ => "''",
        };
        sql.push_str(&format!(" NOT NULL DEFAULT {}", default_val));
    }

    sql
}

/// Static migrations: non-entity tables (e.g. auth_secrets).
/// Equivalent to migrations_sqlite/003_auth_secrets.sql.
async fn run_static_migrations(pool: &SqlitePool) -> SchemaSyncResult {
    let mut result = SchemaSyncResult::default();
    let existed = table_exists(pool, "auth_secrets").await.unwrap_or(false);
    const AUTH_SECRETS_SQL: &str = r#"
        CREATE TABLE IF NOT EXISTS auth_secrets (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
    "#;
    if let Err(e) = sqlx::query(AUTH_SECRETS_SQL.trim()).execute(pool).await {
        let msg = format!("Failed to run static migration (auth_secrets): {}", e);
        warn!("{}", msg);
        result.errors.push(msg);
    } else if !existed {
        result.tables_created.push("auth_secrets".to_string());
    }
    result
}

/// If cast_settings.default_volume is INTEGER (legacy), recreate table with REAL.
/// SQLite does not support ALTER COLUMN type; same logic as 004_cast_setting_default_volume_real.sql.
async fn fix_cast_settings_default_volume_type(pool: &SqlitePool) -> SchemaSyncResult {
    let mut result = SchemaSyncResult::default();
    if !table_exists(pool, "cast_settings").await.unwrap_or(false) {
        return result;
    }
    let ty = match get_column_type(pool, "cast_settings", "default_volume").await {
        Ok(Some(t)) => t,
        _ => return result,
    };
    // SQLite may report "integer", "int", "INTEGER" (we lowercased). Only REAL is correct for f64.
    if ty == "real" {
        return result;
    }
    info!(
        "Fixing cast_settings.default_volume: {:?} -> REAL (Rust expects f64)",
        ty
    );
    let stmts: &[&str] = &[
        r#"CREATE TABLE IF NOT EXISTS cast_settings_new (
            id TEXT PRIMARY KEY,
            auto_discovery_enabled INTEGER NOT NULL DEFAULT 1,
            discovery_interval_seconds INTEGER NOT NULL DEFAULT 30,
            default_volume REAL NOT NULL DEFAULT 1.0,
            transcode_incompatible INTEGER NOT NULL DEFAULT 1,
            preferred_quality TEXT DEFAULT '1080p',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"#,
        r#"INSERT INTO cast_settings_new (id, auto_discovery_enabled, discovery_interval_seconds, default_volume, transcode_incompatible, preferred_quality, created_at, updated_at)
        SELECT id, auto_discovery_enabled, discovery_interval_seconds, CAST(default_volume AS REAL), transcode_incompatible, preferred_quality, created_at, updated_at
        FROM cast_settings"#,
        "DROP TABLE cast_settings",
        "ALTER TABLE cast_settings_new RENAME TO cast_settings",
    ];
    for stmt in stmts {
        if let Err(e) = sqlx::query(stmt).execute(pool).await {
            let msg = format!("Failed to fix cast_settings.default_volume: {}", e);
            warn!("{}", msg);
            result.errors.push(msg);
            return result;
        }
    }
    result.columns_added.push(("cast_settings".to_string(), "default_volume (type fix)".to_string()));
    result
}

/// Sync all entity tables to the database.
///
/// This should be called at startup to ensure all entity tables exist
/// and have the correct columns. Also runs static migrations (e.g. auth_secrets).
pub async fn sync_all_entity_schemas(pool: &SqlitePool) -> SchemaSyncResult {
    use crate::services::graphql::entities::*;

    let mut total_result = SchemaSyncResult::default();

    // Helper macro to reduce boilerplate
    macro_rules! sync_one {
        ($entity:ty) => {
            match sync_entity::<$entity>(pool).await {
                Ok(result) => {
                    total_result.tables_created.extend(result.tables_created);
                    total_result.columns_added.extend(result.columns_added);
                    total_result.errors.extend(result.errors);
                }
                Err(e) => {
                    total_result.errors.push(format!(
                        "Error syncing {}: {}",
                        stringify!($entity),
                        e
                    ));
                }
            }
        };
    }

    // Sync all entity tables

    // Core content entities
    sync_one!(Library);
    sync_one!(Movie);
    sync_one!(Show);
    sync_one!(Episode);
    sync_one!(MediaFile);

    // Music entities
    sync_one!(Artist);
    sync_one!(Album);
    sync_one!(Track);

    // Audiobook entities
    sync_one!(Audiobook);
    sync_one!(Chapter);

    // Download entities
    sync_one!(Torrent);
    sync_one!(TorrentFile);
    sync_one!(RssFeed);
    sync_one!(RssFeedItem);
    sync_one!(PendingFileMatch);

    // Indexer entities
    sync_one!(IndexerConfig);
    sync_one!(IndexerSetting);
    sync_one!(IndexerSearchCache);

    // User and auth entities
    sync_one!(User);
    sync_one!(InviteToken);
    sync_one!(RefreshToken);

    // Settings and logs
    sync_one!(AppSetting);
    sync_one!(AppLog);

    // Media stream entities
    sync_one!(VideoStream);
    sync_one!(AudioStream);
    sync_one!(Subtitle);
    sync_one!(MediaChapter);

    // Playback and cast entities
    sync_one!(PlaybackSession);
    sync_one!(PlaybackProgress);
    sync_one!(CastDevice);
    sync_one!(CastSession);
    sync_one!(CastSetting);

    // Usenet entities
    sync_one!(UsenetServer);
    sync_one!(UsenetDownload);

    // Schedule and automation
    sync_one!(ScheduleCache);
    sync_one!(ScheduleSyncState);
    sync_one!(NamingPattern);
    sync_one!(SourcePriorityRule);

    // Other entities
    sync_one!(Notification);
    sync_one!(ArtworkCache);
    sync_one!(TorznabCategory);

    // Static migrations (non-entity tables, e.g. auth_secrets)
    let static_result = run_static_migrations(pool).await;
    total_result.tables_created.extend(static_result.tables_created);
    total_result.columns_added.extend(static_result.columns_added);
    total_result.errors.extend(static_result.errors);

    // Fix cast_settings.default_volume if it was created as INTEGER (Rust expects REAL/f64)
    let fix_cast = fix_cast_settings_default_volume_type(pool).await;
    total_result.columns_added.extend(fix_cast.columns_added);
    total_result.errors.extend(fix_cast.errors);

    total_result
}
