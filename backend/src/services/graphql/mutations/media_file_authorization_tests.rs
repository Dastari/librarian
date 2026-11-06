//! Hand-written media file mutations run with system authority below the resolver, so the
//! resolver is the only ownership check. These go through the real schema and row policy.
use async_graphql::{Context, EmptySubscription, MergedObject, Object, Request, Response, Schema};
use graphql_orm::graphql::auth::AuthSubject;
use serde_json::{Value, json};

use super::{LibraryScanMutations, authorized_media_file};
use crate::db::Database;
use crate::graphql::entities::{
    LibraryMutations, LibraryQueries, MediaFileMutations, MovieMutations,
};
use crate::services::graphql::auth::AuthUser;
use crate::services::graphql::entities::torrent::authorization_tests::{account, fixture};
use crate::services::graphql::policy::{AppEntityPolicy, AppFieldPolicy};
use crate::services::graphql::row_policy::OwnershipRowPolicy;

#[derive(Default)]
struct Probe;

#[Object]
impl Probe {
    async fn reachable(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<bool> {
        authorized_media_file(ctx, &id).await.map(|_| true)
    }
}

#[derive(Default, MergedObject)]
struct Queries(LibraryQueries, Probe);
#[derive(Default, MergedObject)]
struct Mutations(
    LibraryMutations,
    MovieMutations,
    MediaFileMutations,
    LibraryScanMutations,
);

struct Fixture {
    schema: Schema<Queries, Mutations, EmptySubscription>,
    admin: AuthUser,
    owner: AuthUser,
    other: AuthUser,
    file: String,
    foreign_movie: String,
}

async fn run(fixture: &Fixture, actor: &AuthUser, document: &str, variables: Value) -> Response {
    fixture
        .schema
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

async fn create(schema: &Fixture, entity: &str, input: Value) -> String {
    let node = format!("{}{}", entity[..1].to_ascii_lowercase(), &entity[1..]);
    let response = run(
        schema,
        &schema.admin,
        &format!(
            "mutation($input:Create{entity}Input!){{create{entity}(input:$input){{success error {node}{{id}}}}}}"
        ),
        json!({ "input": input }),
    )
    .await;
    assert!(response.errors.is_empty(), "{response:?}");
    let data = response.data.into_json().unwrap();
    data[format!("create{entity}")][&node]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{data}"))
        .to_string()
}

fn library(user: &AuthUser, name: &str) -> Value {
    json!({"userId":user.user_id,"name":name,"path":format!("/fixture/{name}"),"libraryType":"movies",
        "autoScan":false,"autoOrganize":false,"namingPattern":"","scanIntervalMinutes":60,"watchForChanges":false,"scanning":false})
}

async fn setup() -> Fixture {
    let mut db: Database = fixture().await;
    let admin = account(&db, "admin").await;
    let owner = account(&db, "member").await;
    let other = account(&db, "member").await;
    db.set_entity_policy(AppEntityPolicy);
    db.set_field_policy(AppFieldPolicy);
    db.set_row_policy(OwnershipRowPolicy);
    let schema = Schema::build(Queries::default(), Mutations::default(), EmptySubscription)
        .data(db)
        .finish();
    let mut fixture = Fixture {
        schema,
        admin,
        owner,
        other,
        file: String::new(),
        foreign_movie: String::new(),
    };
    let own_library = create(&fixture, "Library", library(&fixture.owner, "own")).await;
    let foreign_library = create(&fixture, "Library", library(&fixture.other, "foreign")).await;
    fixture.foreign_movie = create(
        &fixture,
        "Movie",
        json!({"libraryId":foreign_library,"userId":fixture.other.user_id,"title":"Foreign","monitored":false,"wanted":false,"hasFile":false,
            "genres":[],"castNames":[],"productionCountries":[],"spokenLanguages":[]}),
    )
    .await;
    fixture.file = create(
        &fixture,
        "MediaFile",
        json!({"libraryId":own_library,"path":"/fixture/own/file.mkv","size":1,"isHdr":false,"addedAt":"2026-01-01T00:00:00Z"}),
    )
    .await;
    fixture
}

fn assert_denied(response: &Response) {
    assert!(
        response
            .errors
            .iter()
            .any(|error| error.message.contains("access denied")),
        "{response:?}"
    );
}

#[tokio::test]
async fn another_member_cannot_reach_a_file_through_hand_written_mutations() {
    let f = setup().await;
    let id = json!({ "id": f.file });
    for document in [
        "mutation($id:String!){unmatchMediaFile(mediaFileId:$id){success}}",
        "mutation($id:String!){organizeMediaFile(input:{mediaFileId:$id}){success}}",
        "mutation($id:String!){matchMediaFile(input:{mediaFileId:$id}){success}}",
    ] {
        assert_denied(&run(&f, &f.other, document, id.clone()).await);
    }
}

#[tokio::test]
async fn deleting_from_disk_takes_an_administrator_even_for_the_owner() {
    let f = setup().await;
    let id = json!({ "id": f.file });
    let document = "mutation($id:String!){deleteMediaFileFromDisk(mediaFileId:$id){success}}";
    for member in [&f.owner, &f.other] {
        let response = run(&f, member, document, id.clone()).await;
        assert!(!response.errors.is_empty(), "{response:?}");
    }
}

#[tokio::test]
async fn a_member_cannot_link_their_file_to_someone_elses_title() {
    let f = setup().await;
    let response = run(
        &f,
        &f.owner,
        "mutation($id:String!,$movie:String!){matchMediaFile(input:{mediaFileId:$id,movieId:$movie}){success}}",
        json!({ "id": f.file, "movie": f.foreign_movie }),
    )
    .await;
    assert_denied(&response);
}

#[tokio::test]
async fn the_owner_and_an_admin_pass_the_check() {
    let f = setup().await;
    let document = "query($id:String!){reachable(id:$id)}";
    let id = json!({ "id": f.file });
    for actor in [&f.owner, &f.admin] {
        let response = run(&f, actor, document, id.clone()).await;
        assert!(response.errors.is_empty(), "{response:?}");
    }
    assert_denied(&run(&f, &f.other, document, id).await);
}
