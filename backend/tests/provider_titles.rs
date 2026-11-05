//! A title's page works before it is in a library: `provider*` read through the metadata cache,
//! and each record's artwork resolves from that same cache. Records are seeded here, so nothing
//! in this test reaches a provider.

mod common;

use common::{GqlClient, TestApp, playback};
use serde_json::{Value, json};

async fn seed(admin: &mut GqlClient, provider: &str, operation: &str, key: &str, payload: Value) {
    playback::create(
        admin,
        "MetadataCache",
        json!({
            "provider": provider, "operation": operation, "cacheKey": key,
            "payload": payload.to_string(), "payloadVersion": 1,
            "fetchedAt": chrono::Utc::now().to_rfc3339(),
        }),
    )
    .await;
}

#[test]
fn provider_titles_come_from_the_metadata_cache() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;

        seed(&mut admin, "tvmaze", "GetTvShow", "77", json!({
            "provider": "tvmaze", "provider_id": 77, "name": "Cached Show", "sort_name": null, "year": 2026,
            "status": "Running", "overview": "A show.", "network": "Example", "runtime": 30,
            "genres": ["Drama"], "poster_url": null, "backdrop_url": null, "tvdb_id": null, "imdb_id": "tt7",
        })).await;
        seed(&mut admin, "tvmaze", "GetTvShowEpisodes", "77", json!([
            { "id": 5, "season": 1, "number": 1, "name": "Pilot", "overview": "First.", "air_date": "2026-01-01", "runtime": 30, "still_url": "https://static.tvmaze.com/none.jpg" },
            { "id": 6, "season": 1, "number": 2, "name": "Second", "overview": null, "air_date": null, "runtime": null, "still_url": null },
        ])).await;

        let data = admin.query(
            "query($id:Int!){providerShow(tvmazeId:$id){tvmazeId name network genres episodes{id number name hasStill}}}",
            json!({ "id": 77 }),
        ).await;
        let show = &data["providerShow"];
        assert_eq!(show["name"], "Cached Show", "{data}");
        assert_eq!(show["network"], "Example");
        assert_eq!(show["episodes"][0]["name"], "Pilot");
        assert_eq!(show["episodes"][0]["hasStill"], true);
        assert_eq!(show["episodes"][1]["hasStill"], false);

        // Provider pages are for signed-in people only.
        let response = app
            .client()
            .post("query{providerShow(tvmazeId:77){name}}", json!({}))
            .await;
        assert!(!response.errors().is_empty());

        app.shutdown().await;
    });
}
