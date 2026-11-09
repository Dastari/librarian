//! RSS feeds as sources, end to end: the address is encrypted and never readable, a poll fills the
//! catalogue with fully read items (whether or not automatic downloading is on), the catalogue
//! answers a release search without exposing a link, and a failing source is recorded and alerted
//! once.
//!
//! The "feed" is the mock indexer's Torznab endpoint, which is RSS 2.0 with the same item shape as a
//! tracker's feed. Its key plays the part of a passkey.

mod common;

use common::mock_indexer::MockIndexer;
use common::{GqlClient, TestApp};
use librarian::graphql::entities::{
    Notification, NotificationWhereInput, RssFeed, RssFeedItem, RssFeedItemWhereInput, Source,
};
use serde_json::json;

async fn create_feed(admin: &mut GqlClient, url: &str) -> String {
    let user_id = admin.user_id.clone().expect("admin id");
    let created = admin
        .query(
            r#"mutation C($input: CreateRssFeedInput!) {
                 createRssFeed(input: $input) { success error rssFeed { id urlDisplay healthStatus } }
               }"#,
            json!({ "input": {
                "userId": user_id, "name": "Mock feed", "url": url,
                "enabled": true, "pollIntervalMinutes": 30
            }}),
        )
        .await;
    assert_eq!(created["createRssFeed"]["success"], true, "{created}");
    created["createRssFeed"]["rssFeed"]["id"]
        .as_str()
        .expect("feed id")
        .to_string()
}

#[test]
fn a_feed_is_encrypted_polled_into_a_catalogue_and_answers_a_release_search() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;
        indexer
            .add_release(
                "Catalogue Show S01E01 1080p WEB-DL x264-GRP",
                "e01.mkv",
                32 * 1024,
            )
            .await;
        indexer
            .add_release(
                "Catalogue Movie 2019 2160p WEB-DL x265-GRP",
                "movie.mkv",
                32 * 1024,
            )
            .await;
        // The mock lists releases as TV by default; this one is a movie, as its category says.
        indexer.amend_last(|release| release.category = 2000);
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let passkey = indexer.api_key.clone();
        let address = format!("{}?t=search&apikey={passkey}", indexer.api_url);
        let feed_id = create_feed(&mut admin, &address).await;

        // The address is stored as an encrypted envelope and shown only with its values hidden.
        let stored = RssFeed::get(app.db.pool(), &feed_id)
            .await
            .unwrap()
            .expect("the feed exists");
        assert!(stored.url.starts_with("v2:"), "{}", stored.url);
        assert!(!stored.url.contains(&passkey));
        let shown = admin
            .query(
                "query F($id: String!) { rssFeed(id: $id) { urlDisplay } }",
                json!({ "id": feed_id }),
            )
            .await;
        let display = shown["rssFeed"]["urlDisplay"].as_str().expect("display");
        assert!(!display.contains(&passkey), "{display}");
        assert!(display.contains("apikey=•••"), "{display}");
        // And there is no field to read it from.
        let response = admin
            .post("query { rssFeeds { edges { node { url } } } }", json!({}))
            .await;
        assert!(
            !response.errors().is_empty(),
            "the address must not be selectable"
        );

        // Poll now: automatic downloading is off, and the catalogue fills all the same.
        let polled = admin
            .query(
                "mutation P($id: String!) { pollRssFeed(id: $id) { success error newItems grabbed } }",
                json!({ "id": feed_id }),
            )
            .await;
        assert_eq!(polled["pollRssFeed"]["success"], true, "{polled}");
        assert_eq!(polled["pollRssFeed"]["newItems"], 2, "{polled}");
        assert_eq!(polled["pollRssFeed"]["grabbed"], 0);

        let items = RssFeedItem::query(app.db.pool())
            .filter(RssFeedItemWhereInput::default())
            .fetch_all()
            .await
            .unwrap();
        assert_eq!(items.len(), 2);
        let movie = items
            .iter()
            .find(|item| item.title.starts_with("Catalogue Movie"))
            .expect("the movie item");
        // Read in full, and the link is an envelope too.
        assert_eq!(movie.parsed_scope.as_deref(), Some("movie"));
        assert_eq!(movie.parsed_year, Some(2019));
        assert_eq!(movie.parsed_resolution.as_deref(), Some("2160p"));
        assert!(movie.size.is_some_and(|size| size > 0));
        assert!(movie.link.starts_with("v2:"));
        assert!(!movie.link.contains(&passkey));
        // Nothing in the library has this name, so it is kept with the reason.
        assert!(movie.match_kind.is_none());
        assert!(movie.match_reason.is_some());

        // The feed's health was recorded.
        let feed = RssFeed::get(app.db.pool(), &feed_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(feed.health_status.as_deref(), Some("ok"));

        // The catalogue answers a release search for the same words, marked as from a feed, with a
        // catalogue item to grab and no link a client could read.
        let found = admin
            .query(
                r#"query S($input: SearchSourcesInput!) {
                     searchSources(input: $input) {
                       sources { sourceName fromFeed releases { title fromFeed feedItemId link magnetUri } }
                     }
                   }"#,
                json!({ "input": { "query": "Catalogue Movie" } }),
            )
            .await;
        let sources = found["searchSources"]["sources"]
            .as_array()
            .expect("sources");
        let entry = sources
            .iter()
            .find(|source| source["fromFeed"] == json!(true))
            .unwrap_or_else(|| panic!("a feed result: {found}"));
        let release = &entry["releases"][0];
        assert_eq!(release["fromFeed"], true);
        assert!(release["feedItemId"].is_string());
        assert!(
            release["link"].is_null() && release["magnetUri"].is_null(),
            "{release}"
        );
        assert!(
            !found.to_string().contains(&passkey),
            "the key must not reach a client"
        );

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

#[test]
fn a_source_that_cannot_sign_in_is_recorded_and_alerted_once_then_closed_when_it_recovers() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;

        let created = admin
            .query(
                r#"mutation C($input: CreateSourceInput!) {
                     createSource(input: $input) { success error source { id } }
                   }"#,
                json!({ "input": {
                    "name": "Mock Torznab", "sourceType": "torrent_indexer", "definitionId": "torznab",
                    "enabled": true, "priority": 1, "mediaTypes": "tv", "siteUrl": indexer.base_url,
                    "supportsSearch": true, "supportsTvSearch": true, "supportsMovieSearch": false,
                    "supportsMusicSearch": false, "supportsBookSearch": false,
                    "credentials": "{\"ApiKey\":\"the-wrong-key\"}", "errorCount": 0
                }}),
            )
            .await;
        assert_eq!(created["createSource"]["success"], true, "{created}");
        let source_id = created["createSource"]["source"]["id"]
            .as_str()
            .expect("source id")
            .to_string();

        // Test it twice: a refused key is an authentication failure and one alert, not two.
        for _ in 0..2 {
            admin
                .wait_until(
                    std::time::Duration::from_secs(60),
                    "the source to be tested",
                    "mutation T($id: String!) { testSource(id: $id) { success error } }",
                    json!({ "id": source_id }),
                    |data| data["testSource"]["success"] == json!(false),
                )
                .await;
        }
        let source = Source::get(app.db.pool(), &source_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(source.health_status.as_deref(), Some("auth_expired"));
        assert_eq!(source.health_kind.as_deref(), Some("auth"));
        assert!(source.last_error.is_some());
        assert!(
            source.next_check_at.is_some(),
            "a failing source is checked later, not at once"
        );

        let open = || async {
            Notification::query(app.db.pool())
                .filter(NotificationWhereInput::default())
                .fetch_all()
                .await
                .unwrap()
                .into_iter()
                .filter(|note| note.category == "source_health" && note.resolved_at.is_none())
                .collect::<Vec<_>>()
        };
        let alerts = open().await;
        assert_eq!(alerts.len(), 1, "{alerts:?}");
        assert!(
            alerts[0].title.contains("can't sign in"),
            "{}",
            alerts[0].title
        );
        assert!(!alerts[0].message.contains("the-wrong-key"));

        // Fix the key: the next test passes, the status is ok and the alert is closed.
        let fixed = admin
            .query(
                "mutation U($id: String!, $input: UpdateSourceInput!) { updateSource(id: $id, input: $input) { success error } }",
                json!({ "id": source_id, "input": { "credentials": format!("{{\"ApiKey\":\"{}\"}}", indexer.api_key) } }),
            )
            .await;
        assert_eq!(fixed["updateSource"]["success"], true, "{fixed}");
        admin
            .wait_until(
                std::time::Duration::from_secs(60),
                "the repaired source to pass",
                "mutation T($id: String!) { testSource(id: $id) { success error } }",
                json!({ "id": source_id }),
                |data| data["testSource"]["success"] == json!(true),
            )
            .await;
        let source = Source::get(app.db.pool(), &source_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(source.health_status.as_deref(), Some("ok"));
        assert_eq!(source.error_count, 0);
        assert!(
            open().await.is_empty(),
            "the alert closes when the source recovers"
        );

        app.shutdown().await;
        indexer.shutdown().await;
    });
}
