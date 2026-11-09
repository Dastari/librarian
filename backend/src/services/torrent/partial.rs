//! Downloading only the missing files from a pack.
//!
//! A season pack or an album holds files for things the library already has. With "Download only the
//! missing files from packs" on (the default), a grab made for a set of missing episodes or tracks
//! selects just those files, using the client's `only_files`. The selection is worked out from the
//! torrent's file names while the torrent is still paused, before a byte is downloaded.
//!
//! The rules are deliberately cautious. A selection that matches nothing, or that cannot tell the
//! files apart, selects *everything*: a wrong guess must never leave the wanted episode out. Small
//! files that belong to the whole release (artwork, cue sheets, logs, `.nfo`) and files that share
//! a name with a selected video (subtitles) come along.
//!
//! **Ratio.** A partial download of a private tracker's torrent never completes it, so it can
//! never seed to 100% and some trackers count an unfinished torrent against the account's ratio.
//! That is why this is a setting, and why its description says so.

use std::collections::HashSet;

use crate::services::quality::scoring::parse_release;
use crate::services::{music_rules, release_match};

/// What a grab is for, in terms of files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WantedFiles {
    /// `(season, episode)` pairs that are missing.
    pub episodes: Vec<(i32, i32)>,
    /// Tracks that are missing: `(track number, title)`.
    pub tracks: Vec<(i32, String)>,
    /// The exact file indexes to download, when the torrent was inspected before the grab
    /// (`torrent::judge`). Wins over matching names when every index is in range.
    pub indices: Vec<usize>,
}

impl WantedFiles {
    pub fn is_empty(&self) -> bool {
        self.episodes.is_empty() && self.tracks.is_empty() && self.indices.is_empty()
    }

    /// Exactly these files, from an inspection.
    pub fn exactly(indices: Vec<usize>) -> Self {
        Self {
            indices,
            ..Default::default()
        }
    }
}

fn stem(name: &str) -> &str {
    let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

fn extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

/// The track number a file name leads with: `01 - Title.flac`, `1-02 Title`, `Disc 1/03. Title`.
fn leading_track_number(name: &str) -> Option<i32> {
    let file_stem = stem(name);
    let digits: String = file_stem
        .trim_start()
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    // Only a number near the start counts: `Artist - 05 - Title` has no leading number.
    let position = file_stem.find(|c: char| c.is_ascii_digit())?;
    (position <= 3 && !digits.is_empty() && digits.len() <= 3)
        .then(|| digits.parse().ok())
        .flatten()
}

/// The indexes of the files to download, or `None` to download everything (nothing wanted is
/// known, nothing matched, or everything matched).
pub fn select_wanted_files(names: &[String], wanted: &WantedFiles) -> Option<Vec<usize>> {
    if wanted.is_empty() || names.is_empty() {
        return None;
    }
    // An inspected selection names the files outright. One out of range means the torrent is not
    // the one inspected: fall back to the names rather than guess.
    if !wanted.indices.is_empty() && wanted.indices.iter().all(|index| *index < names.len()) {
        let mut chosen = wanted.indices.clone();
        // An inspected selection already holds its subtitles; a VobSub half still brings its pair.
        let pairs: HashSet<String> = chosen
            .iter()
            .filter(|index| super::subtitles::is_subtitle(&names[**index]))
            .map(|index| super::subtitles::pair_key(&names[*index]))
            .collect();
        for (index, name) in names.iter().enumerate() {
            if super::subtitles::is_subtitle(name)
                && pairs.contains(&super::subtitles::pair_key(name))
            {
                chosen.push(index);
            }
        }
        chosen.sort_unstable();
        chosen.dedup();
        return (chosen.len() < names.len()).then_some(chosen);
    }
    let mut chosen: HashSet<usize> = HashSet::new();
    let mut matched_media = 0usize;

    for (index, name) in names.iter().enumerate() {
        let ext = extension(name);
        let is_video = release_match::VIDEO_EXTENSIONS.contains(&ext.as_str());
        let is_audio = release_match::AUDIO_EXTENSIONS.contains(&ext.as_str());
        if is_video && !wanted.episodes.is_empty() {
            let parsed = parse_release(stem(name));
            if let Some(season) = parsed.season
                && parsed
                    .episodes
                    .iter()
                    .any(|episode| wanted.episodes.contains(&(season, *episode)))
            {
                chosen.insert(index);
                matched_media += 1;
            }
        } else if is_audio && !wanted.tracks.is_empty() {
            let number = leading_track_number(name);
            let title_match = wanted.tracks.iter().any(|(wanted_number, title)| {
                number == Some(*wanted_number) || music_rules::same_song_title(stem(name), title)
            });
            if title_match {
                chosen.insert(index);
                matched_media += 1;
            }
        }
    }
    if matched_media == 0 {
        return None;
    }

    // Companions: a selected video's subtitles (next to it, in `Subs/<stem>/` or a `Subs/` folder
    // beside it, `.idx` with its `.sub`, Q112), files sharing a selected file's name, and small
    // release-wide files.
    let mut described: Vec<super::inspect::ContentFile> = names
        .iter()
        .enumerate()
        .map(|(index, name)| super::inspect::describe_file(index, name, 0))
        .collect();
    super::subtitles::link_subtitles(&mut described);
    let selected_stems: HashSet<String> = chosen
        .iter()
        .map(|index| stem(&names[*index]).to_ascii_lowercase())
        .collect();
    let videos_chosen = chosen.clone();
    for (index, name) in names.iter().enumerate() {
        if chosen.contains(&index) {
            continue;
        }
        let ext = extension(name);
        let is_media = release_match::VIDEO_EXTENSIONS.contains(&ext.as_str())
            || release_match::AUDIO_EXTENSIONS.contains(&ext.as_str());
        if is_media {
            continue;
        }
        if described[index]
            .belongs_to
            .is_some_and(|owner| videos_chosen.contains(&owner))
        {
            chosen.insert(index);
            continue;
        }
        let shares_name = selected_stems.contains(&stem(name).to_ascii_lowercase());
        let release_wide = matches!(
            ext.as_str(),
            "jpg" | "jpeg" | "png" | "nfo" | "cue" | "log" | "m3u" | "txt" | "sfv"
        ) && !name.contains('/');
        if shares_name || release_wide {
            chosen.insert(index);
        }
    }

    // A VobSub `.idx` is useless without its `.sub`, and the other way round.
    let pairs: HashSet<String> = chosen
        .iter()
        .filter(|index| super::subtitles::is_subtitle(&names[**index]))
        .map(|index| super::subtitles::pair_key(&names[*index]))
        .collect();
    for (index, name) in names.iter().enumerate() {
        if super::subtitles::is_subtitle(name) && pairs.contains(&super::subtitles::pair_key(name))
        {
            chosen.insert(index);
        }
    }
    if chosen.len() >= names.len() {
        return None;
    }
    let mut chosen: Vec<usize> = chosen.into_iter().collect();
    chosen.sort_unstable();
    Some(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn an_inspected_selection_is_taken_as_it_is_when_it_fits_the_torrent() {
        let files = names(&["a.mkv", "b.mkv", "c.mkv"]);
        assert_eq!(
            select_wanted_files(&files, &WantedFiles::exactly(vec![2, 0])),
            Some(vec![0, 2])
        );
        // Everything is no selection; an index past the end falls back to names (here: none).
        assert_eq!(
            select_wanted_files(&files, &WantedFiles::exactly(vec![0, 1, 2])),
            None
        );
        assert_eq!(
            select_wanted_files(&files, &WantedFiles::exactly(vec![7])),
            None
        );
    }

    #[test]
    fn only_the_missing_episodes_of_a_season_pack_are_selected_with_their_subtitles() {
        let files = names(&[
            "Show.S02.1080p/Show.S02E01.1080p.mkv",
            "Show.S02.1080p/Show.S02E02.1080p.mkv",
            "Show.S02.1080p/Show.S02E03.1080p.mkv",
            "Show.S02.1080p/Show.S02E03.1080p.srt",
            "Show.S02.1080p/Show.S02E04.1080p.mkv",
            "poster.jpg",
        ]);
        let wanted = WantedFiles {
            episodes: vec![(2, 3), (2, 4)],
            ..Default::default()
        };
        assert_eq!(select_wanted_files(&files, &wanted), Some(vec![2, 3, 4, 5]));
    }

    #[test]
    fn a_selection_that_matches_nothing_downloads_everything() {
        let files = names(&["Show.S02E01.mkv", "Show.S02E02.mkv"]);
        let wanted = WantedFiles {
            episodes: vec![(5, 9)],
            ..Default::default()
        };
        assert_eq!(select_wanted_files(&files, &wanted), None);
        assert_eq!(select_wanted_files(&files, &WantedFiles::default()), None);
    }

    #[test]
    fn selecting_every_file_is_no_selection() {
        let files = names(&["Show.S01E01.mkv", "Show.S01E02.mkv"]);
        let wanted = WantedFiles {
            episodes: vec![(1, 1), (1, 2)],
            ..Default::default()
        };
        assert_eq!(select_wanted_files(&files, &wanted), None);
    }

    #[test]
    fn an_album_selects_the_missing_tracks_by_number_and_keeps_the_artwork() {
        let files = names(&[
            "Album/01 - First Song.flac",
            "Album/02 - Second Song.flac",
            "Album/03 - Third Song.flac",
            "Album/cover.jpg",
            "Album/album.cue",
        ]);
        let wanted = WantedFiles {
            tracks: vec![(2, "Second Song".to_string())],
            ..Default::default()
        };
        // The artwork and cue sheet are inside a folder here, so only the track is taken.
        assert_eq!(select_wanted_files(&files, &wanted), Some(vec![1]));
        let flat = names(&["01 - First Song.flac", "02 - Second Song.flac", "cover.jpg"]);
        assert_eq!(select_wanted_files(&flat, &wanted), Some(vec![1, 2]));
    }

    #[test]
    fn a_track_is_found_by_title_when_its_number_is_missing_from_the_name() {
        let files = names(&["Second Song.flac", "First Song.flac", "Third Song.flac"]);
        let wanted = WantedFiles {
            tracks: vec![(2, "Second Song".to_string())],
            ..Default::default()
        };
        assert_eq!(select_wanted_files(&files, &wanted), Some(vec![0]));
    }
}
