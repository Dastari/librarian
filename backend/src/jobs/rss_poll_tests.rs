//! RSS polling decisions: when a feed is due, how items are recognised and de-duplicated, and which
//! wanted target an item may be taken for.

use super::*;
use crate::jobs::auto_download::{CandidateLinkage, WantedCandidate};
use crate::jobs::auto_schedule::{Expectation, Timing};
use crate::services::sources::{QueryType, SourceQuery};

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

fn item(title: &str) -> SourceRelease {
    let mut release = SourceRelease::new(
        title.to_string(),
        format!("https://feed.example/{title}"),
        at("2026-11-08T12:00:00Z"),
    );
    release.link = Some(format!("https://feed.example/{title}.torrent"));
    release
}

fn episode_target(show: &str, season: i32, episode: i32) -> WantedCandidate {
    WantedCandidate {
        label: format!("episode '{show} S{season:02}E{episode:02}'"),
        library_id: "lib".into(),
        queries: vec![SourceQuery {
            query_type: QueryType::TvSearch,
            tvdb_id: Some(111),
            ..Default::default()
        }],
        match_title: format!("{show} S{season:02}E{episode:02}"),
        linkage: CandidateLinkage {
            show_id: Some(format!("show:{show}")),
            episode_id: Some(format!("ep:{show}:{season}:{episode}")),
            ..Default::default()
        },
        quality_profile_override_id: None,
        search_key: format!("episode:{show}:{season}:{episode}"),
        episode_count: 1,
        timing: Timing::Fresh,
        expectation: Expectation::Episode { season, episode },
        feed_name: Some(show.to_string()),
        fallbacks: Vec::new(),
    }
}

fn pack_target(show: &str, season: i32) -> WantedCandidate {
    WantedCandidate {
        label: format!("season pack '{show} S{season:02}'"),
        timing: Timing::Back,
        expectation: Expectation::SeasonPack { season },
        episode_count: 8,
        ..episode_target(show, season, 1)
    }
}

fn movie_target(title: &str, year: Option<i32>) -> WantedCandidate {
    WantedCandidate {
        label: format!("movie '{title}'"),
        queries: vec![SourceQuery {
            query_type: QueryType::MovieSearch,
            ..Default::default()
        }],
        match_title: title.to_string(),
        linkage: CandidateLinkage {
            movie_id: Some("movie".into()),
            ..Default::default()
        },
        expectation: Expectation::Movie { year },
        feed_name: Some(title.to_string()),
        ..episode_target(title, 0, 0)
    }
}

fn fates(release: &SourceRelease, targets: &[WantedCandidate]) -> Vec<ItemFate> {
    classify_item(release, targets, &Blocklist::default())
}

// ---------------------------------------------------------------------------
// When a feed is polled
// ---------------------------------------------------------------------------

#[test]
fn a_feed_is_never_polled_faster_than_the_floor() {
    assert_eq!(effective_interval_minutes(1, 0), MIN_POLL_MINUTES);
    assert_eq!(effective_interval_minutes(0, 0), MIN_POLL_MINUTES);
    assert_eq!(effective_interval_minutes(-5, 0), MIN_POLL_MINUTES);
    assert_eq!(effective_interval_minutes(60, 0), 60);
}

#[test]
fn a_failing_feed_backs_off_by_doubling_up_to_a_day() {
    assert_eq!(effective_interval_minutes(30, 1), 60);
    assert_eq!(effective_interval_minutes(30, 2), 120);
    assert_eq!(effective_interval_minutes(30, 3), 240);
    assert_eq!(effective_interval_minutes(30, 20), 24 * 60);
    // A feed that is simply slow by choice is not shortened by the cap.
    assert_eq!(effective_interval_minutes(3000, 0), 3000);
}

#[test]
fn a_feed_is_due_once_its_interval_has_passed() {
    let now = at("2026-11-08T12:00:00Z");
    assert!(feed_due(None, 30, 0, now), "never polled");
    assert!(
        !feed_due(Some("2026-11-08T11:45:00Z"), 30, 0, now),
        "15 minutes into a 30 minute interval"
    );
    assert!(feed_due(Some("2026-11-08T11:30:00Z"), 30, 0, now));
    assert!(
        !feed_due(Some("2026-11-08T11:30:00Z"), 30, 1, now),
        "one failure doubles the wait to an hour"
    );
    assert!(feed_due(Some("2026-11-08T11:00:00Z"), 30, 1, now));
    assert!(
        feed_due(Some("not a time"), 30, 0, now),
        "a garbled stamp never blocks a poll"
    );
}

#[test]
fn a_host_that_asked_to_be_left_alone_is_not_due_until_then() {
    let now = at("2026-11-08T12:00:00Z");
    // `last_polled_at` pushed an hour into the future by a Retry-After.
    assert!(!feed_due(Some("2026-11-08T13:00:00Z"), 30, 1, now));
}

#[test]
fn retry_after_is_read_as_seconds_only() {
    assert_eq!(retry_after_seconds(Some("120")), Some(120));
    assert_eq!(retry_after_seconds(Some(" 30 ")), Some(30));
    assert_eq!(retry_after_seconds(Some("0")), None);
    assert_eq!(
        retry_after_seconds(Some("Wed, 21 Oct 2026 07:28:00 GMT")),
        None
    );
    assert_eq!(retry_after_seconds(None), None);
}

// ---------------------------------------------------------------------------
// De-duplication
// ---------------------------------------------------------------------------

#[test]
fn the_same_item_hashes_the_same_and_a_different_one_does_not() {
    let first = item("Show.S01E01.1080p.WEB-GRP");
    let again = item("Show.S01E01.1080p.WEB-GRP");
    let other = item("Show.S01E02.1080p.WEB-GRP");
    assert_eq!(item_hashes(&first), item_hashes(&again));
    assert_ne!(item_hashes(&first).0, item_hashes(&other).0);
    assert_ne!(item_hashes(&first).1, item_hashes(&other).1);
}

#[test]
fn a_reposted_title_keeps_its_title_hash_across_spelling() {
    let a = item("Show.Name.S01E01.1080p");
    let mut b = item("Show Name S01E01 1080p");
    b.guid = "different guid".into();
    assert_eq!(item_hashes(&a).1, item_hashes(&b).1);
    assert_ne!(item_hashes(&a).0, item_hashes(&b).0);
}

#[test]
fn an_info_hash_is_read_out_of_a_magnet() {
    assert_eq!(
        magnet_info_hash(
            "magnet:?xt=urn:btih:AABBCCDDEEFF00112233445566778899AABBCCDD&dn=Some+Show"
        ),
        Some("aabbccddeeff00112233445566778899aabbccdd".to_string())
    );
    assert_eq!(magnet_info_hash("magnet:?xt=urn:btih:DEADBEEF"), None);
    assert_eq!(magnet_info_hash("https://example.com/a.torrent"), None);
}

// ---------------------------------------------------------------------------
// Naming
// ---------------------------------------------------------------------------

#[test]
fn the_title_prefix_stops_at_the_episode_year_or_quality() {
    assert_eq!(
        title_prefix("Some.Show.S02E05.1080p.WEB.h264-GRP"),
        "some show"
    );
    assert_eq!(title_prefix("Some Show 2x05 720p HDTV"), "some show");
    assert_eq!(title_prefix("Some.Show.S02.COMPLETE.1080p"), "some show");
    assert_eq!(title_prefix("Some.Movie.2019.1080p.BluRay"), "some movie");
    assert_eq!(title_prefix("Some Show Season 2 1080p"), "some show");
    assert_eq!(title_prefix("Some.Show.1080p.WEB"), "some show");
}

#[test]
fn a_show_must_be_named_exactly() {
    assert!(names_match("The.Office.US.S02E05.1080p", "The Office (US)"));
    assert!(names_match("The.Office.S02E05.1080p", "The Office (US)"));
    assert!(
        !names_match("The.Office.US.S02E05.1080p", "The Office"),
        "another show with the same first words is not a match"
    );
    assert!(names_match(
        "Show.Name.2019.S01E01.1080p",
        "Show Name (2019)"
    ));
    assert!(names_match("Dont.Look.Up.2021.1080p.WEB", "Don't Look Up"));
    assert!(!names_match("Show.Names.S01E01.1080p", "Show Name"));
    assert!(!names_match("S01E01.1080p", "Show Name"), "no name at all");
}

// ---------------------------------------------------------------------------
// Which item is taken for which target
// ---------------------------------------------------------------------------

#[test]
fn an_item_for_a_wanted_episode_is_a_candidate_for_it() {
    let targets = vec![
        episode_target("Some Show", 2, 5),
        episode_target("Some Show", 2, 6),
    ];
    assert_eq!(
        fates(&item("Some.Show.S02E05.1080p.WEB.h264-GRP"), &targets),
        vec![ItemFate::Candidate(0)]
    );
}

#[test]
fn an_item_for_an_episode_nobody_wants_is_not_wanted() {
    let targets = vec![episode_target("Some Show", 2, 5)];
    assert_eq!(
        fates(&item("Other.Show.S02E05.1080p.WEB.h264-GRP"), &targets),
        vec![ItemFate::NotWanted],
        "a different show"
    );
}

#[test]
fn the_right_show_with_the_wrong_episode_is_called_what_it_is() {
    let targets = vec![episode_target("Some Show", 2, 5)];
    let result = fates(&item("Some.Show.S02E06.1080p.WEB.h264-GRP"), &targets);
    assert!(
        matches!(result.as_slice(), [ItemFate::WrongTarget(reason)] if reason.contains("S02E06")),
        "got {result:?}"
    );
}

#[test]
fn a_feed_item_that_does_not_say_its_episode_is_refused() {
    let targets = vec![episode_target("Some Show", 2, 5)];
    let result = fates(&item("Some.Show.1080p.WEB.h264-GRP"), &targets);
    assert!(
        matches!(result.as_slice(), [ItemFate::WrongTarget(_)]),
        "got {result:?}"
    );
}

#[test]
fn a_blocklisted_item_is_never_a_candidate() {
    let targets = vec![episode_target("Some Show", 2, 5)];
    let mut release = item("Some.Show.S02E05.1080p.WEB.h264-GRP");
    release.info_hash = Some("ABC123".into());
    let mut blocklist = Blocklist::default();
    blocklist.block_info_hash("abc123");
    assert_eq!(
        classify_item(&release, &targets, &blocklist),
        vec![ItemFate::Blocklisted]
    );
}

#[test]
fn an_id_the_feed_carries_must_agree_with_the_show() {
    let targets = vec![episode_target("Some Show", 2, 5)];
    let mut same = item("Some.Show.S02E05.1080p.WEB.h264-GRP");
    same.tvdb_id = Some(111);
    assert_eq!(fates(&same, &targets), vec![ItemFate::Candidate(0)]);

    let mut other_show = item("Some.Show.S02E05.1080p.WEB.h264-GRP");
    other_show.tvdb_id = Some(999);
    assert_eq!(
        fates(&other_show, &targets),
        vec![ItemFate::NotWanted],
        "same name, a different show's id"
    );
}

#[test]
fn a_movie_must_match_in_year_as_well_as_name() {
    let targets = vec![movie_target("Some Movie", Some(2019))];
    assert_eq!(
        fates(&item("Some.Movie.2019.1080p.WEB-DL.x264-GRP"), &targets),
        vec![ItemFate::Candidate(0)]
    );
    assert!(matches!(
        fates(&item("Some.Movie.1994.1080p.BluRay.x264-GRP"), &targets).as_slice(),
        [ItemFate::WrongTarget(_)]
    ));
    assert!(matches!(
        fates(&item("Some.Movie.1080p.WEB-DL.x264-GRP"), &targets).as_slice(),
        [ItemFate::WrongTarget(_)]
    ));
}

#[test]
fn targets_with_no_feed_name_are_never_matched() {
    let mut track = episode_target("Some Show", 1, 1);
    track.feed_name = None;
    assert_eq!(
        fates(&item("Some.Show.S01E01.1080p.WEB-GRP"), &[track]),
        vec![ItemFate::NotWanted]
    );
}

#[test]
fn a_season_pack_item_matches_the_pack_target_not_an_episode() {
    let targets = flatten_targets(vec![WantedCandidate {
        fallbacks: vec![episode_target("Some Show", 2, 1)],
        ..pack_target("Some Show", 2)
    }]);
    // The pack comes first, the episode after it.
    assert!(matches!(
        targets[0].expectation,
        Expectation::SeasonPack { .. }
    ));
    assert!(matches!(
        targets[1].expectation,
        Expectation::Episode { .. }
    ));
    assert!(targets.iter().all(|target| target.fallbacks.is_empty()));

    assert_eq!(
        fates(&item("Some.Show.S02.COMPLETE.1080p.WEB-DL-GRP"), &targets),
        vec![ItemFate::Candidate(0)]
    );
    assert_eq!(
        fates(&item("Some.Show.S02E01.1080p.WEB-DL-GRP"), &targets),
        vec![ItemFate::Candidate(1)]
    );
}

#[test]
fn a_grabbed_pack_covers_the_episodes_of_its_season_only() {
    let pack = pack_target("Some Show", 2);
    assert!(pack_covers(&pack, &episode_target("Some Show", 2, 3)));
    assert!(!pack_covers(&pack, &episode_target("Some Show", 3, 3)));
    assert!(!pack_covers(&pack, &episode_target("Other Show", 2, 3)));
    assert!(!pack_covers(&episode_target("Some Show", 2, 3), &pack));
}

#[test]
fn one_release_for_two_episodes_is_a_candidate_for_both() {
    let targets = vec![
        episode_target("Some Show", 2, 4),
        episode_target("Some Show", 2, 5),
    ];
    assert_eq!(
        fates(&item("Some.Show.S02E04E05.720p.HDTV.x264-GRP"), &targets),
        vec![ItemFate::Candidate(0), ItemFate::Candidate(1)]
    );
}

#[test]
fn only_http_and_https_feeds_can_be_fetched() {
    assert_eq!(feed_url_problem("https://example.com/rss?key=abc"), None);
    assert_eq!(
        feed_url_problem("http://192.168.1.5:9117/api?t=search"),
        None
    );
    assert!(feed_url_problem("file:///etc/passwd").is_some());
    assert!(feed_url_problem("ftp://example.com/feed").is_some());
    assert!(feed_url_problem("magnet:?xt=urn:btih:abc").is_some());
    assert!(feed_url_problem("nonsense").is_some());
    assert!(feed_url_problem("").is_some());
}
