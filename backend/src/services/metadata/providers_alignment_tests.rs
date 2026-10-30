//! Authorization boundaries exercised through generated fixtures and real resolvers.
use super::*;
use crate::graphql::entities::MediaFile;
use crate::services::{AuthConfig, DatabaseServiceConfig, GraphqlServiceConfig};
use graphql_orm::graphql::auth::AuthSubject;
use serde_json::{Value, json};

struct Fixture {
    _temp: tempfile::TempDir,
    services: Arc<ServicesManager>,
    metadata: MetadataService,
    schema: LibrarianSchema,
    owner: AuthUser,
    other: AuthUser,
}

impl Fixture {
    fn request(&self, document: impl Into<String>, caller: &AuthUser) -> Request {
        // Match production request data without changing the caller's identity
        // or inventing session credentials for these direct-schema tests.
        let mut subject = AuthSubject::from_parts(
            caller.user_id.clone(),
            caller.role.iter().cloned().collect(),
            Vec::new(),
            None,
        );
        subject.user_id = Some(caller.user_id.clone());
        Request::new(document)
            .data(caller.clone())
            .data(caller.user_id.clone())
            .data(subject)
    }

    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let services = ServicesManager::builder()
            .add_service(DatabaseServiceConfig {
                database_url: format!("sqlite://{}", temp.path().join("alignment.db").display()),
                connect_timeout: std::time::Duration::from_secs(5),
            })
            .add_service(AuthConfig::for_tests())
            .add_service(GraphqlServiceConfig { server_port: 0 })
            .start()
            .await
            .unwrap();
        let db = services.get_database().await.unwrap().pool().clone();
        let schema = services
            .get_graphql()
            .await
            .unwrap()
            .schema()
            .await
            .unwrap();
        Self {
            _temp: temp,
            metadata: MetadataService::new_default(db, services.clone()),
            services,
            schema,
            owner: AuthUser {
                user_id: Uuid::new_v4().to_string(),
                email: None,
                role: Some("member".into()),
            },
            other: AuthUser {
                user_id: Uuid::new_v4().to_string(),
                email: None,
                role: Some("member".into()),
            },
        }
    }

    async fn create(&self, entity: &str, input: Value) -> String {
        let field = format!("create{entity}");
        let node = format!("{}{}", entity[..1].to_ascii_lowercase(), &entity[1..]);
        let data = self.metadata.execute_mutation(
            &AuthUser::system_admin(),
            &format!("mutation Fixture($input: Create{entity}Input!) {{ {field}(input: $input) {{ success error {node} {{ id }} }} }}"),
            json!({ "input": input }),
        ).await.unwrap();
        assert_eq!(data[&field]["success"], true, "{data}");
        data[&field][&node]["id"].as_str().unwrap().to_string()
    }

    async fn library(&self, owner: &AuthUser, name: &str) -> Uuid {
        Uuid::parse_str(&self.create("Library", json!({
            "userId": owner.user_id, "name": name, "path": format!("/fixture/{name}"), "libraryType": "movies",
            "autoScan": false, "autoOrganize": false, "namingPattern": "", "scanIntervalMinutes": 60,
            "watchForChanges": false, "scanning": false
        })).await).unwrap()
    }

    async fn movie(&self, library: Uuid, owner: &AuthUser, key: Option<i32>, name: &str) -> String {
        self.create(
            "Movie",
            json!({
                "libraryId": library.to_string(), "userId": owner.user_id, "title": name,
                "collectionId": key, "genres": [], "castNames": [], "productionCountries": [],
                "spokenLanguages": [], "monitored": false, "wanted": false, "hasFile": false
            }),
        )
        .await
    }

    async fn denied(
        &self,
        query: &str,
        variables: Value,
        caller: Option<&AuthUser>,
        expected: &str,
    ) {
        let request = match caller {
            Some(caller) => self.request(query, caller),
            None => Request::new(query),
        }
        .variables(Variables::from_json(variables));
        let response = self.schema.execute(request).await;
        assert!(
            response.errors.iter().any(|e| e.message.contains(expected)),
            "{response:?}"
        );
    }
}

#[tokio::test]
async fn all_import_entrypoints_reject_foreign_and_missing_libraries_before_provider_work() {
    let f = Fixture::new().await;
    let library = f.library(&f.other, "foreign-import").await;
    let requester = Uuid::parse_str(&f.owner.user_id).unwrap();
    for target in [library, Uuid::new_v4()] {
        let movie = f
            .metadata
            .add_movie_from_provider(AddMovieOptions {
                provider: MetadataProvider::Tmdb,
                provider_id: 42,
                library_id: target,
                user_id: requester,
                monitored: false,
            })
            .await
            .unwrap_err();
        let collection = f
            .metadata
            .add_movie_collection_from_provider(AddMovieCollectionOptions {
                provider: MetadataProvider::Tmdb,
                collection_id: 42,
                library_id: target,
                user_id: requester,
                wanted_missing: false,
            })
            .await
            .unwrap_err();
        let show = f
            .metadata
            .add_tv_show_from_provider(AddTvShowOptions {
                provider: MetadataProvider::Tvmaze,
                provider_id: 42,
                library_id: target,
                user_id: requester,
                monitor_type: AutoDownloadMode::None,
                path: None,
            })
            .await
            .unwrap_err();
        let album = f
            .metadata
            .add_album_from_provider(AddAlbumOptions {
                provider: MetadataProvider::Musicbrainz,
                provider_id: Uuid::new_v4().to_string(),
                library_id: target,
                user_id: requester,
                monitor_type: AutoDownloadMode::None,
            })
            .await
            .unwrap_err();
        let book = f
            .metadata
            .add_audiobook_from_provider(AddAudiobookOptions {
                provider: MetadataProvider::OpenLibrary,
                provider_id: "OL1W".into(),
                library_id: target,
                user_id: requester,
                monitor_type: AutoDownloadMode::None,
            })
            .await
            .unwrap_err();
        for error in [movie, collection, show, album, book] {
            assert!(
                error
                    .to_string()
                    .contains("Library not found or access denied"),
                "{error}"
            );
        }
    }
    // Exercise the real caller boundary too, including the album/book resolvers
    // whose integration is maintained by the parent task.
    for action in [
        "addMovie(libraryId: $id, input: {tmdbId: 42})",
        "addMovieCollection(libraryId: $id, input: {collectionId: 42})",
        "addTvShow(libraryId: $id, input: {tvmazeId: 42})",
        "addAlbum(input: {libraryId: $id, musicbrainzId: \"00000000-0000-0000-0000-000000000001\"})",
        "addAudiobook(input: {libraryId: $id, openlibraryId: \"OL1W\"})",
    ] {
        let query = format!("mutation($id: String!) {{ {action} {{ success error }} }}");
        f.denied(
            &query,
            json!({"id": library.to_string()}),
            Some(&f.owner),
            "Library not found or access denied",
        )
        .await;
        f.denied(
            &query,
            json!({"id": library.to_string()}),
            None,
            "Authentication required",
        )
        .await;
    }
    assert!(
        f.metadata
            .authorize_library_import(&f.owner, library)
            .await
            .is_err()
    );
    let owner = f
        .metadata
        .authorize_library_import(&AuthUser::system_admin(), library)
        .await
        .unwrap();
    assert_eq!(owner.to_string(), f.other.user_id);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn collection_query_rejects_foreign_missing_and_anonymous_callers_before_provider_work() {
    let f = Fixture::new().await;
    let library = f.library(&f.other, "foreign-collection").await;
    const QUERY: &str = "query($id: String!) { movieCollectionDetails(libraryId: $id, collectionId: 42) { name movies { libraryMovieId } } }";
    for target in [library, Uuid::new_v4()] {
        f.denied(
            QUERY,
            json!({"id": target.to_string()}),
            Some(&f.owner),
            "Library not found or access denied",
        )
        .await;
    }
    f.denied(
        QUERY,
        json!({"id": library.to_string()}),
        None,
        "Authentication required",
    )
    .await;
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn quality_evaluation_rejects_foreign_and_unowned_files_without_persisting() {
    let f = Fixture::new().await;
    let library = f.library(&f.other, "foreign-quality").await;
    const ACTION: &str = "mutation($id: String!) { evaluateMediaFileQuality(mediaFileId: $id) { success qualityStatus error } }";
    for library_id in [Some(library.to_string()), None] {
        let id = f.create("MediaFile", json!({
            "libraryId": library_id, "path": format!("/fixture/{}.mkv", Uuid::new_v4()), "size": 1000,
            "isHdr": false, "addedAt": "2026-10-24T00:00:00Z", "qualityStatus": "suboptimal"
        })).await;
        f.denied(
            ACTION,
            json!({"id": id}),
            Some(&f.owner),
            "Media file not found or access denied",
        )
        .await;
        f.denied(ACTION, json!({"id": id}), None, "Authentication required")
            .await;
        let row = MediaFile::get(f.metadata.db.pool(), &id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.quality_status.as_deref(), Some("suboptimal"));
    }
    f.denied(
        ACTION,
        json!({"id": Uuid::new_v4().to_string()}),
        Some(&f.owner),
        "Media file not found or access denied",
    )
    .await;
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn source_profile_override_does_not_bypass_foreign_target_or_conflicting_ids() {
    let f = Fixture::new().await;
    let library = f.library(&f.other, "foreign-search").await;
    let movie = f
        .movie(library, &f.other, None, "Private search target")
        .await;
    let profiles = f
        .metadata
        .execute_query(
            &AuthUser::system_admin(),
            "{ qualityProfiles { edges { node { id } } } }",
            json!({}),
        )
        .await
        .unwrap();
    let profile = profiles["qualityProfiles"]["edges"][0]["node"]["id"]
        .as_str()
        .unwrap();
    const QUERY: &str =
        "query($input: SearchSourcesInput!) { searchSources(input: $input) { totalReleases } }";
    for override_id in [None, Some(profile)] {
        let mut input = json!({"query": "fixture", "movieId": movie});
        if let Some(id) = override_id {
            input["qualityProfileId"] = json!(id);
        }
        f.denied(
            QUERY,
            json!({"input": input}),
            Some(&f.owner),
            "Search target not found or access denied",
        )
        .await;
    }
    f.denied(QUERY, json!({"input": {"query": "fixture", "movieId": movie, "showId": "other", "qualityProfileId": profile}}), Some(&f.owner), "Specify at most one search target").await;
    f.denied(
        QUERY,
        json!({"input": {"query": "fixture", "qualityProfileId": "missing"}}),
        Some(&f.owner),
        "Quality profile not found or access denied",
    )
    .await;
    f.denied(
        QUERY,
        json!({"input": {"query": "fixture", "movieId": movie}}),
        None,
        "Authentication required",
    )
    .await;
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn collection_membership_uses_library_and_provider_key_for_owner_and_admin() {
    let f = Fixture::new().await;
    let first = f.library(&f.owner, "first-collection").await;
    let second = f.library(&f.owner, "second-collection").await;
    let foreign = f.library(&f.other, "third-collection").await;
    let included = f.movie(first, &f.owner, Some(42), "Included").await;
    f.movie(first, &f.owner, Some(43), "Different provider key")
        .await;
    f.movie(first, &f.owner, None, "No provider key").await;
    f.movie(second, &f.owner, Some(42), "Same owner different library")
        .await;
    f.movie(foreign, &f.other, Some(42), "Other owner").await;
    let collection = f
        .create(
            "Collection",
            json!({"libraryId": first.to_string(), "userId": f.owner.user_id,
        "tmdbCollectionId": 42, "name": "Fixture collection", "movieCount": 1}),
        )
        .await;
    for caller in [&f.owner, &AuthUser::system_admin()] {
        let response = f.schema.execute(f.request("query($id: String!) { collection(id: $id) { movies { edges { node { id } } pageInfo { totalCount } } } }", caller)
            .variables(Variables::from_json(json!({"id": collection})))).await;
        assert!(response.errors.is_empty(), "{response:?}");
        let data = response.data.into_json().unwrap();
        assert_eq!(data["collection"]["movies"]["pageInfo"]["totalCount"], 1);
        assert_eq!(
            data["collection"]["movies"]["edges"][0]["node"]["id"],
            included
        );
    }
    f.services.stop_all().await.unwrap();
}

#[test]
fn collection_overlay_preserves_local_metadata_and_external_only_parts() {
    let local = LocalCollectionMovieRow {
        id: "local".into(),
        tmdb_id: None,
        collection_id: None,
        title: "Catalog fixture".into(),
        year: Some(2026),
        poster_url: None,
        media_file_id: Some("file".into()),
        file_size_bytes: Some(1000),
        resolution: Some("1080p".into()),
        video_codec: None,
        audio_codec: None,
        audio_channels: None,
        wanted: false,
    };
    let collection = serde_json::from_value(json!({
        "id": 42, "name": "Catalog", "overview": null, "poster_path": null, "backdrop_path": null,
        "parts": [
            {"id": 1, "title": "Catalog fixture", "release_date": "2026-01-01", "adult": false, "video": false},
            {"id": 2, "title": "External only", "release_date": "2026-01-01", "adult": false, "video": false}
        ]
    })).unwrap();
    let result = MetadataService::movie_collection_overlay(
        &TmdbClient::new("fixture".into()),
        collection,
        Uuid::new_v4(),
        vec![local.clone()],
    );
    assert_eq!(result.movies.len(), 2);
    assert_eq!(result.movies[0].library_movie_id.as_deref(), Some("local"));
    assert_eq!(result.movies[0].media_file_id.as_deref(), Some("file"));
    assert_eq!(result.movies[1].library_movie_id, None);
    assert!(!result.movies[1].wanted);
    // The pure overlay has no database/service handle and leaves candidate IDs unmodified.
    assert_eq!(local.tmdb_id, None);
    assert_eq!(local.collection_id, None);
}

#[tokio::test]
async fn catalog_overlay_reads_only_owned_candidates_and_does_not_persist_matches() {
    let f = Fixture::new().await;
    let library = f.library(&f.owner, "pure-overlay").await;
    let other_library = f.library(&f.other, "private-overlay").await;
    let local = f.movie(library, &f.owner, None, "Catalog fixture").await;
    let foreign = f
        .movie(other_library, &f.other, Some(42), "External only")
        .await;
    let before = Movie::get(f.metadata.db.pool(), &local)
        .await
        .unwrap()
        .unwrap();
    let foreign_before = Movie::get(f.metadata.db.pool(), &foreign)
        .await
        .unwrap()
        .unwrap();
    let catalog = || {
        serde_json::from_value(json!({
        "id": 42, "name": "Catalog", "overview": null,
        "poster_path": "/poster.jpg", "backdrop_path": "/backdrop.jpg",
        "parts": [
            {"id": 1, "title": "Catalog fixture", "release_date": "2026-01-01", "adult": false, "video": false},
            {"id": 2, "title": "External only", "release_date": "2026-01-01", "adult": false, "video": false}
        ]
    })).unwrap()
    };
    for _ in 0..2 {
        let result = f
            .metadata
            .overlay_movie_collection_details(
                &TmdbClient::new("fixture".into()),
                catalog(),
                library,
                &f.owner,
            )
            .await
            .unwrap();
        assert_eq!(
            result.movies[0].library_movie_id.as_deref(),
            Some(local.as_str())
        );
        assert_eq!(result.movies[1].library_movie_id, None);
        assert!(
            result
                .poster_url
                .as_deref()
                .unwrap()
                .contains("/poster.jpg")
        );
    }
    let after = Movie::get(f.metadata.db.pool(), &local)
        .await
        .unwrap()
        .unwrap();
    let foreign_after = Movie::get(f.metadata.db.pool(), &foreign)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(after).unwrap()
    );
    assert_eq!(
        serde_json::to_value(foreign_before).unwrap(),
        serde_json::to_value(foreign_after).unwrap()
    );
    f.services.stop_all().await.unwrap();
}

#[test]
fn relation_metadata_has_one_stable_upward_route_and_preserves_physical_foreign_keys() {
    use crate::graphql::entities::*;
    use graphql_orm::graphql::orm::{Entity, RelationChangePropagation};
    use std::collections::{BTreeMap, BTreeSet};
    let mut metadata = Vec::new();
    macro_rules! record { ($($ty:ty),+ $(,)?) => { $(metadata.push(<$ty as Entity>::metadata());)+ }; }
    record!(
        Library,
        Movie,
        Show,
        Collection,
        Artist,
        Album,
        Audiobook,
        Episode,
        Season,
        Track,
        Chapter,
        MediaFile,
        VideoStream,
        AudioStream,
        Subtitle,
        MediaChapter,
        TorrentFile,
        RssFeed,
        RssFeedItem,
        LibraryScanRun,
        LibraryScanIssue,
        MovieCastCredit,
        PlaybackProgress,
        PlaybackSession,
        PendingFileMatch,
        Notification
    );
    let expected_up: BTreeMap<_, _> = [
        ("Movie", "library"),
        ("Show", "library"),
        ("Collection", "library"),
        ("Artist", "library"),
        ("Album", "library"),
        ("Audiobook", "library"),
        ("Episode", "show"),
        ("Season", "show"),
        ("Track", "album"),
        ("Chapter", "audiobook"),
        ("MediaFile", "library"),
        ("VideoStream", "mediaFile"),
        ("AudioStream", "mediaFile"),
        ("Subtitle", "mediaFile"),
        ("MediaChapter", "mediaFile"),
        ("TorrentFile", "torrent"),
        ("RssFeedItem", "feed"),
        ("LibraryScanRun", "library"),
        ("LibraryScanIssue", "scanRun"),
        ("MovieCastCredit", "movie"),
    ]
    .into_iter()
    .collect();
    let existing_fks: BTreeSet<_> = [
        ("Artist", "library"),
        ("Album", "library"),
        ("Audiobook", "library"),
        ("Chapter", "audiobook"),
        ("Chapter", "mediaFile"),
        ("Collection", "library"),
        ("Episode", "show"),
        // Seasons are owned rows of their show: deleting the show removes them.
        ("Season", "show"),
        ("Episode", "mediaFile"),
        ("Movie", "library"),
        ("Movie", "mediaFile"),
        ("MovieCastCredit", "movie"),
        ("MovieCastCredit", "person"),
        ("Show", "library"),
        ("Track", "album"),
        ("Track", "library"),
        ("Track", "mediaFile"),
    ]
    .into_iter()
    .collect();
    let mut actual_fks = BTreeSet::new();
    for entity in &metadata {
        let mut fields = BTreeSet::new();
        let mut upward = Vec::new();
        for relation in entity.relations.iter() {
            assert!(
                fields.insert(relation.field_name),
                "duplicate {}.{}",
                entity.entity_name,
                relation.field_name
            );
            if relation.emit_foreign_key {
                actual_fks.insert((entity.entity_name, relation.field_name));
                let expected_delete = if relation.field_name == "mediaFile" {
                    graphql_orm::graphql::orm::DeletePolicy::SetNull
                } else {
                    graphql_orm::graphql::orm::DeletePolicy::Cascade
                };
                assert_eq!(
                    relation.on_delete, expected_delete,
                    "existing {}.{} delete policy",
                    entity.entity_name, relation.field_name
                );
            }
            if relation.propagate_change == RelationChangePropagation::Up {
                assert!(!relation.is_multiple);
                assert_eq!(relation.target_columns, &["id"]);
                assert_eq!(relation.source_columns.len(), 1);
                upward.push(relation.field_name);
            }
        }
        assert_eq!(
            upward,
            expected_up
                .get(entity.entity_name)
                .copied()
                .into_iter()
                .collect::<Vec<_>>(),
            "{} propagation",
            entity.entity_name
        );
    }
    assert_eq!(
        actual_fks, existing_fks,
        "nested reads must not add physical constraints or delete cascades"
    );
}

#[tokio::test]
async fn generated_schema_exposes_new_relations_without_duplicate_fields() {
    let f = Fixture::new().await;
    let sdl = f.schema.sdl();
    for (entity, fields) in [
        (
            "Library",
            &[
                "artists",
                "scanRuns",
                "scanIssues",
                "rssFeeds",
                "qualityProfile",
            ][..],
        ),
        ("Movie", &["castCredits", "qualityProfile"]),
        ("Show", &["qualityProfile"]),
        ("Artist", &["tracks"]),
        ("Album", &["artist", "qualityProfile"]),
        ("Audiobook", &["qualityProfile"]),
        ("Track", &["artist"]),
        (
            "MediaFile",
            &[
                "library",
                "episode",
                "movie",
                "track",
                "chapter",
                "videoStreams",
                "audioStreams",
                "subtitles",
                "mediaChapters",
            ],
        ),
        ("VideoStream", &["mediaFile"]),
        ("AudioStream", &["mediaFile"]),
        ("Subtitle", &["mediaFile"]),
        ("MediaChapter", &["mediaFile"]),
        ("TorrentFile", &["torrent", "mediaFile"]),
        ("RssFeed", &["library", "items"]),
        ("RssFeedItem", &["feed", "torrent"]),
        ("LibraryScanRun", &["library", "issues"]),
        ("LibraryScanIssue", &["scanRun", "library", "mediaFile"]),
        ("PlaybackProgress", &["mediaFile"]),
        (
            "PlaybackSession",
            &[
                "mediaFile",
                "episode",
                "movie",
                "track",
                "audiobook",
                "show",
                "album",
            ],
        ),
        (
            "PendingFileMatch",
            &["episode", "movie", "track", "chapter", "torrent"],
        ),
        (
            "Notification",
            &["library", "torrent", "mediaFile", "pendingMatch"],
        ),
    ] {
        let declaration = format!("type {entity} {{");
        let body = sdl
            .split_once(&declaration)
            .unwrap_or_else(|| panic!("missing {entity}"))
            .1
            .split_once('}')
            .unwrap()
            .0;
        for field in fields {
            let count = body
                .lines()
                .filter(|line| {
                    let line = line.trim_start();
                    line.strip_prefix(field)
                        .is_some_and(|tail| tail.starts_with(':') || tail.starts_with('('))
                })
                .count();
            assert_eq!(count, 1, "{entity}.{field} must exist exactly once");
        }
    }
    f.services.stop_all().await.unwrap();
}

async fn arm_subscription<S>(stream: &mut S)
where
    S: futures::Stream<Item = async_graphql::Response> + Unpin,
{
    use futures::StreamExt;
    futures::future::poll_fn(|cx| match stream.poll_next_unpin(cx) {
        std::task::Poll::Pending => std::task::Poll::Ready(()),
        std::task::Poll::Ready(response) => {
            panic!("subscription did not become pending: {response:?}")
        }
    })
    .await;
}

async fn propagated_event<S>(stream: &mut S, field: &str, source: &str, source_id: &str) -> Value
where
    S: futures::Stream<Item = async_graphql::Response> + Unpin,
{
    use futures::StreamExt;
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        for _ in 0..32 {
            let response = stream.next().await.expect("subscription ended");
            assert!(response.errors.is_empty(), "{response:?}");
            let data = response.data.into_json().unwrap();
            let event = &data[field];
            if event["changeKind"] == "PROPAGATED"
                && event["sourceEntity"] == source
                && event["sourceId"] == source_id
            {
                return event.clone();
            }
        }
        panic!("expected propagated event missing among 32 parent events");
    })
    .await
    .expect("propagated parent event timed out")
}

#[tokio::test]
async fn episode_change_propagates_to_show_and_library_with_caller_scoped_delivery() {
    use crate::graphql::entities::Show;
    use futures::StreamExt;
    let f = Fixture::new().await;
    let library = f.library(&f.owner, "episode-events").await;
    let show = f
        .create(
            "Show",
            json!({"libraryId": library.to_string(), "userId": f.owner.user_id,
        "name": "Fixture show", "genres": [], "autoDownload": false, "autoDownloadMode": "NONE"}),
        )
        .await;
    let episode = f
        .create(
            "Episode",
            json!({"showId": show, "season": 1, "episode": 1, "title": "Before", "wanted": false}),
        )
        .await;
    let before = Show::get(f.metadata.db.pool(), &show)
        .await
        .unwrap()
        .unwrap();
    let mut show_events = Box::pin(f.schema.execute_stream(f.request(
        "subscription { showChanged { id changeKind sourceEntity sourceId path } }",
        &f.owner,
    )));
    let mut library_events = Box::pin(f.schema.execute_stream(f.request(
        "subscription { libraryChanged { id changeKind sourceEntity sourceId path } }",
        &f.owner,
    )));
    let mut foreign_events = Box::pin(f.schema.execute_stream(f.request(
        "subscription { showChanged { id changeKind sourceEntity sourceId path } }",
        &f.other,
    )));
    arm_subscription(&mut show_events).await;
    arm_subscription(&mut library_events).await;
    arm_subscription(&mut foreign_events).await;
    f.metadata.execute_mutation(&f.owner,
        "mutation($id: String!) { updateEpisode(id: $id, input: {title: \"After\"}) { success error } }",
        json!({"id": episode})).await.unwrap();
    let parent = propagated_event(&mut show_events, "showChanged", "Episode", &episode).await;
    assert_eq!(parent["id"], show);
    assert_eq!(parent["path"], json!(["show"]));
    let ancestor =
        propagated_event(&mut library_events, "libraryChanged", "Episode", &episode).await;
    assert_eq!(ancestor["id"], library.to_string());
    assert_eq!(ancestor["path"], json!(["show", "library"]));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), foreign_events.next())
            .await
            .is_err()
    );
    let after = Show::get(f.metadata.db.pool(), &show)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        before.name, after.name,
        "propagation must not rewrite parent display metadata"
    );
    drop((show_events, library_events, foreign_events));
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn stream_change_propagates_via_media_file_library_with_all_content_targets_null() {
    let f = Fixture::new().await;
    let library = f.library(&f.owner, "stream-events").await;
    let media_file = f.create("MediaFile", json!({"libraryId": library.to_string(), "path": "/fixture/streams.mkv", "size": 1000, "isHdr": false, "addedAt": "2026-10-24T00:00:00Z"})).await;
    let mut file_events = Box::pin(f.schema.execute_stream(f.request(
        "subscription { mediaFileChanged { id changeKind sourceEntity sourceId path } }",
        &f.owner,
    )));
    let mut library_events = Box::pin(f.schema.execute_stream(f.request(
        "subscription { libraryChanged { id changeKind sourceEntity sourceId path } }",
        &f.owner,
    )));
    arm_subscription(&mut file_events).await;
    arm_subscription(&mut library_events).await;
    let audio = f.create("AudioStream", json!({"mediaFileId": media_file, "streamIndex": 0, "codec": "flac", "channels": 2, "isDefault": true, "isCommentary": false})).await;
    assert_eq!(
        propagated_event(&mut file_events, "mediaFileChanged", "AudioStream", &audio).await["path"],
        json!(["mediaFile"])
    );
    let ancestor =
        propagated_event(&mut library_events, "libraryChanged", "AudioStream", &audio).await;
    assert_eq!(ancestor["id"], library.to_string());
    assert_eq!(ancestor["path"], json!(["mediaFile", "library"]));
    let data = f.metadata.execute_query(&f.owner,
        "query($id: String!) { mediaFile(id: $id) { library { id } movie { id } episode { id } track { id } chapter { id } audioStreams { edges { node { id mediaFile { id } } } pageInfo { totalCount } } videoStreams { pageInfo { totalCount } } subtitles { pageInfo { totalCount } } mediaChapters { pageInfo { totalCount } } } }",
        json!({"id": media_file})).await.unwrap();
    assert_eq!(data["mediaFile"]["library"]["id"], library.to_string());
    assert_eq!(
        data["mediaFile"]["audioStreams"]["edges"][0]["node"]["id"],
        audio
    );
    assert_eq!(
        data["mediaFile"]["audioStreams"]["edges"][0]["node"]["mediaFile"]["id"],
        media_file
    );
    for target in ["movie", "episode", "track", "chapter"] {
        assert!(data["mediaFile"][target].is_null());
    }
    drop((file_events, library_events));
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn personal_progress_has_nested_file_reads_without_library_fanout_or_foreign_content() {
    use futures::StreamExt;
    let f = Fixture::new().await;
    let library = f.library(&f.owner, "progress-local").await;
    let foreign_library = f.library(&f.other, "progress-foreign").await;
    let media_file = f.create("MediaFile", json!({"libraryId": library.to_string(), "path": "/fixture/progress.mkv", "size": 1000, "isHdr": false, "addedAt": "2026-10-24T00:00:00Z"})).await;
    let foreign_file = f.create("MediaFile", json!({"libraryId": foreign_library.to_string(), "path": "/fixture/foreign-progress.mkv", "size": 1000, "isHdr": false, "addedAt": "2026-10-24T00:00:00Z"})).await;
    let mut library_events = Box::pin(
        f.schema
            .execute_stream(f.request("subscription { libraryChanged { id } }", &f.owner)),
    );
    arm_subscription(&mut library_events).await;
    let progress = f.create("PlaybackProgress", json!({"userId": f.owner.user_id, "mediaFileId": media_file, "currentPosition": 5.0, "progressPercent": 5.0, "isWatched": false})).await;
    let foreign_progress = f.create("PlaybackProgress", json!({"userId": f.owner.user_id, "mediaFileId": foreign_file, "currentPosition": 5.0, "progressPercent": 5.0, "isWatched": false})).await;
    let query =
        "query($id: String!) { playbackProgress(id: $id) { mediaFile { id library { id } } } }";
    let own = f
        .metadata
        .execute_query(&f.owner, query, json!({"id": progress}))
        .await
        .unwrap();
    assert_eq!(own["playbackProgress"]["mediaFile"]["id"], media_file);
    let hidden = f
        .metadata
        .execute_query(&f.owner, query, json!({"id": foreign_progress}))
        .await
        .unwrap();
    assert!(hidden["playbackProgress"]["mediaFile"].is_null());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), library_events.next())
            .await
            .is_err()
    );
    drop(library_events);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn track_and_chapter_changes_propagate_through_one_parent_path_to_library() {
    let f = Fixture::new().await;
    let library = f.library(&f.owner, "audio-parent-events").await;
    let artist = f
        .create(
            "Artist",
            json!({"libraryId": library.to_string(), "userId": f.owner.user_id, "name": "Artist"}),
        )
        .await;
    let album = f
        .create(
            "Album",
            json!({"libraryId": library.to_string(), "userId": f.owner.user_id,
        "artistId": artist, "name": "Album", "genres": [], "autoDownload": false,
        "autoDownloadMode": "NONE", "hasFiles": false}),
        )
        .await;
    let track = f
        .create(
            "Track",
            json!({"libraryId": library.to_string(), "albumId": album,
        "title": "Track", "trackNumber": 1, "explicit": false, "wanted": false}),
        )
        .await;
    let book = f.create("Audiobook", json!({"libraryId": library.to_string(), "userId": f.owner.user_id,
        "title": "Book", "narrators": [], "autoDownload": false, "autoDownloadMode": "NONE", "hasFiles": false})).await;
    let chapter = f
        .create(
            "Chapter",
            json!({"audiobookId": book, "chapterNumber": 1,
        "title": "Chapter", "startTimeSecs": 0.0, "wanted": false}),
        )
        .await;
    for (entity, child, field, parent_id, edge) in [
        ("Track", track, "albumChanged", album, "album"),
        ("Chapter", chapter, "audiobookChanged", book, "audiobook"),
    ] {
        let mut parent_events = Box::pin(f.schema.execute_stream(f.request(
            format!("subscription {{ {field} {{ id changeKind sourceEntity sourceId path }} }}"),
            &f.owner,
        )));
        let mut library_events = Box::pin(f.schema.execute_stream(f.request(
            "subscription { libraryChanged { id changeKind sourceEntity sourceId path } }",
            &f.owner,
        )));
        arm_subscription(&mut parent_events).await;
        arm_subscription(&mut library_events).await;
        f.metadata.execute_mutation(&f.owner,
            &format!("mutation($id: String!) {{ update{entity}(id: $id, input: {{title: \"Edited\"}}) {{ success error }} }}"),
            json!({"id": child})).await.unwrap();
        let parent = propagated_event(&mut parent_events, field, entity, &child).await;
        assert_eq!(parent["id"], parent_id);
        assert_eq!(parent["path"], json!([edge]));
        let ancestor =
            propagated_event(&mut library_events, "libraryChanged", entity, &child).await;
        assert_eq!(ancestor["id"], library.to_string());
        assert_eq!(ancestor["path"], json!([edge, "library"]));
        drop((parent_events, library_events));
    }
    f.services.stop_all().await.unwrap();
}
