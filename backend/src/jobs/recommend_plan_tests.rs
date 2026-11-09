//! Coverage plans: packs where they exist, episodes for the gaps, quality before count.

use super::*;
use crate::services::quality::scoring::parse_release;

/// Seasons 1–3 of 22, 22 and 10 episodes, all missing.
fn babylon_missing() -> Vec<MissingEpisode> {
    let mut out = Vec::new();
    for (season, count) in [(1, 22), (2, 22), (3, 10)] {
        for episode in 1..=count {
            out.push(MissingEpisode {
                id: format!("s{season}e{episode}"),
                season,
                episode,
            });
        }
    }
    out
}

fn option(
    title: &str,
    missing: &[MissingEpisode],
    tier: u32,
    seeders: i32,
    gb: i64,
    rank: usize,
) -> PlanOption {
    let parsed = parse_release(title);
    let claim = claim_of(&parsed, title).expect("the title claims something");
    PlanOption {
        covers: claimed_cover(&claim, missing),
        tier,
        seeders: Some(seeders),
        size: Some(gb * 1_000_000_000),
        rank,
        subtitles: false,
    }
}

#[test]
fn claims_are_read_from_names() {
    let claim = |title: &str| claim_of(&parse_release(title), title);
    assert_eq!(
        claim("Show.S01E05.1080p"),
        Some(Claim::Episodes(1, vec![5]))
    );
    assert_eq!(claim("Show.S02.1080p"), Some(Claim::Seasons(2, 2)));
    assert_eq!(claim("Show.S01-S03.1080p"), Some(Claim::Seasons(1, 3)));
    assert_eq!(claim("Show Complete Series 1080p"), Some(Claim::Everything));
    assert_eq!(claim("Babylon 5 Complete DVD Rip"), Some(Claim::Vague));
    assert_eq!(claim("Show 1080p WEB"), None);
    assert_eq!(Claim::Seasons(2, 2).scope(), "season_pack");
    assert_eq!(Claim::Seasons(1, 3).scope(), "series_pack");
    assert_eq!(Claim::Episodes(1, vec![1]).scope(), "episode");
}

#[test]
fn one_complete_series_beats_three_season_packs_of_the_same_quality() {
    let missing = babylon_missing();
    let options = vec![
        option("Babylon.5.S01.1080p", &missing, 0, 40, 30, 0),
        option("Babylon.5.S02.1080p", &missing, 0, 40, 30, 1),
        option("Babylon.5.S03.1080p", &missing, 0, 40, 14, 2),
        option("Babylon.5.Complete.Series.1080p", &missing, 0, 12, 80, 3),
    ];
    let plan = plan_cover(missing.len(), &options);
    assert_eq!(plan.releases.len(), 1);
    assert_eq!(plan.releases[0].option, 3);
    assert_eq!(plan.covered(), 54);
    assert!(plan.uncovered.is_empty());
}

#[test]
fn packs_where_they_exist_and_episodes_for_the_gaps() {
    let missing = babylon_missing();
    let mut options = vec![
        option("Babylon.5.S01.1080p", &missing, 0, 40, 30, 0),
        option("Babylon.5.S02.1080p", &missing, 0, 40, 30, 1),
    ];
    // Season 3 only as single episodes, and not all of them.
    for episode in 1..=8 {
        options.push(option(
            &format!("Babylon.5.S03E{episode:02}.1080p"),
            &missing,
            0,
            10,
            2,
            10 + episode as usize,
        ));
    }
    // A second copy of an episode: the healthier one is taken.
    options.push(option(
        "Babylon.5.S03E01.1080p.OTHER",
        &missing,
        0,
        200,
        2,
        30,
    ));
    let plan = plan_cover(missing.len(), &options);
    assert_eq!(plan.releases.len(), 10);
    assert_eq!(plan.covered(), 52);
    assert_eq!(describe_items(&missing, &plan.uncovered), "S03E09–E10");
    let chosen: Vec<usize> = plan.releases.iter().map(|planned| planned.option).collect();
    assert!(chosen.contains(&10), "the better-seeded S03E01: {chosen:?}");
    assert!(!chosen.contains(&2));
}

#[test]
fn quality_comes_before_fewer_releases() {
    let missing = babylon_missing();
    let options = vec![
        // 720p covers everything in one; 1080p only has season packs for S01 and S02.
        option("Babylon.5.Complete.Series.720p", &missing, 1, 50, 40, 2),
        option("Babylon.5.S01.1080p", &missing, 0, 40, 30, 0),
        option("Babylon.5.S02.1080p", &missing, 0, 40, 30, 1),
    ];
    let plan = plan_cover(missing.len(), &options);
    let chosen: Vec<usize> = plan.releases.iter().map(|planned| planned.option).collect();
    // The 1080p packs stay; the 720p series fills only season 3, and is credited with only that.
    assert_eq!(chosen, vec![1, 2, 0]);
    assert_eq!(plan.releases[2].adds.len(), 10);
    assert!(plan.uncovered.is_empty());
}

#[test]
fn a_smaller_release_wins_between_equals_and_nothing_is_planned_from_nothing() {
    let missing = babylon_missing();
    let options = vec![
        option("Babylon.5.S01.1080p.BIG", &missing, 0, 30, 60, 0),
        option("Babylon.5.S01.1080p.SMALL", &missing, 0, 25, 20, 1),
    ];
    let plan = plan_cover(22, &options);
    assert_eq!(plan.releases.len(), 1);
    assert_eq!(plan.releases[0].option, 1);
    assert_eq!(plan_cover(5, &[]).uncovered, vec![0, 1, 2, 3, 4]);
}

#[test]
fn what_is_left_is_said_plainly() {
    let missing = babylon_missing();
    let items: Vec<usize> = (44..54).chain([0, 1, 2, 7]).collect();
    assert_eq!(
        describe_items(&missing, &items),
        "S01E01–E03, S01E08, all 10 of season 3"
    );
    assert_eq!(seasons_of(&missing, &items), vec![1, 3]);
}

#[test]
fn bundled_subtitles_settle_a_tie_but_never_beat_health() {
    let missing: Vec<MissingEpisode> = babylon_missing()
        .into_iter()
        .filter(|item| item.season == 1)
        .collect();
    let mut plain = option("Show.S01.1080p.WEB-A", &missing, 0, 50, 10, 0);
    let mut subbed = option("Show.S01.1080p.WEB-B", &missing, 0, 50, 12, 1);
    subbed.subtitles = true;
    let plan = plan_cover(missing.len(), &[plain.clone(), subbed.clone()]);
    assert_eq!(plan.releases[0].option, 1, "the subtitled pack wins the tie");
    // Fewer seeders (a worse health band) is not made up for by subtitles.
    subbed.seeders = Some(3);
    plain.seeders = Some(50);
    let plan = plan_cover(missing.len(), &[plain, subbed]);
    assert_eq!(plan.releases[0].option, 0);
}
