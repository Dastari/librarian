//! Protected transport/runtime fields must fail through actual generated CRUD,
//! including nullable command fields that a false policy result would drop.
use super::auth::AuthUser;
use super::policy::{AppEntityPolicy, AppFieldPolicy};
use super::row_policy::OwnershipRowPolicy;
use crate::db::Database;
use crate::graphql::entities::{
    Library, LibraryMutations, LibraryQueries, LibraryWhereInput, Notification,
    NotificationMutations, NotificationQueries, NotificationWhereInput, Torrent, TorrentMutations,
    TorrentQueries, TorrentWhereInput,
};
use crate::services::graphql::entities::torrent::authorization_tests::{account, fixture};
use async_graphql::{EmptySubscription, MergedObject, Request, Response, Schema, Variables};
use graphql_orm::graphql::auth::AuthSubject;
use graphql_orm::graphql::filters::StringFilter;
use serde_json::{Value, json};

type TestSchema = Schema<Queries, Mutations, EmptySubscription>;
#[derive(Default, MergedObject)]
struct Queries(LibraryQueries, TorrentQueries, NotificationQueries);
#[derive(Default, MergedObject)]
struct Mutations(LibraryMutations, TorrentMutations, NotificationMutations);

struct Fixture {
    db: Database,
    schema: TestSchema,
    member: AuthUser,
    admin: AuthUser,
}
impl Fixture {
    async fn new() -> Self {
        let mut db = fixture().await;
        let member = account(&db, "member").await;
        let admin = account(&db, "admin").await;
        db.set_entity_policy(AppEntityPolicy);
        db.set_field_policy(AppFieldPolicy);
        db.set_row_policy(OwnershipRowPolicy);
        let schema = Schema::build(Queries::default(), Mutations::default(), EmptySubscription)
            .data(db.clone())
            .finish();
        Self {
            db,
            schema,
            member,
            admin,
        }
    }
    async fn execute(&self, actor: &AuthUser, document: String, variables: Value) -> Response {
        self.schema
            .execute(
                Request::new(document)
                    .variables(Variables::from_json(variables))
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
    async fn create(&self, actor: &AuthUser, entity: &str, input: Value) -> Response {
        let node = format!("{}{}", entity[..1].to_ascii_lowercase(), &entity[1..]);
        self.execute(actor, format!("mutation($input:Create{entity}Input!){{create{entity}(input:$input){{success error {node}{{id}}}}}}"), json!({"input":input})).await
    }
    async fn update(&self, actor: &AuthUser, entity: &str, id: &str, input: Value) -> Response {
        let node = format!("{}{}", entity[..1].to_ascii_lowercase(), &entity[1..]);
        self.execute(actor, format!("mutation($id:String!,$input:Update{entity}Input!){{update{entity}(id:$id,input:$input){{success error {node}{{id}}}}}}"), json!({"id":id,"input":input})).await
    }
    async fn provision(&self, entity: &str, input: Value) -> String {
        let response = self.create(&self.admin, entity, input).await;
        assert!(response.errors.is_empty(), "{response:?}");
        let data = response.data.into_json().unwrap();
        let field = format!("create{entity}");
        let node = format!("{}{}", entity[..1].to_ascii_lowercase(), &entity[1..]);
        assert_eq!(data[&field]["success"], true, "{data}");
        data[&field][&node]["id"].as_str().unwrap().to_string()
    }
    fn library_input(&self, name: &str, path: &str) -> Value {
        json!({"userId":self.member.user_id,"name":name,"path":path,"libraryType":"movies",
            "autoScan":false,"autoOrganize":false,"namingPattern":"","scanIntervalMinutes":60,"watchForChanges":false,"scanning":false})
    }
    fn torrent_input(&self, hash: &str) -> Value {
        json!({"userId":self.member.user_id,"infoHash":hash,"name":"Original torrent","state":"paused",
            "progress":0.0,"totalBytes":100,"downloadedBytes":0,"uploadedBytes":0,"uploadedBytesTotal":0,
            "savePath":"/fixture/downloads","excludedFiles":[],"addedAt":"2026-10-25T00:00:00Z"})
    }
    fn notification_input(&self, title: &str) -> Value {
        json!({"userId":self.member.user_id,"notificationType":"info","category":"test","title":title,
            "message":"Fixture","actionType":"approved_action","actionData":"{\"approved\":true}"})
    }
}
fn assert_forbidden(response: &Response) {
    assert!(
        response.errors.iter().any(|error| error
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.get("code"))
            == Some(&async_graphql::Value::from("FORBIDDEN"))),
        "expected hard FORBIDDEN, not a successful dropped field: {response:?}"
    );
}
fn assert_success(response: Response, field: &str) {
    assert!(response.errors.is_empty(), "{response:?}");
    let data = response.data.into_json().unwrap();
    assert_eq!(data[field]["success"], true, "{data}");
}
fn eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

#[tokio::test]
async fn member_cannot_manufacture_or_rewrite_torrent_runtime_identity() {
    let f = Fixture::new().await;
    let denied = f
        .create(&f.member, "Torrent", f.torrent_input("member-manufactured"))
        .await;
    assert_forbidden(&denied);
    assert!(
        Torrent::query(f.db.pool())
            .filter(TorrentWhereInput {
                info_hash: Some(eq("member-manufactured")),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap()
            .is_empty()
    );
    let id = f
        .provision("Torrent", f.torrent_input("admin-provisioned"))
        .await;
    assert_success(
        f.update(&f.member, "Torrent", &id, json!({"name":"Member label"}))
            .await,
        "updateTorrent",
    );
    let denied = f
        .update(
            &f.member,
            "Torrent",
            &id,
            json!({"infoHash":"forged-hash","name":"must not commit"}),
        )
        .await;
    assert_forbidden(&denied);
    let row = Torrent::get(f.db.pool(), &id).await.unwrap().unwrap();
    assert_eq!(row.info_hash, "admin-provisioned");
    assert_eq!(row.name, "Member label");
    assert_eq!(row.user_id, f.member.user_id);
    assert_success(
        f.update(
            &f.admin,
            "Torrent",
            &id,
            json!({"infoHash":"admin-approved-replacement"}),
        )
        .await,
        "updateTorrent",
    );
    assert_eq!(
        Torrent::get(f.db.pool(), &id)
            .await
            .unwrap()
            .unwrap()
            .info_hash,
        "admin-approved-replacement"
    );
}

#[tokio::test]
async fn member_can_label_an_owned_library_but_cannot_provision_or_change_its_path() {
    let f = Fixture::new().await;
    let denied = f
        .create(
            &f.member,
            "Library",
            f.library_input("Member provision attempt", "/unapproved/path"),
        )
        .await;
    assert_forbidden(&denied);
    assert!(
        Library::query(f.db.pool())
            .filter(LibraryWhereInput {
                name: Some(eq("Member provision attempt")),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap()
            .is_empty()
    );
    let id = f
        .provision(
            "Library",
            f.library_input("Original library", "/approved/path"),
        )
        .await;
    assert_success(
        f.update(&f.member, "Library", &id, json!({"name":"Member label"}))
            .await,
        "updateLibrary",
    );
    let denied = f
        .update(
            &f.member,
            "Library",
            &id,
            json!({"path":"/foreign/path","name":"must not commit"}),
        )
        .await;
    assert_forbidden(&denied);
    let row = Library::get(f.db.pool(), &id).await.unwrap().unwrap();
    assert_eq!(row.path, "/approved/path");
    assert_eq!(row.name, "Member label");
    assert_eq!(row.user_id, f.member.user_id);
    assert_success(
        f.update(
            &f.admin,
            "Library",
            &id,
            json!({"path":"/approved/replacement"}),
        )
        .await,
        "updateLibrary",
    );
    assert_eq!(
        Library::get(f.db.pool(), &id).await.unwrap().unwrap().path,
        "/approved/replacement"
    );
}

#[tokio::test]
async fn nullable_notification_command_fields_cannot_be_created_rewritten_or_cleared_by_members() {
    let f = Fixture::new().await;
    for (field, value) in [
        ("actionType", json!("forged_action")),
        ("actionData", json!("{\"forged\":true}")),
    ] {
        let mut input = f.notification_input("Member envelope attempt");
        input.as_object_mut().unwrap().remove("actionType");
        input.as_object_mut().unwrap().remove("actionData");
        input[field] = value;
        assert_forbidden(&f.create(&f.member, "Notification", input).await);
    }
    assert!(
        Notification::query(f.db.pool())
            .filter(NotificationWhereInput {
                title: Some(eq("Member envelope attempt")),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap()
            .is_empty()
    );
    let id = f
        .provision("Notification", f.notification_input("Approved envelope"))
        .await;
    assert_success(
        f.update(
            &f.member,
            "Notification",
            &id,
            json!({"readAt":"2026-10-25T00:00:00Z"}),
        )
        .await,
        "updateNotification",
    );
    for (field, value) in [
        ("actionType", json!("forged_action")),
        ("actionType", Value::Null),
        ("actionData", json!("{}")),
        ("actionData", Value::Null),
    ] {
        let mut input = json!({"title":"must not commit"});
        input[field] = value;
        assert_forbidden(&f.update(&f.member, "Notification", &id, input).await);
        let row = Notification::get(f.db.pool(), &id).await.unwrap().unwrap();
        assert_eq!(row.title, "Approved envelope");
        assert_eq!(row.action_type.as_deref(), Some("approved_action"));
        assert_eq!(row.action_data.as_deref(), Some("{\"approved\":true}"));
        assert!(row.read_at.is_some());
    }
    assert_success(
        f.update(
            &f.admin,
            "Notification",
            &id,
            json!({"actionType":"admin_replacement","actionData":"{}"}),
        )
        .await,
        "updateNotification",
    );
    let row = Notification::get(f.db.pool(), &id).await.unwrap().unwrap();
    assert_eq!(row.action_type.as_deref(), Some("admin_replacement"));
    assert_eq!(row.action_data.as_deref(), Some("{}"));
    assert_eq!(row.user_id, f.member.user_id);
}
