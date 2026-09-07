//! The "guide"/discovery surface: the TV schedule, the public showcase, and
//! upcoming/now-playing movie releases.
//!
//! All three have properties that only hold end to end — an ordering that
//! comes from the generated ORM, an authentication boundary that comes from
//! *not* calling a guard, and a cache that decides whether the app reaches
//! for the network. None of that is visible from a unit test, and the last one
//! is the reason this file exists: every assertion here must hold with no
//! outbound network access at all.

mod common;

use common::{GqlClient, TestApp};
use serde_json::{Value, json};

const SCHEDULE_FIELDS: &str = "id tvmazeEpisodeId episodeName season episodeNumber \
    airDate airTime airStamp showName showNetwork showPosterUrl showGenres countryCode";

#[test]
fn schedule_caches_sort_by_air_stamp_and_need_a_session() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;

        // Deliberately inserted out of order, and with one row whose `airDate`
        // ordering disagrees with its `airStamp` ordering — `airDate` is the
        // entity's default sort, so sorting by `airStamp` has to be a real
        // instruction rather than an accident of insertion order.
        for (episode_id, name, air_date, air_stamp) in [
            (301, "Late Night", "2026-03-02", "2026-03-02T23:30:00+00:00"),
            (101, "Early Bird", "2026-03-01", "2026-03-01T06:00:00+00:00"),
            (201, "Prime Time", "2026-03-02", "2026-03-02T20:00:00+00:00"),
        ] {
            create_schedule_row(&mut admin, episode_id, name, air_date, Some(air_stamp)).await;
        }

        let ascending = admin
            .query(
                &format!(
                    "query {{ scheduleCaches(orderBy: [{{ airStamp: ASC }}]) \
                     {{ edges {{ node {{ {SCHEDULE_FIELDS} }} }} pageInfo {{ totalCount }} }} }}"
                ),
                json!({}),
            )
            .await;
        assert_eq!(ascending["scheduleCaches"]["pageInfo"]["totalCount"], 3);
        assert_eq!(
            episode_names(&ascending),
            vec!["Early Bird", "Prime Time", "Late Night"],
        );

        let descending = admin
            .query(
                "query { scheduleCaches(orderBy: [{ airStamp: DESC }]) \
                 { edges { node { episodeName } } } }",
                json!({}),
            )
            .await;
        assert_eq!(
            episode_names(&descending),
            vec!["Late Night", "Prime Time", "Early Bird"],
        );

        // A date-window filter plus the ordering is what the guide screen
        // actually issues.
        let window = admin
            .query(
                r#"query { scheduleCaches(
                     where: { airDate: { gte: "2026-03-02", lte: "2026-03-02" } },
                     orderBy: [{ airStamp: ASC }]
                   ) { edges { node { episodeName } } } }"#,
                json!({}),
            )
            .await;
        assert_eq!(episode_names(&window), vec!["Prime Time", "Late Night"]);

        // The schedule is member-readable, not public.
        let mut guest = app.client();
        let anonymous = guest
            .post(
                "query { scheduleCaches { edges { node { id } } } }",
                json!({}),
            )
            .await;
        assert!(
            !anonymous.errors().is_empty(),
            "the schedule must require a session: {}",
            anonymous.body
        );

        app.shutdown().await;
    });
}

#[test]
fn showcase_artwork_is_public_and_only_serves_vetted_posters() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;

        // Two rows for the same show — the showcase de-duplicates by title.
        create_showcase_row(
            &mut admin,
            401,
            "Showcase One",
            Some("https://static.tvmaze.com/uploads/images/medium_portrait/1/1.jpg"),
        )
        .await;
        create_showcase_row(
            &mut admin,
            402,
            "Showcase One",
            Some("https://static.tvmaze.com/uploads/images/medium_portrait/1/1.jpg"),
        )
        .await;
        create_showcase_row(
            &mut admin,
            403,
            "Showcase Two",
            Some("https://static.tvmaze.com/uploads/images/medium_portrait/2/2.jpg"),
        )
        .await;
        // Rejected: not tvmaze, not https, and not a `medium_` rendition.
        create_showcase_row(
            &mut admin,
            404,
            "Evil Host",
            Some("https://evil.example/uploads/images/medium_portrait/3/3.jpg"),
        )
        .await;
        create_showcase_row(
            &mut admin,
            405,
            "Insecure",
            Some("http://static.tvmaze.com/uploads/images/medium_portrait/4/4.jpg"),
        )
        .await;
        create_showcase_row(
            &mut admin,
            406,
            "Wrong Size",
            Some("https://static.tvmaze.com/uploads/images/original_untouched/5/5.jpg"),
        )
        .await;
        create_showcase_row(&mut admin, 407, "No Poster", None).await;

        // The whole point: no cookies, no bearer token, no origin header.
        let mut anonymous = app.client();
        anonymous.forget_cookies();
        let response = anonymous
            .post(
                "query { showcaseArtwork(limit: 12) { title posterUrl } }",
                json!({}),
            )
            .await;
        assert!(
            response.errors().is_empty(),
            "the showcase is the signed-out landing page and must need no session: {}",
            response.body
        );

        let artwork = response.body["data"]["showcaseArtwork"]
            .as_array()
            .expect("showcaseArtwork should be a list")
            .clone();
        // The showcase has no defined order (the entity sorts by `airDate`,
        // which is identical here), so compare it as a set.
        let mut titles: Vec<&str> = artwork
            .iter()
            .map(|item| item["title"].as_str().unwrap_or_default())
            .collect();
        titles.sort_unstable();
        assert_eq!(
            titles,
            vec!["Showcase One", "Showcase Two"],
            "only de-duplicated, vetted posters belong on a public page"
        );
        for item in &artwork {
            let url = item["posterUrl"].as_str().expect("posterUrl");
            assert!(
                url.starts_with("https://static.tvmaze.com/") && url.contains("/medium_"),
                "a poster URL escaped the allowlist: {url}"
            );
        }

        // `limit` is clamped rather than trusted.
        let clamped = anonymous
            .post(
                "query { showcaseArtwork(limit: 9999) { title } }",
                json!({}),
            )
            .await;
        assert!(clamped.errors().is_empty(), "{}", clamped.body);
        assert!(
            clamped.body["data"]["showcaseArtwork"]
                .as_array()
                .expect("list")
                .len()
                <= 24
        );

        app.shutdown().await;
    });
}

#[test]
fn movie_releases_validate_their_arguments_before_reaching_for_the_network() {
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;

        // No TMDB API key is configured in a test database, so *any* request
        // that gets past validation and misses the cache fails at client
        // construction — which is also the proof that these assertions never
        // touch the network.
        for variables in [
            json!({ "region": "invalid", "page": 1 }),
            json!({ "region": "GBR", "page": 1 }),
            json!({ "region": null, "page": 0 }),
            json!({ "region": null, "page": 501 }),
        ] {
            let response = admin
                .post(
                    "query M($region: String, $page: Int!) { \
                       movieReleases(kind: UPCOMING, region: $region, page: $page) { title } }",
                    variables.clone(),
                )
                .await;
            assert!(
                !response.errors().is_empty(),
                "{variables} should have been rejected, got {}",
                response.body
            );
            assert!(response.body["data"]["movieReleases"].is_null());
            // Worth knowing: the resolver's own message ("Region must be a
            // two-letter ISO country code") never reaches the client. Only
            // errors carrying an explicit `code` survive
            // `sanitize_execution_errors`, so a user-fixable argument mistake
            // is reported — and logged — as an internal fault.
            assert_eq!(
                response.body["errors"][0]["extensions"]["code"], "INTERNAL_ERROR",
                "argument errors are currently masked; update this if they gain a public code"
            );
        }

        // A fresh cache row is served verbatim, without a provider call.
        let payload = json!([
            {
                "provider": "tmdb",
                "provider_id": 603,
                "title": "Cached Feature",
                "original_title": null,
                "year": 2026,
                "release_date": "2026-04-01",
                "overview": "From the cache, not the network.",
                "poster_url": null,
                "backdrop_url": null,
                "imdb_id": null,
                "vote_average": 7.5,
                "popularity": 12.0
            }
        ]);
        create_metadata_cache(
            &mut admin,
            "tmdb",
            "MovieReleases",
            // `kind=<endpoint>;region=<region|global>;page=<n>`
            "kind=upcoming;region=GB;page=2",
            &payload,
        )
        .await;

        let cached = admin
            .query(
                r#"query { movieReleases(kind: UPCOMING, region: "gb", page: 2)
                     { providerId title year overview } }"#,
                json!({}),
            )
            .await;
        let results = cached["movieReleases"].as_array().expect("a list");
        assert_eq!(results.len(), 1, "{cached}");
        assert_eq!(results[0]["title"], "Cached Feature");
        assert_eq!(results[0]["providerId"], 603);

        // The cache key includes the kind, the region and the page, so a
        // neighbouring request is a miss — and a miss with no API key is an
        // error, never a silent network call.
        for query in [
            r#"query { movieReleases(kind: NOW_PLAYING, region: "GB", page: 2) { title } }"#,
            r#"query { movieReleases(kind: UPCOMING, region: "US", page: 2) { title } }"#,
            r#"query { movieReleases(kind: UPCOMING, region: "GB", page: 3) { title } }"#,
            r#"query { movieReleases(kind: UPCOMING, page: 2) { title } }"#,
        ] {
            let miss = admin.post(query, json!({})).await;
            assert!(
                !miss.errors().is_empty(),
                "{query} should have missed the cache, but it returned {}",
                miss.body
            );
        }

        // Members may read the guide; anonymous callers may not.
        let mut guest = app.client();
        let anonymous = guest
            .post(
                r#"query { movieReleases(kind: UPCOMING, page: 1) { title } }"#,
                json!({}),
            )
            .await;
        assert!(
            !anonymous.errors().is_empty(),
            "movieReleases must require a session: {}",
            anonymous.body
        );

        app.shutdown().await;
    });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn episode_names(data: &Value) -> Vec<String> {
    data["scheduleCaches"]["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|edge| {
            edge["node"]["episodeName"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

async fn create_schedule_row(
    admin: &mut GqlClient,
    tvmaze_episode_id: i64,
    episode_name: &str,
    air_date: &str,
    air_stamp: Option<&str>,
) {
    create_schedule_row_full(
        admin,
        tvmaze_episode_id,
        episode_name,
        air_date,
        air_stamp,
        "Scheduled Show",
        None,
    )
    .await;
}

async fn create_showcase_row(
    admin: &mut GqlClient,
    tvmaze_episode_id: i64,
    show_name: &str,
    poster_url: Option<&str>,
) {
    create_schedule_row_full(
        admin,
        tvmaze_episode_id,
        "Any Episode",
        "2026-03-01",
        None,
        show_name,
        poster_url,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn create_schedule_row_full(
    admin: &mut GqlClient,
    tvmaze_episode_id: i64,
    episode_name: &str,
    air_date: &str,
    air_stamp: Option<&str>,
    show_name: &str,
    show_poster_url: Option<&str>,
) {
    let created = admin
        .query(
            r#"mutation C($input: CreateScheduleCacheInput!) {
                 createScheduleCache(input: $input) { success error scheduleCache { id } }
               }"#,
            json!({ "input": {
                "tvmazeEpisodeId": tvmaze_episode_id,
                "episodeName": episode_name,
                "season": 1,
                "episodeNumber": 1,
                "airDate": air_date,
                "airStamp": air_stamp,
                "tvmazeShowId": tvmaze_episode_id / 100,
                "showName": show_name,
                "showPosterUrl": show_poster_url,
                "showGenres": ["Drama"],
                "countryCode": "US"
            }}),
        )
        .await;
    assert_eq!(created["createScheduleCache"]["success"], true, "{created}");
}

async fn create_metadata_cache(
    admin: &mut GqlClient,
    provider: &str,
    operation: &str,
    cache_key: &str,
    payload: &Value,
) {
    let created = admin
        .query(
            r#"mutation C($input: CreateMetadataCacheInput!) {
                 createMetadataCache(input: $input) { success error metadataCache { id } }
               }"#,
            json!({ "input": {
                "provider": provider,
                "operation": operation,
                "cacheKey": cache_key,
                "payload": payload.to_string(),
                "payloadVersion": 1,
                "fetchedAt": chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
            }}),
        )
        .await;
    assert_eq!(created["createMetadataCache"]["success"], true, "{created}");
}
