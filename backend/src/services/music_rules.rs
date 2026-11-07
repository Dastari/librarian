//! What a music release's files say, and which album and edition they are. Pure rules with no I/O,
//! shared by library scans and download identification. See `docs/design.md`.

use std::collections::HashMap;

use strsim::jaro_winkler;

use crate::services::identify_rules::tidy_title;
use crate::services::release_match::{ReleaseFile, TrackFacts, match_tracks, normalize_words};

/// How close two spellings have to be to count as the same name.
const SAME_NAME: f64 = 0.93;

/// A title without what's about the release: `Use Your Illusion I (Deluxe_Edition)` is `Use Your
/// Illusion I`, and `Abbey Road - Disc 2` is `Abbey Road`.
pub fn album_title(tag: &str) -> String {
    let spaced = tag.replace('_', " ");
    let tidied = tidy_title(&spaced);
    let mut words: Vec<&str> = tidied.split_whitespace().collect();
    while words.len() > 1 {
        let last = words[words.len() - 1].to_ascii_lowercase();
        let before = words
            .get(words.len().wrapping_sub(2))
            .map(|word| word.to_ascii_lowercase());
        let is_disc_number = last.chars().all(|ch| ch.is_ascii_digit())
            && matches!(before.as_deref(), Some("disc" | "disk" | "cd"));
        let is_edition = last == "edition"
            && matches!(
                before.as_deref(),
                Some("deluxe" | "limited" | "special" | "expanded" | "anniversary" | "remastered")
            );
        if is_disc_number || is_edition {
            words.truncate(words.len() - 2);
        } else if matches!(last.as_str(), "-" | "–" | "—" | "deluxe" | "remastered") {
            words.truncate(words.len() - 1);
        } else {
            break;
        }
    }
    if words.is_empty() {
        tidied
    } else {
        words.join(" ")
    }
}

/// The words of a name, without a leading article or a joining word.
fn name_words(name: &str) -> Vec<String> {
    let mut words: Vec<String> = normalize_words(name)
        .split_whitespace()
        .filter(|word| *word != "and")
        .map(str::to_string)
        .collect();
    if words.len() > 1 && words[0] == "the" {
        words.remove(0);
    }
    words
}

/// Spelling differences don't make two artists different: `Guns N' Roses` and `Guns N’ Roses` are
/// one, and `Xzibit` and `Roses` aren't, however an album is named.
pub fn same_artist(a: &str, b: &str) -> bool {
    let (a, b) = (name_words(a).join(" "), name_words(b).join(" "));
    !a.is_empty() && !b.is_empty() && (a == b || jaro_winkler(&a, &b) >= SAME_NAME)
}

/// A word that says which one of a series this is. Such words have to agree exactly.
fn is_ordinal(word: &str) -> bool {
    word.chars().all(|ch| ch.is_ascii_digit())
        || matches!(
            word,
            "i" | "ii" | "iii" | "iv" | "v" | "vi" | "vii" | "viii" | "ix" | "x"
        )
}

/// Whether two titles are the same album. `Use Your Illusion I` and `Use Your Illusion II` are
/// nearly the same text and entirely different albums.
pub fn same_album_title(a: &str, b: &str) -> bool {
    let (a, b) = (name_words(&album_title(a)), name_words(&album_title(b)));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let ordinals = |words: &[String]| -> Vec<String> {
        words
            .iter()
            .filter(|word| is_ordinal(word))
            .cloned()
            .collect()
    };
    if ordinals(&a) != ordinals(&b) {
        return false;
    }
    let (a, b) = (a.join(" "), b.join(" "));
    a == b || jaro_winkler(&a, &b) >= SAME_NAME
}

/// The artist a group of files agrees on: the album artist, or else the artist most of its files
/// name. Files that don't agree have none, which is what a compilation looks like.
pub fn group_artist(files: &[ReleaseFile]) -> Option<String> {
    fn commonest<'a>(names: impl Iterator<Item = &'a str>, needed: usize) -> Option<String> {
        let mut counts: HashMap<String, (usize, &'a str)> = HashMap::new();
        for name in names.filter(|name| !name.trim().is_empty()) {
            let entry = counts
                .entry(normalize_words(name))
                .or_insert((0usize, name));
            entry.0 += 1;
        }
        counts
            .into_values()
            .max_by_key(|(count, _)| *count)
            .filter(|(count, _)| *count >= needed)
            .map(|(_, name)| name.to_string())
    }
    let needed = files.len().div_ceil(2).max(1);
    commonest(
        files.iter().filter_map(|f| f.tags.album_artist.as_deref()),
        needed,
    )
    .or_else(|| {
        commonest(
            files.iter().filter_map(|f| f.tags.artist.as_deref()),
            needed,
        )
    })
}

/// The files of one release, as far as they can tell.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicGroup {
    /// The album their tags name, without what's about the release.
    pub title: Option<String>,
    /// The artist their tags agree on.
    pub artist: Option<String>,
    /// Indexes into the files that were grouped.
    pub files: Vec<usize>,
}

/// The folder a release lives in, which is above any disc folders.
fn release_folder(path: &str) -> String {
    let mut parts: Vec<&str> = path.split(['/', '\\']).collect();
    parts.pop();
    if disc_from_folder(path).is_some() {
        parts.pop();
    }
    parts.join("/")
}

/// Sort files into releases: those that name one album in one folder. Two folders of one album stay
/// apart, because they may be two copies of it, and worked out in turn they end up on one album
/// with the second copy's repeats left over, rather than spilling onto its other tracks. Files with
/// no album tag go by their folder, and the caller works out what the folder means.
pub fn group_release_files(files: &[ReleaseFile]) -> Vec<MusicGroup> {
    let mut by_folder: Vec<(String, String, Vec<usize>)> = Vec::new();
    for (index, file) in files.iter().enumerate() {
        let album = file
            .tags
            .album
            .as_deref()
            .filter(|album| !album.trim().is_empty())
            .map(album_title)
            .unwrap_or_default();
        let folder = release_folder(&file.path);
        let key = name_words(&album).join(" ");
        match by_folder
            .iter_mut()
            .find(|(k, f, _)| *k == key && *f == folder)
        {
            Some((_, _, list)) => list.push(index),
            None => by_folder.push((key, folder, vec![index])),
        }
    }
    by_folder
        .into_iter()
        .map(|(_, _, list)| {
            let members: Vec<ReleaseFile> =
                list.iter().map(|index| files[*index].clone()).collect();
            MusicGroup {
                title: members
                    .iter()
                    .find_map(|file| file.tags.album.as_deref())
                    .filter(|album| !album.trim().is_empty())
                    .map(album_title),
                artist: group_artist(&members),
                files: list,
            }
        })
        .collect()
}

/// The disc number in a folder's name: `CD2`, `Disc 2`, `disk_2`.
fn disc_from_folder(path: &str) -> Option<i32> {
    let mut parts = path.rsplit(['/', '\\']);
    parts.next();
    let folder = parts.next()?.to_ascii_lowercase();
    let folder = folder.trim();
    let rest = ["cd", "disc", "disk"]
        .iter()
        .find_map(|prefix| folder.strip_prefix(prefix))?;
    let digits = rest.trim_start_matches([' ', '_', '-', '.']);
    (!digits.is_empty() && digits.len() <= 2 && digits.chars().all(|ch| ch.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

/// The disc and track in a scene-style name, where `206-foo_fighters-wattershed` is disc 2, track
/// 6. A tagged track number has to agree, so `100 - Title` on a long album stays track 100.
fn disc_and_track_from_name(path: &str, tagged_track: Option<i32>) -> Option<(i32, i32)> {
    let stem = path.rsplit(['/', '\\']).next()?;
    let digits: String = stem.chars().take_while(char::is_ascii_digit).collect();
    if digits.len() != 3 {
        return None;
    }
    let after = stem[digits.len()..].chars().next()?;
    if !matches!(after, '-' | '_' | ' ' | '.') {
        return None;
    }
    let disc: i32 = digits[..1].parse().ok()?;
    let track: i32 = digits[1..].parse().ok()?;
    if disc == 0 || track == 0 || tagged_track.is_some_and(|tagged| tagged != track) {
        return None;
    }
    Some((disc, track))
}

/// Fill in the discs a release's tags leave out, from its folders and file names. Both have to be
/// consistent across the release: one file named `101` doesn't make an album two discs.
pub fn fill_in_discs(files: &mut [ReleaseFile]) {
    let from_folders: Vec<Option<i32>> = files
        .iter()
        .map(|file| disc_from_folder(&file.path))
        .collect();
    let from_names: Vec<Option<(i32, i32)>> = files
        .iter()
        .map(|file| disc_and_track_from_name(&file.path, file.tags.track))
        .collect();
    let every_name_has_one = from_names.iter().all(Option::is_some);
    for (index, file) in files.iter_mut().enumerate() {
        if file.tags.disc.is_none() {
            file.tags.disc = from_folders[index]
                .or_else(|| every_name_has_one.then(|| from_names[index].map(|(disc, _)| disc))?);
        }
        if file.tags.track.is_none()
            && every_name_has_one
            && let Some((_, track)) = from_names[index]
        {
            file.tags.track = Some(track);
        }
    }
}

/// How well a release's files fit one listing of an album.
#[derive(Debug, Clone, PartialEq)]
pub struct ListingFit {
    /// The listed track each file goes to, by file.
    pub placed: Vec<Option<usize>>,
    /// Files that found a track, as a share of the files.
    pub share: f64,
    /// Listed tracks with no file.
    pub missing: usize,
}

/// Fit files to a listing. `names` are what the files are called, where their tags don't say.
pub fn fit_listing(
    files: &[ReleaseFile],
    names: &[Option<String>],
    listing: &[TrackFacts],
) -> ListingFit {
    let chosen = match_tracks(files, names, listing);
    let mut placed: Vec<Option<usize>> = chosen
        .iter()
        .map(|choice| choice.map(|(track, _)| track))
        .collect();
    // A track with the file's own title and length is that file, whatever number it has. Bonus
    // tracks on one disc can't keep the number their files carry, and that alone mustn't lose them.
    for (index, file) in files.iter().enumerate() {
        if placed[index].is_some() {
            continue;
        }
        let Some(title) = file.tags.title.as_deref().or(names[index].as_deref()) else {
            continue;
        };
        let wanted = normalize_words(title);
        let found = listing.iter().enumerate().find(|(track_index, track)| {
            !placed.contains(&Some(*track_index))
                && normalize_words(&track.title) == wanted
                && match (file.duration, track.seconds) {
                    (Some(have), Some(listed)) => (have - listed).abs() <= 3.0,
                    _ => true,
                }
        });
        if let Some((track_index, _)) = found {
            placed[index] = Some(track_index);
        }
    }
    let found = placed.iter().filter(|choice| choice.is_some()).count();
    ListingFit {
        placed,
        share: found as f64 / files.len().max(1) as f64,
        missing: listing.len().saturating_sub(found),
    }
}

/// Which listing to use: the one most of the files fit, and then the one with the fewest tracks
/// left over, so twelve files take the twelve-track album and not the four-disc box set.
pub fn best_listing(fits: &[ListingFit]) -> Option<usize> {
    fits.iter()
        .enumerate()
        .filter(|(_, fit)| fit.share > 0.0)
        .max_by(|(ai, a), (bi, b)| {
            a.share
                .partial_cmp(&b.share)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.missing.cmp(&a.missing))
                // The earlier listing wins a tie: callers put the likelier ones first.
                .then(bi.cmp(ai))
        })
        .map(|(index, _)| index)
}

/// Which of a listing's tracks an album should take: every disc some file is on. A box set's
/// Blu-ray or a bonus disc nobody has stays out, rather than showing as missing for ever. With no
/// file placed at all, there's nothing to go on and all of it stays.
pub fn tracks_on_held_discs(fit: &ListingFit, discs: &[i32]) -> Vec<usize> {
    let held: Vec<i32> = fit
        .placed
        .iter()
        .flatten()
        .filter_map(|index| discs.get(*index).copied())
        .collect();
    (0..discs.len())
        .filter(|index| held.is_empty() || held.contains(&discs[*index]))
        .collect()
}

/// The biggest edition not yet read, for files the nearest editions didn't fit: a deluxe edition's
/// bonus disc on its own is closer in count to the standard album than to the deluxe one.
pub fn biggest_unread_edition(counts: &[(usize, bool)], read: &[usize]) -> Option<usize> {
    (0..counts.len())
        .filter(|index| !read.contains(index))
        .max_by_key(|index| (counts[*index].0, counts[*index].1))
}

/// Which editions are worth a request, likeliest first. Each one is a call to a rate-limited
/// service, so only the track counts nearest the files are looked at: an exact count first, then
/// official releases, then the closest.
pub fn editions_to_read(counts: &[(usize, bool)], files: usize, at_most: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..counts.len()).collect();
    order.sort_by_key(|index| {
        let (count, official) = counts[*index];
        (count.abs_diff(files), !official, *index)
    });
    order.truncate(at_most);
    order
}

/// A track number for a file that's on no listing, in a free place on its disc. Its own number
/// when that's free, otherwise after the last track on the disc.
pub fn free_track_number(taken: &[(i32, i32)], disc: i32, wanted: Option<i32>) -> i32 {
    let on_disc = || {
        taken
            .iter()
            .filter(move |(d, _)| *d == disc)
            .map(|(_, n)| *n)
    };
    match wanted {
        Some(number) if number > 0 && !on_disc().any(|n| n == number) => number,
        _ => on_disc().max().unwrap_or(0) + 1,
    }
}

/// A naming pattern that can tell a many-disc album's tracks apart. Every disc has a track 1, so a
/// pattern without the disc would send them all to one file name.
pub fn pattern_for_discs(pattern: &str, discs: i32) -> String {
    if discs <= 1 || pattern.contains("{disc") {
        return pattern.to_string();
    }
    match pattern.rfind('/') {
        Some(slash) => format!("{}/{{disc}}-{}", &pattern[..slash], &pattern[slash + 1..]),
        None => format!("{{disc}}-{pattern}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::release_match::FileTags;

    fn file(
        path: &str,
        title: &str,
        track: Option<i32>,
        disc: Option<i32>,
        secs: f64,
    ) -> ReleaseFile {
        ReleaseFile {
            key: path.to_string(),
            path: path.to_string(),
            duration: Some(secs),
            tags: FileTags {
                title: Some(title.to_string()),
                track,
                disc,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn listed(title: &str, disc: i32, number: i32, secs: f64) -> TrackFacts {
        TrackFacts {
            title: title.to_string(),
            number: Some(number),
            disc: Some(disc),
            seconds: Some(secs),
        }
    }

    #[test]
    fn titles_lose_what_is_about_the_release() {
        assert_eq!(
            album_title("Use Your Illusion I (Deluxe_Edition)"),
            "Use Your Illusion I"
        );
        assert_eq!(
            album_title("Appetite For Destruction (Super Deluxe)"),
            "Appetite For Destruction"
        );
        assert_eq!(album_title("Abbey Road - Disc 2"), "Abbey Road");
        assert_eq!(album_title("Nevermind Deluxe Edition"), "Nevermind");
        assert_eq!(album_title("Foo Fighters"), "Foo Fighters");
    }

    #[test]
    fn a_title_that_is_only_edition_words_stays() {
        assert_eq!(album_title("(Deluxe)"), "(Deluxe)");
        assert_eq!(album_title("Deluxe"), "Deluxe");
    }

    #[test]
    fn numbered_albums_are_not_the_same_album() {
        assert!(!same_album_title(
            "Use Your Illusion I",
            "Use Your Illusion II"
        ));
        assert!(!same_album_title("Led Zeppelin II", "Led Zeppelin"));
        assert!(!same_album_title("Volume 2", "Volume 3"));
        assert!(same_album_title(
            "Use Your Illusion I (Deluxe_Edition)",
            "Use Your Illusion I"
        ));
        assert!(same_album_title(
            "Appetite For Destruction (Super Deluxe)",
            "Appetite for Destruction"
        ));
        assert!(same_album_title("The Wall", "Wall"));
    }

    #[test]
    fn an_album_of_the_same_name_by_someone_else_is_another_artist() {
        assert!(same_artist("Guns N' Roses", "Guns N’ Roses"));
        assert!(same_artist("The Beatles", "Beatles"));
        assert!(same_artist("Simon and Garfunkel", "Simon & Garfunkel"));
        assert!(!same_artist("Guns N' Roses", "Xzibit"));
        assert!(!same_artist("Guns N' Roses", "The Roses"));
        assert!(!same_artist("", "Xzibit"));
    }

    #[test]
    fn the_group_artist_is_what_the_files_agree_on() {
        let tagged = |album_artist: Option<&str>, artist: &str| ReleaseFile {
            tags: FileTags {
                album_artist: album_artist.map(str::to_string),
                artist: Some(artist.to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let band = vec![tagged(None, "Foo Fighters"), tagged(None, "Foo Fighters")];
        assert_eq!(group_artist(&band).as_deref(), Some("Foo Fighters"));
        let guests = vec![
            tagged(Some("Santana"), "Santana feat. Rob Thomas"),
            tagged(Some("Santana"), "Santana feat. Eric Clapton"),
        ];
        assert_eq!(group_artist(&guests).as_deref(), Some("Santana"));
        let compilation = vec![
            tagged(None, "One"),
            tagged(None, "Two"),
            tagged(None, "Three"),
        ];
        assert_eq!(group_artist(&compilation), None);
    }

    fn album_file(path: &str, album: Option<&str>, artist: Option<&str>) -> ReleaseFile {
        ReleaseFile {
            path: path.to_string(),
            tags: FileTags {
                album: album.map(str::to_string),
                artist: artist.map(str::to_string),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn each_folder_of_an_album_is_worked_out_in_turn() {
        let files = vec![
            album_file(
                "box/CD1/01.flac",
                Some("Use Your Illusion I (Deluxe_Edition)"),
                Some("GNR"),
            ),
            album_file("ipod/01.m4a", Some("Use Your Illusion I"), Some("GNR")),
            album_file("ipod/02.m4a", Some("Use Your Illusion II"), Some("GNR")),
        ];
        let groups = group_release_files(&files);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].title.as_deref(), Some("Use Your Illusion I"));
        assert_eq!(groups[1].files, vec![1]);
    }

    #[test]
    fn the_same_title_by_two_artists_is_two_releases() {
        let files = vec![
            album_file("a/01.flac", Some("Greatest Hits"), Some("Queen")),
            album_file("b/01.flac", Some("Greatest Hits"), Some("ABBA")),
        ];
        assert_eq!(group_release_files(&files).len(), 2);
    }

    #[test]
    fn disc_folders_are_one_release() {
        let files = vec![
            album_file(
                "box/CD1/01.flac",
                Some("Appetite (Super Deluxe)"),
                Some("GNR"),
            ),
            album_file(
                "box/CD2/01.flac",
                Some("Appetite (Super Deluxe)"),
                Some("GNR"),
            ),
        ];
        let groups = group_release_files(&files);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files, vec![0, 1]);
    }

    #[test]
    fn files_with_no_album_go_by_their_folder() {
        let files = vec![
            album_file("Band/Album/01.flac", None, None),
            album_file("Band/Album/02.flac", None, None),
            album_file("Band/Other/01.flac", None, None),
        ];
        let groups = group_release_files(&files);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].title, None);
    }

    #[test]
    fn scene_names_carry_the_disc() {
        let mut files = vec![
            file(
                "rel/101-foo_fighters-this_is_a_call.flac",
                "This Is A Call",
                Some(1),
                None,
                233.0,
            ),
            file(
                "rel/206-foo_fighters-wattershed_(live).flac",
                "Wattershed (Live)",
                Some(6),
                None,
                135.0,
            ),
        ];
        fill_in_discs(&mut files);
        assert_eq!(files[0].tags.disc, Some(1));
        assert_eq!(files[1].tags.disc, Some(2));
        assert_eq!(files[1].tags.track, Some(6));
    }

    #[test]
    fn one_three_digit_name_does_not_make_two_discs() {
        let mut files = vec![
            file("rel/100 - Hundred.flac", "Hundred", Some(100), None, 200.0),
            file(
                "rel/99 - Ninety Nine.flac",
                "Ninety Nine",
                Some(99),
                None,
                200.0,
            ),
        ];
        fill_in_discs(&mut files);
        assert_eq!(files[0].tags.disc, None);
        assert_eq!(files[1].tags.disc, None);
    }

    #[test]
    fn folders_carry_the_disc_and_tags_win() {
        let mut files = vec![
            file("rel/CD3/01 - A.flac", "A", Some(1), None, 100.0),
            file("rel/Disc 2/01 - B.flac", "B", Some(1), None, 100.0),
            file("rel/CD3/02 - C.flac", "C", Some(2), Some(1), 100.0),
            file("rel/iPod Lossless/03 - D.flac", "D", Some(3), None, 100.0),
        ];
        fill_in_discs(&mut files);
        assert_eq!(files[0].tags.disc, Some(3));
        assert_eq!(files[1].tags.disc, Some(2));
        assert_eq!(files[2].tags.disc, Some(1));
        assert_eq!(files[3].tags.disc, None);
    }

    #[test]
    fn twelve_files_take_the_twelve_track_album_and_not_the_box_set() {
        let standard: Vec<TrackFacts> = (1..=12)
            .map(|n| listed(&format!("Song {n}"), 1, n, 200.0 + f64::from(n) * 11.0))
            .collect();
        let mut boxed = standard.clone();
        boxed.extend((1..=39).map(|n| listed(&format!("Extra {n}"), 2, n, 300.0)));
        let files: Vec<ReleaseFile> = (1..=12)
            .map(|n| {
                file(
                    &format!("{n:02}.flac"),
                    &format!("Song {n}"),
                    Some(n),
                    None,
                    200.0 + f64::from(n) * 11.0,
                )
            })
            .collect();
        let names = vec![None; files.len()];
        let fits = vec![
            fit_listing(&files, &names, &boxed),
            fit_listing(&files, &names, &standard),
        ];
        assert_eq!(fits[0].share, 1.0);
        assert_eq!(fits[0].missing, 39);
        assert_eq!(fits[1].missing, 0);
        assert_eq!(best_listing(&fits), Some(1));
    }

    #[test]
    fn fifty_one_files_take_the_box_set() {
        let standard: Vec<TrackFacts> = (1..=12)
            .map(|n| listed(&format!("Song {n}"), 1, n, 200.0 + f64::from(n) * 11.0))
            .collect();
        let mut boxed = standard.clone();
        boxed.extend(
            (1..=39).map(|n| listed(&format!("Extra {n}"), 2, n, 300.0 + f64::from(n) * 7.0)),
        );
        let mut files: Vec<ReleaseFile> = (1..=12)
            .map(|n| {
                file(
                    &format!("CD1/{n:02}.flac"),
                    &format!("Song {n}"),
                    Some(n),
                    Some(1),
                    200.0 + f64::from(n) * 11.0,
                )
            })
            .collect();
        files.extend((1..=39).map(|n| {
            file(
                &format!("CD2/{n:02}.flac"),
                &format!("Extra {n}"),
                Some(n),
                Some(2),
                300.0 + f64::from(n) * 7.0,
            )
        }));
        let names = vec![None; files.len()];
        let fits = vec![
            fit_listing(&files, &names, &standard),
            fit_listing(&files, &names, &boxed),
        ];
        assert_eq!(fits[1].share, 1.0);
        assert_eq!(best_listing(&fits), Some(1));
    }

    #[test]
    fn track_one_on_disc_two_is_not_track_one_on_disc_one() {
        let listing = vec![
            listed("Winnebago", 2, 1, 200.0),
            listed("This Is a Call", 1, 1, 233.0),
        ];
        let files = vec![
            file("101.flac", "This Is A Call", Some(1), Some(1), 233.0),
            file("201.flac", "Winnebago", Some(1), Some(2), 200.0),
        ];
        let fit = fit_listing(&files, &[None, None], &listing);
        assert_eq!(fit.placed, vec![Some(1), Some(0)]);
    }

    #[test]
    fn a_listing_nothing_fits_is_not_chosen() {
        let fits = vec![ListingFit {
            placed: vec![None],
            share: 0.0,
            missing: 3,
        }];
        assert_eq!(best_listing(&fits), None);
    }

    #[test]
    fn a_disc_nobody_has_stays_out_of_the_album() {
        let fit = ListingFit {
            placed: vec![Some(0), Some(2)],
            share: 1.0,
            missing: 2,
        };
        assert_eq!(tracks_on_held_discs(&fit, &[1, 1, 2, 5]), vec![0, 1, 2]);
        let none = ListingFit {
            placed: vec![None],
            share: 0.0,
            missing: 2,
        };
        assert_eq!(tracks_on_held_discs(&none, &[1, 2]), vec![0, 1]);
    }

    #[test]
    fn a_bonus_track_numbered_after_the_listing_still_takes_its_file() {
        // The file says track 5, but track 5 is the studio version, so its row became 18.
        let listing = vec![
            listed("Dust N' Bones", 1, 5, 298.0),
            listed("Dust N' Bones (Live In London)", 1, 18, 311.0),
        ];
        let files = vec![file(
            "05.flac",
            "Dust N’ Bones (Live In London)",
            Some(5),
            None,
            311.0,
        )];
        let fit = fit_listing(&files, &[None], &listing);
        assert_eq!(fit.placed, vec![Some(1)]);
    }

    #[test]
    fn a_title_match_of_another_length_is_not_the_same_recording() {
        let listing = vec![listed("Coma", 1, 9, 613.0)];
        let files = vec![file("01.flac", "Coma", Some(1), None, 540.0)];
        assert_eq!(fit_listing(&files, &[None], &listing).placed, vec![None]);
    }

    #[test]
    fn the_biggest_edition_is_tried_when_the_nearest_did_not_fit() {
        let counts = [(16, true), (29, true), (16, false), (14, true)];
        assert_eq!(biggest_unread_edition(&counts, &[0, 2, 3]), Some(1));
        assert_eq!(biggest_unread_edition(&counts, &[0, 1, 2, 3]), None);
    }

    #[test]
    fn the_nearest_editions_are_read_first() {
        // Track counts and whether each is official. Twelve files.
        let counts = [(51, true), (12, false), (14, true), (12, true), (13, true)];
        assert_eq!(editions_to_read(&counts, 12, 3), vec![3, 1, 4]);
    }

    #[test]
    fn a_file_on_no_listing_keeps_its_number_when_it_can() {
        let taken = [(1, 1), (1, 2), (2, 1)];
        assert_eq!(free_track_number(&taken, 1, Some(5)), 5);
        assert_eq!(free_track_number(&taken, 1, Some(2)), 3);
        assert_eq!(free_track_number(&taken, 2, None), 2);
        assert_eq!(free_track_number(&taken, 3, None), 1);
    }

    #[test]
    fn a_many_disc_album_gets_the_disc_in_its_file_names() {
        let pattern = "{artist}/{album} ({year})/{track:02} - {title}.{ext}";
        assert_eq!(pattern_for_discs(pattern, 1), pattern);
        assert_eq!(
            pattern_for_discs(pattern, 2),
            "{artist}/{album} ({year})/{disc}-{track:02} - {title}.{ext}"
        );
        let own = "{artist}/{album}/Disc {disc}/{track:02} - {title}.{ext}";
        assert_eq!(pattern_for_discs(own, 4), own);
    }
}
