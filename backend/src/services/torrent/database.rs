//! Database helpers for the torrent service.
//! Uses the pool directly; table/column names must match the GraphQL entity schema (snake_case in DB).

use super::{add_torrent_opts, get_info_hash_hex};

use async_graphql::{Request, Variables};
use uuid::Uuid;

use crate::db::Database;
use crate::services::graphql::{AuthUser, LibrarianSchema};
use librqbit::AddTorrent;

fn now_iso_string() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

async fn execute_mutation(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    query: &str,
    variables: serde_json::Value,
) -> Result<serde_json::Value, anyhow::Error> {
    let request = Request::new(query)
        .variables(Variables::from_json(variables))
        .data(auth_user.clone());
    let response = schema.execute(request).await;
    if !response.errors.is_empty() {
        let msg = response
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(anyhow::anyhow!(msg));
    }
    let data = serde_json::to_value(&response.data)?;
    Ok(data)
}

fn ensure_mutation_success(data: &serde_json::Value, field: &str) -> Result<(), anyhow::Error> {
    let result = data
        .get(field)
        .ok_or_else(|| anyhow::anyhow!("GraphQL response missing {}", field))?;
    let success = result
        .get("Success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if success {
        Ok(())
    } else {
        let error = result
            .get("Error")
            .and_then(|v| v.as_str())
            .unwrap_or("Mutation failed")
            .to_string();
        Err(anyhow::anyhow!(error))
    }
}

fn ensure_bulk_delete_success(data: &serde_json::Value, field: &str) -> Result<(), anyhow::Error> {
    let result = data
        .get(field)
        .ok_or_else(|| anyhow::anyhow!("GraphQL response missing {}", field))?;
    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if success {
        Ok(())
    } else {
        let error = result
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Mutation failed")
            .to_string();
        Err(anyhow::anyhow!(error))
    }
}

/// Read a string value from app_settings (raw value, not JSON).
/// Filters out empty strings and the literal "null" string (legacy seed data issue).
pub async fn get_setting_string(
    pool: &Database,
    key: &str,
) -> Result<Option<String>, anyhow::Error> {
    // Prefer most recently updated row to handle legacy duplicate keys.
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM app_settings WHERE key = ? ORDER BY updated_at DESC, created_at DESC, rowid DESC LIMIT 1",
    )
        .bind(key)
        .fetch_optional(pool)
        .await?;

    Ok(row
        .map(|(s,)| s)
        .filter(|s| !s.trim().is_empty() && s != "null"))
}

/// Read a value from app_settings. Value is parsed as JSON (e.g. "true", "5", "0" for bool/u16/usize).
pub async fn get_setting<T: serde::de::DeserializeOwned>(
    pool: &Database,
    key: &str,
) -> Result<Option<T>, anyhow::Error> {
    // Prefer most recently updated row to handle legacy duplicate keys.
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM app_settings WHERE key = ? ORDER BY updated_at DESC, created_at DESC, rowid DESC LIMIT 1",
    )
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match row {
        Some((s,)) => {
            let s = s.trim();
            if s.is_empty() {
                return Ok(None);
            }
            let v: T = serde_json::from_str(s)
                .map_err(|e| anyhow::anyhow!("app_settings key {}: {}", key, e))?;
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
    let row: Option<Row> = sqlx::query_as("SELECT id FROM users ORDER BY created_at ASC LIMIT 1")
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|r| Uuid::parse_str(&r.id).ok()))
}

/// Insert a new torrent record.
pub async fn create_torrent(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    info_hash: &str,
    magnet_uri: Option<&str>,
    name: &str,
    save_path: &str,
    state: &str,
    progress: f64,
    total_bytes: i64,
    downloaded_bytes: i64,
    uploaded_bytes: i64,
) -> Result<(), anyhow::Error> {
    let ts = now_iso_string();
    let data = execute_mutation(
        schema,
        auth_user,
        r#"mutation CreateTorrent($input: CreateTorrentInput!) {
            CreateTorrent(Input: $input) { Success Error }
        }"#,
        serde_json::json!({
            "input": {
                "UserId": auth_user.user_id.clone(),
                "InfoHash": info_hash,
                "MagnetUri": magnet_uri,
                "Name": name,
                "State": state,
                "Progress": progress,
                "TotalBytes": total_bytes,
                "DownloadedBytes": downloaded_bytes,
                "UploadedBytes": uploaded_bytes,
                "SavePath": save_path,
                "ExcludedFiles": [],
                "AddedAt": ts,
                "CreatedAt": ts,
                "UpdatedAt": ts
            }
        }),
    )
    .await?;

    ensure_mutation_success(&data, "CreateTorrent")
}

/// Upsert a torrent from session state (by info_hash).
pub async fn upsert_from_session(
    pool: &Database,
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    info_hash: &str,
    name: &str,
    state: &str,
    progress: f64,
    total_bytes: i64,
    downloaded_bytes: i64,
    uploaded_bytes: i64,
    save_path: &str,
) -> Result<(), anyhow::Error> {
    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM torrents WHERE info_hash = ?")
        .bind(info_hash)
        .fetch_optional(pool)
        .await?;

    if let Some((id,)) = existing {
        let data = execute_mutation(
            schema,
            auth_user,
            r#"mutation UpdateTorrent($id: String!, $input: UpdateTorrentInput!) {
                UpdateTorrent(Id: $id, Input: $input) { Success Error }
            }"#,
            serde_json::json!({
                "id": id,
                "input": {
                    "Name": name,
                    "State": state,
                    "Progress": progress,
                    "TotalBytes": total_bytes,
                    "DownloadedBytes": downloaded_bytes,
                    "UploadedBytes": uploaded_bytes,
                    "SavePath": save_path
                }
            }),
        )
        .await?;
        ensure_mutation_success(&data, "UpdateTorrent")
    } else {
        create_torrent(
            schema,
            auth_user,
            info_hash,
            None,
            name,
            save_path,
            state,
            progress,
            total_bytes,
            downloaded_bytes,
            uploaded_bytes,
        )
        .await
    }
}

/// Update progress/state for an existing torrent.
pub async fn update_progress(
    pool: &Database,
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    info_hash: &str,
    state: &str,
    progress: f64,
    downloaded_bytes: i64,
    uploaded_bytes: i64,
) -> Result<(), anyhow::Error> {
    let torrent_id = get_torrent_id_by_info_hash(pool, info_hash).await?;
    let Some(torrent_id) = torrent_id else {
        return Ok(());
    };

    let data = execute_mutation(
        schema,
        auth_user,
        r#"mutation UpdateTorrent($id: String!, $input: UpdateTorrentInput!) {
            UpdateTorrent(Id: $id, Input: $input) { Success Error }
        }"#,
        serde_json::json!({
            "id": torrent_id,
            "input": {
                "State": state,
                "Progress": progress,
                "DownloadedBytes": downloaded_bytes,
                "UploadedBytes": uploaded_bytes
            }
        }),
    )
    .await?;
    ensure_mutation_success(&data, "UpdateTorrent")
}

pub async fn mark_completed(
    pool: &Database,
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    info_hash: &str,
) -> Result<(), anyhow::Error> {
    let torrent_id = get_torrent_id_by_info_hash(pool, info_hash).await?;
    let Some(torrent_id) = torrent_id else {
        return Ok(());
    };
    let ts = now_iso_string();
    let data = execute_mutation(
        schema,
        auth_user,
        r#"mutation UpdateTorrent($id: String!, $input: UpdateTorrentInput!) {
            UpdateTorrent(Id: $id, Input: $input) { Success Error }
        }"#,
        serde_json::json!({
            "id": torrent_id,
            "input": {
                "State": "seeding",
                "Progress": 1.0,
                "CompletedAt": ts
            }
        }),
    )
    .await?;
    ensure_mutation_success(&data, "UpdateTorrent")
}

pub async fn update_state(
    pool: &Database,
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    info_hash: &str,
    state: &str,
) -> Result<(), anyhow::Error> {
    let torrent_id = get_torrent_id_by_info_hash(pool, info_hash).await?;
    let Some(torrent_id) = torrent_id else {
        return Ok(());
    };
    let data = execute_mutation(
        schema,
        auth_user,
        r#"mutation UpdateTorrent($id: String!, $input: UpdateTorrentInput!) {
            UpdateTorrent(Id: $id, Input: $input) { Success Error }
        }"#,
        serde_json::json!({
            "id": torrent_id,
            "input": {
                "State": state
            }
        }),
    )
    .await?;
    ensure_mutation_success(&data, "UpdateTorrent")
}

/// Delete a torrent record by info_hash.
pub async fn delete_torrent(
    pool: &Database,
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    info_hash: &str,
) -> Result<(), anyhow::Error> {
    let torrent_id = get_torrent_id_by_info_hash(pool, info_hash).await?;
    let Some(torrent_id) = torrent_id else {
        return Ok(());
    };
    let data = execute_mutation(
        schema,
        auth_user,
        r#"mutation DeleteTorrent($id: String!) {
            DeleteTorrent(Id: $id) { Success Error }
        }"#,
        serde_json::json!({
            "id": torrent_id
        }),
    )
    .await?;
    ensure_mutation_success(&data, "DeleteTorrent")
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

async fn get_torrent_id_by_info_hash(
    pool: &Database,
    info_hash: &str,
) -> Result<Option<String>, anyhow::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM torrents WHERE info_hash = ?")
        .bind(info_hash)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(id,)| id))
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
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    torrent_id: &str,
    files: &[TorrentFileRow],
) -> Result<(), anyhow::Error> {
    let data = execute_mutation(
        schema,
        auth_user,
        r#"mutation DeleteTorrentFiles($where: TorrentFileWhereInput!) {
            DeleteTorrentFiles(Where: $where) { success error DeletedCount }
        }"#,
        serde_json::json!({
            "where": {
                "TorrentId": {
                    "Eq": torrent_id
                }
            }
        }),
    )
    .await?;
    let _ = ensure_bulk_delete_success(&data, "DeleteTorrentFiles");

    let ts = now_iso_string();
    for f in files {
        let data = execute_mutation(
            schema,
            auth_user,
            r#"mutation CreateTorrentFile($input: CreateTorrentFileInput!) {
                CreateTorrentFile(Input: $input) { Success Error }
            }"#,
            serde_json::json!({
                "input": {
                    "TorrentId": torrent_id,
                    "FileIndex": f.file_index,
                    "FilePath": f.file_path.clone(),
                    "RelativePath": f.relative_path.clone(),
                    "FileSize": f.file_size,
                    "DownloadedBytes": f.downloaded_bytes,
                    "Progress": f.progress,
                    "IsExcluded": f.is_excluded,
                    "CreatedAt": ts,
                    "UpdatedAt": ts
                }
            }),
        )
        .await?;
        ensure_mutation_success(&data, "CreateTorrentFile")?;
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
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    config: &super::TorrentServiceConfig,
) -> Result<(), anyhow::Error> {
    use librqbit::TorrentStatsState;

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
            schema,
            auth_user,
            &info_hash,
            &name,
            state,
            progress,
            stats.total_bytes as i64,
            stats.progress_bytes as i64,
            stats.uploaded_bytes as i64,
            &config.download_dir.to_string_lossy(),
        )
        .await
        {
            tracing::warn!(
                error = %e,
                info_hash = %info_hash,
                torrent_name = %name,
                "Failed to sync torrent to database: info_hash={}, name='{}', error={}",
                info_hash,
                name,
                e
            );
        }

        if progress >= 1.0 {
            if let Err(e) = mark_completed(pool, schema, auth_user, &info_hash).await {
                tracing::warn!(
                    error = %e,
                    info_hash = %info_hash,
                    "Failed to mark torrent as completed: info_hash={}, error={}",
                    info_hash,
                    e
                );
            }
        }

        // Sync torrent_files for this torrent
        if let Ok(Some((torrent_id, excluded_files))) =
            get_torrent_id_and_excluded(pool, &info_hash).await
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
                if let Err(e) = upsert_torrent_files(schema, auth_user, &torrent_id, &rows).await {
                    tracing::warn!(
                        error = %e,
                        info_hash = %info_hash,
                        row_count = rows.len(),
                        "Failed to sync torrent files to database: info_hash={}, row_count={}, error={}",
                        info_hash,
                        rows.len(),
                        e
                    );
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
    schema: &LibrarianSchema,
    auth_user: Option<&AuthUser>,
) -> Result<(), anyhow::Error> {
    let records = list_resumable(pool).await?;
    tracing::info!(
        count = records.len(),
        "Restoring torrents from database: resumable_torrent_count={}",
        records.len()
    );

    for record in records {
        if let Some(magnet) = &record.magnet_uri {
            match session
                .add_torrent(AddTorrent::from_url(magnet), Some(add_torrent_opts()))
                .await
            {
                Ok(_) => {
                    tracing::info!(
                        name = %record.name,
                        info_hash = %record.info_hash,
                        "Restored torrent from database: name='{}', info_hash={}",
                        record.name,
                        record.info_hash
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        info_hash = %record.info_hash,
                        error = %e,
                        "Failed to restore torrent from database: info_hash={}, error={}",
                        record.info_hash,
                        e
                    );
                    if let Some(user) = auth_user {
                        let _ = update_state(pool, schema, user, &record.info_hash, "error").await;
                    }
                }
            }
        }
    }

    Ok(())
}
