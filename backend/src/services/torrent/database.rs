//! Database helpers for the torrent service.
//! Uses the pool directly; table/column names must match the GraphQL entity schema (snake_case in DB).

use super::{add_torrent_opts, get_info_hash_hex};

use async_graphql::{Request, Variables};
use std::fs;
use uuid::Uuid;

use crate::db::Database;
use crate::graphql::entities::{AppSetting, Torrent, User};
use crate::services::graphql::{AuthUser, LibrarianSchema};
use librqbit::{AddTorrent, AddTorrentResponse};

fn now_iso_string() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

fn open_fd_diagnostics() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let open_fd_count = fs::read_dir("/proc/self/fd").ok()?.count();
        let limits = fs::read_to_string("/proc/self/limits").ok()?;
        let max_line = limits
            .lines()
            .find(|line| line.starts_with("Max open files"))?;
        let parts: Vec<&str> = max_line.split_whitespace().collect();
        if parts.len() < 5 {
            return None;
        }
        let soft = parts.get(3)?;
        let hard = parts.get(4)?;
        Some(format!(
            "open_fds={}, max_open_files_soft={}, max_open_files_hard={}",
            open_fd_count, soft, hard
        ))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
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

fn is_supported_torrent_media_extension(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());

    match ext.as_deref() {
        // Video containers
        Some("mkv" | "mp4" | "avi" | "m4v" | "mov" | "wmv" | "flv" | "webm" | "mpeg" | "mpg" | "ts" | "m2ts")
        // Audio
        | Some("mp3" | "flac" | "m4a" | "m4b" | "aac" | "ogg" | "opus" | "wav" | "wma" | "aiff" | "alac" | "ape" | "dsf" | "dff")
        // Subtitles
        | Some("srt" | "ass" | "ssa" | "sub" | "vtt" | "ttml" | "smi" | "sami" | "idx" | "sup")
            => true,
        _ => false,
    }
}

async fn find_media_file_id_by_path(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    path: &str,
) -> Result<Option<String>, anyhow::Error> {
    let data = execute_mutation(
        schema,
        auth_user,
        r#"query MediaFileByPath($Path: String!) {
            MediaFiles(Where: { Path: { Eq: $Path } }, Page: { Limit: 1 }) {
                Edges { Node { Id } }
            }
        }"#,
        serde_json::json!({ "Path": path }),
    )
    .await?;

    Ok(data
        .get("MediaFiles")
        .and_then(|v| v.get("Edges"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|edge| edge.get("Node"))
        .and_then(|node| node.get("Id"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string()))
}

async fn create_unmatched_media_file(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    path: &str,
    relative_path: &str,
    size: i64,
) -> Result<Option<String>, anyhow::Error> {
    let added_at = now_iso_string();
    let original_name = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    let data = execute_mutation(
        schema,
        auth_user,
        r#"mutation CreateMediaFileFromTorrent($Input: CreateMediaFileInput!) {
            CreateMediaFile(Input: $Input) {
                Success
                Error
                MediaFile { Id }
            }
        }"#,
        serde_json::json!({
            "Input": {
                "LibraryId": serde_json::Value::Null,
                "Path": path,
                "RelativePath": relative_path,
                "OriginalName": original_name,
                "Size": size,
                "IsHdr": false,
                "AddedAt": added_at,
                "Metadata": serde_json::json!({
                    "SourceType": "torrent",
                    "UnmatchedReason": "Completed torrent file awaiting match"
                }).to_string()
            }
        }),
    )
    .await?;

    let create = data
        .get("CreateMediaFile")
        .ok_or_else(|| anyhow::anyhow!("GraphQL response missing CreateMediaFile"))?;

    let success = create
        .get("Success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let err = create
            .get("Error")
            .and_then(|v| v.as_str())
            .unwrap_or("CreateMediaFile failed")
            .to_string();
        return Err(anyhow::anyhow!(err));
    }

    Ok(create
        .get("MediaFile")
        .and_then(|v| v.get("Id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string()))
}

async fn queue_media_file_analysis(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    media_file_id: &str,
    path: &str,
) -> Result<(), anyhow::Error> {
    let data = execute_mutation(
        schema,
        auth_user,
        r#"mutation AnalyzeMediaFileForTorrent($MediaFileId: String!, $Path: String!) {
            AnalyzeMediaFile(MediaFileId: $MediaFileId, Path: $Path) {
                Success
                Queued
                Message
            }
        }"#,
        serde_json::json!({
            "MediaFileId": media_file_id,
            "Path": path,
        }),
    )
    .await?;

    let analyze = data
        .get("AnalyzeMediaFile")
        .ok_or_else(|| anyhow::anyhow!("GraphQL response missing AnalyzeMediaFile"))?;
    let success = analyze
        .get("Success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if success {
        return Ok(());
    }

    let message = analyze
        .get("Message")
        .and_then(|v| v.as_str())
        .unwrap_or("AnalyzeMediaFile failed")
        .to_string();
    Err(anyhow::anyhow!(message))
}

async fn ensure_completed_file_media_record_and_analyze(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    row: &TorrentFileRow,
) -> Result<(), anyhow::Error> {
    if !is_supported_torrent_media_extension(&row.file_path) {
        return Ok(());
    }

    if row.file_size <= 0 || row.downloaded_bytes < row.file_size {
        return Ok(());
    }

    let media_file_id = match find_media_file_id_by_path(schema, auth_user, &row.file_path).await? {
        Some(id) => id,
        None => {
            match create_unmatched_media_file(
                schema,
                auth_user,
                &row.file_path,
                &row.relative_path,
                row.file_size,
            )
            .await?
            {
                Some(id) => id,
                None => return Ok(()),
            }
        }
    };

    queue_media_file_analysis(schema, auth_user, &media_file_id, &row.file_path).await
}

/// Process completed files for a torrent: create media file records and queue analysis.
///
/// Call this when a torrent finishes downloading (or when manually triggered).
/// Looks up the torrent in the database, builds file rows from the session handle,
/// and calls `ensure_completed_file_media_record_and_analyze` for each eligible file.
pub async fn process_completed_torrent_files(
    session: &std::sync::Arc<librqbit::Session>,
    pool: &Database,
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    info_hash: &str,
    download_dir: &std::path::Path,
) -> Result<(), anyhow::Error> {
    let Some((torrent_id, excluded_files)) = get_torrent_id_and_excluded(pool, info_hash).await?
    else {
        tracing::debug!(
            info_hash = %info_hash,
            "Skipping completed-file processing: torrent not found in database for info_hash={}",
            info_hash
        );
        return Ok(());
    };
    let _ = torrent_id; // used only to verify the torrent exists in DB

    // Find the session handle by info_hash
    let handle = {
        let hash_id = hex_to_info_hash(info_hash)?;
        session
            .get(librqbit::api::TorrentIdOrHash::Hash(hash_id))
            .ok_or_else(|| anyhow::anyhow!("Torrent not found in session: {}", info_hash))?
    };

    let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
    let stats = handle.stats();

    let Some(rows) = build_file_rows(&handle, &stats, &excluded_files, download_dir) else {
        tracing::debug!(
            info_hash = %info_hash,
            "Skipping completed-file processing: no metadata loaded for info_hash={}",
            info_hash
        );
        return Ok(());
    };

    let mut processed = 0usize;
    let mut skipped = 0usize;

    for row in rows.iter().filter(|r| !r.is_excluded) {
        if let Err(e) = ensure_completed_file_media_record_and_analyze(schema, auth_user, row).await
        {
            tracing::warn!(
                error = %e,
                info_hash = %info_hash,
                file_index = row.file_index,
                file_path = %row.file_path,
                "Failed to create/queue media file from completed torrent file: info_hash={}, file_index={}, file_path={}, error={}",
                info_hash,
                row.file_index,
                row.file_path,
                e
            );
        } else {
            processed += 1;
        }
    }
    skipped = rows.iter().filter(|r| r.is_excluded).count();

    tracing::info!(
        info_hash = %info_hash,
        torrent_name = %name,
        processed = processed,
        skipped = skipped,
        "Processed completed torrent files for analysis: info_hash={}, name='{}', processed={}, skipped={}",
        info_hash,
        name,
        processed,
        skipped
    );

    Ok(())
}

/// Parse a hex info_hash string into a 20-byte `Id20`.
fn hex_to_info_hash(hex: &str) -> Result<librqbit::dht::Id20, anyhow::Error> {
    if hex.len() != 40 {
        anyhow::bail!(
            "Invalid info_hash hex length: expected 40, got {}",
            hex.len()
        );
    }
    let mut bytes = [0u8; 20];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk)?;
        bytes[i] = u8::from_str_radix(s, 16)?;
    }
    Ok(librqbit::dht::Id20::new(bytes))
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
    Ok(find_setting(pool, key)
        .await?
        .map(|setting| setting.value)
        .filter(|s| !s.trim().is_empty() && s != "null"))
}

/// Read a value from app_settings. Value is parsed as JSON (e.g. "true", "5", "0" for bool/u16/usize).
pub async fn get_setting<T: serde::de::DeserializeOwned>(
    pool: &Database,
    key: &str,
) -> Result<Option<T>, anyhow::Error> {
    match find_setting(pool, key).await? {
        Some(setting) => {
            let s = setting.value.trim();
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
    Ok(User::query(pool.pool())
        .fetch_all()
        .await?
        .into_iter()
        .min_by(|a, b| a.created_at.cmp(&b.created_at))
        .and_then(|user| Uuid::parse_str(&user.id).ok()))
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
                "AddedAt": ts
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
    if let Some(id) = get_torrent_id_by_info_hash(pool, info_hash).await? {
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
    Ok(find_torrent_by_info_hash(pool, info_hash)
        .await?
        .map(|torrent| (torrent.id, torrent.excluded_files)))
}

async fn get_torrent_id_by_info_hash(
    pool: &Database,
    info_hash: &str,
) -> Result<Option<String>, anyhow::Error> {
    Ok(find_torrent_by_info_hash(pool, info_hash)
        .await?
        .map(|torrent| torrent.id))
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

/// Build `TorrentFileRow` entries from a session handle's metadata and stats.
///
/// Shared by `sync_session_to_database` (periodic sync) and
/// `process_completed_torrent_files` (completion-triggered analysis).
fn build_file_rows(
    handle: &librqbit::ManagedTorrent,
    stats: &librqbit::TorrentStats,
    excluded_files: &[i32],
    download_dir: &std::path::Path,
) -> Option<Vec<TorrentFileRow>> {
    let metadata = handle.metadata.load_full()?;
    let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
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
            download_dir
                .join(&relative_path)
                .to_string_lossy()
                .to_string()
        } else {
            download_dir
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

    Some(rows)
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
                    "IsExcluded": f.is_excluded
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
    Ok(Torrent::query(pool.pool())
        .fetch_all()
        .await?
        .into_iter()
        .filter(|torrent| {
            torrent.magnet_uri.is_some()
                && torrent.state != "completed"
                && torrent.state != "seeding"
        })
        .map(|torrent| ResumableRecord {
            info_hash: torrent.info_hash,
            name: torrent.name,
            magnet_uri: torrent.magnet_uri,
        })
        .collect())
}

async fn find_setting(db: &Database, key: &str) -> Result<Option<AppSetting>, anyhow::Error> {
    Ok(AppSetting::query(db.pool())
        .fetch_all()
        .await?
        .into_iter()
        .find(|setting| setting.key == key))
}

async fn find_torrent_by_info_hash(
    db: &Database,
    info_hash: &str,
) -> Result<Option<Torrent>, anyhow::Error> {
    Ok(Torrent::query(db.pool())
        .fetch_all()
        .await?
        .into_iter()
        .find(|torrent| torrent.info_hash == info_hash))
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
            let fd_diag =
                open_fd_diagnostics().unwrap_or_else(|| "fd_diag=unavailable".to_string());
            tracing::warn!(
                error = %e,
                info_hash = %info_hash,
                torrent_name = %name,
                fd_diag = %fd_diag,
                "Failed to sync torrent to database: info_hash={}, name='{}', error={}, fd_diag={}",
                info_hash,
                name,
                e,
                fd_diag
            );
        }

        if progress >= 1.0 {
            if let Err(e) = mark_completed(pool, schema, auth_user, &info_hash).await {
                let fd_diag =
                    open_fd_diagnostics().unwrap_or_else(|| "fd_diag=unavailable".to_string());
                tracing::warn!(
                    error = %e,
                    info_hash = %info_hash,
                    fd_diag = %fd_diag,
                    "Failed to mark torrent as completed: info_hash={}, error={}, fd_diag={}",
                    info_hash,
                    e,
                    fd_diag
                );
            }
        }

        // Sync torrent_files for this torrent
        if let Ok(Some((torrent_id, excluded_files))) =
            get_torrent_id_and_excluded(pool, &info_hash).await
        {
            if let Some(rows) =
                build_file_rows(&handle, &stats, &excluded_files, &config.download_dir)
            {
                if let Err(e) = upsert_torrent_files(schema, auth_user, &torrent_id, &rows).await {
                    let fd_diag =
                        open_fd_diagnostics().unwrap_or_else(|| "fd_diag=unavailable".to_string());
                    tracing::warn!(
                        error = %e,
                        info_hash = %info_hash,
                        row_count = rows.len(),
                        fd_diag = %fd_diag,
                        "Failed to sync torrent files to database: info_hash={}, row_count={}, error={}, fd_diag={}",
                        info_hash,
                        rows.len(),
                        e,
                        fd_diag
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
    max_concurrent: usize,
    initial_active_downloads: usize,
) -> Result<(), anyhow::Error> {
    let records = list_resumable(pool).await?;
    let mut active_downloads = initial_active_downloads;
    tracing::info!(
        count = records.len(),
        "Restoring torrents from database: resumable_torrent_count={}",
        records.len()
    );

    for record in records {
        if let Some(magnet) = &record.magnet_uri {
            let start_paused = max_concurrent > 0 && active_downloads >= max_concurrent;
            match session
                .add_torrent(
                    AddTorrent::from_url(magnet),
                    Some(add_torrent_opts(start_paused)),
                )
                .await
            {
                Ok(AddTorrentResponse::Added(_, _)) => {
                    if !start_paused {
                        active_downloads += 1;
                    }
                    tracing::info!(
                        name = %record.name,
                        info_hash = %record.info_hash,
                        paused = start_paused,
                        "Restored torrent from database: name='{}', info_hash={}, paused={}",
                        record.name,
                        record.info_hash,
                        start_paused
                    );
                }
                Ok(AddTorrentResponse::AlreadyManaged(_, _)) => {
                    tracing::info!(
                        name = %record.name,
                        info_hash = %record.info_hash,
                        "Skipped restoring torrent already managed by session: name='{}', info_hash={}",
                        record.name,
                        record.info_hash
                    );
                }
                Ok(AddTorrentResponse::ListOnly(_)) => {
                    tracing::warn!(
                        info_hash = %record.info_hash,
                        "Unexpected list-only response while restoring torrent: info_hash={}",
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
