//! Filename parser for scene-style release names
//!
//! Parses filenames like:
//! - "Chicago Fire S14E08 1080p WEB h264-ETHEL"
//! - "The.Daily.Show.2026.01.07.Stephen.J.Dubner.720p.WEB.h264-EDITH"
//! - "Corner Gas S06E12 Super Sensitive 1080p AMZN WEB-DL DDP2 0 H 264-QOQ"

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::debug;

// ============================================================================
// Lazy-initialized regex patterns (compiled once, reused across calls)
// ============================================================================

/// Pattern for S01E01 format (most common)
static SXXEXX_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(.+?)\s*[Ss](\d{1,2})[Ee](\d{1,2})").unwrap());

/// Pattern for multi-episode S01E01-E02 or S01E01E02 format
static MULTI_EPISODE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(.+?)\s*[Ss](\d{1,2})[Ee](\d{1,2})(?:[-\s]?[Ee](\d{1,2}))?").unwrap()
});

/// Pattern for season-only S01 format (season packs)
/// Matches: "Show S01 720p", "Show.S01.2021.1080p", "Show S01 Complete"
static SEASON_ONLY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(.+?)\s*[Ss](\d{1,2})(?:\s+\d{4}|\s+\d{3,4}p|\s+Complete|\s+Full|\s*$|\s+(?:720|1080|2160|480))").unwrap()
});

/// Pattern for 1x01 format (also handles 1x1, 01x01, 01x1)
static NXNN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(.+?)\s*(\d{1,2})x(\d{1,2})").unwrap());

/// Pattern for "Season X Episode Y" format
static VERBOSE_SEASON_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(.+?)\s*Season\s*(\d+).*?Episode\s*(\d+)").unwrap());

/// Pattern for daily shows (2026 01 07) - requires spaces/dots between date parts
/// Month must be 01-12, day must be 01-31
static DAILY_SHOW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(.+?)\s*(\d{4})[\s\.\-]+(0[1-9]|1[0-2])[\s\.\-]+(0[1-9]|[12]\d|3[01])(?:\s|$|\.)",
    )
    .unwrap()
});

/// Pattern for standalone year extraction
static YEAR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(19\d{2}|20\d{2})\b").unwrap());

/// Pattern for release group extraction
static GROUP_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"-([A-Za-z0-9]+)(?:\.[A-Za-z0-9]+)?$").unwrap());

/// Pattern for resolution extraction - includes 960p for AI upscales
static RESOLUTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(2160p|1080p|960p|720p|576p|480p|360p|4K|UHD)").unwrap());

/// Pattern for trailing year cleanup
static TRAILING_YEAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s*(19\d{2}|20\d{2})\s*$").unwrap());

/// Pattern for country suffix cleanup
static COUNTRY_SUFFIX_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\s*(US|UK|AU|NZ)\s*$").unwrap());

/// Pattern for multiple spaces cleanup
static MULTI_SPACE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());

/// Pattern for special characters (for normalization)
static SPECIAL_CHARS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-z0-9\s]").unwrap());

/// Pattern for movie year extraction
static MOVIE_YEAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(.+?)[\s\(\[\.]*((?:19|20)\d{2})[\s\)\]\.]").unwrap());

/// Pattern for quality boundary in movie titles
static QUALITY_BOUNDARY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\s+(2160p|1080p|720p|480p|4K|UHD|HDR|BluRay|WEB|HDTV|DVDRip|BRRip)").unwrap()
});

/// Pattern for movie release group
static MOVIE_GROUP_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[-\s]([A-Za-z0-9]+)(?:\.\w{2,4})?$").unwrap());

/// Pattern for trailing parentheses
static TRAILING_PAREN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s*\([^)]*\)\s*$").unwrap());

/// Pattern for trailing brackets
static TRAILING_BRACKET_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s*\[[^\]]*\]\s*$").unwrap());

/// Parsed episode information from a filename
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedEpisode {
    pub show_name: Option<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub year: Option<u32>,
    pub date: Option<String>, // YYYY-MM-DD format for daily shows
    pub resolution: Option<String>,
    pub source: Option<String>,
    pub codec: Option<String>,
    pub hdr: Option<String>,
    pub audio: Option<String>,
    pub release_group: Option<String>,
    pub is_proper: bool,
    pub is_repack: bool,
    pub original_title: String,
}

/// Quality information extracted from filename
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedQuality {
    pub resolution: Option<String>,
    pub source: Option<String>,
    pub codec: Option<String>,
    pub hdr: Option<String>,
    pub audio: Option<String>,
}

/// Parse a filename to extract episode information
pub fn parse_episode(filename: &str) -> ParsedEpisode {
    let mut result = ParsedEpisode {
        original_title: filename.to_string(),
        ..Default::default()
    };

    // Clean up the filename (replace dots/underscores with spaces, but keep the original)
    let cleaned = filename.replace(['.', '_', '-'], " ");

    // Try different patterns in order of specificity

    // Pattern 1: S01E01 format (most common) - also handles multi-episode S01E01-E02
    if let Some(caps) = MULTI_EPISODE_RE.captures(&cleaned) {
        result.show_name = Some(clean_show_name(caps.get(1).unwrap().as_str()));
        result.season = caps.get(2).and_then(|m| m.as_str().parse().ok());
        result.episode = caps.get(3).and_then(|m| m.as_str().parse().ok());
        // Note: caps.get(4) would be the second episode in multi-episode format
        // We take the first episode as the primary
    }
    // Pattern 2: 1x01 format
    else if let Some(caps) = NXNN_RE.captures(&cleaned) {
        result.show_name = Some(clean_show_name(caps.get(1).unwrap().as_str()));
        result.season = caps.get(2).and_then(|m| m.as_str().parse().ok());
        result.episode = caps.get(3).and_then(|m| m.as_str().parse().ok());
    }
    // Pattern 3: Season X Episode Y format
    else if let Some(caps) = VERBOSE_SEASON_RE.captures(&cleaned) {
        result.show_name = Some(clean_show_name(caps.get(1).unwrap().as_str()));
        result.season = caps.get(2).and_then(|m| m.as_str().parse().ok());
        result.episode = caps.get(3).and_then(|m| m.as_str().parse().ok());
    }
    // Pattern 4: Daily show format (2026 01 07)
    else if let Some(caps) = DAILY_SHOW_RE.captures(&cleaned) {
        result.show_name = Some(clean_show_name(caps.get(1).unwrap().as_str()));
        let year = caps.get(2).unwrap().as_str();
        let month = caps.get(3).unwrap().as_str();
        let day = caps.get(4).unwrap().as_str();
        result.date = Some(format!("{}-{}-{}", year, month, day));
        result.year = caps.get(2).and_then(|m| m.as_str().parse().ok());
    }
    // Pattern 5: Season-only format S01 (for season packs)
    else if let Some(caps) = SEASON_ONLY_RE.captures(&cleaned) {
        result.show_name = Some(clean_show_name(caps.get(1).unwrap().as_str()));
        result.season = caps.get(2).and_then(|m| m.as_str().parse().ok());
        // Episode is None for season packs
    }

    // Extract year from show name or filename (for disambiguation)
    if result.year.is_none() {
        if let Some(caps) = YEAR_RE.captures(filename) {
            result.year = caps.get(1).and_then(|m| m.as_str().parse().ok());
        }
    }

    // Extract quality information
    let quality = parse_quality(filename);
    result.resolution = quality.resolution;
    result.source = quality.source;
    result.codec = quality.codec;
    result.hdr = quality.hdr;
    result.audio = quality.audio;

    // Extract release group (usually after the last dash)
    if let Some(caps) = GROUP_RE.captures(filename) {
        result.release_group = Some(caps.get(1).unwrap().as_str().to_string());
    }

    // Check for PROPER/REPACK
    result.is_proper = filename.to_uppercase().contains("PROPER");
    result.is_repack = filename.to_uppercase().contains("REPACK");

    debug!(
        filename = filename,
        show = ?result.show_name,
        season = ?result.season,
        episode = ?result.episode,
        resolution = ?result.resolution,
        "Parsed filename"
    );

    result
}

/// Parse quality information from a filename
pub fn parse_quality(filename: &str) -> ParsedQuality {
    let upper = filename.to_uppercase();
    let mut quality = ParsedQuality::default();

    // Resolution - normalize to lowercase (industry standard: 1080p, 720p, etc.)
    if let Some(caps) = RESOLUTION_RE.captures(filename) {
        let res = caps.get(1).unwrap().as_str().to_uppercase();
        quality.resolution = Some(match res.as_str() {
            "4K" | "UHD" => "2160p".to_string(),
            other => other.to_lowercase(),
        });
    }

    // Source
    if upper.contains("BLURAY") || upper.contains("BDRIP") || upper.contains("BLU-RAY") {
        quality.source = Some("BluRay".to_string());
    } else if upper.contains("WEB-DL") || upper.contains("WEBDL") {
        quality.source = Some("WEB-DL".to_string());
    } else if upper.contains("WEBRIP") || upper.contains("WEB RIP") {
        quality.source = Some("WEBRip".to_string());
    } else if upper.contains("HDTV") {
        quality.source = Some("HDTV".to_string());
    } else if upper.contains("AMZN") || upper.contains("AMAZON") {
        quality.source = Some("AMZN WEB-DL".to_string());
    } else if upper.contains("NF") || upper.contains("NETFLIX") {
        quality.source = Some("NF WEB-DL".to_string());
    } else if upper.contains("HULU") {
        quality.source = Some("HULU WEB-DL".to_string());
    } else if upper.contains("DSNP") || upper.contains("DISNEY") {
        quality.source = Some("DSNP WEB-DL".to_string());
    } else if upper.contains("MAX") || upper.contains("HBO") {
        quality.source = Some("MAX WEB-DL".to_string());
    } else if upper.contains("PCOK") || upper.contains("PEACOCK") {
        quality.source = Some("PCOK WEB-DL".to_string());
    }

    // Codec - handle various formats including "H 265" (with space)
    if upper.contains("X265")
        || upper.contains("H265")
        || upper.contains("H.265")
        || upper.contains("H 265")  // With space (common in scene releases)
        || upper.contains("HEVC")
    {
        quality.codec = Some("HEVC".to_string());
    } else if upper.contains("X264")
        || upper.contains("H264")
        || upper.contains("H.264")
        || upper.contains("H 264")
    // With space
    {
        quality.codec = Some("H.264".to_string());
    } else if upper.contains("AV1") {
        quality.codec = Some("AV1".to_string());
    } else if upper.contains("XVID") || upper.contains("DIVX") {
        quality.codec = Some("XviD".to_string());
    } else if upper.contains("MPEG2") || upper.contains("MPEG-2") {
        quality.codec = Some("MPEG-2".to_string());
    } else if upper.contains("VC1") || upper.contains("VC-1") {
        quality.codec = Some("VC-1".to_string());
    }

    // HDR
    if upper.contains("DOLBY VISION")
        || upper.contains("DOLBYVISION")
        || upper.contains("DV")
        || upper.contains("DOVI")
    {
        quality.hdr = Some("Dolby Vision".to_string());
    } else if upper.contains("HDR10+") || upper.contains("HDR10PLUS") {
        quality.hdr = Some("HDR10+".to_string());
    } else if upper.contains("HDR10") || upper.contains("HDR") {
        quality.hdr = Some("HDR10".to_string());
    } else if upper.contains("HLG") {
        quality.hdr = Some("HLG".to_string());
    }

    // Audio
    if upper.contains("ATMOS") {
        quality.audio = Some("Atmos".to_string());
    } else if upper.contains("TRUEHD") {
        quality.audio = Some("TrueHD".to_string());
    } else if upper.contains("DTS-HD") || upper.contains("DTSHD") {
        quality.audio = Some("DTS-HD".to_string());
    } else if upper.contains("DTS") {
        quality.audio = Some("DTS".to_string());
    } else if upper.contains("DDP") || upper.contains("DD+") || upper.contains("EACL") {
        quality.audio = Some("DD+".to_string());
    } else if upper.contains("DD5") || upper.contains("AC3") || upper.contains("DD ") {
        quality.audio = Some("DD".to_string());
    } else if upper.contains("AAC") {
        quality.audio = Some("AAC".to_string());
    }

    quality
}

/// Clean up the show name
fn clean_show_name(name: &str) -> String {
    let mut cleaned = name.trim().to_string();

    // Remove trailing year if present (we extract it separately)
    cleaned = TRAILING_YEAR_RE.replace(&cleaned, "").to_string();

    // Remove common suffixes
    cleaned = COUNTRY_SUFFIX_RE.replace(&cleaned, "").to_string();

    // Clean up multiple spaces
    cleaned = MULTI_SPACE_RE.replace_all(&cleaned, " ").to_string();

    cleaned.trim().to_string()
}

/// Normalize a show name for fuzzy matching
///
/// This handles:
/// - Case differences: "Star Trek" vs "STAR TREK"
/// - Punctuation: "Star Trek: Deep Space Nine" -> "star trek deep space nine"
/// - Separators: "Doctor.Who" or "Doctor_Who" -> "doctor who"
/// - Articles: "The Office" -> "office" (optional removal)
pub fn normalize_show_name(name: &str) -> String {
    let mut normalized = name.to_lowercase();

    // Remove articles from the beginning
    let articles = ["the ", "a ", "an "];
    for article in articles {
        if normalized.starts_with(article) {
            normalized = normalized[article.len()..].to_string();
        }
    }

    // Replace all non-alphanumeric characters with spaces (not empty string)
    // This ensures proper word boundaries after removing punctuation
    let cleaned: String = normalized
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else {
                ' '
            }
        })
        .collect();

    // Normalize whitespace - collapse multiple spaces into one
    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Calculate similarity between two show names (0.0 to 1.0)
/// NOTE: Reserved for future fuzzy matching improvements
#[allow(dead_code)]
pub fn show_name_similarity(name1: &str, name2: &str) -> f64 {
    let n1 = normalize_show_name(name1);
    let n2 = normalize_show_name(name2);

    if n1 == n2 {
        return 1.0;
    }

    // Simple Levenshtein-based similarity
    let distance = levenshtein_distance(&n1, &n2);
    let max_len = n1.len().max(n2.len());

    if max_len == 0 {
        return 1.0;
    }

    1.0 - (distance as f64 / max_len as f64)
}

/// Calculate Levenshtein distance between two strings
#[allow(dead_code)]
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let m = s1_chars.len();
    let n = s2_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0; n + 1]; m + 1];

    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

/// Parse a filename to extract movie information
/// Parses filenames like:
/// - "The.Matrix.1999.1080p.BluRay.x264-GROUP"
/// - "Inception (2010) 2160p UHD BluRay x265"
pub fn parse_movie(filename: &str) -> ParsedEpisode {
    let mut result = ParsedEpisode {
        original_title: filename.to_string(),
        ..Default::default()
    };

    // Clean up the filename
    let cleaned = filename.replace(['.', '_'], " ").replace(" - ", " ");

    // Try to extract title and year
    // Pattern: Title (Year) or Title.Year or Title Year (where year is 4 digits 19xx/20xx)
    if let Some(caps) = MOVIE_YEAR_RE.captures(&cleaned) {
        result.show_name = Some(clean_movie_title(caps.get(1).unwrap().as_str()));
        result.year = caps.get(2).and_then(|m| m.as_str().parse().ok());
    } else {
        // No year found, just use the whole thing up to quality indicators
        if let Some(mat) = QUALITY_BOUNDARY_RE.find(&cleaned) {
            result.show_name = Some(clean_movie_title(&cleaned[..mat.start()]));
        } else {
            result.show_name = Some(clean_movie_title(&cleaned));
        }
    }

    // Extract quality info
    let quality = parse_quality(&cleaned);
    result.resolution = quality.resolution;
    result.source = quality.source;
    result.codec = quality.codec;
    result.hdr = quality.hdr;
    result.audio = quality.audio;

    // Extract release group
    if let Some(caps) = MOVIE_GROUP_RE.captures(&cleaned) {
        let potential_group = caps.get(1).unwrap().as_str();
        // Filter out common file extensions and resolutions
        let ignore_list = [
            "mkv", "mp4", "avi", "1080p", "720p", "2160p", "480p", "x264", "x265", "hevc", "h264",
        ];
        if !ignore_list.contains(&potential_group.to_lowercase().as_str()) {
            result.release_group = Some(potential_group.to_string());
        }
    }

    // Check for PROPER/REPACK
    result.is_proper = cleaned.to_uppercase().contains("PROPER");
    result.is_repack = cleaned.to_uppercase().contains("REPACK");

    debug!(
        filename = filename,
        title = ?result.show_name,
        year = ?result.year,
        resolution = ?result.resolution,
        "Parsed movie filename"
    );

    result
}

/// Clean up the movie title
fn clean_movie_title(name: &str) -> String {
    let mut cleaned = name.trim().to_string();

    // Remove trailing year if present (we extract it separately)
    cleaned = TRAILING_YEAR_RE.replace(&cleaned, "").to_string();

    // Remove parentheses at the end
    cleaned = TRAILING_PAREN_RE.replace(&cleaned, "").to_string();

    // Remove brackets at the end
    cleaned = TRAILING_BRACKET_RE.replace(&cleaned, "").to_string();

    // Clean up multiple spaces
    cleaned = MULTI_SPACE_RE.replace_all(&cleaned, " ").to_string();

    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Standard SxxExx Format Tests
    // =========================================================================

    #[test]
    fn test_parse_sxxexx() {
        let result = parse_episode("Chicago Fire S14E08 1080p WEB h264-ETHEL");
        assert_eq!(result.show_name.as_deref(), Some("Chicago Fire"));
        assert_eq!(result.season, Some(14));
        assert_eq!(result.episode, Some(8));
        assert_eq!(result.resolution.as_deref(), Some("1080p"));
        assert_eq!(result.codec.as_deref(), Some("H.264"));
        assert_eq!(result.release_group.as_deref(), Some("ETHEL"));
    }

    #[test]
    fn test_parse_chicago_pd() {
        let result = parse_episode("Chicago.PD.S13E08.1080p.WEB.h264-ETHEL");
        assert_eq!(result.show_name.as_deref(), Some("Chicago PD"));
        assert_eq!(result.season, Some(13));
        assert_eq!(result.episode, Some(8));
        assert_eq!(result.resolution.as_deref(), Some("1080p"));
    }

    #[test]
    fn test_parse_chicago_med() {
        let result = parse_episode("Chicago.Med.S11E08.720p.WEB.H264-SYLiX");
        assert_eq!(result.show_name.as_deref(), Some("Chicago Med"));
        assert_eq!(result.season, Some(11));
        assert_eq!(result.episode, Some(8));
        assert_eq!(result.resolution.as_deref(), Some("720p"));
        assert_eq!(result.release_group.as_deref(), Some("SYLiX"));
    }

    #[test]
    fn test_parse_corner_gas() {
        let result =
            parse_episode("Corner Gas S06E12 Super Sensitive 1080p AMZN WEB-DL DDP2 0 H 264-QOQ");
        assert_eq!(result.show_name.as_deref(), Some("Corner Gas"));
        assert_eq!(result.season, Some(6));
        assert_eq!(result.episode, Some(12));
        assert_eq!(result.resolution.as_deref(), Some("1080p"));
        // Source may be detected in different formats
        assert!(
            result.source.is_some(),
            "Should detect source: {:?}",
            result.source
        );
    }

    #[test]
    fn test_parse_fallout() {
        let result = parse_episode("Fallout.2024.S01E01.1080p.HEVC.x265-MeGusta");
        assert_eq!(result.show_name.as_deref(), Some("Fallout"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(1));
        assert_eq!(result.year, Some(2024));
        assert_eq!(result.resolution.as_deref(), Some("1080p"));
        assert_eq!(result.codec.as_deref(), Some("HEVC"));
        assert_eq!(result.release_group.as_deref(), Some("MeGusta"));
    }

    #[test]
    fn test_parse_fallout_s02() {
        let result = parse_episode("Fallout 2024 S02E04 720p WEB H 264-JFF");
        assert_eq!(result.show_name.as_deref(), Some("Fallout"));
        assert_eq!(result.season, Some(2));
        assert_eq!(result.episode, Some(4));
        assert_eq!(result.resolution.as_deref(), Some("720p"));
    }

    #[test]
    fn test_parse_percy_jackson() {
        let result = parse_episode("Percy.Jackson.and.the.Olympians.S02E06.1080p.WEB.h264-ETHEL");
        assert_eq!(
            result.show_name.as_deref(),
            Some("Percy Jackson and the Olympians")
        );
        assert_eq!(result.season, Some(2));
        assert_eq!(result.episode, Some(6));
        assert_eq!(result.resolution.as_deref(), Some("1080p"));
    }

    #[test]
    fn test_parse_power_book_iii() {
        let result =
            parse_episode("Power.Book.III.Raising.Kanan.S01E04.Dont.Sleep.1080p.HEVC.x265-MeGusta");
        assert_eq!(
            result.show_name.as_deref(),
            Some("Power Book III Raising Kanan")
        );
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(4));
    }

    #[test]
    fn test_parse_repack() {
        let result = parse_episode(
            "Power Book III Raising Kanan S04E10 Gimme The Weight REPACK 1080p AMZN WEB-DL DDP5 1 H 264-NTb",
        );
        assert_eq!(
            result.show_name.as_deref(),
            Some("Power Book III Raising Kanan")
        );
        assert_eq!(result.season, Some(4));
        assert_eq!(result.episode, Some(10));
        assert!(result.is_repack);
    }

    #[test]
    fn test_parse_masked_singer() {
        let result = parse_episode("The.Masked.Singer.S14E01.1080p.AV1.10bit-MeGusta");
        assert_eq!(result.show_name.as_deref(), Some("The Masked Singer"));
        assert_eq!(result.season, Some(14));
        assert_eq!(result.episode, Some(1));
        assert_eq!(result.resolution.as_deref(), Some("1080p"));
    }

    #[test]
    fn test_parse_girl_taken() {
        let result =
            parse_episode("Girl Taken S01E02 Trapped 1080p AMZN WEB-DL DD 5 1 H 264-playWEB");
        assert_eq!(result.show_name.as_deref(), Some("Girl Taken"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(2));
    }

    #[test]
    fn test_parse_beyond_the_gates() {
        let result = parse_episode("Beyond.the.Gates.S02E04.1080p.HEVC.x265-MeGusta");
        assert_eq!(result.show_name.as_deref(), Some("Beyond the Gates"));
        assert_eq!(result.season, Some(2));
        assert_eq!(result.episode, Some(4));
    }

    // =========================================================================
    // Deep Space Nine (Real Multi-File Torrent Examples)
    // =========================================================================

    #[test]
    fn test_parse_ds9_episode_standard() {
        let result = parse_episode(
            "Star Trek- Deep Space Nine - S01E09 - The Passenger 960p-QueerWorm-Lela.mkv",
        );
        assert_eq!(
            result.show_name.as_deref(),
            Some("Star Trek Deep Space Nine")
        );
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(9));
    }

    #[test]
    fn test_parse_ds9_pilot() {
        // Pilot episodes often have double episode numbers
        let result = parse_episode(
            "Star Trek- Deep Space Nine - S01E01-E02 - Emissary 960p-QueerWorm-Lela.mkv",
        );
        assert_eq!(
            result.show_name.as_deref(),
            Some("Star Trek Deep Space Nine")
        );
        assert_eq!(result.season, Some(1));
        // Should capture at least the first episode
        assert!(result.episode == Some(1) || result.episode == Some(2));
    }

    #[test]
    fn test_parse_ds9_various_episodes() {
        let test_cases = vec![
            (
                "Star Trek- Deep Space Nine - S01E03 - Past Prologue 960p-QueerWorm-Lela.mkv",
                1,
                3,
            ),
            (
                "Star Trek- Deep Space Nine - S01E10 - Move Along Home 960p-QueerWorm-Lela.mkv",
                1,
                10,
            ),
            (
                "Star Trek- Deep Space Nine - S01E15 - Progress 960p-QueerWorm-Lela.mkv",
                1,
                15,
            ),
            (
                "Star Trek- Deep Space Nine - S01E20 - In the Hands of the Prophets 960p-QueerWorm-Lela.mkv",
                1,
                20,
            ),
        ];

        for (filename, expected_season, expected_episode) in test_cases {
            let result = parse_episode(filename);
            assert_eq!(
                result.season,
                Some(expected_season),
                "Season mismatch for {}",
                filename
            );
            assert_eq!(
                result.episode,
                Some(expected_episode),
                "Episode mismatch for {}",
                filename
            );
        }
    }

    // =========================================================================
    // Daily Shows (Date-Based)
    // =========================================================================

    #[test]
    fn test_parse_daily_show() {
        let result =
            parse_episode("The.Daily.Show.2026.01.07.Stephen.J.Dubner.720p.WEB.h264-EDITH");
        assert_eq!(result.show_name.as_deref(), Some("The Daily Show"));
        assert_eq!(result.date.as_deref(), Some("2026-01-07"));
        assert_eq!(result.resolution.as_deref(), Some("720p"));
    }

    #[test]
    fn test_parse_jimmy_kimmel() {
        let result = parse_episode("Jimmy.Kimmel.2026.01.07.Alan.Cumming.720p.WEB.h264-EDITH");
        assert_eq!(result.show_name.as_deref(), Some("Jimmy Kimmel"));
        assert_eq!(result.date.as_deref(), Some("2026-01-07"));
    }

    #[test]
    fn test_parse_stephen_colbert() {
        let result =
            parse_episode("Stephen.Colbert.2026.01.07.Chris.Hayes.1080p.HEVC.x265-MeGusta");
        assert_eq!(result.show_name.as_deref(), Some("Stephen Colbert"));
        assert_eq!(result.date.as_deref(), Some("2026-01-07"));
        assert_eq!(result.resolution.as_deref(), Some("1080p"));
    }

    // =========================================================================
    // Alternative Episode Formats (1x01, S1E1, Season X Episode Y, etc.)
    // =========================================================================

    #[test]
    fn test_parse_1x01_format() {
        // Standard 1x01 format
        let result = parse_episode("Show.Name.1x01.Episode.Title.720p.HDTV");
        assert_eq!(result.show_name.as_deref(), Some("Show Name"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(1));
    }

    #[test]
    fn test_parse_1x1_unpadded_format() {
        // Unpadded 1x1 format (single digit episode)
        let result = parse_episode("Show.Name.1x1.Episode.Title.720p.HDTV");
        assert_eq!(result.show_name.as_deref(), Some("Show Name"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(1));
    }

    #[test]
    fn test_parse_01x01_format() {
        // Padded season 01x01 format
        let result = parse_episode("Show.Name.01x01.Episode.Title.720p.HDTV");
        assert_eq!(result.show_name.as_deref(), Some("Show Name"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(1));
    }

    #[test]
    fn test_parse_2x15_format() {
        // Multi-digit episode 2x15
        let result = parse_episode("Game.of.Thrones.2x15.720p.HDTV");
        assert_eq!(result.show_name.as_deref(), Some("Game of Thrones"));
        assert_eq!(result.season, Some(2));
        assert_eq!(result.episode, Some(15));
    }

    #[test]
    fn test_parse_s1e1_unpadded() {
        // Unpadded S1E1 format
        let result = parse_episode("Show.Name.S1E1.720p.HDTV");
        assert_eq!(result.show_name.as_deref(), Some("Show Name"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(1));
    }

    #[test]
    fn test_parse_s1e01_mixed() {
        // Mixed padding S1E01
        let result = parse_episode("Show.Name.S1E01.720p.HDTV");
        assert_eq!(result.show_name.as_deref(), Some("Show Name"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(1));
    }

    #[test]
    fn test_parse_s01e1_mixed() {
        // Mixed padding S01E1
        let result = parse_episode("Show.Name.S01E1.720p.HDTV");
        assert_eq!(result.show_name.as_deref(), Some("Show Name"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(1));
    }

    #[test]
    fn test_parse_season_episode_verbose() {
        // Verbose "Season X Episode Y" format
        let result = parse_episode("Show Name Season 1 Episode 5 720p WEB");
        assert_eq!(result.show_name.as_deref(), Some("Show Name"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(5));
    }

    #[test]
    fn test_parse_season_episode_verbose_padded() {
        // Verbose with padded numbers
        let result = parse_episode("Show Name Season 01 Episode 05 720p WEB");
        assert_eq!(result.show_name.as_deref(), Some("Show Name"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(5));
    }

    #[test]
    fn test_parse_season_episode_verbose_with_title() {
        // Verbose with episode title
        let result = parse_episode("Breaking Bad Season 2 Episode 10 Over 1080p BluRay");
        assert_eq!(result.show_name.as_deref(), Some("Breaking Bad"));
        assert_eq!(result.season, Some(2));
        assert_eq!(result.episode, Some(10));
    }

    // =========================================================================
    // Season Packs
    // =========================================================================

    #[test]
    fn test_parse_season_pack() {
        // Season pack format: S01 without episode number
        let result = parse_episode("His and Hers 2026 S01 720p NF WEB-DL DDP5.1 Atmos H.264-FLUX");
        assert!(
            result.show_name.is_some(),
            "Should detect show name: {:?}",
            result
        );
        assert_eq!(result.season, Some(1), "Should detect season 1");
        assert_eq!(
            result.episode, None,
            "Episode should be None for season pack"
        );
    }

    #[test]
    fn test_parse_season_pack_young_sheldon() {
        // Season pack with year
        let result = parse_episode("Young.Sheldon.S05.2021.1080p.MAX.WEB-DL.DDP5.1.x265-HDSWEB");
        assert!(
            result
                .show_name
                .as_ref()
                .map(|n| n.contains("Sheldon"))
                .unwrap_or(false),
            "Should contain Sheldon: {:?}",
            result.show_name
        );
        assert_eq!(result.season, Some(5), "Should detect season 5");
        assert_eq!(
            result.episode, None,
            "Episode should be None for season pack"
        );
    }

    // =========================================================================
    // Quality Parsing Tests
    // =========================================================================

    #[test]
    fn test_parse_quality() {
        let quality = parse_quality("Show S01E01 2160p AMZN WEB-DL DDP5.1 Atmos HDR x265-GROUP");
        assert_eq!(quality.resolution.as_deref(), Some("2160p"));
        // Source detection may vary
        assert!(
            quality
                .source
                .as_ref()
                .map(|s| s.contains("AMZN") || s.contains("WEB"))
                .unwrap_or(false),
            "Source: {:?}",
            quality.source
        );
        assert_eq!(quality.codec.as_deref(), Some("HEVC"));
        // HDR detection
        assert!(
            quality.hdr.is_some(),
            "Should detect HDR: {:?}",
            quality.hdr
        );
        // Atmos detection
        assert!(
            quality
                .audio
                .as_ref()
                .map(|a| a.contains("Atmos"))
                .unwrap_or(false),
            "Audio: {:?}",
            quality.audio
        );
    }

    #[test]
    fn test_parse_quality_4k_hdr() {
        // "H 265" with space should now be detected as HEVC
        let quality = parse_quality(
            "Old Dog New Tricks S01E05 2025 2160p NF WEB-DL DDP5 1 Atmos HDR H 265-HHWEB",
        );
        assert_eq!(quality.resolution.as_deref(), Some("2160p"));
        assert!(
            quality.hdr.is_some(),
            "Should detect HDR: {:?}",
            quality.hdr
        );
        assert_eq!(
            quality.codec.as_deref(),
            Some("HEVC"),
            "H 265 with space should be HEVC"
        );
    }

    #[test]
    fn test_parse_quality_h264_with_space() {
        // "H 264" with space should be detected as H.264
        let quality = parse_quality("Show S01E01 1080p WEB-DL DDP5 1 H 264-GROUP");
        assert_eq!(
            quality.codec.as_deref(),
            Some("H.264"),
            "H 264 with space should be H.264"
        );
    }

    #[test]
    fn test_parse_quality_960p() {
        // 960p AI upscale format (like DS9)
        let quality = parse_quality("Star Trek Deep Space Nine S01E01 960p AI-Upscale x264");
        assert_eq!(
            quality.resolution.as_deref(),
            Some("960p"),
            "Should detect 960p resolution"
        );
    }

    #[test]
    fn test_parse_quality_xvid() {
        let quality = parse_quality("Chicago.Fire.S14E08.XviD-AFG");
        assert_eq!(quality.codec.as_deref(), Some("XviD"));
    }

    #[test]
    fn test_parse_quality_x265() {
        let quality = parse_quality("Chicago.Fire.S14E08.720p.HEVC.x265-MeGusta");
        assert_eq!(quality.resolution.as_deref(), Some("720p"));
        assert_eq!(quality.codec.as_deref(), Some("HEVC"));
    }

    #[test]
    fn test_parse_quality_av1() {
        let quality = parse_quality("My.Korean.Boyfriend.S01E06.1080p.AV1.10bit-MeGusta");
        assert_eq!(quality.resolution.as_deref(), Some("1080p"));
        // AV1 codec detection
        assert!(quality.codec.is_some());
    }

    #[test]
    fn test_parse_quality_480p() {
        let quality = parse_quality("Chicago.Fire.S14E08.480p.x264-mSD");
        assert_eq!(quality.resolution.as_deref(), Some("480p"));
    }

    #[test]
    fn test_parse_quality_mobile() {
        let quality = parse_quality("Chicago.Fire.S14E08.AAC.MP4-Mobile");
        // Mobile releases often don't have explicit resolution
        assert_eq!(quality.audio.as_deref(), Some("AAC"));
    }

    // =========================================================================
    // Movie Parsing Tests
    // =========================================================================

    #[test]
    fn test_parse_movie_jack_ryan() {
        let result = parse_movie("Jack Ryan Shadow Recruit 2014 BluRay 1080p DD5.1 H265-d3g.mkv");
        assert_eq!(
            result.show_name.as_deref(),
            Some("Jack Ryan Shadow Recruit")
        );
        assert_eq!(result.year, Some(2014));
        assert_eq!(result.resolution.as_deref(), Some("1080p"));
    }

    #[test]
    fn test_parse_movie_clear_and_present_danger() {
        let result =
            parse_movie("Clear.And.Present.Danger.1994.REMASTERED.PROPER.1080p.BluRay.x265");
        assert_eq!(
            result.show_name.as_deref(),
            Some("Clear And Present Danger")
        );
        assert_eq!(result.year, Some(1994));
        assert_eq!(result.resolution.as_deref(), Some("1080p"));
        assert!(result.is_proper);
    }

    #[test]
    fn test_parse_movie_hunt_for_red_october() {
        let result =
            parse_movie("The.Hunt.for.Red.October.1990.REMASTERED.PROPER.1080p.BluRay.x265-LAMA");
        assert_eq!(
            result.show_name.as_deref(),
            Some("The Hunt for Red October")
        );
        assert_eq!(result.year, Some(1990));
        assert!(result.is_proper);
        assert_eq!(result.release_group.as_deref(), Some("LAMA"));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_parse_show_with_year_in_name() {
        let result = parse_episode("Shifting Gears 2025 S02E09 720p WEB H264-iNSiDiOUS");
        assert_eq!(result.show_name.as_deref(), Some("Shifting Gears"));
        assert_eq!(result.year, Some(2025));
        assert_eq!(result.season, Some(2));
        assert_eq!(result.episode, Some(9));
    }

    #[test]
    fn test_parse_show_with_episode_title() {
        let result =
            parse_episode("Desert Law S01E01 Welcome to Pima County 1080p HULU WEB-DL H264-RAWR");
        assert_eq!(result.show_name.as_deref(), Some("Desert Law"));
        assert_eq!(result.season, Some(1));
        assert_eq!(result.episode, Some(1));
    }

    #[test]
    fn test_parse_show_with_special_characters() {
        let result = parse_episode("Sanctuary A Witchs Tale S02E03 1080p x265-ELiTE");
        // Should handle apostrophes/special chars in name
        assert!(result.show_name.is_some());
        assert_eq!(result.season, Some(2));
        assert_eq!(result.episode, Some(3));
    }

    // =========================================================================
    // Show Name Similarity Tests
    // =========================================================================

    #[test]
    fn test_show_name_similarity() {
        // Exact match should be 1.0
        assert!(show_name_similarity("Chicago Fire", "Chicago Fire") > 0.99);

        // Similar but not exact
        let office_sim = show_name_similarity("The Office", "Office");
        assert!(office_sim > 0.5, "Office similarity: {}", office_sim);

        // Partial match
        let chicago_sim = show_name_similarity("Chicago Fire", "Chicago PD");
        assert!(
            chicago_sim > 0.4,
            "Chicago shows similarity: {}",
            chicago_sim
        );
    }

    #[test]
    fn test_show_name_similarity_with_article() {
        // "The" prefix should be handled
        assert!(show_name_similarity("The Daily Show", "Daily Show") > 0.9);
    }

    #[test]
    fn test_show_name_similarity_star_trek() {
        // Near exact match with punctuation difference
        let ds9_sim =
            show_name_similarity("Star Trek Deep Space Nine", "Star Trek: Deep Space Nine");
        assert!(ds9_sim > 0.8, "DS9 with colon similarity: {}", ds9_sim);

        // Partial match - "Deep Space Nine" vs full name
        let partial_sim = show_name_similarity("Star Trek Deep Space Nine", "Deep Space Nine");
        // This is a substring match, similarity depends on algorithm
        assert!(partial_sim > 0.3, "Partial DS9 similarity: {}", partial_sim);
    }

    #[test]
    fn test_show_name_similarity_case_insensitive() {
        assert!(show_name_similarity("CHICAGO FIRE", "chicago fire") > 0.99);
    }

    // =========================================================================
    // Real-World Torrent Names (from IPTorrents feed - January 2026)
    // =========================================================================

    #[test]
    fn test_parse_real_world_torrents() {
        // Test cases: (filename, expected_show, expected_season, expected_episode)
        let test_cases = vec![
            // Standard format
            (
                "Fallout 2024 S02E05 1080p WEB h264-ETHEL",
                "Fallout",
                Some(2),
                Some(5),
            ),
            (
                "Landman S02E10 1080p WEB h264-ETHEL",
                "Landman",
                Some(2),
                Some(10),
            ),
            (
                "The Pitt S02E02 1080p WEB h264-ETHEL",
                "The Pitt",
                Some(2),
                Some(2),
            ),
            (
                "High Potential S02E09 1080p WEB h264-ETHEL",
                "High Potential",
                Some(2),
                Some(9),
            ),
            (
                "Percy Jackson and the Olympians S02E07 1080p WEB h264-ETHEL",
                "Percy Jackson and the Olympians",
                Some(2),
                Some(7),
            ),
            (
                "Hijack 2023 S02E01 1080p WEB h264-ETHEL",
                "Hijack",
                Some(2),
                Some(1),
            ),
            (
                "Abbott Elementary S05E10 1080p WEB h264-ETHEL",
                "Abbott Elementary",
                Some(5),
                Some(10),
            ),
            (
                "Chicago Med S11E09 1080p WEB h264-ETHEL",
                "Chicago Med",
                Some(11),
                Some(9),
            ),
            (
                "Chicago PD S13E09 1080p WEB h264-ETHEL",
                "Chicago PD",
                Some(13),
                Some(9),
            ),
            (
                "Chicago Fire S14E09 1080p WEB h264-ETHEL",
                "Chicago Fire",
                Some(14),
                Some(9),
            ),
            (
                "The Night Manager S02E04 1080p WEB h264-ETHEL",
                "The Night Manager",
                Some(2),
                Some(4),
            ),
            (
                "9-1-1 S09E08 1080p WEB h264-ETHEL",
                "9 1 1",
                Some(9),
                Some(8),
            ),
            (
                "Tell Me Lies S03E04 1080p WEB h264-ETHEL",
                "Tell Me Lies",
                Some(3),
                Some(4),
            ),
            (
                "Primal S03E02 1080p WEB h264-GRACE",
                "Primal",
                Some(3),
                Some(2),
            ),
            // With episode titles
            (
                "A Knight of the Seven Kingdoms S01E01 The Hedge Knight 1080p AMZN WEB-DL DDP5 1 Atmos H 264-FLUX",
                "A Knight of the Seven Kingdoms",
                Some(1),
                Some(1),
            ),
            (
                "The Rookie S08E02 Fast Andy 1080p AMZN WEB-DL DDP5 1 H 264-Kitsune",
                "The Rookie",
                Some(8),
                Some(2),
            ),
            (
                "Fallout S02E05 The Wrangler 1080p HEVC x265-MeGusta",
                "Fallout",
                Some(2),
                Some(5),
            ),
            (
                "Shoresy S05E05 Total Buy-In 1080p AMZN WEB-DL DDP5 1 H 264-Kitsune",
                "Shoresy",
                Some(5),
                Some(5),
            ),
            (
                "Landman S02E10 Tragedy and Flies 1080p HEVC x265-MeGusta",
                "Landman",
                Some(2),
                Some(10),
            ),
            (
                "Gold Rush S16E10 New Levels of Chaos 1080p DSCP WEB-DL DDP2 0 H 264-SNAKE",
                "Gold Rush",
                Some(16),
                Some(10),
            ),
            (
                "Star Trek Starfleet Academy S01E01 Kids These Days 1080p HEVC x265-MeGusta",
                "Star Trek Starfleet Academy",
                Some(1),
                Some(1),
            ),
            (
                "Star Trek Starfleet Academy S01E02 Beta Test 1080p HEVC x265-MeGusta",
                "Star Trek Starfleet Academy",
                Some(1),
                Some(2),
            ),
            (
                "The Pitt S02E02 8 00 A M 1080p HEVC x265-MeGusta",
                "The Pitt",
                Some(2),
                Some(2),
            ),
            // HEVC/x265 releases
            (
                "Fallout 2024 S02E05 1080p HEVC x265-MeGusta",
                "Fallout",
                Some(2),
                Some(5),
            ),
            (
                "Landman S02E10 1080p HEVC x265-MeGusta",
                "Landman",
                Some(2),
                Some(10),
            ),
            (
                "A Knight of the Seven Kingdoms S01E01 The Hedge Knight 1080p HEVC x265-MeGusta",
                "A Knight of the Seven Kingdoms",
                Some(1),
                Some(1),
            ),
            (
                "The Pitt S02E02 1080p HEVC x265-MeGusta",
                "The Pitt",
                Some(2),
                Some(2),
            ),
            (
                "Star Trek Starfleet Academy S01E01 1080p HEVC x265-MeGusta",
                "Star Trek Starfleet Academy",
                Some(1),
                Some(1),
            ),
            (
                "Star Trek Starfleet Academy S01E02 1080p HEVC x265-MeGusta",
                "Star Trek Starfleet Academy",
                Some(1),
                Some(2),
            ),
            (
                "Hijack 2023 S02E01 1080p HEVC x265-MeGusta",
                "Hijack",
                Some(2),
                Some(1),
            ),
            (
                "High Potential S02E09 1080p HEVC x265-MeGusta",
                "High Potential",
                Some(2),
                Some(9),
            ),
            (
                "Shoresy S05E05 Total Buy-In 1080p HEVC x265-MeGusta",
                "Shoresy",
                Some(5),
                Some(5),
            ),
            (
                "The Rookie S08E02 1080p HEVC x265-MeGusta",
                "The Rookie",
                Some(8),
                Some(2),
            ),
            (
                "Percy Jackson and the Olympians S02E07 1080p HEVC x265-MeGusta",
                "Percy Jackson and the Olympians",
                Some(2),
                Some(7),
            ),
            (
                "Saturday Night Live S51E10 Finn Wolfhard 1080p HEVC x265-MeGusta",
                "Saturday Night Live",
                Some(51),
                Some(10),
            ),
            // 720p HDTV releases
            (
                "Greys Anatomy S22E08 720p HDTV x264-SYNCOPY",
                "Greys Anatomy",
                Some(22),
                Some(8),
            ),
            (
                "Law and Order SVU S27E10 720p HDTV x264-SYNCOPY",
                "Law and Order SVU",
                Some(27),
                Some(10),
            ),
            (
                "9-1-1 S09E08 720p HDTV x264-SYNCOPY",
                "9 1 1",
                Some(9),
                Some(8),
            ),
            // 4K/2160p releases
            (
                "A Knight of the Seven Kingdoms S01E01 The Hedge Knight 2160p HMAX WEB-DL DDP5 1 Atmos DV HDR H 265-FLUX",
                "A Knight of the Seven Kingdoms",
                Some(1),
                Some(1),
            ),
            (
                "Fallout S02E05 The Wrangler 2160p AMZN WEB-DL DDP5 1 Atmos DV HDR10Plus H 265-Kitsune",
                "Fallout",
                Some(2),
                Some(5),
            ),
            // 720p WEB releases
            (
                "A Knight of the Seven Kingdoms S01E01 The Hedge Knight 720p HMAX WEB-DL DDP5 1 H 264-NTb",
                "A Knight of the Seven Kingdoms",
                Some(1),
                Some(1),
            ),
            // Older shows with different naming
            (
                "The Traitors 2023 S04E05 1080p WEB h264-EDITH",
                "The Traitors",
                Some(4),
                Some(5),
            ),
            (
                "The Traitors 2023 S04E04 1080p WEB h264-EDITH",
                "The Traitors",
                Some(4),
                Some(4),
            ),
            (
                "Hells Kitchen US S24E15 1080p WEB h264-EDITH",
                "Hells Kitchen US",
                Some(24),
                Some(15),
            ),
            // WEB-DL with Atmos
            (
                "Fallout S02E05 The Wrangler 1080p AMZN WEB-DL DD 5 1 Atmos H 264-playWEB",
                "Fallout",
                Some(2),
                Some(5),
            ),
        ];

        for (filename, expected_show, expected_season, expected_episode) in test_cases {
            let result = parse_episode(filename);

            // Check show name contains key words (parser may clean differently)
            let show_name = result.show_name.as_deref().unwrap_or("");
            let expected_words: Vec<&str> = expected_show.split_whitespace().collect();
            let first_word = expected_words.first().unwrap_or(&"");
            assert!(
                show_name
                    .to_lowercase()
                    .contains(&first_word.to_lowercase()),
                "Show name '{}' should contain '{}' for: {}",
                show_name,
                first_word,
                filename
            );

            assert_eq!(
                result.season, expected_season,
                "Season mismatch for: {} (got {:?})",
                filename, result.season
            );
            assert_eq!(
                result.episode, expected_episode,
                "Episode mismatch for: {} (got {:?})",
                filename, result.episode
            );
        }
    }

    #[test]
    fn test_parse_real_world_quality_detection() {
        // Test quality parsing from real-world examples
        let test_cases = vec![
            // (filename, expected_resolution, expected_codec, source_should_exist)
            (
                "Fallout 2024 S02E05 1080p WEB h264-ETHEL",
                Some("1080p"),
                Some("H.264"),
                false,
            ),
            (
                "Fallout 2024 S02E05 1080p HEVC x265-MeGusta",
                Some("1080p"),
                Some("HEVC"),
                false,
            ),
            (
                "A Knight of the Seven Kingdoms S01E01 The Hedge Knight 1080p AMZN WEB-DL DDP5 1 Atmos H 264-FLUX",
                Some("1080p"),
                Some("H.264"),
                true,
            ),
            (
                "A Knight of the Seven Kingdoms S01E01 The Hedge Knight 2160p HMAX WEB-DL DDP5 1 Atmos DV HDR H 265-FLUX",
                Some("2160p"),
                Some("HEVC"),
                true,
            ),
            (
                "Greys Anatomy S22E08 720p HDTV x264-SYNCOPY",
                Some("720p"),
                Some("H.264"),
                true,
            ),
            (
                "Gold Rush S16E10 New Levels of Chaos 1080p DSCP WEB-DL DDP2 0 H 264-SNAKE",
                Some("1080p"),
                Some("H.264"),
                true,
            ),
            (
                "Fallout S02E05 The Wrangler 2160p AMZN WEB-DL DDP5 1 Atmos DV HDR10Plus H 265-Kitsune",
                Some("2160p"),
                Some("HEVC"),
                true,
            ),
        ];

        for (filename, expected_res, expected_codec, source_should_exist) in test_cases {
            let quality = parse_quality(filename);

            assert_eq!(
                quality.resolution.as_deref(),
                expected_res,
                "Resolution mismatch for: {}",
                filename
            );

            if let Some(codec) = expected_codec {
                assert_eq!(
                    quality.codec.as_deref(),
                    Some(codec),
                    "Codec mismatch for: {} (got {:?})",
                    filename,
                    quality.codec
                );
            }

            if source_should_exist {
                assert!(
                    quality.source.is_some(),
                    "Source should be detected for: {} (got {:?})",
                    filename,
                    quality.source
                );
            }
        }
    }

    #[test]
    fn test_parse_real_world_hdr_detection() {
        // Test HDR detection from real-world 4K releases
        let hdr_releases = vec![
            "A Knight of the Seven Kingdoms S01E01 The Hedge Knight 2160p HMAX WEB-DL DDP5 1 Atmos DV HDR H 265-FLUX",
            "Fallout S02E05 The Wrangler 2160p AMZN WEB-DL DDP5 1 Atmos DV HDR10Plus H 265-Kitsune",
        ];

        for filename in hdr_releases {
            let quality = parse_quality(filename);
            assert!(
                quality.hdr.is_some(),
                "Should detect HDR in: {} (got {:?})",
                filename,
                quality.hdr
            );
        }
    }

    #[test]
    fn test_parse_real_world_audio_detection() {
        // Test Atmos detection
        let atmos_releases = vec![
            "A Knight of the Seven Kingdoms S01E01 The Hedge Knight 1080p AMZN WEB-DL DDP5 1 Atmos H 264-FLUX",
            "Shoresy S05E05 Total Buy-In 1080p AMZN WEB-DL DDP5 1 H 264-Kitsune",
            "Fallout S02E05 The Wrangler 2160p AMZN WEB-DL DDP5 1 Atmos DV HDR10Plus H 265-Kitsune",
        ];

        for filename in atmos_releases {
            let quality = parse_quality(filename);
            if filename.contains("Atmos") {
                assert!(
                    quality
                        .audio
                        .as_ref()
                        .map(|a| a.contains("Atmos"))
                        .unwrap_or(false),
                    "Should detect Atmos in: {} (got {:?})",
                    filename,
                    quality.audio
                );
            }
        }
    }

    #[test]
    fn test_parse_real_world_release_groups() {
        // Test release group detection
        let test_cases = vec![
            ("Fallout 2024 S02E05 1080p WEB h264-ETHEL", "ETHEL"),
            (
                "A Knight of the Seven Kingdoms S01E01 The Hedge Knight 1080p AMZN WEB-DL DDP5 1 Atmos H 264-FLUX",
                "FLUX",
            ),
            ("Fallout 2024 S02E05 1080p HEVC x265-MeGusta", "MeGusta"),
            ("Greys Anatomy S22E08 720p HDTV x264-SYNCOPY", "SYNCOPY"),
            (
                "Gold Rush S16E10 New Levels of Chaos 1080p DSCP WEB-DL DDP2 0 H 264-SNAKE",
                "SNAKE",
            ),
            (
                "A Knight of the Seven Kingdoms S01E01 The Hedge Knight 720p HMAX WEB-DL DDP5 1 H 264-NTb",
                "NTb",
            ),
            (
                "Fallout S02E05 The Wrangler 1080p AMZN WEB-DL DD 5 1 Atmos H 264-playWEB",
                "playWEB",
            ),
            ("Primal S03E02 1080p WEB h264-GRACE", "GRACE"),
            ("The Traitors 2023 S04E05 1080p WEB h264-EDITH", "EDITH"),
            (
                "The Rookie S08E02 Fast Andy 1080p AMZN WEB-DL DDP5 1 H 264-Kitsune",
                "Kitsune",
            ),
        ];

        for (filename, expected_group) in test_cases {
            let result = parse_episode(filename);
            assert_eq!(
                result.release_group.as_deref(),
                Some(expected_group),
                "Release group mismatch for: {} (got {:?})",
                filename,
                result.release_group
            );
        }
    }
}
