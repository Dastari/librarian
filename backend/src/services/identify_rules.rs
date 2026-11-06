//! What a download is, worked out from everything it says about itself. Pure, so the rules can be
//! tested against real releases. `library_scan::identify` runs them and records what they found.
//!
//! Nothing here decides alone. Each source adds evidence: tags, text files, names, lengths against a
//! provider's listing, what the narrator says, and Jev. See `docs/design.md` Q52c.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use strsim::jaro_winkler;

use super::release_match::{BookFacts, ReferenceChapter, normalize_words, title_score};

/// Lengths within this share are the same recording; `release_match` uses the same figure.
const SAME_RECORDING: f64 = 0.03;
/// Within this share it can still be the same book, in a different reading or edit.
const SAME_BOOK: f64 = 0.15;
/// A title below this isn't this book, whatever else fits.
const TITLE_ENOUGH: f64 = 0.75;
/// Spelling slips in a name, like "Sclazi" for "Scalzi", still count as the same person.
const NAME_SIMILAR: f64 = 0.9;
/// Under one album, this much audio in tracks too long to be typical songs is a book in parts.
const LONG_ALBUM_SECS: f64 = 3.0 * 3600.0;
/// Tracks at least this long, on an album that long, aren't songs. Deluxe box sets run for hours, but
/// in tracks of about four minutes.
const LONG_PART_SECS: f64 = 8.0 * 60.0;
/// So much under one album that it's a book however short its tracks, as CD rips can be.
const VERY_LONG_ALBUM_SECS: f64 = 7.0 * 3600.0;
/// Tracks this long are chapters or parts rather than songs.
const LONG_TRACK_SECS: f64 = 20.0 * 60.0;

// ---------------------------------------------------------------------------
// What kind of thing it is
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseKind {
    Audiobook,
    Music,
    Tv,
    Movie,
    Ebook,
    Unknown,
}

impl ReleaseKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Audiobook => "audiobook",
            Self::Music => "music",
            Self::Tv => "tv",
            Self::Movie => "movie",
            Self::Ebook => "ebook",
            Self::Unknown => "unknown",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Audiobook => "an audiobook",
            Self::Music => "music",
            Self::Tv => "a TV show",
            Self::Movie => "a movie",
            Self::Ebook => "an ebook",
            Self::Unknown => "something unrecognised",
        }
    }

    pub fn parse(kind: &str) -> Option<Self> {
        match kind {
            "audiobook" => Some(Self::Audiobook),
            "music" => Some(Self::Music),
            "tv" => Some(Self::Tv),
            "movie" => Some(Self::Movie),
            _ => None,
        }
    }

    /// The kind a library holds.
    pub fn from_library_type(library_type: &str) -> Option<Self> {
        match library_type.to_ascii_lowercase().as_str() {
            "audiobooks" | "audiobook" => Some(Self::Audiobook),
            "music" => Some(Self::Music),
            "tv" | "tvshows" | "tv_shows" | "shows" => Some(Self::Tv),
            "movies" | "movie" => Some(Self::Movie),
            _ => None,
        }
    }
}

/// What the files in a download add up to.
#[derive(Debug, Clone, Default)]
pub struct KindFacts {
    /// The download's name.
    pub name: String,
    pub audio_files: usize,
    pub video_files: usize,
    pub ebook_files: usize,
    /// Files only audiobooks come in, such as `.m4b`.
    pub audiobook_format_files: usize,
    /// Video files named as an episode.
    pub episode_files: usize,
    /// Lengths of audio files read so far.
    pub audio_seconds: Vec<f64>,
    /// How many distinct albums the tags name.
    pub album_tags: usize,
    /// Files whose genre tag says it's a book.
    pub audiobook_genre_files: usize,
    /// A text file in the download says it's an audiobook.
    pub text_says_audiobook: bool,
    /// What it was requested as.
    pub hint: Option<ReleaseKind>,
    /// What a person said it is, which overrides everything the files can back up.
    pub chosen: Option<ReleaseKind>,
    /// What's in the library that its files match.
    pub in_library: Option<ReleaseKind>,
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    Some(sorted[sorted.len() / 2])
}

/// The kind, and the plain reason for it.
pub fn classify_kind(facts: &KindFacts) -> (ReleaseKind, String) {
    let words: Vec<String> = normalize_words(&facts.name)
        .split_whitespace()
        .map(str::to_string)
        .collect();
    let named = |wanted: &[&str]| words.iter().any(|word| wanted.contains(&word.as_str()));

    if facts.video_files == 0 && facts.audio_files == 0 {
        return if facts.ebook_files > 0 {
            (
                ReleaseKind::Ebook,
                format!("{} ebook files and no audio or video", facts.ebook_files),
            )
        } else {
            (
                ReleaseKind::Unknown,
                "No audio or video files were found".to_string(),
            )
        };
    }

    // A request is a strong hint, but it can't turn video into a book.
    let fits_hint = |kind: ReleaseKind| match kind {
        ReleaseKind::Audiobook | ReleaseKind::Music => facts.audio_files > 0,
        ReleaseKind::Tv | ReleaseKind::Movie => facts.video_files > 0,
        ReleaseKind::Ebook | ReleaseKind::Unknown => false,
    };
    if let Some(kind) = facts.chosen.filter(|kind| fits_hint(*kind)) {
        return (kind, format!("You said this is {}", kind.label()));
    }
    if let Some(kind) = facts.hint.filter(|kind| fits_hint(*kind)) {
        return (
            kind,
            format!("Requested as {}, and the files fit", kind.label()),
        );
    }
    if let Some(kind) = facts.in_library.filter(|kind| fits_hint(*kind)) {
        return (
            kind,
            format!("Its files match {} in your library", kind.label()),
        );
    }

    if facts.video_files > facts.audio_files {
        if facts.episode_files * 2 >= facts.video_files && facts.episode_files > 0 {
            return (
                ReleaseKind::Tv,
                format!(
                    "{} of {} video files are named as episodes",
                    facts.episode_files, facts.video_files
                ),
            );
        }
        return (
            ReleaseKind::Movie,
            if facts.video_files == 1 {
                "One video file, not named as an episode".to_string()
            } else {
                format!("{} video files, none named as episodes", facts.video_files)
            },
        );
    }

    if facts.audiobook_format_files > 0 {
        return (
            ReleaseKind::Audiobook,
            format!(
                "{} files in an audiobook format",
                facts.audiobook_format_files
            ),
        );
    }
    if facts.audiobook_genre_files * 2 >= facts.audio_files && facts.audiobook_genre_files > 0 {
        return (
            ReleaseKind::Audiobook,
            format!(
                "{} of {} files are tagged as books",
                facts.audiobook_genre_files, facts.audio_files
            ),
        );
    }
    if named(&[
        "audiobook",
        "audiobooks",
        "unabridged",
        "abridged",
        "narrated",
    ]) {
        return (
            ReleaseKind::Audiobook,
            "Its name says it's an audiobook".to_string(),
        );
    }
    if facts.text_says_audiobook {
        return (
            ReleaseKind::Audiobook,
            "A text file in the download says it's an audiobook".to_string(),
        );
    }
    if let Some(typical) = median(&facts.audio_seconds) {
        if typical >= LONG_TRACK_SECS {
            return (
                ReleaseKind::Audiobook,
                format!(
                    "Files run about {} minutes each, too long for songs",
                    (typical / 60.0).round()
                ),
            );
        }
        let total: f64 = facts.audio_seconds.iter().sum();
        let per_album = total / facts.album_tags.max(1) as f64;
        if (per_album >= LONG_ALBUM_SECS && typical >= LONG_PART_SECS)
            || per_album >= VERY_LONG_ALBUM_SECS
        {
            return (
                ReleaseKind::Audiobook,
                format!(
                    "About {} hours of audio per album, too long for music",
                    (total / facts.album_tags.max(1) as f64 / 3600.0).round()
                ),
            );
        }
        return (
            ReleaseKind::Music,
            format!(
                "Files run about {} minutes each, like songs",
                (typical / 60.0).round()
            ),
        );
    }
    (
        ReleaseKind::Music,
        "Audio files with no length known yet, so this may change".to_string(),
    )
}

// ---------------------------------------------------------------------------
// Text files in the download
// ---------------------------------------------------------------------------

/// What a text file in a download says about one thing in it. Everything in here came from a stranger
/// on the internet: it's evidence to compare, and it's never acted on directly.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextBlock {
    pub title: Option<String>,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub series: Option<String>,
    pub series_position: Option<String>,
    pub genre: Option<String>,
    pub file_count: Option<usize>,
    pub seconds: Option<f64>,
}

/// What a whole text file says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextFacts {
    pub blocks: Vec<TextBlock>,
    /// Lines warning about damaged files.
    pub warnings: Vec<String>,
}

/// `9:54:38` or `54:38`.
fn clock_seconds(text: &str) -> Option<f64> {
    let parts: Vec<&str> = text.trim().split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    let mut total = 0.0;
    for part in parts {
        total = total * 60.0 + part.trim().parse::<f64>().ok()?;
    }
    Some(total)
}

fn quoted_value(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line.rfind('"')?;
    (end > start + 1).then(|| line[start + 1..end].trim().to_string())
}

/// Read an info, nfo or cue file. Keeps what it recognises, and a file it can't read gives nothing.
pub fn read_text_file(file_name: &str, text: &str) -> TextFacts {
    let mut facts = TextFacts::default();
    if file_name.to_ascii_lowercase().ends_with(".cue") {
        let mut block = TextBlock::default();
        for line in text.lines() {
            let line = line.trim();
            if line.to_ascii_uppercase().starts_with("TRACK ") {
                break;
            }
            if line.to_ascii_uppercase().starts_with("TITLE ") {
                block.title = quoted_value(line);
            } else if line.to_ascii_uppercase().starts_with("PERFORMER ") {
                block.author = quoted_value(line);
            }
        }
        if block.title.is_some() {
            facts.blocks.push(block);
        }
        return facts;
    }
    let mut current = TextBlock::default();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.to_ascii_lowercase().contains("bad file") && trimmed.chars().count() <= 200 {
            facts.warnings.push(
                trimmed
                    .trim_start_matches(|c: char| !c.is_alphanumeric())
                    .to_string(),
            );
        }
        let Some((key, value)) = trimmed.split_once([':', '=']) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() || value.chars().count() > 200 {
            continue;
        }
        let key = key
            .trim()
            .trim_end_matches('.')
            .trim()
            .to_ascii_lowercase()
            .replace(['_', '-'], " ");
        let value_owned = value.to_string();
        match key.as_str() {
            "title" | "album" | "book" | "book title" => {
                if current.title.is_some() {
                    facts.blocks.push(std::mem::take(&mut current));
                }
                current.title = Some(value_owned);
            }
            "author" | "written by" | "artist" | "album artist" => {
                current.author = Some(value_owned);
            }
            "read by" | "narrator" | "narrated by" | "reader" => {
                current.narrator = Some(value_owned);
            }
            "series" | "series name" => current.series = Some(value_owned),
            "position in series" | "series number" | "book number" => {
                current.series_position = Some(value_owned);
            }
            "genre" => current.genre = Some(value_owned),
            "number of mp3s" | "number of files" | "files" | "tracks" | "number of tracks" => {
                current.file_count = value.split_whitespace().next().and_then(|n| n.parse().ok());
            }
            "total duration" | "duration" | "total length" | "length" | "running time"
            | "runtime" => {
                current.seconds = clock_seconds(value);
            }
            _ => {}
        }
    }
    if current.title.is_some() {
        facts.blocks.push(current);
    }
    facts
}

/// Whether anything a text file says marks the download as an audiobook.
pub fn text_says_audiobook(text: &TextFacts) -> bool {
    text.blocks.iter().any(|block| {
        block.narrator.is_some() || block.genre.as_deref().is_some_and(is_audiobook_genre)
    })
}

pub fn is_audiobook_genre(genre: &str) -> bool {
    let genre = normalize_words(genre);
    ["audiobook", "audio book", "spoken", "speech", "books"]
        .iter()
        .any(|word| genre.contains(word))
}

/// The block describing a group of files, by its name and then its file count.
pub fn block_for_group<'a>(
    blocks: &'a [TextBlock],
    group_title: &str,
    file_count: usize,
) -> Option<&'a TextBlock> {
    let target = BookFacts {
        title: group_title.to_string(),
        ..Default::default()
    };
    blocks
        .iter()
        .filter_map(|block| {
            let score = title_score(block.title.as_deref()?, &target);
            (score >= TITLE_ENOUGH).then_some((block, score))
        })
        .max_by(|a, b| {
            let exact = |block: &TextBlock| block.file_count == Some(file_count);
            exact(a.0)
                .cmp(&exact(b.0))
                .then_with(|| a.1.total_cmp(&b.1))
        })
        .map(|(block, _)| block)
}

// ---------------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------------

/// A title without what's in brackets, which is about the release rather than the book: `The Last
/// Colony (Unabridged)`, `Zoes Tale (Book 4)`. A title that's all brackets is left alone.
pub fn tidy_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut depth = 0usize;
    for ch in title.chars() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    let out = out.trim_matches(|c: char| c == '-' || c.is_whitespace());
    if out.is_empty() {
        title.trim().to_string()
    } else {
        out.to_string()
    }
}

/// The title in a folder name, after any author: `John Scalzi - The Last Colony (Book 3)`.
pub fn title_from_label(label: &str) -> String {
    let title = match author_from_label(label) {
        Some(_) => label.split_once(" - ").map_or(label, |(_, rest)| rest),
        None => label,
    };
    tidy_title(title)
}

/// `Scalzi, John` to `John Scalzi`. Anything else is left as it is.
pub fn tidy_person_name(name: &str) -> String {
    let name = name.trim();
    match name.split_once(',') {
        Some((last, first)) if !first.contains(',') && !first.trim().is_empty() => {
            format!("{} {}", first.trim(), last.trim())
        }
        _ => name.to_string(),
    }
}

/// `John Scalzi - The Last Colony (Book 3)` names its author. Only a person-shaped first part counts.
pub fn author_from_label(label: &str) -> Option<String> {
    let (first, rest) = label.split_once(" - ")?;
    let words = first.split_whitespace().count();
    let has_digit = first.chars().any(|c| c.is_ascii_digit());
    ((2..=4).contains(&words) && !has_digit && !rest.trim().is_empty())
        .then(|| tidy_person_name(first))
}

/// Whether two names are the same person, allowing for order and spelling slips. Their surnames, or
/// the longest word of each, have to agree.
pub fn same_person(a: &str, b: &str) -> bool {
    let words = |name: &str| -> Vec<String> {
        normalize_words(name)
            .split_whitespace()
            .map(str::to_string)
            .collect()
    };
    let (a, b) = (words(a), words(b));
    let Some(longest) = a.iter().max_by_key(|word| word.len()) else {
        return false;
    };
    b.iter()
        .any(|word| word == longest || jaro_winkler(word, longest) >= NAME_SIMILAR)
}

// ---------------------------------------------------------------------------
// Which edition of a book
// ---------------------------------------------------------------------------

/// A listing that could be this book: on a provider, or already in the library.
#[derive(Debug, Clone, Default)]
pub struct EditionFacts {
    pub title: String,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub seconds: Option<f64>,
    pub chapters: Option<usize>,
}

/// What's known of the files being matched.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GroupIdentity {
    pub titles: Vec<String>,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub seconds: Option<f64>,
    pub file_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditionFit {
    /// The same recording: its length is within a few percent.
    Recording,
    /// Close, so maybe another reading or edit.
    Book,
    /// Not this: its length is far off.
    Different,
    /// Nothing to compare yet.
    Unknown,
}

impl EditionFit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::Book => "book",
            Self::Different => "different",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditionVerdict {
    pub title_score: f64,
    /// `None` when there's no name to compare.
    pub author_matches: Option<bool>,
    pub narrator_matches: Option<bool>,
    pub length_ratio: Option<f64>,
    pub fit: EditionFit,
    pub chapters_match_files: bool,
    /// Zero to one, for ranking and for showing.
    pub score: f64,
    pub reasons: Vec<String>,
}

impl EditionVerdict {
    /// Could this be the book at all? Used for what's already in the library too.
    pub fn plausible(&self) -> bool {
        self.title_score >= TITLE_ENOUGH
            && self.author_matches != Some(false)
            && self.fit != EditionFit::Different
    }
}

fn any_person(names: &[String], other: Option<&str>) -> Option<bool> {
    let other = other.filter(|name| !name.trim().is_empty())?;
    if names.is_empty() {
        return None;
    }
    Some(names.iter().any(|name| same_person(name, other)))
}

/// Compare a listing with the files.
pub fn judge_edition(group: &GroupIdentity, edition: &EditionFacts) -> EditionVerdict {
    // Listings often carry a series or subtitle after a colon: "The Last Colony: Old Man's War Book 3".
    // The title before it counts too, and length and author still have to agree.
    let main_title = edition.title.split_once(':').map(|(main, _)| main.trim());
    let score_against = |listed: &str| {
        let facts = BookFacts {
            title: listed.to_string(),
            author: edition.author.clone(),
            ..Default::default()
        };
        group
            .titles
            .iter()
            .map(|title| title_score(title, &facts))
            .fold(0.0, f64::max)
    };
    let title_score = score_against(&edition.title).max(
        main_title
            .filter(|main| !main.is_empty())
            .map_or(0.0, score_against),
    );
    let author_matches = any_person(&group.authors, edition.author.as_deref());
    let narrator_matches = any_person(&group.narrators, edition.narrator.as_deref());
    let length_ratio = match (group.seconds, edition.seconds) {
        (Some(files), Some(listed)) if listed > 0.0 => Some(files / listed),
        _ => None,
    };
    let fit = match length_ratio {
        Some(ratio) if (ratio - 1.0).abs() <= SAME_RECORDING => EditionFit::Recording,
        Some(ratio) if (ratio - 1.0).abs() <= SAME_BOOK => EditionFit::Book,
        Some(_) => EditionFit::Different,
        None => EditionFit::Unknown,
    };
    let chapters_match_files = group.file_count > 1 && edition.chapters == Some(group.file_count);

    let mut reasons = Vec::new();
    match author_matches {
        Some(true) => reasons.push("Author matches".to_string()),
        Some(false) => reasons.push(format!(
            "A different author: {}",
            edition.author.as_deref().unwrap_or_default()
        )),
        None => {}
    }
    match narrator_matches {
        Some(true) => reasons.push("Narrator matches".to_string()),
        Some(false) => reasons.push(format!(
            "A different narrator: {}",
            edition.narrator.as_deref().unwrap_or_default()
        )),
        None => {}
    }
    if let Some(ratio) = length_ratio {
        reasons.push(format!("{:.0}% of the files' length", 100.0 / ratio));
    }
    if chapters_match_files {
        reasons.push(format!(
            "{} chapters, the same as the {} files",
            group.file_count, group.file_count
        ));
    }

    let length_part = match fit {
        EditionFit::Recording => 1.0,
        EditionFit::Book => 0.6,
        EditionFit::Unknown => 0.4,
        EditionFit::Different => 0.0,
    };
    let author_part = match author_matches {
        Some(true) => 1.0,
        None => 0.5,
        Some(false) => 0.0,
    };
    let narrator_part = match narrator_matches {
        Some(true) => 1.0,
        None => 0.5,
        Some(false) => 0.0,
    };
    let score = title_score * 0.4 + author_part * 0.25 + length_part * 0.25 + narrator_part * 0.1;
    EditionVerdict {
        title_score,
        author_matches,
        narrator_matches,
        length_ratio,
        fit,
        chapters_match_files,
        score,
        reasons,
    }
}

/// How the listings rank, best first: indexes into `verdicts`.
pub fn rank_editions(editions: &[EditionFacts], verdicts: &[EditionVerdict]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..verdicts.len()).collect();
    let closeness = |verdict: &EditionVerdict| {
        verdict
            .length_ratio
            .map_or(f64::MAX, |ratio| (ratio - 1.0).abs())
    };
    order.sort_by(|a, b| {
        verdicts[*b]
            .score
            .total_cmp(&verdicts[*a].score)
            .then_with(|| {
                verdicts[*b]
                    .chapters_match_files
                    .cmp(&verdicts[*a].chapters_match_files)
            })
            .then_with(|| {
                let listed = |index: usize| editions[index].chapters.unwrap_or(0) > 0;
                listed(*b).cmp(&listed(*a))
            })
            .then_with(|| closeness(&verdicts[*a]).total_cmp(&closeness(&verdicts[*b])))
    });
    order
}

/// The one listing that's clearly right, if there is one. It must be the same recording by the right
/// author. Two readings that both fit leave it to a person, whereas two listings of one reading don't
/// matter: the book's the same either way.
pub fn clear_edition(editions: &[EditionFacts], verdicts: &[EditionVerdict]) -> Option<usize> {
    let fitting: Vec<usize> = rank_editions(editions, verdicts)
        .into_iter()
        .filter(|index| {
            let verdict = &verdicts[*index];
            verdict.plausible()
                && verdict.fit == EditionFit::Recording
                && verdict.narrator_matches != Some(false)
        })
        .collect();
    let best = *fitting.first()?;
    let narrator = |index: usize| editions[index].narrator.as_deref().map(normalize_words);
    let same_reading = fitting
        .iter()
        .all(|index| match (narrator(best), narrator(*index)) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        });
    same_reading.then_some(best)
}

/// What to search for, most specific first. An author with a slip in it finds nothing, so a plain title
/// is always tried too, and apostrophes are kept as well as dropped since providers treat them
/// differently.
pub fn edition_searches(titles: &[String], authors: &[String]) -> Vec<String> {
    let mut searches = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |query: String| {
        let query = query.split_whitespace().collect::<Vec<_>>().join(" ");
        if !query.is_empty() && seen.insert(query.to_lowercase()) {
            searches.push(query);
        }
    };
    for title in titles {
        for author in authors {
            add(format!("{title} {}", tidy_person_name(author)));
        }
    }
    for title in titles {
        add(title.clone());
    }
    searches
}

// ---------------------------------------------------------------------------
// What a narrator says
// ---------------------------------------------------------------------------

/// One spoken word and when it starts.
#[derive(Debug, Clone, PartialEq)]
pub struct HeardWord {
    pub text: String,
    pub start: f64,
}

/// What to listen for at a chapter's start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChapterCue {
    /// "Chapter Ten", "Chapter 10", "Chapter X".
    Number(u32),
    /// A section with its own name, such as "Prologue" or "Part One".
    Words(Vec<String>),
}

/// Named sections narrators read aloud. Other single words are too common to trust: a chapter called
/// "Home" would be found wherever someone mentions home.
const SPOKEN_SECTIONS: &[&str] = &[
    "prologue",
    "epilogue",
    "introduction",
    "foreword",
    "preface",
    "afterword",
    "interlude",
    "postscript",
    "acknowledgments",
    "acknowledgements",
];
/// Never announced, so there's nothing to hear.
const UNSPOKEN_SECTIONS: &[&str] = &["credits", "credit", "dedication", "copyright"];

fn word_number(word: &str) -> Option<u32> {
    Some(match word {
        "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        "thirteen" => 13,
        "fourteen" => 14,
        "fifteen" => 15,
        "sixteen" => 16,
        "seventeen" => 17,
        "eighteen" => 18,
        "nineteen" => 19,
        "twenty" => 20,
        "thirty" => 30,
        "forty" => 40,
        "fifty" => 50,
        "sixty" => 60,
        "seventy" => 70,
        "eighty" => 80,
        "ninety" => 90,
        _ => return None,
    })
}

fn roman_number(word: &str) -> Option<u32> {
    if word.is_empty() || word.len() > 8 {
        return None;
    }
    let value = |c: char| match c {
        'i' => Some(1),
        'v' => Some(5),
        'x' => Some(10),
        'l' => Some(50),
        'c' => Some(100),
        _ => None,
    };
    let mut total = 0;
    let digits: Vec<u32> = word.chars().map(value).collect::<Option<_>>()?;
    for (index, digit) in digits.iter().enumerate() {
        if digits.get(index + 1).is_some_and(|next| next > digit) {
            total -= *digit as i32;
        } else {
            total += *digit as i32;
        }
    }
    u32::try_from(total).ok().filter(|total| *total > 0)
}

/// The number at the start of these words, and how many words it used.
fn number_in_words(words: &[String]) -> Option<(u32, usize)> {
    let first = words.first()?;
    if let Ok(number) = first.parse::<u32>() {
        return Some((number, 1));
    }
    if let Some(number) = word_number(first) {
        if number >= 20
            && let Some(unit) = words.get(1).and_then(|word| word_number(word))
            && unit < 10
        {
            return Some((number + unit, 2));
        }
        return Some((number, 1));
    }
    roman_number(first).map(|number| (number, 1))
}

/// Words as they're compared: lower case, without punctuation, hyphens split.
fn speech_words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| c.is_whitespace() || c == '-')
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

/// What to listen for from a chapter's title, if anything can be heard. Credits and titles of a single
/// ordinary word give `None`, and those chapters keep their place relative to the ones around them.
pub fn chapter_cue(title: &str) -> Option<ChapterCue> {
    let words = speech_words(title);
    if words.is_empty()
        || words
            .iter()
            .any(|word| UNSPOKEN_SECTIONS.contains(&word.as_str()))
    {
        return None;
    }
    if let Some(position) = words
        .iter()
        .position(|word| word == "chapter" || word == "ch")
        && let Some((number, _)) = number_in_words(&words[position + 1..])
    {
        return Some(ChapterCue::Number(number));
    }
    if words.len() == 1 && !SPOKEN_SECTIONS.contains(&words[0].as_str()) {
        // A bare "12" is a chapter number. A bare word is too common to trust.
        return words[0].parse::<u32>().ok().map(ChapterCue::Number);
    }
    Some(ChapterCue::Words(words))
}

/// Where this cue was said, nearest to where it was expected.
pub fn find_cue(heard: &[HeardWord], cue: &ChapterCue, expected: f64) -> Option<f64> {
    let tokens: Vec<(String, f64)> = heard
        .iter()
        .flat_map(|word| {
            speech_words(&word.text)
                .into_iter()
                .map(move |text| (text, word.start))
        })
        .collect();
    let words: Vec<String> = tokens.iter().map(|(text, _)| text.clone()).collect();
    let mut found = Vec::new();
    for (index, (text, start)) in tokens.iter().enumerate() {
        let matched = match cue {
            ChapterCue::Number(number) => {
                text == "chapter"
                    && number_in_words(&words[index + 1..]).is_some_and(|(said, _)| said == *number)
            }
            ChapterCue::Words(wanted) => words[index..].starts_with(wanted),
        };
        if matched {
            found.push(*start);
        }
    }
    found
        .into_iter()
        .min_by(|a, b| (a - expected).abs().total_cmp(&(b - expected).abs()))
}

/// Read `whisper-cli`'s word-by-word output: `[00:00:12.340 --> 00:00:12.900]   Chapter`.
pub fn parse_whisper_words(output: &str) -> Vec<HeardWord> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix('[')?;
            let (times, text) = rest.split_once(']')?;
            let (start, _) = times.split_once("-->")?;
            let text = text.trim();
            (!text.is_empty()).then(|| HeardWord {
                text: text.to_string(),
                start: clock_seconds(start.trim()).unwrap_or(0.0),
            })
        })
        .collect()
}

/// Chapter starts, from what was heard. A heard chapter is exact. One that wasn't heard falls between
/// the heard chapters around it, in proportion to the listing, which holds whether a release has extra
/// audio in one place or runs a little longer throughout. Before the first or after the last heard
/// chapter it keeps the listing's distance from that chapter. Starts never go backwards.
pub fn settle_chapter_starts(
    reference: &[ReferenceChapter],
    scale: f64,
    heard: &[Option<f64>],
    total: f64,
) -> Vec<f64> {
    let anchors: Vec<(f64, f64)> = reference
        .iter()
        .zip(heard)
        .filter_map(|(chapter, heard)| heard.map(|at| (chapter.start, at)))
        .collect();
    let mut starts = Vec::with_capacity(reference.len());
    let mut previous: f64 = 0.0;
    for (index, chapter) in reference.iter().enumerate() {
        let listed = chapter.start;
        let before = anchors.iter().rev().find(|(at, _)| *at <= listed);
        let after = anchors.iter().find(|(at, _)| *at >= listed);
        let start = match (heard.get(index).copied().flatten(), before, after) {
            (Some(at), _, _) => at,
            (None, _, _) if listed <= 0.0 => 0.0,
            (None, Some((from, a)), Some((to, b))) if to > from => {
                a + (b - a) * (listed - from) / (to - from)
            }
            (None, Some((from, a)), _) => a + (listed - from),
            (None, None, Some((to, b))) => b - (to - listed),
            (None, None, None) => listed * scale,
        };
        let start = start.max(previous).min(total.max(previous));
        starts.push(start);
        previous = start;
    }
    starts
}

/// A pause in a recording: where it ends and how long it lasted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pause {
    pub end: f64,
    pub seconds: f64,
}

/// The pauses in `ffmpeg -af silencedetect` output.
pub fn parse_pauses(output: &str) -> Vec<Pause> {
    output
        .lines()
        .filter_map(|line| {
            let (_, rest) = line.split_once("silence_end:")?;
            let (end, duration) = rest.split_once("silence_duration:")?;
            Some(Pause {
                end: end.trim().trim_end_matches('|').trim().parse().ok()?,
                seconds: duration.trim().parse().ok()?,
            })
        })
        .collect()
}

/// Where a chapter announced at `word` really starts: the end of the last real pause before the word.
/// Word timings are out by a second or so either way. A short gap can be the one between "Chapter" and
/// its number, so only a pause long enough to be a break counts, unless there's nothing else nearby.
pub fn start_of_announcement(word: f64, pauses: &[Pause]) -> Option<f64> {
    /// Timing can place the word this far before the pause it follows.
    const WORD_EARLY_SECS: f64 = 0.5;
    const BREAK_SECS: f64 = 0.7;
    let before: Vec<&Pause> = pauses
        .iter()
        .filter(|pause| pause.end <= word + WORD_EARLY_SECS)
        .collect();
    before
        .iter()
        .filter(|pause| pause.seconds >= BREAK_SECS)
        .map(|pause| pause.end)
        .fold(None, |latest: Option<f64>, end| {
            Some(latest.map_or(end, |at| at.max(end)))
        })
        .or_else(|| {
            before
                .iter()
                .max_by(|a, b| a.seconds.total_cmp(&b.seconds))
                .map(|pause| pause.end)
        })
}

/// Where a chapter nobody heard announced probably starts, given where it's expected: the longest pause
/// nearby, since narrators leave one between chapters.
pub fn break_near(expected: f64, reach: f64, pauses: &[Pause]) -> Option<f64> {
    pauses
        .iter()
        .filter(|pause| (pause.end - expected).abs() <= reach)
        .max_by(|a, b| a.seconds.total_cmp(&b.seconds))
        .map(|pause| pause.end)
}

/// Whether what was heard is enough to move chapters. Heard chapters must be most of those a narrator
/// could announce, or the few words found may be chance.
pub fn enough_heard(cues: usize, heard: usize) -> bool {
    cues >= 2 && heard >= 2 && heard * 2 >= cues
}

/// The words of a title and author heard in a recording's opening, as evidence of which book it is.
/// Providers' names are often misheard, so word by word and allowing for slips.
pub fn opening_mentions(heard: &str, phrase: &str) -> bool {
    let words = speech_words(heard);
    let wanted = speech_words(phrase);
    let meaningful: Vec<&String> = wanted
        .iter()
        .filter(|word| word.len() > 2 && !matches!(word.as_str(), "the" | "and" | "book"))
        .collect();
    !meaningful.is_empty()
        && meaningful
            .iter()
            .all(|word| words.iter().any(|said| jaro_winkler(said, word) >= 0.85))
}

/// An Audible id in a name or tag: `Harry Potter (Full-Cast Edition) [B0F14RFHQX]`. Whoever made the
/// release looked the book up, so this says exactly which listing it is.
pub fn audible_id_in(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .find(|word| {
            word.len() == 10
                && word.starts_with("B0")
                && word
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                && word.chars().skip(2).any(|c| c.is_ascii_uppercase())
        })
        .map(str::to_string)
}

/// The title and author in the way recordings introduce themselves: `... presents "The Last Colony" by
/// John Scalzi, read by ...`. Names are often misheard, so this is something to search for, not a fact.
pub fn title_from_opening(heard: &str) -> Option<(String, String)> {
    let lower = heard.to_lowercase();
    let start = lower.find("presents ").map(|at| at + "presents ".len())?;
    let rest = &heard[start..];
    let by = rest.to_lowercase().find(" by ")?;
    let title = rest[..by]
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric());
    let after = &rest[by + " by ".len()..];
    let end = after.find([',', '.']).unwrap_or(after.len());
    let author = after[..end]
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric());
    let words = author.split_whitespace().count();
    (!title.is_empty() && (1..=4).contains(&words) && title.split_whitespace().count() <= 12)
        .then(|| (title.to_string(), author.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chapter(title: &str, start: f64) -> ReferenceChapter {
        ReferenceChapter {
            title: Some(title.to_string()),
            start,
            seconds: None,
        }
    }

    fn word(text: &str, start: f64) -> HeardWord {
        HeardWord {
            text: text.to_string(),
            start,
        }
    }

    fn scalzi_pack_group() -> GroupIdentity {
        GroupIdentity {
            titles: vec!["The Last Colony (Unabridged)".to_string()],
            authors: vec!["John Scalzi".to_string()],
            narrators: vec!["William Dufris".to_string()],
            seconds: Some(35596.0),
            file_count: 52,
        }
    }

    fn edition(title: &str, author: &str, narrator: &str, minutes: f64) -> EditionFacts {
        EditionFacts {
            title: title.to_string(),
            author: Some(author.to_string()),
            narrator: Some(narrator.to_string()),
            seconds: Some(minutes * 60.0),
            chapters: None,
        }
    }

    fn judged(group: &GroupIdentity, editions: &[EditionFacts]) -> Vec<EditionVerdict> {
        editions
            .iter()
            .map(|edition| judge_edition(group, edition))
            .collect()
    }

    #[test]
    fn the_right_last_colony_beats_others_of_the_same_name() {
        let editions = vec![
            edition("The Last Colony", "Mike Robb", "A Reader", 115.0),
            edition("The Last Colony", "Philippe Sands", "Philippe Sands", 345.0),
            edition(
                "The Last Colony: Old Man's War Book 3",
                "John Scalzi",
                "William Dufris",
                591.0,
            ),
            edition(
                "The Last Colony: Old Man's War, Book 3",
                "John Scalzi",
                "Wil Wheaton",
                564.0,
            ),
        ];
        let verdicts = judged(&scalzi_pack_group(), &editions);
        assert_eq!(
            clear_edition(&editions, &verdicts),
            Some(2),
            "{verdicts:#?}"
        );
        assert!(!verdicts[0].plausible(), "wrong author and length");
    }

    #[test]
    fn nothing_is_clear_when_only_the_wrong_book_is_listed() {
        let editions = vec![edition("The Last Colony", "Mike Robb", "A Reader", 115.0)];
        let verdicts = judged(&scalzi_pack_group(), &editions);
        assert_eq!(clear_edition(&editions, &verdicts), None);
    }

    #[test]
    fn two_listings_of_one_reading_are_not_a_choice() {
        let editions = vec![
            edition("The Ghost Brigades", "John Scalzi", "William Dufris", 625.0),
            edition(
                "The Ghost Brigades: Old Man's War Book 2",
                "John Scalzi",
                "William Dufris",
                626.0,
            ),
        ];
        let group = GroupIdentity {
            titles: vec!["The Ghost Brigades".to_string()],
            authors: vec!["John Sclazi".to_string()],
            seconds: Some(37527.0),
            file_count: 41,
            ..Default::default()
        };
        assert!(clear_edition(&editions, &judged(&group, &editions)).is_some());
    }

    #[test]
    fn two_readings_of_the_same_length_are_left_to_a_person() {
        let editions = vec![
            edition("Distant Shores", "An Author", "Reader One", 600.0),
            edition("Distant Shores", "An Author", "Reader Two", 601.0),
        ];
        let group = GroupIdentity {
            titles: vec!["Distant Shores".to_string()],
            authors: vec!["An Author".to_string()],
            seconds: Some(36000.0),
            file_count: 10,
            ..Default::default()
        };
        assert_eq!(clear_edition(&editions, &judged(&group, &editions)), None);
    }

    #[test]
    fn matching_chapter_count_breaks_a_tie() {
        let mut editions = vec![
            edition("Distant Shores", "An Author", "Same Reader", 600.0),
            edition("Distant Shores", "An Author", "Same Reader", 600.0),
        ];
        editions[1].chapters = Some(10);
        let group = GroupIdentity {
            titles: vec!["Distant Shores".to_string()],
            authors: vec!["An Author".to_string()],
            seconds: Some(36000.0),
            file_count: 10,
            ..Default::default()
        };
        assert_eq!(
            clear_edition(&editions, &judged(&group, &editions)),
            Some(1)
        );
    }

    #[test]
    fn titles_lose_what_is_about_the_release() {
        assert_eq!(
            tidy_title("The Last Colony (Unabridged)"),
            "The Last Colony"
        );
        assert_eq!(tidy_title("[Unabridged]"), "[Unabridged]");
        assert_eq!(
            title_from_label("John Sclazi - Zoes Tale (Book 4)"),
            "Zoes Tale"
        );
        assert_eq!(title_from_label("The Sagan Diary"), "The Sagan Diary");
    }

    #[test]
    fn people_match_across_order_and_slips() {
        assert!(same_person("Scalzi, John", "John Scalzi"));
        assert!(same_person("John Sclazi", "John Scalzi"));
        assert!(!same_person("Mike Robb", "John Scalzi"));
        assert_eq!(tidy_person_name("Scalzi, John"), "John Scalzi");
    }

    #[test]
    fn labels_name_their_authors_only_when_they_look_like_people() {
        assert_eq!(
            author_from_label("John Scalzi - The Last Colony (Book 3)").as_deref(),
            Some("John Scalzi")
        );
        assert_eq!(author_from_label("Book 3 - Something"), None);
        assert_eq!(author_from_label("The Last Colony"), None);
    }

    #[test]
    fn searches_try_the_author_then_the_plain_title() {
        let searches = edition_searches(
            &["The Last Colony".to_string()],
            &["Scalzi, John".to_string()],
        );
        assert_eq!(
            searches,
            vec!["The Last Colony John Scalzi", "The Last Colony"]
        );
    }

    #[test]
    fn the_real_pack_is_an_audiobook_by_name() {
        let (kind, _) = classify_kind(&KindFacts {
            name: "[Audiobook] Scalzi, John - Old Mans War Series (1-5)".to_string(),
            audio_files: 190,
            ..Default::default()
        });
        assert_eq!(kind, ReleaseKind::Audiobook);
    }

    #[test]
    fn short_parts_of_a_long_album_are_a_book_not_music() {
        let (kind, reason) = classify_kind(&KindFacts {
            name: "Some Release".to_string(),
            audio_files: 52,
            audio_seconds: vec![684.0; 52],
            album_tags: 1,
            ..Default::default()
        });
        assert_eq!(kind, ReleaseKind::Audiobook, "{reason}");
    }

    #[test]
    fn a_deluxe_box_set_of_songs_is_still_music() {
        // Guns N' Roses, Appetite For Destruction (Super Deluxe): 51 tracks, 212 minutes.
        let (kind, reason) = classify_kind(&KindFacts {
            name: "Guns N' Roses - Appetite For Destruction (Super Deluxe)".to_string(),
            audio_files: 51,
            audio_seconds: vec![250.0; 51],
            album_tags: 1,
            ..Default::default()
        });
        assert_eq!(kind, ReleaseKind::Music, "{reason}");
    }

    #[test]
    fn a_long_rip_in_short_tracks_is_a_book() {
        let (kind, reason) = classify_kind(&KindFacts {
            name: "Some Release".to_string(),
            audio_files: 180,
            audio_seconds: vec![200.0; 180],
            album_tags: 1,
            ..Default::default()
        });
        assert_eq!(kind, ReleaseKind::Audiobook, "{reason}");
    }

    #[test]
    fn an_album_named_after_its_artist_can_be_matched() {
        let score = title_score(
            "Foo Fighters",
            &BookFacts {
                title: "Foo Fighters".to_string(),
                author: Some("Foo Fighters".to_string()),
                ..Default::default()
            },
        );
        assert!(score > 0.99, "{score}");
    }

    #[test]
    fn audible_ids_are_found_in_names() {
        assert_eq!(
            audible_id_in("Harry Potter (Full-Cast Edition) [B0F14RFHQX]").as_deref(),
            Some("B0F14RFHQX")
        );
        assert_eq!(audible_id_in("Scalzi, John -- 01 of 52"), None);
        assert_eq!(audible_id_in("B000000000"), None, "a number, not an id");
    }

    #[test]
    fn an_album_of_songs_is_music() {
        let (kind, _) = classify_kind(&KindFacts {
            name: "Artist - Album (2010) [FLAC]".to_string(),
            audio_files: 12,
            audio_seconds: vec![240.0; 12],
            album_tags: 1,
            ..Default::default()
        });
        assert_eq!(kind, ReleaseKind::Music);
    }

    #[test]
    fn video_is_tv_when_named_as_episodes_and_a_movie_otherwise() {
        let tv = KindFacts {
            name: "A Show S01".to_string(),
            video_files: 8,
            episode_files: 8,
            ..Default::default()
        };
        assert_eq!(classify_kind(&tv).0, ReleaseKind::Tv);
        let movie = KindFacts {
            name: "A Film 2010".to_string(),
            video_files: 1,
            ..Default::default()
        };
        assert_eq!(classify_kind(&movie).0, ReleaseKind::Movie);
    }

    #[test]
    fn a_request_decides_unless_the_files_cannot_be_it() {
        let wanted_book_got_video = KindFacts {
            video_files: 1,
            hint: Some(ReleaseKind::Audiobook),
            ..Default::default()
        };
        assert_eq!(classify_kind(&wanted_book_got_video).0, ReleaseKind::Movie);
        let wanted_book_got_audio = KindFacts {
            audio_files: 3,
            audio_seconds: vec![200.0; 3],
            hint: Some(ReleaseKind::Audiobook),
            ..Default::default()
        };
        assert_eq!(
            classify_kind(&wanted_book_got_audio).0,
            ReleaseKind::Audiobook
        );
    }

    #[test]
    fn a_match_in_the_library_outweighs_track_lengths() {
        let short_parts = KindFacts {
            name: "The Series".to_string(),
            audio_files: 8,
            audio_seconds: vec![60.0; 8],
            album_tags: 2,
            in_library: Some(ReleaseKind::Audiobook),
            ..Default::default()
        };
        assert_eq!(classify_kind(&short_parts).0, ReleaseKind::Audiobook);
    }

    #[test]
    fn a_person_saying_what_it_is_decides() {
        let songs_really = KindFacts {
            name: "[Audiobook] Not Really".to_string(),
            audio_files: 8,
            audio_seconds: vec![200.0; 8],
            hint: Some(ReleaseKind::Audiobook),
            chosen: Some(ReleaseKind::Music),
            ..Default::default()
        };
        assert_eq!(classify_kind(&songs_really).0, ReleaseKind::Music);
    }

    #[test]
    fn ebooks_are_recognised() {
        let ebook = KindFacts {
            ebook_files: 3,
            ..Default::default()
        };
        assert_eq!(classify_kind(&ebook).0, ReleaseKind::Ebook);
    }

    #[test]
    fn the_real_info_file_is_read() {
        let text = "This torrent contains the Audiobooks\n\
            *Zoe's Tale [contains two bad files, no replacement]\n\n\
            General Information\n===================\n\
            Title:                  Old Man's War\n\
            Author:                 John Sclazi\n\
            Read By:                William Dufris\n\
            Genre:                  Audiobook\n\
            Series Name:            Old Man's War\n\
            Position in Series:     01\n\n\
            File Information\n================\n\
            Number of MP3s:         20\n\
            Total Duration:         9:54:38\n\n\
            General Information\n===================\n\
            Title:                  The Ghost Brigades\n\
            Number of MP3s:         41\n";
        let facts = read_text_file("info.txt", text);
        assert_eq!(facts.blocks.len(), 2);
        let first = &facts.blocks[0];
        assert_eq!(first.title.as_deref(), Some("Old Man's War"));
        assert_eq!(first.narrator.as_deref(), Some("William Dufris"));
        assert_eq!(first.file_count, Some(20));
        assert_eq!(first.seconds, Some(35678.0));
        assert_eq!(facts.blocks[1].file_count, Some(41));
        assert!(facts.warnings[0].contains("bad files"));
        assert!(text_says_audiobook(&facts));
        assert_eq!(
            block_for_group(&facts.blocks, "The Ghost Brigades", 41)
                .and_then(|block| block.title.as_deref()),
            Some("The Ghost Brigades")
        );
    }

    #[test]
    fn a_cue_sheet_names_its_album() {
        let cue = "PERFORMER \"An Artist\"\nTITLE \"An Album\"\nFILE \"a.flac\" WAVE\n  TRACK 01 AUDIO\n    TITLE \"Song\"\n";
        let facts = read_text_file("album.cue", cue);
        assert_eq!(facts.blocks[0].title.as_deref(), Some("An Album"));
        assert_eq!(facts.blocks[0].author.as_deref(), Some("An Artist"));
    }

    #[test]
    fn nonsense_text_gives_nothing() {
        assert_eq!(
            read_text_file("x.nfo", "\u{fffd}\u{fffd}\n::::\n"),
            TextFacts::default()
        );
    }

    #[test]
    fn cues_come_from_titles() {
        assert_eq!(chapter_cue("Chapter Ten"), Some(ChapterCue::Number(10)));
        assert_eq!(chapter_cue("Chapter 7"), Some(ChapterCue::Number(7)));
        assert_eq!(chapter_cue("Chapter XII"), Some(ChapterCue::Number(12)));
        assert_eq!(
            chapter_cue("Chapter Twenty-One"),
            Some(ChapterCue::Number(21))
        );
        assert_eq!(chapter_cue("12"), Some(ChapterCue::Number(12)));
        assert_eq!(
            chapter_cue("Prologue"),
            Some(ChapterCue::Words(vec!["prologue".to_string()]))
        );
        assert_eq!(
            chapter_cue("Part One"),
            Some(ChapterCue::Words(vec![
                "part".to_string(),
                "one".to_string()
            ]))
        );
    }

    #[test]
    fn credits_and_common_single_words_have_no_cue() {
        assert_eq!(chapter_cue("Opening Credits"), None);
        assert_eq!(chapter_cue("End Credits"), None);
        assert_eq!(chapter_cue("Home"), None);
    }

    #[test]
    fn the_spoken_forms_whisper_really_produced_are_found() {
        for (said, number) in [
            ("CHAPTER III", 3),
            ("Chapter 4", 4),
            ("CHAPTER V", 5),
            ("Chapter Ten", 10),
        ] {
            let heard: Vec<HeardWord> = said
                .split(' ')
                .enumerate()
                .map(|(index, text)| word(text, 100.0 + index as f64))
                .collect();
            assert_eq!(
                find_cue(&heard, &ChapterCue::Number(number), 100.0),
                Some(100.0),
                "{said}"
            );
        }
    }

    #[test]
    fn the_wrong_number_is_not_a_match() {
        let heard = vec![word("Chapter", 5.0), word("Six", 5.5)];
        assert_eq!(find_cue(&heard, &ChapterCue::Number(7), 5.0), None);
    }

    #[test]
    fn a_mention_nearest_where_it_was_expected_wins() {
        let heard = vec![
            word("chapter", 10.0),
            word("two.", 10.3),
            word("chapter", 90.0),
            word("two", 90.3),
        ];
        assert_eq!(find_cue(&heard, &ChapterCue::Number(2), 85.0), Some(90.0));
    }

    #[test]
    fn named_sections_are_found() {
        let heard = vec![word("Prologue.", 3.0), word("It", 4.0)];
        assert_eq!(
            find_cue(&heard, &chapter_cue("Prologue").unwrap(), 0.0),
            Some(3.0)
        );
    }

    #[test]
    fn whisper_word_output_is_read() {
        let output = "\n[00:00:12.340 --> 00:00:12.900]   Chapter\n[00:00:12.900 --> 00:00:13.100]   Ten\n[00:00:13.100 --> 00:00:13.100]   \n";
        assert_eq!(
            parse_whisper_words(output),
            vec![word("Chapter", 12.34), word("Ten", 12.9)]
        );
    }

    #[test]
    fn an_opening_gives_something_to_search_for() {
        let heard = "This is Audible. McMillan audio presents \"The Last Colony\" by John Scalsy, read for you by William DeFries.";
        assert_eq!(
            title_from_opening(heard),
            Some(("The Last Colony".to_string(), "John Scalsy".to_string()))
        );
        assert_eq!(title_from_opening("It was a dark and stormy night."), None);
    }

    #[test]
    fn inserted_audio_moves_what_follows_rather_than_stretching_it() {
        // The Last Colony: a 33 second announcement arrives before Chapter Nine.
        let reference = vec![
            chapter("Opening Credits", 0.0),
            chapter("Chapter Eight", 16787.0),
            chapter("Chapter Nine", 18816.0),
            chapter("Chapter Ten", 20819.0),
        ];
        let heard = vec![None, Some(16786.0), Some(18849.0), None];
        // Scaled for a release a little longer than the listing, as the real one is.
        let starts = settle_chapter_starts(&reference, 1.0024, &heard, 35596.0);
        assert_eq!(starts[0], 0.0);
        assert_eq!(starts[1], 16786.0);
        assert_eq!(starts[2], 18849.0);
        assert_eq!(starts[3], 20852.0, "keeps its distance from Chapter Nine");
    }

    #[test]
    fn a_chapter_not_heard_falls_between_those_that_were() {
        // Chapter Eleven of The Last Colony, which really starts at about 23141.
        let reference = vec![
            chapter("Chapter Ten", 20819.0),
            chapter("Chapter Eleven", 23112.0),
            chapter("Chapter Twelve", 25118.0),
        ];
        let starts = settle_chapter_starts(
            &reference,
            1.0024,
            &[Some(20850.0), None, Some(25149.0)],
            35596.0,
        );
        assert!((starts[1] - 23143.0).abs() < 1.0, "{starts:?}");
    }

    #[test]
    fn a_chapter_starts_after_the_last_real_pause_before_its_announcement() {
        // A sentence ends, the chapter break, then a short gap after "Chapter" that the word's late
        // timing could include.
        let output = "[silencedetect @ 0x1] silence_start: 1.0\n[silencedetect @ 0x1] silence_end: 1.3 | silence_duration: 0.3\n[silencedetect @ 0x1] silence_end: 3.25 | silence_duration: 1.4\n[silencedetect @ 0x1] silence_end: 4.1 | silence_duration: 0.35\n[silencedetect @ 0x1] silence_end: 9.5 | silence_duration: 2.0\n";
        let pauses = parse_pauses(output);
        assert_eq!(pauses.len(), 4);
        assert_eq!(
            pauses[1],
            Pause {
                end: 3.25,
                seconds: 1.4
            }
        );
        assert_eq!(start_of_announcement(3.9, &pauses), Some(3.25));
        assert_eq!(start_of_announcement(0.5, &pauses), None);
        let only_short = [Pause {
            end: 2.0,
            seconds: 0.4,
        }];
        assert_eq!(start_of_announcement(3.0, &only_short), Some(2.0));
        assert_eq!(break_near(4.0, 3.0, &pauses), Some(3.25));
        assert_eq!(break_near(20.0, 3.0, &pauses), None);
    }

    #[test]
    fn starts_never_go_backwards_or_past_the_end() {
        let reference = vec![
            chapter("Chapter 1", 100.0),
            chapter("Chapter 2", 200.0),
            chapter("Chapter 3", 400.0),
        ];
        let starts =
            settle_chapter_starts(&reference, 1.0, &[Some(250.0), Some(150.0), None], 280.0);
        assert_eq!(starts, vec![250.0, 250.0, 280.0]);
    }

    #[test]
    fn a_few_chance_words_are_not_enough() {
        assert!(enough_heard(16, 16));
        assert!(enough_heard(4, 2));
        assert!(!enough_heard(16, 2));
        assert!(!enough_heard(5, 0));
        assert!(!enough_heard(1, 1));
    }

    #[test]
    fn an_opening_names_its_book_despite_misheard_names() {
        // What Whisper really heard.
        let heard = "This is Audible. Macmillan Audio presents The Last Colony by John Scalsy, read for you by William DeFries.";
        assert!(opening_mentions(heard, "The Last Colony"));
        assert!(opening_mentions(heard, "John Scalzi"));
        assert!(!opening_mentions(heard, "Mike Robb"));
    }
}
