//! Music plans: discography packs before albums before tracks, the smallest release for one track,
//! and the files a single track's grab takes.

use super::*;
use crate::jobs::recommend_plan::{PlanOption, plan_cover};
use crate::services::torrent::inspect::{Contents, describe_file};
use crate::services::torrent::judge::{WantedItems, judge};

fn track(album_id: &str, album: &str, number: i32, title: &str) -> MissingTrack {
    MissingTrack {
        id: format!("{album_id}-{number}"),
        album_id: album_id.to_string(),
        album: album.to_string(),
        number,
        title: title.to_string(),
    }
}

/// Queen: all of A Night at the Opera missing, and one track of News of the World.
fn queen() -> (Vec<String>, Vec<MissingTrack>) {
    let albums = vec![
        "A Night at the Opera".to_string(),
        "News of the World".to_string(),
    ];
    let missing = vec![
        track("opera", "A Night at the Opera", 1, "Death on Two Legs"),
        track(
            "opera",
            "A Night at the Opera",
            2,
            "Lazing on a Sunday Afternoon",
        ),
        track("opera", "A Night at the Opera", 11, "Bohemian Rhapsody"),
        track("news", "News of the World", 1, "We Will Rock You"),
    ];
    (albums, missing)
}

fn option(covers: Vec<usize>, seeders: i32, mb: i64, rank: usize) -> PlanOption {
    PlanOption {
        covers,
        tier: 0,
        seeders: Some(seeders),
        size: Some(mb * 1_000_000),
        rank,
        subtitles: false,
    }
}

#[test]
fn claims_are_read_from_music_release_names() {
    let (albums, missing) = queen();
    let claim = |title: &str| music_claim(title, "Queen", &albums, &missing);
    assert_eq!(
        claim("Queen - Discography 1973-1995 [FLAC]"),
        Some(MusicClaim::Discography)
    );
    assert_eq!(
        claim("Queen - The Complete Studio Albums (2011) [MP3 320]"),
        Some(MusicClaim::Discography)
    );
    assert_eq!(
        claim("Queen - A Night at the Opera (1975) [FLAC 24bit]"),
        Some(MusicClaim::Album(0))
    );
    assert_eq!(
        claim("Queen - We Will Rock You (Single) [FLAC]"),
        Some(MusicClaim::Tracks(vec![3]))
    );
    assert_eq!(
        claim("Queen - Greatest Hits (1981) [FLAC]"),
        Some(MusicClaim::Other)
    );
    // Another artist's release is not a claim at all.
    assert_eq!(
        claim("Queens of the Stone Age - Songs for the Deaf (2002) [FLAC]"),
        None
    );
}

#[test]
fn a_discography_pack_beats_albums_and_albums_beat_tracks() {
    let (albums, missing) = queen();
    let opera = claimed_tracks(&MusicClaim::Album(0), &albums, &missing);
    assert_eq!(opera, vec![0, 1, 2]);
    // 0: discography (its files hold everything), 1: the Opera album, 2: the single.
    let options = vec![
        option(vec![0, 1, 2, 3], 40, 4_000, 2),
        option(opera.clone(), 40, 600, 0),
        option(vec![3], 40, 8, 1),
    ];
    let plan = plan_cover(missing.len(), &options);
    assert_eq!(plan.releases.len(), 1);
    assert_eq!(plan.releases[0].option, 0);
    assert!(plan.uncovered.is_empty());

    // No discography: the album for the album, and the single for the lone track.
    let plan = plan_cover(missing.len(), &options[1..]);
    let picked: Vec<usize> = plan.releases.iter().map(|planned| planned.option).collect();
    assert_eq!(picked, vec![0, 1]);
    assert_eq!(plan.releases[1].adds, vec![3]);
}

#[test]
fn a_single_track_comes_from_the_smallest_release_holding_it() {
    let (_, missing) = queen();
    // Three releases hold "We Will Rock You" and nothing else missing: the album it is on, a
    // compilation and the single. Equal health: the smallest wins.
    let options = [
        option(vec![3], 40, 600, 0),
        option(vec![3], 40, 900, 1),
        option(vec![3], 40, 8, 2),
    ];
    let plan = plan_cover(missing.len(), &options);
    assert_eq!(plan.releases.len(), 1);
    // `missing` has four entries but only index 3 is offered: the rest stay uncovered.
    assert_eq!(plan.releases[0].option, 2);
    assert_eq!(plan.uncovered, vec![0, 1, 2]);
}

fn contents(paths: &[(&str, u64)]) -> Contents {
    Contents {
        info_hash: "0".repeat(40),
        name: "pack".into(),
        total_size: paths.iter().map(|(_, size)| *size as i64).sum(),
        is_private: false,
        files: paths
            .iter()
            .enumerate()
            .map(|(index, (path, size))| describe_file(index, path, *size))
            .collect(),
    }
}

#[test]
fn a_single_track_grab_takes_only_that_tracks_files_from_an_album() {
    let album = contents(&[
        (
            "Queen - News of the World (1977) [FLAC]/01 - We Will Rock You.flac",
            8_000_000,
        ),
        (
            "Queen - News of the World (1977) [FLAC]/02 - We Are the Champions.flac",
            9_000_000,
        ),
        (
            "Queen - News of the World (1977) [FLAC]/03 - Sheer Heart Attack.flac",
            10_000_000,
        ),
        ("Queen - News of the World (1977) [FLAC]/cover.jpg", 50_000),
        ("folder.jpg", 10_000),
    ]);
    let verdict = judge(
        &album,
        &WantedItems::ArtistTracks {
            tracks: vec![(
                "News of the World".to_string(),
                1,
                "We Will Rock You".to_string(),
            )],
        },
    );
    assert_eq!(verdict.covers, vec![0]);
    // The track and the release-wide artwork; not the other songs.
    assert_eq!(verdict.files, vec![0, 3, 4]);
    let selected: u64 = verdict
        .files
        .iter()
        .map(|index| album.files[*index].size)
        .sum();
    assert_eq!(
        download_note(
            1,
            Some(selected as i64),
            Some(album.total_size),
            "album",
            true
        ),
        "Downloads 1 track (8 MB) from a 27 MB album"
    );
}

#[test]
fn a_discography_is_judged_album_by_album_where_track_numbers_repeat() {
    let pack = contents(&[
        (
            "Queen Discography/1975 - A Night at the Opera/01 - Death on Two Legs.flac",
            1,
        ),
        (
            "Queen Discography/1975 - A Night at the Opera/11 - Bohemian Rhapsody.flac",
            1,
        ),
        (
            "Queen Discography/1977 - News of the World/01 - We Will Rock You.flac",
            1,
        ),
        (
            "Queen Discography/1977 - News of the World/02 - We Are the Champions.flac",
            1,
        ),
    ]);
    let (_, missing) = queen();
    let wanted: Vec<(String, i32, String)> = missing
        .iter()
        .map(|track| (track.album.clone(), track.number, track.title.clone()))
        .collect();
    let verdict = judge(&pack, &WantedItems::ArtistTracks { tracks: wanted });
    // "Lazing on a Sunday Afternoon" is not in the pack; track 1 of each album goes to its own.
    assert_eq!(verdict.covers, vec![0, 2, 3]);
    assert_eq!(verdict.files, vec![0, 1, 2]);
    assert_eq!(
        verdict.note.as_deref(),
        Some("contains 3 of 4 missing tracks")
    );
}

#[test]
fn a_compilation_holding_none_of_the_missing_tracks_is_turned_down() {
    let pack = contents(&[(
        "Queen - Greatest Hits/01 - Another One Bites the Dust.mp3",
        1,
    )]);
    let verdict = judge(
        &pack,
        &WantedItems::ArtistTracks {
            tracks: vec![("News of the World".into(), 1, "We Will Rock You".into())],
        },
    );
    assert!(verdict.reject.is_some());
}

#[test]
fn download_lines_say_what_is_fetched() {
    assert_eq!(
        download_note(2, None, Some(4_000_000_000), "discography pack", true),
        "Downloads only the 2 missing tracks from a 4.0 GB discography pack"
    );
    assert_eq!(
        download_note(1, Some(8_000_000), Some(80_000_000), "EP", true),
        "Downloads 1 track (8 MB) from an 80 MB EP"
    );
    assert_eq!(
        download_note(1, None, Some(600_000_000), "album", false),
        "Downloads the whole 600 MB album for 1 missing track"
    );
    assert_eq!(
        release_kind(&MusicClaim::Other, "Queen - Live Killers EP"),
        "EP"
    );
    assert_eq!(
        release_kind(&MusicClaim::Other, "Queen - Greatest Hits"),
        "compilation"
    );
}

#[test]
fn what_stays_missing_is_said_per_album() {
    let (_, missing) = queen();
    assert_eq!(
        describe_tracks(&missing, &[0, 1, 2, 3]),
        "all of 'A Night at the Opera', 'We Will Rock You' from 'News of the World'"
    );
    assert_eq!(
        albums_of(&missing, &[3, 0, 1]),
        vec!["news".to_string(), "opera".to_string()]
    );
}
