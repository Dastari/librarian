//! The RSS catalogue: reading a feed item fully, and saying what it is.
//!
//! A feed lists what a tracker just released. Besides being a cheap way to find a wanted episode
//! (`jobs::rss_poll`), every item is kept as a catalogue of torrents that exist right now:
//! what the title says (year, season, episode, pack, quality, group, size, category, and for
//! music artist, album, format and bitrate) and, when one matches, which item in the library it is.
//!
//! This module is pure code with no database or network. It reuses what the hunt already trusts:
//! `quality::scoring::parse_release` for the title, `rss_poll`'s exact-name rules ("The Office" is
//! not "The Office US"), `music_rules` for artists and albums and `release_match` for books.
//!
//! A match here is a *label*, shown in the Feed view and used to find an item's releases. It never
//! authorises a grab by itself: grabbing re-checks the target's [`Expectation`], which still
//! refuses a title that does not say its episode, season or year.
//!
//! [`Expectation`]: crate::jobs::auto_schedule::Expectation

use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::jobs::rss_poll::{acceptable_names, title_prefix};
use crate::services::quality::scoring::{self, ParsedRelease};
use crate::services::{identify_rules, music_rules, release_match};

// ===========================================================================
// What an item says it is
// ===========================================================================

/// The broad kind of thing a release is, from its tracker category and its title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Tv,
    Movie,
    Music,
    Audiobook,
    /// Games, software, ebooks and anything else the library has no home for.
    Other,
    /// Nothing says; matching tries TV and then movies.
    Unknown,
}

/// What the release covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Episode,
    SeasonPack,
    SeriesPack,
    Movie,
    Album,
    Book,
    Unknown,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Episode => "episode",
            Self::SeasonPack => "season_pack",
            Self::SeriesPack => "series_pack",
            Self::Movie => "movie",
            Self::Album => "album",
            Self::Book => "book",
            Self::Unknown => "unknown",
        }
    }
}

/// Artist, album, format and bitrate as a music or audiobook title gives them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MusicFacts {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub format: Option<String>,
    pub bitrate: Option<String>,
}

/// Everything read from one feed item.
#[derive(Debug, Clone)]
pub struct ItemFacts {
    pub kind: ItemKind,
    pub scope: Scope,
    pub size: Option<i64>,
    /// The tracker's category label (`Movie/HD/Bluray`), when the description carries one.
    pub category: Option<String>,
    pub parsed: ParsedRelease,
    /// The show or movie title the release is named after, flattened.
    pub name: String,
    pub music: MusicFacts,
}

impl ItemFacts {
    pub fn year(&self) -> Option<i32> {
        self.parsed.year
    }
}

static SIZE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(\d+(?:[.,]\d+)?)\s*(TiB|GiB|MiB|KiB|TB|GB|MB|KB|B)\b").expect("size pattern")
});

/// A size in bytes from text such as `12.1 GB`, and what follows it.
pub fn parse_size_text(text: &str) -> Option<(i64, String)> {
    let found = SIZE_RE.captures(text)?;
    let number: f64 = found[1].replace(',', ".").parse().ok()?;
    let multiplier = match found[2].to_ascii_uppercase().as_str() {
        "B" => 1.0,
        "KB" | "KIB" => 1024.0,
        "MB" | "MIB" => 1024.0 * 1024.0,
        "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
        "TB" | "TIB" => 1024.0_f64.powi(4),
        _ => return None,
    };
    let rest = &text[found.get(0)?.end()..];
    let rest =
        rest.trim_start_matches(|c: char| c == ';' || c == ',' || c == '|' || c.is_whitespace());
    Some(((number * multiplier) as i64, rest.trim().to_string()))
}

/// The kind of release a tracker category label or Torznab number names, if it names one.
fn kind_from_category(label: Option<&str>, numbers: &[i32]) -> Option<ItemKind> {
    if let Some(label) = label {
        let lower = label.to_ascii_lowercase();
        if lower.contains("audiobook") || lower.contains("audio book") {
            return Some(ItemKind::Audiobook);
        }
        if lower.contains("movie") || lower.contains("film") {
            return Some(ItemKind::Movie);
        }
        if lower.contains("music")
            || lower.contains("flac")
            || lower.contains("lossless")
            || lower.contains("mp3")
            || lower.contains("podcast")
        {
            return Some(ItemKind::Music);
        }
        if lower.starts_with("tv")
            || lower.contains("tv/")
            || lower.contains("sport")
            || lower.contains("anime")
        {
            return Some(ItemKind::Tv);
        }
        if [
            "game",
            "appz",
            "mac",
            "mobile",
            "font",
            "book",
            "comic",
            "magazine",
            "educational",
            "misc",
            "pics",
        ]
        .iter()
        .any(|word| lower.contains(word))
        {
            return Some(ItemKind::Other);
        }
    }
    for number in numbers {
        match number {
            3030 => return Some(ItemKind::Audiobook),
            2000..=2999 => return Some(ItemKind::Movie),
            3000..=3999 => return Some(ItemKind::Music),
            5000..=5999 => return Some(ItemKind::Tv),
            1000..=1999 | 4000..=4999 | 6000..=7999 => return Some(ItemKind::Other),
            _ => {}
        }
    }
    None
}

static FORMAT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(flac|alac|mp3|aac|m4a|m4b|ogg|opus|wav|ape|wv)\b").expect("format pattern")
});
static BITRATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(\d{2,3}\s?kbps|v0|v1|v2|320|256|192|128|24\s?bit|16\s?bit|\d{2}[-/]\d{2,3}\s?khz)\b")
        .expect("bitrate pattern")
});
static TAG_GROUP_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\[\(\{][^\]\)\}]*[\]\)\}]").expect("tag group"));
static YEAR_WORD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(19\d{2}|20\d{2})\b").expect("year"));

/// A title as people write it: separators between words made into spaces.
fn spaced(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    for c in title.chars() {
        out.push(if c == '_' { ' ' } else { c });
    }
    // Scene names use dots between words (but keep "St." style abbreviations harmlessly spaced too).
    if !out.contains(' ') {
        out = out.replace('.', " ");
    }
    out
}

/// Music and audiobook titles: `Artist - Album (2019) [FLAC]`, `Artist - Album - 2019 - FLAC`, or
/// scene `Artist-Album-WEB-2019-GRP`. The parts that are not the name (years, formats, bitrates,
/// bracketed tags) are removed from artist and album.
pub fn read_music_title(title: &str) -> MusicFacts {
    let text = spaced(title);
    let format = FORMAT_RE
        .find(&text)
        .map(|m| m.as_str().to_ascii_uppercase());
    let bitrate = BITRATE_RE
        .find(&text)
        .map(|m| m.as_str().to_ascii_uppercase().replace(' ', ""));
    let without_groups = TAG_GROUP_RE.replace_all(&text, " ").into_owned();
    let clean = |part: &str| -> String {
        let no_year = YEAR_WORD_RE.replace_all(part, " ");
        let no_format = FORMAT_RE.replace_all(&no_year, " ");
        let no_rate = BITRATE_RE.replace_all(&no_format, " ");
        no_rate
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim_matches(|c: char| c == '-' || c == ':' || c.is_whitespace())
            .to_string()
    };
    let (artist, album) = if let Some((artist, rest)) = without_groups.split_once(" - ") {
        // `Artist - Album - 2019 - FLAC`: the album is up to the next separator.
        let album = rest.split(" - ").next().unwrap_or(rest);
        (clean(artist), clean(album))
    } else if let Some((artist, rest)) = without_groups.split_once('-') {
        let album: Vec<&str> = rest
            .split('-')
            .take_while(|part| {
                let word = part.trim().to_ascii_lowercase();
                !(YEAR_WORD_RE.is_match(&word)
                    || FORMAT_RE.is_match(&word)
                    || matches!(
                        word.as_str(),
                        "web" | "cd" | "vinyl" | "ep" | "single" | "cdr"
                    ))
            })
            .collect();
        (clean(artist), clean(&album.join(" ")))
    } else {
        (String::new(), clean(&without_groups))
    };
    MusicFacts {
        artist: Some(artist).filter(|name| !name.is_empty()),
        album: Some(album).filter(|name| !name.is_empty()),
        format,
        bitrate,
    }
}

/// Read one feed item: its title and, if the feed gives them, the description and categories.
pub fn read_item(
    title: &str,
    description: Option<&str>,
    categories: &[i32],
    known_size: Option<i64>,
) -> ItemFacts {
    let parsed = scoring::parse_release(title);
    let (described_size, category) = match description.and_then(parse_size_text) {
        Some((size, rest)) => (Some(size), Some(rest).filter(|rest| !rest.is_empty())),
        None => (
            None,
            description
                .map(str::trim)
                .filter(|text| !text.is_empty() && text.len() < 80 && text.contains('/'))
                .map(str::to_string),
        ),
    };
    let numbered_tv = parsed.season.is_some() || !parsed.episodes.is_empty();
    let mut kind = kind_from_category(category.as_deref(), categories).unwrap_or(ItemKind::Unknown);
    if numbered_tv && matches!(kind, ItemKind::Unknown | ItemKind::Movie | ItemKind::Other) {
        kind = ItemKind::Tv;
    }
    let scope = match kind {
        ItemKind::Tv => {
            if !parsed.episodes.is_empty() {
                Scope::Episode
            } else if (parsed.season.is_none()
                && parsed.season_end.is_none()
                && parsed.is_season_pack)
                || parsed
                    .season_end
                    .is_some_and(|end| Some(end) != parsed.season)
            {
                // A complete series, or a range of seasons.
                Scope::SeriesPack
            } else if parsed.is_season_pack {
                Scope::SeasonPack
            } else {
                Scope::Unknown
            }
        }
        ItemKind::Movie | ItemKind::Unknown => Scope::Movie,
        ItemKind::Music => Scope::Album,
        ItemKind::Audiobook => Scope::Book,
        ItemKind::Other => Scope::Unknown,
    };
    let music = if matches!(kind, ItemKind::Music | ItemKind::Audiobook) {
        read_music_title(title)
    } else {
        MusicFacts::default()
    };
    ItemFacts {
        kind,
        scope,
        size: known_size.or(described_size),
        category,
        name: title_prefix(title),
        parsed,
        music,
    }
}

// ===========================================================================
// What a library holds
// ===========================================================================

#[derive(Debug, Clone)]
pub struct ShowRef {
    pub id: String,
    pub library_id: String,
    pub name: String,
    pub year: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct MovieRef {
    pub id: String,
    pub library_id: String,
    pub title: String,
    pub year: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AlbumRef {
    pub id: String,
    pub library_id: String,
    pub artist: String,
    pub name: String,
    pub year: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct BookRef {
    pub id: String,
    pub library_id: String,
    pub title: String,
    pub author: Option<String>,
}

/// The titles a feed's owner has, indexed by every name a release may use for them.
#[derive(Debug, Default)]
pub struct TitleIndex {
    shows: Vec<ShowRef>,
    show_names: HashMap<String, Vec<usize>>,
    movies: Vec<MovieRef>,
    movie_names: HashMap<String, Vec<usize>>,
    albums: Vec<AlbumRef>,
    books: Vec<BookRef>,
}

impl TitleIndex {
    pub fn new(
        shows: Vec<ShowRef>,
        movies: Vec<MovieRef>,
        albums: Vec<AlbumRef>,
        books: Vec<BookRef>,
    ) -> Self {
        let mut show_names: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, show) in shows.iter().enumerate() {
            for name in acceptable_names(&show.name) {
                show_names.entry(name).or_default().push(index);
            }
        }
        let mut movie_names: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, movie) in movies.iter().enumerate() {
            for name in acceptable_names(&movie.title) {
                movie_names.entry(name).or_default().push(index);
            }
        }
        Self {
            shows,
            show_names,
            movies,
            movie_names,
            albums,
            books,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.shows.is_empty()
            && self.movies.is_empty()
            && self.albums.is_empty()
            && self.books.is_empty()
    }
}

// ===========================================================================
// Matching
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    Movie,
    /// A complete-series pack: the show as a whole.
    Show,
    Season,
    Episode,
    Album,
    Audiobook,
}

impl MatchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Show => "show",
            Self::Season => "season",
            Self::Episode => "episode",
            Self::Album => "album",
            Self::Audiobook => "audiobook",
        }
    }
}

/// What a feed item was taken to be, with how sure that is and why.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemMatch {
    pub kind: MatchKind,
    pub library_id: String,
    pub show_id: Option<String>,
    pub movie_id: Option<String>,
    pub album_id: Option<String>,
    pub audiobook_id: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    /// What to show: `Show S02E05`, `Movie (1990)`, `Artist - Album`.
    pub label: String,
    /// Percent.
    pub confidence: i32,
    pub reason: String,
}

fn blank(
    kind: MatchKind,
    library_id: &str,
    label: String,
    confidence: i32,
    reason: String,
) -> ItemMatch {
    ItemMatch {
        kind,
        library_id: library_id.to_string(),
        show_id: None,
        movie_id: None,
        album_id: None,
        audiobook_id: None,
        season: None,
        episode: None,
        label,
        confidence,
        reason,
    }
}

/// Pick among several titles one name could mean: one, or the one whose year the release gives.
fn pick<'a, T>(
    candidates: &[&'a T],
    year_of: impl Fn(&T) -> Option<i32>,
    item_year: Option<i32>,
) -> Result<&'a T, String> {
    if let [only] = candidates {
        return Ok(*only);
    }
    if let Some(year) = item_year {
        let agreeing: Vec<&'a T> = candidates
            .iter()
            .copied()
            .filter(|candidate| year_of(candidate).is_some_and(|found| (found - year).abs() <= 1))
            .collect();
        if let [only] = agreeing.as_slice() {
            return Ok(*only);
        }
    }
    Err("the name fits more than one title in the library".to_string())
}

fn match_tv(facts: &ItemFacts, index: &TitleIndex) -> Result<ItemMatch, String> {
    if facts.name.is_empty() {
        return Err("the title has no name to look up".to_string());
    }
    let Some(found) = index.show_names.get(&facts.name) else {
        return Err("no show in the library has this name".to_string());
    };
    let mut distinct: Vec<usize> = found.clone();
    distinct.sort_unstable();
    distinct.dedup();
    let shows: Vec<&ShowRef> = distinct
        .iter()
        .map(|index_of| &index.shows[*index_of])
        .collect();
    let show = pick(&shows, |show| show.year, facts.year())?;
    let hit = |kind, label: String, confidence, reason: &str| {
        let mut item = blank(
            kind,
            &show.library_id,
            label,
            confidence,
            reason.to_string(),
        );
        item.show_id = Some(show.id.clone());
        item
    };
    Ok(match facts.scope {
        Scope::Episode => {
            let season = facts.parsed.season.unwrap_or(1);
            let episode = facts.parsed.episodes.first().copied().unwrap_or_default();
            let mut item = hit(
                MatchKind::Episode,
                format!("{} S{season:02}E{episode:02}", show.name),
                100,
                "name matches exactly and the title gives the episode",
            );
            item.season = Some(season);
            item.episode = Some(episode);
            item
        }
        Scope::SeasonPack => {
            let season = facts.parsed.season.unwrap_or_default();
            let mut item = hit(
                MatchKind::Season,
                format!("{} S{season:02}", show.name),
                98,
                "name matches exactly and the title is a season pack",
            );
            item.season = Some(season);
            item
        }
        Scope::SeriesPack => hit(
            MatchKind::Show,
            format!("{} (complete series)", show.name),
            95,
            "name matches exactly and the title is a multi-season or complete pack",
        ),
        _ => {
            return Err(format!(
                "the name is '{}' but the title does not say which season or episode",
                show.name
            ));
        }
    })
}

fn match_movie(facts: &ItemFacts, index: &TitleIndex) -> Result<ItemMatch, String> {
    if facts.name.is_empty() {
        return Err("the title has no name to look up".to_string());
    }
    let Some(found) = index.movie_names.get(&facts.name) else {
        return Err("no movie in the library has this name".to_string());
    };
    let mut distinct: Vec<usize> = found.clone();
    distinct.sort_unstable();
    distinct.dedup();
    // A year more than one off is another film (a remake): drop those first.
    let movies: Vec<&MovieRef> = distinct
        .iter()
        .map(|index_of| &index.movies[*index_of])
        .filter(|movie| match (movie.year, facts.year()) {
            (Some(wanted), Some(found)) => (wanted - found).abs() <= 1,
            _ => true,
        })
        .collect();
    if movies.is_empty() {
        let years: Vec<String> = distinct
            .iter()
            .filter_map(|index_of| index.movies[*index_of].year.map(|year| year.to_string()))
            .collect();
        return Err(format!(
            "the name matches, but from {} where the library has {}",
            facts
                .year()
                .map(|year| year.to_string())
                .unwrap_or_default(),
            years.join("/")
        ));
    }
    let movie = pick(&movies, |movie| movie.year, facts.year())?;
    let label = match movie.year {
        Some(year) => format!("{} ({year})", movie.title),
        None => movie.title.clone(),
    };
    let (confidence, reason) = match (movie.year, facts.year()) {
        (Some(wanted), Some(found)) if wanted == found => (100, "name and year match"),
        (Some(_), Some(_)) => (90, "name matches and the year is one off"),
        (_, None) => (60, "the name matches but the title does not say its year"),
        (None, Some(_)) => (80, "the name matches; the library has no year to compare"),
    };
    let mut item = blank(
        MatchKind::Movie,
        &movie.library_id,
        label,
        confidence,
        reason.to_string(),
    );
    item.movie_id = Some(movie.id.clone());
    Ok(item)
}

fn match_album(facts: &ItemFacts, index: &TitleIndex) -> Result<ItemMatch, String> {
    let (Some(artist), Some(album)) = (facts.music.artist.as_deref(), facts.music.album.as_deref())
    else {
        return Err("the title does not give an artist and an album".to_string());
    };
    let found: Vec<&AlbumRef> = index
        .albums
        .iter()
        .filter(|candidate| {
            music_rules::same_artist(artist, &candidate.artist)
                && music_rules::same_album_title(album, &candidate.name)
        })
        .collect();
    if found.is_empty() {
        return Err("no album in the library has this artist and title".to_string());
    }
    let album_ref = pick(&found, |album| album.year, facts.year())?;
    let (confidence, reason) = match (album_ref.year, facts.year()) {
        (Some(wanted), Some(found)) if wanted == found => (98, "artist, album and year match"),
        (Some(wanted), Some(found)) if (wanted - found).abs() > 1 => (
            75,
            "artist and album match, but the year differs (a reissue?)",
        ),
        _ => (90, "artist and album match"),
    };
    let mut item = blank(
        MatchKind::Album,
        &album_ref.library_id,
        format!("{} - {}", album_ref.artist, album_ref.name),
        confidence,
        reason.to_string(),
    );
    item.album_id = Some(album_ref.id.clone());
    Ok(item)
}

fn match_book(facts: &ItemFacts, index: &TitleIndex) -> Result<ItemMatch, String> {
    let title = facts
        .music
        .album
        .as_deref()
        .ok_or_else(|| "the title does not give a book title".to_string())?;
    let mut best: Option<(&BookRef, f64)> = None;
    for book in &index.books {
        let book_facts = release_match::BookFacts {
            title: book.title.clone(),
            author: book.author.clone(),
            ..Default::default()
        };
        let (score, other_number) = release_match::release_relevance(title, &book_facts, None);
        let author_ok = match (facts.music.artist.as_deref(), book.author.as_deref()) {
            (Some(named), Some(wanted)) => identify_rules::same_person(named, wanted),
            _ => true,
        };
        if other_number.is_some() || !author_ok || score < 0.85 {
            continue;
        }
        if best.is_none_or(|(_, top)| score > top) {
            best = Some((book, score));
        }
    }
    let Some((book, score)) = best else {
        return Err("no audiobook in the library has this title and author".to_string());
    };
    if release_match::book_pack_hint(title).is_some() {
        return Err("a pack of several books cannot be tied to one".to_string());
    }
    let mut item = blank(
        MatchKind::Audiobook,
        &book.library_id,
        match &book.author {
            Some(author) => format!("{author} - {}", book.title),
            None => book.title.clone(),
        },
        (score * 100.0).round() as i32,
        "title and author match".to_string(),
    );
    item.audiobook_id = Some(book.id.clone());
    Ok(item)
}

/// What a feed item is in the library. `Err` is why nothing matched, for the Feed view.
pub fn match_item(facts: &ItemFacts, index: &TitleIndex) -> Result<ItemMatch, String> {
    match facts.kind {
        ItemKind::Tv => match_tv(facts, index),
        ItemKind::Movie => match_movie(facts, index),
        ItemKind::Music => match_album(facts, index),
        ItemKind::Audiobook => match_book(facts, index),
        ItemKind::Other => Err("not a kind of release the library holds".to_string()),
        ItemKind::Unknown => match_tv(facts, index).or_else(|tv| {
            if facts.parsed.season.is_some() {
                Err(tv)
            } else {
                match_movie(facts, index)
            }
        }),
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> TitleIndex {
        TitleIndex::new(
            vec![
                ShowRef {
                    id: "show-office".into(),
                    library_id: "tv".into(),
                    name: "The Office".into(),
                    year: Some(2005),
                },
                ShowRef {
                    id: "show-fallout".into(),
                    library_id: "tv".into(),
                    name: "Fallout".into(),
                    year: Some(2024),
                },
                ShowRef {
                    id: "show-office-us".into(),
                    library_id: "tv".into(),
                    name: "The Office (US)".into(),
                    year: Some(2005),
                },
            ],
            vec![
                MovieRef {
                    id: "m-red".into(),
                    library_id: "movies".into(),
                    title: "The Hunt for Red October".into(),
                    year: Some(1990),
                },
                MovieRef {
                    id: "m-heat".into(),
                    library_id: "movies".into(),
                    title: "Heat".into(),
                    year: Some(1995),
                },
            ],
            vec![AlbumRef {
                id: "alb".into(),
                library_id: "music".into(),
                artist: "Guns N' Roses".into(),
                name: "Appetite for Destruction".into(),
                year: Some(1987),
            }],
            vec![BookRef {
                id: "book".into(),
                library_id: "books".into(),
                title: "Old Man's War".into(),
                author: Some("John Scalzi".into()),
            }],
        )
    }

    fn read(title: &str, description: Option<&str>) -> ItemFacts {
        read_item(title, description, &[], None)
    }

    #[test]
    fn the_fixture_description_gives_size_and_category() {
        let facts = read(
            "The Hunt for Red October 1990 1080p BluRay x264-GROUP",
            Some("12.1 GB; Movie/HD/Bluray"),
        );
        assert_eq!(facts.kind, ItemKind::Movie);
        assert_eq!(facts.scope, Scope::Movie);
        assert_eq!(facts.category.as_deref(), Some("Movie/HD/Bluray"));
        let size = facts.size.unwrap();
        assert!((size - 12_992_276_070).abs() < 1_000_000, "{size}");
        assert_eq!(facts.year(), Some(1990));
        assert_eq!(facts.parsed.resolution.as_deref(), Some("1080p"));
        assert_eq!(facts.parsed.release_group.as_deref(), Some("GROUP"));
    }

    #[test]
    fn a_movie_is_matched_by_exact_name_and_year() {
        let facts = read(
            "The Hunt for Red October 1990 1080p BluRay x264-GROUP",
            Some("12.1 GB; Movie/HD/Bluray"),
        );
        let hit = match_item(&facts, &index()).unwrap();
        assert_eq!(hit.kind, MatchKind::Movie);
        assert_eq!(hit.movie_id.as_deref(), Some("m-red"));
        assert_eq!(hit.confidence, 100);
    }

    #[test]
    fn a_remake_from_another_year_is_not_the_movie_in_the_library() {
        let facts = read("Heat 2023 1080p WEB-DL x264-GRP", Some("3 GB; Movie/HD"));
        let why = match_item(&facts, &index()).unwrap_err();
        assert!(why.contains("1995"), "{why}");
    }

    #[test]
    fn a_movie_that_does_not_say_its_year_matches_with_low_confidence() {
        let facts = read("Heat 1080p BluRay x264-GRP", Some("3 GB; Movie/HD"));
        let hit = match_item(&facts, &index()).unwrap();
        assert_eq!(hit.confidence, 60);
        assert!(hit.reason.contains("year"));
    }

    #[test]
    fn the_office_is_not_the_office_us_but_a_bare_name_fits_the_us_show_too() {
        let ambiguous = read(
            "The Office S02E05 720p HDTV x264-GRP",
            Some("500 MB; TV/x264"),
        );
        // Both "The Office" and "The Office (US)" accept the bare name.
        assert!(
            match_item(&ambiguous, &index())
                .unwrap_err()
                .contains("more than one")
        );
        // A release that says US names only the US show.
        let us = read(
            "The Office US S02E05 720p HDTV x264-GRP",
            Some("500 MB; TV/x264"),
        );
        let hit = match_item(&us, &index()).unwrap();
        assert_eq!(hit.show_id.as_deref(), Some("show-office-us"));
    }

    #[test]
    fn episodes_season_packs_and_series_packs_are_told_apart() {
        let episode = read("Fallout S01E04 1080p WEB h264-GRP", Some("2 GB; TV/Web-DL"));
        let hit = match_item(&episode, &index()).unwrap();
        assert_eq!(
            (hit.kind, hit.season, hit.episode),
            (MatchKind::Episode, Some(1), Some(4))
        );
        assert_eq!(hit.label, "Fallout S01E04");

        let season = read("Fallout S01 1080p WEB h264-GRP", Some("20 GB; TV/Packs"));
        assert_eq!(season.scope, Scope::SeasonPack);
        assert_eq!(
            match_item(&season, &index()).unwrap().kind,
            MatchKind::Season
        );

        let series = read(
            "Fallout Complete Series 1080p WEB h264-GRP",
            Some("90 GB; TV/Packs"),
        );
        assert_eq!(series.scope, Scope::SeriesPack);
        assert_eq!(match_item(&series, &index()).unwrap().kind, MatchKind::Show);

        let range = read(
            "Fallout S01-S02 1080p WEB h264-GRP",
            Some("90 GB; TV/Packs"),
        );
        assert_eq!(range.scope, Scope::SeriesPack);
    }

    #[test]
    fn a_show_title_that_names_no_season_or_episode_is_unmatched_with_a_reason() {
        let facts = read("Fallout 1080p WEB h264-GRP", Some("2 GB; TV/Web-DL"));
        assert!(
            match_item(&facts, &index())
                .unwrap_err()
                .contains("season or episode")
        );
    }

    #[test]
    fn a_title_nobody_has_stays_unmatched_but_keeps_what_it_says() {
        let facts = read(
            "Some Unknown Show S03E01 1080p WEB h264-GRP",
            Some("1 GB; TV/Web-DL"),
        );
        assert!(match_item(&facts, &index()).is_err());
        assert_eq!(facts.parsed.season, Some(3));
        assert_eq!(facts.name, "some unknown show");
    }

    #[test]
    fn music_titles_give_artist_album_format_and_bitrate() {
        let facts = read(
            "Guns N' Roses - Appetite for Destruction (1987) [FLAC 24bit]",
            Some("900 MB; Music/Flac"),
        );
        assert_eq!(facts.kind, ItemKind::Music);
        assert_eq!(facts.music.artist.as_deref(), Some("Guns N' Roses"));
        assert_eq!(
            facts.music.album.as_deref(),
            Some("Appetite for Destruction")
        );
        assert_eq!(facts.music.format.as_deref(), Some("FLAC"));
        assert_eq!(facts.music.bitrate.as_deref(), Some("24BIT"));
        let hit = match_item(&facts, &index()).unwrap();
        assert_eq!(hit.kind, MatchKind::Album);
        assert_eq!(hit.album_id.as_deref(), Some("alb"));
        assert_eq!(hit.confidence, 98);

        let scene = read_music_title("Guns_N_Roses-Appetite_For_Destruction-WEB-1987-GRP");
        assert_eq!(scene.album.as_deref(), Some("Appetite For Destruction"));

        let other = read(
            "Xzibit - Appetite for Destruction [MP3 320]",
            Some("100 MB; Music/Audio"),
        );
        assert!(match_item(&other, &index()).is_err());
    }

    #[test]
    fn an_audiobook_needs_the_right_title_and_author() {
        let facts = read(
            "John Scalzi - Old Man's War (Unabridged) [MP3 64kbps]",
            Some("400 MB; AudioBook"),
        );
        assert_eq!(facts.kind, ItemKind::Audiobook);
        let hit = match_item(&facts, &index()).unwrap();
        assert_eq!(hit.audiobook_id.as_deref(), Some("book"));

        let other_author = read(
            "Someone Else - Old Man's War [MP3]",
            Some("400 MB; AudioBook"),
        );
        assert!(match_item(&other_author, &index()).is_err());

        let pack = read(
            "John Scalzi - Old Man's War Series 1-6 [MP3]",
            Some("4 GB; AudioBook"),
        );
        assert!(match_item(&pack, &index()).is_err());
    }

    #[test]
    fn games_and_software_are_not_matched() {
        let facts = read("Some Game v1.2 PC-GRP", Some("20 GB; Games/PC-ISO"));
        assert_eq!(facts.kind, ItemKind::Other);
        assert!(match_item(&facts, &index()).is_err());
    }

    #[test]
    fn torznab_numbers_say_the_kind_when_there_is_no_label() {
        let facts = read_item("Fallout S01E04 1080p", None, &[5040], None);
        assert_eq!(facts.kind, ItemKind::Tv);
        let movie = read_item("Heat 1995 1080p", None, &[2040], Some(5));
        assert_eq!(movie.kind, ItemKind::Movie);
        assert_eq!(movie.size, Some(5));
    }

    #[test]
    fn sizes_parse_with_decimals_and_binary_units() {
        assert_eq!(parse_size_text("1.5 GB").unwrap().0, 1_610_612_736);
        assert_eq!(
            parse_size_text("500 MiB; Music/Flac").unwrap(),
            (524_288_000, "Music/Flac".to_string())
        );
        assert!(parse_size_text("no size here").is_none());
    }
}
