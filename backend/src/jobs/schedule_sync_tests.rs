use super::*;
use crate::services::{DatabaseServiceConfig, graphql::build_schema};

fn entry(id: u32, date: &str, streaming: bool) -> TvMazeScheduleEntry {
    let show = json!({"id":42,"name":"Guide fixture","genres":["Drama"],"image":{"medium":"https://static.tvmaze.com/uploads/images/medium_portrait/1/2.jpg"}});
    let mut value = json!({"id":id,"name":"Episode","season":1,"number":null,"airdate":date,"airtime":"20:00","airstamp":format!("{date}T20:00:00+02:00")});
    if streaming {
        value["_embedded"] = json!({"show":show});
    } else {
        value["show"] = show;
    }
    serde_json::from_value(value).unwrap()
}

#[test]
fn guide_parses_both_schedule_shapes_and_nullable_special_numbers() {
    for streaming in [false, true] {
        let entry = entry(10, "2026-09-06", streaming);
        let data = schedule_input(&entry, "US").unwrap();
        assert_eq!(data["tvmazeShowId"], 42);
        assert_eq!(data["episodeNumber"], 0);
        assert_eq!(data["airStamp"], "2026-09-06T20:00:00+02:00");
        assert_eq!(
            data["showPosterUrl"],
            "https://static.tvmaze.com/uploads/images/medium_portrait/1/2.jpg"
        );
    }
}

#[tokio::test]
async fn guide_cache_upserts_prunes_and_exposes_only_public_posters() {
    let temp = tempfile::tempdir().unwrap();
    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: format!("sqlite://{}", temp.path().join("guide.db").display()),
            connect_timeout: Duration::from_secs(5),
        })
        .start()
        .await
        .unwrap();
    let service = services.get_database().await.unwrap();
    let db = service.pool();
    let schema = build_schema(db.clone(), (), services.clone());
    let today = NaiveDate::from_ymd_opt(2026, 9, 6).unwrap();
    let entries = HashMap::from([
        (10, entry(10, "2026-09-06", false)),
        (11, entry(11, "2026-09-07", true)),
    ]);
    persist_entries(db, &schema, "US", today, &entries)
        .await
        .unwrap();
    persist_entries(db, &schema, "US", today, &entries)
        .await
        .unwrap();
    assert_eq!(
        ScheduleCache::query(db.pool())
            .fetch_all()
            .await
            .unwrap()
            .len(),
        2
    );
    let denied = schema.execute("{scheduleCaches {edges {node {id}}}}").await;
    assert!(!denied.errors.is_empty());
    let showcase = schema.execute("{showcaseArtwork {title posterUrl}}").await;
    assert!(showcase.errors.is_empty(), "{:?}", showcase.errors);
    assert_eq!(
        showcase.data.into_json().unwrap()["showcaseArtwork"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let member = AuthUser {
        user_id: "member".into(),
        email: None,
        role: Some("member".into()),
    };
    let schedule = schema.execute(authenticated_request("{scheduleCaches(where:{airDate:{gte:\"2026-09-06\",lte:\"2026-09-07\"}},orderBy:[{airStamp:ASC}]){edges{node{tvmazeEpisodeId airStamp}}}}", member)).await;
    assert!(schedule.errors.is_empty(), "{:?}", schedule.errors);
    assert_eq!(
        schedule.data.into_json().unwrap()["scheduleCaches"]["edges"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    persist_entries(
        db,
        &schema,
        "US",
        today,
        &HashMap::from([(11, entry(11, "2026-09-07", true))]),
    )
    .await
    .unwrap();
    let remaining = ScheduleCache::query(db.pool()).fetch_all().await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].tvmaze_episode_id, 11);
    let anonymous_releases = schema
        .execute("{movieReleases(kind:UPCOMING){title}}")
        .await;
    assert!(!anonymous_releases.errors.is_empty());
    let sdl = schema.sdl();
    assert!(sdl.contains("movieReleases(kind: MovieReleaseKind!, region: String, page: Int! = 1): [MovieSearchResult!]!"));
    assert!(sdl.contains("airStamp: String"));
    services.stop_all().await.unwrap();
}

#[tokio::test]
async fn guide_daily_sync_fetches_broadcast_and_web_and_preserves_cache_on_failure() {
    use axum::{
        Json, Router,
        extract::{Query, State},
        http::StatusCode,
        routing::get,
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    #[derive(Clone, Default)]
    struct Upstream {
        fail: Arc<AtomicBool>,
        broadcast: Arc<AtomicUsize>,
        web: Arc<AtomicUsize>,
    }
    async fn serve(
        State(state): State<Upstream>,
        Query(query): Query<HashMap<String, String>>,
    ) -> std::result::Result<Json<Value>, StatusCode> {
        if state.fail.load(Ordering::SeqCst) {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let country = query.get("country").unwrap();
        if country.is_empty() {
            state.web.fetch_add(1, Ordering::SeqCst);
        } else {
            assert_eq!(country, "US");
            state.broadcast.fetch_add(1, Ordering::SeqCst);
        }
        let date = query.get("date").unwrap();
        let id = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .unwrap()
            .signed_duration_since(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
            .num_days() as u32;
        Ok(Json(
            json!([{"id":id,"name":"Fixture","season":1,"number":1,"airdate":date,"airstamp":format!("{date}T20:00:00Z"),"show":{"id":42,"name":"Fixture show","genres":[]}}]),
        ))
    }
    let upstream = Upstream::default();
    let state = upstream.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route("/schedule", get(serve))
                .route("/schedule/web", get(serve))
                .with_state(state),
        )
        .await
        .unwrap()
    });
    let client = TvMazeClient::with_test_url(format!("http://{address}"));
    let temp = tempfile::tempdir().unwrap();
    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: format!("sqlite://{}", temp.path().join("daily.db").display()),
            connect_timeout: Duration::from_secs(5),
        })
        .add_service(crate::services::AuthConfig::for_tests())
        .add_service(crate::services::GraphqlServiceConfig { server_port: 0 })
        .start()
        .await
        .unwrap();
    let config = ScheduleSyncConfig {
        countries: vec!["US".into()],
    };
    run_sync(&services, &config, &client, true).await.unwrap();
    run_sync(&services, &config, &client, true).await.unwrap();
    assert_eq!(upstream.broadcast.load(Ordering::SeqCst), 7);
    assert_eq!(upstream.web.load(Ordering::SeqCst), 7);
    let database = services.get_database().await.unwrap();
    let db = database.pool();
    assert_eq!(
        ScheduleCache::query(db.pool())
            .fetch_all()
            .await
            .unwrap()
            .len(),
        7,
        "broadcast/web duplicate episodes must collapse"
    );
    let states = ScheduleSyncState::query(db.pool())
        .fetch_all()
        .await
        .unwrap();
    assert!(synced_today(&states[0], Utc::now().date_naive()));
    let schema = services
        .get_graphql()
        .await
        .unwrap()
        .schema()
        .await
        .unwrap();
    mutate(&schema,"mutation($id:String!){result:updateScheduleSyncState(id:$id,input:{lastSyncedAt:\"2000-01-01T00:00:00Z\"}){success error}}",json!({"id":states[0].id})).await.unwrap();
    upstream.fail.store(true, Ordering::SeqCst);
    assert!(run_sync(&services, &config, &client, true).await.is_err());
    assert_eq!(
        ScheduleCache::query(db.pool())
            .fetch_all()
            .await
            .unwrap()
            .len(),
        7
    );
    let states = ScheduleSyncState::query(db.pool())
        .fetch_all()
        .await
        .unwrap();
    assert!(states[0].sync_error.is_some());
    assert!(!synced_today(&states[0], Utc::now().date_naive()));
    services.stop_all().await.unwrap();
    task.abort();
}

#[tokio::test]
async fn guide_episode_air_stamp_round_trips_and_sorts_without_changing_wanted() {
    let temp = tempfile::tempdir().unwrap();
    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: format!("sqlite://{}", temp.path().join("air-times.db").display()),
            connect_timeout: Duration::from_secs(5),
        })
        .start()
        .await
        .unwrap();
    let service = services.get_database().await.unwrap();
    let db = service.pool();
    let schema = build_schema(db.clone(), (), services.clone());
    let library=mutate(&schema,"mutation($input:CreateLibraryInput!){result:createLibrary(input:$input){success error library{id}}}",json!({"input":{"userId":"system-internal","name":"Fixture library","path":"/fixture","libraryType":"tv","autoScan":false,"autoOrganize":false,"namingPattern":"{title}","scanIntervalMinutes":60,"watchForChanges":false,"scanning":false}})).await.unwrap();
    let show=mutate(&schema,"mutation($input:CreateShowInput!){result:createShow(input:$input){success error show{id}}}",json!({"input":{"userId":"system-internal","libraryId":library["library"]["id"],"name":"Fixture show","genres":[],"autoDownload":false,"autoDownloadMode":"NONE"}})).await.unwrap();
    let created=mutate(&schema,"mutation($input:CreateEpisodeInput!){result:createEpisode(input:$input){success error episode{id}}}",json!({"input":{"showId":show["show"]["id"],"season":1,"episode":1,"wanted":true,"airDate":"2026-09-06"}})).await.unwrap();
    let id = created["episode"]["id"].as_str().unwrap();
    mutate(&schema,"mutation($id:String!){result:updateEpisode(id:$id,input:{airStamp:\"2026-09-06T20:00:00+02:00\"}){success error}}",json!({"id":id})).await.unwrap();
    let episode = Episode::get(db.pool(), &id.to_owned())
        .await
        .unwrap()
        .unwrap();
    assert!(episode.wanted);
    assert!(episode.air_stamp.is_some());
    let response = schema
        .execute(authenticated_request(
            "{episodes(orderBy:[{airStamp:ASC}]){edges{node{airStamp wanted}}}}",
            AuthUser {
                user_id: "system-internal".into(),
                email: None,
                role: Some("admin".into()),
            },
        ))
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(
        response.data.into_json().unwrap()["episodes"]["edges"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    services.stop_all().await.unwrap();
}
