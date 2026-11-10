//! Write rules and lifecycle hooks that replaced hand-written twins of generated
//! operations: scan issue resolution, user roles, user deletion, artwork replacement
//! and publication, and quality status upkeep. Isolated in-memory database, no servers.
use std::future::Future;
use std::time::Duration;

use async_graphql::{EmptySubscription, MergedObject, Object, Request, Schema};
use graphql_orm::graphql::auth::AuthSubject;
use graphql_orm::graphql::subscriptions::SubscriptionEvent;
use serde_json::{Value, json};

use super::summary_tests::{
    database, library, library_scan_issue, library_scan_run, media_file, movie, session_token, user,
};
use super::*;

async fn eventually<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    for _ in 0..100 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("timed out waiting for {what}");
}

// ---------------------------------------------------------------------------
// Scan issue resolution is a generated update plus the write transform.
// ---------------------------------------------------------------------------

fn resolve(note: Option<&str>) -> UpdateLibraryScanIssueInput {
    UpdateLibraryScanIssueInput {
        resolved_at: Some(Some("1999-01-01T00:00:00Z".to_string())),
        resolution: note.map(|note| Some(note.to_string())),
        ..Default::default()
    }
}

#[tokio::test]
async fn resolving_a_scan_issue_needs_a_note_and_is_stamped_and_bounded_by_the_server() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let run = library_scan_run(&db, &library).await;
    let file = media_file(&db, &library, 1, None, None).await;
    let issue = library_scan_issue(&db, &library, &run, &file).await;

    assert!(
        LibraryScanIssue::update_by_id(&db, &issue.id, resolve(Some("   ")))
            .await
            .is_err(),
        "a blank note is refused"
    );
    assert!(
        LibraryScanIssue::update_by_id(&db, &issue.id, resolve(None))
            .await
            .is_err(),
        "a missing note is refused"
    );
    let unchanged = LibraryScanIssue::get(db.pool(), &issue.id)
        .await
        .unwrap()
        .unwrap();
    assert!(unchanged.resolved_at.is_none());

    let long = format!("  {}  ", "n".repeat(1_500));
    let resolved = LibraryScanIssue::update_by_id(&db, &issue.id, resolve(Some(&long)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        resolved
            .resolution
            .as_deref()
            .map(|note| note.chars().count()),
        Some(1_000)
    );
    assert_ne!(
        resolved.resolved_at.as_deref(),
        Some("1999-01-01T00:00:00Z")
    );

    // Resolving again keeps the first resolution and time.
    let again = LibraryScanIssue::update_by_id(&db, &issue.id, resolve(Some("later note")))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(again.resolved_at, resolved.resolved_at);
    assert_eq!(again.resolution, resolved.resolution);
}

#[derive(Default)]
struct Probe;

#[Object]
impl Probe {
    async fn ok(&self) -> bool {
        true
    }
}

#[derive(Default, MergedObject)]
struct Mutations(LibraryScanIssueMutations, MediaFileMutations, UserMutations);

async fn run(
    schema: &Schema<Probe, Mutations, EmptySubscription>,
    actor: &AuthUser,
    document: &str,
    variables: Value,
) -> async_graphql::Response {
    schema
        .execute(
            Request::new(document)
                .variables(async_graphql::Variables::from_json(variables))
                .data(actor.clone())
                .data(AuthSubject::from_parts(
                    actor.user_id.clone(),
                    actor.role.iter().cloned().collect(),
                    Vec::new(),
                    None,
                )),
        )
        .await
}

fn schema(db: &Database) -> Schema<Probe, Mutations, EmptySubscription> {
    Schema::build(Probe, Mutations::default(), EmptySubscription)
        .data(db.clone())
        .finish()
}

fn actor(user: &User, role: &str) -> AuthUser {
    AuthUser {
        user_id: user.id.clone(),
        email: None,
        role: Some(role.to_string()),
    }
}

#[tokio::test]
async fn an_admin_resolves_a_scan_issue_through_the_generated_update_and_a_member_cannot() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let run_row = library_scan_run(&db, &library).await;
    let file = media_file(&db, &library, 1, None, None).await;
    let issue = library_scan_issue(&db, &library, &run_row, &file).await;
    let schema = schema(&db);
    let document = "mutation($id:String!,$input:UpdateLibraryScanIssueInput!){updateLibraryScanIssue(id:$id,input:$input){success error libraryScanIssue{resolvedAt resolution}}}";
    let variables = json!({"id": issue.id, "input": {"resolvedAt": "2026-01-01T00:00:00Z", "resolution": " reviewed "}});

    let denied = run(
        &schema,
        &actor(&owner, "member"),
        document,
        variables.clone(),
    )
    .await;
    assert!(
        !denied.errors.is_empty(),
        "a member may not write scan issues: {denied:?}"
    );
    let untouched = LibraryScanIssue::get(db.pool(), &issue.id)
        .await
        .unwrap()
        .unwrap();
    assert!(untouched.resolved_at.is_none());

    let admin = user(&db).await;
    let resolved = run(&schema, &actor(&admin, "admin"), document, variables).await;
    assert!(resolved.errors.is_empty(), "{resolved:?}");
    let data = resolved.data.into_json().unwrap();
    assert_eq!(data["updateLibraryScanIssue"]["success"], true, "{data}");
    assert_eq!(
        data["updateLibraryScanIssue"]["libraryScanIssue"]["resolution"],
        "reviewed"
    );
}

// ---------------------------------------------------------------------------
// Manual-match record on generated link writes.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_link_written_by_a_signed_in_admin_is_a_manual_match_and_clearing_it_clears_the_record() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let title = movie(&db, &library).await;
    let file = media_file(&db, &library, 1, None, None).await;
    let admin = user(&db).await;
    let schema = schema(&db);
    let document = "mutation($id:String!,$input:UpdateMediaFileInput!){updateMediaFile(id:$id,input:$input){success error}}";

    let linked = run(
        &schema,
        &actor(&admin, "admin"),
        document,
        json!({"id": file.id, "input": {"movieId": title.id}}),
    )
    .await;
    assert!(linked.errors.is_empty(), "{linked:?}");
    let row = MediaFile::get(db.pool(), &file.id).await.unwrap().unwrap();
    assert_eq!(row.movie_id.as_deref(), Some(title.id.as_str()));
    assert_eq!(row.match_type.as_deref(), Some("manual"));
    assert_eq!(row.matched_by_user_id.as_deref(), Some(admin.id.as_str()));
    assert!(row.match_confirmed_at.is_some());

    let cleared = run(
        &schema,
        &actor(&admin, "admin"),
        document,
        json!({"id": file.id, "input": {"movieId": null}}),
    )
    .await;
    assert!(cleared.errors.is_empty(), "{cleared:?}");
    let row = MediaFile::get(db.pool(), &file.id).await.unwrap().unwrap();
    assert!(row.movie_id.is_none());
    assert!(
        row.match_type.is_none()
            && row.matched_by_user_id.is_none()
            && row.match_confirmed_at.is_none()
    );
}

// ---------------------------------------------------------------------------
// Users.
// ---------------------------------------------------------------------------

fn create_user(role: &str) -> CreateUserInput {
    CreateUserInput {
        username: uuid::Uuid::new_v4().to_string(),
        email: None,
        password_hash: "hash".to_string(),
        role: role.to_string(),
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
    }
}

#[tokio::test]
async fn a_user_role_must_be_one_the_authorizer_accepts_and_is_stored_canonically() {
    let db = database(true).await;
    assert!(User::insert(&db, create_user("superuser")).await.is_err());
    let created = User::insert(&db, create_user(" Admin ")).await.unwrap();
    assert_eq!(created.role, "admin");

    let member = user(&db).await;
    assert!(
        User::update_by_id(
            &db,
            &member.id,
            UpdateUserInput {
                role: Some("root".to_string()),
                ..Default::default()
            }
        )
        .await
        .is_err()
    );
    let updated = User::update_by_id(
        &db,
        &member.id,
        UpdateUserInput {
            role: Some("User".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(updated.role, "member");
    // Untouched roles are not rewritten by unrelated edits.
    let renamed = User::update_by_id(
        &db,
        &member.id,
        UpdateUserInput {
            display_name: Some(Some("Renamed".to_string())),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(renamed.role, "member");
}

#[tokio::test]
async fn deleting_a_user_removes_their_refresh_tokens_and_only_theirs() {
    let db = database(true).await;
    let leaving = user(&db).await;
    let staying = user(&db).await;
    let active = session_token(&db, &leaving, false).await;
    let revoked = session_token(&db, &leaving, true).await;
    let other = session_token(&db, &staying, false).await;

    User::delete_by_id(&db, &leaving.id).await.unwrap();

    for gone in [&active.id, &revoked.id] {
        assert!(RefreshToken::get(db.pool(), gone).await.unwrap().is_none());
    }
    assert!(
        RefreshToken::get(db.pool(), &other.id)
            .await
            .unwrap()
            .is_some()
    );
}

// ---------------------------------------------------------------------------
// Artwork.
// ---------------------------------------------------------------------------

async fn storage_object(db: &Database, name: &str) -> StorageObject {
    StorageObject::insert(
        db,
        CreateStorageObjectInput {
            object_id: uuid::Uuid::new_v4().to_string(),
            namespace: "derivatives".to_string(),
            backend: "local".to_string(),
            storage_key: format!("fixture/{name}"),
            original_file_name: Some(name.to_string()),
            mime_type: Some("image/jpeg".to_string()),
            size_bytes: 1,
            sha256_hex: format!("{:0>64}", name.len()),
        },
    )
    .await
    .unwrap()
}

async fn artwork(
    db: &Database,
    entity_type: &str,
    entity_id: &str,
    artwork_type: &str,
    storage: &StorageObject,
) -> ArtworkCache {
    ArtworkCache::insert(
        db,
        CreateArtworkCacheInput {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            artwork_type: artwork_type.to_string(),
            storage_object_id: storage.id.clone(),
            source_url: Some("https://image.example/a.jpg".to_string()),
            width: Some(100),
            height: Some(150),
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn replacing_cached_artwork_releases_the_old_object_and_drops_resized_copies() {
    let db = database(true).await;
    let old = storage_object(&db, "old").await;
    let resized = storage_object(&db, "resized").await;
    let fresh = storage_object(&db, "fresh").await;
    let original = artwork(&db, "person", "p1", "poster", &old).await;
    let variant = artwork(&db, "person", "p1", "poster@w300", &resized).await;

    ArtworkCache::update_by_id(
        &db,
        &original.id,
        UpdateArtworkCacheInput {
            storage_object_id: Some(fresh.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert!(
        StorageObject::get(db.pool(), &old.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        ArtworkCache::get(db.pool(), &variant.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        StorageObject::get(db.pool(), &resized.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        StorageObject::get(db.pool(), &fresh.id)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn an_object_still_referenced_by_other_artwork_is_kept_on_replacement() {
    let db = database(true).await;
    let shared = storage_object(&db, "shared").await;
    let fresh = storage_object(&db, "fresh").await;
    let first = artwork(&db, "person", "p1", "poster", &shared).await;
    artwork(&db, "person", "p2", "poster", &shared).await;

    ArtworkCache::update_by_id(
        &db,
        &first.id,
        UpdateArtworkCacheInput {
            storage_object_id: Some(fresh.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert!(
        StorageObject::get(db.pool(), &shared.id)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn caching_artwork_republishes_its_title_but_a_resized_copy_does_not() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let title = movie(&db, &library).await;
    let object = storage_object(&db, "poster").await;
    let mut changes = db
        .ensure_event_sender::<SubscriptionEvent<MovieChangedEvent>>()
        .subscribe();

    artwork(&db, "movie", &title.id, "poster@w300", &object).await;
    assert!(
        changes.try_recv().is_err(),
        "a resized copy is not what a view shows"
    );

    let original = storage_object(&db, "poster-original").await;
    artwork(&db, "movie", &title.id, "poster", &original).await;
    let mut republished = false;
    while let Ok(change) = changes.try_recv() {
        republished |= change.event.id == title.id;
    }
    assert!(
        republished,
        "the movie is published when its poster appears"
    );
}

// ---------------------------------------------------------------------------
// Quality status upkeep.
// ---------------------------------------------------------------------------

async fn quality_profile(db: &Database, name: &str, resolutions: &[&str]) -> QualityProfile {
    QualityProfile::insert(
        db,
        CreateQualityProfileInput {
            name: name.to_string(),
            media_kind: MediaKind::Video,
            allowed_resolutions: resolutions.iter().map(ToString::to_string).collect(),
            allowed_video_codecs: Vec::new(),
            allowed_audio_formats: Vec::new(),
            allowed_hdr_types: Vec::new(),
            allowed_sources: Vec::new(),
            release_group_blacklist: Vec::new(),
            release_group_whitelist: Vec::new(),
            require_hdr: false,
            preferred_languages: Vec::new(),
            require_language_match: false,
            min_size_mb: None,
            max_size_mb: None,
            min_seeders: 1,
            max_release_age_days: None,
            preferred_release_groups: Vec::new(),
            allow_season_packs: true,
            prefer_proper_repack: true,
            resolution_preference: Vec::new(),
            cutoff_resolution: None,
            upgrade_until_cutoff: false,
            is_default: false,
        },
    )
    .await
    .unwrap()
}

async fn assign_library_profile(
    db: &Database,
    library: &Library,
    profile: Option<&QualityProfile>,
) {
    Library::update_by_id(
        db,
        &library.id,
        UpdateLibraryInput {
            quality_profile_id: Some(profile.map(|profile| profile.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
}

/// Mark a file analysed at 720p, which is what the analysis write does.
async fn analyse_at_720p(db: &Database, file: &MediaFile) {
    MediaFile::update_by_id(
        db,
        &file.id,
        UpdateMediaFileInput {
            resolution: Some(Some("720p".to_string())),
            analyzed_at: Some(Some(AppMutationHook::now_rfc3339())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
}

async fn status(db: &Database, file: &MediaFile) -> Option<String> {
    MediaFile::get(db.pool(), &file.id)
        .await
        .unwrap()
        .unwrap()
        .quality_status
}

async fn wait_for_status(db: &Database, file: &MediaFile, expected: &str) {
    eventually(&format!("quality status {expected}"), || async {
        status(db, file).await.as_deref() == Some(expected)
    })
    .await;
}

#[tokio::test]
async fn analysis_sets_the_quality_status_without_a_hand_made_recompute() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let strict = quality_profile(&db, "1080p only", &["1080p"]).await;
    assign_library_profile(&db, &library, Some(&strict)).await;
    let file = media_file(&db, &library, 1, None, None).await;
    assert_eq!(status(&db, &file).await, None);

    analyse_at_720p(&db, &file).await;

    assert_eq!(status(&db, &file).await.as_deref(), Some("suboptimal"));
}

#[tokio::test]
async fn an_unanalysed_file_keeps_an_unknown_status() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let strict = quality_profile(&db, "1080p only", &["1080p"]).await;
    assign_library_profile(&db, &library, Some(&strict)).await;
    let file = media_file(&db, &library, 1, None, None).await;

    MediaFile::update_by_id(
        &db,
        &file.id,
        UpdateMediaFileInput {
            resolution: Some(Some("720p".to_string())),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(status(&db, &file).await, None);
}

#[tokio::test]
async fn editing_a_quality_profile_re_evaluates_the_files_that_use_it() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let profile = quality_profile(&db, "1080p only", &["1080p"]).await;
    assign_library_profile(&db, &library, Some(&profile)).await;
    let file = media_file(&db, &library, 1, None, None).await;
    analyse_at_720p(&db, &file).await;
    assert_eq!(status(&db, &file).await.as_deref(), Some("suboptimal"));

    QualityProfile::update_by_id(
        &db,
        &profile.id,
        UpdateQualityProfileInput {
            allowed_resolutions: Some(vec!["720p".to_string(), "1080p".to_string()]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    wait_for_status(&db, &file, "optimal").await;
}

#[tokio::test]
async fn renaming_a_quality_profile_does_not_re_evaluate_anything() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let profile = quality_profile(&db, "1080p only", &["1080p"]).await;
    assign_library_profile(&db, &library, Some(&profile)).await;
    let file = media_file(&db, &library, 1, None, None).await;
    analyse_at_720p(&db, &file).await;
    // Plant a status no evaluation would produce: only a recompute can overwrite it.
    MediaFile::update_by_id(
        &db,
        &file.id,
        UpdateMediaFileInput {
            quality_status: Some(Some("planted".to_string())),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    QualityProfile::update_by_id(
        &db,
        &profile.id,
        UpdateQualityProfileInput {
            name: Some("renamed".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(status(&db, &file).await.as_deref(), Some("planted"));
}

#[tokio::test]
async fn deleting_a_quality_profile_re_evaluates_against_what_it_falls_back_to() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let profile = quality_profile(&db, "1080p only", &["1080p"]).await;
    assign_library_profile(&db, &library, Some(&profile)).await;
    let file = media_file(&db, &library, 1, None, None).await;
    analyse_at_720p(&db, &file).await;
    assert_eq!(status(&db, &file).await.as_deref(), Some("suboptimal"));

    QualityProfile::delete_by_id(&db, &profile.id)
        .await
        .unwrap();

    // No default is seeded here, so the file now resolves to "any quality".
    wait_for_status(&db, &file, "optimal").await;
}

#[tokio::test]
async fn assigning_a_different_profile_to_a_library_re_evaluates_its_files() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let strict = quality_profile(&db, "1080p only", &["1080p"]).await;
    let relaxed = quality_profile(&db, "720p ok", &["720p"]).await;
    assign_library_profile(&db, &library, Some(&strict)).await;
    let file = media_file(&db, &library, 1, None, None).await;
    analyse_at_720p(&db, &file).await;
    assert_eq!(status(&db, &file).await.as_deref(), Some("suboptimal"));

    assign_library_profile(&db, &library, Some(&relaxed)).await;

    wait_for_status(&db, &file, "optimal").await;
}

#[tokio::test]
async fn assigning_a_profile_to_a_movie_re_evaluates_its_file_only() {
    let db = database(true).await;
    let owner = user(&db).await;
    let library = library(&db, &owner).await;
    let strict = quality_profile(&db, "1080p only", &["1080p"]).await;
    let relaxed = quality_profile(&db, "720p ok", &["720p"]).await;
    assign_library_profile(&db, &library, Some(&strict)).await;
    let title = movie(&db, &library).await;
    let linked = media_file(&db, &library, 1, None, None).await;
    let bystander = media_file(&db, &library, 1, None, None).await;
    for file in [&linked, &bystander] {
        analyse_at_720p(&db, file).await;
    }
    MediaFile::update_by_id(
        &db,
        &linked.id,
        UpdateMediaFileInput {
            movie_id: Some(Some(title.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(status(&db, &linked).await.as_deref(), Some("suboptimal"));

    Movie::update_by_id(
        &db,
        &title.id,
        UpdateMovieInput {
            quality_profile_id: Some(Some(relaxed.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    wait_for_status(&db, &linked, "optimal").await;
    assert_eq!(status(&db, &bystander).await.as_deref(), Some("suboptimal"));
}
