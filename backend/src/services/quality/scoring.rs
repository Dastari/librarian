//! Shared release parsing + ranking module.
//!
//! Replaces the phase-A inline scorer (formerly `jobs::scoring`, now deleted —
//! see `docs/tier1-features-plan.md` §2/§5). This module owns:
//! - Fuzzy title similarity (still needed regardless of quality profiles: a
//!   release must be for the right show/movie/album/book before quality even
//!   matters).
//! - `ParsedRelease`: a structured parse of a release title's quality tags
//!   (resolution, codec, HDR, source type, audio, release group).
//! - `is_upgrade`: the Q25 upgrade-comparison rule, shared by auto-download's
//!   duplicate-pending-download handling (Q37) and torrent-import duplicate
//!   handling (Q44).
//!
//! `QualityProfile`-aware filtering (does a parsed release satisfy a user's
//! configured profile?) is a separate concern and lives in
//! `services::quality::profile`, which depends on this module's
//! [`ParsedRelease`] type but not vice versa.

use std::cmp::Ordering;
use std::sync::LazyLock;

use regex::Regex;
use strsim::jaro_winkler;

use crate::services::sources::SourceRelease;

/// Minimum fuzzy title similarity (0.0-1.0) a release must have against the
/// wanted item's search title to be considered at all. Below this we assume
/// the release is for a different show/movie/album/book and must never be
/// auto-grabbed — see tier1-features-plan.md §1 "Risks: without at least a
/// minimal scorer, v1 auto-download grabs garbage."
pub const MIN_TITLE_SIMILARITY: f64 = 0.70;

/// Lowercase, collapse all non-alphanumeric runs to single spaces, and trim.
/// Mirrors the normalization `library_scan.rs`'s matcher uses before
/// `jaro_winkler` (e.g. `Self::normalize_for_match`), kept as a local copy so
/// this module has no dependency on the matcher internals.
pub fn normalize_title(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_was_space = false;
    for ch in input.to_lowercase().chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
            last_was_space = false;
        } else if !last_was_space {
            out.push(' ');
            last_was_space = true;
        }
    }
    out.trim().to_string()
}

/// Fuzzy title similarity in `[0.0, 1.0]` between the wanted item's search
/// title and a candidate release title.
pub fn title_similarity(wanted_title: &str, release_title: &str) -> f64 {
    jaro_winkler(
        &normalize_title(wanted_title),
        &normalize_title(release_title),
    )
}

/// Coarse resolution rank parsed from a release title. Higher is better;
/// `0` means unknown/unparseable. Unknown is *not* rejected outright (many
/// perfectly good releases omit an explicit resolution tag) — it just sorts
/// after every release with a recognized resolution, and per Q25 an unknown
/// *existing* file is always considered upgradeable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct ResolutionRank(pub u8);

/// A structured parse of a release title's quality-relevant tags. Used both
/// pre-download (parsing a `SourceRelease.title` to rank/filter search
/// results) and post-download (parsing a stored file's original filename to
/// supplement ffprobe-verified `MediaFile` fields — see
/// `profile::parsed_release_for_media_file`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedRelease {
    /// Normalized resolution label, e.g. "2160p"/"1080p"/"720p"/"480p".
    pub resolution: Option<String>,
    pub resolution_rank: ResolutionRank,
    /// Normalized video codec, e.g. "hevc"/"h264"/"av1"/"xvid".
    pub codec: Option<String>,
    pub is_hdr: bool,
    /// Normalized HDR type, e.g. "dolby_vision"/"hdr10plus"/"hdr10"/"hdr".
    pub hdr_type: Option<String>,
    /// Normalized source type, e.g. "remux"/"bluray"/"web-dl"/"webrip"/"hdtv"/"dvd"/"cam".
    pub source_type: Option<String>,
    /// Normalized audio format, e.g. "atmos"/"truehd"/"dts-hd"/"dts"/"ddp5.1"/"aac"/"flac".
    pub audio: Option<String>,
    /// Release group parsed off the trailing `-GROUP` token, if present.
    pub release_group: Option<String>,
    /// ISO 639-1 codes for every language tag found in the title, in the order
    /// they appear. Empty when the title carries no recognizable tag (see
    /// [`ParsedRelease::effective_languages`], which then assumes English).
    pub languages: Vec<String>,
    /// `MULTi`/`DUAL`-style tag: the release carries several audio languages
    /// without naming all of them, so it satisfies any language preference.
    pub is_multi_language: bool,
    /// `PROPER` tag (a re-release fixing a broken earlier release).
    pub is_proper: bool,
    /// `REPACK`/`RERIP` tag.
    pub is_repack: bool,
    /// The release covers a whole season (or several) rather than one episode.
    pub is_season_pack: bool,
    /// Season number (`S02`, `Season 2`, `2x05`), when the title carries one.
    pub season: Option<i32>,
    /// Last season of a multi-season pack (`S01-S03` -> `season = 1`,
    /// `season_end = 3`). `None` for single-season releases.
    pub season_end: Option<i32>,
    /// Every episode number the title covers, expanded from ranges
    /// (`S01E01-E04` -> `[1, 2, 3, 4]`). Empty for packs and non-TV releases.
    pub episodes: Vec<i32>,
    /// Release year (`Movie.2020.1080p` -> `2020`). When a title contains
    /// several year-like numbers the *last* plausible one wins, which is what
    /// scene naming implies (`1917.2019.1080p` -> `2019`).
    pub year: Option<i32>,
}

impl ParsedRelease {
    /// The languages this release should be treated as carrying. A title with
    /// no recognizable language tag is treated as English, matching the scene
    /// convention that only non-English releases are tagged.
    pub fn effective_languages(&self) -> Vec<String> {
        if self.languages.is_empty() {
            vec!["en".to_string()]
        } else {
            self.languages.clone()
        }
    }

    /// Whether this release satisfies a `preferred_languages` list. An empty
    /// preference list matches everything, and a `MULTi`/`DUAL` release matches
    /// any preference because it ships several audio tracks.
    pub fn matches_language(&self, preferred: &[String]) -> bool {
        if preferred.is_empty() || self.is_multi_language {
            return true;
        }
        self.effective_languages()
            .iter()
            .any(|lang| preferred.iter().any(|want| want.eq_ignore_ascii_case(lang)))
    }

    /// Index of the best (earliest) matching entry in an ordered
    /// `preferred_languages` list; `None` when nothing matches.
    pub fn language_preference_index(&self, preferred: &[String]) -> Option<usize> {
        if preferred.is_empty() {
            return None;
        }
        if self.is_multi_language {
            return Some(0);
        }
        self.effective_languages()
            .iter()
            .filter_map(|lang| {
                preferred
                    .iter()
                    .position(|want| want.eq_ignore_ascii_case(lang))
            })
            .min()
    }

    /// `PROPER`/`REPACK`/`RERIP`: a corrected re-release of the same content.
    pub fn is_proper_or_repack(&self) -> bool {
        self.is_proper || self.is_repack
    }

    /// How many episodes this release covers. Season packs whose episode list
    /// is unknown report `0` so callers can substitute a real season length.
    pub fn covered_episode_count(&self) -> usize {
        self.episodes.len()
    }
}

fn resolution_rank_and_label(lower: &str) -> (ResolutionRank, Option<String>) {
    if lower.contains("2160p") || lower.contains("4k") || lower.contains("uhd") {
        (ResolutionRank(4), Some("2160p".to_string()))
    } else if lower.contains("1080p") {
        (ResolutionRank(3), Some("1080p".to_string()))
    } else if lower.contains("720p") {
        (ResolutionRank(2), Some("720p".to_string()))
    } else if lower.contains("480p")
        || lower.contains("360p")
        || lower.contains(" sd ")
        || lower.ends_with("sd")
    {
        (ResolutionRank(1), Some("480p".to_string()))
    } else {
        (ResolutionRank(0), None)
    }
}

/// Parse a coarse resolution rank out of a release title's quality tags.
/// 4 = 2160p/4K/UHD, 3 = 1080p, 2 = 720p, 1 = 480p/SD, 0 = unknown.
pub fn parse_resolution_rank(title: &str) -> ResolutionRank {
    resolution_rank_and_label(&title.to_lowercase()).0
}

fn find_token<'a>(lower: &str, tokens: &[(&'a str, &'a str)]) -> Option<&'a str> {
    tokens
        .iter()
        .find(|(needle, _)| lower.contains(needle))
        .map(|(_, label)| *label)
}

fn parse_codec(lower: &str) -> Option<String> {
    const TOKENS: &[(&str, &str)] = &[
        ("x265", "hevc"),
        ("h265", "hevc"),
        ("h.265", "hevc"),
        ("hevc", "hevc"),
        ("x264", "h264"),
        ("h264", "h264"),
        ("h.264", "h264"),
        ("av1", "av1"),
        ("xvid", "xvid"),
    ];
    find_token(lower, TOKENS).map(str::to_string)
}

static DV_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bdv\b").expect("valid regex"));

fn parse_hdr(lower: &str) -> (bool, Option<String>) {
    const TOKENS: &[(&str, &str)] = &[
        ("dovi", "dolby_vision"),
        ("dolby.vision", "dolby_vision"),
        ("dolby vision", "dolby_vision"),
        ("hdr10+", "hdr10plus"),
        ("hdr10plus", "hdr10plus"),
        ("hdr10", "hdr10"),
        ("hlg", "hlg"),
        ("hdr", "hdr"),
    ];
    if let Some(label) = find_token(lower, TOKENS) {
        return (true, Some(label.to_string()));
    }
    // "dv" is a two-letter token (Dolby Vision shorthand) prone to false
    // positives as a plain substring (e.g. inside "dvd"), so it needs a
    // word-boundary regex rather than `str::contains`.
    if DV_TOKEN_RE.is_match(lower) {
        return (true, Some("dolby_vision".to_string()));
    }
    (false, None)
}

fn parse_source_type(lower: &str) -> Option<String> {
    const TOKENS: &[(&str, &str)] = &[
        ("remux", "remux"),
        ("bluray", "bluray"),
        ("blu-ray", "bluray"),
        ("bdrip", "bluray"),
        ("brrip", "bluray"),
        ("webrip", "webrip"),
        ("web-dl", "web-dl"),
        ("web dl", "web-dl"),
        ("webdl", "web-dl"),
        (".web.", "web-dl"),
        ("hdtv", "hdtv"),
        ("dvdrip", "dvd"),
        ("dvdscr", "dvd"),
        ("hdrip", "hdrip"),
        ("vhsrip", "hdrip"),
        ("telesync", "cam"),
        ("telecine", "cam"),
        ("hdts", "cam"),
        ("hdcam", "cam"),
        ("cam", "cam"),
    ];
    find_token(lower, TOKENS).map(str::to_string)
}

fn parse_audio(lower: &str) -> Option<String> {
    const TOKENS: &[(&str, &str)] = &[
        ("atmos", "atmos"),
        ("truehd", "truehd"),
        ("dts-x", "dts-x"),
        ("dtsx", "dts-x"),
        ("dts-hd", "dts-hd"),
        ("dts.hd", "dts-hd"),
        ("dts", "dts"),
        ("ddp5.1", "ddp5.1"),
        ("ddp.5.1", "ddp5.1"),
        ("ddp", "ddp5.1"),
        ("dd5.1", "dd5.1"),
        ("dd.5.1", "dd5.1"),
        ("flac", "flac"),
        ("ac3", "ac3"),
        ("aac", "aac"),
    ];
    find_token(lower, TOKENS).map(str::to_string)
}

static RELEASE_GROUP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)-([a-z0-9]{2,})\s*$").expect("valid regex"));

/// Known media file extensions. Release titles from search results never
/// have one, but filenames parsed post-download (`MediaFile.original_name`)
/// do — strip it before looking for a trailing `-GROUP` token so a
/// non-extension dot-segment (e.g. the "x264" in "...WEB-DL.x264-GROUP")
/// never gets mistaken for a file extension and discarded.
const KNOWN_MEDIA_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v", "ts", "mp3", "flac", "m4a", "aac",
    "ogg", "opus", "wav", "wma",
];

fn parse_release_group(title: &str) -> Option<String> {
    let stem = match title.rsplit_once('.') {
        Some((base, ext))
            if KNOWN_MEDIA_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) =>
        {
            base
        }
        _ => title,
    };
    RELEASE_GROUP_RE
        .captures(stem)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

// ===========================================================================
// Language, edition and season/episode numbering
// ===========================================================================

/// Scene/tracker language tags mapped to ISO 639-1 codes.
///
/// Only whole tokens of three characters or more are matched. Bare two-letter
/// ISO codes are deliberately absent: tokens like `it`, `de`, `es` or `no`
/// collide with ordinary title words far too often ("It", "Up", "No") and a
/// false language tag can hard-reject an otherwise perfect release when a
/// profile sets `require_language_match`.
const LANGUAGE_TOKENS: &[(&str, &str)] = &[
    ("english", "en"),
    ("eng", "en"),
    ("french", "fr"),
    ("francais", "fr"),
    ("truefrench", "fr"),
    ("subfrench", "fr"),
    ("vostfr", "fr"),
    ("vff", "fr"),
    ("vfq", "fr"),
    ("vfi", "fr"),
    ("fre", "fr"),
    ("fra", "fr"),
    ("german", "de"),
    ("deutsch", "de"),
    ("ger", "de"),
    ("deu", "de"),
    ("spanish", "es"),
    ("espanol", "es"),
    ("castellano", "es"),
    ("latino", "es"),
    ("spa", "es"),
    ("esp", "es"),
    ("italian", "it"),
    ("italiano", "it"),
    ("ita", "it"),
    ("japanese", "ja"),
    ("jpn", "ja"),
    ("korean", "ko"),
    ("kor", "ko"),
    ("chinese", "zh"),
    ("mandarin", "zh"),
    ("cantonese", "zh"),
    ("zho", "zh"),
    ("hindi", "hi"),
    ("hin", "hi"),
    ("russian", "ru"),
    ("rus", "ru"),
    ("portuguese", "pt"),
    ("portugues", "pt"),
    ("brazilian", "pt"),
    ("ptbr", "pt"),
    ("dutch", "nl"),
    ("nederlands", "nl"),
    ("nld", "nl"),
    ("swedish", "sv"),
    ("svenska", "sv"),
    ("swe", "sv"),
    ("norwegian", "no"),
    ("danish", "da"),
    ("finnish", "fi"),
    ("polish", "pl"),
    ("turkish", "tr"),
    ("tur", "tr"),
    ("arabic", "ar"),
    ("hebrew", "he"),
    ("greek", "el"),
    ("czech", "cs"),
    ("hungarian", "hu"),
    ("romanian", "ro"),
    ("thai", "th"),
    ("vietnamese", "vi"),
    ("ukrainian", "uk"),
    ("indonesian", "id"),
    ("catalan", "ca"),
    ("persian", "fa"),
    ("farsi", "fa"),
    ("tamil", "ta"),
    ("telugu", "te"),
    ("malayalam", "ml"),
    ("kannada", "kn"),
    ("marathi", "mr"),
    ("punjabi", "pa"),
    ("bengali", "bn"),
    ("urdu", "ur"),
    ("slovak", "sk"),
    ("slovenian", "sl"),
    ("croatian", "hr"),
    ("serbian", "sr"),
    ("bulgarian", "bg"),
    ("icelandic", "is"),
    ("estonian", "et"),
    ("latvian", "lv"),
    ("lithuanian", "lt"),
];

/// Tokens meaning "several audio languages, unspecified".
const MULTI_LANGUAGE_TOKENS: &[&str] = &["multi", "multilang", "multisub", "dual", "dualaudio"];

/// Lowercase the title, keep alphanumerics and `-` (the range separator in
/// `S01E01-E04`/`S01-S03`), and collapse every other run of characters to a
/// single space. Regex-based numbering detection runs on this form so it does
/// not care whether a release uses dots, underscores or spaces.
fn separator_normalized(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut last_was_space = false;
    for ch in title.to_lowercase().chars() {
        if ch.is_alphanumeric() || ch == '-' {
            out.push(ch);
            last_was_space = false;
        } else if !last_was_space {
            out.push(' ');
            last_was_space = true;
        }
    }
    out.trim().to_string()
}

/// Whole alphanumeric tokens of the lowercased title, used for tag lookups
/// (languages, PROPER/REPACK, COMPLETE) where substring matching would give
/// false positives.
fn tokens(title: &str) -> Vec<String> {
    normalize_title(title)
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// Language codes declared by the title, in order of first appearance and
/// de-duplicated, plus whether a `MULTi`/`DUAL` tag is present.
fn parse_languages(tokens: &[String]) -> (Vec<String>, bool) {
    let mut languages: Vec<String> = Vec::new();
    let mut multi = false;
    for token in tokens {
        if MULTI_LANGUAGE_TOKENS.contains(&token.as_str()) {
            multi = true;
            continue;
        }
        if let Some((_, code)) = LANGUAGE_TOKENS.iter().find(|(tag, _)| tag == token)
            && !languages.iter().any(|existing| existing == code)
        {
            languages.push((*code).to_string());
        }
    }
    (languages, multi)
}

static SEASON_RANGE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bs(\d{1,2})\s*-\s*s(\d{1,2})\b").expect("valid regex"));

/// The `SxxEyy` prefix. The episode block that follows is parsed by
/// [`parse_episode_block`] rather than by a regex: the `regex` crate has no
/// look-around, and telling `S01E01E02` (two episodes) apart from
/// `S01E01-1080p` (one episode and a resolution) needs one character of
/// look-ahead per number.
static SEASON_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bs(\d{1,2})").expect("valid regex"));

static SEASON_WORD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bseasons?\s*(\d{1,2})\b").expect("valid regex"));

static SEASON_ONLY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bs(\d{1,2})\b").expect("valid regex"));

/// The `2x05` numbering style. `1920x1080` cannot match: neither `1920` nor
/// `20` sits on a word boundary followed by `x`.
static ALT_EPISODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(\d{1,2})x(\d{1,3})\b").expect("valid regex"));

static YEAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(19\d{2}|20\d{2})\b").expect("valid regex"));

/// Parse the episode block that follows an `Sxx` token, e.g. `e01`,
/// `e01e02`, `e01-e04`, `e01-04`.
///
/// Returns the episode numbers the block covers, expanding `-` ranges. Parsing
/// stops at the first token that is not an episode, which is what keeps
/// `S01E01-1080p` (resolution) and `S01E05-2HD` (release group) from being
/// read as ranges:
/// - a number immediately followed by another digit, `p` or `i` is a
///   resolution, not an episode;
/// - a number after `-` that is not greater than the previous episode is not a
///   range end;
/// - after the first episode, every further number must be introduced by `e`
///   or `-`.
fn parse_episode_block(rest: &str) -> Vec<i32> {
    let bytes: Vec<char> = rest.chars().collect();
    let mut index = 0usize;
    let mut episodes: Vec<i32> = Vec::new();

    loop {
        while bytes.get(index) == Some(&' ') {
            index += 1;
        }
        let is_range = if bytes.get(index) == Some(&'-') {
            index += 1;
            true
        } else {
            false
        };
        while bytes.get(index) == Some(&' ') {
            index += 1;
        }
        let has_e = if bytes.get(index) == Some(&'e') {
            index += 1;
            true
        } else {
            false
        };
        while bytes.get(index) == Some(&' ') {
            index += 1;
        }

        let digits_start = index;
        while bytes.get(index).is_some_and(|c| c.is_ascii_digit()) {
            index += 1;
        }
        let digits: String = bytes[digits_start..index].iter().collect();

        // Not an episode token at all, or a number too long / immediately
        // followed by a resolution suffix.
        if digits.is_empty()
            || digits.len() > 3
            || matches!(bytes.get(index), Some('p') | Some('i'))
            || (!has_e && !is_range)
        {
            break;
        }

        let Ok(number) = digits.parse::<i32>() else {
            break;
        };

        match (is_range, episodes.last().copied()) {
            (true, Some(previous)) if number > previous => {
                episodes.extend((previous + 1)..=number);
            }
            (true, Some(_)) => {
                // `-` followed by a smaller number is a release group or a
                // year, not a range.
                break;
            }
            _ => {
                if !episodes.contains(&number) {
                    episodes.push(number);
                }
            }
        }
    }

    episodes
}

/// Season/episode numbering + season-pack detection.
///
/// Precedence: multi-season range (`S01-S03`) > `SxxEyy` (incl. multi-episode
/// ranges) > `Season 2` > bare `S02` > `2x05`.
fn parse_numbering(spaced: &str, tokens: &[String]) -> (Option<i32>, Option<i32>, Vec<i32>, bool) {
    let mut season = None;
    let mut season_end = None;
    let mut episodes = Vec::new();

    let season_episode = SEASON_TOKEN_RE.captures_iter(spaced).find_map(|caps| {
        let whole = caps.get(0)?;
        let parsed_episodes = parse_episode_block(&spaced[whole.end()..]);
        if parsed_episodes.is_empty() {
            return None;
        }
        Some((caps.get(1)?.as_str().parse::<i32>().ok(), parsed_episodes))
    });

    if let Some(caps) = SEASON_RANGE_RE.captures(spaced) {
        season = caps.get(1).and_then(|m| m.as_str().parse().ok());
        season_end = caps.get(2).and_then(|m| m.as_str().parse().ok());
    } else if let Some((parsed_season, parsed_episodes)) = season_episode {
        season = parsed_season;
        episodes = parsed_episodes;
    } else if let Some(caps) = SEASON_WORD_RE.captures(spaced) {
        season = caps.get(1).and_then(|m| m.as_str().parse().ok());
    } else if let Some(caps) = SEASON_ONLY_RE.captures(spaced) {
        season = caps.get(1).and_then(|m| m.as_str().parse().ok());
    } else if let Some(caps) = ALT_EPISODE_RE.captures(spaced) {
        season = caps.get(1).and_then(|m| m.as_str().parse().ok());
        if let Some(episode) = caps.get(2).and_then(|m| m.as_str().parse().ok()) {
            episodes.push(episode);
        }
    }

    // A release is a pack when it names a season (or a season range) but no
    // episode, or when it is explicitly a "complete series". "Complete" alone
    // is not enough: plenty of movie releases carry it.
    let has_complete = tokens.iter().any(|t| t == "complete");
    let has_series = tokens.iter().any(|t| t == "series" || t == "seasons");
    let is_season_pack = episodes.is_empty()
        && (season.is_some() || season_end.is_some() || (has_complete && has_series));

    (season, season_end, episodes, is_season_pack)
}

/// The last plausible four-digit year in the title. "Last wins" is what scene
/// naming implies: the title comes first, the year after it
/// (`1917.2019.1080p` -> 2019, `Blade.Runner.2049.2017.1080p` -> 2017). Years
/// more than two years in the future are ignored so a futuristic *title*
/// number is not mistaken for a release year.
fn parse_year(spaced: &str) -> Option<i32> {
    use chrono::Datelike;
    let max_year = chrono::Utc::now().year() + 2;
    YEAR_RE
        .captures_iter(spaced)
        .filter_map(|caps| caps.get(1)?.as_str().parse::<i32>().ok())
        .filter(|year| *year >= 1900 && *year <= max_year)
        .last()
}

/// Parse a release title (or filename) into a [`ParsedRelease`].
pub fn parse_release(title: &str) -> ParsedRelease {
    let lower = title.to_lowercase();
    let spaced = separator_normalized(title);
    let tokens = tokens(title);
    let (resolution_rank, resolution) = resolution_rank_and_label(&lower);
    let (is_hdr, hdr_type) = parse_hdr(&lower);
    let (languages, is_multi_language) = parse_languages(&tokens);
    let (season, season_end, episodes, is_season_pack) = parse_numbering(&spaced, &tokens);
    ParsedRelease {
        resolution,
        resolution_rank,
        codec: parse_codec(&lower),
        is_hdr,
        hdr_type,
        source_type: parse_source_type(&lower),
        audio: parse_audio(&lower),
        release_group: parse_release_group(title),
        languages,
        is_multi_language,
        is_proper: tokens.iter().any(|t| t == "proper"),
        is_repack: tokens.iter().any(|t| t == "repack" || t == "rerip"),
        is_season_pack,
        season,
        season_end,
        episodes,
        year: parse_year(&spaced),
    }
}

/// Q25: is `new` an upgrade over `existing`?
///
/// - If `existing` is `None` (nothing known) → always an upgrade.
/// - If `existing`'s resolution is unknown (rank 0) → always an upgrade.
/// - If `new`'s resolution rank is higher → upgrade.
/// - If `new`'s resolution rank is lower → not an upgrade.
/// - If ranks are equal, `new` is an upgrade if it has HDR and `existing` does
///   not, or if it is a PROPER/REPACK of a release that is not (a PROPER fixes
///   a broken encode at the same nominal quality, so it is always preferred).
pub fn is_upgrade(new: &ParsedRelease, existing: Option<&ParsedRelease>) -> bool {
    let Some(existing) = existing else {
        return true;
    };
    if existing.resolution_rank.0 == 0 {
        return true;
    }
    match new.resolution_rank.cmp(&existing.resolution_rank) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => {
            (new.is_hdr && !existing.is_hdr)
                || (new.is_proper_or_repack() && !existing.is_proper_or_repack())
        }
    }
}

/// A release paired with the parsed/scored fields used to rank it.
#[derive(Debug, Clone)]
pub struct ScoredRelease {
    pub release: SourceRelease,
    pub parsed: ParsedRelease,
    pub title_similarity: f64,
}

/// Filter releases below [`MIN_TITLE_SIMILARITY`], then rank survivors by:
/// 1. Higher parsed resolution rank (a 1080p release always beats a 720p one)
/// 2. More seeders (tiebreaker within the same resolution tier)
/// 3. Higher title similarity (final tiebreaker)
///
/// This is the profile-agnostic "given a set of already-acceptable releases,
/// which is best?" ranking. Production code no longer calls it: the hunt uses
/// `profile::filter_and_rank`, which applies the profile's hard filters and
/// then its full preference order (resolution preference, PROPER/REPACK,
/// language, release group, seeders, similarity). Kept, and tested, as the
/// baseline ordering that profile-aware ranking degrades to when a profile
/// expresses no preferences.
#[cfg(test)]
pub fn rank_candidates(wanted_title: &str, releases: Vec<SourceRelease>) -> Vec<ScoredRelease> {
    let mut scored: Vec<ScoredRelease> = releases
        .into_iter()
        .filter_map(|release| {
            let similarity = title_similarity(wanted_title, &release.title);
            if similarity < MIN_TITLE_SIMILARITY {
                return None;
            }
            let parsed = parse_release(&release.title);
            Some(ScoredRelease {
                release,
                parsed,
                title_similarity: similarity,
            })
        })
        .collect();

    scored.sort_by(|a, b| {
        b.parsed
            .resolution_rank
            .cmp(&a.parsed.resolution_rank)
            .then_with(|| {
                let a_seeders = a.release.seeders.unwrap_or(0);
                let b_seeders = b.release.seeders.unwrap_or(0);
                b_seeders.cmp(&a_seeders)
            })
            .then_with(|| {
                b.title_similarity
                    .partial_cmp(&a.title_similarity)
                    .unwrap_or(Ordering::Equal)
            })
            // Final tiebreak so the ordering is total and reproducible across
            // runs even when two releases are otherwise indistinguishable.
            .then_with(|| a.release.title.cmp(&b.release.title))
    });

    scored
}

/// Convenience wrapper around [`rank_candidates`] returning just the winner, if any.
#[cfg(test)]
pub fn pick_best(wanted_title: &str, releases: Vec<SourceRelease>) -> Option<SourceRelease> {
    rank_candidates(wanted_title, releases)
        .into_iter()
        .next()
        .map(|scored| scored.release)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn release(title: &str, seeders: Option<i32>) -> SourceRelease {
        SourceRelease {
            seeders,
            ..SourceRelease::new(title.to_string(), format!("guid-{title}"), Utc::now())
        }
    }

    #[test]
    fn title_similarity_rejects_below_threshold() {
        let sim = title_similarity("Breaking Bad S01E05", "The Simpsons S01E05");
        assert!(
            sim < MIN_TITLE_SIMILARITY,
            "expected unrelated titles to score below threshold, got {sim}"
        );
    }

    #[test]
    fn title_similarity_accepts_close_match() {
        let sim = title_similarity(
            "Breaking Bad S01E05",
            "Breaking.Bad.S01E05.1080p.WEB.x264-GROUP",
        );
        assert!(
            sim >= MIN_TITLE_SIMILARITY,
            "expected close title match to score at/above threshold, got {sim}"
        );
    }

    #[test]
    fn rank_candidates_filters_out_low_similarity() {
        let releases = vec![
            release("Breaking Bad S01E05 1080p WEB x264-GROUP", Some(50)),
            release("Completely Unrelated Movie 2020 2160p", Some(500)),
        ];
        let ranked = rank_candidates("Breaking Bad S01E05", releases);
        assert_eq!(ranked.len(), 1);
        assert!(ranked[0].release.title.contains("Breaking Bad"));
    }

    #[test]
    fn rank_candidates_prefers_higher_resolution_over_more_seeders() {
        let releases = vec![
            release("Breaking Bad S01E05 720p WEB x264-GROUP", Some(1000)),
            release("Breaking Bad S01E05 1080p WEB x264-GROUP", Some(10)),
        ];
        let best = pick_best("Breaking Bad S01E05", releases).expect("should pick a release");
        assert!(
            best.title.contains("1080p"),
            "expected the 1080p release to win despite fewer seeders, got '{}'",
            best.title
        );
    }

    #[test]
    fn rank_candidates_breaks_resolution_ties_with_seeders() {
        let releases = vec![
            release("Breaking Bad S01E05 1080p WEB x264-LOWSEED", Some(5)),
            release("Breaking Bad S01E05 1080p WEB x264-HIGHSEED", Some(500)),
        ];
        let best = pick_best("Breaking Bad S01E05", releases).expect("should pick a release");
        assert!(
            best.title.contains("HIGHSEED"),
            "expected the higher-seeder release to win the resolution tie, got '{}'",
            best.title
        );
    }

    #[test]
    fn parse_resolution_rank_orders_known_tags() {
        assert!(parse_resolution_rank("Show.2160p.WEB") > parse_resolution_rank("Show.1080p.WEB"));
        assert!(parse_resolution_rank("Show.1080p.WEB") > parse_resolution_rank("Show.720p.WEB"));
        assert!(parse_resolution_rank("Show.720p.WEB") > parse_resolution_rank("Show.480p.WEB"));
        assert!(parse_resolution_rank("Show.480p.WEB") > parse_resolution_rank("Show.WEB"));
    }

    #[test]
    fn pick_best_returns_none_when_nothing_survives_threshold() {
        let releases = vec![release("Totally Different Thing", Some(1000))];
        assert!(pick_best("Breaking Bad S01E05", releases).is_none());
    }

    #[test]
    fn parse_release_extracts_all_fields() {
        let parsed = parse_release("Show.Name.S01E01.2160p.HDR.WEB-DL.DDP5.1.x265-GROUP.mkv");
        assert_eq!(parsed.resolution.as_deref(), Some("2160p"));
        assert_eq!(parsed.resolution_rank, ResolutionRank(4));
        assert_eq!(parsed.codec.as_deref(), Some("hevc"));
        assert!(parsed.is_hdr);
        assert_eq!(parsed.hdr_type.as_deref(), Some("hdr"));
        assert_eq!(parsed.source_type.as_deref(), Some("web-dl"));
        assert_eq!(parsed.audio.as_deref(), Some("ddp5.1"));
        assert_eq!(parsed.release_group.as_deref(), Some("GROUP"));
    }

    #[test]
    fn parse_release_detects_dolby_vision_and_bluray_remux() {
        let parsed = parse_release("Movie.2024.2160p.UHD.BluRay.REMUX.DV.TrueHD.7.1.x265-GROUP");
        assert_eq!(parsed.hdr_type.as_deref(), Some("dolby_vision"));
        assert!(parsed.is_hdr);
        assert_eq!(parsed.source_type.as_deref(), Some("remux"));
        assert_eq!(parsed.audio.as_deref(), Some("truehd"));
    }

    #[test]
    fn parse_release_unknown_when_no_tags_present() {
        let parsed = parse_release("Some.Random.File.Name");
        assert_eq!(parsed.resolution, None);
        assert_eq!(parsed.resolution_rank, ResolutionRank(0));
        assert_eq!(parsed.codec, None);
        assert!(!parsed.is_hdr);
        assert_eq!(parsed.source_type, None);
        assert_eq!(parsed.audio, None);
    }

    // -- language / edition / numbering parsing ---------------------------

    #[test]
    fn parse_release_detects_single_episode_numbering() {
        let parsed = parse_release("Show.Name.S01E05.PROPER.720p.HDTV.x264-GROUP");
        assert_eq!(parsed.season, Some(1));
        assert_eq!(parsed.episodes, vec![5]);
        assert!(!parsed.is_season_pack);
        assert!(parsed.is_proper);
        assert!(!parsed.is_repack);
        assert_eq!(parsed.resolution.as_deref(), Some("720p"));
        assert_eq!(parsed.release_group.as_deref(), Some("GROUP"));
    }

    #[test]
    fn parse_release_detects_repack_and_rerip() {
        assert!(parse_release("Show.S01E01.REPACK.1080p.WEB-DL-GRP").is_repack);
        assert!(parse_release("Show.S01E01.RERIP.1080p.WEB-DL-GRP").is_repack);
        assert!(!parse_release("Show.S01E01.1080p.WEB-DL-GRP").is_repack);
    }

    #[test]
    fn parse_release_expands_multi_episode_ranges() {
        let dash = parse_release("Show.Name.S01E01-E04.1080p.WEB-DL-GRP");
        assert_eq!(dash.season, Some(1));
        assert_eq!(dash.episodes, vec![1, 2, 3, 4]);
        assert!(!dash.is_season_pack);

        let bare_range = parse_release("Show.Name.S01E01-04.1080p.WEB-DL-GRP");
        assert_eq!(bare_range.episodes, vec![1, 2, 3, 4]);

        let concatenated = parse_release("Show.Name.S01E01E02.1080p.WEB-DL-GRP");
        assert_eq!(concatenated.episodes, vec![1, 2]);
    }

    #[test]
    fn parse_release_does_not_read_resolution_as_an_episode_range() {
        let parsed = parse_release("Show.Name.S01E01-1080p.WEB-DL-GRP");
        assert_eq!(parsed.episodes, vec![1]);
    }

    #[test]
    fn parse_release_detects_season_packs() {
        let bare = parse_release("Show.Name.S02.1080p.WEB-DL-GRP");
        assert!(bare.is_season_pack);
        assert_eq!(bare.season, Some(2));
        assert!(bare.episodes.is_empty());

        let worded = parse_release("Show Name Season 2 1080p WEB-DL");
        assert!(worded.is_season_pack);
        assert_eq!(worded.season, Some(2));

        let complete = parse_release("Shogun.2024.S01.COMPLETE.1080p.WEB-DL-GRP");
        assert!(complete.is_season_pack);
        assert_eq!(complete.season, Some(1));
        assert_eq!(complete.year, Some(2024));

        let multi_season = parse_release("Show.Name.S01-S03.1080p.WEB-DL-GRP");
        assert!(multi_season.is_season_pack);
        assert_eq!(multi_season.season, Some(1));
        assert_eq!(multi_season.season_end, Some(3));
    }

    #[test]
    fn parse_release_treats_complete_movies_as_not_season_packs() {
        let parsed = parse_release("Movie.Name.2020.Complete.BluRay.1080p-GRP");
        assert!(!parsed.is_season_pack);
        assert_eq!(parsed.season, None);
    }

    #[test]
    fn parse_release_handles_alternate_episode_numbering() {
        let parsed = parse_release("Show Name 2x05 720p HDTV");
        assert_eq!(parsed.season, Some(2));
        assert_eq!(parsed.episodes, vec![5]);
    }

    #[test]
    fn parse_release_picks_the_release_year_not_the_title_year() {
        assert_eq!(
            parse_release("1917.2019.1080p.BluRay.x264-GRP").year,
            Some(2019)
        );
        assert_eq!(
            parse_release("Blade.Runner.2049.2017.2160p.UHD.BluRay-GRP").year,
            Some(2017)
        );
        assert_eq!(
            parse_release("Se7en.1995.1080p.BluRay.x264-GRP").year,
            Some(1995)
        );
    }

    #[test]
    fn parse_release_does_not_invent_numbering_for_tricky_titles() {
        let seven = parse_release("Se7en.1995.1080p.BluRay.x264-GRP");
        assert_eq!(seven.season, None);
        assert!(seven.episodes.is_empty());
        assert!(!seven.is_season_pack);

        // "24" is the show title, not a year; "S01" is still a season pack.
        let twentyfour = parse_release("24.S01.1080p.WEB-DL-GRP");
        assert_eq!(twentyfour.season, Some(1));
        assert_eq!(twentyfour.year, None);
        assert!(twentyfour.is_season_pack);
    }

    #[test]
    fn parse_release_detects_languages_and_multi_tags() {
        let multi = parse_release("Movie.2020.MULTi.VFF.1080p.BluRay.x264-GRP");
        assert!(multi.is_multi_language);
        assert_eq!(multi.languages, vec!["fr".to_string()]);
        assert_eq!(multi.year, Some(2020));
        assert!(multi.matches_language(&["de".to_string()]));

        let german = parse_release("Movie.2020.German.DL.1080p.BluRay.x264-GRP");
        assert_eq!(german.languages, vec!["de".to_string()]);
        assert!(!german.matches_language(&["fr".to_string()]));

        let dual = parse_release("Show.S01E01.DUAL.1080p.WEB-DL-GRP");
        assert!(dual.is_multi_language);
    }

    #[test]
    fn parse_release_defaults_untagged_titles_to_english() {
        let parsed = parse_release("Show.S01E01.1080p.WEB-DL.x264-GRP");
        assert!(parsed.languages.is_empty());
        assert_eq!(parsed.effective_languages(), vec!["en".to_string()]);
        assert!(parsed.matches_language(&["en".to_string()]));
        assert!(!parsed.matches_language(&["fr".to_string()]));
    }

    #[test]
    fn parse_release_language_preference_index_is_ordered() {
        let parsed = parse_release("Movie.2020.German.Italian.1080p-GRP");
        let preferred = vec!["it".to_string(), "de".to_string()];
        assert_eq!(parsed.language_preference_index(&preferred), Some(0));
    }

    #[test]
    fn is_upgrade_true_for_proper_at_the_same_resolution() {
        let existing = parse_release("Show.S01E01.1080p.WEB-DL.x264-GRP");
        let proper = parse_release("Show.S01E01.PROPER.1080p.WEB-DL.x264-GRP");
        assert!(is_upgrade(&proper, Some(&existing)));
        assert!(!is_upgrade(&existing, Some(&proper)));
    }

    // -- is_upgrade (Q25) --------------------------------------------------

    fn parsed(resolution_rank: u8, is_hdr: bool) -> ParsedRelease {
        ParsedRelease {
            resolution_rank: ResolutionRank(resolution_rank),
            is_hdr,
            ..Default::default()
        }
    }

    #[test]
    fn is_upgrade_true_when_no_existing_quality_known() {
        assert!(is_upgrade(&parsed(3, false), None));
    }

    #[test]
    fn is_upgrade_true_when_existing_resolution_unknown() {
        assert!(is_upgrade(&parsed(3, false), Some(&parsed(0, false))));
    }

    #[test]
    fn is_upgrade_true_when_new_resolution_higher() {
        assert!(is_upgrade(&parsed(4, false), Some(&parsed(3, false))));
    }

    #[test]
    fn is_upgrade_false_when_new_resolution_lower() {
        assert!(!is_upgrade(&parsed(2, false), Some(&parsed(3, false))));
    }

    #[test]
    fn is_upgrade_true_when_same_resolution_and_new_has_hdr() {
        assert!(is_upgrade(&parsed(3, true), Some(&parsed(3, false))));
    }

    #[test]
    fn is_upgrade_false_when_same_resolution_and_neither_has_hdr() {
        assert!(!is_upgrade(&parsed(3, false), Some(&parsed(3, false))));
    }

    #[test]
    fn is_upgrade_false_when_same_resolution_and_existing_already_has_hdr() {
        assert!(!is_upgrade(&parsed(3, false), Some(&parsed(3, true))));
    }
}
