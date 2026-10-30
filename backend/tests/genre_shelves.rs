//! A library's genre shelves: its largest genres with their newest titles, readable only by
//! the library's owner (and administrators).
mod common;

use common::{DEFAULT_PASSWORD, TestApp};
use librarian::graphql::entities::{CreateInviteTokenInput, InviteToken};
use serde_json::{Value, json};

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

async fn movie_library(
    app: &TestApp,
    admin: &mut common::GqlClient,
    owner: &str,
    name: &str,
) -> String {
    let path = app.path(&format!("media/{name}"));
    std::fs::create_dir_all(&path).expect("library folder");
    create(
        admin,
        "mutation($input: CreateLibraryInput!) { result: createLibrary(input: $input) { success error library { id } } }",
        json!({ "userId": owner, "name": name, "path": path.display().to_string(), "libraryType": "movies",
                "autoScan": false, "autoOrganize": false, "namingPattern": "{title}",
                "scanIntervalMinutes": 60, "watchForChanges": false, "scanning": false }),
        "library",
    )
    .await
}

async fn movie(
    admin: &mut common::GqlClient,
    library: &str,
    owner: &str,
    title: &str,
    genres: &[&str],
) {
    create(
        admin,
        "mutation($input: CreateMovieInput!) { result: createMovie(input: $input) { success error movie { id } } }",
        json!({ "libraryId": library, "userId": owner, "title": title, "genres": genres, "castNames": [],
                "productionCountries": [], "spokenLanguages": [], "monitored": false, "wanted": false, "hasFile": false }),
        "movie",
    )
    .await;
}

const SHELVES: &str = r#"query($id: String!) {
  libraryGenreShelves(libraryId: $id, maxGenres: 2, perGenre: 2) { genre total movies { title } shows { name } }
}"#;

#[test]
fn genre_shelves_group_a_library_by_its_largest_genres_for_its_owner() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");
        InviteToken::insert(
            &app.db,
            CreateInviteTokenInput {
                token: "invite-genre-member".to_string(),
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
                "mutation Register($input: RegisterUserInput!) { register(input: $input) { success error user { id } } }",
                json!({ "input": { "email": "genres@example.test", "name": "Member",
                    "password": DEFAULT_PASSWORD, "inviteToken": "invite-genre-member" } }),
            )
            .await;
        let member_id = registered["register"]["user"]["id"]
            .as_str()
            .expect("member id")
            .to_string();

        let own = movie_library(&app, &mut admin, &member_id, "member-films").await;
        movie(
            &mut admin,
            &own,
            &member_id,
            "Oldest thriller",
            &["Thriller", "Drama"],
        )
        .await;
        // `createdAt` has one-second resolution; space out titles whose order is asserted.
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        movie(
            &mut admin,
            &own,
            &member_id,
            "Middle thriller",
            &["thriller"],
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        movie(
            &mut admin,
            &own,
            &member_id,
            "Newest thriller",
            &["Thriller"],
        )
        .await;
        movie(&mut admin, &own, &member_id, "Comedy", &["Comedy"]).await;

        let shelves = member.query(SHELVES, json!({ "id": own })).await;
        let rows = shelves["libraryGenreShelves"].as_array().expect("shelves");
        assert_eq!(rows.len(), 2, "maxGenres caps the shelves: {shelves}");
        assert_eq!(rows[0]["genre"], "Thriller");
        assert_eq!(rows[0]["total"], 3, "genres match regardless of case");
        assert_eq!(
            rows[0]["movies"],
            json!([{ "title": "Newest thriller" }, { "title": "Middle thriller" }]),
            "newest first, capped at perGenre"
        );
        // Ties keep a stable alphabetical order.
        assert_eq!(rows[1]["genre"], "Comedy");
        assert_eq!(rows[1]["shows"], json!([]));

        let foreign = movie_library(&app, &mut admin, &admin_id, "admin-films").await;
        let denied = member.post(SHELVES, json!({ "id": foreign })).await;
        assert!(
            denied
                .errors()
                .iter()
                .any(|error| error.contains("Library not found")),
            "another owner's library is indistinguishable from a missing one: {}",
            denied.body
        );
        let admin_view = admin.query(SHELVES, json!({ "id": own })).await;
        assert_eq!(admin_view["libraryGenreShelves"][0]["total"], 3);

        app.shutdown().await;
    });
}
