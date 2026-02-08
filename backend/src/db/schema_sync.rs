//! Automatic schema synchronization from entity definitions
//!
//! This module provides ORM-like auto-migration capabilities:
//! - Compares entity definitions to current database schema
//! - Creates missing tables automatically
//! - Adds missing columns automatically
//! - Does NOT handle column renames or type changes (requires DB wipe)
//! - Pre-seeds default data (app_settings, cast_settings, naming_patterns,
//!   torznab_categories) after sync via `run_seeds`.

use sqlx::SqlitePool;
use tracing::{debug, info, warn};

use crate::services::graphql::orm::{ColumnDef, DatabaseEntity, DatabaseSchema};

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

/// Internal schema entity for non-GraphQL table: auth_secrets.
struct AuthSecretSchema;

impl DatabaseEntity for AuthSecretSchema {
    const TABLE_NAME: &'static str = "auth_secrets";
    const PLURAL_NAME: &'static str = "AuthSecrets";
    const PRIMARY_KEY: &'static str = "key";
    const DEFAULT_SORT: &'static str = "key";

    fn column_names() -> &'static [&'static str] {
        &["key", "value"]
    }
}

impl DatabaseSchema for AuthSecretSchema {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef {
                name: "key",
                sql_type: "TEXT",
                nullable: false,
                is_primary_key: true,
                is_unique: false,
                default: None,
            },
            ColumnDef {
                name: "value",
                sql_type: "TEXT",
                nullable: false,
                is_primary_key: false,
                is_unique: false,
                default: None,
            },
        ];
        COLUMNS
    }
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
    result.columns_added.push((
        "cast_settings".to_string(),
        "default_volume (type fix)".to_string(),
    ));
    result
}

/// Ensure unique indexes exist for columns marked `is_unique`.
async fn ensure_unique_indexes_for_entity<E: DatabaseSchema>(
    pool: &SqlitePool,
) -> SchemaSyncResult {
    let mut result = SchemaSyncResult::default();
    let table = E::TABLE_NAME;
    if !table_exists(pool, table).await.unwrap_or(false) {
        return result;
    }

    for col in E::columns().iter().filter(|c| c.is_unique) {
        let index_name = format!("idx_{}_{}_unique", table, col.name);
        let sql = format!(
            "CREATE UNIQUE INDEX IF NOT EXISTS {} ON {}({})",
            index_name, table, col.name
        );
        if let Err(e) = sqlx::query(&sql).execute(pool).await {
            let msg = format!(
                "Failed to create unique index {} on {}.{}: {}",
                index_name, table, col.name, e
            );
            warn!("{}", msg);
            result.errors.push(msg);
            continue;
        }
        result
            .columns_added
            .push((table.to_string(), format!("{} (unique index)", col.name)));
    }
    result
}

/// Sync all entity tables to the database.
///
/// This should be called at startup to ensure all entity tables exist
/// and have the correct columns.
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

    // Ensure unique indexes exist for entity fields marked with #[unique].
    macro_rules! ensure_unique_for {
        ($entity:ty) => {
            let unique_result = ensure_unique_indexes_for_entity::<$entity>(pool).await;
            total_result
                .columns_added
                .extend(unique_result.columns_added);
            total_result.errors.extend(unique_result.errors);
        };
    }

    // Sync table + ensure unique indexes for an entity.
    macro_rules! sync_and_unique {
        ($entity:ty) => {
            sync_one!($entity);
            ensure_unique_for!($entity);
        };
    }

    // Sync all entity tables

    // Core content entities
    sync_and_unique!(Library);
    sync_and_unique!(Movie);
    sync_and_unique!(Show);
    sync_and_unique!(Episode);
    sync_and_unique!(MediaFile);

    // Music entities
    sync_and_unique!(Artist);
    sync_and_unique!(Album);
    sync_and_unique!(Track);

    // Audiobook entities
    sync_and_unique!(Audiobook);
    sync_and_unique!(Chapter);

    // Download entities
    sync_and_unique!(Torrent);
    sync_and_unique!(TorrentFile);
    sync_and_unique!(RssFeed);
    sync_and_unique!(RssFeedItem);
    sync_and_unique!(PendingFileMatch);

    // Source entities
    sync_and_unique!(Source);

    // User and auth entities
    sync_and_unique!(User);
    sync_and_unique!(InviteToken);
    sync_and_unique!(RefreshToken);

    // Settings and logs
    sync_and_unique!(AppSetting);
    sync_and_unique!(AppLog);

    // Media stream entities
    sync_and_unique!(VideoStream);
    sync_and_unique!(AudioStream);
    sync_and_unique!(Subtitle);
    sync_and_unique!(MediaChapter);

    // Playback and cast entities
    sync_and_unique!(PlaybackSession);
    sync_and_unique!(PlaybackProgress);
    sync_and_unique!(CastDevice);
    sync_and_unique!(CastSession);
    sync_and_unique!(CastSetting);

    // Usenet entities
    sync_and_unique!(UsenetServer);
    sync_and_unique!(UsenetDownload);

    // Schedule and automation
    sync_and_unique!(ScheduleCache);
    sync_and_unique!(ScheduleSyncState);
    sync_and_unique!(NamingPattern);
    sync_and_unique!(SourcePriorityRule);

    // Other entities
    sync_and_unique!(Notification);
    sync_and_unique!(ArtworkCache);
    sync_and_unique!(TorznabCategory);
    sync_one!(AuthSecretSchema);

    // Future one-off/manual migrations can be added here when they cannot be
    // represented by entity schema sync alone.

    // Fix cast_settings.default_volume if it was created as INTEGER (Rust expects REAL/f64)
    let fix_cast = fix_cast_settings_default_volume_type(pool).await;
    total_result.columns_added.extend(fix_cast.columns_added);
    total_result.errors.extend(fix_cast.errors);

    total_result
}
