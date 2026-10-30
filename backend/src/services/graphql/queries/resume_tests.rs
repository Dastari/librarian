use super::*;
use crate::graphql::auth::AuthUser;
use crate::services::graphql::schema::MutationRoot;
use crate::services::{AuthConfig, DatabaseServiceConfig, ServicesManager};
use async_graphql::{EmptySubscription, MergedObject, Request, Schema, Variables};
use graphql_orm::graphql::auth::AuthSubject;
use serde_json::{Value, json};
use std::sync::Arc;

type TestSchema = Schema<TestQueries, MutationRoot, EmptySubscription>;
#[derive(Default, MergedObject)]
struct TestQueries(
    ResumeQueries,
    ProbeQueries,
    PlaybackProgressQueries,
    NotificationQueries,
    AppLogQueries,
);
#[derive(Default)]
struct ProbeQueries;
#[Object]
impl ProbeQueries {
    async fn probe_shelf(&self, ctx: &Context<'_>, budget: i32, limit: i32) -> Result<ResumeShelf> {
        let user = ctx.require_member()?;
        let db = ctx.data::<Database>()?;
        let mut snapshot = MutationContext::begin_with_auth(db, TransactionMode::Default, None)
            .await
            .map_err(query_error)?;
        let result = collect_shelf(
            ctx,
            db,
            &mut snapshot,
            &user.user_id,
            None,
            limit as usize,
            budget as usize,
        )
        .await;
        snapshot.rollback().await.map_err(query_error)?;
        result
    }
}
fn schema(db: Database, services: Arc<ServicesManager>) -> TestSchema {
    Schema::build(
        TestQueries::default(),
        MutationRoot::default(),
        EmptySubscription,
    )
    .data(db)
    .data(services)
    .finish()
}
struct Fixture {
    _temp: tempfile::TempDir,
    db: Database,
    services: Arc<ServicesManager>,
    schema: TestSchema,
    owner: AuthUser,
    other: AuthUser,
}
impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let services = ServicesManager::builder()
            .add_service(DatabaseServiceConfig {
                database_url: format!("sqlite://{}", temp.path().join("resume.db").display()),
                connect_timeout: std::time::Duration::from_secs(5),
            })
            .add_service(AuthConfig::for_tests())
            .start()
            .await
            .unwrap();
        let db = services.get_database().await.unwrap().pool().clone();
        Self {
            schema: schema(db.clone(), services.clone()),
            db,
            services,
            _temp: temp,
            owner: AuthUser {
                user_id: uuid::Uuid::new_v4().to_string(),
                email: None,
                role: Some("member".into()),
            },
            other: AuthUser {
                user_id: uuid::Uuid::new_v4().to_string(),
                email: None,
                role: Some("member".into()),
            },
        }
    }
    async fn create(&self, entity: &str, input: Value) -> String {
        let node = format!("{}{}", entity[..1].to_ascii_lowercase(), &entity[1..]);
        let field = format!("create{entity}");
        let response=self.schema.execute(Request::new(format!("mutation($input:Create{entity}Input!) {{ {field}(input:$input) {{ success error {node} {{ id }} }} }}"))
            .variables(Variables::from_json(json!({"input":input})))
            .data(AuthSubject::from_parts("system", vec!["admin".into()], vec![], None))
            .data(AuthUser::system_admin())).await;
        assert!(response.errors.is_empty(), "{response:?}");
        let data = response.data.into_json().unwrap();
        assert_eq!(data[&field]["success"], true, "{data}");
        data[&field][&node]["id"].as_str().unwrap().into()
    }
    async fn library(&self, user: &AuthUser, name: &str) -> String {
        self.create("Library",json!({"userId":user.user_id,"name":name,"path":format!("/fixture/{name}"),"libraryType":"movies", "autoScan":false,"autoOrganize":false,"namingPattern":"","scanIntervalMinutes":60,"watchForChanges":false,"scanning":false})).await
    }
    async fn movie(&self, library: &str, user: &AuthUser, name: &str) -> String {
        self.create("Movie",json!({"libraryId":library,"userId":user.user_id,"title":name,"genres":[],"castNames":[],"productionCountries":[],"spokenLanguages":[],"monitored":false,"wanted":false,"hasFile":false})).await
    }
    async fn file(&self, library: Option<&str>, kind: &str, target: Option<&str>) -> String {
        let mut input = json!({"libraryId":library,"path":format!("/fixture/{}.mkv",uuid::Uuid::new_v4()),"size":1000,"isHdr":false,"addedAt":"2026-10-25T00:00:00Z"});
        if let Some(target) = target {
            input[kind] = json!(target);
        }
        self.create("MediaFile", input).await
    }
    async fn progress(&self, file: &str, user: &AuthUser, watched: bool) -> String {
        self.create("PlaybackProgress",json!({"userId":user.user_id,"mediaFileId":file,"currentPosition":50.0,"duration":100.0,"progressPercent":50.0,"isWatched":watched})).await
    }
    async fn query(&self, query: &str, user: Option<&AuthUser>) -> async_graphql::Response {
        let mut request = Request::new(query);
        if let Some(user) = user {
            request = request.data(user.clone()).data(AuthSubject::from_parts(
                user.user_id.clone(),
                user.role.iter().cloned().collect(),
                vec![],
                None,
            ));
        }
        self.schema.execute(request).await
    }
    async fn shelf(&self, args: &str, user: &AuthUser) -> Value {
        let response=self.query(&format!("{{ resumeShelf{args} {{ items {{ id userId mediaFileId mediaFile {{ id libraryId movie {{ id }} episode {{ id show {{ id }} }} track {{ id album {{ id }} }} chapter {{ id audiobook {{ id }} }} }} }} hasMore complete scannedCount error }} }}"),Some(user)).await;
        assert!(response.errors.is_empty(), "{response:?}");
        response.data.into_json().unwrap()["resumeShelf"].clone()
    }
}

#[tokio::test]
async fn scans_past_one_hundred_duplicate_title_checkpoints_and_returns_only_bounded_distinct_titles()
 {
    let f = Fixture::new().await;
    let library = f.library(&f.owner, "long-history").await;
    let movie = f.movie(&library, &f.owner, "Older distinct movie").await;
    let movie_file = f.file(Some(&library), "movieId", Some(&movie)).await;
    let older = f.progress(&movie_file, &f.owner, false).await;
    // The ORM generates whole-second timestamps. Cross that timestamp boundary
    // once so fixture ordering is deterministic without SQL/timestamp writes.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let show=f.create("Show",json!({"libraryId":library,"userId":f.owner.user_id,"name":"Repeated show","genres":[],"autoDownload":false,"autoDownloadMode":"NONE"})).await;
    let episode = f
        .create(
            "Episode",
            json!({"showId":show,"season":1,"episode":1,"title":"Episode","wanted":false}),
        )
        .await;
    let file = f.file(Some(&library), "episodeId", Some(&episode)).await;
    let mut checkpoints = Vec::new();
    for _ in 0..130 {
        checkpoints.push(f.progress(&file, &f.owner, false).await);
    }
    let shelf = f.shelf("(limit:2)", &f.owner).await;
    assert_eq!(shelf["items"].as_array().unwrap().len(), 2);
    assert!(shelf["scannedCount"].as_i64().unwrap() > 100);
    assert_eq!(
        shelf["items"][0]["mediaFile"]["episode"]["show"]["id"],
        show
    );
    assert_eq!(shelf["items"][1]["id"], older);
    assert_eq!(shelf["hasMore"], false);
    assert_eq!(shelf["complete"], true);
    // Exact timestamp + ID tie-breaking, independent of UUID generation order.
    let latest = read_progress_rows(&f, &checkpoints)
        .await
        .into_iter()
        .max_by(|a, b| {
            a.updated_at
                .cmp(&b.updated_at)
                .then_with(|| a.id.cmp(&b.id))
        })
        .unwrap();
    assert_eq!(shelf["items"][0]["id"], latest.id);
    f.services.stop_all().await.unwrap();
}
async fn read_progress_rows(f: &Fixture, wanted: &[String]) -> Vec<PlaybackProgress> {
    // Test inspection uses the existing typed entity layer; no application SQL.
    PlaybackProgress::query(f.db.pool())
        .filter(PlaybackProgressWhereInput {
            id: Some(ids(wanted.iter().cloned())),
            ..Default::default()
        })
        .fetch_all()
        .await
        .unwrap()
}

#[tokio::test]
async fn own_progress_cannot_restore_foreign_file_authority_and_admin_history_remains_personal() {
    let f = Fixture::new().await;
    let own = f.library(&f.owner, "own").await;
    let foreign = f.library(&f.other, "foreign").await;
    let movie = f.movie(&own, &f.owner, "Owned movie").await;
    let foreign_movie = f.movie(&foreign, &f.other, "Foreign movie").await;
    let own_file = f.file(Some(&own), "movieId", Some(&movie)).await;
    let foreign_file = f
        .file(Some(&foreign), "movieId", Some(&foreign_movie))
        .await;
    let included = f.progress(&own_file, &f.owner, false).await;
    f.progress(&foreign_file, &f.owner, false).await;
    f.progress(&own_file, &f.other, false).await;
    f.progress(&uuid::Uuid::new_v4().to_string(), &f.owner, false)
        .await;
    let orphan = f.file(Some(&own), "movieId", None).await;
    f.progress(&orphan, &f.owner, false).await;
    let unowned = f.file(None, "movieId", None).await;
    f.progress(&unowned, &f.owner, false).await;
    for caller in [
        f.owner.clone(),
        AuthUser {
            role: Some("admin".into()),
            ..f.owner.clone()
        },
    ] {
        let shelf = f.shelf("(limit:20)", &caller).await;
        assert_eq!(shelf["items"].as_array().unwrap().len(), 1);
        assert_eq!(shelf["items"][0]["id"], included);
        assert_eq!(shelf["items"][0]["userId"], f.owner.user_id);
        assert_eq!(shelf["hasMore"], false);
    }
    let global_admin = f.shelf("", &AuthUser::system_admin()).await;
    assert!(global_admin["items"].as_array().unwrap().is_empty());
    let denied = f
        .query(
            &format!("{{resumeShelf(libraryId:\"{foreign}\"){{complete}}}}"),
            Some(&f.owner),
        )
        .await;
    assert!(
        denied
            .errors
            .iter()
            .any(|e| e.message.contains("Library not found or access denied"))
    );
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn bounded_window_distinguishes_more_exhaustion_and_unproven_scan_limit() {
    let f = Fixture::new().await;
    let library = f.library(&f.owner, "window").await;
    for name in ["one", "two", "three"] {
        let movie = f.movie(&library, &f.owner, name).await;
        let file = f.file(Some(&library), "movieId", Some(&movie)).await;
        f.progress(&file, &f.owner, false).await;
    }
    let shelf = f.shelf("(limit:2)", &f.owner).await;
    assert_eq!(shelf["items"].as_array().unwrap().len(), 2);
    assert_eq!(shelf["hasMore"], true);
    assert_eq!(shelf["complete"], true);
    let probe = f
        .query(
            "{probeShelf(budget:1,limit:2){items{id} hasMore complete error}}",
            Some(&f.owner),
        )
        .await;
    assert!(probe.errors.is_empty(), "{probe:?}");
    let data = probe.data.into_json().unwrap();
    assert_eq!(data["probeShelf"]["complete"], false);
    assert!(data["probeShelf"]["hasMore"].is_null());
    assert!(data["probeShelf"]["items"].as_array().unwrap().is_empty());
    assert!(
        data["probeShelf"]["error"]
            .as_str()
            .unwrap()
            .contains("scan limit")
    );
    for limit in [0, 101] {
        assert!(
            !f.query(
                &format!("{{resumeShelf(limit:{limit}){{complete}}}}"),
                Some(&f.owner)
            )
            .await
            .errors
            .is_empty()
        );
    }
    assert!(
        !f.query("{resumeShelf{complete}}", None)
            .await
            .errors
            .is_empty()
    );
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn current_completed_checkpoint_suppresses_stale_resume_and_library_scope_is_exact() {
    let f = Fixture::new().await;
    let library = f.library(&f.owner, "completion").await;
    let other_library = f.library(&f.owner, "second-owned-library").await;
    let movie = f.movie(&library, &f.owner, "Finished movie").await;
    let file = f.file(Some(&library), "movieId", Some(&movie)).await;
    f.progress(&file, &f.owner, false).await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    f.progress(&file, &f.owner, true).await;
    let other = f.movie(&other_library, &f.owner, "Still active").await;
    let other_file = f.file(Some(&other_library), "movieId", Some(&other)).await;
    let active = f.progress(&other_file, &f.owner, false).await;
    let shelf = f.shelf("", &f.owner).await;
    assert_eq!(shelf["items"].as_array().unwrap().len(), 1);
    assert_eq!(shelf["items"][0]["id"], active);
    let first = f
        .shelf(&format!("(libraryId:\"{library}\")"), &f.owner)
        .await;
    assert!(first["items"].as_array().unwrap().is_empty());
    assert_eq!(first["complete"], true);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn album_and_audiobook_use_logical_parent_identity_with_existing_nested_entities() {
    let f = Fixture::new().await;
    let library = f.library(&f.owner, "audio").await;
    let artist = f
        .create(
            "Artist",
            json!({"libraryId":library,"userId":f.owner.user_id,"name":"Artist"}),
        )
        .await;
    let album=f.create("Album",json!({"libraryId":library,"userId":f.owner.user_id,"artistId":artist,"name":"Album","genres":[],"autoDownload":false,"autoDownloadMode":"NONE","hasFiles":false})).await;
    let book=f.create("Audiobook",json!({"libraryId":library,"userId":f.owner.user_id,"title":"Book","narrators":[],"autoDownload":false,"autoDownloadMode":"NONE","hasFiles":false})).await;
    for number in [1, 2] {
        let track=f.create("Track",json!({"libraryId":library,"albumId":album,"title":format!("Track {number}"),"trackNumber":number,"explicit":false,"wanted":false})).await;
        let track_file = f.file(Some(&library), "trackId", Some(&track)).await;
        f.progress(&track_file, &f.owner, false).await;
        let chapter=f.create("Chapter",json!({"audiobookId":book,"chapterNumber":number,"title":format!("Chapter {number}"),"startTimeSecs":0.0,"wanted":false})).await;
        let chapter_file = f.file(Some(&library), "chapterId", Some(&chapter)).await;
        f.progress(&chapter_file, &f.owner, false).await;
    }
    let shelf = f.shelf("", &f.owner).await;
    assert_eq!(shelf["items"].as_array().unwrap().len(), 2);
    let files: Vec<_> = shelf["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| &row["mediaFile"])
        .collect();
    assert_eq!(
        files
            .iter()
            .filter(|file| file["track"]["album"]["id"] == album)
            .count(),
        1
    );
    assert_eq!(
        files
            .iter()
            .filter(|file| file["chapter"]["audiobook"]["id"] == book)
            .count(),
        1
    );
    f.services.stop_all().await.unwrap();
}

#[test]
fn selected_keysets_keep_unique_timestamp_and_id_tie_breakers() {
    use graphql_orm::graphql::orm::AuthorizedKeysetEntity;
    assert!(PlaybackProgress::KEYSET_FINGERPRINT.contains("updated_at:desc"));
    assert!(Notification::KEYSET_FINGERPRINT.contains("created_at:desc"));
    assert!(AppLog::KEYSET_FINGERPRINT.contains("timestamp:desc"));
    for order in [
        PlaybackProgress::KEYSET_FINGERPRINT,
        Notification::KEYSET_FINGERPRINT,
        AppLog::KEYSET_FINGERPRINT,
    ] {
        assert!(order.contains("id:desc"));
    }
}

struct CallbackOnly;
impl graphql_orm::graphql::orm::RowPolicy for CallbackOnly {
    fn can_read_row<'a>(
        &'a self,
        _: Option<&'a Context<'_>>,
        _: &'a Database,
        _: &'static str,
        _: Option<&'static str>,
        _: EntityAccessSurface,
        _: &'a (dyn std::any::Any + Send + Sync),
    ) -> futures::future::BoxFuture<'a, Result<bool>> {
        Box::pin(async { Ok(true) })
    }
    fn can_write_row<'a>(
        &'a self,
        _: Option<&'a Context<'_>>,
        _: &'a Database,
        _: &'static str,
        _: Option<&'static str>,
        _: EntityAccessSurface,
        _: &'a (dyn std::any::Any + Send + Sync),
    ) -> futures::future::BoxFuture<'a, Result<bool>> {
        Box::pin(async { Ok(true) })
    }
}
struct HideMovieLink;
impl graphql_orm::graphql::orm::FieldPolicy for HideMovieLink {
    fn can_read_field<'a>(
        &'a self,
        _: &'a Context<'_>,
        _: &'a Database,
        entity: &'static str,
        field: &'static str,
        _: Option<&'static str>,
        _: Option<&'a (dyn std::any::Any + Send + Sync)>,
    ) -> futures::future::BoxFuture<'a, Result<bool>> {
        Box::pin(async move { Ok(!(entity == "MediaFile" && field == "movieId")) })
    }
    fn can_write_field<'a>(
        &'a self,
        _: &'a Context<'_>,
        _: &'a Database,
        _: &'static str,
        _: &'static str,
        _: Option<&'static str>,
        _: Option<&'a (dyn std::any::Any + Send + Sync)>,
        _: Option<&'a (dyn std::any::Any + Send + Sync)>,
    ) -> futures::future::BoxFuture<'a, Result<bool>> {
        Box::pin(async { Ok(true) })
    }
}

#[tokio::test]
async fn resume_requires_complete_sql_visibility_and_checks_consumed_field_access() {
    let f = Fixture::new().await;
    let library = f.library(&f.owner, "visibility").await;
    let movie = f.movie(&library, &f.owner, "Restricted field").await;
    let file = f.file(Some(&library), "movieId", Some(&movie)).await;
    f.progress(&file, &f.owner, false).await;
    let mut callback_db = f.db.clone();
    callback_db.set_row_policy(CallbackOnly);
    let callback = schema(callback_db, f.services.clone())
        .execute(Request::new("{resumeShelf{complete}}").data(f.owner.clone()))
        .await;
    assert!(
        callback
            .errors
            .iter()
            .any(|e| e.message.contains("complete database read visibility")),
        "{callback:?}"
    );
    let mut field_db = f.db.clone();
    field_db.set_field_policy(HideMovieLink);
    let denied = schema(field_db, f.services.clone())
        .execute(Request::new("{resumeShelf{items{id}complete}}").data(f.owner.clone()))
        .await;
    assert!(
        !denied.errors.is_empty(),
        "hidden grouping fields must not silently influence public membership"
    );
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn generated_keysets_keep_owner_counts_and_timestamp_id_order_with_admin_only_logs() {
    let f = Fixture::new().await;
    let mut own = Vec::new();
    for number in 0..3 {
        own.push(f.create("Notification",json!({"userId":f.owner.user_id,"notificationType":"info","category":"test","title":format!("Own {number}"),"message":"fixture"})).await);
        f.create("Notification",json!({"userId":f.other.user_id,"notificationType":"info","category":"test","title":format!("Foreign {number}"),"message":"fixture"})).await;
    }
    let response=f.query("{notificationsKeyset(page:{limit:1,includeTotalCount:true}){edges{node{id userId}}pageInfo{totalCount hasNextPage endCursor}}}",Some(&f.owner)).await;
    assert!(response.errors.is_empty(), "{response:?}");
    let data = response.data.into_json().unwrap();
    assert_eq!(data["notificationsKeyset"]["pageInfo"]["totalCount"], 3);
    assert_eq!(
        data["notificationsKeyset"]["edges"][0]["node"]["userId"],
        f.owner.user_id
    );
    let cursor = data["notificationsKeyset"]["pageInfo"]["endCursor"]
        .as_str()
        .unwrap();
    let continued=f.query(&format!("{{notificationsKeyset(page:{{limit:5,after:\"{cursor}\",includeTotalCount:true}}){{edges{{node{{id userId}}}}pageInfo{{totalCount hasNextPage}}}}}}"),Some(&f.owner)).await;
    assert!(continued.errors.is_empty(), "{continued:?}");
    let page = continued.data.into_json().unwrap();
    assert_eq!(
        page["notificationsKeyset"]["edges"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(page["notificationsKeyset"]["pageInfo"]["totalCount"], 3);
    assert_eq!(
        page["notificationsKeyset"]["pageInfo"]["hasNextPage"],
        false
    );
    let mut logs = Vec::new();
    for number in 0..3 {
        logs.push(f.create("AppLog",json!({"timestamp":"2026-10-25T00:00:00Z","level":"INFO","target":"resume-keyset-fixture","message":format!("Log {number}")})).await);
    }
    let query = "{appLogsKeyset(where:{target:{eq:\"resume-keyset-fixture\"}},page:{limit:3,includeTotalCount:true}){edges{node{id}}pageInfo{totalCount}}}";
    assert!(!f.query(query, Some(&f.owner)).await.errors.is_empty());
    let admin = f.query(query, Some(&AuthUser::system_admin())).await;
    assert!(admin.errors.is_empty(), "{admin:?}");
    let data = admin.data.into_json().unwrap();
    logs.sort_by(|a, b| b.cmp(a));
    assert_eq!(
        data["appLogsKeyset"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| edge["node"]["id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>(),
        logs
    );
    assert_eq!(data["appLogsKeyset"]["pageInfo"]["totalCount"], 3);
    f.services.stop_all().await.unwrap();
}
