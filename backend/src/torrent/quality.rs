//! Quality profile matching for torrent selection
//!
//! NOTE: This is a local quality profile for torrent matching used for
//! the actual torrent quality filtering. Quality settings are now stored
//! inline on libraries and tv_shows tables (allowed_resolutions, etc.).

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Quality profile for filtering torrent releases (for future quality filtering)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityProfile {
    pub name: String,
    pub min_size_mb: Option<u64>,
    pub max_size_mb: Option<u64>,
    pub preferred_resolutions: Vec<String>,
    pub preferred_codecs: Vec<String>,
    pub banned_groups: Vec<String>,
    pub required_terms: Vec<String>,
    pub banned_terms: Vec<String>,
}

#[allow(dead_code)]
impl QualityProfile {
    /// Check if a release matches this quality profile
    pub fn matches(&self, release_title: &str, size_mb: u64) -> bool {
        // Check size constraints
        if let Some(min) = self.min_size_mb
            && size_mb < min
        {
            return false;
        }
        if let Some(max) = self.max_size_mb
            && size_mb > max
        {
            return false;
        }

        let title_lower = release_title.to_lowercase();

        // Check banned terms
        for term in &self.banned_terms {
            if title_lower.contains(&term.to_lowercase()) {
                return false;
            }
        }

        // Check banned groups
        for group in &self.banned_groups {
            if title_lower.contains(&group.to_lowercase()) {
                return false;
            }
        }

        // Check required terms
        for term in &self.required_terms {
            if !title_lower.contains(&term.to_lowercase()) {
                return false;
            }
        }

        true
    }

    /// Score a release for ranking (higher is better)
    pub fn score(&self, release_title: &str) -> i32 {
        let mut score = 0;
        let title_lower = release_title.to_lowercase();

        // Boost preferred resolutions
        for (i, res) in self.preferred_resolutions.iter().enumerate() {
            if title_lower.contains(&res.to_lowercase()) {
                score += (self.preferred_resolutions.len() - i) as i32 * 10;
                break;
            }
        }

        // Boost preferred codecs
        for (i, codec) in self.preferred_codecs.iter().enumerate() {
            if title_lower.contains(&codec.to_lowercase()) {
                score += (self.preferred_codecs.len() - i) as i32 * 5;
                break;
            }
        }

        score
    }

    /// Parse resolution from release title
    pub fn parse_resolution(title: &str) -> Option<String> {
        let patterns = [
            (r"2160p|4k|uhd", "2160p"),
            (r"1080p|fullhd|fhd", "1080p"),
            (r"720p|hd", "720p"),
            (r"480p|sd", "480p"),
        ];

        let title_lower = title.to_lowercase();
        for (pattern, resolution) in patterns {
            if let Ok(re) = Regex::new(pattern)
                && re.is_match(&title_lower)
            {
                return Some(resolution.to_string());
            }
        }

        None
    }
}

impl Default for QualityProfile {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            min_size_mb: Some(100),
            max_size_mb: Some(10000),
            preferred_resolutions: vec!["1080p".to_string(), "720p".to_string()],
            preferred_codecs: vec!["x265".to_string(), "hevc".to_string(), "x264".to_string()],
            banned_groups: vec![],
            required_terms: vec![],
            banned_terms: vec!["cam".to_string(), "hdcam".to_string(), "ts".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_profile_matches() {
        let profile = QualityProfile::default();

        assert!(profile.matches("Show.S01E01.1080p.WEB-DL.x264-GROUP", 500));
        assert!(!profile.matches("Show.S01E01.CAM.x264-GROUP", 500));
        assert!(!profile.matches("Show.S01E01.1080p.WEB-DL.x264-GROUP", 50)); // Too small
    }

    #[test]
    fn test_parse_resolution() {
        assert_eq!(
            QualityProfile::parse_resolution("Show.S01E01.1080p.mkv"),
            Some("1080p".to_string())
        );
        assert_eq!(
            QualityProfile::parse_resolution("Show.S01E01.720p.mkv"),
            Some("720p".to_string())
        );
        assert_eq!(
            QualityProfile::parse_resolution("Show.S01E01.2160p.UHD.mkv"),
            Some("2160p".to_string())
        );
    }
}
