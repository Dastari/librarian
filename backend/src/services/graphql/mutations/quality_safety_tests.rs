//! Security regressions through generated fields, real entity scope, and disposable files.
//! No network services are started by these tests.
use super::*;
use crate::graphql::entities::*;
use async_graphql::{Request, Variables};
use graphql_orm::graphql::auth::AuthSubject;
use graphql_orm::graphql::orm::{
    ChangeAction, MutationContext, MutationEvent, MutationHook, MutationPhase,
};
use graphql_orm::graphql::subscriptions::SubscriptionEvent;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

async fn database() -> Database {
    let db = Database::connect_sqlite("sqlite::memory:").await.unwrap();
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

async fn library(db: &Database, user: &User, root: &Path) -> Library {
    Library::insert(
        db,
        CreateLibraryInput {
            user_id: user.id.clone(),
            name: "fixture library".to_string(),
            path: root.to_string_lossy().to_string(),
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

async fn movie(db: &Database, library: &Library) -> Movie {
    Movie::insert(
        db,
        CreateMovieInput {
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            title: "fixture movie".to_string(),
            sort_title: None,
            original_title: None,
            year: None,
            tmdb_id: None,
            imdb_id: None,
            overview: None,
            tagline: None,
            runtime: None,
            genres: Vec::new(),
            director: None,
            cast_names: Vec::new(),
            production_countries: Vec::new(),
            spoken_languages: Vec::new(),
            tmdb_rating: None,
            tmdb_vote_count: None,
            poster_url: None,
            backdrop_url: None,
            collection_id: None,
            collection_name: None,
            collection_poster_url: None,
            release_date: None,
            certification: None,
            monitored: false,
            tmdb_status: None,
            wanted: true,
            ignored: None,
            download_status: None,
            has_file: false,
            media_file_id: None,
            quality_profile_id: None,
        },
    )
    .await
    .unwrap()
}

async fn media_file(
    db: &Database,
    library: &Library,
    size: i64,
    movie: &Movie,
    path: &Path,
) -> MediaFile {
    MediaFile::insert(
        db,
        CreateMediaFileInput {
            library_id: Some(library.id.clone()),
            episode_id: None,
            movie_id: Some(movie.id.clone()),
            track_id: None,
            chapter_id: None,
            path: path.to_string_lossy().to_string(),
            relative_path: None,
            original_name: None,
            size,
            container: None,
            video_codec: None,
            audio_codec: None,
            width: None,
            height: None,
            duration: None,
            bitrate: None,
            resolution: None,
            is_hdr: false,
            hdr_type: None,
            audio_channels: None,
            metadata: None,
            content_type: None,
            added_at: chrono::Utc::now().to_rfc3339(),
            analyzed_at: None,
            file_modified_at: None,
            match_type: None,
            matched_by_user_id: None,
            match_confirmed_at: None,
            quality_status: None,
        },
    )
    .await
    .unwrap()
}

fn torrent_input(user: &AuthUser, hash: &str) -> CreateTorrentInput {
    CreateTorrentInput {
        user_id: user.user_id.clone(),
        info_hash: hash.into(),
        magnet_uri: None,
        name: "persisted name".into(),
        state: "paused".into(),
        progress: 0.1,
        total_bytes: 100,
        downloaded_bytes: 10,
        uploaded_bytes: 0,
        uploaded_bytes_total: 0,
        save_path: "/test/downloads".into(),
        download_path: None,
        source_url: None,
        source_feed_id: None,
        source_indexer_id: None,
        library_id: None,
        post_process_status: None,
        post_process_error: None,
        processed_at: None,
        episode_id: None,
        movie_id: None,
        track_id: None,
        chapter_id: None,
        show_id: None,
        album_id: None,
        audiobook_id: None,
        minimum_ratio: None,
        minimum_seed_time_minutes: None,
        season: None,
        excluded_files: vec![],
        added_at: "2026-10-24T00:00:00Z".into(),
        completed_at: None,
    }
}

struct Fixture {
    _temp: tempfile::TempDir,
    db: Database,
    owner: User,
    admin: AuthUser,
    library: Library,
    movie: Movie,
    file: MediaFile,
    torrent: Torrent,
    torrent_file: TorrentFile,
    pending: PendingFileMatch,
    notification: Notification,
    source: PathBuf,
    target: PathBuf,
}

async fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let downloads = temp.path().join("downloads");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&downloads).unwrap();
    let source = downloads.join("Movie.2160p.mkv");
    let target = root.join("Movie.mkv");
    std::fs::write(&source, b"verified new source bytes").unwrap();
    std::fs::write(&target, b"original target bytes").unwrap();
    let db = database().await;
    let owner = user(&db).await;
    let administrator = user(&db).await;
    User::update_by_id(
        &db,
        &administrator.id,
        UpdateUserInput {
            role: Some("admin".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let admin = AuthUser::system_admin_for(&administrator.id);
    let library = library(&db, &owner, &root).await;
    let movie = movie(&db, &library).await;
    let file = media_file(
        &db,
        &library,
        std::fs::metadata(&target).unwrap().len() as i64,
        &movie,
        &target,
    )
    .await;
    let owner_actor = AuthUser {
        user_id: owner.id.clone(),
        role: Some("member".to_string()),
        email: None,
    };
    let mut input = torrent_input(&owner_actor, &"a".repeat(40));
    input.library_id = Some(library.id.clone());
    input.movie_id = Some(movie.id.clone());
    input.save_path = downloads.to_string_lossy().to_string();
    let torrent = Torrent::insert(&db, input).await.unwrap();
    let torrent_file = TorrentFile::insert(
        &db,
        CreateTorrentFileInput {
            torrent_id: torrent.id.clone(),
            file_index: 0,
            file_path: source.to_string_lossy().to_string(),
            relative_path: "Movie.2160p.mkv".to_string(),
            file_size: std::fs::metadata(&source).unwrap().len() as i64,
            downloaded_bytes: std::fs::metadata(&source).unwrap().len() as i64,
            progress: 1.0,
            media_file_id: None,
            is_excluded: false,
        },
    )
    .await
    .unwrap();
    let pending = PendingFileMatch::insert(
        &db,
        pending_input(&owner.id, &torrent, &torrent_file, &movie),
    )
    .await
    .unwrap();
    let (pending_id, action_data) =
        build_quality_upgrade_action_data(&db, &owner.id, &library.id, &file.id, &source, &target)
            .await
            .unwrap();
    assert_eq!(pending_id, pending.id);
    let notification = Notification::insert(
        &db,
        CreateNotificationInput {
            user_id: owner.id.clone(),
            notification_type: "ACTION_REQUIRED".to_string(),
            category: "QUALITY".to_string(),
            title: "Upgrade".to_string(),
            message: "Verified upgrade".to_string(),
            library_id: Some(library.id.clone()),
            torrent_id: Some(torrent.id.clone()),
            media_file_id: Some(file.id.clone()),
            pending_match_id: Some(pending.id.clone()),
            action_type: Some(QUALITY_UPGRADE_ACTION_TYPE.to_string()),
            action_data: Some(action_data),
            read_at: None,
            resolved_at: None,
            resolution: None,
        },
    )
    .await
    .unwrap();
    Fixture {
        _temp: temp,
        db,
        owner,
        admin,
        library,
        movie,
        file,
        torrent,
        torrent_file,
        pending,
        notification,
        source,
        target,
    }
}

fn pending_input(
    owner: &str,
    torrent: &Torrent,
    file: &TorrentFile,
    movie: &Movie,
) -> CreatePendingFileMatchInput {
    CreatePendingFileMatchInput {
        user_id: owner.to_string(),
        source_path: file.file_path.clone(),
        source_type: "torrent".to_string(),
        source_id: Some(torrent.info_hash.clone()),
        source_file_index: Some(file.file_index),
        file_size: file.file_size,
        movie_id: Some(movie.id.clone()),
        episode_id: None,
        track_id: None,
        chapter_id: None,
        unmatched_reason: None,
        match_type: Some("auto".to_string()),
        match_confidence: Some(1.0),
        match_attempts: 1,
        verification_status: None,
        verification_reason: None,
        parsed_resolution: None,
        parsed_codec: None,
        parsed_source: None,
        parsed_audio: None,
        copied_at: None,
        copy_error: None,
        copy_attempts: 0,
    }
}

async fn replace_action(f: &Fixture, data: serde_json::Value) {
    Notification::update_by_id(
        &f.db,
        &f.notification.id,
        UpdateNotificationInput {
            action_data: Some(Some(data.to_string())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
}
fn member(user: &User) -> AuthUser {
    AuthUser {
        user_id: user.id.clone(),
        email: None,
        role: Some("member".to_string()),
    }
}
fn assert_original_files(f: &Fixture) {
    assert_eq!(
        std::fs::read(&f.source).unwrap(),
        b"verified new source bytes"
    );
    assert_eq!(std::fs::read(&f.target).unwrap(), b"original target bytes");
}

#[tokio::test]
async fn approval_rederives_owned_files_and_preserves_seeding_source_bytes() {
    let f = fixture().await;
    assert!(
        apply_quality_upgrade(&f.db, &member(&f.owner), &f.notification.id)
            .await
            .is_err()
    );
    assert!(
        apply_quality_upgrade(
            &f.db,
            &AuthUser::system_admin_for(&f.owner.id),
            &f.notification.id
        )
        .await
        .is_err()
    );
    assert_original_files(&f);
    apply_quality_upgrade(&f.db, &f.admin, &f.notification.id)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(&f.source).unwrap(),
        b"verified new source bytes"
    );
    assert_eq!(
        std::fs::read(&f.target).unwrap(),
        b"verified new source bytes"
    );
    assert_eq!(
        Notification::get(f.db.pool(), &f.notification.id)
            .await
            .unwrap()
            .unwrap()
            .resolution
            .as_deref(),
        Some("ACCEPTED")
    );
    assert!(
        PendingFileMatch::get(f.db.pool(), &f.pending.id)
            .await
            .unwrap()
            .unwrap()
            .copied_at
            .is_some()
    );
    let file = MediaFile::get(f.db.pool(), &f.file.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        file.size,
        std::fs::metadata(&f.source).unwrap().len() as i64
    );
    assert_eq!(file.movie_id.as_deref(), Some(f.movie.id.as_str()));
    std::fs::write(&f.target, b"later library edit").unwrap();
    assert_eq!(
        std::fs::read(&f.source).unwrap(),
        b"verified new source bytes"
    );
    assert!(
        std::fs::read_dir(f.target.parent().unwrap())
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".librarian-upgrade-"))
    );
}

#[tokio::test]
async fn forged_envelopes_and_legacy_paths_cannot_replace_any_file() {
    let f = fixture().await;
    let original: serde_json::Value =
        serde_json::from_str(f.notification.action_data.as_deref().unwrap()).unwrap();
    let outside = f._temp.path().join("outside.mkv");
    std::fs::write(&outside, b"outside sentinel").unwrap();
    let mut payloads = Vec::new();
    let mut source = original.clone();
    source["sourcePath"] = serde_json::json!(outside);
    source["sourceProof"] =
        serde_json::to_value(UpgradeFileProof::capture(&outside).unwrap()).unwrap();
    payloads.push(source);
    let mut target = original.clone();
    target["targetPath"] = serde_json::json!(outside);
    target["targetProof"] =
        serde_json::to_value(UpgradeFileProof::capture(&outside).unwrap()).unwrap();
    payloads.push(target);
    let mut wrong_id = original.clone();
    wrong_id["mediaFileId"] = serde_json::json!(Uuid::new_v4().to_string());
    payloads.push(wrong_id);
    let mut wrong_pending = original.clone();
    wrong_pending["pendingMatchId"] = serde_json::json!(Uuid::new_v4().to_string());
    payloads.push(wrong_pending);
    let mut wrong_proof = original.clone();
    wrong_proof["sourceProof"]["size"] = serde_json::json!(0);
    payloads.push(wrong_proof);
    payloads.push(serde_json::json!({"sourcePath": f.source, "targetPath": f.target, "mediaFileId": f.file.id}));
    for payload in payloads {
        replace_action(&f, payload).await;
        assert!(
            apply_quality_upgrade(&f.db, &f.admin, &f.notification.id)
                .await
                .is_err()
        );
        assert_original_files(&f);
        assert_eq!(std::fs::read(&outside).unwrap(), b"outside sentinel");
        assert!(
            Notification::get(f.db.pool(), &f.notification.id)
                .await
                .unwrap()
                .unwrap()
                .resolved_at
                .is_none()
        );
    }
}

#[tokio::test]
async fn pending_source_paths_need_complete_unique_torrent_file_and_matching_owner() {
    let f = fixture().await;
    let outside = f._temp.path().join("unregistered-source.mkv");
    std::fs::write(&outside, b"unregistered source bytes").unwrap();
    PendingFileMatch::update_by_id(
        &f.db,
        &f.pending.id,
        UpdatePendingFileMatchInput {
            source_path: Some(outside.to_string_lossy().to_string()),
            file_size: Some(std::fs::metadata(&outside).unwrap().len() as i64),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        build_quality_upgrade_action_data(
            &f.db,
            &f.owner.id,
            &f.library.id,
            &f.file.id,
            &outside,
            &f.target
        )
        .await
        .is_err()
    );
    PendingFileMatch::update_by_id(
        &f.db,
        &f.pending.id,
        UpdatePendingFileMatchInput {
            source_path: Some(f.source.to_string_lossy().to_string()),
            file_size: Some(f.torrent_file.file_size),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    TorrentFile::update_by_id(
        &f.db,
        &f.torrent_file.id,
        UpdateTorrentFileInput {
            downloaded_bytes: Some(0),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(read_upgrade_plan(&f.db, &f.notification.id).await.is_err());
    TorrentFile::update_by_id(
        &f.db,
        &f.torrent_file.id,
        UpdateTorrentFileInput {
            downloaded_bytes: Some(f.torrent_file.file_size),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let other = user(&f.db).await;
    PendingFileMatch::update_by_id(
        &f.db,
        &f.pending.id,
        UpdatePendingFileMatchInput {
            user_id: Some(other.id),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(read_upgrade_plan(&f.db, &f.notification.id).await.is_err());
    PendingFileMatch::update_by_id(
        &f.db,
        &f.pending.id,
        UpdatePendingFileMatchInput {
            user_id: Some(f.owner.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    PendingFileMatch::insert(
        &f.db,
        pending_input(&f.owner.id, &f.torrent, &f.torrent_file, &f.movie),
    )
    .await
    .unwrap();
    assert!(
        build_quality_upgrade_action_data(
            &f.db,
            &f.owner.id,
            &f.library.id,
            &f.file.id,
            &f.source,
            &f.target
        )
        .await
        .is_err()
    );
    assert_original_files(&f);
}

#[tokio::test]
async fn completed_extraction_manifest_is_required_and_bound_to_the_owned_archive() {
    let f = fixture().await;
    let archive = f.source.with_file_name("release.zip");
    std::fs::write(&archive, b"archive fixture").unwrap();
    let size = std::fs::metadata(&archive).unwrap().len() as i64;
    TorrentFile::update_by_id(
        &f.db,
        &f.torrent_file.id,
        UpdateTorrentFileInput {
            file_path: Some(archive.to_string_lossy().to_string()),
            relative_path: Some("release.zip".to_string()),
            file_size: Some(size),
            downloaded_bytes: Some(size),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let staging = archive
        .parent()
        .unwrap()
        .join(crate::services::extract::STAGING_DIR_NAME)
        .join(&f.torrent.info_hash)
        .join("release.zip");
    std::fs::create_dir_all(&staging).unwrap();
    let extracted = staging.join("movie.mkv");
    std::fs::write(&extracted, b"verified extracted bytes").unwrap();
    let extracted_size = std::fs::metadata(&extracted).unwrap().len() as i64;
    PendingFileMatch::update_by_id(
        &f.db,
        &f.pending.id,
        UpdatePendingFileMatchInput {
            source_path: Some(extracted.to_string_lossy().to_string()),
            source_file_index: Some(None),
            file_size: Some(extracted_size),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        build_quality_upgrade_action_data(
            &f.db,
            &f.owner.id,
            &f.library.id,
            &f.file.id,
            &extracted,
            &f.target
        )
        .await
        .is_err()
    );
    let marker = staging.join(".librarian-extract.json");
    let entry = serde_json::json!({"relative_path":"movie.mkv", "size":extracted_size});
    std::fs::write(
        &marker,
        serde_json::json!({"archive":"release.zip", "files":[entry.clone(), entry.clone()]})
            .to_string(),
    )
    .unwrap();
    assert!(
        build_quality_upgrade_action_data(
            &f.db,
            &f.owner.id,
            &f.library.id,
            &f.file.id,
            &extracted,
            &f.target
        )
        .await
        .is_err()
    );
    std::fs::write(
        &marker,
        serde_json::json!({"archive":"release.zip", "files":[entry]}).to_string(),
    )
    .unwrap();
    let (_, action) = build_quality_upgrade_action_data(
        &f.db,
        &f.owner.id,
        &f.library.id,
        &f.file.id,
        &extracted,
        &f.target,
    )
    .await
    .unwrap();
    replace_action(&f, serde_json::from_str(&action).unwrap()).await;
    apply_quality_upgrade(&f.db, &f.admin, &f.notification.id)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(&f.target).unwrap(),
        b"verified extracted bytes"
    );
    assert_eq!(
        std::fs::read(&extracted).unwrap(),
        b"verified extracted bytes"
    );
    assert_eq!(std::fs::read(&archive).unwrap(), b"archive fixture");
    assert_eq!(
        std::fs::read(&f.source).unwrap(),
        b"verified new source bytes"
    );
}

#[tokio::test]
async fn forged_current_media_path_and_matching_json_proof_still_cannot_escape_library() {
    let f = fixture().await;
    let victim = f._temp.path().join("outside-target.mkv");
    std::fs::write(&victim, b"outside target bytes").unwrap();
    let mut action: serde_json::Value =
        serde_json::from_str(f.notification.action_data.as_deref().unwrap()).unwrap();
    action["targetPath"] = serde_json::json!(victim);
    action["targetProof"] =
        serde_json::to_value(UpgradeFileProof::capture(&victim).unwrap()).unwrap();
    MediaFile::update_by_id(
        &f.db,
        &f.file.id,
        UpdateMediaFileInput {
            path: Some(victim.to_string_lossy().to_string()),
            size: Some(std::fs::metadata(&victim).unwrap().len() as i64),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    replace_action(&f, action).await;
    assert!(
        apply_quality_upgrade(&f.db, &f.admin, &f.notification.id)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&victim).unwrap(), b"outside target bytes");
    assert_original_files(&f);
}

#[tokio::test]
async fn changed_file_identity_after_staging_does_not_remove_the_target() {
    let f = fixture().await;
    let plan = read_upgrade_plan(&f.db, &f.notification.id).await.unwrap();
    let staging = stage_upgrade(&plan).unwrap();
    let other = f.source.with_extension("replacement");
    std::fs::write(&other, b"verified new source bytes").unwrap();
    std::fs::rename(&other, &f.source).unwrap();
    assert!(finish_upgrade(&plan, &staging).is_err());
    assert_original_files(&f);
    drop(staging);
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_targets_and_hardlinked_sources_fail_before_replacement() {
    use std::os::unix::fs::symlink;
    let f = fixture().await;
    let outside = f._temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    let victim = outside.join("victim.mkv");
    std::fs::write(&victim, b"outside sentinel").unwrap();
    let link = Path::new(&f.library.path).join("escape");
    symlink(&outside, &link).unwrap();
    MediaFile::update_by_id(
        &f.db,
        &f.file.id,
        UpdateMediaFileInput {
            path: Some(link.join("victim.mkv").to_string_lossy().to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(read_upgrade_plan(&f.db, &f.notification.id).await.is_err());
    assert_eq!(std::fs::read(&victim).unwrap(), b"outside sentinel");
    MediaFile::update_by_id(
        &f.db,
        &f.file.id,
        UpdateMediaFileInput {
            path: Some(f.target.to_string_lossy().to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    std::fs::remove_file(&f.target).unwrap();
    std::fs::hard_link(&f.source, &f.target).unwrap();
    assert!(
        build_quality_upgrade_action_data(
            &f.db,
            &f.owner.id,
            &f.library.id,
            &f.file.id,
            &f.source,
            &f.target
        )
        .await
        .is_err()
    );
    assert_eq!(
        std::fs::read(&f.source).unwrap(),
        b"verified new source bytes"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn approved_root_alias_keeps_the_recorded_path_for_reanalysis() {
    let f = fixture().await;
    let alias = f._temp.path().join("approved-alias");
    std::os::unix::fs::symlink(&f.library.path, &alias).unwrap();
    let recorded = alias.join(f.target.file_name().unwrap());
    Library::update_by_id(
        &f.db,
        &f.library.id,
        UpdateLibraryInput {
            path: Some(alias.to_string_lossy().to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    MediaFile::update_by_id(
        &f.db,
        &f.file.id,
        UpdateMediaFileInput {
            path: Some(recorded.to_string_lossy().to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let (_, reanalysis_path) = apply_quality_upgrade(&f.db, &f.admin, &f.notification.id)
        .await
        .unwrap();
    assert_eq!(reanalysis_path, recorded);
    assert_eq!(
        std::fs::read(&recorded).unwrap(),
        b"verified new source bytes"
    );
    assert_eq!(
        std::fs::read(&f.source).unwrap(),
        b"verified new source bytes"
    );
}

struct FailApprovalOnce {
    previous: Arc<dyn MutationHook>,
    enabled: AtomicBool,
}
impl MutationHook for FailApprovalOnce {
    fn on_mutation<'a>(
        &'a self,
        ctx: Option<&'a async_graphql::Context<'_>>,
        tx: &'a mut MutationContext<'_>,
        event: &'a MutationEvent,
    ) -> futures::future::BoxFuture<'a, async_graphql::Result<()>> {
        Box::pin(async move {
            self.previous.on_mutation(ctx, tx, event).await?;
            if event.entity_name == "Notification"
                && event.action == ChangeAction::Updated
                && event.phase == MutationPhase::After
                && self.enabled.swap(false, Ordering::AcqRel)
            {
                return Err(async_graphql::Error::new(
                    "injected approval persistence failure",
                ));
            }
            Ok(())
        })
    }
}

#[tokio::test]
async fn generated_write_failure_rolls_back_without_replacing_then_retry_succeeds() {
    let mut f = fixture().await;
    let mut events =
        f.db.ensure_event_sender::<SubscriptionEvent<MediaFileChangedEvent>>()
            .subscribe();
    f.db.set_mutation_hook(FailApprovalOnce {
        previous: f.db.mutation_hook().unwrap().clone(),
        enabled: AtomicBool::new(true),
    });
    assert!(
        apply_quality_upgrade(&f.db, &f.admin, &f.notification.id)
            .await
            .is_err()
    );
    assert_original_files(&f);
    assert!(events.try_recv().is_err());
    assert!(
        PendingFileMatch::get(f.db.pool(), &f.pending.id)
            .await
            .unwrap()
            .unwrap()
            .copied_at
            .is_none()
    );
    assert!(
        Notification::get(f.db.pool(), &f.notification.id)
            .await
            .unwrap()
            .unwrap()
            .resolved_at
            .is_none()
    );
    apply_quality_upgrade(&f.db, &f.admin, &f.notification.id)
        .await
        .unwrap();
    assert!(events.try_recv().is_ok());
    assert_eq!(
        std::fs::read(&f.target).unwrap(),
        b"verified new source bytes"
    );
}

async fn generated_request(
    schema: &crate::services::graphql::LibrarianSchema,
    user: &AuthUser,
    document: &str,
    variables: serde_json::Value,
) -> async_graphql::Response {
    schema
        .execute(
            Request::new(document)
                .variables(Variables::from_json(variables))
                .data(user.clone())
                .data(AuthSubject::from_parts(
                    user.user_id.clone(),
                    user.role.iter().cloned().collect(),
                    Vec::new(),
                    None,
                ))
                .data(user.user_id.clone()),
        )
        .await
}
fn rejected(response: &async_graphql::Response, field: &str) -> bool {
    !response.errors.is_empty()
        || serde_json::to_value(&response.data).unwrap()[field]["success"]
            != serde_json::json!(true)
}

#[tokio::test]
async fn actual_generated_member_writes_reject_action_envelopes_and_roots_but_allow_relative_patterns()
 {
    let f = fixture().await;
    let schema =
        crate::services::graphql::build_schema(f.db.clone(), (), Arc::new(ServicesManager::new()));
    let owner = member(&f.owner);
    let response = generated_request(
        &schema,
        &owner,
        "mutation($id: String!) { approveQualityUpgrade(notificationId: $id) { success error } }",
        serde_json::json!({"id": f.notification.id}),
    )
    .await;
    assert!(rejected(&response, "approveQualityUpgrade"));
    for input in [
        serde_json::json!({"actionType":"quality_upgrade", "title":"must rollback"}),
        serde_json::json!({"actionData":"forged", "title":"must rollback"}),
        serde_json::json!({"actionType":null, "actionData":null}),
    ] {
        let response = generated_request(&schema, &owner, "mutation($id: String!, $input: UpdateNotificationInput!) { updateNotification(id: $id, input: $input) { success error } }", serde_json::json!({"id":f.notification.id, "input": input})).await;
        assert!(rejected(&response, "updateNotification"));
        let row = Notification::get(f.db.pool(), &f.notification.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.title, f.notification.title);
        assert_eq!(row.action_data, f.notification.action_data);
    }
    let response = generated_request(&schema, &owner, "mutation($input: CreateNotificationInput!) { createNotification(input: $input) { success error } }", serde_json::json!({"input":{"userId":f.owner.id,"notificationType":"info","category":"QUALITY","title":"forged","message":"forged","actionType":"quality_upgrade","actionData":"{}"}})).await;
    assert!(rejected(&response, "createNotification"));
    assert_eq!(
        Notification::query(f.db.pool())
            .fetch_all()
            .await
            .unwrap()
            .len(),
        1
    );
    let document = "mutation($id: String!, $input: UpdateLibraryInput!) { updateLibrary(id: $id, input: $input) { success error } }";
    let response = generated_request(
        &schema,
        &owner,
        document,
        serde_json::json!({"id":f.library.id,"input":{"path":f._temp.path().to_string_lossy()}}),
    )
    .await;
    assert!(rejected(&response, "updateLibrary"));
    let pattern = "Custom/{title}/{original}.{ext}";
    let response = generated_request(
        &schema,
        &owner,
        document,
        serde_json::json!({"id":f.library.id,"input":{"namingPattern":pattern}}),
    )
    .await;
    assert!(
        !rejected(&response, "updateLibrary"),
        "{:?}",
        response.errors
    );
    let row = Library::get(f.db.pool(), &f.library.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.naming_pattern, pattern);
    assert_eq!(row.path, f.library.path);
    validate_relative_library_path(Path::new(&row.naming_pattern)).unwrap();
    let response = generated_request(&schema, &owner, "mutation($id: String!, $input: UpdateNotificationInput!) { updateNotification(id: $id, input: $input) { success error } }", serde_json::json!({"id": f.notification.id, "input": {"readAt": chrono::Utc::now().to_rfc3339()}})).await;
    assert!(
        !rejected(&response, "updateNotification"),
        "{:?}",
        response.errors
    );
    assert_eq!(
        Notification::get(f.db.pool(), &f.notification.id)
            .await
            .unwrap()
            .unwrap()
            .action_data,
        f.notification.action_data
    );
}
