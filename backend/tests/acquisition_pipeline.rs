//! End-to-end acquisition: indexer search -> quality-profile selection ->
//! grab -> real download -> import into the library.
//!
//! Everything runs against [`common::mock_indexer::MockIndexer`], which is a
//! Torznab API, a BitTorrent tracker and a seeding librqbit session behind one
//! loopback port. No network, no fixtures that can drift from the code: the
//! magnets the mock advertises really resolve, really transfer, and the app
//! really imports them.
//!
//! The pure decision functions (`should_hunt_season_pack`, `filter_and_rank`,
//! `filter_import_candidates`, `decide_seeding_action`, …) are unit-tested in
//! place. What only exists end to end — and what this file covers — is the
//! wiring between them: that a profile rule written through GraphQL actually
//! changes which release is grabbed, that the grab lands the right linkage on
//! the `Torrent` row, and that a completed transfer becomes an `Episode` with
//! a file in the right place.

mod common;

use std::time::Duration;

use common::mock_indexer::{MockIndexer, PayloadFile};
use common::{GqlClient, TestApp, TestAppOptions};
use serde_json::{Value, json};

/// Generous enough for a loaded machine, short enough to fail fast.
const SETTLE: Duration = Duration::from_secs(60);

/// The seeding policy only runs on a 15s maintenance tick, so its test waits
/// noticeably longer than the rest.
const SEEDING_SETTLE: Duration = Duration::from_secs(120);

// ---------------------------------------------------------------------------
// Search and selection
// ---------------------------------------------------------------------------

#[test]
fn quality_profile_rules_decide_which_release_is_grabbed() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;

        // One search returns all of these; only one may survive.
        let wrong_resolution = indexer
            .add_release(
                "Sweep Show S01E01 720p WEB-DL x264-GRP",
                "e01.720p.mkv",
                32 * 1024,
            )
            .await;
        let wrong_language = indexer
            .add_release(
                "Sweep Show S01E01 GERMAN 1080p WEB-DL x264-GRP",
                "e01.de.mkv",
                32 * 1024,
            )
            .await;
        indexer
            .add_release(
                "Sweep Show S01E01 1080p WEB-DL x264-LOWSEED",
                "e01.lowseed.mkv",
                32 * 1024,
            )
            .await;
        let too_few_seeders = indexer.amend_last(|release| release.seeders = 1);
        let plain = indexer
            .add_release(
                "Sweep Show S01E01 1080p WEB-DL x264-GRP",
                "e01.mkv",
                32 * 1024,
            )
            .await;
        // Same everything, but a PROPER — `preferProperRepack` must lift it
        // above `plain` even though `plain` has more seeders.
        indexer
            .add_release(
                "Sweep Show S01E01 PROPER 1080p WEB-DL x264-GRP",
                "e01.proper.mkv",
                32 * 1024,
            )
            .await;
        let proper = indexer.amend_last(|release| release.seeders = 9);

        let app = start_app(&TestAppOptions {
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let world = World::build(&app, &mut admin, &indexer).await;

        let profile = create_profile(
            &mut admin,
            json!({
                "name": "Strict 1080p English",
                "allowedResolutions": ["1080p"],
                "preferredLanguages": ["en"],
                "requireLanguageMatch": true,
                "minSeeders": 5,
                "preferProperRepack": true,
                "allowSeasonPacks": false,
                "isDefault": false
            }),
        )
        .await;
        let show = world
            .create_show(&mut admin, "Sweep Show", "WANTED", Some(&profile))
            .await;
        world
            .create_episode(&mut admin, &show, 1, 1, "Pilot", true)
            .await;

        let result = search_missing(&mut admin, &show).await;
        assert_eq!(result["success"], true, "{result}");
        assert_eq!(result["queued"], 1, "exactly one release should be grabbed");

        let grabbed = torrent_by_hash(&mut admin, &proper.info_hash).await;
        assert!(
            !grabbed.is_null(),
            "the PROPER release should have been grabbed, not one of the rejects"
        );
        for rejected in [&wrong_resolution, &wrong_language, &too_few_seeders, &plain] {
            assert!(
                torrent_by_hash(&mut admin, &rejected.info_hash)
                    .await
                    .is_null(),
                "'{}' should not have been grabbed",
                rejected.title
            );
        }

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

#[test]
fn a_grab_stamps_the_wanted_linkage_and_the_indexer_it_came_from() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;
        // The rest of this file lets the mock advertise `.torrent` URLs, which
        // keeps `add_torrent` off the peer-handshake path. This one test takes
        // the magnet route instead, so the shape most real indexers emit stays
        // covered end to end: the magnet is parsed out of the RSS and its
        // metadata resolved from a peer found through the magnet's tracker.
        indexer.advertise_magnets();
        let release = indexer
            .add_release(
                "Linked Show S03E07 1080p WEB-DL x264-GRP",
                "e07.mkv",
                32 * 1024,
            )
            .await;

        let app = start_app(&TestAppOptions {
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let world = World::build(&app, &mut admin, &indexer).await;
        // One missing episode out of one aired clears the season-pack ratio,
        // so packs are switched off to pin this on the per-episode path.
        let profile = create_profile(&mut admin, json!({ "allowSeasonPacks": false })).await;
        let show = world
            .create_show(&mut admin, "Linked Show", "WANTED", Some(&profile))
            .await;
        let episode = world
            .create_episode(&mut admin, &show, 3, 7, "The One", true)
            .await;

        let result = search_missing(&mut admin, &show).await;
        assert_eq!(result["queued"], 1, "{result}");

        let torrent = torrent_by_hash(&mut admin, &release.info_hash).await;
        assert_eq!(
            torrent["episodeId"], episode,
            "the grab must name the episode"
        );
        assert_eq!(torrent["showId"], show);
        assert!(
            torrent["season"].is_null(),
            "a single-episode grab is not a season pack, so `season` stays null"
        );
        assert_eq!(
            torrent["sourceIndexerId"], world.source_id,
            "the row must record which indexer supplied the release"
        );
        assert_eq!(torrent["libraryId"], world.library_id);
        assert_eq!(
            torrent["magnetUri"], release.magnet,
            "the row should record the magnet the release advertised"
        );
        assert!(
            torrent["postProcessStatus"].is_null(),
            "post-processing has not run yet, so the status is still unset"
        );

        // A second hunt must not grab it again: the in-flight torrent makes
        // the episode a resolved target.
        let again = search_missing(&mut admin, &show).await;
        assert_eq!(again["queued"], 0, "{again}");

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

#[test]
fn season_packs_are_hunted_first_and_stamp_the_season() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;
        // The pack is what the S04 query should find and prefer.
        indexer
            .add_release_with_files(
                "Pack Show S04 1080p WEB-DL x264-GRP",
                Some("Pack.Show.S04.1080p.WEB-DL.x264-GRP"),
                &[
                    PayloadFile::new("Pack.Show.S04E01.mkv", 24 * 1024),
                    PayloadFile::new("Pack.Show.S04E02.mkv", 24 * 1024),
                    PayloadFile::new("Pack.Show.S04E03.mkv", 24 * 1024),
                ],
            )
            .await;
        let pack = indexer.amend_last(|release| release.seeders = 40);

        let app = start_app(&TestAppOptions {
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let world = World::build(&app, &mut admin, &indexer).await;
        let show = world
            .create_show(&mut admin, "Pack Show", "WANTED", None)
            .await;
        // Three missing episodes clears `SEASON_PACK_MIN_MISSING`.
        for episode in 1..=3 {
            world
                .create_episode(&mut admin, &show, 4, episode, &format!("E{episode}"), true)
                .await;
        }

        let result = search_missing(&mut admin, &show).await;
        assert_eq!(
            result["queued"], 1,
            "one pack, not three episodes: {result}"
        );

        let torrent = torrent_by_hash(&mut admin, &pack.info_hash).await;
        assert_eq!(torrent["showId"], show);
        assert_eq!(
            torrent["season"], 4,
            "a pack grab is stamped with the season it satisfies"
        );
        assert!(
            torrent["episodeId"].is_null(),
            "a pack is not linked to any single episode"
        );

        // The pack query is the padded `S04` form, and it is searched before
        // any per-episode query.
        assert!(
            indexer.searches_matching("Pack Show S04") > 0,
            "expected a season-pack query, saw {:?}",
            indexer.search_log()
        );
        assert_eq!(
            indexer.searches_matching("S04E01"),
            0,
            "the per-episode fallback must not run once the pack is grabbed"
        );

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

#[test]
fn a_profile_that_forbids_packs_falls_back_to_per_episode_grabs() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;
        // Only a pack is on offer; the profile refuses packs, so the hunt must
        // fall through to the per-episode searches and grab nothing.
        indexer
            .add_release_with_files(
                "Solo Show S02 1080p WEB-DL x264-GRP",
                Some("Solo.Show.S02.1080p.WEB-DL.x264-GRP"),
                &[
                    PayloadFile::new("Solo.Show.S02E01.mkv", 24 * 1024),
                    PayloadFile::new("Solo.Show.S02E02.mkv", 24 * 1024),
                    PayloadFile::new("Solo.Show.S02E03.mkv", 24 * 1024),
                ],
            )
            .await;

        let app = start_app(&TestAppOptions {
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let world = World::build(&app, &mut admin, &indexer).await;
        let profile = create_profile(
            &mut admin,
            json!({
                "name": "No packs",
                "allowSeasonPacks": false,
                "minSeeders": 1,
                "isDefault": false
            }),
        )
        .await;
        let show = world
            .create_show(&mut admin, "Solo Show", "WANTED", Some(&profile))
            .await;
        for episode in 1..=3 {
            world
                .create_episode(&mut admin, &show, 2, episode, &format!("E{episode}"), true)
                .await;
        }

        let result = search_missing(&mut admin, &show).await;
        assert_eq!(
            result["queued"], 0,
            "the only release is a pack the profile forbids: {result}"
        );
        assert_eq!(
            result["searched"], 3,
            "three per-episode searches, no pack search: {result}"
        );
        for episode in 1..=3 {
            assert!(
                indexer.searches_matching(&format!("S02E0{episode}")) > 0,
                "expected a query for episode {episode}, saw {:?}",
                indexer.search_log()
            );
        }

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

#[test]
fn auto_download_mode_and_the_blocklist_gate_the_hunt() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;
        let release = indexer
            .add_release(
                "Gated Show S01E01 1080p WEB-DL x264-GRP",
                "e01.mkv",
                32 * 1024,
            )
            .await;

        let app = start_app(&TestAppOptions {
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let world = World::build(&app, &mut admin, &indexer).await;

        // NONE: not a candidate at all, whatever `wanted` says.
        let muted = world
            .create_show(&mut admin, "Gated Show", "NONE", None)
            .await;
        world
            .create_episode(&mut admin, &muted, 1, 1, "Pilot", true)
            .await;
        // `World::build` already ran one search through `testSource`.
        let baseline = indexer.total_searches();
        let none_result = search_missing(&mut admin, &muted).await;
        assert_eq!(none_result["searched"], 0, "{none_result}");
        assert_eq!(none_result["queued"], 0);
        assert_eq!(
            indexer.total_searches(),
            baseline,
            "a NONE show must not reach the indexer at all"
        );

        // Blocklisting the only release means WANTED finds nothing.
        admin
            .query(
                r#"mutation B($hash: String!) {
                     createReleaseBlocklist(input: {
                       infoHash: $hash, title: "Gated", reason: "import_failed"
                     }) { success error }
                   }"#,
                json!({ "hash": release.info_hash }),
            )
            .await;

        let no_packs = create_profile(&mut admin, json!({ "allowSeasonPacks": false })).await;
        let hunted = world
            .create_show(&mut admin, "Gated Show Two", "WANTED", Some(&no_packs))
            .await;
        world
            .create_episode(&mut admin, &hunted, 1, 1, "Pilot", true)
            .await;
        let blocked = search_missing(&mut admin, &hunted).await;
        assert_eq!(blocked["searched"], 1, "{blocked}");
        assert_eq!(
            blocked["queued"], 0,
            "the only candidate release is blocklisted: {blocked}"
        );

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

#[test]
fn the_search_backoff_stops_a_second_pass_but_a_manual_search_ignores_it() {
    common::run_app_test(|| async {
        // An indexer with nothing to offer: the hunt searches, grabs nothing,
        // and the candidate therefore survives into the second pass. (A grab
        // would resolve the target and remove it, hiding the backoff.)
        let indexer = MockIndexer::start().await;

        let app = start_app(&TestAppOptions {
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let world = World::build(&app, &mut admin, &indexer).await;
        let profile = create_profile(&mut admin, json!({ "allowSeasonPacks": false })).await;
        let show = world
            .create_show(&mut admin, "Backoff Show", "WANTED", Some(&profile))
            .await;
        let episode = world
            .create_episode(&mut admin, &show, 1, 1, "Pilot", true)
            .await;

        let first = trigger_auto_download(&mut admin, &world.library_id).await;
        assert_eq!(first["candidatesConsidered"], 1, "{first}");
        assert_eq!(first["searched"], 1, "{first}");
        assert_eq!(first["grabbed"], 0, "{first}");

        // Within the 6h window the candidate is still discovered, but not
        // searched again — that is the whole point of the backoff.
        let second = trigger_auto_download(&mut admin, &world.library_id).await;
        assert_eq!(
            second["candidatesConsidered"], 1,
            "discovery runs before the backoff check: {second}"
        );
        assert_eq!(second["searched"], 0, "{second}");

        // The backoff lives in one app_settings row keyed per target.
        let state = admin
            .query(
                r#"query { appSettings(where: { key: { eq: "auto_download.search_state" } })
                     { edges { node { key value category } } } }"#,
                json!({}),
            )
            .await;
        let node = &state["appSettings"]["edges"][0]["node"];
        assert_eq!(node["category"], "auto_download");
        let recorded: Value = serde_json::from_str(
            node["value"]
                .as_str()
                .expect("search state is a JSON string"),
        )
        .expect("search state should be a JSON object");
        assert!(
            recorded.get(format!("episode:{episode}")).is_some(),
            "expected a backoff entry for the episode, got {recorded}"
        );

        // `searchMissing` is the admin pressing "search now", so it always
        // searches regardless of when the last pass ran.
        let manual = search_missing(&mut admin, &show).await;
        assert_eq!(manual["searched"], 1, "{manual}");

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

// ---------------------------------------------------------------------------
// Download, import and cleanup
// ---------------------------------------------------------------------------

#[test]
fn a_completed_pack_is_imported_into_the_library_and_samples_are_skipped() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;
        indexer
            .add_release_with_files(
                "Import Show S01 1080p WEB-DL x264-GRP",
                Some("Import.Show.S01.1080p.WEB-DL.x264-GRP"),
                &[
                    PayloadFile::new("Import.Show.S01E01.1080p.mkv", 48 * 1024),
                    PayloadFile::new("Import.Show.S01E02.1080p.mkv", 48 * 1024),
                    // Smaller than the real episodes and named `sample`, which
                    // is exactly what the import filter drops.
                    PayloadFile::new("Import.Show.S01E01.sample.mkv", 4 * 1024),
                    // Not a media extension: never a candidate.
                    PayloadFile::new("Import.Show.S01.nfo", 512),
                ],
            )
            .await;
        let pack = indexer.amend_last(|release| release.seeders = 30);

        let app = start_app(&TestAppOptions {
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let world = World::build(&app, &mut admin, &indexer).await;
        let show = world
            .create_show(&mut admin, "Import Show", "WANTED", None)
            .await;
        let first = world
            .create_episode(&mut admin, &show, 1, 1, "First Steps", true)
            .await;
        let second = world
            .create_episode(&mut admin, &show, 1, 2, "Second Wind", true)
            .await;
        world
            .create_episode(&mut admin, &show, 1, 3, "Never Aired Here", true)
            .await;

        assert_eq!(search_missing(&mut admin, &show).await["queued"], 1);

        let torrent = wait_for_post_process(&mut admin, &pack.info_hash).await;
        assert_eq!(
            torrent["postProcessStatus"], "completed",
            "both episodes should import cleanly: {torrent}"
        );

        // The seeded tv naming pattern is
        // `{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}`.
        for (episode_id, expected) in [
            (
                &first,
                "Import Show/Season 01/Import Show - S01E01 - First Steps.mkv",
            ),
            (
                &second,
                "Import Show/Season 01/Import Show - S01E02 - Second Wind.mkv",
            ),
        ] {
            let placed = std::path::Path::new(&world.library_path).join(expected);
            assert!(
                placed.is_file(),
                "expected an imported file at {}; library contains {:?}",
                placed.display(),
                list_tree(&world.library_path)
            );

            let episode = episode_by_id(&mut admin, episode_id).await;
            let media_file_id = episode["mediaFileId"]
                .as_str()
                .unwrap_or_else(|| panic!("episode should be linked to a media file: {episode}"));
            assert_eq!(
                episode["wanted"], false,
                "an episode with a file is no longer wanted"
            );

            // The MediaFile row is rewritten to the library path at import,
            // so it never points at the (deletable) download payload.
            let media_file = media_file_by_id(&mut admin, media_file_id).await;
            assert_eq!(media_file["path"], placed.to_string_lossy().as_ref());
            assert_eq!(media_file["episodeId"], *episode_id);
            assert_eq!(media_file["libraryId"], world.library_id);
        }

        // The sample was filtered, not imported and not failed.
        assert!(
            !std::path::Path::new(&world.library_path)
                .join("Import Show/Season 01/Import Show - S01E01 - First Steps.sample.mkv")
                .exists()
        );
        assert_eq!(
            list_tree(&world.library_path).len(),
            2,
            "only the two real episodes belong in the library, found {:?}",
            list_tree(&world.library_path)
        );

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

#[test]
fn a_zip_release_is_extracted_and_its_staging_directory_is_cleaned_up() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;
        indexer
            .add_zip_release(
                "Zipped Show S05E09 1080p WEB-DL x264-GRP",
                "Zipped.Show.S05E09.1080p.WEB-DL.x264-GRP",
                "Zipped.Show.S05E09.zip",
                &[("Zipped.Show.S05E09.1080p.mkv", 40 * 1024)],
            )
            .await;
        let release = indexer.amend_last(|release| release.seeders = 30);

        let app = start_app(&TestAppOptions {
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let world = World::build(&app, &mut admin, &indexer).await;
        let profile = create_profile(&mut admin, json!({ "allowSeasonPacks": false })).await;
        let show = world
            .create_show(&mut admin, "Zipped Show", "WANTED", Some(&profile))
            .await;
        let episode = world
            .create_episode(&mut admin, &show, 5, 9, "Unpacked", true)
            .await;

        assert_eq!(search_missing(&mut admin, &show).await["queued"], 1);

        let torrent = wait_for_post_process(&mut admin, &release.info_hash).await;
        assert_eq!(
            torrent["postProcessStatus"], "completed",
            "the archive should unpack and import: {torrent}"
        );

        let placed = std::path::Path::new(&world.library_path)
            .join("Zipped Show/Season 05/Zipped Show - S05E09 - Unpacked.mkv");
        assert!(
            placed.is_file(),
            "expected the unpacked episode at {}; library contains {:?}",
            placed.display(),
            list_tree(&world.library_path)
        );
        assert_eq!(episode_by_id(&mut admin, &episode).await["wanted"], false);

        // Extraction stages into `.librarian-extract` *inside the download
        // directory*, and must clean up after itself — otherwise every
        // archived release silently doubles its disk cost.
        let save_path = torrent["savePath"].as_str().expect("savePath");
        let staging = std::path::Path::new(save_path).join(".librarian-extract");
        assert!(
            list_tree(&staging).is_empty(),
            "extraction staging {} should have been emptied, it still holds {:?}",
            staging.display(),
            list_tree(&staging)
        );
        // The archive itself is left alone so seeding can continue.
        assert!(
            std::path::Path::new(save_path)
                .join("Zipped.Show.S05E09.zip")
                .is_file()
                || std::path::Path::new(save_path)
                    .join("Zipped.Show.S05E09.1080p.WEB-DL.x264-GRP/Zipped.Show.S05E09.zip")
                    .is_file(),
            "the seeded archive must survive the import, saw {:?}",
            list_tree(save_path)
        );

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

#[test]
fn a_torrent_that_matches_nothing_says_why() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;
        // Linked to the show, but the season/episode in the payload does not
        // exist — the interesting failure, because the linkage means the
        // generic matcher is deliberately never consulted.
        indexer
            .add_release_with_files(
                "Ghost Show S09 1080p WEB-DL x264-GRP",
                Some("Ghost.Show.S09.1080p.WEB-DL.x264-GRP"),
                &[
                    PayloadFile::new("Ghost.Show.S09E01.1080p.mkv", 40 * 1024),
                    PayloadFile::new("Ghost.Show.S09E02.1080p.mkv", 40 * 1024),
                    PayloadFile::new("Ghost.Show.S09E03.1080p.mkv", 40 * 1024),
                ],
            )
            .await;
        let release = indexer.amend_last(|release| release.seeders = 30);

        let app = start_app(&TestAppOptions {
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let world = World::build(&app, &mut admin, &indexer).await;
        let show = world
            .create_show(&mut admin, "Ghost Show", "WANTED", None)
            .await;
        // Three wanted episodes make the pack a candidate, but they are all in
        // a season the release does not contain.
        for episode in 1..=3 {
            world
                .create_episode(&mut admin, &show, 9, episode, &format!("E{episode}"), true)
                .await;
        }
        assert_eq!(search_missing(&mut admin, &show).await["queued"], 1);

        // Delete them once the pack is in flight, so what arrives has nothing
        // to match against while the torrent stays linked to the show.
        for episode_id in episode_ids(&mut admin, &show).await {
            admin
                .query(
                    "mutation D($id: String!) { deleteEpisode(id: $id) { success error } }",
                    json!({ "id": episode_id }),
                )
                .await;
        }

        let torrent = wait_for_post_process(&mut admin, &release.info_hash).await;
        assert_eq!(
            torrent["postProcessStatus"], "unmatched",
            "nothing was imported, so the torrent is unmatched: {torrent}"
        );
        let error = torrent["postProcessError"]
            .as_str()
            .expect("an unmatched torrent must explain itself");
        assert!(
            error.contains("did not match"),
            "the error should name the files that did not match, got: {error}"
        );
        assert!(
            error.contains("Ghost.Show.S09E01.1080p.mkv"),
            "the error should name the offending file, got: {error}"
        );
        assert!(
            error.contains("Ghost Show"),
            "the error should name the target it was linked to, got: {error}"
        );
        assert!(
            list_tree(&world.library_path).is_empty(),
            "nothing may be placed in the library, found {:?}",
            list_tree(&world.library_path)
        );

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

#[test]
fn remove_after_import_deletes_the_payload_and_spares_the_library_file() {
    common::run_app_test(|| async {
        let indexer = MockIndexer::start().await;
        let release = indexer
            .add_release(
                "Seeded Show S02E02 1080p WEB-DL x264-GRP",
                "Seeded.Show.S02E02.1080p.mkv",
                64 * 1024,
            )
            .await;
        indexer.amend_last(|release| release.seeders = 30);

        let app = start_app(&TestAppOptions {
            with_auto_download: true,
            // Stop seeding at a 0.25 share ratio and, once the import is
            // clean, delete the payload. The bar is low on purpose: the point
            // is the policy wiring, not the exact byte accounting.
            torrent_seed_ratio_limit: 0.25,
            torrent_remove_after_import: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let world = World::build(&app, &mut admin, &indexer).await;
        let profile = create_profile(&mut admin, json!({ "allowSeasonPacks": false })).await;
        let show = world
            .create_show(&mut admin, "Seeded Show", "WANTED", Some(&profile))
            .await;
        let episode = world
            .create_episode(&mut admin, &show, 2, 2, "Ratio", true)
            .await;

        assert_eq!(search_missing(&mut admin, &show).await["queued"], 1);
        let torrent = wait_for_post_process(&mut admin, &release.info_hash).await;
        assert_eq!(torrent["postProcessStatus"], "completed", "{torrent}");

        let library_file = std::path::Path::new(&world.library_path)
            .join("Seeded Show/Season 02/Seeded Show - S02E02 - Ratio.mkv");
        assert!(
            library_file.is_file(),
            "{:?}",
            list_tree(&world.library_path)
        );
        let imported_bytes = std::fs::read(&library_file).expect("read library file");
        let payload = std::path::Path::new(torrent["savePath"].as_str().expect("savePath"))
            .join("Seeded.Show.S02E02.1080p.mkv");
        assert!(payload.is_file(), "the payload should still be seeding");

        // The ratio rule reads `Torrent.uploadedBytesTotal`, the restart-safe
        // cumulative counter, so raise it the way a previous seeding session
        // would have. Driving a real upload here would mean a second librqbit
        // session and a peer handshake, and the exact byte accounting is
        // librqbit's job and already covered by `seeding.rs`'s unit tests —
        // what is untested anywhere else is the wiring from "rules satisfied"
        // to "payload deleted, row gone, hardlink intact".
        let total_bytes = torrent["totalBytes"].as_i64().expect("totalBytes");
        let updated = admin
            .query(
                "mutation U($id: String!, $uploaded: Int!) { \
                   updateTorrent(id: $id, input: { uploadedBytesTotal: $uploaded }) \
                   { success error } }",
                json!({
                    "id": torrent["id"],
                    // Comfortably past the 0.25 ratio.
                    "uploaded": total_bytes * 4,
                }),
            )
            .await;
        assert_eq!(updated["updateTorrent"]["success"], true, "{updated}");

        // `enforce_seeding_policy` runs on the 15s maintenance tick: once the
        // ratio is met and the import is `completed`, the torrent is deleted
        // with its files and the row goes away entirely.
        admin
            .wait_until(
                SEEDING_SETTLE,
                "the seeded torrent to be removed after import",
                "query T($hash: String!) { torrents(where: { infoHash: { eq: $hash } }) \
                 { pageInfo { totalCount } } }",
                json!({ "hash": release.info_hash }),
                |data| data["torrents"]["pageInfo"]["totalCount"] == json!(0),
            )
            .await;

        assert!(
            !payload.exists(),
            "the download payload {} should have been deleted",
            payload.display()
        );
        assert!(
            library_file.is_file(),
            "the imported library file must survive payload deletion"
        );
        assert_eq!(
            std::fs::read(&library_file).expect("re-read library file"),
            imported_bytes,
            "the library file changed when the payload was deleted"
        );

        // The episode keeps its file: the MediaFile row was rewritten to the
        // library path at import time, so it never referenced the payload.
        let linked = episode_by_id(&mut admin, &episode).await;
        assert_eq!(linked["wanted"], false, "{linked}");
        let media_file = media_file_by_id(
            &mut admin,
            linked["mediaFileId"].as_str().expect("media file id"),
        )
        .await;
        assert_eq!(media_file["path"], library_file.to_string_lossy().as_ref());

        app.shutdown().await;
        indexer.shutdown().await;
    });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn start_app(options: &TestAppOptions) -> TestApp {
    TestApp::start_with(TestAppOptions {
        with_torrent: options.with_torrent,
        with_auto_download: options.with_auto_download,
        torrent_seed_ratio_limit: options.torrent_seed_ratio_limit,
        torrent_seed_time_minutes: options.torrent_seed_time_minutes,
        torrent_remove_after_import: options.torrent_remove_after_import,
        ..Default::default()
    })
    .await
}

/// The library, source and user ids every test needs.
struct World {
    user_id: String,
    library_id: String,
    library_path: String,
    source_id: String,
}

impl World {
    async fn build(app: &TestApp, admin: &mut GqlClient, indexer: &MockIndexer) -> Self {
        let user_id = admin.user_id.clone().expect("admin id");
        let library_path = app.path("media/tv");
        std::fs::create_dir_all(&library_path).expect("library dir");
        let library_path = library_path.to_string_lossy().to_string();

        let library = admin
            .query(
                r#"mutation C($input: CreateLibraryInput!) {
                     createLibrary(input: $input) { success error library { id } }
                   }"#,
                json!({ "input": {
                    "userId": user_id, "name": "TV", "path": library_path,
                    "libraryType": "tv", "autoScan": false, "autoOrganize": false,
                    "namingPattern": "", "scanIntervalMinutes": 60,
                    "watchForChanges": false, "scanning": false
                }}),
            )
            .await;
        assert_eq!(library["createLibrary"]["success"], true, "{library}");
        let library_id = library["createLibrary"]["library"]["id"]
            .as_str()
            .expect("library id")
            .to_string();

        let source = admin
            .query(
                r#"mutation C($input: CreateSourceInput!) {
                     createSource(input: $input) { success error source { id } }
                   }"#,
                json!({ "input": {
                    "name": "Mock Torznab",
                    "sourceType": "torrent_indexer",
                    "definitionId": "torznab",
                    "enabled": true,
                    "priority": 1,
                    "mediaTypes": "tv,movies",
                    "siteUrl": indexer.base_url,
                    "supportsSearch": true,
                    "supportsTvSearch": true,
                    "supportsMovieSearch": true,
                    "supportsMusicSearch": false,
                    "supportsBookSearch": false,
                    // Stored encrypted by the write transform; the mutation
                    // takes plaintext.
                    "credentials": format!("{{\"ApiKey\":\"{}\"}}", indexer.api_key),
                    "errorCount": 0
                }}),
            )
            .await;
        assert_eq!(source["createSource"]["success"], true, "{source}");
        let source_id = source["createSource"]["source"]["id"]
            .as_str()
            .expect("source id")
            .to_string();

        // `createSource` schedules a *deferred* SourcesService reload, so the
        // indexer is not searchable the instant the mutation returns.
        // `testSource` forces the reload and proves the mock answers.
        admin
            .wait_until(
                SETTLE,
                "the source to become loadable",
                "mutation T($id: String!) { testSource(id: $id) { success error } }",
                json!({ "id": source_id }),
                |data| data["testSource"]["success"] == json!(true),
            )
            .await;

        Self {
            user_id,
            library_id,
            library_path,
            source_id,
        }
    }

    async fn create_show(
        &self,
        admin: &mut GqlClient,
        name: &str,
        mode: &str,
        quality_profile_id: Option<&str>,
    ) -> String {
        let created = admin
            .query(
                r#"mutation C($input: CreateShowInput!) {
                     createShow(input: $input) { success error show { id } }
                   }"#,
                json!({ "input": {
                    "userId": self.user_id,
                    "libraryId": self.library_id,
                    "name": name,
                    "genres": [],
                    "autoDownload": mode != "NONE",
                    "autoDownloadMode": mode,
                    "qualityProfileId": quality_profile_id
                }}),
            )
            .await;
        assert_eq!(created["createShow"]["success"], true, "{created}");
        created["createShow"]["show"]["id"]
            .as_str()
            .expect("show id")
            .to_string()
    }

    async fn create_episode(
        &self,
        admin: &mut GqlClient,
        show_id: &str,
        season: i32,
        episode: i32,
        title: &str,
        wanted: bool,
    ) -> String {
        let created = admin
            .query(
                r#"mutation C($input: CreateEpisodeInput!) {
                     createEpisode(input: $input) { success error episode { id } }
                   }"#,
                json!({ "input": {
                    "showId": show_id,
                    "season": season,
                    "episode": episode,
                    "title": title,
                    // Aired yesterday: `is_released` must say yes, or the
                    // episode is not a candidate.
                    "airDate": (chrono::Utc::now() - chrono::Duration::days(1))
                        .format("%Y-%m-%d").to_string(),
                    "wanted": wanted
                }}),
            )
            .await;
        assert_eq!(created["createEpisode"]["success"], true, "{created}");
        created["createEpisode"]["episode"]["id"]
            .as_str()
            .expect("episode id")
            .to_string()
    }
}

/// `CreateQualityProfileInput` makes every JSON column non-null, so a test
/// that only cares about two rules still has to send all of them.
fn profile_input(overrides: Value) -> Value {
    let mut input = json!({
        "name": "Profile",
        "mediaKind": "VIDEO",
        "allowedResolutions": [],
        "allowedVideoCodecs": [],
        "allowedAudioFormats": [],
        "allowedHdrTypes": [],
        "allowedSources": [],
        "releaseGroupBlacklist": [],
        "releaseGroupWhitelist": [],
        "preferredLanguages": [],
        "preferredReleaseGroups": [],
        "resolutionPreference": [],
        "requireHdr": false,
        "requireLanguageMatch": false,
        "upgradeUntilCutoff": true,
        "minSeeders": 1,
        "allowSeasonPacks": true,
        "preferProperRepack": true,
        "isDefault": false
    });
    let target = input.as_object_mut().expect("profile object");
    for (key, value) in overrides.as_object().expect("overrides object") {
        target.insert(key.clone(), value.clone());
    }
    input
}

async fn create_profile(admin: &mut GqlClient, overrides: Value) -> String {
    let input = profile_input(overrides);
    let created = admin
        .query(
            r#"mutation C($input: CreateQualityProfileInput!) {
                 createQualityProfile(input: $input) { success error qualityProfile { id } }
               }"#,
            json!({ "input": input }),
        )
        .await;
    assert_eq!(
        created["createQualityProfile"]["success"], true,
        "{created}"
    );
    created["createQualityProfile"]["qualityProfile"]["id"]
        .as_str()
        .expect("profile id")
        .to_string()
}

async fn trigger_auto_download(admin: &mut GqlClient, library_id: &str) -> Value {
    admin
        .query(
            "mutation T($id: String!) { triggerAutoDownload(libraryId: $id) \
             { success candidatesConsidered searched grabbed errors error } }",
            json!({ "id": library_id }),
        )
        .await["triggerAutoDownload"]
        .clone()
}

async fn search_missing(admin: &mut GqlClient, show_id: &str) -> Value {
    admin
        .query(
            "mutation S($id: String!) { searchMissing(input: { showId: $id }) \
             { success error queued searched } }",
            json!({ "id": show_id }),
        )
        .await["searchMissing"]
        .clone()
}

const TORRENT_FIELDS: &str = "id infoHash name state progress season showId episodeId libraryId \
    sourceIndexerId sourceUrl magnetUri savePath totalBytes postProcessStatus postProcessError";

async fn torrent_by_hash(admin: &mut GqlClient, info_hash: &str) -> Value {
    let data = admin
        .query(
            &format!(
                "query T($hash: String!) {{ torrents(where: {{ infoHash: {{ eq: $hash }} }}) \
                 {{ edges {{ node {{ {TORRENT_FIELDS} }} }} }} }}"
            ),
            json!({ "hash": info_hash }),
        )
        .await;
    data["torrents"]["edges"][0]["node"].clone()
}

/// Wait until the completion handler has written a terminal post-process
/// status onto the torrent row.
async fn wait_for_post_process(admin: &mut GqlClient, info_hash: &str) -> Value {
    let data = admin
        .wait_until(
            SETTLE,
            "the torrent to finish downloading and post-processing",
            &format!(
                "query T($hash: String!) {{ torrents(where: {{ infoHash: {{ eq: $hash }} }}) \
                 {{ edges {{ node {{ {TORRENT_FIELDS} }} }} }} }}"
            ),
            json!({ "hash": info_hash }),
            |data| !data["torrents"]["edges"][0]["node"]["postProcessStatus"].is_null(),
        )
        .await;
    data["torrents"]["edges"][0]["node"].clone()
}

async fn episode_by_id(admin: &mut GqlClient, id: &str) -> Value {
    admin
        .query(
            "query E($id: String!) { episode(id: $id) \
             { id season episode title wanted mediaFileId } }",
            json!({ "id": id }),
        )
        .await["episode"]
        .clone()
}

async fn episode_ids(admin: &mut GqlClient, show_id: &str) -> Vec<String> {
    let data = admin
        .query(
            "query E($id: String!) { episodes(where: { showId: { eq: $id } }) \
             { edges { node { id } } } }",
            json!({ "id": show_id }),
        )
        .await;
    data["episodes"]["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .filter_map(|edge| edge["node"]["id"].as_str().map(str::to_string))
        .collect()
}

async fn media_file_by_id(admin: &mut GqlClient, id: &str) -> Value {
    admin
        .query(
            "query M($id: String!) { mediaFile(id: $id) \
             { id path relativePath episodeId libraryId matchType size } }",
            json!({ "id": id }),
        )
        .await["mediaFile"]
        .clone()
}

/// Every file under `root`, as paths relative to it, sorted.
fn list_tree(root: impl AsRef<std::path::Path>) -> Vec<String> {
    fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                out.push(
                    path.strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string(),
                );
            }
        }
    }
    let root = root.as_ref();
    let mut files = Vec::new();
    walk(root, root, &mut files);
    files.sort();
    files
}
