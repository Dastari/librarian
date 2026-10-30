use std::sync::Arc;

use crate::graphql::entities::*;
use async_graphql::{Context, InputObject, Object, Result};
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::auth::{AuthExt, AuthUser, Role};
use super::torrent_file::TorrentFile;
use crate::db::Database;
use crate::services::ServicesManager;
use crate::services::torrent::{
    TorrentAddMetadata, TorrentInfo as ServiceTorrentInfo, TorrentService,
    TorrentState as ServiceTorrentState,
};
use graphql_orm::graphql::filters::StringFilter;

#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    async_graphql::SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(complex)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "torrents", plural = "Torrents", default_sort = "added_at")]
pub struct Torrent {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "infoHash")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "admin.write")]
    #[unique]
    pub info_hash: String,

    #[graphql(name = "magnetUri")]
    pub magnet_uri: Option<String>,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "state")]
    #[filterable(type = "string")]
    #[sortable]
    pub state: String,

    #[graphql(name = "progress")]
    #[filterable(type = "number")]
    #[sortable]
    pub progress: f64,

    #[graphql(name = "totalBytes")]
    #[filterable(type = "number")]
    #[sortable]
    pub total_bytes: i64,

    #[graphql(name = "downloadedBytes")]
    #[filterable(type = "number")]
    pub downloaded_bytes: i64,

    /// Bytes uploaded by the *current* librqbit session. librqbit resets this
    /// counter on restart, so it is only ever a delta source — never use it
    /// for a share ratio; use `uploaded_bytes_total`.
    #[graphql(name = "uploadedBytes")]
    #[filterable(type = "number")]
    pub uploaded_bytes: i64,

    /// Cumulative bytes uploaded across every session, kept monotonic by the
    /// db sync loop (see [`Torrent::accumulate_uploaded_bytes`]). This is the
    /// counter the seeding-ratio rule is evaluated against.
    #[graphql(name = "uploadedBytesTotal")]
    #[filterable(type = "number")]
    #[graphql_orm(default = "0")]
    pub uploaded_bytes_total: i64,

    #[graphql(name = "savePath")]
    #[filterable(type = "string")]
    pub save_path: String,

    #[graphql(name = "downloadPath")]
    pub download_path: Option<String>,

    #[graphql(name = "sourceUrl")]
    pub source_url: Option<String>,

    #[graphql(name = "sourceFeedId")]
    #[filterable(type = "string")]
    pub source_feed_id: Option<String>,

    #[graphql(name = "sourceIndexerId")]
    #[filterable(type = "string")]
    pub source_indexer_id: Option<String>,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.mutable-link")]
    pub library_id: Option<String>,

    #[graphql(name = "postProcessStatus")]
    #[filterable(type = "string")]
    pub post_process_status: Option<String>,

    #[graphql(name = "postProcessError")]
    pub post_process_error: Option<String>,

    #[graphql(name = "processedAt")]
    #[filterable(type = "date")]
    pub processed_at: Option<String>,

    // ========================================================================
    // Wanted-target linkage (auto-download, phase A — tier1-features-plan.md §1)
    // ========================================================================
    //
    // Set by `jobs::auto_download` at grab time so a torrent records which
    // wanted item(s) it was grabbed to fulfill. This closes the "downloading"
    // status gap: between grab and completion there was previously no DB
    // record that a wanted item had an active download (only
    // `PendingFileMatch`, created post-completion). All nullable since
    // manually-added/RSS torrents have no wanted-item linkage.
    #[graphql(name = "episodeId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub episode_id: Option<String>,

    #[graphql(name = "movieId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub movie_id: Option<String>,

    #[graphql(name = "trackId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub track_id: Option<String>,

    #[graphql(name = "chapterId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub chapter_id: Option<String>,

    /// Parent show id, set alongside `episode_id` so show-level "has an active
    /// download" checks don't need to join through episodes.
    #[graphql(name = "showId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub show_id: Option<String>,

    /// Parent album id, set alongside `track_id`.
    #[graphql(name = "albumId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub album_id: Option<String>,

    /// Audiobook id. Audiobook releases are generally whole-book (not
    /// per-chapter) torrents, so auto-download grabs at the audiobook level
    /// and only sets this column (`chapter_id` stays null); chapter-level
    /// linkage happens during import matching, same as a manually-downloaded
    /// season pack (see docs/design.md Q52).
    #[graphql(name = "audiobookId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub audiobook_id: Option<String>,

    /// Private-tracker minimum share ratio this release must reach before it
    /// may stop seeding, recorded from `SourceRelease.minimum_ratio` at grab
    /// time. Null when the source declares no floor.
    #[graphql(name = "minimumRatio")]
    pub minimum_ratio: Option<f64>,

    /// Private-tracker minimum seed time in **minutes**, converted from
    /// `SourceRelease.minimum_seed_time` (which the Torznab spec reports in
    /// seconds) at grab time. Null when the source declares no floor.
    #[graphql(name = "minimumSeedTimeMinutes")]
    pub minimum_seed_time_minutes: Option<i32>,

    /// Season number for a season-pack grab, set alongside `show_id` so the
    /// hunt job can tell "a pack for S02 is already in flight" from
    /// "some episode of this show is downloading". Null for per-episode
    /// grabs and every non-TV torrent.
    #[graphql(name = "season")]
    #[filterable(type = "number")]
    pub season: Option<i32>,

    #[graphql(name = "excludedFiles")]
    #[json_field]
    pub excluded_files: Vec<i32>,

    #[graphql(name = "addedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub added_at: String,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    #[graphql(name = "completedAt")]
    #[filterable(type = "date")]
    pub completed_at: Option<String>,

    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "TorrentFile", from = "id", to = "torrent_id", multiple)]
    pub files: Vec<TorrentFile>,
}

#[derive(Default)]
pub struct TorrentCustomOperations;

// =============================================================================
// GraphQL types for custom operations (live client state).
// =============================================================================

/// Live torrent file (from torrent client)
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "LiveTorrentFile")]
pub struct LiveTorrentFile {
    #[graphql(name = "index")]
    pub index: i32,
    #[graphql(name = "path")]
    pub path: String,
    #[graphql(name = "size")]
    pub size: i64,
    #[graphql(name = "progress")]
    pub progress: f64,
}

/// Live torrent (from torrent client, not DB)
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "LiveTorrent")]
pub struct LiveTorrent {
    #[graphql(name = "id")]
    pub id: i32,
    #[graphql(name = "infoHash")]
    pub info_hash: String,
    #[graphql(name = "name")]
    pub name: String,
    #[graphql(name = "state")]
    pub state: String,
    #[graphql(name = "progress")]
    pub progress: f64,
    #[graphql(name = "size")]
    pub size: i64,
    #[graphql(name = "downloaded")]
    pub downloaded: i64,
    #[graphql(name = "uploaded")]
    pub uploaded: i64,
    #[graphql(name = "downloadSpeed")]
    pub download_speed: i64,
    #[graphql(name = "uploadSpeed")]
    pub upload_speed: i64,
    #[graphql(name = "peers")]
    pub peers: i32,
    #[graphql(name = "savePath")]
    pub save_path: String,
    #[graphql(name = "files")]
    pub files: Vec<LiveTorrentFile>,
}

fn service_state_to_string(s: ServiceTorrentState) -> &'static str {
    match s {
        ServiceTorrentState::Queued => "queued",
        ServiceTorrentState::Checking => "checking",
        ServiceTorrentState::Downloading => "downloading",
        ServiceTorrentState::Seeding => "seeding",
        ServiceTorrentState::Paused => "paused",
        ServiceTorrentState::Error => "error",
    }
}

fn service_torrent_to_live(t: ServiceTorrentInfo) -> LiveTorrent {
    LiveTorrent {
        id: t.id as i32,
        info_hash: t.info_hash,
        name: t.name,
        state: service_state_to_string(t.state).to_string(),
        progress: t.progress,
        size: t.size as i64,
        downloaded: t.downloaded as i64,
        uploaded: t.uploaded as i64,
        download_speed: t.download_speed as i64,
        upload_speed: t.upload_speed as i64,
        peers: t.peers as i32,
        save_path: t.save_path,
        files: t
            .files
            .into_iter()
            .map(|f| LiveTorrentFile {
                index: f.index as i32,
                path: f.path,
                size: f.size as i64,
                progress: f.progress,
            })
            .collect(),
    }
}

/// Recheck persisted authority for runtime operations and long-lived streams.
/// Repository reads are trusted: their results do not inherit GraphQL row policy.
/// Never acquire new admin privileges from a stale member authentication context.
pub(crate) async fn current_torrent_user(
    db: &Database,
    authenticated: &AuthUser,
) -> Result<AuthUser> {
    if !authenticated.has_role(Role::Member) {
        return Err(async_graphql::Error::new("Torrent access denied"));
    }
    let user = User::get(db.pool(), &authenticated.user_id)
        .await
        .map_err(|error| async_graphql::Error::new(error.to_string()))?
        .filter(|user| user.is_active)
        .ok_or_else(|| async_graphql::Error::new("Torrent access denied"))?;
    let current_role = Role::parse(&user.role)
        .ok_or_else(|| async_graphql::Error::new("Torrent access denied"))?;
    Ok(AuthUser {
        user_id: user.id,
        email: user.email,
        role: Some(
            if authenticated.is_admin() && current_role == Role::Admin {
                "admin"
            } else {
                "member"
            }
            .to_string(),
        ),
    })
}

pub(crate) fn owns_runtime_torrent(user: &AuthUser, torrent: &Torrent) -> bool {
    user.is_admin() || (user.has_role(Role::Member) && torrent.user_id == user.user_id)
}

/// Exact hash lookup through the generated entity layer. Two matches are enough
/// to detect ambiguity, rather than choosing a possibly foreign persisted ID.
/// Lookup per runtime identity avoids a capped scan of unrelated persisted rows.
pub(crate) async fn persisted_runtime_torrent(
    db: &Database,
    info_hash: &str,
) -> Result<Option<Torrent>> {
    let mut rows = Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            info_hash: Some(StringFilter {
                eq: Some(info_hash.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .limit(2)
        .fetch_all()
        .await
        .map_err(|error| async_graphql::Error::new(error.to_string()))?;
    if rows.len() > 1 {
        return Err(async_graphql::Error::new("Ambiguous torrent identity"));
    }
    Ok(rows.pop())
}

async fn require_runtime_torrent(
    db: &Database,
    user: &AuthUser,
    info_hash: &str,
) -> Result<Torrent> {
    persisted_runtime_torrent(db, info_hash)
        .await?
        .filter(|torrent| owns_runtime_torrent(user, torrent))
        .ok_or_else(|| async_graphql::Error::new("Torrent not found or access denied"))
}

/// Bind requested acquisition targets to one readable library before any runtime
/// add, and keep actor authority distinct from the owner stamped on new rows.
pub(crate) async fn authorize_add_torrent_targets(
    db: &Database,
    caller: &AuthUser,
    input: &AddTorrentInput,
) -> Result<Uuid> {
    use std::collections::BTreeSet;
    let caller = current_torrent_user(db, caller).await?;
    let deny = || async_graphql::Error::new("Torrent target not found or access denied");
    let classes = [
        input.movie_id.is_some(),
        input.show_id.is_some() || input.episode_id.is_some(),
        input.album_id.is_some() || input.track_id.is_some(),
        input.audiobook_id.is_some() || input.chapter_id.is_some(),
    ];
    if classes.into_iter().filter(|present| *present).count() > 1 {
        return Err(deny());
    }
    let mut libraries = BTreeSet::new();
    if let Some(id) = &input.library_id {
        libraries.insert(id.clone());
    }
    macro_rules! owned {
        ($key:expr, $id:expr) => {
            if !caller.is_admin()
                && !crate::services::graphql::policy::AppFieldPolicy::owns_link(
                    db,
                    &caller.user_id,
                    $key,
                    $id,
                )
                .await?
            {
                return Err(deny());
            }
        };
    }
    macro_rules! direct {
        ($field:ident, $ty:ty, $key:literal) => {
            if let Some(id) = &input.$field {
                owned!($key, id);
                let row = <$ty>::get(db.pool(), id)
                    .await
                    .map_err(|_| deny())?
                    .ok_or_else(deny)?;
                libraries.insert(row.library_id);
            }
        };
    }
    direct!(movie_id, Movie, "movieId");
    direct!(show_id, Show, "showId");
    direct!(album_id, Album, "albumId");
    direct!(audiobook_id, Audiobook, "audiobookId");
    if let Some(id) = &input.episode_id {
        owned!("episodeId", id);
        let episode = Episode::get(db.pool(), id)
            .await
            .map_err(|_| deny())?
            .ok_or_else(deny)?;
        if input
            .show_id
            .as_ref()
            .is_some_and(|show| show != &episode.show_id)
        {
            return Err(deny());
        }
        let show = Show::get(db.pool(), &episode.show_id)
            .await
            .map_err(|_| deny())?
            .ok_or_else(deny)?;
        libraries.insert(show.library_id);
    }
    if let Some(id) = &input.track_id {
        owned!("trackId", id);
        let track = Track::get(db.pool(), id)
            .await
            .map_err(|_| deny())?
            .ok_or_else(deny)?;
        if input
            .album_id
            .as_ref()
            .is_some_and(|album| album != &track.album_id)
        {
            return Err(deny());
        }
        libraries.insert(track.library_id);
    }
    if let Some(id) = &input.chapter_id {
        owned!("chapterId", id);
        let chapter = Chapter::get(db.pool(), id)
            .await
            .map_err(|_| deny())?
            .ok_or_else(deny)?;
        if input
            .audiobook_id
            .as_ref()
            .is_some_and(|book| book != &chapter.audiobook_id)
        {
            return Err(deny());
        }
        let book = Audiobook::get(db.pool(), &chapter.audiobook_id)
            .await
            .map_err(|_| deny())?
            .ok_or_else(deny)?;
        libraries.insert(book.library_id);
    }
    if let Some(id) = &input.source_feed_id {
        owned!("feedId", id);
        let feed = RssFeed::get(db.pool(), id)
            .await
            .map_err(|_| deny())?
            .ok_or_else(deny)?;
        if let Some(library) = feed.library_id {
            libraries.insert(library);
        } else if feed.user_id != caller.user_id {
            return Err(deny());
        }
    }
    if libraries.len() > 1 {
        return Err(deny());
    }
    let owner = if let Some(id) = libraries.first() {
        let library = Library::get(db.pool(), id)
            .await
            .map_err(|_| deny())?
            .ok_or_else(deny)?;
        if !caller.is_admin() && library.user_id != caller.user_id {
            return Err(deny());
        }
        library.user_id
    } else {
        caller.user_id
    };
    Uuid::parse_str(&owner).map_err(|_| deny())
}

fn runtime_id(id: i32) -> Result<usize> {
    usize::try_from(id).map_err(|_| async_graphql::Error::new("Invalid torrent ID"))
}

async fn authorized_runtime_hash(
    ctx: &Context<'_>,
    service: &TorrentService,
    id: i32,
) -> Result<String> {
    let id = runtime_id(id)?;
    let info = service
        .get_torrent_info(id)
        .await
        .map_err(|_| async_graphql::Error::new("Torrent not found or access denied"))?;
    // Control by the checked hash so reusing a numeric runtime slot cannot
    // redirect the operation to a different torrent.
    authorized_info_hash(ctx, &info.info_hash).await
}

async fn authorized_info_hash(ctx: &Context<'_>, info_hash: &str) -> Result<String> {
    let db = ctx.data::<Database>()?;
    let user = current_torrent_user(db, ctx.librarian_auth_user()?).await?;
    let hash = info_hash.trim().to_ascii_lowercase();
    let torrent = require_runtime_torrent(db, &user, &hash).await?;
    let current = current_torrent_user(db, ctx.librarian_auth_user()?).await?;
    if !owns_runtime_torrent(&current, &torrent) {
        return Err(async_graphql::Error::new(
            "Torrent not found or access denied",
        ));
    }
    Ok(torrent.info_hash)
}

async fn visible_runtime_torrents(
    db: &Database,
    user: &AuthUser,
    list: Vec<ServiceTorrentInfo>,
) -> Result<Vec<LiveTorrent>> {
    let mut visible = Vec::new();
    for info in list {
        // Administrators intentionally see the entire live session, including
        // runtime orphans; controls still require an unambiguous persisted row.
        if user.is_admin()
            || persisted_runtime_torrent(db, &info.info_hash)
                .await?
                .is_some_and(|torrent| owns_runtime_torrent(user, &torrent))
        {
            visible.push(service_torrent_to_live(info));
        }
    }
    // Runtime enumeration and mapping both await. Do not release a snapshot
    // built with admin privileges if the account was demoted in the meantime.
    let current = current_torrent_user(db, user).await?;
    if current.parsed_role() != user.parsed_role() {
        return Err(async_graphql::Error::new("Torrent authority changed"));
    }
    Ok(visible)
}

/// Input for adding a torrent
#[derive(InputObject, Default)]
#[graphql(name = "AddTorrentInput")]
#[graphql(rename_fields = "camelCase")]
pub struct AddTorrentInput {
    pub magnet: Option<String>,
    pub url: Option<String>,
    pub library_id: Option<String>,
    pub movie_id: Option<String>,
    pub show_id: Option<String>,
    /// Wanted-target linkage persisted onto the created `Torrent` row, so a
    /// manually added release counts as "downloading" for that item exactly
    /// like an auto-download grab does.
    pub episode_id: Option<String>,
    pub track_id: Option<String>,
    pub chapter_id: Option<String>,
    pub album_id: Option<String>,
    pub audiobook_id: Option<String>,
    /// Season number when the added torrent is a season pack.
    pub season: Option<i32>,
    /// `minimumRatio` reported by the source for this release, if any.
    pub minimum_ratio: Option<f64>,
    /// `minimumSeedTime` reported by the source, in **seconds** (as Torznab
    /// reports it); stored on the row in minutes.
    pub minimum_seed_time: Option<i64>,
    pub source_url: Option<String>,
    pub source_indexer_id: Option<String>,
    pub source_feed_id: Option<String>,
}

/// Result of add torrent mutation
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "AddTorrentResult")]
#[graphql(rename_fields = "camelCase")]
pub struct AddTorrentResult {
    pub success: bool,
    pub torrent: Option<LiveTorrent>,
    pub error: Option<String>,
}

/// Result of pause/resume/remove
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "TorrentActionResult")]
pub struct TorrentActionResult {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

/// Result of processing matched files from a source
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "ProcessSourceResult")]
#[graphql(rename_fields = "camelCase")]
pub struct ProcessSourceResult {
    pub success: bool,
    pub files_processed: i32,
    pub files_failed: i32,
    pub messages: Vec<String>,
    pub error: Option<String>,
}

/// Result of re-matching files for a source
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "RematchSourceResult")]
#[graphql(rename_fields = "camelCase")]
pub struct RematchSourceResult {
    pub success: bool,
    pub match_count: i32,
    pub error: Option<String>,
}

#[Object]
impl TorrentCustomOperations {
    /// Get all torrents with live state from the torrent client
    #[graphql(name = "liveTorrents")]
    async fn live_torrents(&self, ctx: &Context<'_>) -> Result<Vec<LiveTorrent>> {
        let db = ctx.data::<Database>()?;
        let user = current_torrent_user(db, ctx.librarian_auth_user()?).await?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let list = service
            .list_torrents()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        visible_runtime_torrents(db, &user, list).await
    }

    /// Get a single live torrent by numeric id
    #[graphql(name = "liveTorrent")]
    async fn live_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: i32,
    ) -> Result<Option<LiveTorrent>> {
        let db = ctx.data::<Database>()?;
        let user = current_torrent_user(db, ctx.librarian_auth_user()?).await?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager.get_torrent().await;
        let id = runtime_id(id)?;
        let Some(service) = service else {
            return Ok(None);
        };
        let Ok(info) = service.get_torrent_info(id).await else {
            return Ok(None);
        };
        Ok(visible_runtime_torrents(db, &user, vec![info]).await?.pop())
    }

    /// Count of active (downloading/checking) torrents
    #[graphql(name = "activeDownloadCount")]
    async fn active_download_count(&self, ctx: &Context<'_>) -> Result<i32> {
        let db = ctx.data::<Database>()?;
        let user = current_torrent_user(db, ctx.librarian_auth_user()?).await?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service: Arc<TorrentService> = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let list: Vec<ServiceTorrentInfo> = service
            .list_active_downloads()
            .await
            .map_err(|e: anyhow::Error| async_graphql::Error::new(e.to_string()))?;
        Ok(visible_runtime_torrents(db, &user, list).await?.len() as i32)
    }
}

/// Details known only at grab time that the torrent service's
/// `TorrentAddMetadata` does not carry.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TorrentGrabDetails {
    /// Season number for a season-pack grab.
    pub season: Option<i32>,
    /// `SourceRelease.minimum_ratio` for the grabbed release.
    pub minimum_ratio: Option<f64>,
    /// `SourceRelease.minimum_seed_time` converted from seconds to minutes.
    pub minimum_seed_time_minutes: Option<i32>,
}

impl TorrentGrabDetails {
    /// Whether there is anything at all to persist.
    pub fn is_empty(&self) -> bool {
        self.season.is_none()
            && self.minimum_ratio.is_none()
            && self.minimum_seed_time_minutes.is_none()
    }

    /// Build the grab details from a release: `minimum_seed_time` is reported
    /// in seconds by Torznab and stored in minutes.
    pub fn from_release_limits(
        season: Option<i32>,
        minimum_ratio: Option<f64>,
        minimum_seed_time_seconds: Option<i64>,
    ) -> Self {
        Self {
            season,
            minimum_ratio,
            minimum_seed_time_minutes: minimum_seed_time_seconds
                .map(|seconds| (seconds / 60).clamp(0, i32::MAX as i64) as i32),
        }
    }
}

/// Persist grab-time details on the `Torrent` row identified by `info_hash`.
///
/// The torrent service creates the row while adding the magnet and its
/// `TorrentAddMetadata` carries none of these fields, so they are stamped
/// afterwards through the generated entity mutation. Shared by `addTorrent`
/// and `jobs::auto_download`.
pub async fn stamp_torrent_grab_details(
    db: &crate::db::Database,
    info_hash: &str,
    details: TorrentGrabDetails,
) -> anyhow::Result<()> {
    use graphql_orm::graphql::filters::StringFilter;

    if details.is_empty() {
        return Ok(());
    }

    let rows = Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            info_hash: Some(StringFilter {
                eq: Some(info_hash.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    let Some(torrent) = rows.into_iter().next() else {
        anyhow::bail!("torrent row for info hash {info_hash} not found");
    };
    Torrent::update_by_id(
        db,
        &torrent.id,
        UpdateTorrentInput {
            season: details.season.map(Some),
            minimum_ratio: details.minimum_ratio.map(Some),
            minimum_seed_time_minutes: details.minimum_seed_time_minutes.map(Some),
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

impl Torrent {
    /// Fold a fresh reading of librqbit's per-session upload counter into the
    /// cumulative total.
    ///
    /// librqbit resets `stats.uploaded_bytes` to zero whenever the process
    /// restarts (and whenever a torrent is re-added), so the stored session
    /// counter is only a baseline. Each tick contributes
    /// `session - previous_session` while the counter is climbing; when it
    /// goes *backwards* the session restarted, so the whole current reading is
    /// new upload. The result is monotonic: it never decreases, which is what
    /// the seeding ratio rule needs.
    pub fn accumulate_uploaded_bytes(
        previous_total: i64,
        previous_session: i64,
        session: i64,
    ) -> i64 {
        let session = session.max(0);
        let previous_session = previous_session.max(0);
        let delta = if session >= previous_session {
            session - previous_session
        } else {
            // Counter reset: the baseline is gone, everything reported now is
            // upload this process has not accounted for yet.
            session
        };
        previous_total.max(0).saturating_add(delta)
    }
}

/// Mutations that use the torrent client (add, pause, resume, remove)
#[derive(Default)]
pub struct TorrentClientMutations;

#[Object]
impl TorrentClientMutations {
    /// Add a torrent from a magnet link or URL
    #[graphql(name = "addTorrent")]
    async fn add_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: AddTorrentInput,
    ) -> Result<AddTorrentResult> {
        let db = ctx.data::<Database>()?;
        let user = current_torrent_user(db, ctx.require_member()?).await?;
        let owner_id = authorize_add_torrent_targets(db, &user, &input).await?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let Some(uri) = input
            .magnet
            .as_deref()
            .or(input.url.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(AddTorrentResult {
                success: false,
                torrent: None,
                error: Some("Either magnet or url must be provided".to_string()),
            });
        };
        let result = service
            .add_magnet_for_actor_with_outcome(
                uri,
                &user,
                TorrentAddMetadata {
                    library_id: input.library_id,
                    movie_id: input.movie_id,
                    show_id: input.show_id,
                    episode_id: input.episode_id,
                    track_id: input.track_id,
                    chapter_id: input.chapter_id,
                    album_id: input.album_id,
                    audiobook_id: input.audiobook_id,
                    source_url: input.source_url,
                    source_indexer_id: input.source_indexer_id,
                    source_feed_id: input.source_feed_id,
                },
            )
            .await;
        match result {
            Ok(outcome) => {
                let info = outcome.info;
                let current = current_torrent_user(db, &user).await?;
                let persisted = require_runtime_torrent(db, &current, &info.info_hash).await?;
                if persisted.user_id != owner_id.to_string() {
                    return Err(async_graphql::Error::new(
                        "Torrent not found or access denied",
                    ));
                }

                // `season` and the tracker seeding floors are not part of
                // `TorrentAddMetadata` (owned by the torrent service); stamp
                // them on the row the service just created, through the
                // entity layer.
                let details = TorrentGrabDetails::from_release_limits(
                    input.season,
                    input.minimum_ratio,
                    input.minimum_seed_time,
                );
                if outcome.created && !details.is_empty() {
                    let db = ctx.data::<crate::db::Database>()?;
                    if let Err(error) =
                        stamp_torrent_grab_details(db, &info.info_hash, details).await
                    {
                        tracing::warn!(
                            info_hash = %info.info_hash,
                            torrent_name = %info.name,
                            error = %error,
                            "addTorrent: failed to stamp grab details on the new torrent row"
                        );
                    }
                }
                Ok(AddTorrentResult {
                    success: true,
                    torrent: Some(service_torrent_to_live(info)),
                    error: None,
                })
            }
            Err(e) => Ok(AddTorrentResult {
                success: false,
                torrent: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Pause a torrent
    #[graphql(name = "pauseTorrent")]
    async fn pause_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: i32,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.librarian_auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let info_hash = authorized_runtime_hash(ctx, &service, id).await?;
        match service.pause_by_info_hash(&info_hash).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Resume a paused torrent
    #[graphql(name = "resumeTorrent")]
    async fn resume_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: i32,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.librarian_auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let info_hash = authorized_runtime_hash(ctx, &service, id).await?;
        match service.resume_by_info_hash(&info_hash).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Remove a torrent
    #[graphql(name = "removeTorrent")]
    async fn remove_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: i32,
        #[graphql(name = "deleteFiles", default = false)] delete_files: bool,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.librarian_auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let info_hash = authorized_runtime_hash(ctx, &service, id).await?;
        match service.remove_by_info_hash(&info_hash, delete_files).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "pauseTorrentByInfoHash")]
    async fn pause_torrent_by_info_hash(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "infoHash")] info_hash: String,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.librarian_auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let info_hash = authorized_info_hash(ctx, &info_hash).await?;
        match service.pause_by_info_hash(&info_hash).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "resumeTorrentByInfoHash")]
    async fn resume_torrent_by_info_hash(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "infoHash")] info_hash: String,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.librarian_auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let info_hash = authorized_info_hash(ctx, &info_hash).await?;
        match service.resume_by_info_hash(&info_hash).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "removeTorrentByInfoHash")]
    async fn remove_torrent_by_info_hash(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "infoHash")] info_hash: String,
        #[graphql(name = "deleteFiles", default = false)] delete_files: bool,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.librarian_auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let info_hash = authorized_info_hash(ctx, &info_hash).await?;
        match service.remove_by_info_hash(&info_hash, delete_files).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Process pending file matches for a source.
    #[graphql(name = "processSource")]
    async fn process_source(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "sourceType")] source_type: String,
        #[graphql(name = "sourceId")] source_id: String,
    ) -> Result<ProcessSourceResult> {
        let user = ctx.librarian_auth_user()?;

        let normalized_source = source_type.trim().to_ascii_lowercase();
        if normalized_source != "torrent" {
            return Ok(ProcessSourceResult {
                success: false,
                files_processed: 0,
                files_failed: 0,
                messages: vec![],
                error: Some("Only source type 'torrent' is currently supported".to_string()),
            });
        }

        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;

        match service
            .process_completed_torrent(user, &source_id, None)
            .await
        {
            Ok(summary) => Ok(ProcessSourceResult {
                success: summary.success,
                files_processed: summary.files_processed,
                files_failed: summary.files_failed,
                messages: summary.messages,
                error: summary.error,
            }),
            Err(error) => Ok(ProcessSourceResult {
                success: false,
                files_processed: 0,
                files_failed: 0,
                messages: vec![],
                error: Some(error.to_string()),
            }),
        }
    }

    /// Re-run matching for files from a source.
    #[graphql(name = "rematchSource")]
    async fn rematch_source(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "sourceType")] source_type: String,
        #[graphql(name = "sourceId")] source_id: String,
        #[graphql(name = "libraryId")] library_id: Option<String>,
    ) -> Result<RematchSourceResult> {
        let user = ctx.librarian_auth_user()?;

        let normalized_source = source_type.trim().to_ascii_lowercase();
        if normalized_source != "torrent" {
            return Ok(RematchSourceResult {
                success: false,
                match_count: 0,
                error: Some("Only source type 'torrent' is currently supported".to_string()),
            });
        }

        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;

        match service
            .process_completed_torrent(user, &source_id, library_id)
            .await
        {
            Ok(summary) => Ok(RematchSourceResult {
                success: summary.success,
                match_count: summary.files_processed,
                error: summary.error,
            }),
            Err(error) => Ok(RematchSourceResult {
                success: false,
                match_count: 0,
                error: Some(error.to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uploaded_total_accumulates_session_deltas() {
        // First sync of a fresh torrent.
        assert_eq!(Torrent::accumulate_uploaded_bytes(0, 0, 500), 500);
        // Steady climb: only the delta is added.
        assert_eq!(Torrent::accumulate_uploaded_bytes(500, 500, 900), 900);
        assert_eq!(Torrent::accumulate_uploaded_bytes(900, 900, 1_000), 1_000);
    }

    #[test]
    fn uploaded_total_survives_a_session_counter_reset() {
        // 1 MB uploaded before the restart, session counter back to 0.
        assert_eq!(
            Torrent::accumulate_uploaded_bytes(1_000_000, 1_000_000, 0),
            1_000_000
        );
        // The new session then uploads 250 KB.
        assert_eq!(
            Torrent::accumulate_uploaded_bytes(1_000_000, 0, 250_000),
            1_250_000
        );
        // A partial reset (counter lower than the baseline but non-zero)
        // still contributes the whole current reading.
        assert_eq!(Torrent::accumulate_uploaded_bytes(1_000, 900, 400), 1_400);
    }

    #[test]
    fn uploaded_total_is_monotonic_across_an_arbitrary_sequence() {
        let readings = [0i64, 10, 40, 40, 5, 25, 0, 7, 7, 100];
        let mut total = 0i64;
        let mut previous_session = 0i64;
        let mut last_total = 0i64;
        for session in readings {
            total = Torrent::accumulate_uploaded_bytes(total, previous_session, session);
            assert!(
                total >= last_total,
                "cumulative upload went backwards: {last_total} -> {total}"
            );
            last_total = total;
            previous_session = session;
        }
        // 40 (first session) + 25 (second) + 100 (third) = 165.
        assert_eq!(total, 165);
    }

    #[test]
    fn uploaded_total_ignores_negative_readings() {
        assert_eq!(Torrent::accumulate_uploaded_bytes(-5, -5, -5), 0);
        assert_eq!(Torrent::accumulate_uploaded_bytes(100, 100, -1), 100);
    }

    #[test]
    fn grab_details_convert_seed_time_seconds_to_minutes() {
        let details = TorrentGrabDetails::from_release_limits(Some(2), Some(1.5), Some(1_209_600));
        assert_eq!(details.season, Some(2));
        assert_eq!(details.minimum_ratio, Some(1.5));
        // 14 days in seconds -> 20160 minutes.
        assert_eq!(details.minimum_seed_time_minutes, Some(20_160));
        assert!(!details.is_empty());

        assert!(TorrentGrabDetails::default().is_empty());
        assert_eq!(
            TorrentGrabDetails::from_release_limits(None, None, Some(30)).minimum_seed_time_minutes,
            Some(0)
        );
    }
}

#[cfg(test)]
#[path = "torrent_authorization_tests.rs"]
pub(crate) mod authorization_tests;
