//! Filename parser for scene-style release names
//!
//! Parses filenames like:
//! - "Chicago Fire S14E08 1080p WEB h264-ETHEL"
//! - "The.Daily.Show.2026.01.07.Stephen.J.Dubner.720p.WEB.h264-EDITH"
//! - "Corner Gas S06E12 Super Sensitive 1080p AMZN WEB-DL DDP2 0 H 264-QOQ"

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::debug;

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
    let cleaned = filename
        .replace('.', " ")
        .replace('_', " ")
        .replace('-', " ");

    // Try different patterns in order of specificity

    // Pattern 1: S01E01 format (most common)
    let sxxexx_re = Regex::new(r"(?i)(.+?)\s*[Ss](\d{1,2})[Ee](\d{1,2})").unwrap();
    if let Some(caps) = sxxexx_re.captures(&cleaned) {
        result.show_name = Some(clean_show_name(caps.get(1).unwrap().as_str()));
        result.season = caps.get(2).and_then(|m| m.as_str().parse().ok());
        result.episode = caps.get(3).and_then(|m| m.as_str().parse().ok());
    }
    // Pattern 2: 1x01 format
    else {
        let nxnn_re = Regex::new(r"(?i)(.+?)\s*(\d{1,2})x(\d{2})").unwrap();
        if let Some(caps) = nxnn_re.captures(&cleaned) {
            result.show_name = Some(clean_show_name(caps.get(1).unwrap().as_str()));
            result.season = caps.get(2).and_then(|m| m.as_str().parse().ok());
            result.episode = caps.get(3).and_then(|m| m.as_str().parse().ok());
        }
        // Pattern 3: Season X Episode Y format
        else {
            let verbose_re =
                Regex::new(r"(?i)(.+?)\s*Season\s*(\d+).*?Episode\s*(\d+)").unwrap();
            if let Some(caps) = verbose_re.captures(&cleaned) {
                result.show_name = Some(clean_show_name(caps.get(1).unwrap().as_str()));
                result.season = caps.get(2).and_then(|m| m.as_str().parse().ok());
                result.episode = caps.get(3).and_then(|m| m.as_str().parse().ok());
            }
            // Pattern 4: Daily show format (2026 01 07)
            else {
                let daily_re = Regex::new(r"(?i)(.+?)\s*(\d{4})\s*(\d{2})\s*(\d{2})").unwrap();
                if let Some(caps) = daily_re.captures(&cleaned) {
                    result.show_name = Some(clean_show_name(caps.get(1).unwrap().as_str()));
                    let year = caps.get(2).unwrap().as_str();
                    let month = caps.get(3).unwrap().as_str();
                    let day = caps.get(4).unwrap().as_str();
                    result.date = Some(format!("{}-{}-{}", year, month, day));
                    result.year = caps.get(2).and_then(|m| m.as_str().parse().ok());
                }
            }
        }
    }

    // Extract year from show name or filename (for disambiguation)
    if result.year.is_none() {
        let year_re = Regex::new(r"\b(19\d{2}|20\d{2})\b").unwrap();
        if let Some(caps) = year_re.captures(filename) {
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
    let group_re = Regex::new(r"-([A-Za-z0-9]+)(?:\.[A-Za-z0-9]+)?$").unwrap();
    if let Some(caps) = group_re.captures(filename) {
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

    // Resolution
    let res_re = Regex::new(r"(?i)(2160p|1080p|720p|480p|4K|UHD)").unwrap();
    if let Some(caps) = res_re.captures(filename) {
        let res = caps.get(1).unwrap().as_str().to_uppercase();
        quality.resolution = Some(match res.as_str() {
            "4K" | "UHD" => "2160p".to_string(),
            other => other.to_string(),
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

    // Codec
    if upper.contains("X265") || upper.contains("H265") || upper.contains("H.265") || upper.contains("HEVC") {
        quality.codec = Some("HEVC".to_string());
    } else if upper.contains("X264") || upper.contains("H264") || upper.contains("H.264") {
        quality.codec = Some("H.264".to_string());
    } else if upper.contains("AV1") {
        quality.codec = Some("AV1".to_string());
    } else if upper.contains("XVID") {
        quality.codec = Some("XviD".to_string());
    }

    // HDR
    if upper.contains("DOLBY VISION") || upper.contains("DOLBYVISION") || upper.contains("DV") || upper.contains("DOVI") {
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
    let year_re = Regex::new(r"\s*(19\d{2}|20\d{2})\s*$").unwrap();
    cleaned = year_re.replace(&cleaned, "").to_string();

    // Remove common suffixes
    let suffix_re = Regex::new(r"(?i)\s*(US|UK|AU|NZ)\s*$").unwrap();
    cleaned = suffix_re.replace(&cleaned, "").to_string();

    // Clean up multiple spaces
    let space_re = Regex::new(r"\s+").unwrap();
    cleaned = space_re.replace_all(&cleaned, " ").to_string();

    cleaned.trim().to_string()
}

/// Try to match a parsed episode to a show name (fuzzy matching)
pub fn normalize_show_name(name: &str) -> String {
    let mut normalized = name.to_lowercase();

    // Remove articles from the beginning
    let articles = ["the ", "a ", "an "];
    for article in articles {
        if normalized.starts_with(article) {
            normalized = normalized[article.len()..].to_string();
        }
    }

    // Remove special characters
    let special_re = Regex::new(r"[^a-z0-9\s]").unwrap();
    normalized = special_re.replace_all(&normalized, "").to_string();

    // Remove multiple spaces
    let space_re = Regex::new(r"\s+").unwrap();
    normalized = space_re.replace_all(&normalized, " ").to_string();

    normalized.trim().to_string()
}

/// Calculate similarity between two show names (0.0 to 1.0)
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
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_parse_daily_show() {
        let result = parse_episode("The.Daily.Show.2026.01.07.Stephen.J.Dubner.720p.WEB.h264-EDITH");
        assert_eq!(result.show_name.as_deref(), Some("The Daily Show"));
        assert_eq!(result.date.as_deref(), Some("2026-01-07"));
        assert_eq!(result.resolution.as_deref(), Some("720p"));
    }

    #[test]
    fn test_parse_quality() {
        let quality = parse_quality("Show S01E01 2160p AMZN WEB-DL DDP5 1 Atmos HDR H 265-GROUP");
        assert_eq!(quality.resolution.as_deref(), Some("2160p"));
        assert_eq!(quality.source.as_deref(), Some("AMZN WEB-DL"));
        assert_eq!(quality.codec.as_deref(), Some("HEVC"));
        assert_eq!(quality.hdr.as_deref(), Some("HDR10"));
        assert_eq!(quality.audio.as_deref(), Some("Atmos"));
    }

    #[test]
    fn test_show_name_similarity() {
        assert!(show_name_similarity("Chicago Fire", "Chicago Fire") > 0.99);
        assert!(show_name_similarity("The Office", "Office") > 0.9);
        assert!(show_name_similarity("Chicago Fire", "Chicago PD") > 0.7);
    }
}
