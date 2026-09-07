//! Generated-entity API contract: CRUD, `where`, `orderBy`, `page`, the
//! columns the acquisition work added, and the generated `*Changed`
//! subscription over `graphql-transport-ws`.
//!
//! These assertions exist so a change to the ORM (or to an entity's
//! attributes) that silently reshapes the public API fails here rather than in
//! the web client.

mod common;

use common::{TestApp, WsClient};
use serde_json::{Value, json};

const CREATE_LIBRARY: &str = r#"mutation Create($input: CreateLibraryInput!) {
  createLibrary(input: $input) { success error library { id name path libraryType userId } }
}"#;

fn library_input(user_id: &str, name: &str, path: &str, library_type: &str) -> Value {
    json!({ "input": {
        "userId": user_id, "name": name, "path": path, "libraryType": library_type,
        "autoScan": false, "autoOrganize": false, "namingPattern": "",
        "scanIntervalMinutes": 60, "watchForChanges": false, "scanning": false
    }})
}

fn node_strings(connection: &Value, field: &str) -> Vec<String> {
    connection["edges"]
        .as_array()
        .expect("edges should be an array")
        .iter()
        .map(|edge| {
            edge["node"][field]
                .as_str()
                .unwrap_or_else(|| panic!("node.{field} should be a string"))
                .to_string()
        })
        .collect()
}

#[test]
fn library_crud_where_order_by_and_page_contract() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");

        for (name, kind) in [
            ("Anime", "tv"),
            ("Cinema", "movies"),
            ("Docs", "movies"),
            ("Everything", "tv"),
        ] {
            let created = admin
                .query(
                    CREATE_LIBRARY,
                    library_input(&admin_id, name, &format!("/tmp/{name}"), kind),
                )
                .await;
            assert_eq!(created["createLibrary"]["success"], true, "{created}");
            assert_eq!(created["createLibrary"]["library"]["name"], name);
        }

        // where: two filters on one input are ANDed.
        let filtered = admin
            .query(
                r#"query {
              libraries(where: { libraryType: { eq: "movies" }, name: { contains: "o" } })
                { edges { node { name } } pageInfo { totalCount } }
            }"#,
                json!({}),
            )
            .await;
        assert_eq!(node_strings(&filtered["libraries"], "name"), vec!["Docs"]);
        assert_eq!(filtered["libraries"]["pageInfo"]["totalCount"], 1);

        // orderBy is a list of single-field maps.
        let descending = admin
            .query(
                "query { libraries(orderBy: [{ name: DESC }]) { edges { node { name } } } }",
                json!({}),
            )
            .await;
        assert_eq!(
            node_strings(&descending["libraries"], "name"),
            vec!["Everything", "Docs", "Cinema", "Anime"]
        );

        // page: offset pagination, with totalCount covering the whole match.
        let page = admin
            .query(
                r#"query { libraries(orderBy: [{ name: ASC }], page: { limit: 2, offset: 1 }) {
                 edges { node { name } cursor }
                 pageInfo { hasNextPage hasPreviousPage totalCount startCursor endCursor }
               } }"#,
                json!({}),
            )
            .await;
        assert_eq!(
            node_strings(&page["libraries"], "name"),
            vec!["Cinema", "Docs"]
        );
        assert_eq!(page["libraries"]["pageInfo"]["totalCount"], 4);
        assert_eq!(page["libraries"]["pageInfo"]["hasNextPage"], true);
        assert_eq!(page["libraries"]["pageInfo"]["hasPreviousPage"], true);

        // Single get, update, delete.
        let target = admin
        .query(
            r#"query { libraries(where: { name: { eq: "Anime" } }) { edges { node { id } } } }"#,
            json!({}),
        )
        .await;
        let id = target["libraries"]["edges"][0]["node"]["id"]
            .as_str()
            .expect("library id")
            .to_string();

        let updated = admin
            .query(
                r#"mutation U($id: String!) {
                 updateLibrary(id: $id, input: { name: "Anime (renamed)", scanIntervalMinutes: 15 })
                   { success error library { name scanIntervalMinutes } }
               }"#,
                json!({ "id": id }),
            )
            .await;
        assert_eq!(updated["updateLibrary"]["success"], true, "{updated}");
        assert_eq!(
            updated["updateLibrary"]["library"]["name"],
            "Anime (renamed)"
        );
        assert_eq!(
            updated["updateLibrary"]["library"]["scanIntervalMinutes"],
            15
        );

        let deleted = admin
            .query(
                "mutation D($id: String!) { deleteLibrary(id: $id) { success error } }",
                json!({ "id": id }),
            )
            .await;
        assert_eq!(deleted["deleteLibrary"]["success"], true);

        let gone = admin
            .query(
                "query G($id: String!) { library(id: $id) { id } }",
                json!({ "id": id }),
            )
            .await;
        assert!(gone["library"].is_null());

        app.shutdown().await;
    });
}

#[test]
fn quality_profile_round_trips_every_field_including_the_new_rules() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;

        let input = json!({
            "name": "Strict 1080p",
            "mediaKind": "VIDEO",
            "allowedResolutions": ["1080p", "720p"],
            "allowedVideoCodecs": ["x265", "h264"],
            "allowedAudioFormats": ["DTS", "DDP"],
            "allowedHdrTypes": ["HDR10"],
            "allowedSources": ["WEB-DL", "BluRay"],
            "releaseGroupBlacklist": ["BADGRP"],
            "releaseGroupWhitelist": ["GOODGRP"],
            "requireHdr": false,
            "cutoffResolution": "1080p",
            "upgradeUntilCutoff": true,
            "preferredLanguages": ["en", "de"],
            "requireLanguageMatch": true,
            "minSizeMb": 300,
            "maxSizeMb": 4000,
            "minSeeders": 5,
            "maxReleaseAgeDays": 400,
            "preferredReleaseGroups": ["GOODGRP", "OKGRP"],
            "allowSeasonPacks": true,
            "preferProperRepack": true,
            "resolutionPreference": ["1080p", "720p"],
            "isDefault": false
        });

        const ALL_FIELDS: &str = r#"
        id name mediaKind allowedResolutions allowedVideoCodecs allowedAudioFormats
        allowedHdrTypes allowedSources releaseGroupBlacklist releaseGroupWhitelist
        requireHdr cutoffResolution upgradeUntilCutoff preferredLanguages
        requireLanguageMatch minSizeMb maxSizeMb minSeeders maxReleaseAgeDays
        preferredReleaseGroups allowSeasonPacks preferProperRepack resolutionPreference
        isDefault createdAt updatedAt"#;

        let created = admin
            .query(
                &format!(
                    "mutation C($input: CreateQualityProfileInput!) {{ \
                   createQualityProfile(input: $input) \
                   {{ success error qualityProfile {{ {ALL_FIELDS} }} }} }}"
                ),
                json!({ "input": input }),
            )
            .await;
        assert_eq!(
            created["createQualityProfile"]["success"], true,
            "{created}"
        );
        let profile = &created["createQualityProfile"]["qualityProfile"];
        let id = profile["id"].as_str().expect("profile id").to_string();

        // Every supplied field comes back identical.
        for (key, expected) in input.as_object().expect("input object") {
            assert_eq!(
                &profile[key], expected,
                "field {key} did not round-trip: got {}",
                profile[key]
            );
        }

        // Re-read through the single-get query to prove it is persisted, not just
        // echoed back from the mutation payload.
        let fetched = admin
            .query(
                &format!("query G($id: String!) {{ qualityProfile(id: $id) {{ {ALL_FIELDS} }} }}"),
                json!({ "id": id }),
            )
            .await;
        for (key, expected) in input.as_object().expect("input object") {
            assert_eq!(&fetched["qualityProfile"][key], expected, "field {key}");
        }

        // A partial update leaves untouched JSON columns alone.
        let updated = admin
            .query(
                &format!(
                    "mutation U($id: String!) {{ updateQualityProfile(id: $id, input: \
                 {{ minSeeders: 12, resolutionPreference: [\"2160p\"], allowSeasonPacks: false }}) \
                 {{ success error qualityProfile {{ {ALL_FIELDS} }} }} }}"
                ),
                json!({ "id": id }),
            )
            .await;
        let after = &updated["updateQualityProfile"]["qualityProfile"];
        assert_eq!(after["minSeeders"], 12);
        assert_eq!(after["resolutionPreference"], json!(["2160p"]));
        assert_eq!(after["allowSeasonPacks"], false);
        assert_eq!(after["preferredLanguages"], json!(["en", "de"]));
        assert_eq!(after["allowedResolutions"], json!(["1080p", "720p"]));

        // Bootstrap seeds exactly one default profile, and it stays the default.
        let defaults = admin
            .query(
                "query { qualityProfiles(where: { isDefault: { eq: true } }) \
             { edges { node { name } } } }",
                json!({}),
            )
            .await;
        assert_eq!(
            node_strings(&defaults["qualityProfiles"], "name"),
            vec!["Any Quality"]
        );

        app.shutdown().await;
    });
}

#[test]
fn release_blocklist_crud_and_filters() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;

        const FIELDS: &str = "id infoHash guid title sourceId reason showId movieId albumId \
        audiobookId createdAt expiresAt";
        const CREATE: &str = "mutation C($input: CreateReleaseBlocklistInput!) { \
        createReleaseBlocklist(input: $input) { success error releaseBlocklist { %FIELDS% } } }";

        let create_document = CREATE.replace("%FIELDS%", FIELDS);
        let created = admin
            .query(
                &create_document,
                json!({ "input": {
                    "infoHash": "aabbcc00112233445566778899aabbccddeeff00",
                    "guid": "https://indexer.test/details/1",
                    "title": "Some Show S01E01 1080p WEB-DL-BADGRP",
                    "sourceId": "source-1",
                    "reason": "import_failed",
                    "expiresAt": null
                }}),
            )
            .await;
        assert_eq!(
            created["createReleaseBlocklist"]["success"], true,
            "{created}"
        );
        let row = &created["createReleaseBlocklist"]["releaseBlocklist"];
        assert_eq!(row["reason"], "import_failed");
        assert!(
            row["expiresAt"].is_null(),
            "a null expiry means a permanent block"
        );
        let id = row["id"].as_str().expect("id").to_string();

        admin
            .query(
                &create_document,
                json!({ "input": {
                    "infoHash": "1111111111111111111111111111111111111111",
                    "title": "Other Release",
                    "reason": "stalled"
                }}),
            )
            .await;

        let stalled = admin
            .query(
                r#"query { releaseBlocklists(where: { reason: { eq: "stalled" } },
                                         orderBy: [{ title: ASC }])
                 { edges { node { title reason } } pageInfo { totalCount } } }"#,
                json!({}),
            )
            .await;
        assert_eq!(stalled["releaseBlocklists"]["pageInfo"]["totalCount"], 1);
        assert_eq!(
            node_strings(&stalled["releaseBlocklists"], "title"),
            vec!["Other Release"]
        );

        let by_hash = admin
            .query(
                r#"query { releaseBlocklists(where: { infoHash: { startsWith: "aabb" } })
                 { edges { node { id } } } }"#,
                json!({}),
            )
            .await;
        assert_eq!(by_hash["releaseBlocklists"]["edges"][0]["node"]["id"], id);

        let deleted = admin
            .query(
                "mutation D($id: String!) { deleteReleaseBlocklist(id: $id) { success error } }",
                json!({ "id": id }),
            )
            .await;
        assert_eq!(deleted["deleteReleaseBlocklist"]["success"], true);

        app.shutdown().await;
    });
}

#[test]
fn torrent_exposes_the_seeding_and_season_pack_columns() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");

        const FIELDS: &str = "id infoHash name state season uploadedBytes uploadedBytesTotal \
        minimumRatio minimumSeedTimeMinutes showId episodeId postProcessStatus postProcessError";

        let created = admin
            .query(
                &format!(
                    "mutation C($input: CreateTorrentInput!) {{ \
                   createTorrent(input: $input) {{ success error torrent {{ {FIELDS} }} }} }}"
                ),
                json!({ "input": {
                    "userId": admin_id,
                    "infoHash": "deadbeef00000000000000000000000000000001",
                    "name": "Some Show S02 1080p WEB-DL",
                    "state": "downloading",
                    "progress": 0.0,
                    "totalBytes": 10_000_000,
                    "downloadedBytes": 0,
                    "uploadedBytes": 0,
                    "uploadedBytesTotal": 4_096,
                    "savePath": "/tmp/downloads",
                    "excludedFiles": [],
                    "addedAt": "2026-01-01T00:00:00.000Z",
                    "season": 2,
                    "minimumRatio": 1.5,
                    "minimumSeedTimeMinutes": 2880
                }}),
            )
            .await;
        assert_eq!(created["createTorrent"]["success"], true, "{created}");
        let torrent = &created["createTorrent"]["torrent"];
        assert_eq!(torrent["season"], 2);
        assert_eq!(torrent["uploadedBytesTotal"], 4096);
        assert_eq!(torrent["minimumRatio"], 1.5);
        assert_eq!(torrent["minimumSeedTimeMinutes"], 2880);
        let id = torrent["id"].as_str().expect("torrent id").to_string();

        // `season` and `uploadedBytesTotal` are filterable; the tracker minimums
        // deliberately are not (they are per-grab facts, not query dimensions).
        let by_season = admin
            .query(
                "query { torrents(where: { season: { eq: 2 } }) { edges { node { id } } } }",
                json!({}),
            )
            .await;
        assert_eq!(by_season["torrents"]["edges"][0]["node"]["id"], id);

        let by_uploaded = admin
            .query(
                "query { torrents(where: { uploadedBytesTotal: { gte: 4096 } }) \
             { pageInfo { totalCount } } }",
                json!({}),
            )
            .await;
        assert_eq!(by_uploaded["torrents"]["pageInfo"]["totalCount"], 1);

        let unfilterable = admin
        .post(
            "query { torrents(where: { minimumRatio: { eq: 1.5 } }) { edges { node { id } } } }",
            json!({}),
        )
        .await;
        assert!(
            !unfilterable.errors().is_empty(),
            "minimumRatio must not be part of TorrentWhereInput"
        );

        // The monotonic upload counter is what the seeding rules read, so it must
        // survive an update that also resets the per-session counter.
        let updated = admin
            .query(
                &format!(
                    "mutation U($id: String!) {{ updateTorrent(id: $id, input: \
                 {{ uploadedBytes: 0, uploadedBytesTotal: 20000000, \
                    postProcessStatus: \"completed\" }}) \
                 {{ success error torrent {{ {FIELDS} }} }} }}"
                ),
                json!({ "id": id }),
            )
            .await;
        assert_eq!(updated["updateTorrent"]["success"], true, "{updated}");
        assert_eq!(updated["updateTorrent"]["torrent"]["uploadedBytes"], 0);
        assert_eq!(
            updated["updateTorrent"]["torrent"]["uploadedBytesTotal"],
            20_000_000
        );
        assert_eq!(
            updated["updateTorrent"]["torrent"]["postProcessStatus"],
            "completed"
        );

        app.shutdown().await;
    });
}

#[test]
fn generated_changed_subscription_streams_creates_and_updates() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");
        let access_token = admin
            .cookies
            .get(common::ACCESS_COOKIE)
            .cloned()
            .expect("access token");

        let mut ws = WsClient::connect(&app, &access_token).await;
        ws.subscribe(
            "libs",
            "subscription { libraryChanged { action changeKind id library { name libraryType } } }",
        )
        .await;

        // Let the subscription register before triggering the write.
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;

        let created = admin
            .query(
                CREATE_LIBRARY,
                library_input(&admin_id, "Streamed", "/tmp/streamed", "tv"),
            )
            .await;
        let library_id = created["createLibrary"]["library"]["id"]
            .as_str()
            .expect("library id")
            .to_string();

        let event = ws.next_payload("libs").await;
        assert_eq!(event["data"]["libraryChanged"]["action"], "CREATED");
        assert_eq!(event["data"]["libraryChanged"]["changeKind"], "DIRECT");
        assert_eq!(event["data"]["libraryChanged"]["id"], library_id);
        assert_eq!(
            event["data"]["libraryChanged"]["library"]["name"],
            "Streamed"
        );

        admin
            .query(
                r#"mutation U($id: String!) {
                 updateLibrary(id: $id, input: { name: "Streamed again" }) { success error }
               }"#,
                json!({ "id": library_id }),
            )
            .await;

        let update_event = ws.next_payload("libs").await;
        assert_eq!(update_event["data"]["libraryChanged"]["action"], "UPDATED");
        assert_eq!(
            update_event["data"]["libraryChanged"]["library"]["name"],
            "Streamed again"
        );

        app.shutdown().await;
    });
}
