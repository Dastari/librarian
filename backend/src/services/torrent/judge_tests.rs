//! Judging releases by their files: foreign packs, short packs, vague names, tracks and books.

use super::*;
use crate::services::torrent::inspect::describe_file;

fn contents(name: &str, paths: &[(&str, u64)]) -> Contents {
    Contents {
        info_hash: "0".repeat(40),
        name: name.to_string(),
        total_size: paths.iter().map(|(_, size)| *size as i64).sum(),
        is_private: false,
        files: paths
            .iter()
            .enumerate()
            .map(|(index, (path, size))| describe_file(index, path, *size))
            .collect(),
    }
}

fn season(show: &str, season: i32, episodes: std::ops::RangeInclusive<i32>) -> WantedItems {
    WantedItems::Episodes {
        show: WantedTitle::work(show),
        episodes: episodes.map(|episode| (season, episode)).collect(),
        default_season: Some(season),
    }
}

const GB: u64 = 1_000_000_000;

#[test]
fn a_pack_of_another_shows_files_is_rejected() {
    // Named like the wanted show, but the files are Babylon Berlin.
    let pack = contents(
        "Babylon 5 Season 1",
        &[
            ("Babylon Berlin S01/Babylon.Berlin.S01E01.1080p.mkv", 2 * GB),
            ("Babylon Berlin S01/Babylon.Berlin.S01E02.1080p.mkv", 2 * GB),
            ("Babylon Berlin S01/Babylon.Berlin.S01E03.1080p.mkv", 2 * GB),
        ],
    );
    let verdict = judge(&pack, &season("Babylon 5", 1, 1..=22));
    assert_eq!(
        verdict.reject.as_deref(),
        Some("its files are 'babylon berlin', not 'Babylon 5'")
    );
}

#[test]
fn a_short_pack_counts_for_what_it_holds_and_names_the_files_to_take() {
    let mut files: Vec<(String, u64)> = (1..=18)
        .map(|episode| {
            (
                format!("Babylon.5.S01.1080p/Babylon.5.S01E{episode:02}.1080p.BluRay.mkv"),
                GB,
            )
        })
        .collect();
    files.push((
        "Babylon.5.S01.1080p/Babylon.5.S01E02.1080p.BluRay.srt".into(),
        50_000,
    ));
    files.push((
        "Babylon.5.S01.1080p/Sample/b5.sample.mkv".into(),
        40_000_000,
    ));
    files.push(("Babylon.5.S01.1080p/Babylon.5.nfo".into(), 2_000));
    let paths: Vec<(&str, u64)> = files
        .iter()
        .map(|(path, size)| (path.as_str(), *size))
        .collect();
    let pack = contents("Babylon.5.S01.1080p", &paths);

    // 22 missing; the pack holds 18 of them.
    let all = judge(&pack, &season("Babylon 5", 1, 1..=22));
    assert!(!all.is_rejected());
    assert_eq!(all.covers.len(), 18);
    assert_eq!(
        all.note.as_deref(),
        Some("contains 18 of 22 missing episodes")
    );

    // Only episodes 2 and 3 missing: those two, E02's subtitle and the nfo, never the sample.
    let some = judge(
        &pack,
        &WantedItems::Episodes {
            show: WantedTitle::work("Babylon 5"),
            episodes: vec![(1, 2), (1, 3)],
            default_season: Some(1),
        },
    );
    let chosen: Vec<&str> = some
        .files
        .iter()
        .map(|index| pack.files[*index].path.as_str())
        .collect();
    assert_eq!(
        chosen,
        vec![
            "Babylon.5.S01.1080p/Babylon.5.S01E02.1080p.BluRay.mkv",
            "Babylon.5.S01.1080p/Babylon.5.S01E03.1080p.BluRay.mkv",
            "Babylon.5.S01.1080p/Babylon.5.S01E02.1080p.BluRay.srt",
            "Babylon.5.S01.1080p/Babylon.5.nfo",
        ]
    );

    // Missing episodes it does not hold: rejected, saying what it has.
    let none = judge(&pack, &season("Babylon 5", 1, 19..=22));
    assert_eq!(
        none.reject.as_deref(),
        Some("holds none of the missing episodes (it has S01E01–E18)")
    );
}

#[test]
fn a_vaguely_named_pack_counts_for_the_seasons_its_files_show() {
    let pack = contents(
        "Babylon 5 Complete DVD Rip",
        &[
            ("B5/Season 1/01 - Midnight on the Firing Line.avi", GB),
            ("B5/Season 1/02 - Soul Hunter.avi", GB),
            ("B5/Season 2/01 - Points of Departure.avi", GB),
            ("B5/Extras/Making of Babylon 5.avi", GB),
        ],
    );
    let wanted = WantedItems::Episodes {
        show: WantedTitle::work("Babylon 5"),
        episodes: vec![(1, 1), (1, 2), (2, 1), (3, 1)],
        default_season: None,
    };
    let verdict = judge(&pack, &wanted);
    assert!(!verdict.is_rejected());
    assert_eq!(verdict.covers, vec![0, 1, 2]);
    assert_eq!(verdict.held_episodes, vec![(1, 1), (1, 2), (2, 1)]);
    // The extra is never taken for an episode.
    assert!(
        verdict
            .files
            .iter()
            .all(|index| !pack.files[*index].path.contains("Extras"))
    );
}

#[test]
fn a_pack_with_no_video_or_only_archives() {
    let ebook = contents("Show S01", &[("Show.S01.epub", 1_000_000)]);
    assert!(judge(&ebook, &season("Show", 1, 1..=3)).is_rejected());
    let archived = contents(
        "Show.S01.1080p",
        &[("Show.S01.1080p.rar", GB), ("Show.S01.1080p.r00", GB)],
    );
    let verdict = judge(&archived, &season("Show", 1, 1..=3));
    assert!(!verdict.is_rejected());
    assert_eq!(verdict.covers.len(), 3);
    assert!(verdict.files.is_empty());
}

#[test]
fn an_album_is_judged_by_its_tracks() {
    let album = contents(
        "Queen - A Night at the Opera (1975) [FLAC]",
        &[
            ("CD1/01 - Death on Two Legs.flac", 30_000_000),
            ("CD1/02 - Lazing on a Sunday Afternoon.flac", 20_000_000),
            ("CD1/11 - Bohemian Rhapsody.flac", 45_000_000),
            ("cover.jpg", 300_000),
        ],
    );
    let verdict = judge(
        &album,
        &WantedItems::Tracks {
            tracks: vec![
                (11, "Bohemian Rhapsody".into()),
                (12, "God Save the Queen".into()),
            ],
        },
    );
    assert_eq!(verdict.covers, vec![0]);
    assert_eq!(
        verdict.note.as_deref(),
        Some("contains 1 of 2 missing tracks")
    );
    let chosen: Vec<&str> = verdict
        .files
        .iter()
        .map(|index| album.files[*index].path.as_str())
        .collect();
    assert_eq!(chosen, vec!["CD1/11 - Bohemian Rhapsody.flac", "cover.jpg"]);

    let wrong = judge(
        &album,
        &WantedItems::Tracks {
            tracks: vec![(12, "God Save the Queen".into())],
        },
    );
    assert!(wrong.is_rejected());
}

#[test]
fn books_and_movies_need_their_kind_of_file() {
    let book = contents("Author - Book", &[("Book/Book.m4b", 300_000_000)]);
    assert_eq!(judge(&book, &WantedItems::Book).covers, vec![0]);
    let ebook = contents("Author - Book", &[("Book/Book.epub", 3_000_000)]);
    let verdict = judge(&ebook, &WantedItems::Book);
    assert!(
        verdict
            .reject
            .as_deref()
            .is_some_and(|why| why.contains("no audio files")),
        "{verdict:?}"
    );
    let film = contents("Heat.1995.1080p", &[("Heat.1995.1080p.mkv", 9 * GB)]);
    assert_eq!(judge(&film, &WantedItems::Movie).covers, vec![0]);
}
