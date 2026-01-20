//! Track matching utilities for music albums
//!
//! This module provides fuzzy matching between expected tracks (from MusicBrainz)
//! and actual files in a torrent. Used to validate that a torrent contains
//! the correct tracks before downloading.

use regex::Regex;
use std::collections::HashSet;
use tracing::debug;

use super::torrent_metadata::TorrentFileInfo;
use crate::db::tracks::TrackRecord;

/// Result of matching torrent files against expected tracks
#[derive(Debug, Clone)]
pub struct TrackMatchResult {
    /// Number of expected tracks that were matched
    pub matched_count: usize,
    /// Total number of expected tracks
    pub expected_count: usize,
    /// Match percentage (0.0 to 1.0)
    pub match_percentage: f64,
    /// Individual track matches
    pub matches: Vec<TrackMatch>,
    /// Track titles that weren't matched
    pub unmatched_tracks: Vec<String>,
    /// Audio files that weren't matched to any track
    pub unmatched_files: Vec<String>,
}

impl TrackMatchResult {
    /// Check if the match meets a given threshold
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.match_percentage >= threshold
    }

    /// Default threshold of 80%
    pub const DEFAULT_THRESHOLD: f64 = 0.8;
}

/// A single track-to-file match
#[derive(Debug, Clone)]
pub struct TrackMatch {
    /// The expected track title
    pub track_title: String,
    /// The matched file name
    pub file_name: String,
    /// Match confidence (0.0 to 1.0)
    pub confidence: f64,
    /// How the match was determined
    pub match_type: MatchType,
}

/// How a track was matched to a file
#[derive(Debug, Clone, PartialEq)]
pub enum MatchType {
    /// Exact title match (after normalization)
    ExactTitle,
    /// File contains the track title
    ContainsTitle,
    /// Track number position matches
    TrackNumber,
    /// Fuzzy similarity match
    Fuzzy,
}

/// Match torrent audio files against expected tracks from the database
///
/// # Arguments
/// * `expected_tracks` - Tracks from MusicBrainz/database
/// * `torrent_files` - Audio files extracted from the torrent
///
/// # Returns
/// A `TrackMatchResult` with match statistics and details
pub fn match_tracks(
    expected_tracks: &[TrackRecord],
    torrent_files: &[TorrentFileInfo],
) -> TrackMatchResult {
    let audio_files: Vec<&TorrentFileInfo> =
        torrent_files.iter().filter(|f| f.is_audio()).collect();

    if expected_tracks.is_empty() {
        return TrackMatchResult {
            matched_count: 0,
            expected_count: 0,
            match_percentage: 0.0,
            matches: vec![],
            unmatched_tracks: vec![],
            unmatched_files: audio_files.iter().map(|f| f.name.clone()).collect(),
        };
    }

    let mut matches = Vec::new();
    let mut matched_file_indices: HashSet<usize> = HashSet::new();
    let mut unmatched_tracks = Vec::new();

    // Try to match each expected track
    for track in expected_tracks {
        let normalized_title = normalize_title(&track.title);

        // Find the best matching file
        let best_match = find_best_match(
            &normalized_title,
            track.track_number,
            track.disc_number,
            &audio_files,
            &matched_file_indices,
        );

        if let Some((file_idx, confidence, match_type)) = best_match {
            matched_file_indices.insert(file_idx);
            matches.push(TrackMatch {
                track_title: track.title.clone(),
                file_name: audio_files[file_idx].name.clone(),
                confidence,
                match_type,
            });
        } else {
            unmatched_tracks.push(track.title.clone());
        }
    }

    // Collect unmatched files
    let unmatched_files: Vec<String> = audio_files
        .iter()
        .enumerate()
        .filter(|(idx, _)| !matched_file_indices.contains(idx))
        .map(|(_, f)| f.name.clone())
        .collect();

    let matched_count = matches.len();
    let expected_count = expected_tracks.len();
    let match_percentage = if expected_count > 0 {
        matched_count as f64 / expected_count as f64
    } else {
        0.0
    };

    debug!(
        matched = matched_count,
        expected = expected_count,
        percentage = format!("{:.1}%", match_percentage * 100.0),
        unmatched_tracks = ?unmatched_tracks,
        "Track matching complete"
    );

    TrackMatchResult {
        matched_count,
        expected_count,
        match_percentage,
        matches,
        unmatched_tracks,
        unmatched_files,
    }
}

/// Find the best matching file for a track
fn find_best_match(
    normalized_title: &str,
    track_number: i32,
    disc_number: i32,
    audio_files: &[&TorrentFileInfo],
    already_matched: &HashSet<usize>,
) -> Option<(usize, f64, MatchType)> {
    let mut best_match: Option<(usize, f64, MatchType)> = None;

    for (idx, file) in audio_files.iter().enumerate() {
        if already_matched.contains(&idx) {
            continue;
        }

        let file_name = extract_title_from_filename(&file.name);
        let normalized_file = normalize_title(&file_name);

        // Try different matching strategies in order of confidence

        // 1. Exact title match (highest confidence)
        if normalized_file == normalized_title {
            return Some((idx, 1.0, MatchType::ExactTitle));
        }

        // 2. File contains the track title
        if normalized_file.contains(normalized_title) || normalized_title.contains(&normalized_file)
        {
            let confidence = 0.9;
            if best_match.as_ref().map(|m| m.1).unwrap_or(0.0) < confidence {
                best_match = Some((idx, confidence, MatchType::ContainsTitle));
            }
            continue;
        }

        // 3. Track number matching
        if let Some(file_track_num) = extract_track_number(&file.name) {
            let file_disc_num = extract_disc_number(&file.name).unwrap_or(1);

            if file_track_num == track_number && file_disc_num == disc_number {
                let confidence = 0.85;
                if best_match.as_ref().map(|m| m.1).unwrap_or(0.0) < confidence {
                    best_match = Some((idx, confidence, MatchType::TrackNumber));
                }
                continue;
            }
        }

        // 4. Fuzzy similarity
        let similarity = calculate_similarity(normalized_title, &normalized_file);
        if similarity >= 0.6 {
            let confidence = similarity * 0.8; // Scale down fuzzy matches
            if best_match.as_ref().map(|m| m.1).unwrap_or(0.0) < confidence {
                best_match = Some((idx, confidence, MatchType::Fuzzy));
            }
        }
    }

    best_match
}

/// Normalize a track title for comparison
///
/// - Lowercase
/// - Remove brackets and their contents (e.g., "(Remastered 2011)")
/// - Normalize separators
/// - Collapse whitespace
fn normalize_title(title: &str) -> String {
    // Remove content in parentheses/brackets (often version info)
    let without_brackets = Regex::new(r"\([^)]*\)|\[[^\]]*\]|\{[^}]*\}")
        .map(|re| re.replace_all(title, "").to_string())
        .unwrap_or_else(|_| title.to_string());

    without_brackets
        .to_lowercase()
        .replace(['\'', '"', '`'], "") // Remove quotes
        .replace(['–', '—'], "-") // Normalize dashes
        .replace(['-', '_', '.', ',', ':', ';', '!', '?'], " ") // Separators to space
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract the likely title portion from a filename
///
/// Removes track numbers, artist prefixes, and file extensions
fn extract_title_from_filename(filename: &str) -> String {
    // Remove extension
    let without_ext = filename
        .rsplit_once('.')
        .map(|(name, _)| name)
        .unwrap_or(filename);

    // Remove leading track number patterns like "01 - ", "1. ", "01_", "01-"
    // Pattern: digits followed by separator(s) including space-dash-space
    let track_num_re = Regex::new(r"^(\d{1,2})\s*[-._]\s*").unwrap();
    let without_track = track_num_re.replace(without_ext, "").to_string();

    // Remove common artist separator patterns like "Artist - Title" → "Title"
    // This handles "Pink Floyd - 01 - Speak to Me" after track number is removed
    // Now we have "Pink Floyd - Speak to Me" and want "Speak to Me"
    // But also handle "01 - Speak to Me" → "Speak to Me" (already removed track)

    // If there's still a "something - " pattern, try to extract the last part
    if let Some(idx) = without_track.rfind(" - ") {
        let after_sep = &without_track[idx + 3..];
        // Make sure there's meaningful content after the separator
        if after_sep.len() >= 3 {
            return after_sep.to_string();
        }
    }

    // Also try single dash with spaces
    if let Some(idx) = without_track.find(" - ") {
        let after_sep = &without_track[idx + 3..];
        if after_sep.len() >= 3 {
            return after_sep.to_string();
        }
    }

    without_track
}

/// Extract track number from a filename
///
/// Handles formats like:
/// - "01 - Track Title.flac"
/// - "01_Track_Title.mp3"
/// - "1. Track Title.flac"
/// - "Track 01.mp3"
fn extract_track_number(filename: &str) -> Option<i32> {
    // Pattern 1: Leading number (most common)
    let leading_re = Regex::new(r"^(\d{1,2})[\s._-]").unwrap();
    if let Some(caps) = leading_re.captures(filename) {
        if let Some(num) = caps.get(1) {
            if let Ok(n) = num.as_str().parse::<i32>() {
                return Some(n);
            }
        }
    }

    // Pattern 2: "Track XX" format
    let track_re = Regex::new(r"(?i)track\s*(\d{1,2})").unwrap();
    if let Some(caps) = track_re.captures(filename) {
        if let Some(num) = caps.get(1) {
            if let Ok(n) = num.as_str().parse::<i32>() {
                return Some(n);
            }
        }
    }

    None
}

/// Extract disc number from a filename
///
/// Handles formats like:
/// - "CD1/01 - Track.flac" (from path)
/// - "1-01 - Track.flac" (disc-track format)
/// - "Disc 2 - 01 - Track.flac"
fn extract_disc_number(filename: &str) -> Option<i32> {
    // Pattern 1: "CD1", "Disc 1", "Disk1"
    let disc_re = Regex::new(r"(?i)(?:cd|disc|disk)\s*(\d)").unwrap();
    if let Some(caps) = disc_re.captures(filename) {
        if let Some(num) = caps.get(1) {
            if let Ok(n) = num.as_str().parse::<i32>() {
                return Some(n);
            }
        }
    }

    // Pattern 2: Disc-Track format "1-01" at the start
    let disc_track_re = Regex::new(r"^(\d)-\d{2}[\s._-]").unwrap();
    if let Some(caps) = disc_track_re.captures(filename) {
        if let Some(num) = caps.get(1) {
            if let Ok(n) = num.as_str().parse::<i32>() {
                return Some(n);
            }
        }
    }

    None
}

/// Calculate string similarity using token-based comparison
///
/// Returns a value between 0.0 (no similarity) and 1.0 (identical)
fn calculate_similarity(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }

    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    // Token-based comparison (word overlap)
    let tokens_a: HashSet<&str> = a.split_whitespace().collect();
    let tokens_b: HashSet<&str> = b.split_whitespace().collect();

    if tokens_a.is_empty() || tokens_b.is_empty() {
        return 0.0;
    }

    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();

    if union == 0 {
        return 0.0;
    }

    // Jaccard similarity
    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn make_track(title: &str, track_num: i32, disc_num: i32) -> TrackRecord {
        TrackRecord {
            id: Uuid::new_v4(),
            album_id: Uuid::new_v4(),
            library_id: Uuid::new_v4(),
            title: title.to_string(),
            track_number: track_num,
            disc_number: disc_num,
            musicbrainz_id: None,
            isrc: None,
            duration_secs: Some(300),
            explicit: false,
            artist_name: None,
            artist_id: None,
            media_file_id: None,
            status: "wanted".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_file(name: &str, index: usize) -> TorrentFileInfo {
        TorrentFileInfo {
            path: PathBuf::from(name),
            name: name.to_string(),
            size: 50_000_000,
            index,
        }
    }

    #[test]
    fn test_normalize_title() {
        assert_eq!(normalize_title("Hello World"), "hello world");
        assert_eq!(normalize_title("Hello-World"), "hello world");
        assert_eq!(normalize_title("Hello_World"), "hello world");
        assert_eq!(normalize_title("  Hello   World  "), "hello world");
        assert_eq!(normalize_title("Hello (Remastered 2011)"), "hello");
        assert_eq!(normalize_title("Hello [Live]"), "hello");
    }

    #[test]
    fn test_extract_track_number() {
        assert_eq!(extract_track_number("01 - Track Title.flac"), Some(1));
        assert_eq!(extract_track_number("12_Track_Title.mp3"), Some(12));
        assert_eq!(extract_track_number("3. Track Title.flac"), Some(3));
        assert_eq!(extract_track_number("Track 05.mp3"), Some(5));
        assert_eq!(extract_track_number("Some Random File.mp3"), None);
    }

    #[test]
    fn test_extract_disc_number() {
        assert_eq!(extract_disc_number("CD1 - 01 - Track.flac"), Some(1));
        assert_eq!(extract_disc_number("Disc 2 - Track.flac"), Some(2));
        assert_eq!(extract_disc_number("2-01 - Track.flac"), Some(2));
        assert_eq!(extract_disc_number("01 - Track.flac"), None);
    }

    #[test]
    fn test_extract_title_from_filename() {
        assert_eq!(
            extract_title_from_filename("01 - Speak to Me.flac"),
            "Speak to Me"
        );
        assert_eq!(
            extract_title_from_filename("01_Speak_to_Me.mp3"),
            "Speak_to_Me"
        );
        assert_eq!(
            extract_title_from_filename("Pink Floyd - 01 - Speak to Me.flac"),
            "Speak to Me"
        );
    }

    #[test]
    fn test_match_tracks_exact() {
        let tracks = vec![
            make_track("Speak to Me", 1, 1),
            make_track("Breathe", 2, 1),
            make_track("Time", 3, 1),
        ];

        let files = vec![
            make_file("01 - Speak to Me.flac", 0),
            make_file("02 - Breathe.flac", 1),
            make_file("03 - Time.flac", 2),
            make_file("cover.jpg", 3),
        ];

        let result = match_tracks(&tracks, &files);

        assert_eq!(result.matched_count, 3);
        assert_eq!(result.expected_count, 3);
        assert_eq!(result.match_percentage, 1.0);
        assert!(result.unmatched_tracks.is_empty());
        assert!(result.unmatched_files.is_empty());
    }

    #[test]
    fn test_match_tracks_partial() {
        let tracks = vec![
            make_track("Speak to Me", 1, 1),
            make_track("Breathe", 2, 1),
            make_track("Time", 3, 1),
            make_track("Money", 4, 1),
        ];

        let files = vec![
            make_file("01 - Speak to Me.flac", 0),
            make_file("02 - Breathe.flac", 1),
            // Missing Time and Money
        ];

        let result = match_tracks(&tracks, &files);

        assert_eq!(result.matched_count, 2);
        assert_eq!(result.expected_count, 4);
        assert_eq!(result.match_percentage, 0.5);
        assert_eq!(result.unmatched_tracks.len(), 2);
    }

    #[test]
    fn test_match_tracks_by_number() {
        let tracks = vec![
            make_track("Some Title", 1, 1),
            make_track("Another Title", 2, 1),
        ];

        // Files have track numbers but completely different names
        let files = vec![
            make_file("01 - Completely Different.flac", 0),
            make_file("02 - Also Different.flac", 1),
        ];

        let result = match_tracks(&tracks, &files);

        // Should match by track number
        assert_eq!(result.matched_count, 2);
        assert!(
            result
                .matches
                .iter()
                .all(|m| m.match_type == MatchType::TrackNumber)
        );
    }

    #[test]
    fn test_match_tracks_bonus_tracks_ok() {
        // Album expects 3 tracks but torrent has 5 (bonus tracks)
        let tracks = vec![
            make_track("Track One", 1, 1),
            make_track("Track Two", 2, 1),
            make_track("Track Three", 3, 1),
        ];

        let files = vec![
            make_file("01 - Track One.flac", 0),
            make_file("02 - Track Two.flac", 1),
            make_file("03 - Track Three.flac", 2),
            make_file("04 - Bonus Track.flac", 3),
            make_file("05 - Another Bonus.flac", 4),
        ];

        let result = match_tracks(&tracks, &files);

        // All expected tracks matched = 100%
        assert_eq!(result.matched_count, 3);
        assert_eq!(result.expected_count, 3);
        assert_eq!(result.match_percentage, 1.0);
        // But we have unmatched files (bonus tracks)
        assert_eq!(result.unmatched_files.len(), 2);
    }

    #[test]
    fn test_similarity() {
        assert_eq!(calculate_similarity("hello world", "hello world"), 1.0);
        assert!(calculate_similarity("hello world", "hello") > 0.3);
        assert!(calculate_similarity("completely different", "nothing alike") < 0.2);
    }

    #[test]
    fn test_meets_threshold() {
        let result = TrackMatchResult {
            matched_count: 8,
            expected_count: 10,
            match_percentage: 0.8,
            matches: vec![],
            unmatched_tracks: vec![],
            unmatched_files: vec![],
        };

        assert!(result.meets_threshold(0.8));
        assert!(result.meets_threshold(0.7));
        assert!(!result.meets_threshold(0.9));
    }
}
