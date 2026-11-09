//! What the hunt searches for, with a fixed clock: mode filtering, air-time timing, movies, the
//! explanations given when nothing is taken, and the lock that keeps passes from overlapping.
//! Disposable in-memory databases only; nothing here searches a source or grabs a torrent.

use super::*;
use crate::graphql::entities::{
    CreateEpisodeInput, CreateLibraryInput, CreateMovieInput, CreateShowInput, CreateUserInput,
    Library, User,
};
use graphql_orm::DbPool;

async fn database() -> Database {
    let mut db = Database::new(DbPool::connect("sqlite::memory:").await.unwrap());
    crate::services::database::bootstrap_schema(&db)
        .await
        .unwrap();
    // Declared policies without lifecycle hooks, as the other disposable-database tests do.
    db.set_entity_policy(crate::services::graphql::policy::AppEntityPolicy);
    db.set_field_policy(crate::services::graphql::policy::AppFieldPolicy);
    db.set_row_policy(crate::services::graphql::row_policy::OwnershipRowPolicy);
    db
}

async fn library(db: &Database) -> Library {
    let user = User::insert(
        db,
        CreateUserInput {
            username: uuid::Uuid::new_v4().to_string(),
            email: None,
            password_hash: "fixture".to_string(),
            role: "member".to_string(),
            display_name: None,
            avatar_url: None,
            is_active: true,
            last_login_at: None,
        },
    )
    .await
    .unwrap();
    Library::insert(
        db,
        CreateLibraryInput {
            user_id: user.id.clone(),
            name: "fixture library".to_string(),
            path: format!("/fixture/{}", uuid::Uuid::new_v4()),
            library_type: "tv".to_string(),
            icon: None,
            color: None,
            auto_scan: false,
            auto_organize: false,
            auto_resolve_duplicates: None,
            naming_pattern: "fixture".to_string(),
            scan_interval_minutes: 0,
            watch_for_changes: false,
            watch_status: None,
            watch_detail: None,
            watch_status_since: None,
            scanning: false,
            last_scanned_at: None,
            quality_profile_id: None,
        },
    )
    .await
    .unwrap()
}

async fn show(db: &Database, library: &Library, name: &str, mode: AutoDownloadMode) -> Show {
    Show::insert(
        db,
        CreateShowInput {
            locked_fields: None,
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            name: name.to_string(),
            sort_name: None,
            year: None,
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            imdb_id: None,
            overview: None,
            network: None,
            runtime: None,
            genres: Vec::new(),
            poster_url: None,
            backdrop_url: None,
            content_rating: None,
            status: None,
            auto_download: mode != AutoDownloadMode::None,
            auto_download_mode: mode,
            quality_profile_id: None,
            path: None,
        },
    )
    .await
    .unwrap()
}

/// An episode with no file, aired at `stamp` (or undated).
async fn episode(
    db: &Database,
    show: &Show,
    number: i32,
    stamp: Option<DateTime<Utc>>,
    wanted: bool,
    ignored: bool,
) -> Episode {
    Episode::insert(
        db,
        CreateEpisodeInput {
            show_id: show.id.clone(),
            season: 1,
            episode: number,
            absolute_number: None,
            title: None,
            overview: None,
            air_date: stamp.map(|stamp| stamp.format("%Y-%m-%d").to_string()),
            air_stamp: stamp.map(|stamp| stamp.to_rfc3339()),
            runtime: None,
            still_url: None,
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            wanted,
            ignored: Some(ignored),
            media_file_id: None,
        },
    )
    .await
    .unwrap()
}

async fn movie(
    db: &Database,
    library: &Library,
    title: &str,
    year: Option<i32>,
    release_date: Option<&str>,
    monitored: bool,
) -> Movie {
    Movie::insert(
        db,
        CreateMovieInput {
            locked_fields: None,
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            title: title.to_string(),
            sort_title: None,
            original_title: None,
            year,
            tmdb_id: None,
            imdb_id: None,
            overview: None,
            tagline: None,
            runtime: None,
            genres: Vec::new(),
            director: None,
            cast_names: Vec::new(),
            production_countries: Vec::new(),
            spoken_languages: Vec::new(),
            tmdb_rating: None,
            tmdb_vote_count: None,
            poster_url: None,
            backdrop_url: None,
            collection_id: None,
            collection_name: None,
            collection_poster_url: None,
            release_date: release_date.map(ToString::to_string),
            certification: None,
            monitored,
            tmdb_status: None,
            wanted: monitored,
            ignored: None,
            download_status: None,
            has_file: false,
            media_file_id: None,
            quality_profile_id: None,
        },
    )
    .await
    .unwrap()
}

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-08T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

/// Keeps a show from being hunted as a pack, so its candidates are plain episodes: nine ignored
/// episodes aired long ago make one missing episode far less than half of the season.
async fn pad_season(db: &Database, show: &Show) {
    for number in 100..109 {
        episode(
            db,
            show,
            number,
            Some(now() - chrono::Duration::days(400)),
            false,
            true,
        )
        .await;
    }
}

async fn discover(db: &Database) -> Vec<WantedCandidate> {
    discover_candidates(
        db,
        &HuntScope::default(),
        &Blocklist::default(),
        &AutoDownloadSettings::default(),
        now(),
    )
    .await
    .unwrap()
}

fn labels(candidates: &[WantedCandidate]) -> Vec<String> {
    let mut labels: Vec<String> = candidates
        .iter()
        .flat_map(|candidate| {
            std::iter::once(candidate.label.clone())
                .chain(candidate.fallbacks.iter().map(|f| f.label.clone()))
        })
        .collect();
    labels.sort();
    labels
}

// ---------------------------------------------------------------------------
// Mode filtering
// ---------------------------------------------------------------------------

#[tokio::test]
async fn each_mode_decides_which_missing_episodes_are_hunted() {
    let db = database().await;
    let library = library(&db).await;
    let aired = Some(now() - chrono::Duration::days(2));

    let off = show(&db, &library, "Off Show", AutoDownloadMode::None).await;
    episode(&db, &off, 1, aired, true, false).await;
    pad_season(&db, &off).await;

    let wanted_only = show(&db, &library, "Wanted Show", AutoDownloadMode::Wanted).await;
    episode(&db, &wanted_only, 1, aired, true, false).await;
    episode(&db, &wanted_only, 2, aired, false, false).await;
    pad_season(&db, &wanted_only).await;

    let everything = show(&db, &library, "All Show", AutoDownloadMode::All).await;
    episode(&db, &everything, 1, aired, false, false).await;
    pad_season(&db, &everything).await;

    let found = labels(&discover(&db).await);
    assert_eq!(
        found,
        vec![
            "episode 'All Show S01E01'".to_string(),
            "episode 'Wanted Show S01E01'".to_string(),
        ],
        "NONE hunts nothing, WANTED only what is flagged, ALL everything that is missing"
    );
}

#[tokio::test]
async fn all_mode_flags_what_it_hunts_as_wanted_and_ignored_episodes_are_left_alone() {
    let db = database().await;
    let library = library(&db).await;
    let aired = Some(now() - chrono::Duration::days(2));
    let everything = show(&db, &library, "All Show", AutoDownloadMode::All).await;
    let missing = episode(&db, &everything, 1, aired, false, false).await;
    let skipped = episode(&db, &everything, 2, aired, false, true).await;
    pad_season(&db, &everything).await;

    assert_eq!(
        labels(&discover(&db).await),
        vec!["episode 'All Show S01E01'".to_string()]
    );

    let reloaded = Episode::get(db.pool(), &missing.id).await.unwrap().unwrap();
    assert!(
        reloaded.wanted,
        "the stored status matches what the hunt does"
    );
    let reloaded = Episode::get(db.pool(), &skipped.id).await.unwrap().unwrap();
    assert!(!reloaded.wanted, "an ignored episode is never flagged");
}

// ---------------------------------------------------------------------------
// Air time
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_episode_is_searched_only_after_it_airs_and_its_grace_period_passes() {
    let db = database().await;
    let library = library(&db).await;
    let everything = show(&db, &library, "All Show", AutoDownloadMode::All).await;
    // Airs in an hour; aired 30 minutes ago (grace is 60); aired 90 minutes ago; aired ten days ago.
    episode(
        &db,
        &everything,
        1,
        Some(now() + chrono::Duration::hours(1)),
        false,
        false,
    )
    .await;
    episode(
        &db,
        &everything,
        2,
        Some(now() - chrono::Duration::minutes(30)),
        false,
        false,
    )
    .await;
    episode(
        &db,
        &everything,
        3,
        Some(now() - chrono::Duration::minutes(90)),
        false,
        false,
    )
    .await;
    episode(
        &db,
        &everything,
        4,
        Some(now() - chrono::Duration::days(10)),
        false,
        false,
    )
    .await;
    pad_season(&db, &everything).await;

    let candidates = discover(&db).await;
    let by_label: std::collections::HashMap<_, _> = candidates
        .iter()
        .map(|candidate| (candidate.label.clone(), candidate.timing))
        .collect();
    assert!(
        !by_label.contains_key("episode 'All Show S01E01'"),
        "not aired yet"
    );
    assert!(
        !by_label.contains_key("episode 'All Show S01E02'"),
        "inside the grace period"
    );
    assert_eq!(by_label["episode 'All Show S01E03'"], Timing::Fresh);
    assert_eq!(by_label["episode 'All Show S01E04'"], Timing::Back);
    assert_eq!(by_label.len(), 2);
}

#[tokio::test]
async fn the_air_time_pass_keeps_only_episodes_on_the_quick_schedule() {
    let db = database().await;
    let library = library(&db).await;
    let everything = show(&db, &library, "All Show", AutoDownloadMode::All).await;
    episode(
        &db,
        &everything,
        3,
        Some(now() - chrono::Duration::minutes(90)),
        false,
        false,
    )
    .await;
    episode(
        &db,
        &everything,
        4,
        Some(now() - chrono::Duration::days(10)),
        false,
        false,
    )
    .await;
    pad_season(&db, &everything).await;

    let fresh = fresh_targets(discover(&db).await);
    assert_eq!(
        labels(&fresh),
        vec!["episode 'All Show S01E03'".to_string()]
    );
}

#[tokio::test]
async fn a_pack_is_left_to_the_sweep_but_its_fresh_episodes_are_still_searched() {
    let db = database().await;
    let library = library(&db).await;
    let everything = show(&db, &library, "Binge Show", AutoDownloadMode::All).await;
    // Three missing episodes: hunted as a pack, with the episodes as fallbacks.
    for number in 1..=3 {
        episode(
            &db,
            &everything,
            number,
            Some(now() - chrono::Duration::hours(3)),
            false,
            false,
        )
        .await;
    }
    let candidates = discover(&db).await;
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].label, "season pack 'Binge Show S01'");
    assert_eq!(candidates[0].timing, Timing::Back);
    assert_eq!(candidates[0].fallbacks.len(), 3);

    let fresh = fresh_targets(candidates);
    assert_eq!(fresh.len(), 3);
    assert!(
        fresh
            .iter()
            .all(|candidate| candidate.timing == Timing::Fresh)
    );
}

// ---------------------------------------------------------------------------
// Movies
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_monitored_movie_is_hunted_once_its_release_date_has_passed() {
    let db = database().await;
    let library = library(&db).await;
    movie(
        &db,
        &library,
        "Released Movie",
        Some(2026),
        Some("2026-10-01"),
        true,
    )
    .await;
    movie(
        &db,
        &library,
        "Future Movie",
        Some(2027),
        Some("2027-03-01"),
        true,
    )
    .await;
    movie(
        &db,
        &library,
        "Unmonitored Movie",
        Some(2026),
        Some("2026-10-01"),
        false,
    )
    .await;
    movie(&db, &library, "Undated Movie", None, None, true).await;

    let candidates = discover(&db).await;
    assert_eq!(
        labels(&candidates),
        vec![
            "movie 'Released Movie 2026'".to_string(),
            "movie 'Undated Movie'".to_string()
        ]
    );
    let released = candidates
        .iter()
        .find(|candidate| candidate.label.contains("Released"))
        .unwrap();
    assert_eq!(
        released.expectation,
        Expectation::Movie { year: Some(2026) }
    );
    assert_eq!(
        released.timing,
        Timing::Back,
        "no grace or quick window for movies"
    );
}

// ---------------------------------------------------------------------------
// Verifying and explaining
// ---------------------------------------------------------------------------

fn episode_candidate_for(season: i32, episode: i32) -> WantedCandidate {
    WantedCandidate {
        label: format!("episode 'Show S{season:02}E{episode:02}'"),
        library_id: "lib".into(),
        queries: Vec::new(),
        match_title: WantedTitle::work("Show"),
        linkage: CandidateLinkage::default(),
        quality_profile_override_id: None,
        search_key: "k".into(),
        episode_count: 1,
        timing: Timing::Fresh,
        expectation: Expectation::Episode { season, episode },
        feed_name: Some("Show".into()),
        fallbacks: Vec::new(),
    }
}

fn release(title: &str) -> SourceRelease {
    let mut release = SourceRelease::new(title.to_string(), format!("guid:{title}"), now());
    release.seeders = Some(10);
    release
}

#[test]
fn neighbouring_episodes_are_dropped_from_a_search_and_counted() {
    let candidate = episode_candidate_for(1, 2);
    let (kept, wrong, example) = verify_releases(
        &candidate,
        vec![
            release("Show.S01E02.1080p.WEB-GRP"),
            release("Show.S01E03.1080p.WEB-GRP"),
            release("Show.S02E02.1080p.WEB-GRP"),
        ],
        false,
    );
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].title, "Show.S01E02.1080p.WEB-GRP");
    assert_eq!(wrong, 2);
    assert!(example.unwrap().contains("Show.S01E03"));
}

#[test]
fn profile_rejections_are_counted_and_the_commonest_reasons_named() {
    let mut quality = profile::builtin_any_profile();
    quality.name = "HD only".to_string();
    quality.allowed_resolutions = vec!["1080p".to_string()];
    let releases = vec![
        release("Show S01E02 720p WEB-DL x264-A"),
        release("Show S01E02 720p WEB-DL x264-B"),
        release("Show S01E02 480p WEB-DL x264-C"),
        release("Completely Different Thing 1080p"),
        release("Show S01E02 1080p WEB-DL x264-D"),
    ];
    let rejections = explain_profile_rejections(&WantedTitle::work("Show"), &releases, &quality, 1);
    assert_eq!(rejections.too_different, 1);
    assert!(
        rejections
            .too_different_example
            .as_deref()
            .is_some_and(|example| example.contains("Completely Different Thing")),
        "{rejections:?}"
    );
    assert_eq!(rejections.below, 3);
    assert!(
        rejections.reasons[0].contains("720p"),
        "the commonest reason leads: {:?}",
        rejections.reasons
    );
}

#[test]
fn a_show_with_a_similar_name_is_explained_as_a_different_title() {
    // Babylon 5 wanted; the only release is Babylon Berlin.
    let mut candidate = episode_candidate_for(1, 1);
    candidate.match_title = WantedTitle::work("Babylon 5");
    candidate.expectation = Expectation::SeasonPack { season: 1 };
    candidate.episode_count = 22;
    let collected = Collected {
        releases: vec![release(
            "Babylon.Berlin.S01.1080p.10bit.WEBRip.6CH.x265.HEVC-PSA",
        )],
        seen: 1,
        sources: 1,
        ..Default::default()
    };
    let quality = profile::builtin_any_profile();
    let facts = rank_for_candidate(&candidate, &collected, &quality, true).unwrap_err();
    assert_eq!(facts.too_different, 1);
    let why = describe_no_grab(&facts);
    assert!(
        why.contains("1 named after a different title") && why.contains("babylon berlin"),
        "{why}"
    );
}

#[test]
fn every_way_a_search_can_come_up_empty_is_worded() {
    let none = describe_no_grab(&NoGrabFacts {
        sources: 0,
        ..Default::default()
    });
    assert!(none.contains("no source can take this search"));

    let nothing = describe_no_grab(&NoGrabFacts {
        sources: 3,
        ..Default::default()
    });
    assert_eq!(nothing, "no release found on 3 source(s)");

    let broken = describe_no_grab(&NoGrabFacts {
        sources: 2,
        errors: vec!["IPT: login expired".into()],
        ..Default::default()
    });
    assert!(broken.contains("1 source error(s): IPT: login expired"));

    let everything = describe_no_grab(&NoGrabFacts {
        sources: 2,
        seen: 9,
        blocklisted: 2,
        wrong_target: 3,
        wrong_example: Some("'Show.S01E03' is for S01E03, wanted S01E02".into()),
        too_different: 1,
        below_profile: 3,
        profile: "HD only".into(),
        profile_reasons: vec!["resolution 720p is not in the allowed list (1080p)".into()],
        ..Default::default()
    });
    assert!(everything.starts_with("9 release(s) found, none usable: "));
    assert!(everything.contains("2 blocklisted"));
    assert!(everything.contains("3 for something else (e.g. 'Show.S01E03'"));
    assert!(everything.contains("1 named after a different title"));
    assert!(everything.contains("3 below the 'HD only' profile (resolution 720p"));
}

// ---------------------------------------------------------------------------
// Overlap
// ---------------------------------------------------------------------------

#[tokio::test]
async fn passes_never_overlap() {
    use std::sync::atomic::{AtomicU32, Ordering};
    static RUNNING: AtomicU32 = AtomicU32::new(0);
    static OVERLAPS: AtomicU32 = AtomicU32::new(0);

    let tasks: Vec<_> = (0..4)
        .map(|_| {
            tokio::spawn(async {
                let _guard = hunt_lock().await;
                if RUNNING.fetch_add(1, Ordering::SeqCst) > 0 {
                    OVERLAPS.fetch_add(1, Ordering::SeqCst);
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                RUNNING.fetch_sub(1, Ordering::SeqCst);
            })
        })
        .collect();
    for task in tasks {
        task.await.unwrap();
    }
    assert_eq!(OVERLAPS.load(Ordering::SeqCst), 0);
}
