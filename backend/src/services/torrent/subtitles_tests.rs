//! Subtitles bundled in torrents (Q114): languages, the video each belongs to in every common
//! layout, and that selections of videos take their subtitles along.

use super::*;
use crate::services::quality::title_match::WantedTitle;
use crate::services::torrent::inspect::{Contents, describe_file};
use crate::services::torrent::judge::{WantedItems, judge};
use crate::services::torrent::partial::{WantedFiles, select_wanted_files};

fn files(paths: &[&str]) -> Vec<ContentFile> {
    let mut files: Vec<ContentFile> = paths
        .iter()
        .enumerate()
        .map(|(index, path)| describe_file(index, path, 1_000_000))
        .collect();
    link_subtitles(&mut files);
    files
}

fn contents(paths: &[&str]) -> Contents {
    Contents {
        info_hash: "0".repeat(40),
        name: "pack".into(),
        total_size: 0,
        is_private: false,
        files: files(paths),
    }
}

fn owner(files: &[ContentFile], index: usize) -> Option<usize> {
    files[index].belongs_to
}

#[test]
fn languages_come_from_names_codes_and_folders() {
    let cases = [
        ("Subs/2_English.srt", Some("English")),
        ("Show.S01E01.1080p.en.srt", Some("English")),
        ("Show.S01E01.1080p.eng.forced.srt", Some("English")),
        ("Movie.2019.fr.sdh.srt", Some("French")),
        ("Movie.2019.en.hi.srt", Some("English")),
        ("Subs/3_Spanish (Latin America).srt", Some("Spanish")),
        ("Subs/English/1.srt", Some("English")),
        ("Subs/ger/1.srt", Some("German")),
        ("Movie.2019.1080p.BluRay.srt", None),
        ("Show.S01E01.srt", None),
    ];
    for (path, language) in cases {
        assert_eq!(subtitle_language(path).as_deref(), language, "{path}");
    }
}

#[test]
fn a_subtitle_next_to_its_video_belongs_to_it() {
    let files = files(&[
        "Show.S02/Show.S02E01.1080p.mkv",
        "Show.S02/Show.S02E02.1080p.mkv",
        "Show.S02/Show.S02E01.1080p.en.srt",
        "Show.S02/Show.S02E02.1080p.srt",
    ]);
    assert_eq!(owner(&files, 2), Some(0));
    assert_eq!(owner(&files, 3), Some(1));
    assert_eq!(files[2].language.as_deref(), Some("English"));
}

#[test]
fn a_subs_folder_beside_one_video_belongs_to_it() {
    let files = files(&[
        "Movie.2019.1080p/Movie.2019.1080p.mkv",
        "Movie.2019.1080p/Movie.2019.1080p-sample.mkv",
        "Movie.2019.1080p/Subs/2_English.srt",
        "Movie.2019.1080p/Subs/3_French.srt",
    ]);
    assert_eq!(owner(&files, 2), Some(0));
    assert_eq!(owner(&files, 3), Some(0));
    assert_eq!(
        languages_for(&files, None),
        vec!["English".to_string(), "French".to_string()]
    );
}

#[test]
fn a_season_pack_with_a_folder_per_episode_maps_each_subtitle_to_its_episode() {
    let pack = files(&[
        "Show.S01.1080p/Show.S01E01.1080p.mkv",
        "Show.S01.1080p/Show.S01E02.1080p.mkv",
        "Show.S01.1080p/Subs/Show.S01E01.1080p/2_English.srt",
        "Show.S01.1080p/Subs/Show.S01E01.1080p/5_German.srt",
        "Show.S01.1080p/Subs/Show.S01E02.1080p/2_English.srt",
    ]);
    assert_eq!(owner(&pack, 2), Some(0));
    assert_eq!(owner(&pack, 3), Some(0));
    assert_eq!(owner(&pack, 4), Some(1));
    // Nothing can be said of a subtitle in a shared `Subs/` folder of a pack with no episode in it.
    let shared = files(&["Pack/A.mkv", "Pack/B.mkv", "Pack/Subs/English.srt"]);
    assert_eq!(owner(&shared, 2), None);
}

#[test]
fn a_vobsub_pair_goes_together() {
    let files = files(&[
        "Movie/Movie.mkv",
        "Movie/Other.mkv",
        "Movie/Movie.idx",
        "Movie/Movie.sub",
    ]);
    assert_eq!(owner(&files, 2), Some(0));
    assert_eq!(owner(&files, 3), Some(0));
}

#[test]
fn the_judge_selects_an_episode_with_its_subtitles_in_every_layout() {
    let pack = contents(&[
        "Show.S01.1080p/Show.S01E01.1080p.mkv",
        "Show.S01.1080p/Show.S01E02.1080p.mkv",
        "Show.S01.1080p/Show.S01E03.1080p.mkv",
        "Show.S01.1080p/Show.S01E02.1080p.en.srt",
        "Show.S01.1080p/Subs/Show.S01E02.1080p/2_English.srt",
        "Show.S01.1080p/Subs/Show.S01E02.1080p/3_French.srt",
        "Show.S01.1080p/Subs/Show.S01E03.1080p/2_English.srt",
        "Show.S01.1080p/Show.S01E02.1080p.idx",
        "Show.S01.1080p/Show.S01E02.1080p.sub",
    ]);
    let verdict = judge(
        &pack,
        &WantedItems::Episodes {
            show: WantedTitle::work("Show"),
            episodes: vec![(1, 2)],
            default_season: Some(1),
        },
    );
    assert_eq!(verdict.covers, vec![0]);
    assert_eq!(verdict.files, vec![1, 3, 4, 5, 7, 8]);
    assert_eq!(
        verdict.subtitle_languages,
        vec![
            "English".to_string(),
            "French".to_string(),
            "unknown".to_string()
        ]
    );
}

#[test]
fn a_movie_judged_whole_reports_its_subtitle_languages_and_none_is_no_rejection() {
    let movie = contents(&["Movie.2019/Movie.2019.mkv", "Movie.2019/Subs/2_English.srt"]);
    let verdict = judge(&movie, &WantedItems::Movie);
    assert!(verdict.reject.is_none());
    assert_eq!(verdict.subtitle_languages, vec!["English".to_string()]);
    let bare = contents(&["Movie.2019/Movie.2019.mkv"]);
    let verdict = judge(&bare, &WantedItems::Movie);
    assert!(verdict.reject.is_none());
    assert!(verdict.subtitle_languages.is_empty());
}

#[test]
fn a_selection_by_name_keeps_subtitles_in_every_layout() {
    let names: Vec<String> = [
        "Show.S01/Show.S01E01.mkv",
        "Show.S01/Show.S01E02.mkv",
        "Show.S01/Subs/Show.S01E02/2_English.srt",
        "Show.S01/Subs/Show.S01E01/2_English.srt",
        "Show.S01/Show.S01E02.en.srt",
        "Show.S01/Show.S01E02.idx",
        "Show.S01/Show.S01E02.sub",
    ]
    .iter()
    .map(|name| name.to_string())
    .collect();
    let wanted = WantedFiles {
        episodes: vec![(1, 2)],
        ..Default::default()
    };
    assert_eq!(
        select_wanted_files(&names, &wanted),
        Some(vec![1, 2, 4, 5, 6])
    );
    // An inspected selection holding one half of a VobSub pair brings the other.
    assert_eq!(
        select_wanted_files(&names, &WantedFiles::exactly(vec![1, 5])),
        Some(vec![1, 5, 6])
    );
}

#[test]
fn a_track_pick_is_not_disturbed_by_subtitles() {
    let names: Vec<String> = [
        "Album/01 - One.flac",
        "Album/02 - Two.flac",
        "Album/02 - Two.lrc",
    ]
    .iter()
    .map(|name| name.to_string())
    .collect();
    let wanted = WantedFiles {
        tracks: vec![(2, "Two".to_string())],
        ..Default::default()
    };
    assert_eq!(select_wanted_files(&names, &wanted), Some(vec![1, 2]));
}

#[test]
fn languages_are_described_for_a_reason() {
    assert_eq!(describe_languages(&[]), None);
    assert_eq!(
        describe_languages(&["English".into()]).as_deref(),
        Some("Includes English subtitles")
    );
    assert_eq!(
        describe_languages(&[
            "English".into(),
            "French".into(),
            "German".into(),
            "Dutch".into()
        ])
        .as_deref(),
        Some("Includes English, French and 2 more subtitles")
    );
    assert_eq!(
        describe_languages(&["unknown".into()]).as_deref(),
        Some("Includes subtitles")
    );
}
