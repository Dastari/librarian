//! Database helpers for the torrent service.
//! Uses the pool directly; table/column names must match the GraphQL entity schema (snake_case in DB).

use super::{add_torrent_opts, get_info_hash_hex};

use uuid::Uuid;

use crate::db::Database;
use librqbit::AddTorrent;

/// Read a string value from app_settings (raw value, not JSON).
pub async fn get_setting_string(pool: &Database, key: &str) -> Result<Option<String>, anyhow::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(s,)| s).filter(|s| !s.trim().is_empty()))
}

/// Read a value from app_settings. Value is parsed as JSON (e.g. "true", "5", "0" for bool/u16/usize).
pub async fn get_setting<T: serde::de::DeserializeOwned>(pool: &Database, key: &str) -> Result<Option<T>, anyhow::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;

    match row {
        Some((s,)) => {
            let s = s.trim();
            if s.is_empty() {
                return Ok(None);
            }
            let v: T = serde_json::from_str(s).map_err(|e| anyhow::anyhow!("app_settings key {}: {}", key, e))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

/// First user id from users table (for fallback when creating torrent records).
pub async fn get_default_user_id(pool: &Database) -> Result<Option<Uuid>, anyhow::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
    }
    let row: Option<Row> =
        sqlx::query_as("SELECT id FROM users ORDER BY created_at ASC LIMIT 1")
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|r| Uuid::parse_str(&r.id).ok()))
}

/// Insert a new torrent record.
pub async fn create_torrent(
    pool: &Database,
    user_id: Uuid,
    info_hash: &str,
    magnet_uri: Option<&str>,
    name: &str,
    save_path: &str,
    total_bytes: i64,
) -> Result<(), anyhow::Error> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let added_at = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let created_at = added_at.clone();

    sqlx::query(
        r#"INSERT INTO torrents (id, user_id, info_hash, magnet_uri, name, state, progress, total_bytes, downloaded_bytes, uploaded_bytes, save_path, added_at, created_at)
           VALUES (?, ?, ?, ?, ?, 'downloading', 0, ?, 0, 0, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(user_id.to_string())
    .bind(info_hash)
    .bind(magnet_uri)
    .bind(name)
    .bind(total_bytes)
    .bind(save_path)
    .bind(&added_at)
    .bind(&created_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Upsert a torrent from session state (by info_hash).
pub async fn upsert_from_session(
    pool: &Database,
    info_hash: &str,
    name: &str,
    state: &str,
    progress: f64,
    total_bytes: i64,
    downloaded_bytes: i64,
    uploaded_bytes: i64,
    save_path: &str,
    user_id: Uuid,
) -> Result<(), anyhow::Error> {
    let now = chrono::Utc::now();
    let ts = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    // Check if exists
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM torrents WHERE info_hash = ?")
            .bind(info_hash)
            .fetch_optional(pool)
            .await?;

    if let Some((id,)) = existing {
        sqlx::query(
            r#"UPDATE torrents SET name = ?, state = ?, progress = ?, downloaded_bytes = ?, uploaded_bytes = ?, save_path = ?
               WHERE info_hash = ?"#,
        )
        .bind(name)
        .bind(state)
        .bind(progress)
        .bind(downloaded_bytes)
        .bind(uploaded_bytes)
        .bind(save_path)
        .bind(info_hash)
        .execute(pool)
        .await?;
    } else {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"INSERT INTO torrents (id, user_id, info_hash, magnet_uri, name, state, progress, total_bytes, downloaded_bytes, uploaded_bytes, save_path, added_at, created_at)
               VALUES (?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(user_id.to_string())
        .bind(info_hash)
        .bind(name)
        .bind(state)
        .bind(progress)
        .bind(total_bytes)
        .bind(downloaded_bytes)
        .bind(uploaded_bytes)
        .bind(save_path)
        .bind(&ts)
        .bind(&ts)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Update progress/state for an existing torrent.
pub async fn update_progress(
    pool: &Database,
    info_hash: &str,
    state: &str,
    progress: f64,
    downloaded_bytes: i64,
    uploaded_bytes: i64,
) -> Result<(), anyhow::Error> {
    sqlx::query(
        r#"UPDATE torrents SET state = ?, progress = ?, downloaded_bytes = ?, uploaded_bytes = ? WHERE info_hash = ?"#,
    )
    .bind(state)
    .bind(progress)
    .bind(downloaded_bytes)
    .bind(uploaded_bytes)
    .bind(info_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_completed(pool: &Database, info_hash: &str) -> Result<(), anyhow::Error> {
    let now = chrono::Utc::now();
    let ts = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    sqlx::query(
        r#"UPDATE torrents SET state = 'seeding', progress = 1.0, completed_at = ? WHERE info_hash = ?"#,
    )
    .bind(&ts)
    .bind(info_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_state(pool: &Database, info_hash: &str, state: &str) -> Result<(), anyhow::Error> {
    sqlx::query(r#"UPDATE torrents SET state = ? WHERE info_hash = ?"#)
        .bind(state)
        .bind(info_hash)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete a torrent record by info_hash.
pub async fn delete_torrent(pool: &Database, info_hash: &str) -> Result<(), anyhow::Error> {
    sqlx::query("DELETE FROM torrents WHERE info_hash = ?")
        .bind(info_hash)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get torrent id and excluded file indices by info_hash (for syncing files).
pub async fn get_torrent_id_and_excluded(
    pool: &Database,
    info_hash: &str,
) -> Result<Option<(String, Vec<i32>)>, anyhow::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        excluded_files: Option<String>,
    }
    let row: Option<Row> =
        sqlx::query_as("SELECT id, excluded_files FROM torrents WHERE info_hash = ?")
            .bind(info_hash)
            .fetch_optional(pool)
            .await?;

    Ok(row.map(|r| {
        let excluded = r
            .excluded_files
            .and_then(|s| serde_json::from_str::<Vec<i32>>(s.trim()).ok())
            .unwrap_or_default();
        (r.id, excluded)
    }))
}

/// Row for upserting a single torrent file.
pub struct TorrentFileRow {
    pub file_index: i32,
    pub file_path: String,
    pub relative_path: String,
    pub file_size: i64,
    pub downloaded_bytes: i64,
    pub progress: f64,
    pub is_excluded: bool,
}

/// Replace all torrent_files for a torrent with the given list (delete then insert).
pub async fn upsert_torrent_files(
    pool: &Database,
    torrent_id: &str,
    files: &[TorrentFileRow],
) -> Result<(), anyhow::Error> {
    sqlx::query("DELETE FROM torrent_files WHERE torrent_id = ?")
        .bind(torrent_id)
        .execute(pool)
        .await?;

    let now = chrono::Utc::now();
    let ts = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    for f in files {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"INSERT INTO torrent_files (id, torrent_id, file_index, file_path, relative_path, file_size, downloaded_bytes, progress, is_excluded, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(torrent_id)
        .bind(f.file_index)
        .bind(&f.file_path)
        .bind(&f.relative_path)
        .bind(f.file_size)
        .bind(f.downloaded_bytes)
        .bind(f.progress)
        .bind(f.is_excluded)
        .bind(&ts)
        .bind(&ts)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Record returned for resuming torrents (has magnet_uri).
pub struct ResumableRecord {
    pub info_hash: String,
    pub name: String,
    pub magnet_uri: Option<String>,
}

/// List torrents that can be resumed (have magnet_uri and are not completed).
pub async fn list_resumable(pool: &Database) -> Result<Vec<ResumableRecord>, anyhow::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        info_hash: String,
        name: String,
        magnet_uri: Option<String>,
    }
    let rows = sqlx::query_as::<_, Row>(
        r#"SELECT info_hash, name, magnet_uri FROM torrents WHERE magnet_uri IS NOT NULL AND state NOT IN ('completed', 'seeding')"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ResumableRecord {
            info_hash: r.info_hash,
            name: r.name,
            magnet_uri: r.magnet_uri,
        })
        .collect())
}

/// Sync all session torrents into the database (upsert by info_hash).
pub async fn sync_session_to_database(
    session: &std::sync::Arc<librqbit::Session>,
    pool: &Database,
    config: &super::TorrentServiceConfig,
) -> Result<(), anyhow::Error> {
    use librqbit::TorrentStatsState;

    let fallback_user_id = match get_default_user_id(pool).await? {
        Some(u) => u,
        None => {
            tracing::info!("No user found in database, skipping session sync");
            return Ok(());
        }
    };

    let session_torrents: Vec<(usize, std::sync::Arc<librqbit::ManagedTorrent>)> =
        session.with_torrents(|iter| iter.map(|(id, h)| (id, h.clone())).collect());

    for (_id, handle) in session_torrents {
        let info_hash = super::get_info_hash_hex(&handle);
        let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
        let stats = handle.stats();
        let progress = stats.progress_bytes as f64 / stats.total_bytes.max(1) as f64;
        let state = match &stats.state {
            TorrentStatsState::Paused => "paused",
            TorrentStatsState::Error => "error",
            TorrentStatsState::Live if progress >= 1.0 => "seeding",
            TorrentStatsState::Live => "downloading",
            TorrentStatsState::Initializing => "queued",
        };

        if let Err(e) = upsert_from_session(
            pool,
            &info_hash,
            &name,
            state,
            progress,
            stats.total_bytes as i64,
            stats.progress_bytes as i64,
            stats.uploaded_bytes as i64,
            &config.download_dir.to_string_lossy(),
            fallback_user_id,
        )
        .await
        {
            tracing::warn!(error = %e, "Failed to sync torrent to database");
        }

        if progress >= 1.0 {
            if let Err(e) = mark_completed(pool, &info_hash).await {
                tracing::warn!(error = %e, "Failed to mark torrent as completed");
            }
        }

        // Sync torrent_files for this torrent
        if let Ok(Some((torrent_id, excluded_files))) = get_torrent_id_and_excluded(pool, &info_hash).await
        {
            if let Some(metadata) = handle.metadata.load_full() {
                let mut rows = Vec::with_capacity(metadata.file_infos.len());
                for (idx, file_info) in metadata.file_infos.iter().enumerate() {
                    let file_progress = stats.file_progress.get(idx).copied().unwrap_or(0);
                    let size = file_info.len;
                    let progress_ratio = if size > 0 {
                        (file_progress as f64 / size as f64).min(1.0)
                    } else {
                        0.0
                    };
                    let relative_path = file_info.relative_filename.to_string_lossy().to_string();
                    let full_path = if metadata.file_infos.len() == 1 {
                        config
                            .download_dir
                            .join(&relative_path)
                            .to_string_lossy()
                            .to_string()
                    } else {
                        config
                            .download_dir
                            .join(&name)
                            .join(&relative_path)
                            .to_string_lossy()
                            .to_string()
                    };
                    let is_excluded = excluded_files.contains(&(idx as i32));
                    rows.push(TorrentFileRow {
                        file_index: idx as i32,
                        file_path: full_path,
                        relative_path,
                        file_size: size as i64,
                        downloaded_bytes: file_progress as i64,
                        progress: progress_ratio,
                        is_excluded,
                    });
                }
                if let Err(e) = upsert_torrent_files(pool, &torrent_id, &rows).await {
                    tracing::warn!(error = %e, info_hash = %info_hash, "Failed to sync torrent files to database");
                }
            }
        }
    }

    Ok(())
}

/// Restore torrents from DB (list_resumable and add to session).
pub async fn restore_from_database(
    session: &std::sync::Arc<librqbit::Session>,
    pool: &Database,
) -> Result<(), anyhow::Error> {
    let records = list_resumable(pool).await?;
    tracing::info!(count = records.len(), "Restoring torrents from database");

    for record in records {
        if let Some(magnet) = &record.magnet_uri {
            match session
                .add_torrent(
                    AddTorrent::from_url(magnet),
                    Some(add_torrent_opts()),
                )
                .await
            {
                Ok(_) => {
                    tracing::info!(name = %record.name, "Restored torrent");
                }
                Err(e) => {
                    tracing::warn!(info_hash = %record.info_hash, error = %e, "Failed to restore torrent");
                    let _ = update_state(pool, &record.info_hash, "error").await;
                }
            }
        }
    }

    Ok(())
}
