//! A person edits only their own profile through the generated User roots; administrators edit
//! and manage everyone's (design.md Q147).
use async_graphql::{MergedObject, Request, Schema, Variables};
use graphql_orm::graphql::auth::AuthSubject;
use serde_json::{Value, json};

use crate::db::Database;
use crate::graphql::entities::*;
use crate::services::database::DatabaseService;
use crate::services::graphql::auth::AuthUser;
use crate::services::graphql::entities::torrent::authorization_tests::{account, fixture};

#[derive(Default, MergedObject)]
struct Queries(UserQueries, PasswordResetTokenQueries);
#[derive(Default, MergedObject)]
struct Mutations(UserMutations, PasswordResetTokenMutations);
type TestSchema = Schema<Queries, Mutations, async_graphql::EmptySubscription>;

struct Fixture {
    db: Database,
    schema: TestSchema,
    member: AuthUser,
    other: AuthUser,
    admin: AuthUser,
}

impl Fixture {
    async fn new() -> Self {
        let db = DatabaseService::new(fixture().await).pool().clone();
        let member = account(&db, "member").await;
        let other = account(&db, "member").await;
        let admin = account(&db, "admin").await;
        let schema = Schema::build(Queries::default(), Mutations::default(), async_graphql::EmptySubscription)
            .data(db.clone())
            .finish();
        Self { db, schema, member, other, admin }
    }

    async fn run(&self, document: &str, variables: Value, user: &AuthUser) -> Value {
        let request = Request::new(document)
            .variables(Variables::from_json(variables))
            .data(user.clone())
            .data(user.user_id.clone())
            .data(AuthSubject::from_parts(user.user_id.clone(), vec![user.role.clone().unwrap()], vec![], None));
        let response = self.schema.execute(request).await;
        json!({ "errors": response.errors.iter().map(|error| error.message.clone()).collect::<Vec<_>>(), "data": response.data.into_json().unwrap_or(Value::Null) })
    }

    async fn update(&self, id: &str, input: Value, user: &AuthUser) -> Value {
        self.run(
            "mutation($id:String!,$input:UpdateUserInput!){updateUser(id:$id,input:$input){success error user{id displayName phone genreInterests role}}}",
            json!({"id": id, "input": input}),
            user,
        )
        .await
    }
}

fn refused(result: &Value, field: &str) -> bool {
    !result["errors"].as_array().unwrap().is_empty() || result["data"][field]["success"] == false
}

#[tokio::test]
async fn a_member_reads_and_edits_only_their_own_profile() {
    let f = Fixture::new().await;
    let listed = f.run("{users{edges{node{id}}}}", json!({}), &f.member).await;
    let ids: Vec<&str> = listed["data"]["users"]["edges"].as_array().unwrap().iter().map(|edge| edge["node"]["id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec![f.member.user_id.as_str()], "{listed}");
    let other = f.run("query($id:String!){user(id:$id){id}}", json!({"id": f.other.user_id}), &f.member).await;
    assert!(other["data"]["user"].is_null(), "{other}");

    let saved = f.update(&f.member.user_id, json!({"displayName": "  Sam  ", "phone": "+61412345678", "genreInterests": ["Sci-Fi", "sci-fi", " Drama "]}), &f.member).await;
    assert_eq!(saved["data"]["updateUser"]["success"], true, "{saved}");
    assert_eq!(saved["data"]["updateUser"]["user"]["displayName"], "Sam");
    assert_eq!(saved["data"]["updateUser"]["user"]["genreInterests"], json!(["Sci-Fi", "Drama"]));

    let theirs = f.update(&f.other.user_id, json!({"displayName": "Not mine"}), &f.member).await;
    assert!(refused(&theirs, "updateUser"), "{theirs}");
    assert_eq!(User::get(f.db.pool(), &f.other.user_id).await.unwrap().unwrap().display_name, None);
}

#[tokio::test]
async fn a_member_cannot_raise_their_role_or_manage_accounts() {
    let f = Fixture::new().await;
    let role = f.update(&f.member.user_id, json!({"role": "admin"}), &f.member).await;
    assert!(refused(&role, "updateUser"), "{role}");
    let active = f.update(&f.other.user_id, json!({"isActive": false}), &f.member).await;
    assert!(refused(&active, "updateUser"), "{active}");
    assert_eq!(User::get(f.db.pool(), &f.member.user_id).await.unwrap().unwrap().role, "member");

    // The security facts have no input at all: the schema rejects them.
    let facts = f.update(&f.member.user_id, json!({"mustChangePassword": false}), &f.member).await;
    assert!(!facts["errors"].as_array().unwrap().is_empty(), "{facts}");

    let deleted = f.run("mutation($id:String!){deleteUser(id:$id){success error}}", json!({"id": f.member.user_id}), &f.member).await;
    assert!(refused(&deleted, "deleteUser"), "{deleted}");
    assert!(User::get(f.db.pool(), &f.member.user_id).await.unwrap().is_some());

    let created = f.run("mutation($input:CreateUserInput!){createUser(input:$input){success error}}", json!({"input": {"username": "sneaky", "role": "admin", "isActive": true}}), &f.member).await;
    assert!(refused(&created, "createUser"), "{created}");

    let links = f.run("{passwordResetTokens{edges{node{id}}}}", json!({}), &f.member).await;
    assert!(!links["errors"].as_array().unwrap().is_empty() || links["data"]["passwordResetTokens"].is_null(), "{links}");
}

#[tokio::test]
async fn an_administrator_edits_and_removes_others() {
    let f = Fixture::new().await;
    let role = f.update(&f.other.user_id, json!({"role": "admin", "displayName": "Promoted"}), &f.admin).await;
    assert_eq!(role["data"]["updateUser"]["success"], true, "{role}");
    assert_eq!(role["data"]["updateUser"]["user"]["role"], "admin");
    let deleted = f.run("mutation($id:String!){deleteUser(id:$id){success error}}", json!({"id": f.member.user_id}), &f.admin).await;
    assert_eq!(deleted["data"]["deleteUser"]["success"], true, "{deleted}");
}

#[tokio::test]
async fn profile_fields_are_checked() {
    let f = Fixture::new().await;
    let email = f.update(&f.member.user_id, json!({"email": "not an email"}), &f.member).await;
    assert!(refused(&email, "updateUser"), "{email}");
    let phone = f.update(&f.member.user_id, json!({"phone": "call me"}), &f.member).await;
    assert!(refused(&phone, "updateUser"), "{phone}");
    let many: Vec<String> = (0..13).map(|index| format!("Genre {index}")).collect();
    let genres = f.update(&f.member.user_id, json!({"genreInterests": many}), &f.member).await;
    assert!(refused(&genres, "updateUser"), "{genres}");

    // An address another account uses would make sign-in ambiguous.
    assert_eq!(f.update(&f.other.user_id, json!({"email": "taken@example.test"}), &f.admin).await["data"]["updateUser"]["success"], true);
    let taken = f.update(&f.member.user_id, json!({"email": "Taken@Example.test"}), &f.member).await;
    assert!(refused(&taken, "updateUser"), "{taken}");
}
