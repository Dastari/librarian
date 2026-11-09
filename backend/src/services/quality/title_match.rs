//! Does a release name the wanted title? Token for token, not "looks similar".
//!
//! The hunt used to drop a release only when the Jaro-Winkler similarity of the *whole* normalised
//! release title (quality tags and all) against the wanted title fell under 0.70. A shared prefix
//! carries Jaro-Winkler a long way: `Babylon.Berlin.S01.1080p.10bit.WEBRip.6CH.x265.HEVC-PSA` passed
//! for **Babylon 5**, was recommended as covering all 22 missing episodes, and would have been
//! grabbed by automatic downloading.
//!
//! This module replaces that gate with a check on the release's *title part*: the words before the
//! season/episode, year or quality markers. Every significant word of the wanted name must be
//! there, in order, and nothing else may be. What is tolerated:
//!
//! - `the`, `a`, `an`, `and` and `&`, punctuation, apostrophes, accents and case;
//! - a year or a country code (`US`, `UK`, ...) after the name, unless the wanted title names a
//!   different one (`The Office (US)` is not `The.Office.UK`);
//! - letters written apart (`S.H.I.E.L.D.` is `SHIELD`) and words run together (`SpiderMan`);
//! - a release group in brackets or a site name before the title (`[SubsPlease]`, `www.x.org -`);
//! - any alias the target carries (a movie's original title), and an `aka` alias in the release.
//!
//! Music and books are named differently (artist, album, author and narrator all appear in one
//! title, in no fixed order), so they have their own rules: every word of the artist and of the
//! album or track for music, and [`release_match::title_score`] with every title word present for
//! books.
//!
//! Pure code: no I/O. See `docs/design.md` Q98.

use std::sync::LazyLock;

use regex::Regex;
use strsim::jaro_winkler;

use crate::services::release_match::{self, BookFacts, normalize_words};

/// What a release has to be named after to be taken for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WantedTitle {
    /// A show or a movie: the release's title part must be one of these names (the first is the
    /// main one; the rest are aliases). A trailing `(2005)` or `(US)` on a name is read as the
    /// year or country it is.
    Work { names: Vec<String> },
    /// A track or an album: every word of the artist, and every word of the album or of the track.
    Music {
        artist: String,
        album: Option<String>,
        track: Option<String>,
    },
    /// An audiobook, by its short title (the author is ignored: a pack of the author's other books
    /// shares it).
    Book {
        title: String,
        author: Option<String>,
    },
}

impl WantedTitle {
    /// A show or movie known by one name.
    pub fn work(name: impl Into<String>) -> Self {
        Self::Work {
            names: vec![name.into()],
        }
    }

    /// A show or movie with aliases. Empty and repeated names are dropped.
    pub fn work_with_aliases(name: impl Into<String>, aliases: &[Option<String>]) -> Self {
        let mut names = vec![name.into()];
        for alias in aliases.iter().flatten() {
            let alias = alias.trim();
            if !alias.is_empty() && !names.iter().any(|name| name.eq_ignore_ascii_case(alias)) {
                names.push(alias.to_string());
            }
        }
        Self::Work { names }
    }

    /// What the wanted title reads as, for logs and for the similarity tiebreak.
    pub fn display(&self) -> String {
        match self {
            Self::Work { names } => names.first().cloned().unwrap_or_default(),
            Self::Music {
                artist,
                album,
                track,
            } => [Some(artist.as_str()), album.as_deref(), track.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" "),
            Self::Book { title, author } => match author {
                Some(author) => format!("{author} {title}"),
                None => title.clone(),
            },
        }
    }

    /// Why `release_title` does not name this title, or `None` when it does.
    pub fn mismatch(&self, release_title: &str) -> Option<String> {
        match self {
            Self::Work { names } => work_mismatch(names, release_title),
            Self::Music {
                artist,
                album,
                track,
            } => music_mismatch(artist, album.as_deref(), track.as_deref(), release_title),
            Self::Book { title, author } => book_mismatch(title, author.as_deref(), release_title),
        }
    }

    pub fn matches(&self, release_title: &str) -> bool {
        self.mismatch(release_title).is_none()
    }

    /// Closeness in `[0, 1]`, used only to order releases that all passed [`Self::mismatch`]: the
    /// title part against the main name for shows and movies, the whole title otherwise.
    pub fn similarity(&self, release_title: &str) -> f64 {
        match self {
            Self::Work { names } => {
                let part = title_part(release_title).join(" ");
                names
                    .iter()
                    .map(|name| jaro_winkler(&tokens(name).join(" "), &part))
                    .fold(0.0, f64::max)
            }
            _ => jaro_winkler(
                &tokens(&self.display()).join(" "),
                &tokens(release_title).join(" "),
            ),
        }
    }
}

// ===========================================================================
// Words
// ===========================================================================

/// Accented Latin letters as releases spell them (`Shōgun` is `Shogun`, `Amélie` is `Amelie`).
fn fold_accents(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        let folded = match ch {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' | 'Ă' | 'Ą' => "A",
            'æ' => "ae",
            'Æ' => "AE",
            'ç' | 'ć' | 'č' => "c",
            'Ç' | 'Ć' | 'Č' => "C",
            'ď' | 'đ' => "d",
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => "e",
            'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ė' | 'Ę' | 'Ě' => "E",
            'ğ' => "g",
            'ì' | 'í' | 'î' | 'ï' | 'ī' | 'į' | 'ı' => "i",
            'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' | 'Į' | 'İ' => "I",
            'ł' | 'ľ' => "l",
            'Ł' | 'Ľ' => "L",
            'ñ' | 'ń' | 'ň' => "n",
            'Ñ' | 'Ń' | 'Ň' => "N",
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => "o",
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'Ō' | 'Ő' => "O",
            'œ' => "oe",
            'Œ' => "OE",
            'ř' => "r",
            'ś' | 'š' | 'ş' => "s",
            'Ś' | 'Š' | 'Ş' => "S",
            'ß' => "ss",
            'ť' | 'ţ' => "t",
            'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => "u",
            'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' | 'Ů' | 'Ű' => "U",
            'ý' | 'ÿ' => "y",
            'Ý' | 'Ÿ' => "Y",
            'ž' | 'ź' | 'ż' => "z",
            'Ž' | 'Ź' | 'Ż' => "Z",
            _ => {
                out.push(ch);
                continue;
            }
        };
        out.push_str(folded);
    }
    out
}

static LEADING_GROUP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:[\[【][^\]】]*[\]】]\s*)+").expect("leading group pattern")
});
static LEADING_SITE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(?:www\.)?[a-z0-9-]+\.(?:org|com|net|to|me|cc|io|info|xyz|club)\s*-+\s*")
        .expect("leading site pattern")
});

/// The words of a title: a leading `[Group]` or `www.site.org -` dropped, accents folded, `&` read
/// as `and`, apostrophes closed up, punctuation gone, and letters written apart joined
/// (`S.H.I.E.L.D` is `shield`).
pub fn tokens(title: &str) -> Vec<String> {
    let without_group = LEADING_GROUP.replace(title, "");
    let without_site = LEADING_SITE.replace(&without_group, "");
    let text = fold_accents(&without_site).replace('&', " and ");
    let raw: Vec<String> = normalize_words(&text)
        .split_whitespace()
        .map(str::to_string)
        .collect();
    let mut out: Vec<String> = Vec::with_capacity(raw.len());
    let mut index = 0;
    while index < raw.len() {
        let single = |word: &String| word.len() == 1 && word.chars().all(|c| c.is_alphabetic());
        if single(&raw[index]) {
            let start = index;
            while index < raw.len() && single(&raw[index]) {
                index += 1;
            }
            if index - start >= 2 {
                out.push(raw[start..index].concat());
            } else {
                out.push(raw[start].clone());
            }
            continue;
        }
        out.push(raw[index].clone());
        index += 1;
    }
    out
}

/// Words that never tell two titles apart.
fn is_filler(word: &str) -> bool {
    matches!(word, "the" | "a" | "an" | "and")
}

fn is_year(word: &str) -> bool {
    word.len() == 4
        && word.chars().all(|c| c.is_ascii_digit())
        && (word.starts_with("19") || word.starts_with("20"))
}

/// Country codes releases add to tell same-named shows apart.
fn is_country(word: &str) -> bool {
    matches!(word, "us" | "uk" | "gb" | "au" | "ca" | "nz")
}

/// Same country, written two ways.
fn same_country(a: &str, b: &str) -> bool {
    let canonical = |code: &str| if code == "gb" { "uk" } else { code }.to_string();
    canonical(a) == canonical(b)
}

static NUMBERING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:s\d{1,3}(?:e\d{1,4})*.*|e\d{1,4}|ep\d{1,4}|\d{1,2}x\d{1,3}|\d{3,4}[pi]|\d{2,})$",
    )
    .expect("numbering pattern")
});

/// Tags that start the part of a release title after the name.
const TAGS: &[&str] = &[
    // Numbering and scope.
    "season",
    "seasons",
    "series",
    "complete",
    "collection",
    "episode",
    "episodes",
    "vol",
    "volume",
    "special",
    "specials",
    "ova",
    "pack",
    "batch",
    "boxset",
    "aka",
    // Resolution.
    "4k",
    "uhd",
    "8k",
    "hd",
    "sd",
    "fhd",
    // Source.
    "web",
    "webrip",
    "webdl",
    "hdtv",
    "pdtv",
    "sdtv",
    "dsr",
    "bluray",
    "bdrip",
    "brrip",
    "bdremux",
    "remux",
    "dvd",
    "dvdrip",
    "dvd5",
    "dvd9",
    "dvdscr",
    "hdrip",
    "hdcam",
    "telesync",
    "telecine",
    "vhsrip",
    "laserdisc",
    "amzn",
    "nf",
    "dsnp",
    "hmax",
    "atvp",
    "hulu",
    "pcok",
    "itunes",
    // Codec and colour.
    "x264",
    "x265",
    "h264",
    "h265",
    "hevc",
    "avc",
    "xvid",
    "divx",
    "av1",
    "vp9",
    "10bit",
    "8bit",
    "12bit",
    "hdr",
    "hdr10",
    "dovi",
    "sdr",
    // Audio.
    "aac",
    "ac3",
    "eac3",
    "dts",
    "ddp",
    "ddp5",
    "dd5",
    "atmos",
    "truehd",
    "flac",
    "opus",
    "6ch",
    "2ch",
    "5ch",
    "8ch",
    // Edition.
    "proper",
    "repack",
    "rerip",
    "internal",
    "limited",
    "extended",
    "unrated",
    "uncut",
    "directors",
    "remastered",
    "imax",
    "theatrical",
    "multi",
    "dual",
    "subbed",
    "dubbed",
    "multisub",
    "vostfr",
];

/// A word that ends the title part: numbering, a year, a resolution, or a known tag.
fn is_marker(word: &str) -> bool {
    is_year(word) || NUMBERING.is_match(word) || TAGS.contains(&word)
}

/// The words of a release title before the first marker. Used for the similarity tiebreak and in
/// messages; [`work_mismatch`] walks the words itself so a year that is part of a name
/// (`Blade Runner 2049`) is not taken for a marker.
pub fn title_part(release_title: &str) -> Vec<String> {
    tokens(release_title)
        .into_iter()
        .take_while(|word| !is_marker(word))
        .collect()
}

// ===========================================================================
// Shows and movies
// ===========================================================================

static TRAILING_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*\(([^)]*)\)\s*$").expect("trailing tag pattern"));

/// A wanted name split into its words and what a trailing `(…)` said.
#[derive(Debug, Clone, Default)]
struct NameSpec {
    words: Vec<String>,
    year: Option<String>,
    country: Option<String>,
    /// Words of a trailing tag that is neither (`(Miniseries)`): allowed, not required.
    optional: Vec<String>,
}

fn name_spec(name: &str) -> NameSpec {
    let mut spec = NameSpec::default();
    let mut base = name.to_string();
    if let Some(found) = TRAILING_TAG.captures(name)
        && let (Some(whole), Some(inner)) = (found.get(0), found.get(1))
    {
        let inner_words = tokens(inner.as_str());
        base = name[..whole.start()].to_string();
        match inner_words.as_slice() {
            [word] if is_year(word) => spec.year = Some(word.clone()),
            [word] if is_country(word) => spec.country = Some(word.clone()),
            _ => spec.optional = inner_words,
        }
    }
    let mut words: Vec<String> = tokens(&base)
        .into_iter()
        .filter(|word| !is_filler(word))
        .collect();
    // `The Office US` as a name: the tail is the country. (A year at the end of a name without
    // brackets stays a word of it: `Blade Runner 2049`.)
    if words.len() > 1
        && spec.country.is_none()
        && words.last().is_some_and(|last| is_country(last))
    {
        spec.country = words.pop();
    }
    spec.words = words;
    spec
}

/// The outcome of walking a release title against one wanted name.
fn walk(spec: &NameSpec, release: &[String]) -> Result<(), String> {
    if spec.words.is_empty() {
        return Err("the wanted title has no words to look for".to_string());
    }
    let wanted = &spec.words;
    let mut i = 0usize;
    let mut j = 0usize;
    while j < wanted.len() {
        let Some(word) = release.get(i) else {
            return Err(format!("is missing '{}'", wanted[j..].join(" ")));
        };
        if *word == wanted[j] {
            i += 1;
            j += 1;
            continue;
        }
        // Words run together or written apart: `SpiderMan` / `Spider Man`.
        if j + 1 < wanted.len() && *word == format!("{}{}", wanted[j], wanted[j + 1]) {
            i += 1;
            j += 2;
            continue;
        }
        if let Some(next) = release.get(i + 1)
            && format!("{word}{next}") == wanted[j]
        {
            i += 2;
            j += 1;
            continue;
        }
        if is_filler(word) {
            i += 1;
            continue;
        }
        if spec.optional.contains(word) {
            i += 1;
            continue;
        }
        if is_marker(word) || is_country(word) {
            return Err(format!("is missing '{}'", wanted[j..].join(" ")));
        }
        return Err(format!("names '{word}' where '{}' was wanted", wanted[j]));
    }
    // The name is complete. What follows must be a marker, or something a name may carry.
    while let Some(word) = release.get(i) {
        if is_filler(word) || spec.optional.contains(word) {
            i += 1;
            continue;
        }
        if is_country(word) {
            if let Some(country) = &spec.country
                && !same_country(country, word)
            {
                return Err(format!(
                    "is the {} title, wanted the {} one",
                    word.to_ascii_uppercase(),
                    country.to_ascii_uppercase()
                ));
            }
            i += 1;
            continue;
        }
        if is_year(word) {
            // Two years in a row: the first is part of a longer name (`Blade.Runner.2049.2017`).
            if release.get(i + 1).is_some_and(|next| is_year(next)) {
                return Err(format!("has '{word}' after the name"));
            }
            if let Some(year) = &spec.year
                && let (Ok(found), Ok(wanted)) = (word.parse::<i32>(), year.parse::<i32>())
                && (found - wanted).abs() > 1
            {
                return Err(format!("is from {found}, wanted the {wanted} title"));
            }
            return Ok(());
        }
        if is_marker(word) {
            return Ok(());
        }
        return Err(format!("has '{word}' after the name"));
    }
    Ok(())
}

/// The words a release title is named with, each `aka` alias as its own start.
fn name_starts(release: &[String]) -> Vec<&[String]> {
    let mut starts = vec![release];
    for (index, word) in release.iter().enumerate() {
        if word == "aka" && index + 1 < release.len() {
            starts.push(&release[index + 1..]);
        }
    }
    starts
}

fn work_mismatch(names: &[String], release_title: &str) -> Option<String> {
    let release = tokens(release_title);
    if release.is_empty() {
        return Some("has no title".to_string());
    }
    let mut first_reason = None;
    for name in names {
        let spec = name_spec(name);
        for start in name_starts(&release) {
            match walk(&spec, start) {
                Ok(()) => return None,
                Err(reason) => {
                    first_reason.get_or_insert(reason);
                }
            }
        }
    }
    let part = title_part(release_title).join(" ");
    let named = if part.is_empty() {
        String::new()
    } else {
        format!("names '{part}', ")
    };
    Some(format!(
        "{named}not '{}' ({})",
        names.first().map(String::as_str).unwrap_or_default(),
        first_reason.unwrap_or_default()
    ))
}

// ===========================================================================
// Music and books
// ===========================================================================

fn significant(text: &str) -> Vec<String> {
    tokens(text)
        .into_iter()
        .filter(|word| !is_filler(word))
        .collect()
}

fn all_present(wanted: &[String], found: &[String]) -> bool {
    !wanted.is_empty() && wanted.iter().all(|word| found.contains(word))
}

fn music_mismatch(
    artist: &str,
    album: Option<&str>,
    track: Option<&str>,
    release_title: &str,
) -> Option<String> {
    let found = tokens(release_title);
    let artist_words = significant(artist);
    // A self-titled or untagged artist ("Various Artists") is not checked on its own.
    if !artist_words.is_empty()
        && !artist_words
            .iter()
            .all(|word| word == "various" || word == "artists")
        && !all_present(&artist_words, &found)
    {
        return Some(format!("does not name the artist '{artist}'"));
    }
    let album_words = album.map(significant).unwrap_or_default();
    let track_words = track.map(significant).unwrap_or_default();
    if album_words.is_empty() && track_words.is_empty() {
        return None;
    }
    if all_present(&album_words, &found) || all_present(&track_words, &found) {
        return None;
    }
    Some(match (album, track) {
        (Some(album), Some(track)) => {
            format!("names neither the album '{album}' nor the track '{track}'")
        }
        (Some(album), None) => format!("does not name the album '{album}'"),
        (None, Some(track)) => format!("does not name the track '{track}'"),
        (None, None) => unreachable!("checked above"),
    })
}

/// Below this a name does not look like the book (`release_match`'s own threshold for names).
const BOOK_TITLE_SCORE: f64 = 0.75;

fn book_mismatch(title: &str, author: Option<&str>, release_title: &str) -> Option<String> {
    let book = BookFacts {
        title: title.to_string(),
        author: author.map(str::to_string),
        ..Default::default()
    };
    let score = release_match::title_score(release_title, &book);
    let found = tokens(release_title);
    let wanted: Vec<String> = significant(&release_match::short_title(title));
    let missing: Vec<&String> = wanted
        .iter()
        .filter(|word| {
            !found
                .iter()
                .any(|other| other == *word || jaro_winkler(word, other) >= 0.92)
        })
        .collect();
    if !missing.is_empty() {
        return Some(format!(
            "does not name '{title}' (no '{}')",
            missing
                .iter()
                .map(|word| word.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    (score < BOOK_TITLE_SCORE).then(|| format!("does not look like '{title}'"))
}

#[cfg(test)]
#[path = "title_match_tests.rs"]
mod tests;
