//! Coverage plans for an artist's missing tracks (Q110): discography packs, albums, and the smallest
//! release holding a single track.
//!
//! Pure code. The recommender gathers releases (feed catalogue, a discography search, album
//! searches, track searches), says what each one claims ([`music_claim`]), looks into the most
//! promising ones (`torrent::judge` with `WantedItems::ArtistTracks`), and hands the result to
//! the show plans' [`plan_cover`](super::recommend_plan::plan_cover): within a quality tier the
//! release adding the most missing tracks goes first, so a discography pack beats albums and an
//! album beats its tracks; between equals, the healthier, then the smaller.

use crate::services::feed_catalogue::read_music_title;
use crate::services::music_rules::{same_album_title, same_artist, same_song_title};
use crate::services::quality::title_match::tokens;

/// One missing track of the artist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingTrack {
    pub id: String,
    pub album_id: String,
    pub album: String,
    pub number: i32,
    pub title: String,
}

/// What a music release's name says it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MusicClaim {
    /// A discography or a set of the artist's albums: what it holds is known from its files.
    Discography,
    /// One of the artist's albums, by its index in the album list.
    Album(usize),
    /// Named after missing tracks (a single): their indexes into the missing list.
    Tracks(Vec<usize>),
    /// Another release of the artist (an EP, a compilation, a live album): usable only once its
    /// files show it holds a missing track.
    Other,
}

impl MusicClaim {
    /// The scope a recommendation row gets for a release of this claim.
    pub fn scope(&self) -> &'static str {
        match self {
            Self::Discography => "discography",
            Self::Album(_) => "album",
            Self::Tracks(_) | Self::Other => "track",
        }
    }

    /// Whether only its files can say what it holds.
    pub fn needs_files(&self) -> bool {
        matches!(self, Self::Discography | Self::Other)
    }
}

/// Words that make a release a discography or a set of albums.
fn says_discography(title: &str) -> bool {
    let words = tokens(title);
    words.iter().any(|word| {
        matches!(
            word.as_str(),
            "discography" | "discografia" | "discographie" | "diskografie" | "anthology"
        )
    }) || words
        .windows(2)
        .any(|pair| matches!((pair[0].as_str(), pair[1].as_str()), ("complete", "albums") | ("studio", "albums") | ("all", "albums") | ("albums", "collection")))
}

/// What a release's name claims, for an artist with these albums and missing tracks. `None` when it
/// names another artist.
pub fn music_claim(
    title: &str,
    artist: &str,
    albums: &[String],
    missing: &[MissingTrack],
) -> Option<MusicClaim> {
    let facts = read_music_title(title);
    if let Some(found) = facts.artist.as_deref()
        && !found.is_empty()
        && !same_artist(found, artist)
    {
        return None;
    }
    if says_discography(title) {
        return Some(MusicClaim::Discography);
    }
    let Some(named) = facts.album.as_deref().filter(|named| !named.is_empty()) else {
        return Some(MusicClaim::Other);
    };
    if let Some(index) = albums.iter().position(|album| same_album_title(album, named)) {
        return Some(MusicClaim::Album(index));
    }
    // `Artist - Song Title (Single)`, `Artist - Song Title [WEB]`.
    let stripped = strip_single_words(named);
    let tracks: Vec<usize> = missing
        .iter()
        .enumerate()
        .filter(|(_, track)| same_song_title(&stripped, &track.title))
        .map(|(index, _)| index)
        .collect();
    if !tracks.is_empty() {
        return Some(MusicClaim::Tracks(tracks));
    }
    Some(MusicClaim::Other)
}

/// A name without the words that mark a single or an EP.
fn strip_single_words(named: &str) -> String {
    named
        .split_whitespace()
        .filter(|word| {
            !matches!(
                word.to_ascii_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric()),
                "single" | "ep" | "cds" | "promo"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The indexes of `missing` a claim covers by name alone.
pub fn claimed_tracks(claim: &MusicClaim, albums: &[String], missing: &[MissingTrack]) -> Vec<usize> {
    match claim {
        MusicClaim::Album(index) => {
            let Some(album) = albums.get(*index) else {
                return Vec::new();
            };
            missing
                .iter()
                .enumerate()
                .filter(|(_, track)| &track.album == album)
                .map(|(index, _)| index)
                .collect()
        }
        MusicClaim::Tracks(tracks) => tracks.clone(),
        MusicClaim::Discography | MusicClaim::Other => Vec::new(),
    }
}

/// What kind of release it is, for "from a 600 MB album".
pub fn release_kind(claim: &MusicClaim, title: &str) -> &'static str {
    let words = tokens(title);
    let has = |word: &str| words.iter().any(|found| found == word);
    match claim {
        MusicClaim::Discography => "discography pack",
        MusicClaim::Album(_) => "album",
        _ if has("ep") => "EP",
        _ if has("single") => "single",
        MusicClaim::Tracks(_) => "single",
        _ if has("compilation") || has("hits") || has("best") || has("collection") => {
            "compilation"
        }
        _ => "release",
    }
}

/// `8 MB`, `1.2 GB`.
pub fn human_size(bytes: i64) -> String {
    let bytes = bytes.max(0) as f64;
    if bytes >= 1e9 {
        format!("{:.1} GB", bytes / 1e9)
    } else if bytes >= 1e6 {
        format!("{:.0} MB", (bytes / 1e6).max(1.0))
    } else {
        format!("{:.0} KB", (bytes / 1e3).max(1.0))
    }
}

/// What a recommended music release downloads, for its row: "Downloads 1 track (8 MB) from a 600 MB
/// album". `selected` is the size of exactly the files that will be fetched, when its files were
/// read; `total` the release's size. With "only the missing files" off, the whole release is
/// downloaded and the line says so. The caller leaves it out when the whole release is wanted.
pub fn download_note(
    tracks: usize,
    selected: Option<i64>,
    total: Option<i64>,
    kind: &str,
    only_missing_files: bool,
) -> String {
    let noun = if tracks == 1 { "track" } else { "tracks" };
    let release = match total {
        Some(total) => format!("{} {kind}", human_size(total)),
        None => kind.to_string(),
    };
    if !only_missing_files {
        return format!("Downloads the whole {release} for {tracks} missing {noun}");
    }
    let article = if release.starts_with(['a', 'e', 'i', 'o', 'u', 'E', '8']) {
        "an"
    } else {
        "a"
    };
    match selected {
        Some(selected) => format!(
            "Downloads {tracks} {noun} ({}) from {article} {release}",
            human_size(selected)
        ),
        None => format!("Downloads only the {tracks} missing {noun} from {article} {release}"),
    }
}

/// What is still missing after an artist's plan, as a person would say it: `Album A (3 tracks),
/// all of Album B`.
pub fn describe_tracks(missing: &[MissingTrack], items: &[usize]) -> String {
    let mut albums: Vec<(String, usize)> = Vec::new();
    for index in items {
        let Some(track) = missing.get(*index) else {
            continue;
        };
        match albums.iter_mut().find(|(album, _)| *album == track.album) {
            Some((_, count)) => *count += 1,
            None => albums.push((track.album.clone(), 1)),
        }
    }
    let mut parts: Vec<String> = albums
        .iter()
        .map(|(album, count)| {
            let whole = missing.iter().filter(|track| &track.album == album).count();
            if *count == whole && whole > 2 {
                format!("all of '{album}'")
            } else if *count == 1 {
                let title = items
                    .iter()
                    .filter_map(|index| missing.get(*index))
                    .find(|track| &track.album == album)
                    .map(|track| track.title.clone())
                    .unwrap_or_default();
                format!("'{title}' from '{album}'")
            } else {
                format!("{count} tracks of '{album}'")
            }
        })
        .collect();
    if parts.len() > 5 {
        let more = parts.len() - 4;
        parts.truncate(4);
        parts.push(format!("and {more} more albums"));
    }
    parts.join(", ")
}

/// The album ids a set of missing tracks belongs to, in order, each once.
pub fn albums_of(missing: &[MissingTrack], items: &[usize]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for index in items {
        if let Some(track) = missing.get(*index)
            && !out.contains(&track.album_id)
        {
            out.push(track.album_id.clone());
        }
    }
    out
}

#[cfg(test)]
#[path = "recommend_music_tests.rs"]
mod tests;
