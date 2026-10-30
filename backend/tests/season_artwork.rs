//! Seasons are provider metadata derived from their show: members read the seasons of their
//! own shows, only the metadata sync (admin) writes them, and episodes carry the still
//! image source that the artwork cache downloads from.
mod common;

use common::{DEFAULT_PASSWORD, TestApp};
use librarian::graphql::entities::{CreateInviteTokenInput, InviteToken};
use serde_json::{Value, json};

const REGISTER: &str = r#"mutation Register($input: RegisterUserInput!) {
  register(input: $input) { success error user { id role } }
}"#;

async fn create(
    client: &mut common::GqlClient,
    mutation: &str,
    input: Value,
    entity: &str,
) -> String {
    let data = client.query(mutation, json!({ "input": input })).await;
    let result = data
        .as_object()
        .and_then(|object| object.values().next())
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        result["success"], true,
        "{entity} fixture should save: {data}"
    );
    result[entity]["id"]
        .as_str()
        .expect("fixture id")
        .to_string()
}

async fn show_for(app: &TestApp, admin: &mut common::GqlClient, owner: &str, name: &str) -> String {
    let path = app.path(&format!("media/{name}"));
    std::fs::create_dir_all(&path).expect("library folder");
    let library = create(
        admin,
        "mutation($input: CreateLibraryInput!) { result: createLibrary(input: $input) { success error library { id } } }",
        json!({ "userId": owner, "name": name, "path": path.display().to_string(), "libraryType": "tv",
                "autoScan": false, "autoOrganize": false, "namingPattern": "{title}",
                "scanIntervalMinutes": 60, "watchForChanges": false, "scanning": false }),
        "library",
    )
    .await;
    create(
        admin,
        "mutation($input: CreateShowInput!) { result: createShow(input: $input) { success error show { id } } }",
        json!({ "userId": owner, "libraryId": library, "name": name, "genres": [],
                "autoDownload": false, "autoDownloadMode": "NONE" }),
        "show",
    )
    .await
}

#[test]
fn seasons_follow_their_show_and_only_metadata_sync_writes_them() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");
        InviteToken::insert(
            &app.db,
            CreateInviteTokenInput {
                token: "invite-season-member".to_string(),
                created_by: admin_id.clone(),
                library_ids: Vec::new(),
                role: "member".to_string(),
                access_level: "full".to_string(),
                expires_at: None,
                max_uses: Some(1),
                use_count: 0,
                apply_restrictions: false,
                restrictions_template: None,
                is_active: true,
            },
        )
        .await
        .expect("invite fixture");
        let mut member = app.client();
        let registered = member
            .query(
                REGISTER,
                json!({ "input": { "email": "seasons@example.test", "name": "Member",
                "password": DEFAULT_PASSWORD, "inviteToken": "invite-season-member" } }),
            )
            .await;
        let member_id = registered["register"]["user"]["id"]
            .as_str()
            .expect("member id")
            .to_string();

        let own_show = show_for(&app, &mut admin, &member_id, "member-show").await;
        let other_show = show_for(&app, &mut admin, &admin_id, "admin-show").await;
        const CREATE_SEASON: &str = "mutation($input: CreateSeasonInput!) { result: upsertSeason(input: $input) { success error season { id } } }";
        let poster = "https://static.tvmaze.com/uploads/images/original_untouched/1/1.jpg";
        let own_season = create(&mut admin, CREATE_SEASON,
            json!({ "showId": own_show, "season": 1, "tvmazeId": 11, "name": "Arrival", "episodeOrder": 10, "posterUrl": poster }),
            "season").await;
        create(
            &mut admin,
            CREATE_SEASON,
            json!({ "showId": other_show, "season": 1 }),
            "season",
        )
        .await;

        // Upsert keys on (show, season): a second sync updates instead of duplicating.
        let again = create(&mut admin, CREATE_SEASON,
            json!({ "showId": own_show, "season": 1, "tvmazeId": 11, "name": "Arrival (Part 1)", "posterUrl": poster }),
            "season").await;
        assert_eq!(again, own_season);

        let visible = member
            .query("query { seasons(page: { limit: 10 }) { edges { node { id showId name posterUrl episodeOrder } } } }", json!({}))
            .await;
        let edges = visible["seasons"]["edges"]
            .as_array()
            .expect("season edges");
        assert_eq!(
            edges.len(),
            1,
            "a member sees only their own shows' seasons: {visible}"
        );
        assert_eq!(edges[0]["node"]["id"], own_season);
        assert_eq!(edges[0]["node"]["name"], "Arrival (Part 1)");
        assert_eq!(edges[0]["node"]["posterUrl"], poster);

        let related = member
            .query(
                "query($id: String!) { show(id: $id) { seasons { edges { node { season } } } } }",
                json!({ "id": own_show }),
            )
            .await;
        assert_eq!(related["show"]["seasons"]["edges"][0]["node"]["season"], 1);

        let write = member
            .post(
                CREATE_SEASON,
                json!({ "input": { "showId": own_show, "season": 2 } }),
            )
            .await;
        assert!(
            !write.errors().is_empty() || write.body["data"]["result"]["success"] != json!(true),
            "seasons are admin.write: a member cannot create one: {}",
            write.body
        );

        // Episodes keep the still source the artwork cache downloads from.
        let still = "https://static.tvmaze.com/uploads/images/original_untouched/2/2.jpg";
        let episode = create(&mut admin,
            "mutation($input: CreateEpisodeInput!) { result: createEpisode(input: $input) { success error episode { id } } }",
            json!({ "showId": own_show, "season": 1, "episode": 1, "wanted": false, "stillUrl": still }),
            "episode").await;
        let read = member
            .query(
                "query($id: String!) { episode(id: $id) { stillUrl } }",
                json!({ "id": episode }),
            )
            .await;
        assert_eq!(read["episode"]["stillUrl"], still);

        app.shutdown().await;
    });
}

/// Serve as a browser would: the member's session cookies and an optional ETag.
async fn get_artwork(
    app: &TestApp,
    client: &common::GqlClient,
    path: &str,
    etag: Option<&str>,
) -> reqwest::Response {
    let cookie = client
        .cookies
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ");
    let mut request = reqwest::Client::new()
        .get(format!("{}{}", app.base_url, path))
        .header("Cookie", cookie);
    if let Some(etag) = etag {
        request = request.header("If-None-Match", etag);
    }
    request.send().await.expect("artwork request")
}

#[test]
fn cached_artwork_serves_stored_resized_variants_and_revalidates() {
    use graphql_orm::graphql::filters::StringFilter;
    use graphql_orm_storage::{StorageNamespace, StoragePutRequest};
    use librarian::graphql::entities::{
        ArtworkCache, ArtworkCacheWhereInput, CreateArtworkCacheInput, CreateStorageObjectInput,
        StorageObject,
    };

    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");
        InviteToken::insert(
            &app.db,
            CreateInviteTokenInput {
                token: "invite-artwork-member".to_string(),
                created_by: admin_id.clone(),
                library_ids: Vec::new(),
                role: "member".to_string(),
                access_level: "full".to_string(),
                expires_at: None,
                max_uses: Some(1),
                use_count: 0,
                apply_restrictions: false,
                restrictions_template: None,
                is_active: true,
            },
        )
        .await
        .expect("invite fixture");
        let mut member = app.client();
        let registered = member
            .query(
                REGISTER,
                json!({ "input": { "email": "artwork@example.test", "name": "Member",
                "password": DEFAULT_PASSWORD, "inviteToken": "invite-artwork-member" } }),
            )
            .await;
        let member_id = registered["register"]["user"]["id"]
            .as_str()
            .expect("member id")
            .to_string();
        let own_show = show_for(&app, &mut admin, &member_id, "member-art").await;
        let other_show = show_for(&app, &mut admin, &admin_id, "admin-art").await;
        const SEASON: &str = "mutation($input: CreateSeasonInput!) { result: upsertSeason(input: $input) { success error season { id } } }";
        let own_season = create(
            &mut admin,
            SEASON,
            json!({ "showId": own_show, "season": 1 }),
            "season",
        )
        .await;
        let other_season = create(
            &mut admin,
            SEASON,
            json!({ "showId": other_show, "season": 1 }),
            "season",
        )
        .await;

        // A poster already downloaded into object storage (downloads need a public host).
        let storage = app.services.get_storage().await.expect("object storage");
        let mut poster = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(680, 1000)
            .write_to(&mut poster, image::ImageFormat::Jpeg)
            .expect("encode poster");
        let stored = storage
            .storage()
            .put_object(StoragePutRequest {
                namespace: StorageNamespace::Derivatives,
                file_name: Some("poster.jpg".to_string()),
                mime_type: Some("image/jpeg".to_string()),
                bytes: poster.into_inner(),
            })
            .await
            .expect("store poster");
        let object = StorageObject::insert(
            &app.db,
            CreateStorageObjectInput {
                object_id: stored.object_id.to_string(),
                namespace: stored.namespace.as_str().to_string(),
                backend: stored.backend.as_str().to_string(),
                storage_key: stored.storage_key.clone(),
                original_file_name: stored.original_file_name.clone(),
                mime_type: stored.mime_type.clone(),
                size_bytes: stored.size_bytes as i64,
                sha256_hex: stored.sha256_hex.clone(),
            },
        )
        .await
        .expect("storage metadata");
        for season in [&own_season, &other_season] {
            ArtworkCache::insert(
                &app.db,
                CreateArtworkCacheInput {
                    entity_type: "season".to_string(),
                    entity_id: season.clone(),
                    artwork_type: "poster".to_string(),
                    storage_object_id: object.id.clone(),
                    source_url: None,
                    width: Some(680),
                    height: Some(1000),
                },
            )
            .await
            .expect("artwork cache row");
        }

        let original = get_artwork(
            &app,
            &member,
            &format!("/api/artwork/season/{own_season}/poster"),
            None,
        )
        .await;
        assert_eq!(original.status(), 200);
        let etag = original.headers()["etag"]
            .to_str()
            .expect("etag")
            .to_string();

        let resized = get_artwork(
            &app,
            &member,
            &format!("/api/artwork/season/{own_season}/poster?w=300"),
            None,
        )
        .await;
        assert_eq!(resized.status(), 200);
        let bytes = resized.bytes().await.expect("resized bytes");
        let decoded = image::load_from_memory(&bytes).expect("resized image");
        assert_eq!(
            (decoded.width(), decoded.height()),
            (342, 502),
            "300px snaps up to the 342px step"
        );

        // The variant is stored once and reused.
        get_artwork(
            &app,
            &member,
            &format!("/api/artwork/season/{own_season}/poster?w=342"),
            None,
        )
        .await;
        let rows = ArtworkCache::query(app.db.pool())
            .filter(ArtworkCacheWhereInput {
                entity_id: Some(StringFilter {
                    eq: Some(own_season.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
            .expect("artwork rows");
        let mut kinds = rows
            .iter()
            .map(|row| row.artwork_type.as_str())
            .collect::<Vec<_>>();
        kinds.sort();
        assert_eq!(kinds, ["poster", "poster@w342"]);

        // Never upscaled: wider than the original returns the original bytes.
        let wide = get_artwork(
            &app,
            &member,
            &format!("/api/artwork/season/{own_season}/poster?w=1920"),
            None,
        )
        .await;
        assert_eq!(wide.headers()["etag"].to_str().unwrap(), etag);

        let revalidated = get_artwork(
            &app,
            &member,
            &format!("/api/artwork/season/{own_season}/poster"),
            Some(&etag),
        )
        .await;
        assert_eq!(revalidated.status(), 304);

        // Another owner's season artwork is indistinguishable from missing artwork.
        let foreign = get_artwork(
            &app,
            &member,
            &format!("/api/artwork/season/{other_season}/poster"),
            None,
        )
        .await;
        assert_eq!(foreign.status(), 404);

        // Nothing cached and no source URL: a miss, not a server error.
        let episode = create(&mut admin,
            "mutation($input: CreateEpisodeInput!) { result: createEpisode(input: $input) { success error episode { id } } }",
            json!({ "showId": own_show, "season": 1, "episode": 2, "wanted": false }),
            "episode").await;
        let missing = get_artwork(
            &app,
            &member,
            &format!("/api/artwork/episode/{episode}/thumbnail"),
            None,
        )
        .await;
        assert_eq!(missing.status(), 404);

        app.shutdown().await;
    });
}

/// A cached image for one entity, stored the way the artwork service stores downloads.
async fn seed_artwork(
    app: &TestApp,
    entity_type: &str,
    entity_id: &str,
    artwork_type: &str,
) -> graphql_orm_storage::StoredObject {
    use graphql_orm_storage::{StorageNamespace, StoragePutRequest};
    use librarian::graphql::entities::{
        ArtworkCache, CreateArtworkCacheInput, CreateStorageObjectInput, StorageObject,
    };
    let storage = app.services.get_storage().await.expect("object storage");
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(40, 60)
        .write_to(&mut bytes, image::ImageFormat::Jpeg)
        .expect("encode artwork");
    let stored = storage
        .storage()
        .put_object(StoragePutRequest {
            namespace: StorageNamespace::Derivatives,
            file_name: Some(format!("{entity_type}-{artwork_type}.jpg")),
            mime_type: Some("image/jpeg".to_string()),
            bytes: bytes.into_inner(),
        })
        .await
        .expect("store artwork");
    let object = StorageObject::insert(
        &app.db,
        CreateStorageObjectInput {
            object_id: stored.object_id.to_string(),
            namespace: stored.namespace.as_str().to_string(),
            backend: stored.backend.as_str().to_string(),
            storage_key: stored.storage_key.clone(),
            original_file_name: stored.original_file_name.clone(),
            mime_type: stored.mime_type.clone(),
            size_bytes: stored.size_bytes as i64,
            sha256_hex: stored.sha256_hex.clone(),
        },
    )
    .await
    .expect("storage metadata");
    ArtworkCache::insert(
        &app.db,
        CreateArtworkCacheInput {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            artwork_type: artwork_type.to_string(),
            storage_object_id: object.id,
            source_url: None,
            width: Some(40),
            height: Some(60),
        },
    )
    .await
    .expect("artwork cache row");
    stored
}

async fn artwork_rows(app: &TestApp, entity_type: &str, entity_id: &str) -> usize {
    use graphql_orm::graphql::filters::StringFilter;
    use librarian::graphql::entities::{ArtworkCache, ArtworkCacheWhereInput};
    ArtworkCache::query(app.db.pool())
        .filter(ArtworkCacheWhereInput {
            entity_type: Some(StringFilter {
                eq: Some(entity_type.to_string()),
                ..Default::default()
            }),
            entity_id: Some(StringFilter {
                eq: Some(entity_id.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .expect("artwork rows")
        .len()
}

/// Wait for the post-commit storage hook to remove an object's bytes.
async fn assert_bytes_removed(
    app: &TestApp,
    stored: &graphql_orm_storage::StoredObject,
    label: &str,
) {
    let storage = app.services.get_storage().await.expect("object storage");
    common::wait_for(std::time::Duration::from_secs(5), label, || {
        let storage = storage.clone();
        async move {
            storage
                .storage()
                .get_object(stored)
                .await
                .is_err()
                .then_some(())
        }
    })
    .await;
}

#[test]
fn deleting_media_removes_its_cached_artwork_and_bytes() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");
        let show = show_for(&app, &mut admin, &admin_id, "deleted-show").await;
        let season = create(
            &mut admin,
            "mutation($input: CreateSeasonInput!) { result: upsertSeason(input: $input) { success error season { id } } }",
            json!({ "showId": show, "season": 1 }),
            "season",
        )
        .await;
        let episode = create(
            &mut admin,
            "mutation($input: CreateEpisodeInput!) { result: createEpisode(input: $input) { success error episode { id } } }",
            json!({ "showId": show, "season": 1, "episode": 1, "wanted": false }),
            "episode",
        )
        .await;
        let show_poster = seed_artwork(&app, "show", &show, "poster").await;
        let show_variant = seed_artwork(&app, "show", &show, "poster@w342").await;
        let season_poster = seed_artwork(&app, "season", &season, "poster").await;
        let still = seed_artwork(&app, "episode", &episode, "thumbnail").await;

        let deleted = admin
            .query(
                "mutation($id: String!) { deleteShow(id: $id) { success error } }",
                json!({ "id": show }),
            )
            .await;
        assert_eq!(deleted["deleteShow"]["success"], true, "{deleted}");

        // The show, its seasons and episodes take their artwork rows with them...
        assert_eq!(artwork_rows(&app, "show", &show).await, 0);
        assert_eq!(artwork_rows(&app, "season", &season).await, 0);
        assert_eq!(artwork_rows(&app, "episode", &episode).await, 0);
        // ...and the stored bytes, including resized copies, after the commit.
        for (stored, label) in [
            (&show_poster, "show poster bytes"),
            (&show_variant, "resized show poster bytes"),
            (&season_poster, "season poster bytes"),
            (&still, "episode still bytes"),
        ] {
            assert_bytes_removed(&app, stored, label).await;
        }

        // Collection artwork is shared by TMDB id: it outlives one library's collection.
        let mut collections = Vec::new();
        for name in ["first-library", "second-library"] {
            let path = app.path(&format!("media/{name}"));
            std::fs::create_dir_all(&path).expect("library folder");
            let library = create(
                &mut admin,
                "mutation($input: CreateLibraryInput!) { result: createLibrary(input: $input) { success error library { id } } }",
                json!({ "userId": admin_id, "name": name, "path": path.display().to_string(), "libraryType": "movies",
                        "autoScan": false, "autoOrganize": false, "namingPattern": "{title}",
                        "scanIntervalMinutes": 60, "watchForChanges": false, "scanning": false }),
                "library",
            )
            .await;
            collections.push(
                create(
                    &mut admin,
                    "mutation($input: CreateCollectionInput!) { result: createCollection(input: $input) { success error collection { id } } }",
                    json!({ "libraryId": library, "userId": admin_id, "tmdbCollectionId": 99, "name": "Saga", "movieCount": 3 }),
                    "collection",
                )
                .await,
            );
        }
        let collection_art = seed_artwork(&app, "collection", "99", "poster").await;
        const DELETE_COLLECTION: &str =
            "mutation($id: String!) { deleteCollection(id: $id) { success error } }";
        admin
            .query(DELETE_COLLECTION, json!({ "id": collections[0] }))
            .await;
        assert_eq!(
            artwork_rows(&app, "collection", "99").await,
            1,
            "still used by the other library"
        );
        admin
            .query(DELETE_COLLECTION, json!({ "id": collections[1] }))
            .await;
        assert_eq!(artwork_rows(&app, "collection", "99").await, 0);
        assert_bytes_removed(&app, &collection_art, "collection poster bytes").await;

        // Rows whose entity disappeared some other way are swept.
        let orphan = seed_artwork(&app, "movie", "no-such-movie", "poster").await;
        let storage = app.services.get_storage().await.expect("object storage");
        let removed = librarian::services::ArtworkService::new(app.db.clone(), storage)
            .sweep_orphans()
            .await
            .expect("sweep");
        assert_eq!(removed, 1);
        assert_eq!(artwork_rows(&app, "movie", "no-such-movie").await, 0);
        assert_bytes_removed(&app, &orphan, "orphaned artwork bytes").await;

        app.shutdown().await;
    });
}
