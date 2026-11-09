//! Moving the download folder (Q101): like moving a library, the person copies the files first
//! (keeping them seeding from the old place meanwhile), Librarian checks the copy, then points
//! every torrent at the new place and rewrites the records that name a download path.
//!
//! *Plan* (read only): every file of every torrent stored under the old folder is looked for at
//! the same place under the new one, by size. *Apply*: a marker app setting is written
//! (`torrent.downloads_move`), each torrent under the old folder is removed from the client
//! (files kept) and added again from its own metadata with its folder under the new one (its
//! wanted files and paused state kept; the client checks the copied files, so nothing that is
//! there is downloaded again), the records are rewritten (torrent save paths, torrent file
//! paths, pending matches and file records of downloads), `torrent.download_dir` is set, and the
//! marker cleared. Every step skips what is already done, so a move that stopped is finished
//! by running it again, and the server resumes a marked move when it starts. The client is then
//! restarted so new downloads go to the new folder. The old files are never deleted.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::Database;
use crate::graphql::entities::{
    MediaFile, MediaFileWhereInput, PendingFileMatch, PendingFileMatchWhereInput, Torrent,
    TorrentFile, TorrentFileWhereInput,
};
use crate::services::graphql::LibrarianSchema;
use crate::services::library_storage::{StorageKind, StorageLocation, library_storage};
use graphql_orm::graphql::filters::StringFilter;

use super::database::{self, execute_mutation, system_auth_user};

/// The app setting that marks a move under way.
pub const MARKER_KEY: &str = "torrent.downloads_move";
/// How many file names a plan lists of each kind.
const LISTED: usize = 50;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveMarker {
    pub old_root: String,
    pub new_root: String,
    pub move_torrents: bool,
    pub started_at: String,
}

/// What moving the download folder would find. Nothing is changed.
#[derive(Clone, Debug, Default)]
pub struct DownloadsMovePlan {
    pub old_path: String,
    pub new_path: String,
    pub torrent_count: usize,
    pub file_count: usize,
    pub matched: usize,
    pub missing: usize,
    pub missing_files: Vec<String>,
    pub changed: usize,
    pub changed_files: Vec<String>,
    /// Why the move cannot be made at all.
    pub blocked: Option<String>,
}

impl DownloadsMovePlan {
    pub fn can_apply(&self) -> bool {
        self.blocked.is_none() && self.missing == 0 && self.changed == 0
    }
}

/// A folder as it is kept: an `smb://` address normalised, a local path without a trailing slash.
pub fn normalize_root(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("Choose the new download folder");
    }
    let location = StorageLocation::parse(trimmed).map_err(|error| {
        anyhow::anyhow!("'{trimmed}' is not a folder Librarian can use: {error}")
    })?;
    let normalized = location.to_path_string();
    if location.kind() == StorageKind::Local && !Path::new(&normalized).is_absolute() {
        bail!("Use a full path for a folder on this server (it starts with /)");
    }
    Ok(normalized.trim_end_matches('/').to_string())
}

/// The part of `path` below `root`, if it is under it (`""` for the root itself).
pub fn relative_under(root: &str, path: &str) -> Option<String> {
    let root = root.trim_end_matches('/');
    if path == root {
        return Some(String::new());
    }
    path.strip_prefix(root)
        .and_then(|rest| rest.strip_prefix('/'))
        .map(str::to_string)
}

fn join(root: &str, relative: &str) -> String {
    if relative.is_empty() {
        root.to_string()
    } else {
        format!("{}/{relative}", root.trim_end_matches('/'))
    }
}

fn eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

fn under(root: &str) -> StringFilter {
    StringFilter {
        starts_with: Some(root.trim_end_matches('/').to_string()),
        ..Default::default()
    }
}

/// The torrents stored under `root`, by their saved path.
async fn torrents_under(db: &Database, root: &str) -> Result<Vec<Torrent>> {
    Ok(Torrent::query(db.pool())
        .filter(crate::graphql::entities::TorrentWhereInput {
            save_path: Some(under(root)),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .filter(|torrent| relative_under(root, &torrent.save_path).is_some())
        .collect())
}

/// Compare the torrents under `old_root` with their copy under `new_path`.
pub async fn plan(db: &Database, old_root: &str, new_path: &str) -> Result<DownloadsMovePlan> {
    let old_root = normalize_root(old_root)?;
    let new_root = normalize_root(new_path)?;
    let mut plan = DownloadsMovePlan {
        old_path: old_root.clone(),
        new_path: new_root.clone(),
        ..Default::default()
    };
    if new_root == old_root {
        plan.blocked = Some("That is the download folder already".to_string());
        return Ok(plan);
    }
    if relative_under(&old_root, &new_root).is_some()
        || relative_under(&new_root, &old_root).is_some()
    {
        plan.blocked = Some(
            "The new folder cannot be inside the old one, or the old one inside it".to_string(),
        );
        return Ok(plan);
    }
    match library_storage().stat(new_root.as_str()).await {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            plan.blocked = Some("The new location is not a folder".to_string());
            return Ok(plan);
        }
        Err(error) => {
            plan.blocked = Some(format!("The new folder cannot be read: {error}"));
            return Ok(plan);
        }
    }
    let torrents = torrents_under(db, &old_root).await?;
    plan.torrent_count = torrents.len();
    for torrent in torrents {
        let files = TorrentFile::query(db.pool())
            .filter(TorrentFileWhereInput {
                torrent_id: Some(eq(&torrent.id)),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        for file in files {
            let Some(relative) = relative_under(&old_root, &file.file_path) else {
                continue;
            };
            // Only what is on disk now is expected in the copy (files a partial download skips
            // were never written).
            let old_size = match library_storage().stat(file.file_path.as_str()).await {
                Ok(meta) if meta.is_file() => meta.size,
                _ => continue,
            };
            plan.file_count += 1;
            match library_storage().stat(join(&new_root, &relative)).await {
                Ok(meta) if meta.is_file() && meta.size == old_size => plan.matched += 1,
                Ok(_) => {
                    plan.changed += 1;
                    if plan.changed_files.len() < LISTED {
                        plan.changed_files.push(relative);
                    }
                }
                Err(error) if error.is_not_found() => {
                    plan.missing += 1;
                    if plan.missing_files.len() < LISTED {
                        plan.missing_files.push(relative);
                    }
                }
                Err(error) => {
                    plan.blocked = Some(format!("The new folder could not be checked: {error}"));
                    return Ok(plan);
                }
            }
        }
    }
    Ok(plan)
}

async fn update(
    schema: &LibrarianSchema,
    entity: &str,
    id: &str,
    input: serde_json::Value,
) -> Result<()> {
    let document = format!(
        "mutation Move($id: String!, $input: Update{entity}Input!) {{ Moved: update{entity}(id: $id, input: $input) {{ success error }} }}"
    );
    execute_mutation(
        schema,
        &system_auth_user(),
        &document,
        json!({ "id": id, "input": input }),
    )
    .await
    .with_context(|| format!("Could not rewrite a {entity} record"))?;
    Ok(())
}

/// What the record rewrite changed.
#[derive(Clone, Debug, Default)]
pub struct Rewritten {
    pub torrents: usize,
    pub files: usize,
}

/// Rewrite every record that names a path under `old_root` to the same place under `new_root`.
pub async fn rewrite_records(
    db: &Database,
    schema: &LibrarianSchema,
    old_root: &str,
    new_root: &str,
) -> Result<Rewritten> {
    let mut done = Rewritten::default();
    for torrent in torrents_under(db, old_root).await? {
        let relative = relative_under(old_root, &torrent.save_path).unwrap_or_default();
        let mut input = json!({ "savePath": join(new_root, &relative) });
        if let Some(download) = torrent.download_path.as_deref()
            && let Some(relative) = relative_under(old_root, download)
        {
            input["downloadPath"] = json!(join(new_root, &relative));
        }
        update(schema, "Torrent", &torrent.id, input).await?;
        done.torrents += 1;
    }
    let files = TorrentFile::query(db.pool())
        .filter(TorrentFileWhereInput {
            file_path: Some(under(old_root)),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    for file in files {
        if let Some(relative) = relative_under(old_root, &file.file_path) {
            update(
                schema,
                "TorrentFile",
                &file.id,
                json!({ "filePath": join(new_root, &relative) }),
            )
            .await?;
            done.files += 1;
        }
    }
    let pending = PendingFileMatch::query(db.pool())
        .filter(PendingFileMatchWhereInput {
            source_path: Some(under(old_root)),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    for row in pending {
        if let Some(relative) = relative_under(old_root, &row.source_path) {
            update(
                schema,
                "PendingFileMatch",
                &row.id,
                json!({ "sourcePath": join(new_root, &relative) }),
            )
            .await?;
        }
    }
    // File records of downloads themselves (a release being identified), never library files:
    // a library cannot be inside the download folder.
    let records = MediaFile::query(db.pool())
        .filter(MediaFileWhereInput {
            path: Some(under(old_root)),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    for record in records {
        if let Some(relative) = relative_under(old_root, &record.path) {
            update(
                schema,
                "MediaFile",
                &record.id,
                json!({ "path": join(new_root, &relative) }),
            )
            .await?;
        }
    }
    Ok(done)
}

pub async fn read_marker(db: &Database) -> Option<MoveMarker> {
    database::get_setting_string(db, MARKER_KEY)
        .await
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

pub async fn write_marker(db: &Database, marker: &MoveMarker) -> Result<()> {
    database::put_setting_value(
        db,
        MARKER_KEY,
        &serde_json::to_string(marker)?,
        "torrent",
        "A move of the download folder that is under way",
    )
    .await
}

pub async fn clear_marker(db: &Database) -> Result<()> {
    // An empty value reads as no marker (`get_setting_string` skips blank values).
    database::put_setting_value(
        db,
        MARKER_KEY,
        "",
        "torrent",
        "No move of the download folder is under way",
    )
    .await
}

/// The new folder of a torrent stored at `folder` under `old_root`.
pub fn moved_folder(old_root: &str, new_root: &str, folder: &Path) -> Option<PathBuf> {
    relative_under(old_root, &folder.to_string_lossy())
        .map(|relative| PathBuf::from(join(new_root, &relative)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_under_a_root_keeps_its_relative_part() {
        assert_eq!(
            relative_under("/data/downloads", "/data/downloads/A/b.mkv").as_deref(),
            Some("A/b.mkv")
        );
        assert_eq!(
            relative_under("/data/downloads/", "/data/downloads").as_deref(),
            Some("")
        );
        assert_eq!(
            relative_under("/data/downloads", "/data/downloads2/a.mkv"),
            None
        );
        assert_eq!(
            moved_folder(
                "/data/downloads",
                "smb://nas/media/Downloads",
                Path::new("/data/downloads/Show.S01")
            ),
            Some(PathBuf::from("smb://nas/media/Downloads/Show.S01"))
        );
        assert_eq!(
            moved_folder(
                "/data/downloads",
                "smb://nas/media/Downloads",
                Path::new("/data/downloads")
            ),
            Some(PathBuf::from("smb://nas/media/Downloads"))
        );
    }

    #[test]
    fn folders_are_kept_normalised() {
        assert_eq!(
            normalize_root("/data/downloads/").unwrap(),
            "/data/downloads"
        );
        assert_eq!(
            normalize_root("smb://NAS/Media/Downloads/").unwrap(),
            "smb://nas/media/Downloads"
        );
        assert!(normalize_root("downloads").is_err());
        assert!(normalize_root("  ").is_err());
    }
}
