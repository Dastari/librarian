//! Working out what a release actually is before anything moves.
//!
//! A provider's chapter list describes the provider's edition. A release can be cut up differently,
//! hold several books, or not be audio at all. So this module never fills chapters by position: it
//! finds which files belong to the book from folders, tags and names, compares their length with the
//! book, and only then decides where the chapters come from.
//!
//! Everything here is pure, so previews and imports share one answer and it can be tested on data
//! from real releases. The caller supplies facts, from a torrent's file list or from probing files.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use strsim::jaro_winkler;

/// How far a release's length may stray from the book and still be the same recording.
const SAME_RECORDING_TOLERANCE: f64 = 0.03;
/// Beyond this the files are not the whole book, whatever their names say.
const SAME_BOOK_TOLERANCE: f64 = 0.15;
/// A file boundary this close to a chapter boundary is the same boundary.
const BOUNDARY_TOLERANCE_SECS: f64 = 15.0;
/// Share of file boundaries that must land on chapter boundaries to call it one file per chapter.
const ALIGNED_SHARE: f64 = 0.9;
/// How well a group's name must match the book's title to be that book.
const TITLE_MATCH_SCORE: f64 = 0.75;
/// How well a file's own title must match a chapter's.
const CHAPTER_TITLE_MATCH: f64 = 0.88;
/// Share of files that must carry a real name before those names are trusted as chapters.
const MEANINGFUL_SHARE: f64 = 0.8;
/// An audiobook below this rate can't be speech, so a release this small is something else.
const MIN_AUDIOBOOK_BYTES_PER_SEC: f64 = 1_000.0;
/// With no known book length, a release below this can't be an audiobook of any real length.
const MIN_AUDIOBOOK_BYTES: i64 = 40 * 1024 * 1024;

pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "m4a", "m4b", "aac", "ogg", "oga", "opus", "flac", "wav", "wma", "aiff", "alac", "ape",
    "dsf", "dff", "mka", "wv",
];
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "m4v", "avi", "mov", "ts", "webm", "wmv", "mpg", "mpeg", "m2ts", "flv",
];
pub const EBOOK_EXTENSIONS: &[&str] = &[
    "epub", "mobi", "azw", "azw3", "pdf", "kfx", "djvu", "cbz", "cbr", "lit", "fb2",
];
pub const ARCHIVE_EXTENSIONS: &[&str] = &["rar", "zip", "7z", "tar", "gz", "bz2", "xz"];

// ---------------------------------------------------------------------------
// Words
// ---------------------------------------------------------------------------

/// Lowercase words with punctuation gone and apostrophes closed up, so `Man's` and `Mans` are the
/// same word and `War:` is just `war`.
pub fn normalize_words(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for ch in text.chars().flat_map(char::to_lowercase) {
        // U+FFFD is what a mis-decoded apostrophe usually became ("Don\u{FFFD}t Cry").
        if matches!(ch, '\'' | '\u{2019}' | '\u{2018}' | '`' | '\u{FFFD}') {
            continue;
        }
        if ch.is_alphanumeric() {
            out.push(ch);
            space = false;
        } else if !space {
            out.push(' ');
            space = true;
        }
    }
    out.trim().to_string()
}

fn words(text: &str) -> Vec<String> {
    normalize_words(text)
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// The check sources apply to what comes back: every word of the search appears in the title.
pub fn title_has_all_words(term: &str, title: &str) -> bool {
    let title = normalize_words(title);
    normalize_words(term)
        .split_whitespace()
        .all(|word| title.contains(word))
}

/// A term safe to hand to a site's search: no punctuation a site could read as syntax.
pub fn clean_search_term(term: &str) -> String {
    let mut out = String::with_capacity(term.len());
    for ch in term.chars() {
        if ch.is_alphanumeric() || matches!(ch, '\'' | '\u{2019}' | '.' | '-') {
            out.push(if ch == '\u{2019}' { '\'' } else { ch });
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `Man's` as sites that dropped the apostrophe list it, if the term has one.
pub fn without_apostrophes(term: &str) -> Option<String> {
    term.contains('\'')
        .then(|| term.replace('\'', ""))
        .filter(|plain| !plain.trim().is_empty())
}

fn is_series_marker(text: &str) -> bool {
    let list = words(text);
    let Some(first) = list.first() else {
        return true;
    };
    if matches!(
        first.as_str(),
        "book" | "volume" | "vol" | "part" | "episode" | "unabridged" | "abridged"
    ) {
        return list.len() <= 3;
    }
    if text.trim_start().starts_with('#') {
        return true;
    }
    matches!(
        normalize_words(text).as_str(),
        "a novel" | "a memoir" | "a thriller" | "the novel"
    )
}

/// A title as releases name it: without `: Book 1`, `(Full-Cast Edition)` and the like.
pub fn short_title(title: &str) -> String {
    let mut current = title.trim().to_string();
    loop {
        let trimmed = current.trim_end();
        let stripped = trimmed
            .strip_suffix(')')
            .and_then(|rest| rest.rfind('(').map(|open| (rest, open)))
            .or_else(|| {
                trimmed
                    .strip_suffix(']')
                    .and_then(|rest| rest.rfind('[').map(|open| (rest, open)))
            });
        match stripped {
            Some((rest, open))
                if open > 0
                    && (is_series_marker(&rest[open + 1..])
                        || normalize_words(&rest[open + 1..]).contains("edition")) =>
            {
                current = rest[..open].trim_end().to_string();
            }
            _ => break,
        }
    }
    for separator in [':', ',', '-'] {
        if let Some(index) = current.rfind(separator)
            && index > 0
            && is_series_marker(&current[index + 1..])
        {
            current = current[..index].trim_end().to_string();
        }
    }
    current
}

/// What to search for, best first. Only the first that finds anything is used.
pub fn book_search_ladder(
    query: &str,
    author: Option<&str>,
    title: &str,
    series: Option<&str>,
) -> Vec<String> {
    let short = short_title(title);
    let main = short
        .split_once(':')
        .map_or(short.as_str(), |(head, _)| head)
        .trim()
        .to_string();
    let mut ladder = vec![clean_search_term(query)];
    for name in [&short, &main] {
        if let Some(author) = author.filter(|author| !author.trim().is_empty()) {
            ladder.push(clean_search_term(&format!("{author} {name}")));
        }
    }
    ladder.push(clean_search_term(&short));
    ladder.push(clean_search_term(&main));
    if let Some(series) = series.filter(|series| !series.trim().is_empty()) {
        ladder.push(clean_search_term(series));
    }
    let mut seen = Vec::new();
    ladder.retain(|term| {
        let key = normalize_words(term);
        if key.is_empty() || seen.contains(&key) {
            return false;
        }
        seen.push(key);
        true
    });
    ladder
}

// ---------------------------------------------------------------------------
// What a release looks like from its title, category and size
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReleaseFormat {
    Audio,
    Ebook,
    Unknown,
}

impl ReleaseFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Ebook => "ebook",
            Self::Unknown => "unknown",
        }
    }
}

/// Torznab categories for ebooks and audiobooks.
const CATEGORY_AUDIOBOOK: i32 = 3030;
const CATEGORY_BOOKS: i32 = 7000;
const CATEGORY_EBOOK: i32 = 7020;

/// What a release seems to contain, for someone looking for an audiobook.
pub fn classify_book_release(
    title: &str,
    categories: &[i32],
    size: Option<i64>,
    book_seconds: Option<f64>,
) -> (ReleaseFormat, Option<String>) {
    let list = words(title);
    let has = |options: &[&str]| list.iter().any(|word| options.contains(&word.as_str()));
    let audio_word = has(&[
        "audiobook",
        "audiobooks",
        "audio",
        "m4b",
        "mp3",
        "flac",
        "aac",
        "narrated",
        "unabridged",
        "abridged",
        "kbps",
        "audible",
    ]);
    let ebook_words: Vec<&str> = EBOOK_EXTENSIONS
        .iter()
        .copied()
        .filter(|extension| has(&[extension]))
        .collect();
    if !ebook_words.is_empty() && !audio_word {
        return (
            ReleaseFormat::Ebook,
            Some(format!("Named as {}", ebook_words.join(", "))),
        );
    }
    if audio_word || categories.contains(&CATEGORY_AUDIOBOOK) {
        return (ReleaseFormat::Audio, None);
    }
    if categories.contains(&CATEGORY_EBOOK) || categories.contains(&CATEGORY_BOOKS) {
        return (
            ReleaseFormat::Ebook,
            Some("Listed under books, not audiobooks".to_string()),
        );
    }
    if let Some(size) = size.filter(|size| *size > 0) {
        let too_small = match book_seconds.filter(|seconds| *seconds > 0.0) {
            Some(seconds) => (size as f64) < seconds * MIN_AUDIOBOOK_BYTES_PER_SEC,
            None => size < MIN_AUDIOBOOK_BYTES,
        };
        if too_small {
            return (
                ReleaseFormat::Ebook,
                Some("Too small to be this audiobook".to_string()),
            );
        }
    }
    (ReleaseFormat::Unknown, None)
}

/// How many books a release's name says it holds, for ranges like `(1-5)` or `01-07` and words like
/// `series` or `collection`. A single `Book 7` is not a pack.
pub fn book_pack_hint(title: &str) -> Option<String> {
    static RANGE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?:^|[\s\[\(#])(\d{1,2})\s*-\s*(\d{1,2})(?:[\s\]\)\.,]|$)")
            .expect("pack range")
    });
    for found in RANGE.captures_iter(title) {
        let first: u32 = found[1].parse().unwrap_or(0);
        let last: u32 = found[2].parse().unwrap_or(0);
        if first <= 2 && last > first && last <= 60 {
            return Some(format!("{} books", last - first + 1));
        }
    }
    let list = words(title);
    list.iter()
        .any(|word| {
            matches!(
                word.as_str(),
                "series" | "collection" | "omnibus" | "anthology" | "boxset" | "complete"
            )
        })
        .then(|| "Several books".to_string())
}

/// Which book of a series a name says it is: `Book 7`, `Vol. 2`, `#6`, or `[Old Man's War 06]`.
pub fn book_number(name: &str) -> Option<i32> {
    static NUMBERED: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)(?:\bbook|\bvol(?:ume)?\.?|#)\s*(\d{1,2})\b|\b(\d{2})\]")
            .expect("book number")
    });
    NUMBERED
        .captures(name)
        .and_then(|found| found.get(1).or_else(|| found.get(2)))
        .and_then(|number| number.as_str().parse().ok())
}

/// How well a release's name fits the book wanted, from 0 to 1, and the other book of the series it
/// names, if it does. Another book's number counts against it, however well the rest matches.
pub fn release_relevance(
    title: &str,
    book: &BookFacts,
    series_position: Option<&str>,
) -> (f64, Option<i32>) {
    let wanted = series_position
        .and_then(|position| position.trim().parse::<f64>().ok())
        .map(|position| position as i32)
        .or_else(|| book_number(&book.title));
    let other = book_number(title).filter(|found| {
        book_pack_hint(title).is_none() && wanted.is_some_and(|wanted| wanted != *found)
    });
    let score = title_score(title, book);
    (if other.is_some() { score * 0.5 } else { score }, other)
}

/// What a download holds compared with what it was grabbed for, from its file list alone. `None`
/// when it could be right, which includes archives, since those are unpacked later.
pub fn payload_mismatch(wants_audio: bool, wants_video: bool, names: &[String]) -> Option<String> {
    if names.is_empty() || (!wants_audio && !wants_video) {
        return None;
    }
    let mut extensions: BTreeMap<String, usize> = BTreeMap::new();
    for name in names {
        let extension = extension_of(name);
        if ARCHIVE_EXTENSIONS.contains(&extension.as_str()) || is_split_archive(&extension) {
            return None;
        }
        if (wants_audio && AUDIO_EXTENSIONS.contains(&extension.as_str()))
            || (wants_video && VIDEO_EXTENSIONS.contains(&extension.as_str()))
        {
            return None;
        }
        if !extension.is_empty() {
            *extensions.entry(extension).or_default() += 1;
        }
    }
    let mut common: Vec<(String, usize)> = extensions.into_iter().collect();
    common.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let found = common
        .iter()
        .take(4)
        .map(|(extension, _)| format!(".{extension}"))
        .collect::<Vec<_>>()
        .join(", ");
    let wanted = if wants_audio { "audio" } else { "video" };
    Some(if found.is_empty() {
        format!("This release has no {wanted} files. Nothing was downloaded.")
    } else {
        format!("This release has no {wanted} files, only {found}. Nothing was downloaded.")
    })
}

fn is_split_archive(extension: &str) -> bool {
    let bytes = extension.as_bytes();
    (bytes.len() == 3 && bytes[0] == b'r' && bytes[1].is_ascii_digit())
        || (bytes.len() == 3 && bytes.iter().all(u8::is_ascii_digit))
}

fn extension_of(name: &str) -> String {
    name.rsplit(['/', '\\'])
        .next()
        .and_then(|file| file.rsplit_once('.'))
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

/// The tags that matter for matching, whichever format they came from. ffprobe reads ID3, MP4 atoms,
/// APE and Vorbis comments alike, but names them differently, and custom ones keep their own names.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct FileTags {
    pub title: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub composer: Option<String>,
    pub narrator: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub track: Option<i32>,
    pub track_total: Option<i32>,
    pub disc: Option<i32>,
    pub disc_total: Option<i32>,
    pub series: Option<String>,
    pub series_part: Option<String>,
    pub asin: Option<String>,
    pub isbn: Option<String>,
}

fn number_pair(value: &str) -> (Option<i32>, Option<i32>) {
    let mut parts = value.split(['/', '-', ' ']).filter(|part| !part.is_empty());
    let number = |part: Option<&str>| {
        part.and_then(|part| part.trim().parse::<i32>().ok())
            .filter(|number| *number > 0)
    };
    (number(parts.next()), number(parts.next()))
}

/// Read tags from any format's key/value pairs. Earlier pairs win, so pass the container's tags
/// before the audio stream's.
pub fn tags_from_pairs<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> FileTags {
    fn set(slot: &mut Option<String>, value: &str) {
        if slot.is_none() && !value.is_empty() {
            *slot = Some(value.to_string());
        }
    }
    fn set_number(slot: &mut Option<i32>, value: Option<i32>) {
        if slot.is_none() {
            *slot = value;
        }
    }
    let mut tags = FileTags::default();
    for (key, value) in pairs {
        let value = value.trim().trim_matches('\0');
        let key = key.trim().to_ascii_lowercase();
        // iTunes freeform atoms and ID3 user frames carry a prefix before the real name.
        let name = key.rsplit([':', '/']).next().unwrap_or(key.as_str());
        match name {
            "title" | "tit2" | "\u{a9}nam" => set(&mut tags.title, value),
            "album" | "talb" | "\u{a9}alb" => set(&mut tags.album, value),
            "artist" | "tpe1" | "\u{a9}art" | "author" => set(&mut tags.artist, value),
            "album_artist" | "albumartist" | "album artist" | "tpe2" | "aart" => {
                set(&mut tags.album_artist, value);
            }
            "composer" | "tcom" | "\u{a9}wrt" => set(&mut tags.composer, value),
            "narrator" | "narratedby" | "narrated_by" | "reader" | "performer" | "tnar"
            | "\u{a9}nrt" => set(&mut tags.narrator, value),
            "genre" | "tcon" | "\u{a9}gen" => set(&mut tags.genre, value),
            "date" | "year" | "tdrc" | "tyer" | "\u{a9}day" => set(&mut tags.date, value),
            "track" | "tracknumber" | "trck" | "trkn" => {
                let (number, total) = number_pair(value);
                set_number(&mut tags.track, number);
                set_number(&mut tags.track_total, total);
            }
            "tracktotal" | "totaltracks" | "track_total" => {
                set_number(&mut tags.track_total, number_pair(value).0);
            }
            "disc" | "disk" | "discnumber" | "disknumber" | "tpos" => {
                let (number, total) = number_pair(value);
                set_number(&mut tags.disc, number);
                set_number(&mut tags.disc_total, total);
            }
            "disctotal" | "totaldiscs" | "disc_total" => {
                set_number(&mut tags.disc_total, number_pair(value).0);
            }
            "series" | "series_name" | "seriesname" | "mvnm" | "movement_name" | "movementname"
            | "show" | "tvsh" | "grouping" | "tit1" | "\u{a9}grp" => set(&mut tags.series, value),
            "series_part" | "seriespart" | "series_position" | "mvin" | "movement"
            | "movementnumber" => set(&mut tags.series_part, value),
            "asin" | "audible_asin" | "audibleasin" | "cdek"
                if value.len() == 10 && value.chars().all(|ch| ch.is_ascii_alphanumeric()) =>
            {
                set(&mut tags.asin, &value.to_ascii_uppercase());
            }
            "isbn" | "isbn13" | "isbn10" => set(&mut tags.isbn, value),
            _ => {}
        }
    }
    tags
}

// ---------------------------------------------------------------------------
// Facts
// ---------------------------------------------------------------------------

/// A chapter marker inside a file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EmbeddedChapter {
    pub title: Option<String>,
    pub start: f64,
    pub end: f64,
}

/// One audio file of a release, with whatever is known so far. Before a file has downloaded there
/// is only its name and size.
#[derive(Debug, Clone, Default)]
pub struct ReleaseFile {
    /// What the caller knows this file by.
    pub key: String,
    /// Path inside the release, which is where its folders come from.
    pub path: String,
    pub size: i64,
    pub duration: Option<f64>,
    pub tags: FileTags,
    pub embedded: Vec<EmbeddedChapter>,
}

/// The book the release is being compared with.
#[derive(Debug, Clone, Default)]
pub struct BookFacts {
    pub title: String,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub asin: Option<String>,
    pub seconds: Option<f64>,
    pub series: Option<String>,
}

/// A chapter from the reference list, which is the provider's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceChapter {
    pub title: Option<String>,
    pub start: f64,
    pub seconds: Option<f64>,
}

// ---------------------------------------------------------------------------
// Which files are which book
// ---------------------------------------------------------------------------

/// Files that belong together, in play order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BookGroup {
    pub key: String,
    pub label: String,
    /// Indexes into the files given to `group_files`.
    pub files: Vec<usize>,
    /// What grouped them: `tags`, `folders` or `names`.
    pub grouped_by: &'static str,
}

/// A natural sort key: digit runs compare as numbers.
fn natural_key(text: &str) -> Vec<(u8, u64, String)> {
    let mut key = Vec::new();
    let mut digits = String::new();
    let mut other = String::new();
    for ch in text.to_lowercase().chars() {
        if ch.is_ascii_digit() {
            if !other.is_empty() {
                key.push((1, 0, std::mem::take(&mut other)));
            }
            digits.push(ch);
        } else {
            if !digits.is_empty() {
                key.push((0, digits.parse().unwrap_or(u64::MAX), String::new()));
                digits.clear();
            }
            other.push(ch);
        }
    }
    if !digits.is_empty() {
        key.push((0, digits.parse().unwrap_or(u64::MAX), String::new()));
    }
    if !other.is_empty() {
        key.push((1, 0, other));
    }
    key
}

fn file_stem(path: &str) -> &str {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

/// The folder a file sits in, skipping `CD 1` and `Disc 2`, which are part of the same book.
fn book_folder(path: &str) -> Option<String> {
    let mut folders: Vec<&str> = path.split(['/', '\\']).collect();
    folders.pop();
    while let Some(last) = folders.last() {
        let list = words(last);
        let is_disc = list.len() <= 3
            && list
                .first()
                .is_some_and(|word| matches!(word.as_str(), "cd" | "disc" | "disk" | "part"))
            || list
                .iter()
                .all(|word| word.chars().all(|ch| ch.is_ascii_digit()));
        if is_disc {
            folders.pop();
        } else {
            break;
        }
    }
    folders.last().map(|folder| (*folder).to_string())
}

/// A file name without its numbering, which is what parts of one book share.
fn name_pattern(path: &str) -> String {
    words(file_stem(path))
        .into_iter()
        .filter(|word| {
            !word.chars().all(|ch| ch.is_ascii_digit())
                && !matches!(
                    word.as_str(),
                    "of" | "part" | "pt" | "track" | "disc" | "cd" | "chapter" | "ch"
                )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `Old Man's War, Disc 2` and `Old Man's War` are one album.
fn album_key(album: &str) -> String {
    let list = words(album);
    let end = list
        .iter()
        .position(|word| matches!(word.as_str(), "disc" | "disk" | "cd"))
        .filter(|index| *index > 0)
        .unwrap_or(list.len());
    list[..end].join(" ")
}

fn group_by<F>(files: &[ReleaseFile], key: F) -> BTreeMap<String, (String, Vec<usize>)>
where
    F: Fn(&ReleaseFile) -> Option<(String, String)>,
{
    let mut groups: BTreeMap<String, (String, Vec<usize>)> = BTreeMap::new();
    for (index, file) in files.iter().enumerate() {
        if let Some((key, label)) = key(file) {
            groups
                .entry(key)
                .or_insert((label, Vec::new()))
                .1
                .push(index);
        }
    }
    groups
}

/// Split a release's audio files into books: by album tag when files have them, then by folder, then
/// by the pattern of their names. Names only divide a release if they fall into a few clear groups,
/// because files named after chapters all differ.
pub fn group_files(files: &[ReleaseFile]) -> Vec<BookGroup> {
    if files.is_empty() {
        return Vec::new();
    }
    let with_album = files
        .iter()
        .filter(|file| {
            file.tags
                .album
                .as_deref()
                .is_some_and(|a| !a.trim().is_empty())
        })
        .count();
    let (groups, grouped_by) = if with_album * 10 >= files.len() * 9 {
        (
            group_by(files, |file| {
                let album = file.tags.album.as_deref().unwrap_or("Untitled");
                Some((album_key(album), album.to_string()))
            }),
            "tags",
        )
    } else {
        let folders = group_by(files, |file| {
            book_folder(&file.path).map(|folder| (normalize_words(&folder), folder))
        });
        let placed: usize = folders.values().map(|(_, list)| list.len()).sum();
        if folders.len() > 1 && placed == files.len() {
            (folders, "folders")
        } else {
            let names = group_by(files, |file| {
                let pattern = name_pattern(&file.path);
                Some((pattern.clone(), pattern))
            });
            if names.len() > 1 && names.len() * 3 <= files.len() {
                (names, "names")
            } else {
                let label = folders
                    .values()
                    .next()
                    .map(|(label, _)| label.clone())
                    .unwrap_or_else(|| file_stem(&files[0].path).to_string());
                let mut one = BTreeMap::new();
                one.insert(normalize_words(&label), (label, (0..files.len()).collect()));
                (one, "folders")
            }
        }
    };
    groups
        .into_iter()
        .map(|(key, (label, mut list))| {
            list.sort_by_key(|index| {
                let file = &files[*index];
                (
                    file.tags.disc.unwrap_or(0),
                    file.tags.track.unwrap_or(0),
                    natural_key(&file.path),
                )
            });
            BookGroup {
                key,
                label,
                files: list,
                grouped_by,
            }
        })
        .collect()
}

/// Words in a release's name that say nothing about which book it is.
fn is_noise_word(word: &str) -> bool {
    matches!(
        word,
        "audiobook"
            | "audiobooks"
            | "audio"
            | "book"
            | "books"
            | "novel"
            | "novella"
            | "unabridged"
            | "abridged"
            | "mp3"
            | "m4b"
            | "m4a"
            | "aac"
            | "flac"
            | "kbps"
            | "retail"
            | "the"
            | "a"
            | "an"
            | "by"
            | "read"
            | "and"
            | "series"
    ) || word.chars().all(|ch| ch.is_ascii_digit())
}

fn word_found(word: &str, list: &[String]) -> bool {
    list.iter()
        .any(|other| other == word || jaro_winkler(word, other) >= 0.92)
}

/// How much a name looks like this book, from 0 to 1. The author's name is ignored, since a whole
/// pack shares it. Each part of the title counts more than each extra word in the name, because
/// release names add things.
pub fn title_score(name: &str, book: &BookFacts) -> f64 {
    let author = book.author.as_deref().map(words).unwrap_or_default();
    let keep = |list: Vec<String>| -> Vec<String> {
        list.into_iter()
            .filter(|word| !is_noise_word(word) && !author.contains(word))
            .collect()
    };
    let target = keep(words(&short_title(&book.title)));
    let found = keep(words(name));
    // A self-titled work is all author's words. Then they're the title.
    let (target, found) = if target.is_empty() || found.is_empty() {
        (words(&short_title(&book.title)), words(name))
    } else {
        (target, found)
    };
    if target.is_empty() || found.is_empty() {
        return 0.0;
    }
    let recall = target
        .iter()
        .filter(|word| word_found(word, &found))
        .count() as f64
        / target.len() as f64;
    let precision = found
        .iter()
        .filter(|word| word_found(word, &target))
        .count() as f64
        / found.len() as f64;
    recall * 0.7 + precision * 0.3
}

fn group_evidence(
    group: &BookGroup,
    files: &[ReleaseFile],
    book: &BookFacts,
) -> (f64, Vec<String>) {
    let mut evidence = Vec::new();
    if let Some(asin) = book.asin.as_deref()
        && group
            .files
            .iter()
            .any(|index| files[*index].tags.asin.as_deref() == Some(asin))
    {
        evidence.push(format!("ASIN tag {asin} matches"));
        return (1.0, evidence);
    }
    let mut score = title_score(&group.label, book);
    if score >= TITLE_MATCH_SCORE {
        evidence.push(match group.grouped_by {
            "tags" => format!(
                "Album tag \u{201c}{}\u{201d} matches the title",
                group.label
            ),
            "folders" => format!("Folder \u{201c}{}\u{201d} matches the title", group.label),
            _ => "File names match the title".to_string(),
        });
    }
    // Tags on the files can settle what a folder name doesn't.
    let tag_score = group
        .files
        .first()
        .and_then(|index| files[*index].tags.album.as_deref())
        .map(|album| title_score(album, book))
        .unwrap_or(0.0);
    if tag_score > score {
        score = tag_score;
        if score >= TITLE_MATCH_SCORE {
            evidence.push("Album tag matches the title".to_string());
        }
    }
    (score, evidence)
}

// ---------------------------------------------------------------------------
// How a book's files relate to its chapters
// ---------------------------------------------------------------------------

/// Where a book's chapters end up coming from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ChapterSource {
    /// Chapter markers inside the file.
    Embedded,
    /// One file per chapter, named after what's in it.
    Files,
    /// The provider's list, placed on the files by time.
    Provider,
    /// No real chapters and a length that doesn't match: the files, in order.
    Parts,
    /// The provider's list, at the times its chapters were heard being announced.
    Listened,
}

impl ChapterSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Embedded => "embedded",
            Self::Files => "files",
            Self::Provider => "provider",
            Self::Parts => "parts",
            Self::Listened => "listened",
        }
    }
}

/// How the files are cut, which decides what they are named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FileShape {
    WholeBook,
    PerChapter,
    Parts,
}

impl FileShape {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WholeBook => "whole_book",
            Self::PerChapter => "per_chapter",
            Self::Parts => "parts",
        }
    }
}

/// A chapter of the finished book and where it is.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlannedChapter {
    pub title: String,
    /// Position in the whole book.
    pub start: f64,
    pub end: Option<f64>,
    /// Index into the group's files. `None` leaves the chapter wanted.
    pub file: Option<usize>,
    /// Position in that file.
    pub offset: f64,
    /// The reference chapter this is, when it comes from the reference list.
    pub reference: Option<usize>,
}

/// The decision for one book in one release.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BookLayout {
    pub shape: FileShape,
    pub source: ChapterSource,
    pub chapters: Vec<PlannedChapter>,
    /// Release length over book length, when both are known.
    pub length_ratio: Option<f64>,
    /// True while durations are only estimated from file sizes.
    pub estimated: bool,
    /// Plain words for why, shown to people.
    pub evidence: Vec<String>,
}

/// Why a release needs a person. The book is still identified where possible.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReviewNeeded {
    pub reason: String,
    /// What would happen if a person accepts it anyway.
    pub fallback: Option<BookLayout>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum BookOutcome {
    Ready(BookLayout),
    Review(ReviewNeeded),
}

/// The whole answer for a release and the book it was grabbed for.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReleasePlan {
    pub groups: Vec<BookGroup>,
    /// Which group is the book, if any.
    pub chosen: Option<usize>,
    pub outcome: BookOutcome,
    /// Whether tags and durations were available, or only names and sizes.
    pub complete_facts: bool,
    pub evidence: Vec<String>,
}

fn clock(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as i64;
    if total >= 3600 {
        format!("{}h {:02}m", total / 3600, (total % 3600) / 60)
    } else {
        format!("{}m {:02}s", total / 60, total % 60)
    }
}

/// What a file is called apart from its numbering and the book's name. `None` if nothing is left,
/// which is the case for `01 - Old Man's War` or `Part 3 of 20`.
pub fn meaningful_name(file: &ReleaseFile, book: &BookFacts) -> Option<String> {
    let mut drop: Vec<String> = words(&short_title(&book.title));
    drop.extend(book.author.as_deref().map(words).unwrap_or_default());
    drop.extend(file.tags.album.as_deref().map(words).unwrap_or_default());
    drop.extend(file.tags.artist.as_deref().map(words).unwrap_or_default());
    let read = |text: &str| -> Option<String> {
        let list = words(text);
        let chapter_number = list
            .windows(2)
            .any(|pair| pair[0] == "chapter" && pair[1].chars().all(|ch| ch.is_ascii_digit()));
        // `CD05` and `disc2` count releases, like the plain numbers beside them.
        let numbered = |word: &str| {
            ["cd", "disc", "disk", "part", "pt", "track"]
                .iter()
                .any(|prefix| {
                    word.strip_prefix(prefix).is_some_and(|rest| {
                        !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit())
                    })
                })
        };
        let real = list.iter().any(|word| {
            !word.chars().all(|ch| ch.is_ascii_digit())
                && !numbered(word)
                && !drop.contains(word)
                && !matches!(
                    word.as_str(),
                    "of" | "part"
                        | "pt"
                        | "track"
                        | "disc"
                        | "disk"
                        | "cd"
                        | "chapter"
                        | "ch"
                        | "unabridged"
                        | "abridged"
                        | "audiobook"
                        | "mp3"
                        | "m4b"
                )
        });
        (real || chapter_number).then(|| tidy_chapter_name(text))
    };
    file.tags
        .title
        .as_deref()
        .and_then(read)
        .or_else(|| read(file_stem(&file.path)))
}

/// `07 - The Duel` reads as `The Duel`.
fn tidy_chapter_name(text: &str) -> String {
    let trimmed = text.trim();
    let without_number = trimmed
        .trim_start_matches(|ch: char| ch.is_ascii_digit())
        .trim_start_matches([' ', '-', '_', '.', ')']);
    if without_number.is_empty() {
        trimmed.to_string()
    } else {
        without_number.to_string()
    }
}

fn is_generic_marker(title: &str) -> bool {
    let list = words(title);
    list.is_empty()
        || (list.len() <= 2
            && list.iter().all(|word| {
                word.chars().all(|ch| ch.is_ascii_digit())
                    || matches!(word.as_str(), "chapter" | "ch" | "track" | "part")
            }))
}

/// Durations for a group: real ones if every file has one, otherwise sizes scaled to the book's
/// length, because a constant bitrate makes size stand in for time.
fn timeline(files: &[&ReleaseFile], book_seconds: Option<f64>) -> Option<(Vec<f64>, bool)> {
    if files.iter().all(|file| file.duration.is_some()) {
        return Some((
            files
                .iter()
                .map(|file| file.duration.unwrap_or(0.0))
                .collect(),
            false,
        ));
    }
    let seconds = book_seconds?;
    let total: f64 = files.iter().map(|file| file.size.max(0) as f64).sum();
    if total <= 0.0 {
        return None;
    }
    Some((
        files
            .iter()
            .map(|file| file.size.max(0) as f64 / total * seconds)
            .collect(),
        true,
    ))
}

/// Share of inner file boundaries that land on a chapter boundary.
fn aligned_share(durations: &[f64], reference: &[ReferenceChapter], scale: f64) -> f64 {
    if durations.len() < 2 {
        return 0.0;
    }
    let mut position = 0.0;
    let mut hits = 0usize;
    for duration in &durations[..durations.len() - 1] {
        position += duration;
        if reference
            .iter()
            .any(|chapter| (chapter.start * scale - position).abs() <= BOUNDARY_TOLERANCE_SECS)
        {
            hits += 1;
        }
    }
    hits as f64 / (durations.len() - 1) as f64
}

/// Put reference chapters on the files by time. The whole list is stretched to the files' length,
/// since one recording's chapter times drift slightly against a re-encode of it.
fn place_by_time(
    durations: &[f64],
    reference: &[ReferenceChapter],
    scale: f64,
) -> Vec<PlannedChapter> {
    let total: f64 = durations.iter().sum();
    let starts: Vec<f64> = durations
        .iter()
        .scan(0.0, |position, duration| {
            let start = *position;
            *position += duration;
            Some(start)
        })
        .collect();
    reference
        .iter()
        .enumerate()
        .map(|(index, chapter)| {
            let start = chapter.start * scale;
            // A chapter starting just before a file's start is that file's opening.
            let mut file = starts
                .iter()
                .rposition(|file_start| *file_start <= start + BOUNDARY_TOLERANCE_SECS)
                .unwrap_or(0);
            if file >= durations.len() {
                file = durations.len() - 1;
            }
            let raw_offset = start - starts[file];
            let offset = if raw_offset.abs() <= BOUNDARY_TOLERANCE_SECS {
                0.0
            } else {
                raw_offset.max(0.0).min(durations[file])
            };
            let end = reference
                .get(index + 1)
                .map(|next| next.start * scale)
                .unwrap_or(total);
            PlannedChapter {
                title: chapter
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("Chapter {}", index + 1)),
                start,
                end: Some(end),
                file: Some(file),
                offset,
                reference: Some(index),
            }
        })
        .collect()
}

/// Each file as a chapter, under the names given.
fn files_as_chapters(durations: &[f64], titles: Vec<String>) -> Vec<PlannedChapter> {
    let mut position = 0.0;
    durations
        .iter()
        .zip(titles)
        .enumerate()
        .map(|(index, (duration, title))| {
            let start = position;
            position += duration;
            PlannedChapter {
                title,
                start,
                end: Some(position),
                file: Some(index),
                offset: 0.0,
                reference: None,
            }
        })
        .collect()
}

fn parts_layout(
    durations: &[f64],
    book: &BookFacts,
    length_ratio: Option<f64>,
    estimated: bool,
    evidence: Vec<String>,
) -> BookLayout {
    let width = durations.len().to_string().len().max(2);
    let titles = if durations.len() == 1 {
        vec![short_title(&book.title)]
    } else {
        (1..=durations.len())
            .map(|number| format!("Part {number:0width$}"))
            .collect()
    };
    BookLayout {
        shape: if durations.len() == 1 {
            FileShape::WholeBook
        } else {
            FileShape::Parts
        },
        source: ChapterSource::Parts,
        chapters: files_as_chapters(durations, titles),
        length_ratio,
        estimated,
        evidence,
    }
}

/// Chapters from markers inside the files, on one timeline across them.
fn embedded_chapters(
    files: &[&ReleaseFile],
    durations: &[f64],
    reference: &[ReferenceChapter],
) -> Option<Vec<PlannedChapter>> {
    let total: usize = files.iter().map(|file| file.embedded.len()).sum();
    if total < 2 {
        return None;
    }
    let mut chapters = Vec::with_capacity(total);
    let mut position = 0.0;
    for (index, file) in files.iter().enumerate() {
        for marker in &file.embedded {
            chapters.push(PlannedChapter {
                title: marker
                    .title
                    .as_deref()
                    .map(|name| name.replace('\u{a0}', " ").trim().to_string())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_default(),
                start: position + marker.start,
                end: Some(position + marker.end),
                file: Some(index),
                offset: marker.start,
                reference: None,
            });
        }
        position += durations[index];
    }
    // Real times with plain names: the provider's names are better if the lists line up.
    if chapters.len() == reference.len()
        && chapters
            .iter()
            .all(|chapter| is_generic_marker(&chapter.title))
    {
        for (index, (chapter, known)) in chapters.iter_mut().zip(reference).enumerate() {
            if let Some(title) = known
                .title
                .as_ref()
                .filter(|title| !title.trim().is_empty())
            {
                chapter.title = title.clone();
                chapter.reference = Some(index);
            }
        }
    }
    for (index, chapter) in chapters.iter_mut().enumerate() {
        if chapter.title.is_empty() {
            chapter.title = format!("Chapter {}", index + 1);
        }
    }
    Some(chapters)
}

fn title_matches_chapters(
    titles: &[Option<String>],
    reference: &[ReferenceChapter],
) -> Option<Vec<Option<usize>>> {
    if reference.is_empty() {
        return None;
    }
    let known: Vec<String> = reference
        .iter()
        .map(|chapter| normalize_words(chapter.title.as_deref().unwrap_or_default()))
        .collect();
    let mut used = vec![false; known.len()];
    let mut found = 0usize;
    let mapping: Vec<Option<usize>> = titles
        .iter()
        .map(|title| {
            let title = normalize_words(title.as_deref()?);
            let (best, score) = known
                .iter()
                .enumerate()
                .filter(|(index, name)| !used[*index] && !name.is_empty())
                .map(|(index, name)| (index, jaro_winkler(&title, name)))
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))?;
            (score >= CHAPTER_TITLE_MATCH).then(|| {
                used[best] = true;
                found += 1;
                best
            })
        })
        .collect();
    (found as f64 >= titles.len() as f64 * MEANINGFUL_SHARE).then_some(mapping)
}

/// Decide how one group of files relates to one book. This is the rule list: chapter markers in the
/// file, then real names on the files, then the provider's list placed by time, then the files in
/// order, and a person if the length says this isn't the whole book.
pub fn layout_book(
    files: &[ReleaseFile],
    group: &BookGroup,
    book: &BookFacts,
    reference: &[ReferenceChapter],
) -> BookOutcome {
    let members: Vec<&ReleaseFile> = group.files.iter().map(|index| &files[*index]).collect();
    let listed: f64 = reference.iter().filter_map(|chapter| chapter.seconds).sum();
    let book_seconds = book
        .seconds
        .filter(|seconds| *seconds > 0.0)
        .or((listed > 0.0).then_some(listed));
    let Some((durations, estimated)) = timeline(&members, book_seconds) else {
        return BookOutcome::Review(ReviewNeeded {
            reason: "Can't tell how long these files are yet.".to_string(),
            fallback: None,
        });
    };
    let total: f64 = durations.iter().sum();
    let length_ratio = (!estimated)
        .then(|| book_seconds.map(|seconds| total / seconds))
        .flatten();
    let same_recording =
        length_ratio.is_none_or(|ratio| (ratio - 1.0).abs() <= SAME_RECORDING_TOLERANCE);
    let same_book = length_ratio.is_none_or(|ratio| (ratio - 1.0).abs() <= SAME_BOOK_TOLERANCE);
    let mut evidence = Vec::new();
    if let (Some(ratio), Some(seconds)) = (length_ratio, book_seconds) {
        evidence.push(format!(
            "{} of audio against a book of {} ({:.1}%)",
            clock(total),
            clock(seconds),
            ratio * 100.0
        ));
    } else if estimated {
        evidence
            .push("Lengths are estimated from file sizes until the download finishes".to_string());
    }
    let shape_for = |count: usize| {
        if count == 1 {
            FileShape::WholeBook
        } else {
            FileShape::Parts
        }
    };

    // A track tag saying 3 of 20 is the clearest sign of a partial release.
    let expected = members
        .iter()
        .filter_map(|file| file.tags.track_total)
        .max();
    let missing_parts = expected.is_some_and(|expected| (members.len() as i32) < expected);

    // 1. Chapter markers inside the file win: they describe exactly what will play.
    if !missing_parts
        && !estimated
        && let Some(chapters) = embedded_chapters(&members, &durations, reference)
    {
        if !same_book && book_seconds.is_some() {
            let reason = format!(
                "This release has its own chapters but is {:.0}% of the book's length, so it may be a different edition.",
                length_ratio.unwrap_or(1.0) * 100.0
            );
            return BookOutcome::Review(ReviewNeeded {
                reason,
                fallback: Some(BookLayout {
                    shape: shape_for(members.len()),
                    source: ChapterSource::Embedded,
                    chapters,
                    length_ratio,
                    estimated,
                    evidence,
                }),
            });
        }
        evidence.push(format!(
            "{} chapter markers inside the file",
            chapters.len()
        ));
        return BookOutcome::Ready(BookLayout {
            shape: shape_for(members.len()),
            source: ChapterSource::Embedded,
            chapters,
            length_ratio,
            estimated,
            evidence,
        });
    }

    let scale = book_seconds
        .filter(|seconds| same_recording && *seconds > 0.0)
        .map_or(1.0, |seconds| total / seconds);

    // One file for the whole book.
    if members.len() == 1 {
        if !reference.is_empty() && same_recording && !estimated {
            evidence.push(format!("One file holds all {} chapters", reference.len()));
            return BookOutcome::Ready(BookLayout {
                shape: FileShape::WholeBook,
                source: ChapterSource::Provider,
                chapters: place_by_time(&durations, reference, scale),
                length_ratio,
                estimated,
                evidence,
            });
        }
        if estimated {
            return BookOutcome::Ready(BookLayout {
                shape: FileShape::WholeBook,
                source: if reference.is_empty() {
                    ChapterSource::Parts
                } else {
                    ChapterSource::Provider
                },
                chapters: if reference.is_empty() {
                    files_as_chapters(&durations, vec![short_title(&book.title)])
                } else {
                    place_by_time(&durations, reference, scale)
                },
                length_ratio,
                estimated,
                evidence,
            });
        }
        let fallback = parts_layout(&durations, book, length_ratio, estimated, evidence.clone());
        if same_book || book_seconds.is_none() {
            evidence.push(
                "Its length doesn't match the book's chapters, so it plays as one".to_string(),
            );
            return BookOutcome::Ready(BookLayout {
                evidence,
                ..fallback
            });
        }
        return BookOutcome::Review(ReviewNeeded {
            reason: format!(
                "This file is {:.0}% of the book's length.",
                length_ratio.unwrap_or(1.0) * 100.0
            ),
            fallback: Some(fallback),
        });
    }

    let names: Vec<Option<String>> = members
        .iter()
        .map(|file| meaningful_name(file, book))
        .collect();
    let named = names.iter().filter(|name| name.is_some()).count();
    let meaningful = named as f64 >= members.len() as f64 * MEANINGFUL_SHARE;

    // 2. Files named after the provider's chapters are those chapters, even if some are missing.
    if meaningful && let Some(mapping) = title_matches_chapters(&names, reference) {
        let mut chapters: Vec<PlannedChapter> = reference
            .iter()
            .enumerate()
            .map(|(index, chapter)| PlannedChapter {
                title: chapter
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("Chapter {}", index + 1)),
                start: chapter.start,
                end: chapter.seconds.map(|seconds| chapter.start + seconds),
                file: None,
                offset: 0.0,
                reference: Some(index),
            })
            .collect();
        for (file, chapter) in mapping.iter().enumerate() {
            if let Some(chapter) = chapter {
                chapters[*chapter].file = Some(file);
            }
        }
        evidence.push("File names match the chapter titles".to_string());
        return BookOutcome::Ready(BookLayout {
            shape: FileShape::PerChapter,
            source: ChapterSource::Provider,
            chapters,
            length_ratio,
            estimated,
            evidence,
        });
    }

    if missing_parts {
        let reason = format!(
            "This release has {} of the {} parts its tags list.",
            members.len(),
            expected.unwrap_or_default()
        );
        return BookOutcome::Review(ReviewNeeded {
            reason,
            fallback: Some(parts_layout(
                &durations,
                book,
                length_ratio,
                estimated,
                evidence,
            )),
        });
    }

    // 3. The same recording: the provider's chapters are right, wherever the files happen to be cut.
    if !reference.is_empty() && same_recording {
        let aligned = aligned_share(&durations, reference, scale);
        let shape = if aligned >= ALIGNED_SHARE {
            FileShape::PerChapter
        } else {
            FileShape::Parts
        };
        // Real names that don't line up with the provider describe a different cut: rule 4.
        if shape == FileShape::PerChapter || !meaningful {
            evidence.push(if shape == FileShape::PerChapter {
                format!(
                    "{:.0}% of file breaks fall on chapter breaks",
                    aligned * 100.0
                )
            } else {
                format!(
                    "{} parts that don't follow the chapters, so chapters are placed by time",
                    members.len()
                )
            });
            return BookOutcome::Ready(BookLayout {
                shape,
                source: ChapterSource::Provider,
                chapters: place_by_time(&durations, reference, scale),
                length_ratio,
                estimated,
                evidence,
            });
        }
    }

    // 4. Real chapter names of its own: the release's chapters replace the provider's.
    if meaningful {
        let titles = names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                name.clone()
                    .unwrap_or_else(|| format!("Part {}", index + 1))
            })
            .collect();
        let layout = BookLayout {
            shape: FileShape::PerChapter,
            source: ChapterSource::Files,
            chapters: files_as_chapters(&durations, titles),
            length_ratio,
            estimated,
            evidence: evidence.clone(),
        };
        if same_book {
            evidence.push("Each file is named as its own chapter".to_string());
            return BookOutcome::Ready(BookLayout { evidence, ..layout });
        }
        return BookOutcome::Review(ReviewNeeded {
            reason: format!(
                "These files are {:.0}% of the book's length.",
                length_ratio.unwrap_or(1.0) * 100.0
            ),
            fallback: Some(layout),
        });
    }

    // 5. No real chapters, close in length: the files in order, honestly labelled.
    let layout = parts_layout(&durations, book, length_ratio, estimated, evidence.clone());
    if same_book {
        evidence.push(
            "No chapter information, and the length differs from the book's chapters".to_string(),
        );
        return BookOutcome::Ready(BookLayout { evidence, ..layout });
    }
    BookOutcome::Review(ReviewNeeded {
        reason: format!(
            "These files are {:.0}% of the book's length, so this isn't the whole book.",
            length_ratio.unwrap_or(1.0) * 100.0
        ),
        fallback: Some(layout),
    })
}

/// The provider's chapters at the given starts, as measured rather than scaled. Starts are exact, so a
/// chapter near a file's start keeps its offset instead of being snapped to the file's opening.
pub fn listened_layout(
    durations: &[f64],
    reference: &[ReferenceChapter],
    starts: &[f64],
) -> BookLayout {
    const SAME_INSTANT_SECS: f64 = 1.0;
    let moved: Vec<ReferenceChapter> = reference
        .iter()
        .zip(starts)
        .map(|(chapter, start)| ReferenceChapter {
            title: chapter.title.clone(),
            start: *start,
            seconds: chapter.seconds,
        })
        .collect();
    let mut position = 0.0;
    let file_starts: Vec<f64> = durations
        .iter()
        .map(|duration| {
            let start = position;
            position += duration;
            start
        })
        .collect();
    let mut chapters = place_by_time(durations, &moved, 1.0);
    for chapter in &mut chapters {
        let file = file_starts
            .iter()
            .rposition(|start| *start <= chapter.start + SAME_INSTANT_SECS)
            .unwrap_or(0);
        chapter.file = Some(file);
        let offset = chapter.start - file_starts[file];
        chapter.offset = if offset.abs() <= SAME_INSTANT_SECS {
            0.0
        } else {
            offset.max(0.0)
        };
    }
    BookLayout {
        shape: if durations.len() == 1 {
            FileShape::WholeBook
        } else if aligned_share(durations, &moved, 1.0) >= ALIGNED_SHARE {
            FileShape::PerChapter
        } else {
            FileShape::Parts
        },
        source: ChapterSource::Listened,
        chapters,
        length_ratio: None,
        estimated: false,
        evidence: Vec::new(),
    }
}

/// The provider's chapters on the given files by time, whatever their length, for when a person asks
/// for them back.
pub fn provider_layout(
    durations: &[f64],
    reference: &[ReferenceChapter],
    book_seconds: Option<f64>,
) -> BookLayout {
    let total: f64 = durations.iter().sum();
    let listed: f64 = reference.iter().filter_map(|chapter| chapter.seconds).sum();
    let book = book_seconds
        .filter(|seconds| *seconds > 0.0)
        .unwrap_or(listed);
    let scale = if book > 0.0 && total > 0.0 {
        total / book
    } else {
        1.0
    };
    if durations.is_empty() {
        return BookLayout {
            shape: FileShape::PerChapter,
            source: ChapterSource::Provider,
            chapters: reference
                .iter()
                .enumerate()
                .map(|(index, chapter)| PlannedChapter {
                    title: chapter
                        .title
                        .clone()
                        .unwrap_or_else(|| format!("Chapter {}", index + 1)),
                    start: chapter.start,
                    end: chapter.seconds.map(|seconds| chapter.start + seconds),
                    file: None,
                    offset: 0.0,
                    reference: Some(index),
                })
                .collect(),
            length_ratio: None,
            estimated: false,
            evidence: Vec::new(),
        };
    }
    BookLayout {
        shape: if durations.len() == 1 {
            FileShape::WholeBook
        } else if aligned_share(durations, reference, scale) >= ALIGNED_SHARE {
            FileShape::PerChapter
        } else {
            FileShape::Parts
        },
        source: ChapterSource::Provider,
        chapters: place_by_time(durations, reference, scale),
        length_ratio: (book > 0.0).then(|| total / book),
        estimated: false,
        evidence: Vec::new(),
    }
}

/// Pick the group of files that is this book. With a single group there is nothing to choose, so its
/// length decides later. With several, the name or tags must say so.
pub fn choose_group(
    groups: &[BookGroup],
    files: &[ReleaseFile],
    book: &BookFacts,
) -> (Option<usize>, Vec<String>) {
    let mut best: Option<(usize, f64, Vec<String>)> = None;
    for (index, group) in groups.iter().enumerate() {
        let (score, evidence) = group_evidence(group, files, book);
        if best.as_ref().is_none_or(|(_, current, _)| score > *current) {
            best = Some((index, score, evidence));
        }
    }
    match best {
        Some((index, score, evidence)) if score >= TITLE_MATCH_SCORE => (Some(index), evidence),
        Some((index, _, _)) if groups.len() == 1 => (Some(index), Vec::new()),
        _ => (None, Vec::new()),
    }
}

/// A lone group whose name doesn't say it's this book. A close length alone can't prove it, since
/// plenty of books run about ten hours. Tags naming something else rule it out. With no tags, only
/// a length matching the recording closely enough carries it, and a person can still accept it.
fn unconfirmed_book(
    outcome: BookOutcome,
    group: &BookGroup,
    files: &[ReleaseFile],
    book: &BookFacts,
) -> BookOutcome {
    let BookOutcome::Ready(layout) = outcome else {
        return outcome;
    };
    let tagged = group
        .files
        .first()
        .and_then(|index| files[*index].tags.album.as_deref());
    let reason = if let Some(album) = tagged {
        format!(
            "The tags on these files say \u{201c}{album}\u{201d}, not \u{201c}{}\u{201d}.",
            short_title(&book.title)
        )
    } else if layout.estimated {
        "The name doesn't match this book, so it needs checking once it has downloaded.".to_string()
    } else if layout
        .length_ratio
        .is_some_and(|ratio| (ratio - 1.0).abs() <= SAME_RECORDING_TOLERANCE)
    {
        return BookOutcome::Ready(BookLayout {
            evidence: {
                let mut evidence = layout.evidence.clone();
                evidence.push(
                    "The name doesn't say which book it is, but the length matches".to_string(),
                );
                evidence
            },
            ..layout
        });
    } else {
        format!(
            "The name \u{201c}{}\u{201d} doesn't match \u{201c}{}\u{201d}, and neither does the length.",
            group.label,
            short_title(&book.title)
        )
    };
    BookOutcome::Review(ReviewNeeded {
        reason,
        fallback: Some(layout),
    })
}

/// The full answer for a release and a book.
pub fn plan_release(
    files: &[ReleaseFile],
    book: &BookFacts,
    reference: &[ReferenceChapter],
) -> ReleasePlan {
    let complete_facts = !files.is_empty() && files.iter().all(|file| file.duration.is_some());
    let groups = group_files(files);
    let (chosen, mut evidence) = choose_group(&groups, files, book);
    let outcome = match chosen {
        Some(index) => {
            let outcome = layout_book(files, &groups[index], book, reference);
            if evidence.is_empty() {
                unconfirmed_book(outcome, &groups[index], files, book)
            } else {
                outcome
            }
        }
        None => BookOutcome::Review(ReviewNeeded {
            reason: if groups.is_empty() {
                "This release has no audio files.".to_string()
            } else {
                format!(
                    "None of the {} books in this release looks like \u{201c}{}\u{201d}.",
                    groups.len(),
                    short_title(&book.title)
                )
            },
            fallback: None,
        }),
    };
    if groups.len() > 1
        && let Some(index) = chosen
    {
        evidence.push(format!(
            "{} of {} files are this book",
            groups[index].files.len(),
            files.len()
        ));
    }
    ReleasePlan {
        groups,
        chosen,
        outcome,
        complete_facts,
        evidence,
    }
}

/// Which chapter each file begins in, for naming and for linking every file to the book. Returns
/// indexes into the planned chapters, one per file in the group.
pub fn chapter_of_each_file(layout: &BookLayout, file_count: usize) -> Vec<Option<usize>> {
    let mut found: Vec<Option<usize>> = vec![None; file_count];
    for (index, chapter) in layout.chapters.iter().enumerate() {
        if let Some(file) = chapter.file
            && file < file_count
            && found[file].is_none()
            && chapter.offset <= BOUNDARY_TOLERANCE_SECS
        {
            found[file] = Some(index);
        }
    }
    // A part that begins inside a long chapter belongs to the last chapter started before it.
    let mut last = None;
    for (file, holder) in found.iter_mut().enumerate() {
        if holder.is_none() {
            *holder = layout
                .chapters
                .iter()
                .enumerate()
                .rfind(|(_, chapter)| chapter.file.is_some_and(|held| held <= file))
                .map(|(index, _)| index)
                .or(last);
        }
        last = *holder;
    }
    found
}

/// Readable summary, for the torrent and for logs.
pub fn describe(plan: &ReleasePlan, book: &BookFacts) -> String {
    let title = short_title(&book.title);
    let total_files: usize = plan.groups.iter().map(|group| group.files.len()).sum();
    let others = plan.groups.len().saturating_sub(1);
    let extra = match plan.chosen {
        Some(index) if others > 0 => {
            let rest = total_files - plan.groups[index].files.len();
            format!(
                " {rest} other {} belong to {others} other {}.",
                if rest == 1 { "file" } else { "files" },
                if others == 1 { "book" } else { "books" }
            )
        }
        _ => String::new(),
    };
    match &plan.outcome {
        BookOutcome::Ready(layout) => {
            let what = match layout.shape {
                FileShape::WholeBook => "one file".to_string(),
                FileShape::PerChapter => format!(
                    "{} files, one per chapter",
                    layout
                        .chapters
                        .iter()
                        .filter_map(|chapter| chapter.file)
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                ),
                FileShape::Parts => {
                    let index = plan.chosen.unwrap_or(0);
                    format!(
                        "{} parts",
                        plan.groups.get(index).map_or(0, |group| group.files.len())
                    )
                }
            };
            let length = layout
                .length_ratio
                .map(|ratio| format!(", {:.1}% of the book's length", ratio * 100.0))
                .unwrap_or_default();
            format!("{title}: {what}{length}.{extra}")
        }
        BookOutcome::Review(review) => format!("{}{extra}", review.reason),
    }
}

/// Group a list of plain track facts against an album's tracks. See `match_tracks`.
#[derive(Debug, Clone, Default)]
pub struct TrackFacts {
    pub title: String,
    pub number: Option<i32>,
    pub disc: Option<i32>,
    pub seconds: Option<f64>,
}

/// Below this, a file's title and a track's are different songs.
const TITLES_DIFFER: f64 = 0.7;

/// Whether a normalized title names something, rather than only numbering it ("Track 03", "03").
fn names_a_song(title: &str) -> bool {
    title.split_whitespace().any(|word| {
        word.chars().any(char::is_alphabetic)
            && !matches!(
                word,
                "track" | "piste" | "titel" | "audiotrack" | "untitled" | "cd" | "disc" | "disk"
            )
    })
}

/// How well one file fits one album track, from 0 to 1, or `None` if something rules it out.
fn track_fit(file: &ReleaseFile, name: Option<&str>, track: &TrackFacts) -> Option<f64> {
    let mut score = 0.0;
    let mut weight = 0.0;
    if let (Some(seconds), Some(wanted)) = (file.duration, track.seconds) {
        let difference = (seconds - wanted).abs();
        // Beyond this a track is a different recording, or a different track.
        if difference > 12.0 && difference > wanted * 0.05 {
            return None;
        }
        score += (1.0 - (difference / 12.0).min(1.0)) * 2.0;
        weight += 2.0;
    }
    // A name that only numbers the file ("Track 03") says nothing about which song it is.
    if let Some(name) = name.map(normalize_words).filter(|name| names_a_song(name)) {
        let wanted = normalize_words(&track.title);
        let similar = if !wanted.is_empty() && (name.contains(&wanted) || wanted.contains(&name)) {
            1.0
        } else {
            jaro_winkler(&name, &wanted)
        };
        // Two real titles that disagree are two songs, whatever number and length they share.
        if similar < TITLES_DIFFER && names_a_song(&wanted) {
            return None;
        }
        score += similar * 3.0;
        weight += 3.0;
    }
    if let (Some(number), Some(wanted)) = (file.tags.track, track.number) {
        let same_disc = match (file.tags.disc, track.disc) {
            (Some(disc), Some(other)) => disc == other,
            _ => true,
        };
        score += if number == wanted && same_disc {
            2.0
        } else {
            0.0
        };
        weight += 2.0;
    }
    (weight > 0.0).then(|| score / weight)
}

/// One file's best track, using tags, names and lengths together. `chosen[file]` is an index into
/// `tracks`. Pairs are taken best first, so a strong match can't lose its track to a weak one.
pub fn match_tracks(
    files: &[ReleaseFile],
    names: &[Option<String>],
    tracks: &[TrackFacts],
) -> Vec<Option<(usize, f64)>> {
    const STRONG_ENOUGH: f64 = 0.72;
    let mut pairs: Vec<(usize, usize, f64)> = Vec::new();
    for (file_index, file) in files.iter().enumerate() {
        let name = file
            .tags
            .title
            .as_deref()
            .or_else(|| names.get(file_index).and_then(|name| name.as_deref()));
        for (track_index, track) in tracks.iter().enumerate() {
            if let Some(score) = track_fit(file, name, track)
                && score >= STRONG_ENOUGH
            {
                pairs.push((file_index, track_index, score));
            }
        }
    }
    pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let mut chosen = vec![None; files.len()];
    let mut used: HashMap<usize, ()> = HashMap::new();
    for (file, track, score) in pairs {
        if chosen[file].is_none() && !used.contains_key(&track) {
            chosen[file] = Some((track, score));
            used.insert(track, ());
        }
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbered_discs_in_a_name_are_not_a_chapter_name() {
        // Zoe's Tale in the real pack: its tag is only numbering, and so is the rest of its file name.
        let file = ReleaseFile {
            path: "Scalzi, John -- Zoe's Tale -- 28 of 68 -- CD05 -- 02.mp3".to_string(),
            tags: FileTags {
                title: Some("Zoe's Tale - 28-68".to_string()),
                album: Some("Zoe's Tale".to_string()),
                artist: Some("John Scalzi".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let book = BookFacts {
            title: "Zoe's Tale".to_string(),
            author: Some("John Scalzi".to_string()),
            ..Default::default()
        };
        assert_eq!(meaningful_name(&file, &book), None);
    }

    fn book() -> BookFacts {
        BookFacts {
            title: "Old Man's War: Book 1".to_string(),
            author: Some("John Scalzi".to_string()),
            asin: Some("B0DSQ3WM2D".to_string()),
            seconds: Some(35_748.0),
            ..Default::default()
        }
    }

    /// The chapter lengths Audible and Audnexus list for this book.
    fn chapters() -> Vec<ReferenceChapter> {
        let lengths = [
            11.0, 5.0, 1697.0, 1887.0, 2110.0, 2561.0, 2148.0, 2207.0, 3.0, 1583.0, 2568.0, 1694.0,
            2323.0, 1607.0, 2021.0, 3.0, 1976.0, 2232.0, 1925.0, 2136.0, 2460.0, 562.0, 31.0,
        ];
        let mut start = 0.0;
        lengths
            .iter()
            .enumerate()
            .map(|(index, seconds)| {
                let chapter = ReferenceChapter {
                    title: Some(format!("Title {}", index + 1)),
                    start,
                    seconds: Some(*seconds),
                };
                start += seconds;
                chapter
            })
            .collect()
    }

    /// Book 1 and Book 2 of the real pack that was imported wrongly, with their real tags.
    fn pack() -> Vec<ReleaseFile> {
        let mut files = Vec::new();
        for number in 1..=20 {
            let seconds = if number <= 10 { 1846.0 } else { 1722.0 };
            files.push(ReleaseFile {
                key: format!("omw-{number}"),
                path: format!(
                    "[Audiobook] Scalzi, John - Old Mans War Series (1-5)/John Scalzi - Old Man's War (Book 1)/Scalzi, John -- Old Mans War -- {number:02} of 20.mp3"
                ),
                size: 18_000_000,
                duration: Some(seconds),
                tags: FileTags {
                    title: Some(format!("{number:02} - Old Man's War")),
                    album: Some("Old Man's War".to_string()),
                    artist: Some("Scalzi, John".to_string()),
                    track: Some(number),
                    track_total: Some(20),
                    ..Default::default()
                },
                embedded: Vec::new(),
            });
        }
        for number in 1..=41 {
            files.push(ReleaseFile {
                key: format!("gb-{number}"),
                path: format!(
                    "[Audiobook] Scalzi, John - Old Mans War Series (1-5)/John Scalzi - The Ghost Brigades (Book 2)/Scalzi, John -- The Ghost Brigades -- {number:02}-41.mp3"
                ),
                size: 9_000_000,
                duration: Some(915.0),
                tags: FileTags {
                    title: Some(format!("{number:02} - The Ghost Brigades")),
                    album: Some("The Ghost Brigades".to_string()),
                    artist: Some("Scalzi, John".to_string()),
                    track: Some(number),
                    track_total: Some(41),
                    ..Default::default()
                },
                embedded: Vec::new(),
            });
        }
        files
    }

    fn ready(plan: &ReleasePlan) -> &BookLayout {
        match &plan.outcome {
            BookOutcome::Ready(layout) => layout,
            BookOutcome::Review(review) => panic!("expected ready, got review: {}", review.reason),
        }
    }

    #[test]
    fn punctuation_never_decides_whether_a_release_matches() {
        assert!(title_has_all_words(
            "John Scalzi Old Man's War: Book 1",
            "John Scalzi - Old Man's War (Book 1)"
        ));
        assert!(title_has_all_words(
            "Old Man's War",
            "Scalzi, John -- Old Mans War"
        ));
        assert!(title_has_all_words(
            "Old Mans war",
            "John Scalzi - Old Man\u{2019}s War"
        ));
        assert!(!title_has_all_words("Old Man's War", "The Ghost Brigades"));
    }

    #[test]
    fn a_title_reads_as_releases_name_it() {
        assert_eq!(short_title("Old Man's War: Book 1"), "Old Man's War");
        assert_eq!(
            short_title("Harry Potter and the Chamber of Secrets (Full-Cast Edition)"),
            "Harry Potter and the Chamber of Secrets"
        );
        assert_eq!(short_title("Dune, Book 1"), "Dune");
        assert_eq!(
            short_title("Project Hail Mary: A Novel"),
            "Project Hail Mary"
        );
        assert_eq!(
            short_title("Thinking, Fast and Slow"),
            "Thinking, Fast and Slow"
        );
        assert_eq!(short_title("Spider-Man"), "Spider-Man");
    }

    #[test]
    fn searches_widen_from_the_exact_term_to_the_series() {
        assert_eq!(
            book_search_ladder(
                "John Scalzi Old Man's War: Book 1",
                Some("John Scalzi"),
                "Old Man's War: Book 1",
                Some("Old Man's War Series"),
            ),
            vec![
                "John Scalzi Old Man's War Book 1",
                "John Scalzi Old Man's War",
                "Old Man's War",
                "Old Man's War Series",
            ]
        );
        assert_eq!(
            without_apostrophes("Old Man's War").as_deref(),
            Some("Old Mans War")
        );
    }

    #[test]
    fn ebooks_are_told_apart_from_audiobooks() {
        let seconds = Some(35_748.0);
        let format = |title: &str, categories: &[i32], size: Option<i64>| {
            classify_book_release(title, categories, size, seconds).0
        };
        let megabytes = |count: i64| Some(count * 1024 * 1024);
        assert_eq!(
            format(
                "John Scalzi - [Old Man's War 01-07] (azw3 epub mobi)",
                &[],
                megabytes(25)
            ),
            ReleaseFormat::Ebook
        );
        assert_eq!(
            format(
                "John Scalzi - [Old Man's War 01] - Old Man's War (2011 Retail)",
                &[],
                megabytes(3)
            ),
            ReleaseFormat::Ebook,
            "too small for 10 hours of speech"
        );
        assert_eq!(
            format(
                "[Audiobook] Scalzi, John - Old Mans War Series (1-5)",
                &[],
                megabytes(1400)
            ),
            ReleaseFormat::Audio
        );
        assert_eq!(
            format(
                "John Scalzi - Old Man's War, book 7 (M4B)",
                &[],
                megabytes(531)
            ),
            ReleaseFormat::Audio
        );
        assert_eq!(
            format("Old Man's War", &[CATEGORY_AUDIOBOOK], None),
            ReleaseFormat::Audio
        );
        assert_eq!(
            format("Old Man's War", &[CATEGORY_BOOKS], None),
            ReleaseFormat::Ebook
        );
        assert_eq!(
            format("John Scalzi - Old Man's War (Book 1)", &[], megabytes(340)),
            ReleaseFormat::Unknown
        );
    }

    #[test]
    fn packs_are_told_apart_from_a_numbered_book() {
        assert!(book_pack_hint("[Audiobook] Scalzi, John - Old Mans War Series (1-5)").is_some());
        assert_eq!(
            book_pack_hint("John Scalzi - [Old Man's War 01-07] (azw3 epub mobi)").as_deref(),
            Some("7 books")
        );
        assert!(book_pack_hint("John Scalzi - Old Man's War (Book 1)").is_none());
        assert!(
            book_pack_hint("John Scalzi - Old Man's War, book 7: The Shattering Peace").is_none()
        );
        assert!(book_pack_hint("Movie.2019-2020.1080p").is_none());
    }

    #[test]
    fn the_wanted_book_ranks_above_others_in_its_series() {
        let rank = |title: &str| release_relevance(title, &book(), None);
        let wanted = rank("John Scalzi - Old Man's War (Book 1)");
        let seventh = rank("John Scalzi - Old Man's War, book 7: The Shattering Peace (M4B)");
        let sixth = rank("[r3q] John Scalzi - [Old Man's War 06] - The End of All Things");
        let tagged = rank("(REQ) John Scalzi - The End of All Things (Old Man's War #6) Audiobook");
        assert_eq!(wanted.1, None);
        assert_eq!(seventh.1, Some(7));
        assert_eq!(sixth.1, Some(6));
        assert_eq!(tagged.1, Some(6));
        assert!(
            wanted.0 > seventh.0 && wanted.0 > sixth.0 && wanted.0 > tagged.0,
            "{wanted:?} {seventh:?} {sixth:?} {tagged:?}"
        );
        // A pack's range isn't another book, and a book with no number has nothing to disagree with.
        assert_eq!(
            rank("[Audiobook] Scalzi, John - Old Mans War Series (1-5)").1,
            None
        );
        let plain = BookFacts {
            title: "Dune".to_string(),
            ..Default::default()
        };
        assert_eq!(release_relevance("Dune Book 2", &plain, None).1, None);
        assert_eq!(
            release_relevance("Dune Book 2", &plain, Some("1")).1,
            Some(2)
        );
    }

    #[test]
    fn a_release_with_nothing_usable_is_refused_before_it_downloads() {
        let names = |list: &[&str]| {
            list.iter()
                .map(|name| (*name).to_string())
                .collect::<Vec<_>>()
        };
        let refused = payload_mismatch(
            true,
            false,
            &names(&[
                "a/Old Man's War.azw3",
                "a/Old Man's War.epub",
                "a/Old Man's War.mobi",
            ]),
        )
        .expect("ebooks are not audio");
        assert!(refused.contains(".azw3"), "{refused}");
        assert!(payload_mismatch(true, false, &names(&["a/book.m4b", "a/cover.jpg"])).is_none());
        assert!(payload_mismatch(true, false, &names(&["a/book.rar", "a/book.r00"])).is_none());
        assert!(payload_mismatch(false, true, &names(&["a/movie.mkv"])).is_none());
        assert!(payload_mismatch(false, true, &names(&["a/movie.iso"])).is_some());
        assert!(payload_mismatch(false, false, &names(&["a/anything.txt"])).is_none());
        assert!(
            payload_mismatch(true, false, &[]).is_none(),
            "metadata not known yet"
        );
    }

    #[test]
    fn every_tag_format_reads_the_same() {
        let id3 = tags_from_pairs([
            ("title", "01 - Old Man's War"),
            ("album", "Old Man's War"),
            ("artist", "Scalzi, John"),
            ("track", "01/20"),
        ]);
        assert_eq!(id3.album.as_deref(), Some("Old Man's War"));
        assert_eq!((id3.track, id3.track_total), (Some(1), Some(20)));
        let mp4 = tags_from_pairs([
            ("\u{a9}alb", "Dune"),
            ("com.apple.iTunes:ASIN", "b00b5hzgua"),
            ("\u{a9}nrt", "Scott Brick"),
            ("disc", "2/3"),
            ("----:com.apple.iTunes:SERIES", "Dune Chronicles"),
        ]);
        assert_eq!(mp4.album.as_deref(), Some("Dune"));
        assert_eq!(mp4.asin.as_deref(), Some("B00B5HZGUA"));
        assert_eq!(mp4.narrator.as_deref(), Some("Scott Brick"));
        assert_eq!((mp4.disc, mp4.disc_total), (Some(2), Some(3)));
        assert_eq!(mp4.series.as_deref(), Some("Dune Chronicles"));
        let ape = tags_from_pairs([("Album", "Kind of Blue"), ("Track", "3"), ("Year", "1959")]);
        assert_eq!(ape.album.as_deref(), Some("Kind of Blue"));
        assert_eq!(ape.track, Some(3));
        assert_eq!(ape.date.as_deref(), Some("1959"));
        let vorbis = tags_from_pairs([
            ("ALBUM", "Kind of Blue"),
            ("ALBUMARTIST", "Miles Davis"),
            ("TRACKNUMBER", "4"),
            ("TRACKTOTAL", "5"),
            ("DISCNUMBER", "1"),
        ]);
        assert_eq!(vorbis.album_artist.as_deref(), Some("Miles Davis"));
        assert_eq!(
            (vorbis.track, vorbis.track_total, vorbis.disc),
            (Some(4), Some(5), Some(1))
        );
        assert_eq!(tags_from_pairs([("asin", "not an asin")]).asin, None);
    }

    #[test]
    fn a_pack_is_split_into_its_books_by_tags_or_folders() {
        let files = pack();
        let groups = group_files(&files);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].grouped_by, "tags");
        assert_eq!(groups[0].label, "Old Man's War");
        assert_eq!(groups[0].files.len(), 20);

        // Before anything has downloaded there are no tags, and the folders say the same.
        let bare: Vec<ReleaseFile> = files
            .iter()
            .map(|file| ReleaseFile {
                tags: FileTags::default(),
                duration: None,
                ..file.clone()
            })
            .collect();
        let groups = group_files(&bare);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].grouped_by, "folders");
        assert_eq!(groups[0].files.len(), 20);
    }

    #[test]
    fn the_real_pack_finds_its_book_and_places_chapters_by_time() {
        let files = pack();
        let plan = plan_release(&files, &book(), &chapters());
        let layout = ready(&plan);
        assert_eq!(plan.groups[plan.chosen.unwrap()].label, "Old Man's War");
        assert_eq!(layout.shape, FileShape::Parts);
        assert_eq!(layout.source, ChapterSource::Provider);
        assert_eq!(layout.chapters.len(), 23);
        assert!((layout.length_ratio.unwrap() - 0.998).abs() < 0.001);
        // Chapter One starts about 16 s into part 1; the last chapter is in part 20.
        assert_eq!(layout.chapters[2].file, Some(0));
        assert_eq!(layout.chapters[22].file, Some(19));
        // Chapter Ten starts about 25 minutes into part 7, not at the start of a file.
        let chapter = &layout.chapters[9];
        assert_eq!(chapter.file, Some(6));
        assert!(chapter.offset > 1400.0, "{}", chapter.offset);
        assert!(
            describe(&plan, &book()).contains("20 parts"),
            "{}",
            describe(&plan, &book())
        );
        assert!(
            describe(&plan, &book()).contains("41 other files"),
            "{}",
            describe(&plan, &book())
        );
    }

    #[test]
    fn the_preliminary_answer_from_names_and_sizes_agrees() {
        let bare: Vec<ReleaseFile> = pack()
            .into_iter()
            .map(|file| ReleaseFile {
                tags: FileTags::default(),
                duration: None,
                ..file
            })
            .collect();
        let plan = plan_release(&bare, &book(), &chapters());
        assert!(!plan.complete_facts);
        let layout = ready(&plan);
        assert!(layout.estimated);
        assert_eq!(layout.shape, FileShape::Parts);
        assert_eq!(plan.groups[plan.chosen.unwrap()].files.len(), 20);
    }

    #[test]
    fn the_wrong_book_is_never_accepted_from_a_pack() {
        let only_other: Vec<ReleaseFile> = pack()
            .into_iter()
            .filter(|file| file.key.starts_with("gb-"))
            .collect();
        let plan = plan_release(&only_other, &book(), &chapters());
        match plan.outcome {
            BookOutcome::Review(review) => {
                assert!(
                    review.reason.contains("The Ghost Brigades"),
                    "{}",
                    review.reason
                );
            }
            BookOutcome::Ready(_) => panic!("41 files of another book are not this book"),
        }
    }

    #[test]
    fn an_unhelpful_name_needs_the_length_to_match_closely() {
        let unnamed = |seconds: f64| {
            vec![ReleaseFile {
                key: "x".to_string(),
                path: "abc-def-2019-GROUP/abcdef.m4b".to_string(),
                size: 500_000_000,
                duration: Some(seconds),
                ..Default::default()
            }]
        };
        let plan = plan_release(&unnamed(35_700.0), &book(), &chapters());
        assert!(matches!(plan.outcome, BookOutcome::Ready(_)));
        let plan = plan_release(&unnamed(38_000.0), &book(), &chapters());
        assert!(matches!(plan.outcome, BookOutcome::Review(_)));
    }

    #[test]
    fn part_of_a_book_needs_a_person() {
        let partial: Vec<ReleaseFile> = pack()
            .into_iter()
            .filter(|file| file.key.starts_with("omw-"))
            .take(3)
            .collect();
        let plan = plan_release(&partial, &book(), &chapters());
        match plan.outcome {
            BookOutcome::Review(review) => {
                assert!(review.reason.contains("3 of the 20"), "{}", review.reason);
                assert!(review.fallback.is_some());
            }
            BookOutcome::Ready(_) => panic!("3 of 20 parts is not the whole book"),
        }
    }

    #[test]
    fn an_asin_tag_settles_which_book_it_is() {
        let mut files = pack();
        for file in &mut files {
            if file.key.starts_with("omw-") {
                file.tags.album = Some("Something Unrelated".to_string());
                file.tags.asin = Some("B0DSQ3WM2D".to_string());
            }
        }
        let plan = plan_release(&files, &book(), &chapters());
        assert_eq!(plan.groups[plan.chosen.unwrap()].files.len(), 20);
    }

    fn single_file(seconds: f64, embedded: Vec<EmbeddedChapter>) -> Vec<ReleaseFile> {
        vec![ReleaseFile {
            key: "whole".to_string(),
            path: "Old Man's War/Old Man's War.m4b".to_string(),
            size: 500_000_000,
            duration: Some(seconds),
            tags: FileTags::default(),
            embedded,
        }]
    }

    fn marker(title: &str, start: f64, end: f64) -> EmbeddedChapter {
        EmbeddedChapter {
            title: Some(title.to_string()),
            start,
            end,
        }
    }

    #[test]
    fn markers_in_the_file_replace_the_providers_chapters() {
        let files = single_file(
            35_700.0,
            vec![
                marker("Enlist", 0.0, 20_000.0),
                marker("Serve", 20_000.0, 35_700.0),
            ],
        );
        let plan = plan_release(&files, &book(), &chapters());
        let layout = ready(&plan);
        assert_eq!(layout.source, ChapterSource::Embedded);
        assert_eq!(layout.shape, FileShape::WholeBook);
        assert_eq!(layout.chapters.len(), 2);
        assert_eq!(layout.chapters[1].title, "Serve");
        assert_eq!(layout.chapters[1].offset, 20_000.0);
    }

    #[test]
    fn plain_markers_keep_their_times_and_take_the_providers_names() {
        let mut reference = chapters();
        reference.truncate(2);
        reference[1].start = 18_000.0;
        let files = single_file(
            35_700.0,
            vec![
                marker("Chapter 1", 0.0, 19_000.0),
                marker("Chapter 2", 19_000.0, 35_700.0),
            ],
        );
        let layout_plan = plan_release(&files, &book(), &reference);
        let layout = ready(&layout_plan);
        assert_eq!(layout.chapters[1].title, "Title 2");
        assert_eq!(
            layout.chapters[1].offset, 19_000.0,
            "times come from the file"
        );
    }

    #[test]
    fn one_file_with_no_markers_shares_the_providers_chapters() {
        let plan = plan_release(&single_file(35_700.0, Vec::new()), &book(), &chapters());
        let layout = ready(&plan);
        assert_eq!(layout.shape, FileShape::WholeBook);
        assert_eq!(layout.source, ChapterSource::Provider);
        assert!(
            layout
                .chapters
                .iter()
                .all(|chapter| chapter.file == Some(0))
        );
    }

    #[test]
    fn a_much_shorter_file_is_not_accepted_as_the_book() {
        let plan = plan_release(&single_file(9_000.0, Vec::new()), &book(), &chapters());
        assert!(matches!(plan.outcome, BookOutcome::Review(_)));
    }

    fn per_chapter(names: impl Fn(usize) -> String, jitter: f64) -> Vec<ReleaseFile> {
        chapters()
            .iter()
            .enumerate()
            .map(|(index, chapter)| ReleaseFile {
                key: format!("c{index}"),
                path: format!("Old Man's War/{}.mp3", names(index)),
                size: 10_000_000,
                duration: Some(chapter.seconds.unwrap_or(0.0) + jitter),
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn files_cut_at_the_chapters_are_those_chapters() {
        let files = per_chapter(|index| format!("Old Man's War - {:02}", index + 1), 0.2);
        let plan = plan_release(&files, &book(), &chapters());
        let layout = ready(&plan);
        assert_eq!(layout.shape, FileShape::PerChapter);
        assert_eq!(layout.source, ChapterSource::Provider);
        assert_eq!(layout.chapters[5].title, "Title 6");
        assert_eq!(layout.chapters[5].file, Some(5));
        assert_eq!(layout.chapters[5].offset, 0.0);
    }

    #[test]
    fn file_names_matching_chapter_titles_win_even_if_some_are_missing() {
        let mut files = per_chapter(
            |index| format!("{:02} - Title {}", index + 1, index + 1),
            0.0,
        );
        files.truncate(10);
        let plan = plan_release(&files, &book(), &chapters());
        let layout = ready(&plan);
        assert_eq!(layout.source, ChapterSource::Provider);
        assert_eq!(layout.chapters[3].file, Some(3));
        assert_eq!(layout.chapters[20].file, None, "still wanted");
    }

    #[test]
    fn real_chapter_names_on_a_different_cut_replace_the_providers_chapters() {
        let files: Vec<ReleaseFile> = ["Enlist", "Train", "Fight", "Return"]
            .iter()
            .enumerate()
            .map(|(index, name)| ReleaseFile {
                key: format!("n{index}"),
                path: format!("Old Man's War/{:02} - {name}.mp3", index + 1),
                size: 10_000_000,
                duration: Some(8_937.0),
                ..Default::default()
            })
            .collect();
        let plan = plan_release(&files, &book(), &chapters());
        let layout = ready(&plan);
        assert_eq!(layout.source, ChapterSource::Files);
        assert_eq!(layout.chapters.len(), 4);
        assert_eq!(layout.chapters[2].title, "Fight");
    }

    #[test]
    fn a_different_recording_with_no_chapters_plays_as_honest_parts() {
        let files: Vec<ReleaseFile> = (1..=10)
            .map(|number| ReleaseFile {
                key: format!("p{number}"),
                path: format!("Old Man's War/Old Man's War - {number:02}.mp3"),
                size: 10_000_000,
                // 8% longer than the listed book: a different reading.
                duration: Some(3_860.0),
                ..Default::default()
            })
            .collect();
        let plan = plan_release(&files, &book(), &chapters());
        let layout = ready(&plan);
        assert_eq!(layout.source, ChapterSource::Parts);
        assert_eq!(layout.chapters[0].title, "Part 01");
    }

    #[test]
    fn every_part_belongs_to_a_chapter_even_inside_a_long_one() {
        let plan = plan_release(&pack(), &book(), &chapters());
        let layout = ready(&plan);
        let held = chapter_of_each_file(layout, 20);
        assert!(held.iter().all(Option::is_some), "{held:?}");
        // Sorted by time, so a later part never belongs to an earlier chapter.
        let numbers: Vec<usize> = held.iter().flatten().copied().collect();
        assert!(
            numbers.windows(2).all(|pair| pair[0] <= pair[1]),
            "{numbers:?}"
        );
    }

    #[test]
    fn tracks_use_tags_names_and_lengths_together() {
        let tracks = vec![
            TrackFacts {
                title: "So What".into(),
                number: Some(1),
                disc: Some(1),
                seconds: Some(562.0),
            },
            TrackFacts {
                title: "Freddie Freeloader".into(),
                number: Some(2),
                disc: Some(1),
                seconds: Some(589.0),
            },
            TrackFacts {
                title: "Blue in Green".into(),
                number: Some(3),
                disc: Some(1),
                seconds: Some(337.0),
            },
        ];
        let file =
            |path: &str, title: Option<&str>, track: Option<i32>, seconds: f64| ReleaseFile {
                key: path.to_string(),
                path: path.to_string(),
                duration: Some(seconds),
                tags: FileTags {
                    title: title.map(str::to_string),
                    track,
                    ..Default::default()
                },
                ..Default::default()
            };
        // Tags disagree with the file order, and one file is from another album.
        let files = vec![
            file("a.flac", Some("Blue in Green"), Some(3), 337.5),
            file("b.flac", Some("So What"), Some(1), 561.0),
            file("c.flac", Some("Some Other Song"), None, 200.0),
        ];
        let matched = match_tracks(&files, &[None, None, None], &tracks);
        assert_eq!(matched[0].map(|(track, _)| track), Some(2));
        assert_eq!(matched[1].map(|(track, _)| track), Some(0));
        assert_eq!(matched[2], None);
    }
}
