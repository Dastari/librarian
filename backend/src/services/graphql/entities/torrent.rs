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
    #[graphql_orm(write_policy = "admin.write")]
    pub state: String,

    #[graphql(name = "progress")]
    #[filterable(type = "number")]
    #[sortable]
    #[graphql_orm(write_policy = "admin.write")]
    pub progress: f64,

    #[graphql(name = "totalBytes")]
    #[filterable(type = "number")]
    #[sortable]
    #[graphql_orm(write_policy = "admin.write")]
    pub total_bytes: i64,

    #[graphql(name = "downloadedBytes")]
    #[filterable(type = "number")]
    #[graphql_orm(write_policy = "admin.write")]
    pub downloaded_bytes: i64,

    /// Bytes uploaded by the *current* librqbit session. librqbit resets this
    /// counter on restart, so it is only ever a delta source — never use it
    /// for a share ratio; use `uploaded_bytes_total`.
    #[graphql(name = "uploadedBytes")]
    #[filterable(type = "number")]
    #[graphql_orm(write_policy = "admin.write")]
    pub uploaded_bytes: i64,

    /// Cumulative bytes uploaded across every session, kept monotonic by the
    /// db sync loop (see [`Torrent::accumulate_uploaded_bytes`]). This is the
    /// counter the seeding-ratio rule is evaluated against.
    #[graphql(name = "uploadedBytesTotal")]
    #[filterable(type = "number")]
    #[graphql_orm(default = "0", write_policy = "admin.write")]
    pub uploaded_bytes_total: i64,

    #[graphql(name = "savePath")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "admin.write")]
    pub save_path: String,

    #[graphql(name = "downloadPath")]
    #[graphql_orm(write_policy = "admin.write")]
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
    #[graphql_orm(write_policy = "admin.write")]
    pub post_process_status: Option<String>,

    /// How identifying this download is going: `running`, `done` or `failed`.
    #[graphql(name = "identificationStatus")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "admin.write")]
    pub identification_status: Option<String>,

    /// What a person said this download is, when identification got it wrong.
    #[graphql(name = "identificationKind")]
    #[graphql_orm(write_policy = "admin.write")]
    pub identification_kind: Option<String>,

    /// One line on what it was found to be.
    #[graphql(name = "identificationSummary")]
    #[graphql_orm(write_policy = "admin.write")]
    pub identification_summary: Option<String>,

    #[graphql(name = "postProcessError")]
    #[graphql_orm(write_policy = "admin.write")]
    pub post_process_error: Option<String>,

    #[graphql(name = "processedAt")]
    #[filterable(type = "date")]
    #[graphql_orm(write_policy = "admin.write")]
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
    #[graphql_orm(write_policy = "admin.write")]
    pub excluded_files: Vec<i32>,

    /// Delete the downloaded payload from disk when this torrent is removed.
    ///
    /// Lifecycle hooks only see the mutation event, so this is how the option
    /// reaches the `deleteTorrent` hook: the owner (or an admin) sets it with
    /// `updateTorrent` and then calls `deleteTorrent`. The hook refuses the
    /// delete while completed-file post-processing is still running for the
    /// torrent, because deleting the payload would race the import.
    #[graphql(name = "deleteFilesOnRemove")]
    #[filterable(type = "boolean")]
    #[graphql_orm(default = "0")]
    pub delete_files_on_remove: bool,

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
    #[graphql_orm(write_policy = "admin.write")]
    pub completed_at: Option<String>,

    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "TorrentFile", from = "id", to = "torrent_id", multiple)]
    pub files: Vec<TorrentFile>,

    // Query relations for naming and linking what the torrent was grabbed for. They do not
    // add physical FKs or change existing delete behavior.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub library: Option<Box<Library>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Movie",
        from = "movie_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub movie: Option<Box<Movie>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Show",
        from = "show_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub show: Option<Box<Show>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Episode",
        from = "episode_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub episode: Option<Box<Episode>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Track",
        from = "track_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub track: Option<Box<Track>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Chapter",
        from = "chapter_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub chapter: Option<Box<Chapter>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Album",
        from = "album_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub album: Option<Box<Album>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Audiobook",
        from = "audiobook_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub audiobook: Option<Box<Audiobook>>,

    /// The RSS feed this torrent was grabbed from. Deleting the feed clears the link.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "RssFeed",
        from = "source_feed_id",
        to = "id",
        on_delete = "set_null",
        emit_fk = false
    )]
    pub source_feed: Option<Box<RssFeed>>,

    /// Pending file matches whose source is this torrent. Pending matches store the info
    /// hash (never `Torrent.id`) qualified by owner; see `PendingFileMatch.torrent`.
    /// Deleting the torrent keeps these rows as import history.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "PendingFileMatch",
        from = ["info_hash", "user_id"],
        to = ["source_id", "user_id"],
        target_condition(column = "source_type", equals = "torrent"),
        multiple,
        emit_fk = false
    )]
    pub pending_matches: Vec<PendingFileMatch>,
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
    /// Why the torrent is stopped or waiting: the client's error, or that its download share is
    /// unreachable (it resumes by itself when the share answers).
    #[graphql(name = "statusDetail")]
    pub status_detail: Option<String>,
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
        status_detail: t.status_detail,
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
    /// Grab a release from an RSS feed's catalogue. The link carries the tracker's passkey and is
    /// never sent to a client, so the server reads it; `magnet` and `url` are ignored.
    pub feed_item_id: Option<String>,
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

/// What matching makes of a download. Shown while it downloads and after, and based on the same plan
/// the import uses, so a wrong match can be seen before anything moves.
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct TorrentMatchPreview {
    /// `ready`, `review`, `unlinked` or `none`.
    pub status: String,
    pub summary: Option<String>,
    /// Why it needs a person.
    pub reason: Option<String>,
    pub can_accept: bool,
    pub chapter_source: Option<String>,
    pub file_shape: Option<String>,
    pub evidence: Vec<String>,
    /// False while the plan rests only on names and sizes.
    pub complete_facts: bool,
    pub groups: Vec<TorrentMatchGroup>,
    pub files: Vec<TorrentMatchFile>,
}

/// One book, or what looks like one, in a download.
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct TorrentMatchGroup {
    pub key: String,
    pub label: String,
    pub file_count: i32,
    pub seconds: Option<f64>,
    pub is_target: bool,
    pub book_id: Option<String>,
    pub book_title: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct TorrentMatchFile {
    pub file_index: i32,
    pub group_key: Option<String>,
    /// `match`, `review`, `other` or `none`.
    pub status: String,
    pub target: Option<String>,
    pub detail: Option<String>,
    pub warning: Option<String>,
}

/// Something a download could be.
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct IdentificationCandidate {
    pub source: String,
    pub provider: Option<String>,
    pub provider_id: Option<String>,
    pub library_item_type: Option<String>,
    pub library_item_id: Option<String>,
    pub library_id: Option<String>,
    pub title: String,
    pub creator: Option<String>,
    pub narrator: Option<String>,
    pub year: Option<i32>,
    pub cover_url: Option<String>,
    pub seconds: Option<f64>,
    pub chapters: Option<i32>,
    pub length_percent: Option<f64>,
    pub fit: Option<String>,
    pub score: f64,
    pub reasons: Vec<String>,
}

impl From<crate::services::library_scan::IdentifyCandidate> for IdentificationCandidate {
    fn from(found: crate::services::library_scan::IdentifyCandidate) -> Self {
        Self {
            source: found.source,
            provider: found.provider,
            provider_id: found.provider_id,
            library_item_type: found.library_item_type,
            library_item_id: found.library_item_id,
            library_id: found.library_id,
            title: found.title,
            creator: found.creator,
            narrator: found.narrator,
            year: found.year,
            cover_url: found.cover_url,
            seconds: found.seconds,
            chapters: found.chapters,
            length_percent: found.length_percent,
            fit: found.fit,
            score: found.score,
            reasons: found.reasons,
        }
    }
}

/// What a person decided a result is.
#[derive(Debug, InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct ImportIdentifiedItemInput {
    pub item_id: String,
    /// A listing to add to the library first.
    pub provider_id: Option<String>,
    /// Something already in the library.
    pub library_item_id: Option<String>,
    /// Which library to add it to, when there's a choice.
    pub library_id: Option<String>,
    /// Import even though the plan asks for review.
    pub accept_review: Option<bool>,
    /// Put better copies on tracks that already have files, keeping those files on disk.
    pub better_copies: Option<bool>,
}

/// Whether listening to recordings is possible here.
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct ListeningStatus {
    pub available: bool,
    pub detail: String,
}

/// One file of a download, and what a person says it is.
#[derive(Debug, InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct MatchTorrentFileInput {
    pub info_hash: String,
    pub file_index: i32,
    pub library_id: String,
    pub target_type: String,
    pub target_id: String,
}

/// What one file of a download could be, for a person to choose from.
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct TorrentFileCandidate {
    pub target_type: String,
    pub target_id: String,
    pub target_name: Option<String>,
    pub score: f64,
    pub reason: Option<String>,
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
    /// What one file of a download could be. Administrators, as for matching a library file.
    #[graphql(name = "torrentFileCandidates")]
    async fn torrent_file_candidates(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "infoHash")] info_hash: String,
        #[graphql(name = "fileIndex")] file_index: i32,
        #[graphql(name = "libraryId")] library_id: String,
    ) -> Result<Vec<TorrentFileCandidate>> {
        let user = ctx.require_admin()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let torrents = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        let file = torrents
            .completed_file(&info_hash, file_index)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        let result = scan
            .torrent_file_candidates(user, &info_hash, &file, &library_id)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        Ok(result
            .candidates
            .into_iter()
            .map(|candidate| TorrentFileCandidate {
                target_type: candidate.target_type,
                target_id: candidate.target_id,
                target_name: candidate.target_name,
                score: candidate.score,
                reason: candidate.reason,
            })
            .collect())
    }

    /// What a result of identifying a download could be: a person's own search, or the first one.
    #[graphql(name = "identificationCandidates")]
    async fn identification_candidates(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "itemId")] item_id: String,
        query: Option<String>,
    ) -> Result<Vec<IdentificationCandidate>> {
        let user = ctx.require_admin()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        Ok(scan
            .identification_candidates(user, &item_id, query)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Other listings of a book, compared with its files.
    #[graphql(name = "audiobookEditions")]
    async fn audiobook_editions(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "audiobookId")] audiobook_id: String,
        query: Option<String>,
    ) -> Result<Vec<IdentificationCandidate>> {
        ctx.require_admin()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        Ok(scan
            .book_editions(&audiobook_id, query)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Whether this server can listen to recordings.
    #[graphql(name = "listeningStatus")]
    async fn listening_status(&self, ctx: &Context<'_>) -> Result<ListeningStatus> {
        ctx.require_admin()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        let (available, detail) = scan.listening_status().await;
        Ok(ListeningStatus { available, detail })
    }

    /// What matching makes of a download's files, from names and sizes until they have downloaded
    /// and from their tags and lengths after. Changes nothing.
    #[graphql(name = "torrentMatchPreview")]
    async fn torrent_match_preview(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "infoHash")] info_hash: String,
    ) -> Result<TorrentMatchPreview> {
        let user = ctx.librarian_auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let torrents = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        let files = torrents
            .release_files(&info_hash)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        let index_of: std::collections::HashMap<String, i32> = files
            .iter()
            .filter_map(|file| Some((file.source_path.clone(), file.file_index?)))
            .collect();
        let preview = scan
            .torrent_match_preview(user, &info_hash, files)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        Ok(TorrentMatchPreview {
            status: preview.status.to_string(),
            summary: preview.summary,
            reason: preview.reason,
            can_accept: preview.can_accept,
            chapter_source: preview.chapter_source,
            file_shape: preview.file_shape,
            evidence: preview.evidence,
            complete_facts: preview.complete_facts,
            groups: preview
                .groups
                .into_iter()
                .map(|group| TorrentMatchGroup {
                    key: group.key,
                    label: group.label,
                    file_count: group.file_count,
                    seconds: group.seconds,
                    is_target: group.is_target,
                    book_id: group.book_id,
                    book_title: group.book_title,
                    author: group.author,
                })
                .collect(),
            files: preview
                .files
                .into_iter()
                .filter_map(|file| {
                    Some(TorrentMatchFile {
                        file_index: *index_of.get(&file.path)?,
                        group_key: file.group_key,
                        status: file.status.to_string(),
                        target: file.target,
                        detail: file.detail,
                        warning: file.warning,
                    })
                })
                .collect(),
        })
    }

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
        #[graphql(name = "input")] mut input: AddTorrentInput,
    ) -> Result<AddTorrentResult> {
        let db = ctx.data::<Database>()?;
        let user = current_torrent_user(db, ctx.require_member()?).await?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        if let Some(item_id) = input.feed_item_id.take() {
            let (link, feed) = resolve_feed_item_for_grab(db, manager, &user, &item_id).await?;
            input.magnet = None;
            input.url = None;
            if link.to_ascii_lowercase().starts_with("magnet:") {
                input.magnet = Some(link);
            } else {
                input.url = Some(link);
            }
            input.source_feed_id = Some(feed.id);
        }
        let owner_id = authorize_add_torrent_targets(db, &user, &input).await?;
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
        // Some sources (IPTorrents) only hand a torrent file to a logged-in client, and the torrent
        // client has no login. Fetch those through the source, which does.
        let source_file = match (input.magnet.as_deref(), input.source_indexer_id.as_deref()) {
            (None, Some(source_id)) => {
                match fetch_torrent_from_source(manager, source_id, uri).await {
                    Ok(file) => file,
                    Err(error) => {
                        tracing::warn!(source_id = %source_id, error = %error, "addTorrent: source could not supply the torrent file");
                        return Ok(AddTorrentResult {
                            success: false,
                            torrent: None,
                            error: Some(error.to_string()),
                        });
                    }
                }
            }
            _ => None,
        };
        let linkage = crate::jobs::auto_download::CandidateLinkage {
            episode_id: input.episode_id.clone(),
            movie_id: input.movie_id.clone(),
            track_id: input.track_id.clone(),
            chapter_id: input.chapter_id.clone(),
            show_id: input.show_id.clone(),
            album_id: input.album_id.clone(),
            audiobook_id: input.audiobook_id.clone(),
            season: input.season,
        };
        let metadata = TorrentAddMetadata {
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
            wanted_files: None,
        };
        let result = match source_file {
            Some(file) => {
                service
                    .add_torrent_file_for_actor_with_outcome(file, &user, metadata)
                    .await
            }
            None => {
                service
                    .add_magnet_for_actor_with_outcome(uri, &user, metadata)
                    .await
            }
        };
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
                if outcome.created {
                    // Anything recommended for this target is settled by a download of it.
                    crate::jobs::recommend::forget_for_linkage(db, &linkage).await;
                }
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
            // The error may quote the link, and a feed's links carry the account's passkey.
            Err(e) => Ok(AddTorrentResult {
                success: false,
                torrent: None,
                error: Some(crate::services::redact::redact_text(&e.to_string())),
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

    /// Say what one file of a download is, and import it as that.
    #[graphql(name = "matchTorrentFile")]
    async fn match_torrent_file(
        &self,
        ctx: &Context<'_>,
        input: MatchTorrentFileInput,
    ) -> Result<RematchSourceResult> {
        let user = ctx.require_admin()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let torrents = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        let file = match torrents
            .completed_file(&input.info_hash, input.file_index)
            .await
        {
            Ok(file) => file,
            Err(error) => {
                return Ok(RematchSourceResult {
                    success: false,
                    match_count: 0,
                    error: Some(error.to_string()),
                });
            }
        };
        let summary = scan
            .import_torrent_file_as(
                user,
                &input.info_hash,
                file,
                crate::services::library_scan::ManualFileTarget {
                    library_id: input.library_id,
                    target_type: input.target_type,
                    target_id: input.target_id,
                },
            )
            .await;
        Ok(RematchSourceResult {
            success: summary.success,
            match_count: summary.files_processed,
            error: summary.error,
        })
    }

    /// Work out what a download is again.
    #[graphql(name = "identifyTorrent")]
    async fn identify_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "infoHash")] info_hash: String,
        #[graphql(
            desc = "Treat it as this kind: audiobook, music, tv or movie. `auto` goes back to working it out."
        )]
        kind: Option<String>,
    ) -> Result<RematchSourceResult> {
        ctx.require_admin()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        if let Some(kind) = kind.as_deref()
            && let Err(error) = scan.set_identification_kind(&info_hash, kind).await
        {
            return Ok(RematchSourceResult {
                success: false,
                match_count: 0,
                error: Some(error.to_string()),
            });
        }
        scan.request_identification(
            &info_hash,
            crate::services::library_scan::IdentifyTrigger::Asked,
        );
        Ok(RematchSourceResult {
            success: true,
            match_count: 0,
            error: None,
        })
    }

    /// Bring in something identification found, as what a person says it is.
    #[graphql(name = "importIdentifiedItem")]
    async fn import_identified_item(
        &self,
        ctx: &Context<'_>,
        input: ImportIdentifiedItemInput,
    ) -> Result<RematchSourceResult> {
        let user = ctx.require_admin()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        let summary = scan
            .import_identified_item(
                user,
                crate::services::library_scan::IdentifiedChoice {
                    item_id: input.item_id,
                    provider_id: input.provider_id,
                    library_item_id: input.library_item_id,
                    library_id: input.library_id,
                    accept_review: input.accept_review.unwrap_or(false),
                    better_copies: input.better_copies.unwrap_or(false),
                },
            )
            .await;
        Ok(RematchSourceResult {
            success: summary.success,
            match_count: summary.files_processed,
            error: summary.error,
        })
    }

    /// Make a book a different listing, keeping its files.
    #[graphql(name = "changeAudiobookEdition")]
    async fn change_audiobook_edition(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "audiobookId")] audiobook_id: String,
        #[graphql(name = "providerId")] provider_id: String,
    ) -> Result<RematchSourceResult> {
        ctx.require_admin()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        Ok(
            match scan.change_book_edition(&audiobook_id, &provider_id).await {
                Ok(()) => RematchSourceResult {
                    success: true,
                    match_count: 0,
                    error: None,
                },
                Err(error) => RematchSourceResult {
                    success: false,
                    match_count: 0,
                    error: Some(error.to_string()),
                },
            },
        )
    }

    /// Check a book's chapters by listening for them, in the background.
    #[graphql(name = "listenToAudiobookChapters")]
    async fn listen_to_audiobook_chapters(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "audiobookId")] audiobook_id: String,
    ) -> Result<RematchSourceResult> {
        ctx.require_admin()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        let (available, detail) = scan.listening_status().await;
        if !available {
            return Ok(RematchSourceResult {
                success: false,
                match_count: 0,
                error: Some(detail),
            });
        }
        scan.start_chapter_check(&audiobook_id);
        Ok(RematchSourceResult {
            success: true,
            match_count: 0,
            error: None,
        })
    }

    /// Put a book's provider chapters back after a release's own replaced them.
    #[graphql(name = "restoreAudiobookChapters")]
    async fn restore_audiobook_chapters(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "audiobookId")] audiobook_id: String,
    ) -> Result<RematchSourceResult> {
        ctx.require_admin()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let scan = manager
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        Ok(match scan.restore_provider_chapters(&audiobook_id).await {
            Ok(()) => RematchSourceResult {
                success: true,
                match_count: 0,
                error: None,
            },
            Err(error) => RematchSourceResult {
                success: false,
                match_count: 0,
                error: Some(error.to_string()),
            },
        })
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

/// True when `link` is on the source's own site. The source sends its login with the request, so
/// a link from anywhere else, and the caller supplies it, must never get that far.
fn link_is_on_source_site(link: &str, site_link: &str) -> bool {
    match (url::Url::parse(link), url::Url::parse(site_link)) {
        (Ok(link), Ok(site)) => {
            matches!(link.scheme(), "http" | "https")
                && link.host_str().is_some()
                && link.host_str() == site.host_str()
        }
        _ => false,
    }
}

/// A bencoded torrent is a dictionary; a login or error page is HTML.
fn looks_like_torrent_file(bytes: &[u8]) -> bool {
    bytes.first() == Some(&b'd')
}

/// The release's torrent file, fetched with its source's login. `None` means the source has no
/// part in this link, and the client should fetch it directly as before.
pub(crate) async fn fetch_torrent_from_source(
    manager: &ServicesManager,
    source_id: &str,
    link: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    let Some(sources) = manager.get_sources().await else {
        return Ok(None);
    };
    let Some(sources) = sources.get_manager().await else {
        return Ok(None);
    };
    let Some(source) = sources.get_source(source_id) else {
        return Ok(None);
    };
    if !link_is_on_source_site(link, source.site_link()) {
        return Ok(None);
    }
    let bytes = source.download(link).await.map_err(|error| {
        anyhow::anyhow!("{} could not supply this torrent: {error}", source.name())
    })?;
    if !looks_like_torrent_file(&bytes) {
        anyhow::bail!(
            "{} returned a web page instead of a torrent. Its login may have expired: check the source in Settings.",
            source.name()
        );
    }
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_login_is_only_sent_to_its_own_site() {
        let site = "https://iptorrents.com/";
        assert!(link_is_on_source_site(
            "https://iptorrents.com/download.php/1/a.torrent",
            site
        ));
        assert!(!link_is_on_source_site(
            "https://evil.example/download.php/1/a.torrent",
            site
        ));
        assert!(!link_is_on_source_site(
            "https://iptorrents.com.evil.example/a.torrent",
            site
        ));
        assert!(!link_is_on_source_site(
            "https://iptorrents.com@evil.example/a.torrent",
            site
        ));
        assert!(!link_is_on_source_site("file:///etc/passwd", site));
        assert!(!link_is_on_source_site("not a url", site));
    }

    #[test]
    fn a_login_page_is_not_a_torrent_file() {
        assert!(looks_like_torrent_file(b"d8:announce"));
        assert!(!looks_like_torrent_file(b"<!DOCTYPE html>"));
        assert!(!looks_like_torrent_file(b""));
    }

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
