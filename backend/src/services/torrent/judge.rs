//! Judging a release by its files (Q100): which wanted items an inspected torrent really holds.
//!
//! A name promises; the files say. A season pack whose files are another show's, or that lacks
//! episodes its name implies, is rejected or counted for what it holds ("contains 18 of 22
//! episodes"), and a vaguely named pack ("Babylon 5 Complete DVD Rip") counts for the seasons its
//! files show. The files that fill what is missing are named exactly, so a grab can select them.
//!
//! Pure code over [`inspect::Contents`](super::inspect::Contents): no I/O.

use std::collections::{BTreeSet, HashSet};

use super::inspect::{ContentFile, Contents, FileKind};
use crate::services::music_rules;
use crate::services::quality::title_match::{self, WantedTitle};
use crate::services::release_match;

/// What a release is wanted for.
#[derive(Debug, Clone)]
pub enum WantedItems {
    /// Missing episodes of a show, as `(season, episode)`. `default_season` is the season a file
    /// that gives only an episode number belongs to (a single-season pack's).
    Episodes {
        show: WantedTitle,
        episodes: Vec<(i32, i32)>,
        default_season: Option<i32>,
    },
    /// Missing tracks of an album, as `(track number, title)`.
    Tracks { tracks: Vec<(i32, String)> },
    /// Missing tracks of an artist across albums, as `(album, track number, title)`: for a
    /// discography pack, an album, an EP, a single or a compilation (Q110). A track is looked for
    /// in its album's folder first (by title, then by a unique number), else anywhere by title.
    ArtistTracks { tracks: Vec<(String, i32, String)> },
    /// A whole audiobook.
    Book,
    /// A movie.
    Movie,
}

/// What an inspected torrent holds of what is wanted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentsVerdict {
    /// Indexes into the wanted list (episodes or tracks) that the files hold. For a book or a
    /// movie, `[0]` when it holds one.
    pub covers: Vec<usize>,
    /// The torrent's file indexes to download for exactly those, with their companions, or empty
    /// when that would be every file.
    pub files: Vec<usize>,
    /// Every `(season, episode)` the files hold, wanted or not.
    pub held_episodes: Vec<(i32, i32)>,
    /// Why the release must not be taken, from its files.
    pub reject: Option<String>,
    /// A short line for the reasons: "contains 18 of 22 missing episodes".
    pub note: Option<String>,
    /// The languages of the subtitles bundled with what it holds of what is wanted (Q114), English
    /// first. A small tiebreak and a reason, never a reason to turn a release down.
    pub subtitle_languages: Vec<String>,
}

impl ContentsVerdict {
    pub fn is_rejected(&self) -> bool {
        self.reject.is_some()
    }
}

fn names(contents: &Contents) -> Vec<String> {
    contents
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect()
}

/// `S01E01–E03, S02E05` for a message, at most a few ranges.
fn describe_episodes(held: &[(i32, i32)]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut iter = held.iter().peekable();
    while let Some(&(season, first)) = iter.next() {
        let mut last = first;
        while let Some(&&(next_season, next)) = iter.peek() {
            if next_season == season && next == last + 1 {
                last = next;
                iter.next();
            } else {
                break;
            }
        }
        parts.push(if last == first {
            format!("S{season:02}E{first:02}")
        } else {
            format!("S{season:02}E{first:02}–E{last:02}")
        });
    }
    if parts.len() > 4 {
        let more = parts.len() - 3;
        parts.truncate(3);
        parts.push(format!("{more} more"));
    }
    parts.join(", ")
}

/// Small files that belong to the whole release (artwork, sheets, `.nfo`) at its top level.
fn release_wide(file: &ContentFile) -> bool {
    matches!(file.kind, FileKind::Info | FileKind::Image) && file.path.matches('/').count() <= 1
}

/// The selection for `chosen` main files: those, their subtitles (same stem, or same episode),
/// and release-wide sidecars. Empty when it would be everything.
fn selection(contents: &Contents, chosen: &HashSet<usize>) -> Vec<usize> {
    let chosen_files: Vec<&ContentFile> = contents
        .files
        .iter()
        .filter(|file| chosen.contains(&file.index))
        .collect();
    let stems: HashSet<String> = chosen_files
        .iter()
        .map(|file| file.stem().to_ascii_lowercase())
        .collect();
    let episodes: HashSet<(Option<i32>, i32)> = chosen_files
        .iter()
        .flat_map(|file| file.episodes.iter().map(|episode| (file.season, *episode)))
        .collect();
    let mut out: BTreeSet<usize> = chosen.iter().copied().collect();
    for file in &contents.files {
        let companion = match file.kind {
            FileKind::Subtitle => match file.belongs_to {
                // Linked to its video (Q112): next to it, `Subs/<stem>/`, or a `Subs/` folder.
                Some(owner) => chosen.contains(&owner),
                None => {
                    let stem = file.stem().to_ascii_lowercase();
                    stems
                        .iter()
                        .any(|selected| stem.starts_with(selected.as_str()))
                        || file
                            .episodes
                            .iter()
                            .any(|episode| episodes.contains(&(file.season, *episode)))
                }
            },
            _ => release_wide(file),
        };
        if companion {
            out.insert(file.index);
        }
    }
    // A VobSub `.idx` is useless without its `.sub`, and the other way round.
    let pairs: HashSet<String> = contents
        .files
        .iter()
        .filter(|file| out.contains(&file.index) && file.kind == FileKind::Subtitle)
        .map(|file| super::subtitles::pair_key(&file.path))
        .collect();
    for file in &contents.files {
        if file.kind == FileKind::Subtitle && pairs.contains(&super::subtitles::pair_key(&file.path))
        {
            out.insert(file.index);
        }
    }
    if out.len() >= contents.files.len() {
        Vec::new()
    } else {
        out.into_iter().collect()
    }
}

/// What the files hold of what is wanted.
pub fn judge(contents: &Contents, wanted: &WantedItems) -> ContentsVerdict {
    let mut verdict = judge_files(contents, wanted);
    if verdict.reject.is_none() {
        // The subtitles of the videos it is taken for: the selected ones, or all of them.
        let chosen: Vec<usize> = verdict
            .files
            .iter()
            .copied()
            .filter(|index| {
                contents
                    .files
                    .iter()
                    .any(|file| file.index == *index && file.kind == FileKind::Video)
            })
            .collect();
        verdict.subtitle_languages = super::subtitles::languages_for(
            &contents.files,
            (!chosen.is_empty()).then_some(chosen.as_slice()),
        );
    }
    verdict
}

fn judge_files(contents: &Contents, wanted: &WantedItems) -> ContentsVerdict {
    match wanted {
        WantedItems::Episodes {
            show,
            episodes,
            default_season,
        } => judge_episodes(contents, show, episodes, *default_season),
        WantedItems::Tracks { tracks } => judge_tracks(contents, tracks),
        WantedItems::ArtistTracks { tracks } => judge_artist_tracks(contents, tracks),
        WantedItems::Book => judge_whole(contents, true),
        WantedItems::Movie => judge_whole(contents, false),
    }
}

fn judge_whole(contents: &Contents, audio: bool) -> ContentsVerdict {
    let mut verdict = ContentsVerdict::default();
    if let Some(reason) = release_match::payload_mismatch(audio, !audio, &names(contents)) {
        verdict.reject = Some(
            reason
                .trim_end_matches(" Nothing was downloaded.")
                .to_string(),
        );
        return verdict;
    }
    verdict.covers = vec![0];
    verdict
}

fn judge_episodes(
    contents: &Contents,
    show: &WantedTitle,
    wanted: &[(i32, i32)],
    default_season: Option<i32>,
) -> ContentsVerdict {
    let mut verdict = ContentsVerdict::default();
    if let Some(reason) = release_match::payload_mismatch(false, true, &names(contents)) {
        verdict.reject = Some(
            reason
                .trim_end_matches(" Nothing was downloaded.")
                .to_string(),
        );
        return verdict;
    }
    let main: Vec<&ContentFile> = contents
        .files
        .iter()
        .filter(|file| file.kind == FileKind::Video)
        .collect();
    if main.is_empty() {
        // Archives only: what they hold is known after unpacking. Judged by the name.
        verdict.note = Some("packed in archives: its files can't be checked".to_string());
        verdict.covers = (0..wanted.len()).collect();
        return verdict;
    }

    // Whose files are these? A file named after another show counts against the release; one
    // that names nothing (`05 - Title.mkv`) counts for neither.
    let mut ours = 0usize;
    let mut foreign: Vec<String> = Vec::new();
    for file in &main {
        // Only a file numbered like `Show.S01E01` is named after its show; `Title of It.mkv` names
        // an episode, and a two-letter tag (`B5.S01E01`) is too short to say.
        let part = title_match::title_part(file.stem());
        if file.episodes.is_empty() || part.is_empty() || part.concat().len() < 4 {
            continue;
        }
        if show.matches(file.stem()) {
            ours += 1;
        } else {
            foreign.push(part.join(" "));
        }
    }
    if !foreign.is_empty() && foreign.len() > ours {
        foreign.sort();
        let mut counts: Vec<(String, usize)> = Vec::new();
        for name in foreign {
            match counts.last_mut() {
                Some((last, count)) if *last == name => *count += 1,
                _ => counts.push((name, 1)),
            }
        }
        counts.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        verdict.reject = Some(format!(
            "its files are '{}', not '{}'",
            counts[0].0,
            show.display()
        ));
        return verdict;
    }

    let season_of = |file: &ContentFile| file.season.or(default_season);
    let mut held: BTreeSet<(i32, i32)> = BTreeSet::new();
    let mut chosen: HashSet<usize> = HashSet::new();
    for file in &main {
        let Some(season) = season_of(file) else {
            continue;
        };
        for episode in &file.episodes {
            held.insert((season, *episode));
            if wanted.contains(&(season, *episode)) {
                chosen.insert(file.index);
            }
        }
    }
    verdict.held_episodes = held.iter().copied().collect();
    verdict.covers = wanted
        .iter()
        .enumerate()
        .filter(|(_, item)| held.contains(item))
        .map(|(index, _)| index)
        .collect();
    if held.is_empty() {
        // Videos that say no episode (a DVD rip of `VTS_01_1.VOB`, a film): can't tell.
        verdict.note = Some("its files don't say which episodes they are".to_string());
        verdict.covers = (0..wanted.len()).collect();
        return verdict;
    }
    if verdict.covers.is_empty() {
        verdict.reject = Some(format!(
            "holds none of the missing episodes (it has {})",
            describe_episodes(&verdict.held_episodes)
        ));
        return verdict;
    }
    if wanted.len() > 1 || verdict.covers.len() < wanted.len() {
        verdict.note = Some(format!(
            "contains {} of {} missing episode{}",
            verdict.covers.len(),
            wanted.len(),
            if wanted.len() == 1 { "" } else { "s" }
        ));
    }
    verdict.files = selection(contents, &chosen);
    verdict
}

/// A song title from a file stem without its leading number: `01 - Title` is `Title`.
fn stem_title(stem: &str) -> &str {
    stem.trim_start_matches(|c: char| c.is_ascii_digit() || matches!(c, ' ' | '-' | '.' | '_'))
}

fn judge_tracks(contents: &Contents, wanted: &[(i32, String)]) -> ContentsVerdict {
    let mut verdict = ContentsVerdict::default();
    if let Some(reason) = release_match::payload_mismatch(true, false, &names(contents)) {
        verdict.reject = Some(
            reason
                .trim_end_matches(" Nothing was downloaded.")
                .to_string(),
        );
        return verdict;
    }
    let main: Vec<&ContentFile> = contents
        .files
        .iter()
        .filter(|file| file.kind == FileKind::Audio)
        .collect();
    if main.is_empty() {
        verdict.note = Some("packed in archives: its files can't be checked".to_string());
        verdict.covers = (0..wanted.len()).collect();
        return verdict;
    }
    let mut chosen: HashSet<usize> = HashSet::new();
    let mut covered: BTreeSet<usize> = BTreeSet::new();
    for (index, (number, title)) in wanted.iter().enumerate() {
        // A title match wins over a number: on a multi-disc set the numbers repeat.
        let by_title = main
            .iter()
            .find(|file| music_rules::same_song_title(stem_title(file.stem()), title));
        let by_number = || {
            let numbered: Vec<&&ContentFile> = main
                .iter()
                .filter(|file| file.track == Some(*number))
                .collect();
            (numbered.len() == 1).then(|| *numbered[0])
        };
        if let Some(file) = by_title.copied().or_else(by_number) {
            covered.insert(index);
            chosen.insert(file.index);
        }
    }
    verdict.covers = covered.into_iter().collect();
    if verdict.covers.is_empty() {
        let numbered = main.iter().any(|file| file.track.is_some());
        if numbered {
            verdict.reject = Some(format!(
                "holds none of the {} missing track{}",
                wanted.len(),
                if wanted.len() == 1 { "" } else { "s" }
            ));
        } else {
            verdict.note = Some("its files don't say which tracks they are".to_string());
            verdict.covers = (0..wanted.len()).collect();
        }
        return verdict;
    }
    if wanted.len() > 1 || verdict.covers.len() < wanted.len() {
        verdict.note = Some(format!(
            "contains {} of {} missing track{}",
            verdict.covers.len(),
            wanted.len(),
            if wanted.len() == 1 { "" } else { "s" }
        ));
    }
    verdict.files = selection(contents, &chosen);
    verdict
}

/// Whether a file sits in a folder named after `album` (`Artist - Album (1975) [FLAC]/`, or
/// `Album/CD1/`).
fn in_album_folder(file: &ContentFile, album: &str) -> bool {
    let mut folders: Vec<&str> = file.path.split(['/', '\\']).collect();
    folders.pop();
    folders.iter().any(|folder| {
        let facts = crate::services::feed_catalogue::read_music_title(folder);
        let named = facts.album.as_deref().unwrap_or(folder);
        music_rules::same_album_title(named, album) || music_rules::same_album_title(folder, album)
    })
}

fn judge_artist_tracks(contents: &Contents, wanted: &[(String, i32, String)]) -> ContentsVerdict {
    let mut verdict = ContentsVerdict::default();
    if let Some(reason) = release_match::payload_mismatch(true, false, &names(contents)) {
        verdict.reject = Some(
            reason
                .trim_end_matches(" Nothing was downloaded.")
                .to_string(),
        );
        return verdict;
    }
    let main: Vec<&ContentFile> = contents
        .files
        .iter()
        .filter(|file| file.kind == FileKind::Audio)
        .collect();
    if main.is_empty() {
        // Archives: nothing can be said of which tracks it holds, so it covers nothing for sure.
        verdict.reject = Some("its music is packed in archives, so its tracks can't be checked".to_string());
        return verdict;
    }
    let mut chosen: HashSet<usize> = HashSet::new();
    let mut covered: BTreeSet<usize> = BTreeSet::new();
    for (index, (album, number, title)) in wanted.iter().enumerate() {
        let in_album: Vec<&&ContentFile> = main
            .iter()
            .filter(|file| in_album_folder(file, album))
            .collect();
        let found = if in_album.is_empty() {
            // A single, a compilation, or a flat pack: by title only (numbers mean nothing here).
            main.iter()
                .find(|file| music_rules::same_song_title(stem_title(file.stem()), title))
                .copied()
        } else {
            in_album
                .iter()
                .find(|file| music_rules::same_song_title(stem_title(file.stem()), title))
                .map(|file| **file)
                .or_else(|| {
                    let numbered: Vec<&&&ContentFile> = in_album
                        .iter()
                        .filter(|file| file.track == Some(*number))
                        .collect();
                    (numbered.len() == 1).then(|| **numbered[0])
                })
        };
        if let Some(file) = found {
            covered.insert(index);
            chosen.insert(file.index);
        }
    }
    verdict.covers = covered.into_iter().collect();
    if verdict.covers.is_empty() {
        verdict.reject = Some(format!(
            "holds none of the {} missing track{}",
            wanted.len(),
            if wanted.len() == 1 { "" } else { "s" }
        ));
        return verdict;
    }
    if wanted.len() > 1 || verdict.covers.len() < wanted.len() {
        verdict.note = Some(format!(
            "contains {} of {} missing track{}",
            verdict.covers.len(),
            wanted.len(),
            if wanted.len() == 1 { "" } else { "s" }
        ));
    }
    verdict.files = selection(contents, &chosen);
    verdict
}

#[cfg(test)]
#[path = "judge_tests.rs"]
mod tests;
