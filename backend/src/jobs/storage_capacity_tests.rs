//! The capacity job's record and alerts, against a real schema with the app's hooks and
//! readings made up (no share, no statvfs).

use super::*;
use crate::graphql::entities::CreateUserInput;
use crate::services::DatabaseService;
use crate::services::database::summary_tests::{library, user};
use graphql_orm::DbPool;

const GB: u64 = 1_000_000_000;
const TB: u64 = 1_000 * GB;
const SHARE: &str = "smb://10.0.1.190/media";

async fn database() -> Database {
    let db = Database::new(DbPool::connect("sqlite::memory:").await.unwrap());
    crate::services::database::bootstrap_schema(&db)
        .await
        .unwrap();
    DatabaseService::new(db).pool().clone()
}

async fn admin(db: &Database) -> User {
    User::insert(
        db,
        CreateUserInput {
            username: uuid::Uuid::new_v4().to_string(),
            email: None,
            password_hash: "fixture".to_string(),
            role: "admin".to_string(),
            display_name: None,
            avatar_url: None,
            is_active: true,
            last_login_at: None,
            phone: None,
            genre_interests: None,
            must_change_password: false,
            two_factor_enabled_at: None,
            recovery_codes_remaining: 0,
            password_changed_at: None,
            totp_secret: None,
            totp_pending_secret: None,
            recovery_codes: None,
        },
    )
    .await
    .unwrap()
}

fn on_share(library: &Library) -> Location {
    Location {
        resident: Resident::Library {
            id: library.id.clone(),
            name: library.name.clone(),
            owner_id: library.user_id.clone(),
        },
        path: format!("{SHARE}/{}", library.name),
    }
}

fn groups(members: Vec<Location>) -> BTreeMap<String, Vec<Location>> {
    let mut groups = BTreeMap::new();
    groups.insert(SHARE.to_string(), members);
    groups
}

fn reading(available: u64) -> BTreeMap<String, Reading> {
    let mut readings = BTreeMap::new();
    readings.insert(
        SHARE.to_string(),
        Ok(VolumeCapacity::new(TB, available, available)),
    );
    readings
}

fn unreachable() -> BTreeMap<String, Reading> {
    let mut readings = BTreeMap::new();
    readings.insert(
        SHARE.to_string(),
        Err("The share is not connected".to_string()),
    );
    readings
}

async fn open_alerts(db: &Database) -> Vec<Notification> {
    Notification::query(db.pool())
        .fetch_all()
        .await
        .unwrap()
        .into_iter()
        .filter(|row| row.category == CATEGORY && row.resolved_at.is_none())
        .collect()
}

async fn all_alerts(db: &Database) -> Vec<Notification> {
    Notification::query(db.pool())
        .fetch_all()
        .await
        .unwrap()
        .into_iter()
        .filter(|row| row.category == CATEGORY)
        .collect()
}

async fn libraries(db: &Database) -> Vec<Library> {
    Library::query(db.pool()).fetch_all().await.unwrap()
}

async fn volume(db: &Database) -> StorageVolume {
    let rows = StorageVolume::query(db.pool()).fetch_all().await.unwrap();
    assert_eq!(rows.len(), 1, "one volume for the share");
    rows.into_iter().next().unwrap()
}

#[tokio::test]
async fn a_low_share_alerts_once_per_level_and_resolves_when_space_comes_back() {
    let db = database().await;
    let admin = admin(&db).await;
    let owner = user(&db).await;
    let movies = library(&db, &owner).await;
    let members = vec![
        on_share(&movies),
        Location {
            resident: Resident::Downloads,
            path: format!("{SHARE}/downloads"),
        },
    ];
    let now = Utc::now();

    // Plenty of room: a record, the library linked, no alert.
    let summary = store(
        &db,
        &groups(members.clone()),
        reading(500 * GB),
        &libraries(&db).await,
        now,
    )
    .await
    .unwrap();
    assert_eq!(summary.written, 1);
    let stored = volume(&db).await;
    assert_eq!(stored.level, "normal");
    assert_eq!(stored.status, "ok");
    assert_eq!(stored.label, "media on 10.0.1.190");
    assert!(stored.holds_downloads);
    assert_eq!(stored.available_bytes, Some((500 * GB) as i64));
    let linked = Library::get(db.pool(), &movies.id).await.unwrap().unwrap();
    assert_eq!(
        linked.storage_volume_id.as_deref(),
        Some(stored.id.as_str())
    );
    assert!(open_alerts(&db).await.is_empty());

    // A small move is not written.
    let summary = store(
        &db,
        &groups(members.clone()),
        reading(498 * GB),
        &libraries(&db).await,
        now,
    )
    .await
    .unwrap();
    assert_eq!(summary.written, 0);

    // Below 10 % of 1 TB: one warning each for the admin and the owner.
    store(
        &db,
        &groups(members.clone()),
        reading(90 * GB),
        &libraries(&db).await,
        now,
    )
    .await
    .unwrap();
    let open = open_alerts(&db).await;
    assert_eq!(open.len(), 2);
    let recipients: HashSet<&str> = open.iter().map(|row| row.user_id.as_str()).collect();
    assert!(recipients.contains(admin.id.as_str()) && recipients.contains(owner.id.as_str()));
    assert!(open.iter().all(|row| row.notification_type == "WARNING"));
    assert!(
        open[0]
            .title
            .contains("Disk space is low on media on 10.0.1.190")
    );
    assert!(
        open[0]
            .message
            .contains("the fixture library library and the download folder"),
        "{}",
        open[0].message
    );
    assert_eq!(volume(&db).await.level, "warning");

    // Again at the same level: nothing new.
    store(
        &db,
        &groups(members.clone()),
        reading(80 * GB),
        &libraries(&db).await,
        now,
    )
    .await
    .unwrap();
    assert_eq!(all_alerts(&db).await.len(), 2);

    // The share cannot be read: it shows as unavailable and the alert stays as it is.
    store(
        &db,
        &groups(members.clone()),
        unreachable(),
        &libraries(&db).await,
        now,
    )
    .await
    .unwrap();
    let stored = volume(&db).await;
    assert_eq!(stored.status, "unavailable");
    assert_eq!(
        stored.available_bytes,
        Some((80 * GB) as i64),
        "the last reading is kept"
    );
    assert_eq!(open_alerts(&db).await.len(), 2);

    // Critical: the warnings are resolved as escalated and critical ones raised.
    store(
        &db,
        &groups(members.clone()),
        reading(10 * GB),
        &libraries(&db).await,
        now,
    )
    .await
    .unwrap();
    let open = open_alerts(&db).await;
    assert_eq!(open.len(), 2);
    assert!(open.iter().all(|row| row.notification_type == "ERROR"));
    assert_eq!(
        all_alerts(&db)
            .await
            .iter()
            .filter(|row| row.resolution.as_deref() == Some("escalated"))
            .count(),
        2
    );

    // Just above the critical line: still critical (hysteresis), nothing raised.
    store(
        &db,
        &groups(members.clone()),
        reading(52 * GB),
        &libraries(&db).await,
        now,
    )
    .await
    .unwrap();
    assert_eq!(volume(&db).await.level, "critical");
    assert_eq!(all_alerts(&db).await.len(), 4);

    // Plenty again: everything resolved as recovered.
    store(
        &db,
        &groups(members.clone()),
        reading(400 * GB),
        &libraries(&db).await,
        now,
    )
    .await
    .unwrap();
    assert!(open_alerts(&db).await.is_empty());
    assert_eq!(volume(&db).await.level, "normal");
    assert_eq!(
        all_alerts(&db)
            .await
            .iter()
            .filter(|row| row.resolution.as_deref() == Some("recovered"))
            .count(),
        2
    );
}

#[tokio::test]
async fn a_critical_download_volume_refuses_a_release_that_would_not_fit() {
    let db = database().await;
    let members = vec![Location {
        resident: Resident::Downloads,
        path: format!("{SHARE}/downloads"),
    }];
    store(&db, &groups(members), reading(10 * GB), &[], Utc::now())
        .await
        .unwrap();
    let reason = download_refusal(&db, Some((30 * GB) as i64)).await;
    assert!(reason.is_some_and(|reason| reason.contains("10 GB free")));
    assert!(download_refusal(&db, Some(GB as i64)).await.is_none());
    assert!(download_refusal(&db, None).await.is_none());
}

#[tokio::test]
async fn a_volume_nothing_lives_on_any_more_is_removed_with_its_alerts() {
    let db = database().await;
    admin(&db).await;
    let members = vec![Location {
        resident: Resident::Downloads,
        path: format!("{SHARE}/downloads"),
    }];
    store(&db, &groups(members), reading(10 * GB), &[], Utc::now())
        .await
        .unwrap();
    assert_eq!(open_alerts(&db).await.len(), 1);
    store(&db, &BTreeMap::new(), BTreeMap::new(), &[], Utc::now())
        .await
        .unwrap();
    assert!(
        StorageVolume::query(db.pool())
            .fetch_all()
            .await
            .unwrap()
            .is_empty()
    );
    assert!(open_alerts(&db).await.is_empty());
}

#[tokio::test]
async fn a_pass_over_local_folders_reads_this_machine_once() {
    let db = database().await;
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let cache = dir.path().join("data/cache");
    let summary = run_pass_with(
        &db,
        &[
            data.to_string_lossy().into_owned(),
            cache.to_string_lossy().into_owned(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(summary.volumes, 1);
    assert_eq!(summary.unavailable, 0);
    let stored = volume(&db).await;
    assert_eq!(stored.kind, "local");
    assert!(stored.holds_app_data);
    assert!(stored.total_bytes.unwrap_or(0) > 0);
}
