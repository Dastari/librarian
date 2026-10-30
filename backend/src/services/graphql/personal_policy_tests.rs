//! Source-only personal authorization and transaction regression tests.
//! Runtime validation is owned by the parent task.

use super::*;
use async_graphql::{MergedObject, MergedSubscription, Request, Schema, Variables};
use futures::{FutureExt, StreamExt};
use graphql_orm::graphql::auth::AuthSubject;
use graphql_orm::graphql::orm::{Entity, SchemaModel, TransactionMode};
use graphql_orm::graphql::subscriptions::SubscriptionEvent;
use serde_json::{Value, json};

use crate::services::database::DatabaseService;
use crate::services::graphql::entities::torrent::authorization_tests::{account, fixture};

#[derive(Default, MergedObject)]
struct Queries(
    PersonalListQueries,
    ListEntryQueries,
    SavedViewQueries,
    HomeShelfQueries,
    LibraryPreferenceQueries,
    MovieQueries,
);
#[derive(Default, MergedObject)]
struct Mutations(
    PersonalListMutations,
    ListEntryMutations,
    SavedViewMutations,
    HomeShelfMutations,
    LibraryPreferenceMutations,
    LibraryMutations,
    MovieMutations,
    ArtistMutations,
    AlbumMutations,
    AudiobookMutations,
    ShowMutations,
    TrackMutations,
);
#[derive(Default, MergedSubscription)]
struct Subscriptions(
    PersonalListSubscriptions,
    ListEntrySubscriptions,
    SavedViewSubscriptions,
    HomeShelfSubscriptions,
    LibraryPreferenceSubscriptions,
);
type TestSchema = Schema<Queries, Mutations, Subscriptions>;

struct Fixture {
    db: Database,
    schema: TestSchema,
    user: AuthUser,
    other: AuthUser,
    admin: AuthUser,
}

impl Fixture {
    async fn new() -> Self {
        let db = DatabaseService::new(fixture().await).pool().clone();
        let user = account(&db, "member").await;
        let other = account(&db, "member").await;
        let admin = account(&db, "admin").await;
        let schema = Schema::build(
            Queries::default(),
            Mutations::default(),
            Subscriptions::default(),
        )
        .data(db.clone())
        .finish();
        Self {
            db,
            schema,
            user,
            other,
            admin,
        }
    }
    fn request(&self, document: &str, variables: Value, user: &AuthUser) -> Request {
        Request::new(document)
            .variables(Variables::from_json(variables))
            .data(user.clone())
            .data(user.user_id.clone())
            .data(AuthSubject::from_parts(
                user.user_id.clone(),
                vec![user.role.clone().unwrap()],
                vec![],
                None,
            ))
    }
    async fn execute(
        &self,
        document: &str,
        variables: Value,
        user: &AuthUser,
    ) -> async_graphql::Response {
        self.schema
            .execute(self.request(document, variables, user))
            .await
    }
    async fn create(&self, entity: &str, input: Value, user: &AuthUser) -> String {
        let node = format!("{}{}", entity[..1].to_ascii_lowercase(), &entity[1..]);
        let field = format!("create{entity}");
        let response = self.execute(&format!("mutation($input:Create{entity}Input!){{{field}(input:$input){{success error {node}{{id}}}}}}"), json!({"input":input}), user).await;
        assert!(response.errors.is_empty(), "{response:?}");
        let data = response.data.into_json().unwrap();
        assert_eq!(data[&field]["success"], true, "{data}");
        data[&field][&node]["id"].as_str().unwrap().into()
    }
    async fn list(&self, user: &AuthUser, name: &str, kind: &str) -> String {
        self.create(
            "PersonalList",
            json!({"userId":user.user_id,"name":name,"kind":kind}),
            user,
        )
        .await
    }
    async fn library(&self, user: &AuthUser, name: &str) -> String {
        // General catalogue administration has its existing explicit admin
        // scope. The personal policy must never inherit that override.
        self.create("Library", json!({"userId":user.user_id,"name":name,"path":format!("/personal-fixture/{name}"),"libraryType":"movies","autoScan":false,"autoOrganize":false,"namingPattern":"","scanIntervalMinutes":60,"watchForChanges":false,"scanning":false}), &self.admin).await
    }
    async fn movie(&self, user: &AuthUser, library: &str, title: &str) -> String {
        self.create("Movie", json!({"userId":user.user_id,"libraryId":library,"title":title,"genres":[],"castNames":[],"productionCountries":[],"spokenLanguages":[],"monitored":false,"wanted":false,"hasFile":false}), &self.admin).await
    }
    async fn saved_view(&self, user: &AuthUser, library: Option<&str>) -> String {
        self.create("SavedView", json!({"userId":user.user_id,"name":"View","version":1,"contentType":"MOVIE","libraryId":library,"availability":"ANY","sort":"TITLE","descending":false}), user).await
    }
}

fn rejected(response: async_graphql::Response, field: &str) {
    if !response.errors.is_empty() {
        return;
    }
    let data = response.data.into_json().unwrap();
    assert_eq!(data[field]["success"], false, "{data}");
    assert!(data[field]["error"].is_string(), "{data}");
}

fn entry(user: &AuthUser, list: &str, movie: &str, position: i32) -> CreateListEntryInput {
    CreateListEntryInput {
        user_id: user.user_id.clone(),
        list_id: list.into(),
        position,
        movie_id: Some(movie.into()),
        show_id: None,
        album_id: None,
        audiobook_id: None,
        track_id: None,
    }
}

#[tokio::test]
async fn generated_reads_scope_admins_members_counts_offsets_nested_rows_and_single_ids() {
    let f = Fixture::new().await;
    // Alphabetically earlier foreign rows must not consume the caller's page.
    for name in ["A foreign", "B foreign", "C foreign"] {
        f.list(&f.other, name, "MANUAL").await;
    }
    let own = f.list(&f.user, "Z own", "MANUAL").await;
    let admin = f.list(&f.admin, "Z admin", "MANUAL").await;
    for (user, expected) in [(&f.user, &own), (&f.admin, &admin)] {
        let response = f.execute("{personalLists(orderBy:[{name:ASC}],page:{limit:1,offset:0}){edges{node{id userId entries{pageInfo{totalCount}}}} pageInfo{totalCount}}}", json!({}), user).await;
        assert!(response.errors.is_empty(), "{response:?}");
        let data = response.data.into_json().unwrap();
        assert_eq!(data["personalLists"]["pageInfo"]["totalCount"], 1);
        assert_eq!(data["personalLists"]["edges"][0]["node"]["id"], *expected);
        assert_eq!(
            data["personalLists"]["edges"][0]["node"]["entries"]["pageInfo"]["totalCount"],
            0
        );
    }
    let response = f
        .execute(
            "query($id:String!){personalList(id:$id){id}}",
            json!({"id":own}),
            &f.admin,
        )
        .await;
    assert!(response.errors.is_empty(), "{response:?}");
    assert!(response.data.into_json().unwrap()["personalList"].is_null());
    let response = f
        .execute(
            "mutation($id:String!){deletePersonalList(id:$id){success error}}",
            json!({"id":own}),
            &f.admin,
        )
        .await;
    rejected(response, "deletePersonalList");
    assert!(
        PersonalList::get(f.db.pool(), &own)
            .await
            .unwrap()
            .is_some()
    );
    let response = f
        .schema
        .execute("{personalLists{pageInfo{totalCount}}}")
        .await;
    assert!(!response.errors.is_empty());
}

#[tokio::test]
async fn public_ownership_and_nullable_foreign_links_fail_closed_for_admins_too() {
    let f = Fixture::new().await;
    let foreign_list = f.list(&f.other, "Foreign", "MANUAL").await;
    let library = f.library(&f.user, "own").await;
    let foreign_library = f.library(&f.other, "foreign").await;
    let movie = f.movie(&f.user, &library, "Own").await;
    let foreign_movie = f.movie(&f.other, &foreign_library, "Foreign").await;
    let own_list = f.list(&f.user, "Own", "MANUAL").await;
    for user in [&f.user, &f.admin] {
        for (list, target) in [(&foreign_list, &movie), (&own_list, &foreign_movie)] {
            let response = f.execute("mutation($input:CreateListEntryInput!){createListEntry(input:$input){success error listEntry{id}}}", json!({"input":{"userId":user.user_id,"listId":list,"movieId":target,"position":0}}), user).await;
            rejected(response, "createListEntry");
        }
        let response = f.execute("mutation($input:CreateHomeShelfInput!){createHomeShelf(input:$input){success error homeShelf{id}}}", json!({"input":{"userId":user.user_id,"shelfKey":"pinned","position":0,"visible":true,"listId":foreign_list}}), user).await;
        rejected(response, "createHomeShelf");
        let response = f.execute("mutation($input:CreateLibraryPreferenceInput!){createLibraryPreference(input:$input){success error libraryPreference{id}}}", json!({"input":{"userId":user.user_id,"libraryId":foreign_library,"favourite":true,"position":0}}), user).await;
        rejected(response, "createLibraryPreference");
    }
    // Supplied ownership may be replaced by the create transform or rejected by
    // field authorization; either outcome must never produce a foreign row.
    let response = f.execute("mutation($input:CreatePersonalListInput!){createPersonalList(input:$input){success error personalList{userId}}}", json!({"input":{"userId":f.other.user_id,"name":"Forged owner","kind":"MANUAL"}}), &f.admin).await;
    if response.errors.is_empty() {
        let data = response.data.into_json().unwrap();
        if data["createPersonalList"]["success"] == true {
            assert_eq!(
                data["createPersonalList"]["personalList"]["userId"],
                f.admin.user_id
            );
        }
    }
    assert!(
        PersonalList::query(f.db.pool())
            .filter(PersonalListWhereInput {
                user_id: Some(eq(&f.other.user_id)),
                name: Some(eq("Forged owner")),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn uniqueness_typed_targets_and_immutable_membership_preserve_watch_later_intent() {
    let f = Fixture::new().await;
    let library = f.library(&f.user, "movie-library").await;
    let movie = f.movie(&f.user, &library, "A film").await;
    let list = f.list(&f.user, WATCH_LATER_NAME, "WATCH_LATER").await;
    let other_list = f.list(&f.user, "Other", "MANUAL").await;
    let first = ListEntry::insert(&f.db, entry(&f.user, &list, &movie, 20))
        .await
        .unwrap();
    assert!(
        ListEntry::insert(&f.db, entry(&f.user, &list, &movie, 30))
            .await
            .is_err()
    );
    let no_target = CreateListEntryInput {
        movie_id: None,
        ..entry(&f.user, &list, &movie, 0)
    };
    assert!(ListEntry::insert(&f.db, no_target).await.is_err());
    // A second typed reference cannot become a generic unvalidated content ID.
    let two_targets = CreateListEntryInput {
        show_id: Some("missing".into()),
        ..entry(&f.user, &other_list, &movie, 0)
    };
    assert!(ListEntry::insert(&f.db, two_targets).await.is_err());
    let response = f.execute("mutation($input:CreatePersonalListInput!){createPersonalList(input:$input){success error personalList{id}}}", json!({"input":{"userId":f.user.user_id,"name":WATCH_LATER_NAME,"kind":"WATCH_LATER"}}), &f.user).await;
    rejected(response, "createPersonalList");
    for input in [
        json!({"listId":other_list}),
        json!({"movieId":null}),
        json!({"userId":f.other.user_id}),
    ] {
        let response = f.execute("mutation($id:String!,$input:UpdateListEntryInput!){updateListEntry(id:$id,input:$input){success error listEntry{id}}}", json!({"id":first.id,"input":input}), &f.user).await;
        rejected(response, "updateListEntry");
    }
    let response = f.execute("mutation($id:String!){updateListEntry(id:$id,input:{position:5}){success error listEntry{position}}}", json!({"id":first.id}), &f.user).await;
    assert!(response.errors.is_empty(), "{response:?}");
    assert_eq!(
        response.data.into_json().unwrap()["updateListEntry"]["success"],
        true
    );
    let row = ListEntry::get(f.db.pool(), &first.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (row.list_id.as_str(), row.movie_id.as_deref(), row.position),
        (list.as_str(), Some(movie.as_str()), 5)
    );
    let persisted = Movie::get(f.db.pool(), &movie).await.unwrap().unwrap();
    assert!(!persisted.wanted && !persisted.monitored && !persisted.has_file);
    assert!(persisted.media_file_id.is_none());
    assert!(
        Torrent::query(f.db.pool())
            .fetch_all()
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        PendingFileMatch::query(f.db.pool())
            .fetch_all()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn rejected_transaction_and_foreign_links_emit_nothing_and_touch_no_acquisition_rows() {
    let f = Fixture::new().await;
    let library = f.library(&f.user, "rollback-library").await;
    let movie = f.movie(&f.user, &library, "Rollback film").await;
    let list = f.list(&f.user, "Rollback list", "MANUAL").await;
    let mut entries =
        f.db.ensure_event_sender::<SubscriptionEvent<ListEntryChangedEvent>>()
            .subscribe();
    let mut lists =
        f.db.ensure_event_sender::<SubscriptionEvent<PersonalListChangedEvent>>()
            .subscribe();
    let mut movies =
        f.db.ensure_event_sender::<SubscriptionEvent<MovieChangedEvent>>()
            .subscribe();
    let mut libraries =
        f.db.ensure_event_sender::<SubscriptionEvent<LibraryChangedEvent>>()
            .subscribe();
    let mut tx = MutationContext::begin_with_auth(&f.db, TransactionMode::Default, None)
        .await
        .unwrap();
    let inserted = tx
        .insert::<ListEntry>(entry(&f.user, &list, &movie, 0))
        .await
        .unwrap();
    let bad = CreateListEntryInput {
        movie_id: Some("missing-title".into()),
        ..entry(&f.user, &list, &movie, 1)
    };
    assert!(tx.insert::<ListEntry>(bad).await.is_err());
    tx.rollback().await.unwrap();
    assert!(
        ListEntry::get(f.db.pool(), &inserted.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(entries.try_recv().is_err());
    assert!(lists.try_recv().is_err());
    assert!(movies.try_recv().is_err());
    assert!(libraries.try_recv().is_err());
    let inserted = ListEntry::insert(&f.db, entry(&f.user, &list, &movie, 0))
        .await
        .unwrap();
    assert!(entries.try_recv().is_ok());
    assert!(lists.try_recv().is_ok());
    assert!(movies.try_recv().is_err());
    assert!(libraries.try_recv().is_err());
    while lists.try_recv().is_ok() {}
    ListEntry::delete_by_id(&f.db, &inserted.id).await.unwrap();
    assert!(entries.try_recv().is_ok());
    assert!(
        lists.try_recv().is_ok(),
        "surviving parent needs an invalidation after entry deletion"
    );
    assert!(movies.try_recv().is_err());
    assert!(libraries.try_recv().is_err());
}

#[tokio::test]
async fn smart_views_shelf_keys_preferences_and_cascades_preserve_scope_and_order() {
    let f = Fixture::new().await;
    let library = f.library(&f.user, "smart-library").await;
    let view = f.saved_view(&f.user, Some(&library)).await;
    let list = f
        .create(
            "PersonalList",
            json!({"userId":f.user.user_id,"name":"Smart","kind":"SMART","savedViewId":view}),
            &f.user,
        )
        .await;
    let shelf = f.create("HomeShelf", json!({"userId":f.user.user_id,"shelfKey":format!("list:{list}"),"position":1000,"visible":true,"listId":list}), &f.user).await;
    f.create(
        "HomeShelf",
        json!({"userId":f.user.user_id,"shelfKey":"continue","position":0,"visible":true}),
        &f.user,
    )
    .await;
    let preference = f
        .create(
            "LibraryPreference",
            json!({"userId":f.user.user_id,"libraryId":library,"favourite":true,"position":0}),
            &f.user,
        )
        .await;
    let duplicate = f.execute("mutation($input:CreateLibraryPreferenceInput!){createLibraryPreference(input:$input){success error libraryPreference{id}}}", json!({"input":{"userId":f.user.user_id,"libraryId":library,"favourite":true,"position":5}}), &f.user).await;
    rejected(duplicate, "createLibraryPreference");
    let duplicate = f.execute("mutation($input:CreateHomeShelfInput!){createHomeShelf(input:$input){success error homeShelf{id}}}", json!({"input":{"userId":f.user.user_id,"shelfKey":"continue","position":5,"visible":false}}), &f.user).await;
    rejected(duplicate, "createHomeShelf");
    let movie = f.movie(&f.user, &library, "Smart target").await;
    assert!(
        ListEntry::insert(&f.db, entry(&f.user, &list, &movie, 0))
            .await
            .is_err()
    );
    for input in [
        json!({"version":2}),
        json!({"contentType":"SHOW","availability":"AVAILABLE"}),
    ] {
        let response = f.execute("mutation($id:String!,$input:UpdateSavedViewInput!){updateSavedView(id:$id,input:$input){success error savedView{id}}}", json!({"id":view,"input":input}), &f.user).await;
        rejected(response, "updateSavedView");
    }
    assert_eq!(
        SavedView::get(f.db.pool(), &view)
            .await
            .unwrap()
            .unwrap()
            .version,
        1
    );
    Library::delete_by_id(&f.db, &library).await.unwrap();
    assert!(SavedView::get(f.db.pool(), &view).await.unwrap().is_none());
    assert!(
        PersonalList::get(f.db.pool(), &list)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        LibraryPreference::get(f.db.pool(), &preference)
            .await
            .unwrap()
            .is_none()
    );
    let shelf = HomeShelf::get(f.db.pool(), &shelf).await.unwrap().unwrap();
    assert!(!shelf.visible && shelf.list_id.is_none());
    assert_eq!(shelf.position, 1000);
}

#[tokio::test]
async fn generated_subscriptions_hide_foreign_rows_even_from_admin_and_deliver_own_delete() {
    let f = Fixture::new().await;
    let own = f.list(&f.admin, "Admin personal", "MANUAL").await;
    let foreign = f.list(&f.user, "Member personal", "MANUAL").await;
    let mut stream = Box::pin(f.schema.execute_stream(f.request(
        "subscription{personalListChanged{id action}}",
        json!({}),
        &f.admin,
    )));
    assert!(stream.next().now_or_never().is_none());
    PersonalList::update_by_id(
        &f.db,
        &foreign,
        UpdatePersonalListInput {
            name: Some("Foreign updated".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), stream.next())
            .await
            .is_err(),
        "admin must not receive foreign personal invalidations"
    );
    // A single poll can suspend event authorization during its database read.
    // Keep polling delivery alongside the delete so the subscription can release
    // the fixture's sole connection before the mutation acquires it.
    let (deleted, response) = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        tokio::join!(PersonalList::delete_by_id(&f.db, &own), stream.next())
    })
    .await
    .expect("personal delete and subscription delivery timed out");
    assert!(deleted.unwrap());
    let response = response.expect("personal subscription ended before delete delivery");
    assert!(response.errors.is_empty(), "{response:?}");
    let data = response.data.into_json().unwrap();
    assert_eq!(data["personalListChanged"]["id"], own);
    assert_eq!(data["personalListChanged"]["action"], "DELETED");
}

#[test]
fn personal_schema_declares_database_uniqueness_and_only_bounded_parent_propagation() {
    let model = SchemaModel::from_entities(&[
        PersonalList::metadata(),
        ListEntry::metadata(),
        SavedView::metadata(),
        HomeShelf::metadata(),
        LibraryPreference::metadata(),
    ]);
    for (table, columns) in [
        ("personal_lists", vec!["user_id", "name"]),
        ("list_entries", vec!["list_id", "movie_id"]),
        ("list_entries", vec!["list_id", "track_id"]),
        ("home_shelves", vec!["user_id", "shelf_key"]),
        ("library_preferences", vec!["user_id", "library_id"]),
    ] {
        assert!(
            model
                .tables
                .iter()
                .find(|t| t.table_name == table)
                .unwrap()
                .indexes
                .iter()
                .any(|index| index.is_unique && index.columns == columns),
            "missing unique index on {table} {columns:?}"
        );
    }
}

#[tokio::test]
async fn manual_playlists_support_each_typed_title_and_track_and_reject_forged_library_links() {
    let f = Fixture::new().await;
    let library = f.library(&f.user, "mixed-library").await;
    let foreign_library = f.library(&f.other, "foreign-mixed-library").await;
    let list = f.list(&f.user, "Mixed playlist", "MANUAL").await;
    let movie = f.movie(&f.user, &library, "Film").await;
    let show = f.create("Show", json!({"userId":f.user.user_id,"libraryId":library,"name":"Series","genres":[],"autoDownload":false,"autoDownloadMode":"NONE"}), &f.admin).await;
    let artist = f
        .create(
            "Artist",
            json!({"userId":f.user.user_id,"libraryId":library,"name":"Performer"}),
            &f.admin,
        )
        .await;
    let album = f.create("Album", json!({"userId":f.user.user_id,"libraryId":library,"artistId":artist,"name":"Record","genres":[],"autoDownload":false,"autoDownloadMode":"NONE","hasFiles":false}), &f.admin).await;
    let audiobook = f.create("Audiobook", json!({"userId":f.user.user_id,"libraryId":library,"title":"Book","narrators":[],"autoDownload":false,"autoDownloadMode":"NONE","hasFiles":false}), &f.admin).await;
    let track = f.create("Track", json!({"libraryId":library,"albumId":album,"title":"Song","trackNumber":1,"explicit":false,"wanted":false}), &f.admin).await;
    for (position, (field, target)) in [
        ("movieId", &movie),
        ("showId", &show),
        ("albumId", &album),
        ("audiobookId", &audiobook),
        ("trackId", &track),
    ]
    .into_iter()
    .enumerate()
    {
        let mut input = json!({"userId":f.user.user_id,"listId":list,"position":position});
        input[field] = json!(target);
        f.create("ListEntry", input, &f.user).await;
    }
    let response = f.execute("query($id:String!){personalList(id:$id){entries(orderBy:{position:ASC,id:ASC}){edges{node{position movie{id} show{id} album{id} audiobook{id} track{id}}} pageInfo{totalCount}}}}", json!({"id":list}), &f.user).await;
    assert!(response.errors.is_empty(), "{response:?}");
    let data = response.data.into_json().unwrap();
    assert_eq!(data["personalList"]["entries"]["pageInfo"]["totalCount"], 5);
    for (index, field) in ["movie", "show", "album", "audiobook", "track"]
        .into_iter()
        .enumerate()
    {
        let node = &data["personalList"]["entries"]["edges"][index]["node"];
        assert_eq!(node["position"], index);
        assert!(node[field]["id"].is_string(), "{node}");
    }
    // General admin fixtures can expose inconsistent redundant title ownership.
    // Personal membership must validate the actual library as well as user_id.
    let forged = f
        .movie(&f.user, &foreign_library, "Forged redundant owner")
        .await;
    assert!(
        ListEntry::insert(&f.db, entry(&f.user, &list, &forged, 10))
            .await
            .is_err()
    );
    Track::delete_by_id(&f.db, &track).await.unwrap();
    let rows = ListEntry::query(f.db.pool())
        .filter(ListEntryWhereInput {
            list_id: Some(eq(&list)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .unwrap();
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|row| row.track_id.is_none()));
}

#[tokio::test]
async fn all_personal_roots_keep_admin_counts_private_and_smart_rules_cannot_cross_owners() {
    let f = Fixture::new().await;
    for user in [&f.user, &f.other, &f.admin] {
        let library = f.library(user, &user.user_id).await;
        f.saved_view(user, Some(&library)).await;
        f.create("HomeShelf", json!({"userId":user.user_id,"shelfKey":"libraries","position":10,"visible":true,"libraryId":library}), user).await;
        f.create(
            "LibraryPreference",
            json!({"userId":user.user_id,"libraryId":library,"position":0,"favourite":true}),
            user,
        )
        .await;
    }
    let response = f.execute("{savedViews{edges{node{userId}}pageInfo{totalCount}} homeShelves{edges{node{userId library{id userId}}}pageInfo{totalCount}} libraryPreferences{edges{node{userId}}pageInfo{totalCount}}}", json!({}), &f.admin).await;
    assert!(response.errors.is_empty(), "{response:?}");
    let data = response.data.into_json().unwrap();
    for root in ["savedViews", "homeShelves", "libraryPreferences"] {
        assert_eq!(data[root]["pageInfo"]["totalCount"], 1);
        assert_eq!(data[root]["edges"][0]["node"]["userId"], f.admin.user_id);
    }
    let foreign = f.saved_view(&f.other, None).await;
    let response = f.execute("mutation($input:CreatePersonalListInput!){createPersonalList(input:$input){success error personalList{id}}}", json!({"input":{"userId":f.admin.user_id,"name":"Foreign rule","kind":"SMART","savedViewId":foreign}}), &f.admin).await;
    rejected(response, "createPersonalList");
    for (name, kind, view) in [
        ("Renamed Watch Later", "WATCH_LATER", None),
        (WATCH_LATER_NAME, "MANUAL", None),
        ("Missing rule", "SMART", None),
        ("Manual rule", "MANUAL", Some(foreign.as_str())),
    ] {
        let response = f.execute("mutation($input:CreatePersonalListInput!){createPersonalList(input:$input){success error personalList{id}}}", json!({"input":{"userId":f.user.user_id,"name":name,"kind":kind,"savedViewId":view}}), &f.user).await;
        rejected(response, "createPersonalList");
    }
}
