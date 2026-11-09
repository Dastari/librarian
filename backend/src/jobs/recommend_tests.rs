//! The pure parts of recommendations: the hourly budget, the queue, choosing the smallest release at
//! the best quality, and describing the choice.

use super::*;
use crate::services::quality::scoring::{ScoredRelease, parse_release};

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

fn scored(title: &str, size_gb: f64, seeders: Option<i32>) -> ScoredRelease {
    let mut release = SourceRelease::new(
        title.to_string(),
        format!("guid:{title}"),
        at("2026-11-08T12:00:00Z"),
    );
    release.size = Some((size_gb * 1024.0 * 1024.0 * 1024.0) as i64);
    release.seeders = seeders;
    let parsed = parse_release(title);
    ScoredRelease {
        release,
        parsed,
        title_similarity: 1.0,
    }
}

#[test]
fn the_budget_counts_requests_in_the_last_hour_only() {
    let mut budget = SearchBudget::default();
    let start = at("2026-11-08T12:00:00Z");
    assert_eq!(budget.remaining(30, start), 30);
    budget.spend(2, start);
    budget.spend(1, start + Duration::minutes(30));
    assert_eq!(budget.remaining(30, start + Duration::minutes(40)), 27);
    // The first two age out after an hour; the third has not yet.
    assert_eq!(budget.remaining(30, start + Duration::minutes(61)), 29);
    assert_eq!(budget.remaining(30, start + Duration::minutes(91)), 30);
    // Spending past the budget never underflows; zero turns everything off.
    budget.spend(100, start + Duration::minutes(91));
    assert_eq!(budget.remaining(30, start + Duration::minutes(92)), 0);
    assert_eq!(SearchBudget::default().remaining(0, start), 0);
}

#[test]
fn the_same_lookup_is_queued_once() {
    // Drain anything another test left behind.
    while next_request().is_some() {}
    let request = LookupRequest {
        show_id: Some("show".into()),
        season: Some(2),
        ..Default::default()
    };
    assert!(enqueue(request.clone()));
    assert!(!enqueue(request.clone()));
    assert!(enqueue(LookupRequest {
        season: Some(3),
        ..request.clone()
    }));
    assert_eq!(next_request(), Some(request));
    assert!(next_request().is_some());
    assert!(next_request().is_none());
}

#[test]
fn the_smallest_release_at_the_best_quality_wins_over_a_bigger_one_of_the_same_quality() {
    // Ranked best first: the profile put the biggest first on seeders.
    let ranked = vec![
        scored("Show.S02.1080p.WEB-DL.x264-AAA", 40.0, Some(100)),
        scored("Show.S02.1080p.WEB-DL.x265-BBB", 18.0, Some(80)),
        scored("Show.S02.720p.WEB-DL.x264-CCC", 8.0, Some(200)),
    ];
    let (chosen, smaller) = smallest_at_best_quality(&ranked).unwrap();
    assert_eq!(chosen.release.title, "Show.S02.1080p.WEB-DL.x265-BBB");
    assert!(smaller);
}

#[test]
fn a_smaller_release_nobody_seeds_is_no_bargain() {
    let ranked = vec![
        scored("Show.S02.1080p.WEB-DL.x264-AAA", 40.0, Some(100)),
        scored("Show.S02.1080p.WEB-DL.x265-BBB", 18.0, Some(3)),
    ];
    let (chosen, smaller) = smallest_at_best_quality(&ranked).unwrap();
    assert_eq!(chosen.release.title, "Show.S02.1080p.WEB-DL.x264-AAA");
    assert!(!smaller);
}

#[test]
fn feed_releases_with_no_seeder_count_compete_on_size() {
    let ranked = vec![
        scored("Show.S02.1080p.WEB-DL.x264-AAA", 40.0, None),
        scored("Show.S02.1080p.WEB-DL.x265-BBB", 18.0, None),
    ];
    assert_eq!(
        smallest_at_best_quality(&ranked).unwrap().0.release.title,
        "Show.S02.1080p.WEB-DL.x265-BBB"
    );
    assert!(smallest_at_best_quality(&[]).is_none());
}

#[test]
fn quality_reads_as_one_short_line() {
    let label = quality_label(&parse_release(
        "Heat.1995.2160p.UHD.BluRay.x265.HDR.DTS-HD-GRP",
    ));
    assert!(label.starts_with("2160p"), "{label}");
    assert!(label.contains("BLURAY") && label.contains("HDR"), "{label}");
    assert_eq!(
        quality_label(&parse_release("something odd")),
        "quality not stated"
    );
}

#[test]
fn reasons_say_why_it_won_and_what_it_covers() {
    let chosen = scored("Show.S02.1080p.WEB-DL.x265-BBB", 18.0, Some(80));
    let reasons = reasons_for(&chosen, true, "HD", Some("IPT TV"), 8, 10, 3);
    assert!(reasons[0].contains("fits the 'HD' profile"), "{reasons:?}");
    assert!(
        reasons
            .iter()
            .any(|r| r.starts_with("Smallest at that quality (18.0 GB)"))
    );
    assert!(reasons.iter().any(|r| r == "80 seeders"));
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("IPT TV") && r.contains("no search needed"))
    );
    assert!(reasons.iter().any(|r| r == "Covers 8 of 10 missing"));
    // A single missing item says nothing about coverage.
    let one = reasons_for(&chosen, false, "HD", None, 1, 1, 1);
    assert!(!one.iter().any(|r| r.starts_with("Covers")));
}

#[test]
fn a_packs_seasons_come_from_its_title() {
    assert_eq!(
        pack_season_range(&parse_release("Show S01-S03 1080p")),
        Some((1, 3))
    );
    assert_eq!(
        pack_season_range(&parse_release("Show S02 1080p")),
        Some((2, 2))
    );
    assert_eq!(
        pack_season_range(&parse_release("Show Complete Series 1080p")),
        None
    );
}

#[test]
fn a_series_expectation_refuses_a_single_season_and_an_album_one_by_another_artist() {
    let series = Expectation::SeriesPack;
    assert!(
        series
            .mismatch(&parse_release("Show S02 1080p"), true)
            .is_some()
    );
    assert!(
        series
            .mismatch(&parse_release("Show S01-S05 1080p"), true)
            .is_none()
    );
    assert!(
        series
            .mismatch(&parse_release("Show Complete Series 1080p"), true)
            .is_none()
    );
    assert!(
        series
            .mismatch(&parse_release("Show S02E05 1080p"), true)
            .is_some()
    );

    let album = Expectation::Album {
        artist: "Guns N' Roses".into(),
        name: "Appetite for Destruction".into(),
    };
    assert!(
        album
            .title_mismatch(
                "Guns N' Roses - Appetite for Destruction (1987) [FLAC]",
                true
            )
            .is_none()
    );
    assert!(
        album
            .title_mismatch("Xzibit - Appetite for Destruction [FLAC]", true)
            .is_some()
    );
    assert!(
        album
            .title_mismatch("Guns N' Roses - Use Your Illusion I [FLAC]", true)
            .is_some()
    );
    assert!(album.title_mismatch("flac pack", true).is_some());
    assert!(album.title_mismatch("flac pack", false).is_none());
}

// ---------------------------------------------------------------------------
// A show's coverage plan
// ---------------------------------------------------------------------------

fn release(title: &str, seeders: i32, gb: f64) -> SourceRelease {
    let mut release = SourceRelease::new(
        title.to_string(),
        format!("guid:{title}"),
        at("2026-11-08T12:00:00Z"),
    );
    release.seeders = Some(seeders);
    release.size = Some((gb * 1024.0 * 1024.0 * 1024.0) as i64);
    release.magnet_uri = Some(format!("magnet:?xt=urn:btih:{}", "a".repeat(40)));
    release
}

#[tokio::test]
async fn a_show_plan_mixes_a_pack_and_episodes_and_never_takes_another_show() {
    let db = Database::new(
        graphql_orm::DbPool::connect("sqlite::memory:")
            .await
            .unwrap(),
    );
    let settings = AutoDownloadSettings::default();
    let blocklist = Blocklist::default();
    let env = Env {
        db: &db,
        manager: None,
        sources: None,
        blocklist: &blocklist,
        settings: &settings,
        owner_id: "owner".into(),
    };
    let mut missing = Vec::new();
    for season in 1..=2 {
        for episode in 1..=22 {
            missing.push(MissingEpisode {
                id: format!("s{season}e{episode}"),
                season,
                episode,
            });
        }
    }
    let collected = Collected {
        releases: vec![
            // The live regression: well seeded, and another show.
            release(
                "Babylon.Berlin.S01.1080p.10bit.WEBRip.6CH.x265.HEVC-PSA",
                300,
                20.0,
            ),
            release("Babylon.5.S01.1080p.BluRay.x264-GRP", 20, 40.0),
            release("Babylon.5.S02E01.1080p.WEB.h264-GRP", 15, 1.5),
            release("Babylon.5.S02E02.1080p.WEB.h264-GRP", 15, 1.5),
            // Says only that it is complete: used only once its files are read.
            release("Babylon 5 Complete DVD Rip", 50, 60.0),
            // Names no season or episode at all.
            release("Babylon.5.1080p.BluRay", 9, 8.0),
        ],
        seen: 6,
        sources: 1,
        ..Default::default()
    };
    let quality = profile::builtin_any_profile();
    let mut inspected = HashMap::new();
    let evaluation = evaluate_show(
        &env,
        &WantedTitle::work("Babylon 5"),
        &quality,
        &collected,
        &missing,
        &mut inspected,
    )
    .await;
    assert!(
        evaluation
            .options
            .iter()
            .all(|option| !option.scored.release.title.contains("Berlin")),
        "Babylon Berlin must never be planned for Babylon 5"
    );
    let titles: Vec<&str> = evaluation
        .plan
        .releases
        .iter()
        .map(|planned| {
            evaluation.options[planned.option]
                .scored
                .release
                .title
                .as_str()
        })
        .collect();
    assert_eq!(
        titles,
        vec![
            "Babylon.5.S01.1080p.BluRay.x264-GRP",
            "Babylon.5.S02E01.1080p.WEB.h264-GRP",
            "Babylon.5.S02E02.1080p.WEB.h264-GRP",
        ]
    );
    assert_eq!(evaluation.plan.covered(), 24);
    assert_eq!(
        describe_items(&missing, &evaluation.plan.uncovered),
        "S02E03–E22"
    );
    assert_eq!(evaluation.unnumbered, 1);
}
