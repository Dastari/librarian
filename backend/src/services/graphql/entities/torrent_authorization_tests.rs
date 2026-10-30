//! Focused generated-repository/runtime authorization tests; no service or server.
use super::*;
use crate::services::graphql::policy::{AppEntityPolicy, AppFieldPolicy};
use crate::services::graphql::row_policy::OwnershipRowPolicy;

pub(crate) async fn fixture() -> Database {
    let mut db = Database::connect_sqlite("sqlite::memory:").await.unwrap();
    // Match application authorization without installing service lifecycle hooks.
    db.set_entity_policy(AppEntityPolicy);
    db.set_field_policy(AppFieldPolicy);
    db.set_row_policy(OwnershipRowPolicy);
    crate::services::database::bootstrap_schema(&db)
        .await
        .unwrap();
    db
}

pub(crate) async fn account(db: &Database, role: &str) -> AuthUser {
    let user = User::insert(
        db,
        CreateUserInput {
            username: Uuid::new_v4().to_string(),
            email: None,
            password_hash: "test-only".into(),
            role: role.into(),
            display_name: None,
            avatar_url: None,
            is_active: true,
            last_login_at: None,
        },
    )
    .await
    .unwrap();
    AuthUser {
        user_id: user.id,
        email: None,
        role: Some(role.into()),
    }
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

pub(crate) async fn torrent(db: &Database, user: &AuthUser, hash: &str) -> Torrent {
    Torrent::insert(db, torrent_input(user, hash))
        .await
        .unwrap()
}

pub(crate) fn live(hash: &str, id: usize) -> ServiceTorrentInfo {
    ServiceTorrentInfo {
        id,
        info_hash: hash.into(),
        name: "fresh runtime name".into(),
        state: ServiceTorrentState::Downloading,
        progress: 0.75,
        size: 100,
        downloaded: 75,
        uploaded: 20,
        download_speed: 1234,
        upload_speed: 56,
        peers: 3,
        seeds: 2,
        save_path: "/test/live".into(),
        files: vec![],
    }
}

#[tokio::test]
async fn runtime_mapping_denies_foreign_missing_and_ambiguous_identities() {
    let db = fixture().await;
    let member = account(&db, "member").await;
    let other = account(&db, "member").await;
    let own = torrent(&db, &member, "own").await;
    torrent(&db, &other, "foreign").await;
    assert_eq!(
        require_runtime_torrent(&db, &member, "own")
            .await
            .unwrap()
            .id,
        own.id
    );
    assert!(
        require_runtime_torrent(&db, &member, "foreign")
            .await
            .is_err()
    );
    assert!(
        require_runtime_torrent(&db, &member, "missing")
            .await
            .is_err()
    );
    // The schema prevents a second owner from manufacturing a duplicate hash.
    assert!(
        Torrent::insert(&db, torrent_input(&other, "own"))
            .await
            .is_err()
    );
    assert_eq!(
        require_runtime_torrent(&db, &member, "own")
            .await
            .unwrap()
            .id,
        own.id
    );
    assert!(runtime_id(-1).is_err());
    assert_eq!(runtime_id(7).unwrap(), 7);
}

#[tokio::test]
async fn snapshots_find_owned_rows_beyond_default_page_and_preserve_live_values() {
    let db = fixture().await;
    let member = account(&db, "member").await;
    let other = account(&db, "member").await;
    for i in 0..105 {
        torrent(&db, &other, &format!("unrelated-{i}")).await;
    }
    torrent(&db, &member, "owned-after-default-page").await;
    torrent(&db, &other, "foreign-live").await;
    let visible = visible_runtime_torrents(
        &db,
        &member,
        vec![
            live("foreign-live", 0),
            live("unmapped", 1),
            live("owned-after-default-page", 2),
        ],
    )
    .await
    .unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, 2);
    assert_eq!(visible[0].progress, 0.75);
    assert_eq!(visible[0].download_speed, 1234);
    assert_eq!(visible[0].name, "fresh runtime name");
    assert_eq!(visible[0].state, "downloading");
    assert_eq!(
        visible_runtime_torrents(&db, &member, vec![live("foreign-live", 0)])
            .await
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn admin_inspection_is_global_but_controls_require_persisted_identity() {
    let db = fixture().await;
    let admin = account(&db, "admin").await;
    let member = account(&db, "member").await;
    torrent(&db, &member, "member-owned").await;
    assert!(
        require_runtime_torrent(&db, &admin, "member-owned")
            .await
            .is_ok()
    );
    assert!(
        require_runtime_torrent(&db, &admin, "orphan")
            .await
            .is_err()
    );
    assert_eq!(
        visible_runtime_torrents(
            &db,
            &admin,
            vec![live("member-owned", 0), live("orphan", 1)]
        )
        .await
        .unwrap()
        .len(),
        2
    );
}

#[tokio::test]
async fn current_authority_denies_deleted_disabled_and_unknown_users_and_does_not_escalate() {
    let db = fixture().await;
    let member = account(&db, "member").await;
    User::update_by_id(
        &db,
        &member.user_id,
        UpdateUserInput {
            role: Some("admin".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(!current_torrent_user(&db, &member).await.unwrap().is_admin());
    let admin = account(&db, "admin").await;
    User::update_by_id(
        &db,
        &admin.user_id,
        UpdateUserInput {
            role: Some("member".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(!current_torrent_user(&db, &admin).await.unwrap().is_admin());
    User::update_by_id(
        &db,
        &member.user_id,
        UpdateUserInput {
            is_active: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(current_torrent_user(&db, &member).await.is_err());
    User::delete_by_id(&db, &admin.user_id).await.unwrap();
    assert!(current_torrent_user(&db, &admin).await.is_err());
    let unknown = account(&db, "unknown-role").await;
    assert!(current_torrent_user(&db, &unknown).await.is_err());
}

#[tokio::test]
async fn add_targets_require_actual_library_ownership_before_runtime_work() {
    let db = fixture().await;
    let member = account(&db, "member").await;
    let other = account(&db, "member").await;
    let admin = account(&db, "admin").await;
    let library = Library::insert(
        &db,
        CreateLibraryInput {
            user_id: other.user_id.clone(),
            name: "Foreign".into(),
            path: "/not-used".into(),
            library_type: "movies".into(),
            icon: None,
            color: None,
            auto_scan: false,
            auto_organize: false,
            naming_pattern: "".into(),
            scan_interval_minutes: 60,
            watch_for_changes: false,
            scanning: false,
            last_scanned_at: None,
            quality_profile_id: None,
        },
    )
    .await
    .unwrap();
    let request = AddTorrentInput {
        library_id: Some(library.id.clone()),
        ..Default::default()
    };
    assert!(
        authorize_add_torrent_targets(&db, &member, &request)
            .await
            .is_err()
    );
    assert_eq!(
        authorize_add_torrent_targets(&db, &admin, &request)
            .await
            .unwrap()
            .to_string(),
        other.user_id
    );
    assert_eq!(
        authorize_add_torrent_targets(&db, &member, &AddTorrentInput::default())
            .await
            .unwrap()
            .to_string(),
        member.user_id
    );
    assert!(
        authorize_add_torrent_targets(
            &db,
            &member,
            &AddTorrentInput {
                movie_id: Some("missing".into()),
                ..Default::default()
            }
        )
        .await
        .is_err()
    );
    assert!(
        authorize_add_torrent_targets(
            &db,
            &member,
            &AddTorrentInput {
                movie_id: Some("movie".into()),
                track_id: Some("track".into()),
                ..Default::default()
            }
        )
        .await
        .is_err()
    );
}
