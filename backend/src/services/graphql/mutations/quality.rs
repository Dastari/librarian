//! Quality profile custom mutations (`docs/tier1-features-plan.md` §2):
//! - `evaluateMediaFileQuality`: resolve the effective profile for a media
//!   file and (re)compute/persist its `quality_status`.
//! - `recomputeLibraryQualityStatus`: bulk re-evaluate every media file in a
//!   library, e.g. after changing which profile is assigned.
//! - `approveQualityUpgrade`: Q38 — the user-facing "approve" action for a
//!   quality-upgrade notification (see
//!   `LibraryScanService::maybe_notify_quality_upgrade`, wired from Q44's
//!   duplicate-torrent handling). Replaces the on-disk file with the
//!   candidate that triggered the notification, queues re-analysis, and
//!   resolves the notification. Dismissing an upgrade notification needs no
//!   custom mutation — the frontend already resolves it via the generic
//!   `updateNotification` mutation (`resolution: DISMISSED`).

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Context as AnyhowContext;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use async_graphql::{Context, Object, Result, SimpleObject};
use graphql_orm::graphql::filters::StringFilter;

use crate::db::Database;
use crate::graphql::entities::{
    Album, Audiobook, Chapter, Episode, Library, MediaFile, MediaFileWhereInput, Movie,
    Notification, PendingFileMatch, PendingFileMatchWhereInput, Show, Torrent, TorrentFile,
    TorrentFileWhereInput, TorrentWhereInput, UpdateMediaFileInput, UpdateNotificationInput,
    UpdatePendingFileMatchInput, User,
};
use crate::services::ServicesManager;
use crate::services::graphql::auth::{AuthExt, AuthUser, Role};
use crate::services::library_scan::{checked_library_path, validate_relative_library_path};
use crate::services::quality::profile::{self, QualityEvaluation};
use graphql_orm::graphql::errors::{OrmErrorCode, OrmPublicError};
use graphql_orm::graphql::orm::{DbAuthContext, TransactionMode};

/// Notification `actionType` used for Q38/Q44 upgrade-available notifications.
pub const QUALITY_UPGRADE_ACTION_TYPE: &str = "quality_upgrade";

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "QualityEvaluationResult")]
#[graphql(rename_fields = "camelCase")]
pub struct QualityEvaluationResultGql {
    pub success: bool,
    pub quality_status: Option<String>,
    pub reasons: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "RecomputeQualityResult")]
#[graphql(rename_fields = "camelCase")]
pub struct RecomputeQualityResultGql {
    pub success: bool,
    pub evaluated: i32,
    pub optimal: i32,
    pub suboptimal: i32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "ApproveQualityUpgradeResult")]
#[graphql(rename_fields = "camelCase")]
pub struct ApproveQualityUpgradeResultGql {
    pub success: bool,
    pub error: Option<String>,
}

async fn evaluate_and_persist(
    manager: &Arc<ServicesManager>,
    media_file_id: &str,
) -> anyhow::Result<QualityEvaluation> {
    let db_service = manager
        .get_database()
        .await
        .ok_or_else(|| anyhow::anyhow!("database service not available"))?;
    let db = db_service.pool().clone();
    profile::recompute_and_persist(&db, media_file_id).await
}

#[derive(Default)]
pub struct QualityMutations;

#[Object]
impl QualityMutations {
    /// Resolve the effective quality profile for a media file, evaluate it,
    /// and persist `quality_status`.
    #[graphql(name = "evaluateMediaFileQuality")]
    async fn evaluate_media_file_quality(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "mediaFileId")] media_file_id: String,
    ) -> Result<QualityEvaluationResultGql> {
        ctx.require_member()?;
        let db = ctx.data::<crate::db::Database>()?;
        let policy_key = <MediaFile as graphql_orm::graphql::orm::Entity>::metadata().read_policy;
        db.ensure_entity_access(
            Some(ctx),
            "MediaFile",
            policy_key,
            graphql_orm::graphql::orm::EntityAccessKind::Read,
            graphql_orm::graphql::orm::EntityAccessSurface::GraphqlMutation,
        )
        .await?;
        let file = MediaFile::get(db.pool(), &media_file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Media file not found or access denied"))?;
        let policy = db
            .row_policy()
            .ok_or_else(|| async_graphql::Error::new("Media file access policy unavailable"))?;
        if !policy
            .can_read_row(
                Some(ctx),
                db,
                "MediaFile",
                policy_key,
                graphql_orm::graphql::orm::EntityAccessSurface::GraphqlMutation,
                &file,
            )
            .await?
        {
            return Err(async_graphql::Error::new(
                "Media file not found or access denied",
            ));
        }
        let manager = ctx.data_unchecked::<Arc<ServicesManager>>();

        match evaluate_and_persist(manager, &media_file_id).await {
            Ok(evaluation) => Ok(QualityEvaluationResultGql {
                success: true,
                quality_status: Some(evaluation.status_str().to_string()),
                reasons: evaluation.reasons().to_vec(),
                error: None,
            }),
            Err(e) => Ok(QualityEvaluationResultGql {
                success: false,
                quality_status: None,
                reasons: vec![],
                error: Some(e.to_string()),
            }),
        }
    }

    /// Re-evaluate every media file in a library against its resolved
    /// profile. Use after (re)assigning a `qualityProfileId`.
    #[graphql(name = "recomputeLibraryQualityStatus")]
    async fn recompute_library_quality_status(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "libraryId")] library_id: String,
    ) -> Result<RecomputeQualityResultGql> {
        ctx.require_admin()?;
        let manager = ctx.data_unchecked::<Arc<ServicesManager>>();

        let Some(db_service) = manager.get_database().await else {
            return Ok(RecomputeQualityResultGql {
                success: false,
                evaluated: 0,
                optimal: 0,
                suboptimal: 0,
                error: Some("database service not available".to_string()),
            });
        };
        let db = db_service.pool().clone();

        let files = match MediaFile::query(db.pool())
            .filter(MediaFileWhereInput {
                library_id: Some(string_eq(&library_id)),
                ..Default::default()
            })
            .fetch_all()
            .await
        {
            Ok(files) => files,
            Err(e) => {
                return Ok(RecomputeQualityResultGql {
                    success: false,
                    evaluated: 0,
                    optimal: 0,
                    suboptimal: 0,
                    error: Some(e.to_string()),
                });
            }
        };

        let mut optimal = 0;
        let mut suboptimal = 0;
        for media_file in &files {
            match evaluate_and_persist(manager, &media_file.id).await {
                Ok(evaluation) => {
                    if evaluation.is_optimal() {
                        optimal += 1;
                    } else {
                        suboptimal += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        media_file_id = %media_file.id,
                        error = %e,
                        "recomputeLibraryQualityStatus: failed to evaluate media file"
                    );
                }
            }
        }

        Ok(RecomputeQualityResultGql {
            success: true,
            evaluated: files.len() as i32,
            optimal,
            suboptimal,
            error: None,
        })
    }

    /// Q38: approve a pending quality-upgrade notification. Replaces the
    /// existing library file with the candidate that triggered the
    /// notification (never done automatically — see
    /// `LibraryScanService::maybe_notify_quality_upgrade`), queues
    /// re-analysis, and marks the notification resolved/`ACCEPTED`.
    #[graphql(name = "approveQualityUpgrade")]
    async fn approve_quality_upgrade(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "notificationId")] notification_id: String,
    ) -> Result<ApproveQualityUpgradeResultGql> {
        let actor = ctx.require_admin()?;
        let manager = ctx.data_unchecked::<Arc<ServicesManager>>();

        match approve_quality_upgrade_inner(manager, actor, &notification_id).await {
            Ok(()) => Ok(ApproveQualityUpgradeResultGql {
                success: true,
                error: None,
            }),
            Err(e) => Ok(ApproveQualityUpgradeResultGql {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }
}

/// Filesystem proof captured by the internal notification producer, then checked
/// against both the entity-derived path and the opened file before replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpgradeFileProof {
    canonical_path: PathBuf,
    size: u64,
    modified_seconds: u64,
    modified_nanos: u32,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    changed_seconds: i64,
    #[cfg(unix)]
    changed_nanos: i64,
}

impl UpgradeFileProof {
    fn from_metadata(path: &Path, metadata: &std::fs::Metadata) -> anyhow::Result<Self> {
        if !metadata.is_file() {
            anyhow::bail!("Upgrade files must be regular files");
        }
        let modified = metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?;
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            canonical_path: std::fs::canonicalize(path)?,
            size: metadata.len(),
            modified_seconds: modified.as_secs(),
            modified_nanos: modified.subsec_nanos(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(unix)]
            changed_seconds: metadata.ctime(),
            #[cfg(unix)]
            changed_nanos: metadata.ctime_nsec(),
        })
    }
    fn capture(path: &Path) -> anyhow::Result<Self> {
        let metadata = std::fs::symlink_metadata(path).context("Upgrade file is unavailable")?;
        if metadata.file_type().is_symlink() {
            anyhow::bail!("Upgrade files cannot be symlinks");
        }
        Self::from_metadata(path, &metadata)
    }
    fn recheck(&self, path: &Path) -> anyhow::Result<()> {
        if &Self::capture(path)? != self {
            anyhow::bail!("Upgrade file changed; refresh the notification before approving");
        }
        Ok(())
    }
    fn same_file(&self, other: &Self) -> bool {
        #[cfg(unix)]
        {
            self.device == other.device && self.inode == other.inode
        }
        #[cfg(not(unix))]
        {
            self.canonical_path == other.canonical_path
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpgradeActionData {
    source_path: PathBuf,
    target_path: PathBuf,
    media_file_id: String,
    pending_match_id: String,
    source_proof: UpgradeFileProof,
    target_proof: UpgradeFileProof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpgradeScope {
    owner_id: String,
    library_id: String,
    media_file_id: String,
    pending_match_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QualityUpgradePlan {
    scope: UpgradeScope,
    library_root: PathBuf,
    source: PathBuf,
    target: PathBuf,
    target_record_path: PathBuf,
    source_proof: UpgradeFileProof,
    target_proof: UpgradeFileProof,
    action_data: String,
    pending_snapshot: String,
    target_snapshot: String,
    source_torrent_id: String,
    source_torrent_hash: String,
    source_torrent_library: Option<String>,
    source_file_id: String,
    source_file_snapshot: String,
}

fn targets_match(file: &MediaFile, pending: &PendingFileMatch) -> bool {
    let pairs = [
        (&file.movie_id, &pending.movie_id),
        (&file.episode_id, &pending.episode_id),
        (&file.track_id, &pending.track_id),
        (&file.chapter_id, &pending.chapter_id),
    ];
    pairs.iter().filter(|(target, _)| target.is_some()).count() == 1
        && pairs
            .into_iter()
            .all(|(target, candidate)| target == candidate)
}

async fn verify_target_owner(
    db: &Database,
    file: &MediaFile,
    library: &Library,
) -> anyhow::Result<()> {
    let (library_id, owner_id) = if let Some(id) = &file.movie_id {
        let row = Movie::get(db.pool(), id)
            .await?
            .context("Upgrade movie is missing")?;
        (row.library_id, row.user_id)
    } else if let Some(id) = &file.episode_id {
        let episode = Episode::get(db.pool(), id)
            .await?
            .context("Upgrade episode is missing")?;
        let row = Show::get(db.pool(), &episode.show_id)
            .await?
            .context("Upgrade show is missing")?;
        (row.library_id, row.user_id)
    } else if let Some(id) = &file.track_id {
        let track = crate::graphql::entities::Track::get(db.pool(), id)
            .await?
            .context("Upgrade track is missing")?;
        let row = Album::get(db.pool(), &track.album_id)
            .await?
            .context("Upgrade album is missing")?;
        if track.library_id != row.library_id {
            anyhow::bail!("Upgrade track library is inconsistent");
        }
        (row.library_id, row.user_id)
    } else if let Some(id) = &file.chapter_id {
        let chapter = Chapter::get(db.pool(), id)
            .await?
            .context("Upgrade chapter is missing")?;
        let row = Audiobook::get(db.pool(), &chapter.audiobook_id)
            .await?
            .context("Upgrade audiobook is missing")?;
        (row.library_id, row.user_id)
    } else {
        anyhow::bail!("Upgrade target is unlinked");
    };
    if library_id != library.id || owner_id != library.user_id {
        anyhow::bail!("Upgrade target ownership disagrees with its library");
    }
    Ok(())
}

#[derive(Deserialize)]
struct ExtractionMarker {
    archive: String,
    files: Vec<ExtractionMarkerFile>,
}
#[derive(Deserialize)]
struct ExtractionMarkerFile {
    relative_path: String,
    size: u64,
}

/// The extraction layout and completed manifest are the filesystem protocol in
/// services/extract.rs. Anchor it to a persisted, administrator-managed TorrentFile;
/// pending/source JSON alone never grants authority to read an arbitrary file.
fn extracted_source_matches(
    file: &TorrentFile,
    hash: &str,
    source: &Path,
    size: u64,
) -> anyhow::Result<bool> {
    let archive = Path::new(&file.file_path);
    if !crate::services::extract::is_extraction_entry_point(archive) {
        return Ok(false);
    }
    if UpgradeFileProof::capture(archive)?.size != file.file_size as u64 {
        anyhow::bail!("Upgrade source archive size changed");
    }
    let Some(parent) = archive.parent() else {
        return Ok(false);
    };
    let name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .context("Archive name is invalid")?;
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let subdir = if sanitized.len() > 120 {
        &sanitized[sanitized.len() - 120..]
    } else {
        &sanitized
    };
    let staging = parent
        .join(crate::services::extract::STAGING_DIR_NAME)
        .join(hash)
        .join(subdir);
    if !source.starts_with(&staging) {
        return Ok(false);
    }
    let canonical_staging = checked_library_path(parent, &staging)?;
    let canonical_source = checked_library_path(&staging, source)?;
    if !canonical_source.starts_with(&canonical_staging) {
        anyhow::bail!("Upgrade extraction source escaped staging");
    }
    let marker_path = staging.join(".librarian-extract.json");
    checked_library_path(parent, &marker_path)?;
    let mut raw = Vec::new();
    std::fs::File::open(marker_path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)?;
    if raw.len() > 16 * 1024 * 1024 {
        anyhow::bail!("Extraction manifest exceeds its limit");
    }
    let marker: ExtractionMarker = serde_json::from_slice(&raw)?;
    let relative = source.strip_prefix(&staging)?;
    validate_relative_library_path(relative)?;
    Ok(marker.archive == name
        && marker
            .files
            .iter()
            .filter(|entry| Path::new(&entry.relative_path) == relative && entry.size == size)
            .count()
            == 1)
}

async fn verified_pending_source(
    db: &Database,
    pending: &PendingFileMatch,
    library: &Library,
) -> anyhow::Result<(PathBuf, Torrent, TorrentFile)> {
    if pending.source_type != "torrent"
        || pending.copied_at.is_some()
        || pending.user_id != library.user_id
        || pending.file_size <= 0
    {
        anyhow::bail!("Upgrade source is not an owned pending torrent import");
    }
    let hash = pending
        .source_id
        .as_deref()
        .context("Upgrade source has no torrent identity")?;
    if hash.len() != 40 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        anyhow::bail!("Invalid upgrade source identity");
    }
    let mut torrents = Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            info_hash: Some(string_eq(hash)),
            ..Default::default()
        })
        .limit(2)
        .fetch_all()
        .await?;
    if torrents.len() != 1 {
        anyhow::bail!("Upgrade torrent identity is missing or ambiguous");
    }
    let torrent = torrents.pop().unwrap();
    if torrent.user_id != library.user_id
        || torrent
            .library_id
            .as_deref()
            .is_some_and(|id| id != library.id)
    {
        anyhow::bail!("Upgrade source and target have different owners or libraries");
    }
    let source = Path::new(&pending.source_path);
    let mut offset = 0;
    let mut matches = 0;
    let mut matched_file = None;
    loop {
        let page = TorrentFile::query(db.pool())
            .filter(TorrentFileWhereInput {
                torrent_id: Some(string_eq(&torrent.id)),
                ..Default::default()
            })
            .limit(100)
            .offset(offset)
            .fetch_all()
            .await?;
        if page.is_empty() {
            break;
        }
        offset += page.len() as i64;
        for file in page {
            if file.is_excluded || file.file_size <= 0 || file.downloaded_bytes < file.file_size {
                continue;
            }
            if let Some(index) = pending.source_file_index {
                if file.file_index == index {
                    if file.file_path != pending.source_path || file.file_size != pending.file_size
                    {
                        anyhow::bail!("Pending source disagrees with the persisted torrent file");
                    }
                    UpgradeFileProof::capture(Path::new(&file.file_path))?;
                    matches += 1;
                    matched_file = Some(file);
                }
            } else if extracted_source_matches(&file, hash, source, pending.file_size as u64)? {
                matches += 1;
                matched_file = Some(file);
            }
        }
    }
    if matches != 1 {
        anyhow::bail!("Upgrade source has no unique complete persisted file or extraction proof");
    }
    let proof = UpgradeFileProof::capture(source)?;
    if proof.size != pending.file_size as u64 {
        anyhow::bail!("Upgrade source size changed");
    }
    Ok((
        proof.canonical_path,
        torrent,
        matched_file.context("Upgrade source file is missing")?,
    ))
}

async fn verify_upgrade_plan(
    db: &Database,
    scope: UpgradeScope,
    action_data: &str,
) -> anyhow::Result<QualityUpgradePlan> {
    let action: UpgradeActionData = serde_json::from_str(action_data).context(
        "Upgrade notification lacks verified identities; retry the import to regenerate it",
    )?;
    if action.media_file_id != scope.media_file_id
        || action.pending_match_id != scope.pending_match_id
    {
        anyhow::bail!("Upgrade payload identities disagree with the notification");
    }
    let library = Library::get(db.pool(), &scope.library_id)
        .await?
        .context("Upgrade library is missing")?;
    if library.user_id != scope.owner_id || User::get(db.pool(), &scope.owner_id).await?.is_none() {
        anyhow::bail!("Upgrade notification has no valid target owner");
    }
    let file = MediaFile::get(db.pool(), &scope.media_file_id)
        .await?
        .context("Upgrade target is missing")?;
    let pending = PendingFileMatch::get(db.pool(), &scope.pending_match_id)
        .await?
        .context("Upgrade pending source is missing")?;
    if file.library_id.as_deref() != Some(library.id.as_str()) || !targets_match(&file, &pending) {
        anyhow::bail!("Upgrade pending source does not identify the current library target");
    }
    verify_target_owner(db, &file, &library).await?;
    UpgradeFileProof::capture(Path::new(&file.path))?;
    let target = checked_library_path(Path::new(&library.path), Path::new(&file.path))?;
    let (source, source_torrent, source_file) =
        verified_pending_source(db, &pending, &library).await?;
    if UpgradeFileProof::capture(&source)?.same_file(&UpgradeFileProof::capture(&target)?) {
        anyhow::bail!("Upgrade source and target are the same file");
    }
    if std::fs::canonicalize(&action.source_path)? != source
        || std::fs::canonicalize(&action.target_path)? != target
    {
        anyhow::bail!("Upgrade payload paths disagree with the entity-derived files");
    }
    action.source_proof.recheck(&source)?;
    action.target_proof.recheck(&target)?;
    if file.size < 0 || action.target_proof.size != file.size as u64 {
        anyhow::bail!("Upgrade target size changed since its last scan");
    }
    Ok(QualityUpgradePlan {
        scope,
        library_root: std::fs::canonicalize(&library.path)?,
        source,
        target,
        target_record_path: PathBuf::from(&file.path),
        source_proof: action.source_proof,
        target_proof: action.target_proof,
        action_data: action_data.to_string(),
        pending_snapshot: serde_json::to_string(&pending)?,
        target_snapshot: serde_json::to_string(&file)?,
        source_torrent_id: source_torrent.id,
        source_torrent_hash: source_torrent.info_hash,
        source_torrent_library: source_torrent.library_id,
        source_file_id: source_file.id.clone(),
        source_file_snapshot: serde_json::to_string(&source_file)?,
    })
}

/// Called only by the authorized conflict workflow after its pending match is persisted.
/// Legacy/path-only notifications cannot be approved; a retry produces these proofs.
pub(crate) async fn build_quality_upgrade_action_data(
    db: &Database,
    owner: &str,
    library: &str,
    media_file: &str,
    source: &Path,
    target: &Path,
) -> anyhow::Result<(String, String)> {
    let mut pending = PendingFileMatch::query(db.pool())
        .filter(PendingFileMatchWhereInput {
            user_id: Some(string_eq(owner)),
            source_path: Some(string_eq(&source.to_string_lossy())),
            ..Default::default()
        })
        .limit(2)
        .fetch_all()
        .await?;
    if pending.len() != 1 {
        anyhow::bail!("Upgrade pending source identity is missing or ambiguous");
    }
    let pending = pending.pop().unwrap();
    let action = UpgradeActionData {
        source_path: source.to_path_buf(),
        target_path: target.to_path_buf(),
        media_file_id: media_file.to_string(),
        pending_match_id: pending.id.clone(),
        source_proof: UpgradeFileProof::capture(source)?,
        target_proof: UpgradeFileProof::capture(target)?,
    };
    let action_data = serde_json::to_string(&action)?;
    verify_upgrade_plan(
        db,
        UpgradeScope {
            owner_id: owner.to_string(),
            library_id: library.to_string(),
            media_file_id: media_file.to_string(),
            pending_match_id: pending.id.clone(),
        },
        &action_data,
    )
    .await?;
    Ok((pending.id, action_data))
}

async fn read_upgrade_plan(
    db: &Database,
    notification_id: &str,
) -> anyhow::Result<QualityUpgradePlan> {
    let notification_id = notification_id.to_string();
    let notification = Notification::get(db.pool(), &notification_id)
        .await?
        .context("Upgrade notification is missing")?;
    if notification.action_type.as_deref() != Some(QUALITY_UPGRADE_ACTION_TYPE)
        || notification.resolved_at.is_some()
    {
        anyhow::bail!("Notification is not a pending quality upgrade");
    }
    let scope = UpgradeScope {
        owner_id: notification.user_id,
        library_id: notification
            .library_id
            .context("Upgrade notification has no library")?,
        media_file_id: notification
            .media_file_id
            .context("Upgrade notification has no target")?,
        pending_match_id: notification
            .pending_match_id
            .context("Upgrade notification has no pending source")?,
    };
    verify_upgrade_plan(
        db,
        scope,
        notification
            .action_data
            .as_deref()
            .context("Upgrade notification has no action data")?,
    )
    .await
}

async fn require_current_admin(db: &Database, actor: &AuthUser) -> anyhow::Result<()> {
    let user = User::get(db.pool(), &actor.user_id)
        .await?
        .context("Upgrade administrator is unavailable")?;
    if !actor.is_admin() || !user.is_active || Role::parse(&user.role) != Some(Role::Admin) {
        anyhow::bail!("Upgrade approval requires a current administrator");
    }
    Ok(())
}

struct StagedUpgrade {
    path: PathBuf,
}
impl Drop for StagedUpgrade {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn stage_upgrade(plan: &QualityUpgradePlan) -> anyhow::Result<StagedUpgrade> {
    checked_library_path(&plan.library_root, &plan.target)?;
    plan.target_proof.recheck(&plan.target)?;
    plan.source_proof.recheck(&plan.source)?;
    let mut source = std::fs::File::open(&plan.source)?;
    if UpgradeFileProof::from_metadata(&plan.source, &source.metadata()?)? != plan.source_proof {
        anyhow::bail!("Opened upgrade source changed");
    }
    let path = plan
        .target
        .with_file_name(format!(".librarian-upgrade-{}", uuid::Uuid::new_v4()));
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    let staging = StagedUpgrade { path };
    // Copy into our own inode: later library writes cannot modify a seeding hardlink.
    if std::io::copy(&mut source, &mut output)? != plan.source_proof.size {
        anyhow::bail!("Upgrade source size changed during copy");
    }
    output.sync_all()?;
    plan.source_proof.recheck(&plan.source)?;
    if UpgradeFileProof::from_metadata(&plan.source, &source.metadata()?)? != plan.source_proof {
        anyhow::bail!("Opened upgrade source changed during copy");
    }
    Ok(staging)
}

fn finish_upgrade(plan: &QualityUpgradePlan, staging: &StagedUpgrade) -> anyhow::Result<()> {
    if checked_library_path(&plan.library_root, &plan.target)? != plan.target {
        anyhow::bail!("Upgrade target parent changed");
    }
    plan.source_proof.recheck(&plan.source)?;
    plan.target_proof.recheck(&plan.target)?;
    std::fs::rename(&staging.path, &plan.target)
        .context("Cannot atomically replace the verified library file")
}

async fn apply_quality_upgrade(
    db: &Database,
    actor: &AuthUser,
    notification_id: &str,
) -> anyhow::Result<(String, PathBuf)> {
    require_current_admin(db, actor).await?;
    let plan = read_upgrade_plan(db, notification_id).await?;
    let copy_plan = plan.clone();
    let staging = tokio::task::spawn_blocking(move || stage_upgrade(&copy_plan)).await??;
    // Disk copying can be slow. Recheck persisted scope and authority before the only replacement.
    require_current_admin(db, actor).await?;
    if read_upgrade_plan(db, notification_id).await? != plan {
        anyhow::bail!("Upgrade bindings changed during preparation");
    }
    let now = chrono::Utc::now().to_rfc3339();
    let modified =
        chrono::DateTime::<chrono::Utc>::from(std::fs::metadata(&staging.path)?.modified()?)
            .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
    let auth = DbAuthContext {
        user_id: Some(plan.scope.owner_id.clone()),
        actor_id: Some(actor.user_id.clone()),
        subject: Some(actor.user_id.clone()),
        roles: vec!["admin".to_string()],
        ..Default::default()
    };
    let notification_id = notification_id.to_string();
    let target_path = plan.target_record_path.clone();
    let media_file_id = plan.scope.media_file_id.clone();
    let actor_id = actor.user_id.clone();
    db.transaction_with_auth(TransactionMode::StateMachine, Some(&auth), move |tx| {
        Box::pin(async move {
            let notification = tx
                .find_by_id::<Notification>(&notification_id)
                .await?
                .ok_or_else(OrmPublicError::forbidden)?;
            if notification.action_data.as_deref() != Some(plan.action_data.as_str())
                || notification.resolved_at.is_some()
                || notification.action_type.as_deref() != Some(QUALITY_UPGRADE_ACTION_TYPE)
                || notification.user_id != plan.scope.owner_id
                || notification.library_id.as_deref() != Some(plan.scope.library_id.as_str())
                || notification.media_file_id.as_deref() != Some(plan.scope.media_file_id.as_str())
                || notification.pending_match_id.as_deref()
                    != Some(plan.scope.pending_match_id.as_str())
            {
                return Err(OrmPublicError::forbidden());
            }
            let admin = tx
                .find_by_id::<User>(&actor_id)
                .await?
                .ok_or_else(OrmPublicError::forbidden)?;
            let library = tx
                .find_by_id::<Library>(&plan.scope.library_id)
                .await?
                .ok_or_else(OrmPublicError::forbidden)?;
            let target = tx
                .find_by_id::<MediaFile>(&plan.scope.media_file_id)
                .await?
                .ok_or_else(OrmPublicError::forbidden)?;
            let pending = tx
                .find_by_id::<PendingFileMatch>(&plan.scope.pending_match_id)
                .await?
                .ok_or_else(OrmPublicError::forbidden)?;
            let torrent = tx
                .find_by_id::<Torrent>(&plan.source_torrent_id)
                .await?
                .ok_or_else(OrmPublicError::forbidden)?;
            let source_file = tx
                .find_by_id::<TorrentFile>(&plan.source_file_id)
                .await?
                .ok_or_else(OrmPublicError::forbidden)?;
            let changed = || {
                OrmPublicError::with_message(
                    OrmErrorCode::Conflict,
                    "Upgrade scope changed before replacement",
                )
            };
            if !admin.is_active
                || Role::parse(&admin.role) != Some(Role::Admin)
                || library.user_id != plan.scope.owner_id
                || std::fs::canonicalize(&library.path).map_err(|_| changed())? != plan.library_root
                || serde_json::to_string(&target).map_err(|_| changed())? != plan.target_snapshot
                || serde_json::to_string(&pending).map_err(|_| changed())? != plan.pending_snapshot
                || torrent.user_id != plan.scope.owner_id
                || torrent.info_hash != plan.source_torrent_hash
                || torrent.library_id != plan.source_torrent_library
                || serde_json::to_string(&source_file).map_err(|_| changed())?
                    != plan.source_file_snapshot
            {
                return Err(changed());
            }
            tx.update_by_id::<MediaFile>(
                &plan.scope.media_file_id,
                UpdateMediaFileInput {
                    size: Some(plan.source_proof.size as i64),
                    file_modified_at: Some(Some(modified)),
                    analyzed_at: Some(None),
                    quality_status: Some(None),
                    ..Default::default()
                },
            )
            .await?
            .ok_or_else(OrmPublicError::forbidden)?;
            tx.update_by_id::<PendingFileMatch>(
                &plan.scope.pending_match_id,
                UpdatePendingFileMatchInput {
                    copied_at: Some(Some(now.clone())),
                    copy_error: Some(None),
                    ..Default::default()
                },
            )
            .await?
            .ok_or_else(OrmPublicError::forbidden)?;
            tx.update_by_id::<Notification>(
                &notification_id,
                UpdateNotificationInput {
                    resolved_at: Some(Some(now.clone())),
                    resolution: Some(Some("ACCEPTED".to_string())),
                    read_at: Some(Some(now)),
                    ..Default::default()
                },
            )
            .await?
            .ok_or_else(OrmPublicError::forbidden)?;
            // All generated writes and their hooks must succeed before touching the
            // existing file. The state-machine transaction holds the checked DB scope.
            finish_upgrade(&plan, &staging).map_err(|error| {
                OrmPublicError::with_message(OrmErrorCode::Conflict, error.to_string())
            })?;
            Ok(())
        })
    })
    .await
    .context("Upgrade approval failed; source payload bytes were retained")?;
    Ok((media_file_id, target_path))
}

async fn approve_quality_upgrade_inner(
    manager: &Arc<ServicesManager>,
    actor: &AuthUser,
    notification_id: &str,
) -> anyhow::Result<()> {
    let db_service = manager
        .get_database()
        .await
        .context("Database service is unavailable")?;
    let (media_file_id, target_path) =
        apply_quality_upgrade(db_service.pool(), actor, notification_id).await?;
    if let Some(scan_service) = manager.get_library_scan().await {
        scan_service
            .queue_analyze_job(&media_file_id, &target_path.to_string_lossy())
            .await?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "quality_safety_tests.rs"]
mod safety_tests;
