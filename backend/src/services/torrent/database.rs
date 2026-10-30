//! Database helpers for the torrent service.
//! Reads use typed entity queries; writes use checked generated GraphQL mutations.

use super::{add_torrent_opts, get_info_hash_hex};

use anyhow::Context as _;
use async_graphql::{Request, Variables};
use std::fs;
use uuid::Uuid;

use crate::db::Database;
use crate::graphql::entities::{
    AppSetting, AppSettingWhereInput, Notification, NotificationWhereInput, Torrent, TorrentFile,
    TorrentFileWhereInput, TorrentWhereInput, User,
};
use crate::services::graphql::{AuthUser, LibrarianSchema};
use graphql_orm::graphql::filters::{DateFilter, IntFilter, StringFilter};
use librqbit::{AddTorrent, AddTorrentResponse};

/// Build an `eq` string filter (mirrors the helper the graphql mutation-hook
/// layer uses for entity `where` clauses).
fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

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

async fn execute_query(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    query: &str,
    variables: serde_json::Value,
) -> Result<serde_json::Value, anyhow::Error> {
    let request = Request::new(query)
        .variables(Variables::from_json(variables))
        .data(auth_user.clone())
        .data(graphql_orm::graphql::auth::AuthSubject::from_parts(
            auth_user.user_id.clone(),
            auth_user.role.iter().cloned().collect(),
            Vec::new(),
            None,
        ))
        .data(auth_user.user_id.clone());
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

async fn execute_mutation(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    query: &str,
    variables: serde_json::Value,
) -> Result<serde_json::Value, anyhow::Error> {
    if let Some(owner) = variables.get("input").and_then(|input| input.get("userId")) {
        let owner = owner
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Record ownership must be explicit"))?;
        Uuid::parse_str(owner).map_err(|_| {
            anyhow::anyhow!("System authority cannot own torrent records or notifications")
        })?;
    }
    let data = execute_query(schema, auth_user, query, variables).await?;
    check_mutation_payloads(&data)?;
    Ok(data)
}

/// Transport success is not mutation success. These internal documents select a
/// success/error payload for every mutation root, including compatibility aliases.
fn check_mutation_payloads(data: &serde_json::Value) -> anyhow::Result<()> {
    let roots = data
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Mutation response is not an object"))?;
    if roots.is_empty() {
        anyhow::bail!("Mutation response has no result payload");
    }
    let mut checked = false;
    for (name, payload) in roots {
        if name == "__typename" {
            continue;
        }
        checked = true;
        let success = payload
            .get("success")
            .or_else(|| payload.get("Success"))
            .and_then(serde_json::Value::as_bool);
        if success != Some(true) {
            let error = payload
                .get("error")
                .or_else(|| payload.get("Error"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("missing or unsuccessful mutation payload");
            anyhow::bail!("{name}: {error}");
        }
    }
    if !checked {
        anyhow::bail!("Mutation response has no result payload");
    }
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

/// Look up a live session handle by hex info_hash.
pub fn session_handle(
    session: &std::sync::Arc<librqbit::Session>,
    info_hash: &str,
) -> Option<std::sync::Arc<librqbit::ManagedTorrent>> {
    let id = hex_to_info_hash(info_hash).ok()?;
    session.get(librqbit::api::TorrentIdOrHash::Hash(id))
}

/// Record an operator-facing notification for a torrent service problem
/// (e.g. an unusable download directory) so it is visible in the UI and not
/// only in the log. Repeated restarts with the same misconfiguration reuse the
/// existing unresolved notification instead of piling up duplicates.
pub async fn create_service_notification(
    db: &Database,
    schema: &LibrarianSchema,
    actor: &AuthUser,
    title: &str,
    message: &str,
) -> Result<(), anyhow::Error> {
    let recipients = if actor.user_id == "system" && actor.is_admin() {
        let mut recipients = Vec::new();
        let mut offset = 0;
        loop {
            let page = User::query(db.pool())
                .limit(100)
                .offset(offset)
                .fetch_all()
                .await?;
            if page.is_empty() {
                break;
            }
            offset += page.len() as i64;
            recipients.extend(
                page.into_iter()
                    .filter(|user| {
                        user.is_active
                            && crate::services::graphql::auth::Role::parse(&user.role)
                                == Some(crate::services::graphql::auth::Role::Admin)
                    })
                    .map(|user| user.id),
            );
        }
        recipients
    } else {
        let user = User::get(db.pool(), &actor.user_id)
            .await?
            .filter(|user| user.is_active)
            .ok_or_else(|| anyhow::anyhow!("Notification recipient is unknown"))?;
        vec![user.id]
    };
    if recipients.is_empty() {
        anyhow::bail!("No active administrator is available for this service notification");
    }
    for owner in recipients {
        create_service_notification_for_owner(db, schema, actor, &owner, title, message).await?;
    }
    Ok(())
}

async fn create_service_notification_for_owner(
    db: &Database,
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    owner_id: &str,
    title: &str,
    message: &str,
) -> Result<(), anyhow::Error> {
    let duplicate = Notification::query(db.pool())
        .filter(NotificationWhereInput {
            user_id: Some(string_eq(owner_id)),
            category: Some(string_eq("torrent")),
            title: Some(string_eq(title)),
            resolved_at: Some(DateFilter {
                is_null: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .any(|row| row.message == message);
    if duplicate {
        return Ok(());
    }

    let data = execute_mutation(
        schema,
        auth_user,
        r#"mutation CreateTorrentServiceNotification($input: CreateNotificationInput!) {
            CreateNotification: createNotification(input: $input) { Success: success Error: error }
        }"#,
        serde_json::json!({
            "input": {
                "userId": owner_id,
                "notificationType": "error",
                "category": "torrent",
                "title": title,
                "message": message
            }
        }),
    )
    .await?;
    ensure_mutation_success(&data, "CreateNotification")
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

/// Decode an `app_settings` value that is meant to be read as a plain string.
///
/// `app_settings.value` holds **JSON text**: `seed_app_settings` writes
/// `"\"/data/downloads\""` for `torrent.download_dir`, so the stored column
/// contains the surrounding double quotes. Reading that column raw produced a
/// path with literal `"` characters in it (`<cwd>/"/data/downloads"`), which is
/// where every torrent payload silently landed instead of the configured
/// directory. Values written unquoted by older/hand-edited rows still work
/// because a non-JSON value falls back to the raw text.
pub fn decode_setting_string(raw: &str) -> String {
    let trimmed = raw.trim();
    serde_json::from_str::<String>(trimmed).unwrap_or_else(|_| trimmed.to_string())
}

/// Read a string value from app_settings, decoding the stored JSON text.
/// Filters out empty strings and the literal "null" string (legacy seed data issue).
pub async fn get_setting_string(
    pool: &Database,
    key: &str,
) -> Result<Option<String>, anyhow::Error> {
    Ok(find_setting(pool, key)
        .await?
        .map(|setting| decode_setting_string(&setting.value))
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

/// Unscoped authority for maintenance of already-owned torrent records.
pub fn system_auth_user() -> AuthUser {
    AuthUser::system_admin()
}

/// Internal acquisition boundary: intersect the authenticated role ceiling with
/// the current stored account before any administrator-only identity insertion.
async fn current_acquisition_actor(
    schema: &LibrarianSchema,
    actor: &AuthUser,
) -> anyhow::Result<AuthUser> {
    use crate::services::graphql::auth::Role;
    if actor.user_id == "system" && actor.is_admin() {
        return Ok(actor.clone());
    }
    if !actor.has_role(Role::Member) {
        anyhow::bail!("Torrent acquisition requires authenticated authority");
    }
    let data = execute_query(
        schema,
        &system_auth_user(),
        "query TorrentAcquisitionActor($id: String!) { user(id: $id) { id role isActive } }",
        serde_json::json!({"id": actor.user_id}),
    )
    .await?;
    let user = &data["user"];
    let role = user["role"]
        .as_str()
        .and_then(Role::parse)
        .context("Torrent caller is unavailable")?;
    if user["id"].as_str() != Some(actor.user_id.as_str())
        || user["isActive"].as_bool() != Some(true)
    {
        anyhow::bail!("Torrent caller is unavailable");
    }
    Ok(AuthUser {
        user_id: actor.user_id.clone(),
        email: actor.email.clone(),
        role: Some(
            if actor.is_admin() && role == Role::Admin {
                "admin"
            } else {
                "member"
            }
            .to_string(),
        ),
    })
}

/// Wanted-item linkage to stamp on a newly created torrent (auto-download,
/// phase A). All `None` for manual/RSS adds. See
/// `docs/tier1-features-plan.md` §1 and the `Torrent` entity's linkage columns.
#[derive(Debug, Clone, Copy, Default)]
pub struct WantedLinkage<'a> {
    pub episode_id: Option<&'a str>,
    pub movie_id: Option<&'a str>,
    pub track_id: Option<&'a str>,
    pub chapter_id: Option<&'a str>,
    pub show_id: Option<&'a str>,
    pub album_id: Option<&'a str>,
    pub audiobook_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TorrentCreateScope {
    pub(super) owner_id: String,
    pub(super) library_id: Option<String>,
}

async fn torrent_library_owner(
    schema: &LibrarianSchema,
    actor: &AuthUser,
    id: &str,
) -> Result<String, anyhow::Error> {
    let data = execute_query(
        schema,
        actor,
        "query TorrentLibraryOwner($id: String!) { library(id: $id) { id userId } }",
        serde_json::json!({"id": id}),
    )
    .await?;
    data["library"]["userId"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("Torrent library not found or access denied"))
}

pub(super) async fn resolve_torrent_create_owner(
    schema: &LibrarianSchema,
    actor: &AuthUser,
    library_id: Option<&str>,
    wanted: WantedLinkage<'_>,
    source_feed_id: Option<&str>,
) -> Result<TorrentCreateScope, anyhow::Error> {
    if !actor.has_role(crate::services::graphql::auth::Role::Member) {
        anyhow::bail!("Torrent creation requires authenticated authority");
    }
    let target_classes = [
        wanted.movie_id.is_some(),
        wanted.show_id.is_some() || wanted.episode_id.is_some(),
        wanted.album_id.is_some() || wanted.track_id.is_some(),
        wanted.audiobook_id.is_some() || wanted.chapter_id.is_some(),
    ];
    if target_classes
        .into_iter()
        .filter(|present| *present)
        .count()
        > 1
    {
        anyhow::bail!("Torrent linkage targets have incompatible media types");
    }
    let mut owner: Option<String> = None;
    let mut target_library: Option<String> = library_id.map(str::to_string);
    if let Some(id) = library_id {
        owner = Some(torrent_library_owner(schema, actor, id).await?);
    }
    if let Some(id) = source_feed_id {
        let data = execute_query(
            schema,
            actor,
            "query TorrentSourceFeedOwner($id: String!) { rssFeed(id: $id) { userId libraryId } }",
            serde_json::json!({ "id": id }),
        )
        .await?;
        let feed_owner = data["rssFeed"]["userId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Torrent source feed not found or access denied"))?;
        let feed_library = data["rssFeed"]["libraryId"].as_str();
        if let Some(library) = feed_library
            && torrent_library_owner(schema, actor, library).await? != feed_owner
        {
            anyhow::bail!("Torrent feed ownership disagrees with its library");
        }
        if owner.as_deref().is_some_and(|owner| owner != feed_owner)
            || target_library
                .as_deref()
                .zip(feed_library)
                .is_some_and(|(library, feed)| library != feed)
        {
            anyhow::bail!("Torrent feed and target do not share an owner and library");
        }
        owner = Some(feed_owner.to_string());
        if target_library.is_none() {
            target_library = feed_library.map(str::to_string);
        }
    }
    for (root, id, nested) in [
        ("movie", wanted.movie_id, None),
        ("show", wanted.show_id, None),
        ("album", wanted.album_id, None),
        ("audiobook", wanted.audiobook_id, None),
        ("episode", wanted.episode_id, Some("show")),
        ("track", wanted.track_id, Some("album")),
        ("chapter", wanted.chapter_id, Some("audiobook")),
    ] {
        let Some(id) = id else { continue };
        let selection = match nested {
            Some(parent) if root == "track" => {
                format!("libraryId {parent} {{ id libraryId userId }}")
            }
            Some(parent) => format!("{parent} {{ id libraryId userId }}"),
            None => "libraryId userId".to_string(),
        };
        let document = format!(
            "query TorrentTargetOwner($id: String!) {{ {root}(id: $id) {{ {selection} }} }}"
        );
        let data = execute_query(schema, actor, &document, serde_json::json!({"id": id})).await?;
        let node = nested.map_or(&data[root], |parent| &data[root][parent]);
        let target_owner = node["userId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Torrent target not found or access denied"))?;
        let library = node["libraryId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Torrent target library is unknown"))?;
        let requested_parent = match root {
            "episode" => wanted.show_id,
            "track" => wanted.album_id,
            "chapter" => wanted.audiobook_id,
            _ => None,
        };
        if requested_parent.is_some_and(|id| node["id"].as_str() != Some(id))
            || (root == "track" && data[root]["libraryId"].as_str() != Some(library))
        {
            anyhow::bail!("Torrent child does not belong to the requested parent and library");
        }
        if torrent_library_owner(schema, actor, library).await? != target_owner {
            anyhow::bail!("Torrent target ownership disagrees with its library");
        }
        if owner.as_deref().is_some_and(|owner| owner != target_owner)
            || target_library.as_deref().is_some_and(|id| id != library)
        {
            anyhow::bail!("Torrent linkage targets do not share an owner and library");
        }
        owner = Some(target_owner.to_string());
        target_library = Some(library.to_string());
    }
    let owner = owner.unwrap_or_else(|| actor.user_id.clone());
    Uuid::parse_str(&owner).map_err(|_| {
        anyhow::anyhow!("Torrent ownership is unknown; system authority is not a record owner")
    })?;
    if !actor.is_admin() && owner != actor.user_id {
        anyhow::bail!("Torrent target not found or access denied");
    }
    if actor.is_admin() {
        let data = execute_query(
            schema,
            actor,
            "query TorrentRecordOwner($id: String!) { user(id: $id) { id } }",
            serde_json::json!({ "id": owner }),
        )
        .await?;
        if data["user"]["id"].as_str() != Some(owner.as_str()) {
            anyhow::bail!("Torrent record owner is unknown");
        }
    }
    Ok(TorrentCreateScope {
        owner_id: owner,
        library_id: target_library,
    })
}

/// Insert a new torrent record.
#[allow(clippy::too_many_arguments)]
pub async fn create_torrent(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    info_hash: &str,
    magnet_uri: Option<&str>,
    library_id: Option<&str>,
    source_url: Option<&str>,
    source_indexer_id: Option<&str>,
    source_feed_id: Option<&str>,
    name: &str,
    save_path: &str,
    state: &str,
    progress: f64,
    total_bytes: i64,
    downloaded_bytes: i64,
    uploaded_bytes: i64,
    wanted: WantedLinkage<'_>,
) -> Result<(), anyhow::Error> {
    let actor = current_acquisition_actor(schema, auth_user).await?;
    let scope =
        resolve_torrent_create_owner(schema, &actor, library_id, wanted, source_feed_id).await?;
    let ts = now_iso_string();
    let data = execute_mutation(
        schema,
        // `infoHash` is intentionally administrator-only through public CRUD.
        // This checked internal action supplies the validated owner explicitly;
        // worker authority is never used as ownership or returned to the caller.
        &system_auth_user(),
        r#"mutation CreateTorrent($input: CreateTorrentInput!) {
            CreateTorrent: createTorrent(input: $input) { Success: success Error: error }
        }"#,
        serde_json::json!({
            "input": {
                "userId": scope.owner_id,
                "infoHash": info_hash,
                "magnetUri": magnet_uri,
                "libraryId": scope.library_id,
                "sourceUrl": source_url,
                "sourceIndexerId": source_indexer_id,
                "sourceFeedId": source_feed_id,
                "name": name,
                "state": state,
                "progress": progress,
                "totalBytes": total_bytes,
                "downloadedBytes": downloaded_bytes,
                "uploadedBytes": uploaded_bytes,
                "uploadedBytesTotal": uploaded_bytes,
                "savePath": save_path,
                "excludedFiles": [],
                "addedAt": ts,
                "episodeId": wanted.episode_id,
                "movieId": wanted.movie_id,
                "trackId": wanted.track_id,
                "chapterId": wanted.chapter_id,
                "showId": wanted.show_id,
                "albumId": wanted.album_id,
                "audiobookId": wanted.audiobook_id
            }
        }),
    )
    .await?;

    ensure_mutation_success(&data, "CreateTorrent")
}

/// Update mutable fields on an existing torrent row. `mark_completed_now` also
/// stamps `completedAt` in the same mutation (used when a torrent transitions
/// to done, instead of a second separate "mark completed" write).
#[allow(clippy::too_many_arguments)]
async fn update_torrent_row(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    torrent_id: &str,
    name: &str,
    state: &str,
    progress: f64,
    total_bytes: i64,
    downloaded_bytes: i64,
    uploaded_bytes: i64,
    uploaded_bytes_total: i64,
    save_path: &str,
    mark_completed_now: bool,
) -> Result<(), anyhow::Error> {
    let mut input = serde_json::json!({
        "name": name,
        "state": state,
        "progress": progress,
        "totalBytes": total_bytes,
        "downloadedBytes": downloaded_bytes,
        "uploadedBytes": uploaded_bytes,
        "uploadedBytesTotal": uploaded_bytes_total,
        "savePath": save_path
    });
    if mark_completed_now {
        input["completedAt"] = serde_json::Value::String(now_iso_string());
    }

    let data = execute_mutation(
        schema,
        auth_user,
        r#"mutation UpdateTorrent($id: String!, $input: UpdateTorrentInput!) {
            UpdateTorrent: updateTorrent(id: $id, input: $input) { Success: success Error: error }
        }"#,
        serde_json::json!({
            "id": torrent_id,
            "input": input
        }),
    )
    .await?;
    ensure_mutation_success(&data, "UpdateTorrent")
}

/// Upsert a torrent from session state (by info_hash) and return the resulting
/// DB row (fetched *before* any update is applied — callers only rely on
/// fields an update never changes, namely `id` and `excluded_files`).
///
/// Steady state (nothing tracked has changed since the last sync tick) issues
/// zero write mutations — this is the hot path called every `db_sync_loop`
/// tick (every 10s) for every torrent in the session, so a no-op fast path
/// matters at scale.
#[allow(clippy::too_many_arguments)]
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
) -> Result<Torrent, anyhow::Error> {
    match find_torrent_by_info_hash(pool, info_hash).await? {
        Some(existing) => {
            if !auth_user.is_admin() && auth_user.user_id != existing.user_id {
                anyhow::bail!("Torrent not found or access denied");
            }
            // librqbit's `uploaded_bytes` is per-session and resets on
            // restart, so the row keeps both the last session reading (as the
            // delta baseline) and a monotonic cumulative total.
            let uploaded_bytes_total = Torrent::accumulate_uploaded_bytes(
                existing.uploaded_bytes_total,
                existing.uploaded_bytes,
                uploaded_bytes,
            );
            let needs_completed_at = progress >= 1.0 && existing.completed_at.is_none();
            let unchanged = !needs_completed_at
                && existing.name == name
                && existing.state == state
                && (existing.progress - progress).abs() < 1e-9
                && existing.total_bytes == total_bytes
                && existing.downloaded_bytes == downloaded_bytes
                && existing.uploaded_bytes == uploaded_bytes
                && existing.uploaded_bytes_total == uploaded_bytes_total
                && existing.save_path == save_path;

            if !unchanged {
                update_torrent_row(
                    schema,
                    auth_user,
                    &existing.id,
                    name,
                    state,
                    progress,
                    total_bytes,
                    downloaded_bytes,
                    uploaded_bytes,
                    uploaded_bytes_total,
                    save_path,
                    needs_completed_at,
                )
                .await?;
            }

            Ok(existing)
        }
        None => {
            create_torrent(
                schema,
                auth_user,
                info_hash,
                None,
                None,
                None,
                None,
                None,
                name,
                save_path,
                state,
                progress,
                total_bytes,
                downloaded_bytes,
                uploaded_bytes,
                WantedLinkage::default(),
            )
            .await?;

            find_torrent_by_info_hash(pool, info_hash)
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!("torrent not found immediately after create: {}", info_hash)
                })
        }
    }
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
            UpdateTorrent: updateTorrent(id: $id, input: $input) { Success: success Error: error }
        }"#,
        serde_json::json!({
            "id": torrent_id,
            "input": {
                "state": state
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
            DeleteTorrent: deleteTorrent(id: $id) { Success: success Error: error }
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
#[derive(Debug, Clone, PartialEq)]
pub struct TorrentFileRow {
    pub file_index: i32,
    pub file_path: String,
    pub relative_path: String,
    pub file_size: i64,
    pub downloaded_bytes: i64,
    pub progress: f64,
    pub is_excluded: bool,
}

pub async fn completed_torrent_file_rows(
    session: &std::sync::Arc<librqbit::Session>,
    pool: &Database,
    info_hash: &str,
) -> Result<Option<Vec<TorrentFileRow>>, anyhow::Error> {
    let Some((_torrent_id, excluded_files)) = get_torrent_id_and_excluded(pool, info_hash).await?
    else {
        tracing::debug!(
            info_hash = %info_hash,
            "Skipping completed-file processing: torrent not found in database for info_hash={}",
            info_hash
        );
        return Ok(None);
    };

    let hash_id = hex_to_info_hash(info_hash)?;
    let handle = session
        .get(librqbit::api::TorrentIdOrHash::Hash(hash_id))
        .ok_or_else(|| anyhow::anyhow!("Torrent not found in session: {}", info_hash))?;
    let stats = handle.stats();
    Ok(build_file_rows(&handle, &stats, &excluded_files))
}

/// Build `TorrentFileRow` entries from a session handle's metadata and stats.
///
/// Paths come from `handle.output_folder()` — librqbit's actual destination for
/// this torrent — rather than being reconstructed from the configured download
/// directory plus `handle.name()`. librqbit picks the sub-folder for a
/// multi-file torrent itself (falling back to the longest filename when the
/// torrent has no name), so the reconstructed path could point at a file that
/// does not exist, which made the completed-file import silently skip
/// everything.
///
/// Used by `sync_session_to_database` (periodic sync) and
/// `completed_torrent_file_rows` (post-processing).
fn build_file_rows(
    handle: &librqbit::ManagedTorrent,
    stats: &librqbit::TorrentStats,
    excluded_files: &[i32],
) -> Option<Vec<TorrentFileRow>> {
    let metadata = handle.metadata.load_full()?;
    let output_folder = handle.output_folder();
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
        let full_path = output_folder
            .join(&relative_path)
            .to_string_lossy()
            .to_string();
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

/// A single-field diff between an existing `torrent_files` row and the
/// desired state built from the live session handle.
#[derive(Debug, Clone, PartialEq)]
struct TorrentFileSyncPlan {
    /// New file indices not present in the DB yet.
    to_insert: Vec<TorrentFileRow>,
    /// (existing row id, desired values) pairs whose tracked fields changed.
    to_update: Vec<(String, TorrentFileRow)>,
    /// Existing row ids with no matching desired file index anymore.
    to_delete: Vec<String>,
}

fn torrent_file_row_changed(existing: &TorrentFile, desired: &TorrentFileRow) -> bool {
    existing.file_path != desired.file_path
        || existing.relative_path != desired.relative_path
        || existing.file_size != desired.file_size
        || existing.downloaded_bytes != desired.downloaded_bytes
        || (existing.progress - desired.progress).abs() > 1e-9
        || existing.is_excluded != desired.is_excluded
}

/// Pure change-detection: compares the DB's current `torrent_files` rows for a
/// torrent against the desired state from the live session handle, keyed by
/// `file_index`. A torrent whose files haven't changed since the last sync
/// tick (the common steady-state case — completed/seeding torrents, or simply
/// no progress since the last 10s tick) produces an empty plan, so
/// `upsert_torrent_files` issues zero write mutations for it.
fn plan_torrent_file_sync(
    existing: &[TorrentFile],
    desired: &[TorrentFileRow],
) -> TorrentFileSyncPlan {
    let mut existing_by_index: std::collections::HashMap<i32, &TorrentFile> =
        existing.iter().map(|row| (row.file_index, row)).collect();

    let mut to_insert = Vec::new();
    let mut to_update = Vec::new();

    for row in desired {
        match existing_by_index.remove(&row.file_index) {
            Some(existing_row) => {
                if torrent_file_row_changed(existing_row, row) {
                    to_update.push((existing_row.id.clone(), row.clone()));
                }
            }
            None => to_insert.push(row.clone()),
        }
    }

    // Anything left has no corresponding desired row (e.g. excluded/removed files).
    let to_delete = existing_by_index
        .values()
        .map(|row| row.id.clone())
        .collect();

    TorrentFileSyncPlan {
        to_insert,
        to_update,
        to_delete,
    }
}

/// Sync `torrent_files` rows for a torrent to match the given desired state.
///
/// Replaces the previous "delete all rows then reinsert everything" approach
/// (which wrote ~1 delete + 1 insert per file on every 10s tick, forever, even
/// when nothing changed) with change detection: only rows whose tracked
/// fields differ are updated, only missing indices are inserted, and only
/// stale indices are deleted. Every mutation's result is checked (`?`) —
/// no failure is silently swallowed.
pub async fn upsert_torrent_files(
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
    db: &Database,
    torrent_id: &str,
    files: &[TorrentFileRow],
) -> Result<(), anyhow::Error> {
    let existing = TorrentFile::query(db.pool())
        .filter(TorrentFileWhereInput {
            torrent_id: Some(string_eq(torrent_id)),
            ..Default::default()
        })
        .fetch_all()
        .await?;

    let plan = plan_torrent_file_sync(&existing, files);

    if plan.to_insert.is_empty() && plan.to_update.is_empty() && plan.to_delete.is_empty() {
        return Ok(());
    }

    for id in &plan.to_delete {
        let data = execute_mutation(
            schema,
            auth_user,
            r#"mutation DeleteTorrentFile($id: String!) {
                DeleteTorrentFile: deleteTorrentFile(id: $id) { Success: success Error: error }
            }"#,
            serde_json::json!({ "id": id }),
        )
        .await?;
        ensure_mutation_success(&data, "DeleteTorrentFile")?;
    }

    for (id, f) in &plan.to_update {
        let data = execute_mutation(
            schema,
            auth_user,
            r#"mutation UpdateTorrentFile($id: String!, $input: UpdateTorrentFileInput!) {
                UpdateTorrentFile: updateTorrentFile(id: $id, input: $input) { Success: success Error: error }
            }"#,
            serde_json::json!({
                "id": id,
                "input": {
                    "filePath": f.file_path.clone(),
                    "relativePath": f.relative_path.clone(),
                    "fileSize": f.file_size,
                    "downloadedBytes": f.downloaded_bytes,
                    "progress": f.progress,
                    "isExcluded": f.is_excluded
                }
            }),
        )
        .await?;
        ensure_mutation_success(&data, "UpdateTorrentFile")?;
    }

    for f in &plan.to_insert {
        let data = execute_mutation(
            schema,
            auth_user,
            r#"mutation CreateTorrentFile($input: CreateTorrentFileInput!) {
                CreateTorrentFile: createTorrentFile(input: $input) { Success: success Error: error }
            }"#,
            serde_json::json!({
                "input": {
                    "torrentId": torrent_id,
                    "fileIndex": f.file_index,
                    "filePath": f.file_path.clone(),
                    "relativePath": f.relative_path.clone(),
                    "fileSize": f.file_size,
                    "downloadedBytes": f.downloaded_bytes,
                    "progress": f.progress,
                    "isExcluded": f.is_excluded
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
        .filter(AppSettingWhereInput {
            key: Some(string_eq(key)),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .next())
}

pub(super) async fn find_torrent_by_info_hash(
    db: &Database,
    info_hash: &str,
) -> Result<Option<Torrent>, anyhow::Error> {
    let mut rows = Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            info_hash: Some(string_eq(info_hash)),
            ..Default::default()
        })
        .limit(2)
        .fetch_all()
        .await?;
    if rows.len() > 1 {
        anyhow::bail!("Ambiguous persisted torrent identity");
    }
    Ok(rows.pop())
}

/// Torrents that finished downloading but whose completed-file post-processing
/// never ran (or never reached a terminal status) — used by the reconciliation
/// sweep as the safety net for a lost `Completed` event.
pub async fn find_unprocessed_completed_torrents(
    db: &Database,
) -> Result<Vec<Torrent>, anyhow::Error> {
    Ok(Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            progress: Some(IntFilter {
                gte: Some(1),
                ..Default::default()
            }),
            post_process_status: Some(StringFilter {
                is_null: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?)
}

/// Torrents the database still believes are in progress. The reconciliation
/// sweep cross-checks these against the live session: a row with no session
/// handle is orphaned (its resume data was lost, or the magnet never
/// re-resolved) and would otherwise sit at "downloading" forever.
pub async fn find_active_torrents(db: &Database) -> Result<Vec<Torrent>, anyhow::Error> {
    Ok(Torrent::query(db.pool())
        .fetch_all()
        .await?
        .into_iter()
        .filter(|torrent| {
            matches!(
                torrent.state.as_str(),
                "downloading" | "queued" | "checking"
            )
        })
        .collect())
}

/// Every torrent that has finished downloading, for the seeding-policy sweep.
pub async fn find_completed_torrents(db: &Database) -> Result<Vec<Torrent>, anyhow::Error> {
    Ok(Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            progress: Some(IntFilter {
                gte: Some(1),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?)
}

/// Sync all session torrents into the database (upsert by info_hash).
pub async fn sync_session_to_database(
    session: &std::sync::Arc<librqbit::Session>,
    pool: &Database,
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
) -> Result<(), anyhow::Error> {
    let session_torrents: Vec<(usize, std::sync::Arc<librqbit::ManagedTorrent>)> =
        session.with_torrents(|iter| iter.map(|(id, h)| (id, h.clone())).collect());

    for (_id, handle) in session_torrents {
        let info_hash = super::get_info_hash_hex(&handle);
        let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
        let stats = handle.stats();
        let progress = super::progress_ratio(stats.progress_bytes, stats.total_bytes);
        // Same mapping the live API and the progress events use, so a
        // `LiveTorrent.state` can never disagree with `Torrent.state`.
        let state = super::torrent_state_from_stats(&stats.state, progress).to_string();
        let save_path = handle.output_folder().to_string_lossy().to_string();

        // `upsert_from_session` fetches the current DB row once, skips the write
        // entirely if nothing tracked has changed, and returns the row (id +
        // excluded_files) so the file sync below doesn't need its own lookup.
        let torrent_row = match upsert_from_session(
            pool,
            schema,
            auth_user,
            &info_hash,
            &name,
            &state,
            progress,
            stats.total_bytes as i64,
            stats.progress_bytes as i64,
            stats.uploaded_bytes as i64,
            &save_path,
        )
        .await
        {
            Ok(row) => row,
            Err(e) => {
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
                continue;
            }
        };

        // Sync torrent_files for this torrent
        if let Some(rows) = build_file_rows(&handle, &stats, &torrent_row.excluded_files)
            && let Err(e) =
                upsert_torrent_files(schema, auth_user, pool, &torrent_row.id, &rows).await
        {
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

    Ok(())
}

/// Restore torrents from DB (list_resumable and add to session).
///
/// Returns the info hashes that were restored **paused** because
/// `torrent.max_concurrent` was already reached, so the caller can put them on
/// the start queue instead of leaving them paused forever.
///
/// Each add is bounded by `restore_timeout`: a magnet whose metadata cannot be
/// resolved otherwise blocks here indefinitely, and this function is awaited
/// inline by `TorrentService::start`, which `ServicesManager::start_all`
/// awaits in turn — one dead magnet used to stop the whole backend booting.
pub async fn restore_from_database(
    session: &std::sync::Arc<librqbit::Session>,
    pool: &Database,
    schema: &LibrarianSchema,
    auth_user: Option<&AuthUser>,
    max_concurrent: usize,
    initial_active_downloads: usize,
    restore_timeout: std::time::Duration,
) -> Result<Vec<String>, anyhow::Error> {
    let records = list_resumable(pool).await?;
    let mut active_downloads = initial_active_downloads;
    let mut queued: Vec<String> = Vec::new();
    tracing::info!(
        count = records.len(),
        "Restoring torrents from database: resumable_torrent_count={}",
        records.len()
    );

    for record in records {
        if let Some(magnet) = &record.magnet_uri {
            let start_paused = max_concurrent > 0 && active_downloads >= max_concurrent;
            let add = tokio::time::timeout(
                restore_timeout,
                session.add_torrent(
                    AddTorrent::from_url(magnet),
                    Some(add_torrent_opts(start_paused)),
                ),
            )
            .await
            .unwrap_or_else(|_| {
                Err(anyhow::anyhow!(
                    "timed out after {}s resolving torrent metadata",
                    restore_timeout.as_secs()
                ))
            });
            match add {
                Ok(AddTorrentResponse::Added(_, _)) => {
                    if start_paused {
                        queued.push(record.info_hash.clone());
                    } else {
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
                    tracing::debug!(
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

    Ok(queued)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn existing_row(
        id: &str,
        file_index: i32,
        file_size: i64,
        downloaded_bytes: i64,
    ) -> TorrentFile {
        TorrentFile {
            id: id.to_string(),
            torrent_id: "torrent-1".to_string(),
            file_index,
            file_path: format!("/downloads/file-{file_index}.mkv"),
            relative_path: format!("file-{file_index}.mkv"),
            file_size,
            downloaded_bytes,
            progress: if file_size > 0 {
                downloaded_bytes as f64 / file_size as f64
            } else {
                0.0
            },
            media_file_id: None,
            torrent: None,
            media_file: None,
            is_excluded: false,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
        }
    }

    fn desired_row(file_index: i32, file_size: i64, downloaded_bytes: i64) -> TorrentFileRow {
        TorrentFileRow {
            file_index,
            file_path: format!("/downloads/file-{file_index}.mkv"),
            relative_path: format!("file-{file_index}.mkv"),
            file_size,
            downloaded_bytes,
            progress: if file_size > 0 {
                downloaded_bytes as f64 / file_size as f64
            } else {
                0.0
            },
            is_excluded: false,
        }
    }

    // -------------------------------------------------------------------------
    // app_settings value decoding
    // -------------------------------------------------------------------------

    #[test]
    fn json_quoted_setting_values_lose_their_quotes() {
        // Exactly what `bootstrap_defaults::seed_app_settings` stores.
        assert_eq!(
            decode_setting_string("\"/data/downloads\""),
            "/data/downloads"
        );
        assert_eq!(
            decode_setting_string("  \"/tmp/librarian/downloads\"  "),
            "/tmp/librarian/downloads"
        );
    }

    #[test]
    fn unquoted_legacy_setting_values_are_returned_as_is() {
        assert_eq!(decode_setting_string("/data/downloads"), "/data/downloads");
        assert_eq!(
            decode_setting_string("  /data/downloads  "),
            "/data/downloads"
        );
    }

    #[test]
    fn json_escapes_inside_a_setting_value_are_decoded() {
        assert_eq!(
            decode_setting_string("\"/mnt/my \\\"media\\\" disk\""),
            "/mnt/my \"media\" disk"
        );
    }

    #[test]
    fn a_null_setting_value_stays_the_literal_null_so_callers_can_filter_it() {
        // `get_setting_string` drops this; decoding must not turn it into "".
        assert_eq!(decode_setting_string("null"), "null");
    }

    #[test]
    fn decoded_paths_do_not_contain_quote_characters() {
        // Regression guard for payloads landing in `<cwd>/"/data/downloads"`.
        for raw in ["\"/data/downloads\"", "/data/downloads"] {
            let decoded = decode_setting_string(raw);
            assert!(
                !decoded.contains('"'),
                "decoded path {decoded:?} still contains a quote"
            );
            assert!(
                std::path::Path::new(&decoded).is_absolute(),
                "decoded path {decoded:?} should be absolute"
            );
        }
    }

    #[test]
    fn steady_state_produces_empty_plan() {
        let existing = vec![
            existing_row("row-0", 0, 1000, 1000),
            existing_row("row-1", 1, 2000, 500),
        ];
        let desired = vec![desired_row(0, 1000, 1000), desired_row(1, 2000, 500)];

        let plan = plan_torrent_file_sync(&existing, &desired);

        assert!(plan.to_insert.is_empty());
        assert!(plan.to_update.is_empty());
        assert!(plan.to_delete.is_empty());
    }

    #[test]
    fn new_file_index_is_inserted() {
        let existing = vec![existing_row("row-0", 0, 1000, 1000)];
        let desired = vec![desired_row(0, 1000, 1000), desired_row(1, 2000, 2000)];

        let plan = plan_torrent_file_sync(&existing, &desired);

        assert_eq!(plan.to_insert, vec![desired_row(1, 2000, 2000)]);
        assert!(plan.to_update.is_empty());
        assert!(plan.to_delete.is_empty());
    }

    #[test]
    fn changed_progress_triggers_update_not_reinsert() {
        let existing = vec![existing_row("row-0", 0, 1000, 500)];
        // Same file, more bytes downloaded since the last tick.
        let desired = vec![desired_row(0, 1000, 1000)];

        let plan = plan_torrent_file_sync(&existing, &desired);

        assert!(plan.to_insert.is_empty());
        assert_eq!(
            plan.to_update,
            vec![("row-0".to_string(), desired_row(0, 1000, 1000))]
        );
        assert!(plan.to_delete.is_empty());
    }

    #[test]
    fn missing_file_index_is_deleted() {
        let existing = vec![
            existing_row("row-0", 0, 1000, 1000),
            existing_row("row-1", 1, 2000, 2000),
        ];
        // File index 1 no longer present in the torrent (e.g. re-hashed/excluded).
        let desired = vec![desired_row(0, 1000, 1000)];

        let plan = plan_torrent_file_sync(&existing, &desired);

        assert!(plan.to_insert.is_empty());
        assert!(plan.to_update.is_empty());
        assert_eq!(plan.to_delete, vec!["row-1".to_string()]);
    }

    #[test]
    fn excluded_flag_change_triggers_update() {
        let mut existing = existing_row("row-0", 0, 1000, 1000);
        existing.is_excluded = false;
        let mut desired = desired_row(0, 1000, 1000);
        desired.is_excluded = true;

        let plan = plan_torrent_file_sync(&[existing], &[desired.clone()]);

        assert_eq!(plan.to_update, vec![("row-0".to_string(), desired)]);
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::*;
    use crate::graphql::entities::{
        AutoDownloadMode, CreateEpisodeInput, CreateLibraryInput, CreateShowInput, CreateUserInput,
        Episode, Library, Show, User,
    };
    use std::sync::Arc;

    async fn database() -> Database {
        let db = Database::new(
            graphql_orm::DbPool::connect("sqlite::memory:")
                .await
                .unwrap(),
        );
        crate::services::database::bootstrap_schema(&db)
            .await
            .unwrap();
        crate::services::DatabaseService::new(db).pool().clone()
    }

    async fn user(db: &Database) -> User {
        User::insert(
            db,
            CreateUserInput {
                username: uuid::Uuid::new_v4().to_string(),
                email: None,
                password_hash: "fixture password_hash".to_string(),
                role: "member".to_string(),
                display_name: None,
                avatar_url: None,
                is_active: true,
                last_login_at: None,
            },
        )
        .await
        .unwrap()
    }
    async fn library(db: &Database, user: &User) -> Library {
        Library::insert(
            db,
            CreateLibraryInput {
                user_id: user.id.clone(),
                name: "fixture library".to_string(),
                path: format!("/fixture/{}", uuid::Uuid::new_v4()),
                library_type: "music".to_string(),
                icon: None,
                color: None,
                auto_scan: false,
                auto_organize: false,
                naming_pattern: "fixture naming_pattern".to_string(),
                scan_interval_minutes: 0,
                watch_for_changes: false,
                scanning: false,
                last_scanned_at: None,
                quality_profile_id: None,
            },
        )
        .await
        .unwrap()
    }
    async fn show(db: &Database, library: &Library) -> Show {
        Show::insert(
            db,
            CreateShowInput {
                library_id: library.id.clone(),
                user_id: library.user_id.clone(),
                name: "fixture show".to_string(),
                sort_name: None,
                year: None,
                tvmaze_id: None,
                tmdb_id: None,
                tvdb_id: None,
                imdb_id: None,
                overview: None,
                network: None,
                runtime: None,
                genres: Vec::new(),
                poster_url: None,
                backdrop_url: None,
                content_rating: None,
                auto_download: false,
                auto_download_mode: AutoDownloadMode::None,
                quality_profile_id: None,
                path: None,
            },
        )
        .await
        .unwrap()
    }

    async fn episode(db: &Database, show: &Show, file_id: Option<&str>) -> Episode {
        Episode::insert(
            db,
            CreateEpisodeInput {
                show_id: show.id.clone(),
                season: 0,
                episode: 0,
                absolute_number: None,
                title: None,
                overview: None,
                air_date: None,
                air_stamp: None,
                runtime: None,
                still_url: None,
                tvmaze_id: None,
                tmdb_id: None,
                tvdb_id: None,
                wanted: file_id.is_none(),
                ignored: None,
                media_file_id: file_id.map(str::to_string),
            },
        )
        .await
        .unwrap()
    }

    fn schema(db: &Database) -> LibrarianSchema {
        crate::services::graphql::build_schema(
            db.clone(),
            (),
            Arc::new(crate::services::ServicesManager::new()),
        )
    }

    async fn create(
        schema: &LibrarianSchema,
        actor: &AuthUser,
        hash: &str,
        library: Option<&str>,
    ) -> anyhow::Result<()> {
        create_torrent(
            schema,
            actor,
            hash,
            None,
            library,
            None,
            None,
            None,
            "fixture",
            "/fixture/downloads",
            "initializing",
            0.0,
            10,
            0,
            0,
            WantedLinkage::default(),
        )
        .await
    }

    #[tokio::test]
    async fn admin_library_import_keeps_target_owner_and_system_sync_preserves_it() {
        let db = database().await;
        let owner = user(&db).await;
        let admin = user(&db).await;
        User::update_by_id(
            &db,
            &admin.id,
            crate::graphql::entities::UpdateUserInput {
                role: Some("admin".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let lib = library(&db, &owner).await;
        let schema = schema(&db);
        let actor = AuthUser::system_admin_for(&admin.id);
        let hash = "a".repeat(40);
        create(&schema, &actor, &hash, Some(&lib.id)).await.unwrap();
        let created = find_torrent_by_info_hash(&db, &hash)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(created.user_id, owner.id);
        upsert_from_session(
            &db,
            &schema,
            &system_auth_user(),
            &hash,
            "renamed",
            "live",
            0.5,
            10,
            5,
            0,
            "/fixture/downloads",
        )
        .await
        .unwrap();
        assert_eq!(
            find_torrent_by_info_hash(&db, &hash)
                .await
                .unwrap()
                .unwrap()
                .user_id,
            owner.id
        );
    }

    #[tokio::test]
    async fn unknown_runtime_owner_and_foreign_library_creation_fail_closed() {
        let db = database().await;
        let owner = user(&db).await;
        let other = user(&db).await;
        let lib = library(&db, &owner).await;
        let schema = schema(&db);
        assert_eq!(system_auth_user().user_id, "system");
        let actor = AuthUser {
            user_id: other.id.clone(),
            email: None,
            role: Some("member".to_string()),
        };
        assert!(
            create(&schema, &actor, &"b".repeat(40), Some(&lib.id))
                .await
                .is_err()
        );
        assert!(
            upsert_from_session(
                &db,
                &schema,
                &system_auth_user(),
                &"c".repeat(40),
                "unknown",
                "live",
                0.0,
                1,
                0,
                0,
                "/fixture/downloads"
            )
            .await
            .is_err()
        );
        assert!(
            Torrent::query(db.pool())
                .fetch_all()
                .await
                .unwrap()
                .is_empty()
        );
        create(&schema, &actor, &"d".repeat(40), None)
            .await
            .unwrap();
        let row = find_torrent_by_info_hash(&db, &"d".repeat(40))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.user_id, other.id);
    }

    #[tokio::test]
    async fn checked_member_acquisition_persists_identity_but_public_member_identity_writes_fail() {
        let db = database().await;
        let owner = user(&db).await;
        let other = user(&db).await;
        let lib = library(&db, &owner).await;
        let foreign = library(&db, &other).await;
        let schema = schema(&db);
        let actor = AuthUser {
            user_id: owner.id.clone(),
            email: None,
            role: Some("member".to_string()),
        };
        let hash = "f".repeat(40);
        let public_create = execute_mutation(&schema, &actor,
            "mutation($input: CreateTorrentInput!) { createTorrent(input: $input) { success error } }",
            serde_json::json!({"input": {"userId": owner.id, "infoHash":hash, "libraryId":lib.id, "name":"member grab", "state":"paused", "progress":0.0,
                "totalBytes":1,"downloadedBytes":0,"uploadedBytes":0,"uploadedBytesTotal":0,"savePath":"/fixture/downloads","excludedFiles":[],"addedAt":now_iso_string()}}),
        ).await;
        assert!(public_create.is_err());
        assert!(
            find_torrent_by_info_hash(&db, &hash)
                .await
                .unwrap()
                .is_none()
        );
        create(&schema, &actor, &hash, Some(&lib.id)).await.unwrap();
        let row = find_torrent_by_info_hash(&db, &hash)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.user_id, owner.id);
        assert_eq!(row.library_id, Some(lib.id.clone()));
        assert!(execute_mutation(&schema, &actor,
            "mutation($id: String!, $input: UpdateTorrentInput!) { updateTorrent(id: $id, input: $input) { success error } }",
            serde_json::json!({"id":row.id,"input":{"infoHash":"0".repeat(40)}}),
        ).await.is_err());
        assert_eq!(
            find_torrent_by_info_hash(&db, &hash)
                .await
                .unwrap()
                .unwrap()
                .id,
            row.id
        );
        assert!(
            create(
                &schema,
                &AuthUser::system_admin_for(&owner.id),
                &"1".repeat(40),
                Some(&foreign.id)
            )
            .await
            .is_err()
        );
        User::update_by_id(
            &db,
            &owner.id,
            crate::graphql::entities::UpdateUserInput {
                is_active: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(
            create(&schema, &actor, &"2".repeat(40), Some(&lib.id))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn system_target_bindings_require_one_real_parent_library_and_owner() {
        let db = database().await;
        let owner = user(&db).await;
        let other = user(&db).await;
        let lib = library(&db, &owner).await;
        let other_lib = library(&db, &other).await;
        let parent = show(&db, &lib).await;
        let sibling = show(&db, &lib).await;
        let child = episode(&db, &parent, None).await;
        let schema = schema(&db);
        let actor = system_auth_user();
        let wanted = WantedLinkage {
            episode_id: Some(&child.id),
            show_id: Some(&parent.id),
            ..Default::default()
        };
        let scope = resolve_torrent_create_owner(&schema, &actor, None, wanted, None)
            .await
            .unwrap();
        assert_eq!(scope.owner_id, owner.id);
        assert_eq!(scope.library_id.as_deref(), Some(lib.id.as_str()));
        assert!(
            resolve_torrent_create_owner(&schema, &actor, Some(&other_lib.id), wanted, None)
                .await
                .is_err()
        );
        assert!(
            resolve_torrent_create_owner(
                &schema,
                &actor,
                None,
                WantedLinkage {
                    show_id: Some(&sibling.id),
                    ..wanted
                },
                None
            )
            .await
            .is_err()
        );
        assert!(
            resolve_torrent_create_owner(
                &schema,
                &actor,
                None,
                WantedLinkage {
                    movie_id: Some("unknown"),
                    ..wanted
                },
                None
            )
            .await
            .is_err()
        );
        assert!(
            resolve_torrent_create_owner(
                &schema,
                &actor,
                None,
                WantedLinkage {
                    episode_id: Some("unknown"),
                    ..Default::default()
                },
                None
            )
            .await
            .is_err()
        );
        assert!(
            Torrent::query(db.pool())
                .fetch_all()
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn system_updates_and_cleanup_do_not_require_an_arbitrary_human_actor() {
        let db = database().await;
        let owner = user(&db).await;
        let lib = library(&db, &owner).await;
        let schema = schema(&db);
        let hash = "e".repeat(40);
        create(&schema, &system_auth_user(), &hash, Some(&lib.id))
            .await
            .unwrap();
        update_state(&db, &schema, &system_auth_user(), &hash, "error")
            .await
            .unwrap();
        let failed = find_torrent_by_info_hash(&db, &hash)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(failed.user_id, owner.id);
        assert_eq!(failed.state, "error");
        delete_torrent(&db, &schema, &system_auth_user(), &hash)
            .await
            .unwrap();
        assert!(
            find_torrent_by_info_hash(&db, &hash)
                .await
                .unwrap()
                .is_none()
        );
        delete_torrent(&db, &schema, &system_auth_user(), &hash)
            .await
            .unwrap();
    }

    #[test]
    fn torrent_mutation_payload_failure_is_not_transport_success() {
        assert!(
            check_mutation_payloads(
                &serde_json::json!({"createTorrent": {"success": false, "error": "denied"}})
            )
            .is_err()
        );
        assert!(
            check_mutation_payloads(&serde_json::json!({"CreateTorrent": {"Success": true}}))
                .is_ok()
        );
        assert!(check_mutation_payloads(&serde_json::json!({"createTorrent": null})).is_err());
    }

    #[tokio::test]
    async fn service_notifications_have_explicit_admin_recipients_and_never_pick_a_member() {
        let db = database().await;
        let member = user(&db).await;
        let admin = user(&db).await;
        let schema = schema(&db);
        let actor = system_auth_user();
        assert!(
            create_service_notification(&db, &schema, &actor, "storage", "unavailable")
                .await
                .is_err()
        );
        assert!(
            Notification::query(db.pool())
                .fetch_all()
                .await
                .unwrap()
                .is_empty()
        );
        User::update_by_id(
            &db,
            &admin.id,
            crate::graphql::entities::UpdateUserInput {
                role: Some("admin".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        for _ in 0..2 {
            create_service_notification(&db, &schema, &actor, "storage", "unavailable")
                .await
                .unwrap();
        }
        let notifications = Notification::query(db.pool()).fetch_all().await.unwrap();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].user_id, admin.id);
        assert_ne!(notifications[0].user_id, member.id);
    }
}
