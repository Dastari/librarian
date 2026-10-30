//! Auth surface contract: setup/registration, cookie shape, the cookie-CSRF
//! origin guard, refresh-token rotation, logout, and member-vs-admin
//! authorization on generated entities.
//!
//! Everything runs against the real Axum app over loopback HTTP, so the
//! assertions cover the handler-level guards in
//! `services/graphql/service.rs` as well as the resolvers themselves.

mod common;

use common::{
    ACCESS_COOKIE, DEFAULT_PASSWORD, LOGOUT_MUTATION, REFRESH_COOKIE, REFRESH_MUTATION,
    REGISTER_MUTATION, TestApp, TestAppOptions,
};
use librarian::services::graphql::entities::{CreateInviteTokenInput, InviteToken};
use serde_json::json;

const AUTHORIZATION_MATRIX: &str = include_str!("fixtures/authorization-matrix.json");

/// The policy pair the shipped matrix records for `entity`.
fn matrix_policy(entity: &str) -> (String, String) {
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(AUTHORIZATION_MATRIX).expect("authorization matrix should parse");
    let entry = entries
        .iter()
        .find(|entry| entry["entity"] == entity)
        .unwrap_or_else(|| panic!("authorization matrix has no entry for {entity}"));
    (
        entry["read"].as_str().expect("read policy").to_string(),
        entry["write"].as_str().expect("write policy").to_string(),
    )
}

fn assert_forbidden_write(response: &common::GqlResponse, field: &str) {
    assert_eq!(response.status, 200, "{}", response.body);
    assert!(
        response.body["errors"].as_array().is_some_and(|errors| {
            errors
                .iter()
                .any(|error| error["extensions"]["code"] == "FORBIDDEN")
        }),
        "{field} must reject protected fields with FORBIDDEN: {}",
        response.body
    );
    assert_ne!(response.body["data"][field]["success"], json!(true));
}

/// Mint an invite token straight through the entity layer.
///
/// `InviteToken.token` is `#[graphql_orm(private)]`, so it exists on the Rust
/// create input but not on `CreateInviteTokenInput` in the GraphQL schema —
/// there is no API-level way to mint one (noted in the report). Tests that
/// need a second, non-admin account therefore seed the row directly.
async fn mint_invite(app: &TestApp, admin_id: &str, role: &str, token: &str) {
    InviteToken::insert(
        &app.db,
        CreateInviteTokenInput {
            token: token.to_string(),
            created_by: admin_id.to_string(),
            library_ids: Vec::new(),
            role: role.to_string(),
            access_level: "full".to_string(),
            expires_at: None,
            max_uses: Some(5),
            use_count: 0,
            apply_restrictions: false,
            restrictions_template: None,
            is_active: true,
        },
    )
    .await
    .expect("invite token fixture should insert");
}

#[test]
fn first_registration_bootstraps_an_admin_and_later_ones_need_an_invite() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut first = app.client();

        let data = first
            .query(
                REGISTER_MUTATION,
                json!({ "input": { "email": "owner@example.test", "name": "Owner",
                               "password": DEFAULT_PASSWORD } }),
            )
            .await;
        assert_eq!(data["register"]["success"], true, "{data}");
        assert_eq!(data["register"]["user"]["role"], "admin");
        // The username is derived from the local part of the email.
        assert_eq!(data["register"]["user"]["username"], "owner");
        assert_eq!(data["register"]["tokens"]["tokenType"], "Bearer");

        // The second registration has no invite: rejected as a payload error, not
        // a raw GraphQL error, so the UI can render it.
        let mut second = app.client();
        let denied = second
            .query(
                REGISTER_MUTATION,
                json!({ "input": { "email": "intruder@example.test", "name": "Intruder",
                               "password": DEFAULT_PASSWORD } }),
            )
            .await;
        assert_eq!(denied["register"]["success"], false);
        assert_eq!(
            denied["register"]["error"],
            "Registration requires a valid invite token"
        );
        assert!(denied["register"]["user"].is_null());

        // With a valid invite the role comes from the invite, not from the request.
        let admin_id = data["register"]["user"]["id"]
            .as_str()
            .expect("admin id")
            .to_string();
        mint_invite(&app, &admin_id, "member", "invite-abc").await;
        let mut third = app.client();
        let invited = third
            .query(
                REGISTER_MUTATION,
                json!({ "input": { "email": "member@example.test", "name": "Member",
                               "password": DEFAULT_PASSWORD, "inviteToken": "invite-abc" } }),
            )
            .await;
        assert_eq!(invited["register"]["success"], true, "{invited}");
        assert_eq!(invited["register"]["user"]["role"], "member");

        app.shutdown().await;
    });
}

#[test]
fn login_sets_http_only_path_scoped_cookies() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut client = app.client();
        client.register_admin("admin", "admin@example.test").await;
        client.forget_cookies();

        let response = client.login("admin@example.test", DEFAULT_PASSWORD).await;
        assert_eq!(response.status, 200);
        assert_eq!(response.data()["login"]["success"], true);

        let access = response
            .cookie(ACCESS_COOKIE)
            .expect("login must set the access cookie");
        assert_eq!(access.path.as_deref(), Some("/"));
        assert!(access.http_only, "access cookie must be HttpOnly");
        assert_eq!(access.same_site.as_deref(), Some("Lax"));
        assert!(!access.secure, "plain-HTTP test origin must not set Secure");
        assert_eq!(access.max_age, Some(15 * 60));
        assert!(!access.value.is_empty());

        let refresh = response
            .cookie(REFRESH_COOKIE)
            .expect("login must set the refresh cookie");
        assert_eq!(
            refresh.path.as_deref(),
            Some("/graphql"),
            "the refresh cookie is scoped to /graphql so it is never sent to media routes"
        );
        assert!(refresh.http_only);
        assert_eq!(refresh.same_site.as_deref(), Some("Lax"));
        assert_eq!(refresh.max_age, Some(30 * 24 * 60 * 60));

        // Neither raw token is readable from the GraphQL payload.
        let body = response.body.to_string();
        assert!(
            !body.contains(&access.value) && !body.contains(&refresh.value),
            "tokens must live only in cookies, never in the response body"
        );

        // A wrong password is rejected with the generic message.
        let bad = client.login("admin@example.test", "wrong-password").await;
        assert_eq!(bad.data()["login"]["success"], false);
        assert_eq!(
            bad.data()["login"]["error"],
            "Invalid username/email or password"
        );

        app.shutdown().await;
    });
}

#[test]
fn secure_cookies_are_flagged_when_configured() {
    common::run_app_test(|| async {
        let app = TestApp::start_with(TestAppOptions {
            secure_cookies: true,
            // The guard drops `Secure` for cleartext-http Origins, so this client
            // must look like it came from an HTTPS deployment.
            extra_cors_origins: vec!["https://library.example".to_string()],
            ..TestAppOptions::default()
        })
        .await;

        let mut client = app.client().with_origin("https://library.example");
        let response = client
            .post(
                REGISTER_MUTATION,
                json!({ "input": { "email": "owner@example.test", "name": "Owner",
                               "password": DEFAULT_PASSWORD } }),
            )
            .await;
        assert_eq!(response.data()["register"]["success"], true);
        assert!(
            response
                .cookie(ACCESS_COOKIE)
                .expect("access cookie")
                .secure,
            "LIBRARIAN_SECURE_COOKIES must mark the access cookie Secure"
        );
        assert!(
            response
                .cookie(REFRESH_COOKIE)
                .expect("refresh cookie")
                .secure
        );

        app.shutdown().await;
    });
}

#[test]
fn cookie_authenticated_requests_require_an_allowed_origin() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut client = app.admin_client().await;
        assert!(client.cookies.contains_key(ACCESS_COOKIE));

        const ME: &str = "query { users(page: { limit: 1 }) { edges { node { id } } } }";

        // Allowed origin (the app's own): fine.
        let allowed = client.post(ME, json!({})).await;
        assert_eq!(allowed.status, 200);
        assert!(allowed.errors().is_empty(), "{:?}", allowed.errors());

        // Cross-site origin with cookies attached: rejected before the schema runs.
        let mut attacker = app.client().with_origin("https://attacker.example");
        attacker.cookies = client.cookies.clone();
        let blocked = attacker.post(ME, json!({})).await;
        assert_eq!(
            blocked.status, 403,
            "a cookie-authenticated request from an unlisted origin must be refused"
        );

        // Cookies but no Origin header at all: also refused (a browser always
        // sends one for cross-site POSTs, so a missing header is not trustworthy).
        let mut originless = app.client().without_origin();
        originless.cookies = client.cookies.clone();
        let no_origin = originless.post(ME, json!({})).await;
        assert_eq!(no_origin.status, 403);

        // The same untrusted origin without cookies is not a CSRF risk and is
        // allowed through to the schema (where it is simply unauthenticated).
        let mut anonymous = app.client().with_origin("https://attacker.example");
        let anonymous_response = anonymous.post(ME, json!({})).await;
        assert_eq!(anonymous_response.status, 200);
        assert!(
            !anonymous_response.errors().is_empty(),
            "anonymous access to users must still be denied by the policy layer"
        );

        app.shutdown().await;
    });
}

#[test]
fn refresh_rotates_the_token_and_invalidates_the_old_one() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut client = app.admin_client().await;

        let first_refresh = client
            .cookies
            .get(REFRESH_COOKIE)
            .cloned()
            .expect("registration should set a refresh cookie");
        let first_access = client
            .cookies
            .get(ACCESS_COOKIE)
            .cloned()
            .expect("registration should set an access cookie");

        let rotated = client.post(REFRESH_MUTATION, json!({})).await;
        assert_eq!(rotated.data()["refreshToken"]["success"], true);
        let new_refresh = rotated
            .cookie(REFRESH_COOKIE)
            .expect("refresh must re-issue the refresh cookie");
        assert_ne!(
            new_refresh.value, first_refresh,
            "refresh must rotate the refresh token, not reuse it"
        );
        assert_eq!(new_refresh.path.as_deref(), Some("/graphql"));
        assert!(new_refresh.http_only);
        let new_access = rotated
            .cookie(ACCESS_COOKIE)
            .expect("refresh must re-issue the access cookie");
        assert_ne!(new_access.value, first_access);
        assert_eq!(new_access.path.as_deref(), Some("/"));

        // Replaying the superseded refresh token fails and clears the cookies.
        let mut replay = app.client();
        replay
            .cookies
            .insert(REFRESH_COOKIE.to_string(), first_refresh);
        let replayed = replay.post(REFRESH_MUTATION, json!({})).await;
        assert_eq!(
            replayed.data()["refreshToken"]["success"],
            false,
            "a rotated-away refresh token must not be redeemable again"
        );
        assert_eq!(
            replayed
                .cookie(REFRESH_COOKIE)
                .expect("failed refresh clears the cookie")
                .max_age,
            Some(0)
        );

        // No refresh cookie at all is a clean failure, not a 500.
        let mut bare = app.client();
        let missing = bare.post(REFRESH_MUTATION, json!({})).await;
        assert_eq!(missing.data()["refreshToken"]["success"], false);
        assert_eq!(
            missing.data()["refreshToken"]["error"],
            "Refresh token missing"
        );

        app.shutdown().await;
    });
}

#[test]
fn logout_clears_both_cookies_and_revokes_the_session() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut client = app.admin_client().await;
        let refresh = client
            .cookies
            .get(REFRESH_COOKIE)
            .cloned()
            .expect("refresh cookie");

        let response = client.post(LOGOUT_MUTATION, json!({})).await;
        assert_eq!(response.data()["logout"]["success"], true);
        for name in [ACCESS_COOKIE, REFRESH_COOKIE] {
            let cookie = response
                .cookie(name)
                .unwrap_or_else(|| panic!("logout must clear {name}"));
            assert_eq!(cookie.max_age, Some(0));
            assert!(cookie.http_only, "cleared cookies keep their attributes");
            assert_eq!(cookie.same_site.as_deref(), Some("Lax"));
        }
        assert!(
            client.cookies.is_empty(),
            "client should have dropped both cookies"
        );

        // The revoked refresh token cannot be redeemed after logout.
        let mut stale = app.client();
        stale.cookies.insert(REFRESH_COOKIE.to_string(), refresh);
        let after = stale.post(REFRESH_MUTATION, json!({})).await;
        assert_eq!(after.data()["refreshToken"]["success"], false);

        app.shutdown().await;
    });
}

#[test]
fn member_and_admin_authorization_matches_the_shipped_matrix() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");
        mint_invite(&app, &admin_id, "member", "invite-member").await;

        let mut member = app.client();
        let registered = member
            .query(
                REGISTER_MUTATION,
                json!({ "input": { "email": "member@example.test", "name": "Member",
                               "password": DEFAULT_PASSWORD, "inviteToken": "invite-member" } }),
            )
            .await;
        assert_eq!(registered["register"]["user"]["role"], "member");
        member.user_id = registered["register"]["user"]["id"]
            .as_str()
            .map(str::to_string);

        // --- AppSetting: admin.read / admin.write -----------------------------
        assert_eq!(
            matrix_policy("AppSetting"),
            ("admin.read".to_string(), "admin.write".to_string())
        );
        const APP_SETTINGS: &str =
            "query { appSettings(page: { limit: 1 }) { edges { node { id key } } } }";
        let member_settings = member.post(APP_SETTINGS, json!({})).await;
        assert!(
            !member_settings.errors().is_empty(),
            "an admin.read entity must not be readable by a member"
        );
        let admin_settings = admin.query(APP_SETTINGS, json!({})).await;
        assert!(
            !admin_settings["appSettings"]["edges"]
                .as_array()
                .expect("edges")
                .is_empty(),
            "seeded defaults should be visible to an admin"
        );

        // --- QualityProfile: member.read / admin.write ------------------------
        assert_eq!(
            matrix_policy("QualityProfile"),
            ("member.read".to_string(), "admin.write".to_string())
        );
        const PROFILES: &str = "query { qualityProfiles(page: { limit: 5 }) { edges { node { id name isDefault } } } }";
        let member_profiles = member.query(PROFILES, json!({})).await;
        let seeded = member_profiles["qualityProfiles"]["edges"]
            .as_array()
            .expect("edges");
        assert_eq!(
            seeded.len(),
            1,
            "one default profile is seeded at bootstrap"
        );
        assert_eq!(seeded[0]["node"]["isDefault"], true);
        let profile_id = seeded[0]["node"]["id"].as_str().expect("profile id");

        const RENAME: &str = r#"mutation Rename($id: String!) {
      updateQualityProfile(id: $id, input: { name: "member edit" }) { success error }
    }"#;
        let member_write = member.post(RENAME, json!({ "id": profile_id })).await;
        let member_write_rejected = !member_write.errors().is_empty()
            || member_write.body["data"]["updateQualityProfile"]["success"] != json!(true);
        assert!(
            member_write_rejected,
            "an admin.write entity must not be writable by a member: {}",
            member_write.body
        );
        let admin_write = admin.query(RENAME, json!({ "id": profile_id })).await;
        assert_eq!(admin_write["updateQualityProfile"]["success"], true);

        // --- Library: owned metadata is member-writable; roots require admin --
        assert_eq!(
            matrix_policy("Library"),
            ("member.read".to_string(), "member.write".to_string())
        );
        const CREATE_LIBRARY: &str = r#"mutation Create($input: CreateLibraryInput!) {
      createLibrary(input: $input) { success error library { id userId name path } }
    }"#;
        let library_input = |user_id: &str, name: &str, path: &str| {
            json!({ "input": {
                "userId": user_id, "name": name, "path": path, "libraryType": "tv",
                "autoScan": false, "autoOrganize": false, "namingPattern": "",
                "scanIntervalMinutes": 60, "watchForChanges": false, "scanning": false
            }})
        };

        let admin_path = app
            .path("media/admin-library")
            .to_string_lossy()
            .to_string();
        let member_path = app
            .path("media/member-library")
            .to_string_lossy()
            .to_string();
        let admin_library = admin
            .query(
                CREATE_LIBRARY,
                library_input(&admin_id, "Admin Library", &admin_path),
            )
            .await;
        assert_eq!(admin_library["createLibrary"]["success"], true);
        let admin_library_id = admin_library["createLibrary"]["library"]["id"]
            .as_str()
            .expect("admin library id")
            .to_string();

        let member_id = member.user_id.clone().expect("member id");
        let member_create = member
            .post(
                CREATE_LIBRARY,
                library_input(&member_id, "Unapproved Library", &member_path),
            )
            .await;
        assert_forbidden_write(&member_create, "createLibrary");
        let rejected_library = admin.query(
            "query { libraries(where: { name: { eq: \"Unapproved Library\" } }) { pageInfo { totalCount } } }",
            json!({}),
        ).await;
        assert_eq!(rejected_library["libraries"]["pageInfo"]["totalCount"], 0);

        // Provision the approved root as admin without changing its intended owner.
        let member_library = admin
            .query(
                CREATE_LIBRARY,
                library_input(&member_id, "Member Library", &member_path),
            )
            .await;
        assert_eq!(member_library["createLibrary"]["success"], true);
        assert_eq!(
            member_library["createLibrary"]["library"]["userId"],
            member_id
        );
        assert_eq!(
            member_library["createLibrary"]["library"]["path"],
            member_path
        );
        let member_library_id = member_library["createLibrary"]["library"]["id"]
            .as_str()
            .expect("member library id")
            .to_string();

        const LIBRARIES: &str =
            "query { libraries(page: { limit: 50 }) { edges { node { id name userId } } } }";
        let member_view = member.query(LIBRARIES, json!({})).await;
        let member_rows = member_view["libraries"]["edges"].as_array().expect("edges");
        assert_eq!(member_rows.len(), 1, "a member only sees its own libraries");
        assert_eq!(member_rows[0]["node"]["userId"], member_id);
        assert_eq!(member_rows[0]["node"]["id"], member_library_id);

        let admin_view = admin.query(LIBRARIES, json!({})).await;
        assert_eq!(
            admin_view["libraries"]["edges"]
                .as_array()
                .expect("edges")
                .len(),
            2,
            "an admin sees every library"
        );

        const UPDATE_LIBRARY: &str = r#"mutation Update($id: String!, $input: UpdateLibraryInput!) {
          updateLibrary(id: $id, input: $input) { success error library { userId name path namingPattern scanIntervalMinutes } }
        }"#;
        let member_update = member.query(UPDATE_LIBRARY, json!({
            "id": member_library_id,
            "input": { "name": "Member Label", "namingPattern": "Custom/{show}/{title}.{ext}", "scanIntervalMinutes": 15 }
        })).await;
        assert_eq!(
            member_update["updateLibrary"]["success"], true,
            "{member_update}"
        );
        assert_eq!(
            member_update["updateLibrary"]["library"]["userId"],
            member_id
        );
        assert_eq!(
            member_update["updateLibrary"]["library"]["path"],
            member_path
        );
        assert_eq!(
            member_update["updateLibrary"]["library"]["namingPattern"],
            "Custom/{show}/{title}.{ext}"
        );
        assert_eq!(
            member_update["updateLibrary"]["library"]["scanIntervalMinutes"],
            15
        );
        let member_root_update = member.post(UPDATE_LIBRARY, json!({
            "id": member_library_id,
            "input": { "name": "must not commit", "path": app.path("media/unapproved-replacement") }
        })).await;
        assert_forbidden_write(&member_root_update, "updateLibrary");
        let member_persisted = member
            .query(
                "query Get($id: String!) { library(id: $id) { userId name path namingPattern } }",
                json!({"id": member_library_id}),
            )
            .await;
        assert_eq!(member_persisted["library"]["name"], "Member Label");
        assert_eq!(member_persisted["library"]["userId"], member_id);
        assert_eq!(member_persisted["library"]["path"], member_path);
        assert_eq!(
            member_persisted["library"]["namingPattern"],
            "Custom/{show}/{title}.{ext}"
        );

        // A valid owned library does not authorize manufacturing a runtime identity.
        let forged_hash = "abcdef0000000000000000000000000000000001";
        let member_torrent_create = member.post(
            "mutation Create($input: CreateTorrentInput!) { createTorrent(input: $input) { success error } }",
            json!({"input": {
                "userId": member_id, "libraryId": member_library_id, "infoHash": forged_hash,
                "name": "Member manufactured", "state": "paused", "progress": 0.0, "totalBytes": 100,
                "downloadedBytes": 0, "uploadedBytes": 0, "uploadedBytesTotal": 0,
                "savePath": app.path("downloads"), "excludedFiles": [], "addedAt": "2026-10-25T00:00:00Z"
            }}),
        ).await;
        assert_forbidden_write(&member_torrent_create, "createTorrent");
        let rejected_torrent = admin.query(
            "query Get($hash: String!) { torrents(where: { infoHash: { eq: $hash } }) { pageInfo { totalCount } } }",
            json!({"hash": forged_hash}),
        ).await;
        assert_eq!(rejected_torrent["torrents"]["pageInfo"]["totalCount"], 0);

        // A member cannot reach across the ownership boundary.
        const STEAL: &str = r#"mutation Steal($id: String!) {
      updateLibrary(id: $id, input: { name: "stolen" }) { success error }
    }"#;
        let steal = member.post(STEAL, json!({ "id": admin_library_id })).await;
        let steal_rejected = !steal.errors().is_empty()
            || steal.body["data"]["updateLibrary"]["success"] != json!(true);
        assert!(
            steal_rejected,
            "cross-owner write must fail: {}",
            steal.body
        );

        let still_named = admin
            .query(
                "query Get($id: String!) { library(id: $id) { name } }",
                json!({ "id": admin_library_id }),
            )
            .await;
        assert_eq!(still_named["library"]["name"], "Admin Library");

        let member_delete = member
            .query(
                "mutation Delete($id: String!) { deleteLibrary(id: $id) { success error } }",
                json!({"id": member_library_id}),
            )
            .await;
        assert_eq!(
            member_delete["deleteLibrary"]["success"], true,
            "{member_delete}"
        );
        let remaining = admin.query(LIBRARIES, json!({})).await;
        let remaining_rows = remaining["libraries"]["edges"].as_array().expect("edges");
        assert_eq!(remaining_rows.len(), 1);
        assert_eq!(remaining_rows[0]["node"]["id"], admin_library_id);

        app.shutdown().await;
    });
}

#[test]
fn unauthenticated_requests_are_rejected_by_the_policy_layer() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut anonymous = app.client();

        let response = anonymous
            .post(
                "query { qualityProfiles(page: { limit: 1 }) { edges { node { id } } } }",
                json!({}),
            )
            .await;
        assert_eq!(response.status, 200, "no cookies means no origin guard");
        assert!(
            !response.errors().is_empty(),
            "member.read entities must not be world-readable"
        );

        app.shutdown().await;
    });
}
