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

/// Internal table used to track one-off in-code migrations/fixes applied by schema_sync.
const SCHEMA_MIGRATIONS_TABLE: &str = "schema_migrations";

async fn ensure_schema_migrations_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let sql = format!(
        r#"
        CREATE TABLE IF NOT EXISTS {} (
            id TEXT PRIMARY KEY NOT NULL,
            description TEXT,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
        SCHEMA_MIGRATIONS_TABLE
    );
    sqlx::query(&sql).execute(pool).await?;
    Ok(())
}

async fn is_migration_applied(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let sql = format!("SELECT id FROM {} WHERE id = ?1", SCHEMA_MIGRATIONS_TABLE);
    let row: Option<(String,)> = sqlx::query_as(&sql).bind(id).fetch_optional(pool).await?;
    Ok(row.is_some())
}

async fn mark_migration_applied(
    pool: &SqlitePool,
    id: &str,
    description: &str,
) -> Result<(), sqlx::Error> {
    let sql = format!(
        "INSERT OR IGNORE INTO {} (id, description) VALUES (?1, ?2)",
        SCHEMA_MIGRATIONS_TABLE
    );
    sqlx::query(&sql)
        .bind(id)
        .bind(description)
        .execute(pool)
        .await?;
    Ok(())
}

fn append_schema_sync_result(to: &mut SchemaSyncResult, from: SchemaSyncResult) {
    to.tables_created.extend(from.tables_created);
    to.columns_added.extend(from.columns_added);
    to.errors.extend(from.errors);
}

async fn apply_in_code_migration_once<F>(
    pool: &SqlitePool,
    total_result: &mut SchemaSyncResult,
    id: &str,
    description: &str,
    runner: F,
) where
    F: std::future::Future<Output = SchemaSyncResult>,
{
    let already_applied = match is_migration_applied(pool, id).await {
        Ok(v) => v,
        Err(e) => {
            total_result.errors.push(format!(
                "Failed checking migration {} applied state: {}",
                id, e
            ));
            return;
        }
    };
    if already_applied {
        return;
    }

    info!(
        migration_id = id,
        description = description,
        "Applying in-code schema migration"
    );
    let migration_result = runner.await;
    let has_errors = !migration_result.errors.is_empty();
    append_schema_sync_result(total_result, migration_result);

    if has_errors {
        warn!(
            migration_id = id,
            "In-code schema migration finished with errors; leaving unapplied"
        );
        return;
    }

    if let Err(e) = mark_migration_applied(pool, id, description).await {
        total_result.errors.push(format!(
            "Migration {} ran but failed to mark as applied: {}",
            id, e
        ));
        return;
    }
    info!(migration_id = id, "In-code schema migration applied");
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

/// Get whether a column is declared NOT NULL.
async fn is_column_not_null(
    pool: &SqlitePool,
    table_name: &str,
    column_name: &str,
) -> Result<Option<bool>, sqlx::Error> {
    let rows: Vec<(i32, String, String, i32, Option<String>, i32)> =
        sqlx::query_as(&format!("PRAGMA table_info({})", table_name))
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .find(|(_, name, _, _, _, _)| name == column_name)
        .map(|(_, _, _, notnull, _, _)| notnull != 0))
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

/// If media_files.library_id is NOT NULL (legacy), recreate table with nullable library_id.
/// SQLite cannot ALTER COLUMN nullability in place.
async fn fix_media_files_library_id_nullable(pool: &SqlitePool) -> SchemaSyncResult {
    let mut result = SchemaSyncResult::default();

    if !table_exists(pool, "media_files").await.unwrap_or(false) {
        return result;
    }

    let needs_fix = match is_column_not_null(pool, "media_files", "library_id").await {
        Ok(Some(v)) => v,
        Ok(None) => false,
        Err(e) => {
            result.errors.push(format!(
                "Failed checking media_files.library_id nullability: {}",
                e
            ));
            return result;
        }
    };

    if !needs_fix {
        // Normalize legacy sentinel values from earlier unmatched-ingest implementation.
        if let Err(e) = sqlx::query(
            "UPDATE media_files SET library_id = NULL WHERE library_id = '__torrent_unmatched__'",
        )
        .execute(pool)
        .await
        {
            result.errors.push(format!(
                "Failed normalizing media_files.library_id sentinel values: {}",
                e
            ));
        }
        return result;
    }

    info!("Fixing media_files.library_id: NOT NULL -> NULL");

    let stmts: &[&str] = &[
        r#"CREATE TABLE IF NOT EXISTS media_files_new (
            id TEXT PRIMARY KEY,
            library_id TEXT,
            episode_id TEXT,
            movie_id TEXT,
            track_id TEXT,
            path TEXT NOT NULL,
            relative_path TEXT,
            original_name TEXT,
            size INTEGER NOT NULL,
            container TEXT,
            video_codec TEXT,
            audio_codec TEXT,
            width INTEGER,
            height INTEGER,
            duration INTEGER,
            bitrate INTEGER,
            resolution TEXT,
            is_hdr INTEGER NOT NULL DEFAULT 0,
            hdr_type TEXT,
            audio_channels TEXT,
            metadata TEXT,
            content_type TEXT,
            added_at TEXT NOT NULL,
            analyzed_at TEXT
        )"#,
        r#"INSERT INTO media_files_new (
            id, library_id, episode_id, movie_id, track_id, path, relative_path, original_name,
            size, container, video_codec, audio_codec, width, height, duration, bitrate,
            resolution, is_hdr, hdr_type, audio_channels, metadata, content_type, added_at,
            analyzed_at
        )
        SELECT
            id,
            CASE
                WHEN library_id = '__torrent_unmatched__' THEN NULL
                ELSE library_id
            END AS library_id,
            episode_id, movie_id, track_id, path, relative_path, original_name,
            size, container, video_codec, audio_codec, width, height, duration, bitrate,
            resolution, is_hdr, hdr_type, audio_channels, metadata, content_type, added_at,
            analyzed_at
        FROM media_files"#,
        "DROP TABLE media_files",
        "ALTER TABLE media_files_new RENAME TO media_files",
    ];

    for stmt in stmts {
        if let Err(e) = sqlx::query(stmt).execute(pool).await {
            let msg = format!("Failed to fix media_files.library_id nullability: {}", e);
            warn!("{}", msg);
            result.errors.push(msg);
            return result;
        }
    }

    result.columns_added.push((
        "media_files".to_string(),
        "library_id (nullable type fix)".to_string(),
    ));
    result
}

/// Ensure episodes have a composite unique key on (show_id, season, episode).
///
/// This prevents duplicate episodes from refresh/sync paths and enforces the
/// natural per-show episode identity at the DB level.
async fn ensure_episode_composite_unique_index(pool: &SqlitePool) -> SchemaSyncResult {
    let mut result = SchemaSyncResult::default();

    if !table_exists(pool, "episodes").await.unwrap_or(false) {
        return result;
    }

    // Remove duplicate rows so the unique index can be created.
    // Keep the best candidate by:
    // 1) linked media file present
    // 2) wanted=true
    // 3) most recently updated
    // 4) latest rowid as tie-breaker
    if let Err(e) = sqlx::query(
        r#"
        DELETE FROM episodes
        WHERE rowid IN (
            SELECT rowid FROM (
                SELECT
                    rowid,
                    ROW_NUMBER() OVER (
                        PARTITION BY show_id, season, episode
                        ORDER BY
                            CASE WHEN media_file_id IS NOT NULL THEN 1 ELSE 0 END DESC,
                            CASE WHEN wanted THEN 1 ELSE 0 END DESC,
                            COALESCE(updated_at, created_at) DESC,
                            rowid DESC
                    ) AS rn
                FROM episodes
            )
            WHERE rn > 1
        )
        "#,
    )
    .execute(pool)
    .await
    {
        let msg = format!("Failed deduplicating episodes before unique index: {}", e);
        warn!("{}", msg);
        result.errors.push(msg);
        return result;
    }

    if let Err(e) = sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_episodes_show_season_episode_unique ON episodes(show_id, season, episode)",
    )
    .execute(pool)
    .await
    {
        let msg = format!(
            "Failed to create unique index idx_episodes_show_season_episode_unique: {}",
            e
        );
        warn!("{}", msg);
        result.errors.push(msg);
        return result;
    }

    result.columns_added.push((
        "episodes".to_string(),
        "show_id,season,episode (composite unique index)".to_string(),
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

/// Ensure a collection can only exist once per (library_id, tmdb_collection_id).
async fn ensure_collections_library_tmdb_unique_index(pool: &SqlitePool) -> SchemaSyncResult {
    let mut result = SchemaSyncResult::default();
    if !table_exists(pool, "collections").await.unwrap_or(false) {
        return result;
    }

    let index_sql = "CREATE UNIQUE INDEX IF NOT EXISTS idx_collections_library_tmdb_unique ON collections(library_id, tmdb_collection_id)";
    if let Err(e) = sqlx::query(index_sql).execute(pool).await {
        result.errors.push(format!(
            "Failed creating collections composite unique index: {}",
            e
        ));
        return result;
    }

    result.columns_added.push((
        "collections".to_string(),
        "library_id,tmdb_collection_id (composite unique index)".to_string(),
    ));
    result
}

/// Ensure a movie can only exist once per (library_id, tmdb_id).
///
/// This prevents duplicate movie rows from repeated scans/refreshes while still
/// allowing the same TMDB movie in different libraries.
async fn ensure_movies_library_tmdb_unique_index(pool: &SqlitePool) -> SchemaSyncResult {
    let mut result = SchemaSyncResult::default();
    if !table_exists(pool, "movies").await.unwrap_or(false) {
        return result;
    }

    // Remove duplicate rows so the unique index can be created.
    // Keep the best candidate by:
    // 1) linked media file present
    // 2) has_file=true
    // 3) wanted=true
    // 4) most recently updated
    // 5) latest rowid as tie-breaker
    if let Err(e) = sqlx::query(
        r#"
        DELETE FROM movies
        WHERE rowid IN (
            SELECT rowid FROM (
                SELECT
                    rowid,
                    ROW_NUMBER() OVER (
                        PARTITION BY library_id, tmdb_id
                        ORDER BY
                            CASE WHEN media_file_id IS NOT NULL THEN 1 ELSE 0 END DESC,
                            CASE WHEN has_file THEN 1 ELSE 0 END DESC,
                            CASE WHEN wanted THEN 1 ELSE 0 END DESC,
                            COALESCE(updated_at, created_at) DESC,
                            rowid DESC
                    ) AS rn
                FROM movies
                WHERE tmdb_id IS NOT NULL
            )
            WHERE rn > 1
        )
        "#,
    )
    .execute(pool)
    .await
    {
        result.errors.push(format!(
            "Failed deduplicating movies before composite unique index: {}",
            e
        ));
        return result;
    }

    let index_sql = "CREATE UNIQUE INDEX IF NOT EXISTS idx_movies_library_tmdb_unique ON movies(library_id, tmdb_id)";
    if let Err(e) = sqlx::query(index_sql).execute(pool).await {
        result.errors.push(format!(
            "Failed creating movies composite unique index: {}",
            e
        ));
        return result;
    }

    result.columns_added.push((
        "movies".to_string(),
        "library_id,tmdb_id (composite unique index)".to_string(),
    ));
    result
}

/// Ensure a person appears only once per movie in movie cast credits.
async fn ensure_movie_cast_credit_unique_index(pool: &SqlitePool) -> SchemaSyncResult {
    let mut result = SchemaSyncResult::default();
    if !table_exists(pool, "movie_cast_credits")
        .await
        .unwrap_or(false)
    {
        return result;
    }

    let index_sql = "CREATE UNIQUE INDEX IF NOT EXISTS idx_movie_cast_credits_movie_person_unique ON movie_cast_credits(movie_id, person_id)";
    if let Err(e) = sqlx::query(index_sql).execute(pool).await {
        result.errors.push(format!(
            "Failed creating movie_cast_credits composite unique index: {}",
            e
        ));
        return result;
    }

    result.columns_added.push((
        "movie_cast_credits".to_string(),
        "movie_id,person_id (composite unique index)".to_string(),
    ));
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
    sync_and_unique!(Person);
    sync_and_unique!(MovieCastCredit);
    sync_and_unique!(Collection);
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

    // One-off/manual in-code migrations (ordered, applied once, recorded).
    if let Err(e) = ensure_schema_migrations_table(pool).await {
        total_result.errors.push(format!(
            "Failed to ensure {} table: {}",
            SCHEMA_MIGRATIONS_TABLE, e
        ));
        return total_result;
    }

    apply_in_code_migration_once(
        pool,
        &mut total_result,
        "2026_02_14_fix_cast_settings_default_volume_type",
        "Recreate cast_settings with REAL default_volume where legacy INTEGER exists",
        fix_cast_settings_default_volume_type(pool),
    )
    .await;
    apply_in_code_migration_once(
        pool,
        &mut total_result,
        "2026_02_14_fix_media_files_library_id_nullable",
        "Recreate media_files to make library_id nullable for unmatched ingest",
        fix_media_files_library_id_nullable(pool),
    )
    .await;
    apply_in_code_migration_once(
        pool,
        &mut total_result,
        "2026_02_14_enforce_episode_unique",
        "Dedupe episodes and enforce unique (show_id, season, episode)",
        ensure_episode_composite_unique_index(pool),
    )
    .await;
    apply_in_code_migration_once(
        pool,
        &mut total_result,
        "2026_02_14_enforce_collections_library_tmdb_unique",
        "Enforce unique collections per (library_id, tmdb_collection_id)",
        ensure_collections_library_tmdb_unique_index(pool),
    )
    .await;
    apply_in_code_migration_once(
        pool,
        &mut total_result,
        "2026_02_14_enforce_movies_library_tmdb_unique",
        "Dedupe movies and enforce unique (library_id, tmdb_id)",
        ensure_movies_library_tmdb_unique_index(pool),
    )
    .await;
    apply_in_code_migration_once(
        pool,
        &mut total_result,
        "2026_02_14_enforce_movie_cast_credit_unique",
        "Enforce unique movie cast credits per (movie_id, person_id)",
        ensure_movie_cast_credit_unique_index(pool),
    )
    .await;

    total_result
}
