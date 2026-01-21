//! Shared text normalization and comparison utilities
//!
//! This module consolidates text processing functions that were previously
//! duplicated across multiple modules.

use regex::Regex;

/// Normalize a quality/codec string for comparison.
/// Removes common separators and converts to lowercase.
///
/// Used for comparing resolution, codec, and other quality-related strings.
///
/// # Example
/// ```ignore
/// assert_eq!(normalize_quality("1080p"), normalize_quality("1080P"));
/// assert_eq!(normalize_quality("x264"), normalize_quality("X.264"));
/// ```
pub fn normalize_quality(s: &str) -> String {
    s.to_lowercase().replace(['-', '.', ' ', '_'], "")
}

/// Normalize a show name for fuzzy matching.
/// Removes separators and extra whitespace, converts to lowercase.
///
/// Used for matching torrent filenames to show names in the library.
pub fn normalize_show_name(name: &str) -> String {
    name.to_lowercase()
        .replace(['.', '-', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Normalize a show name with article removal for sorting/matching.
/// Removes leading articles ("the", "a", "an") and replaces separators.
///
/// Used for more aggressive show name matching.
pub fn normalize_show_name_no_articles(name: &str) -> String {
    let mut normalized = name.to_lowercase();

    // Remove articles from the beginning
    let articles = ["the ", "a ", "an "];
    for article in articles {
        if normalized.starts_with(article) {
            normalized = normalized[article.len()..].to_string();
        }
    }

    // Replace separators and collapse whitespace
    normalized
        .replace(['.', '-', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Normalize a title for database matching (movies, etc.).
/// Removes punctuation and special characters, normalizes whitespace.
pub fn normalize_title(title: &str) -> String {
    title
        .to_lowercase()
        .replace(['\'', '\u{2019}', ':', '-', '.', '_'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Normalize a music track title for matching.
/// Removes parenthetical content (version info), quotes, and normalizes whitespace.
pub fn normalize_track_title(title: &str) -> String {
    // Remove content in parentheses/brackets (often version info)
    let without_brackets = Regex::new(r"\([^)]*\)|\[[^\]]*\]|\{[^}]*\}")
        .map(|re| re.replace_all(title, "").to_string())
        .unwrap_or_else(|_| title.to_string());

    without_brackets
        .to_lowercase()
        .replace(['\'', '"', '`'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Calculate Levenshtein distance between two strings.
/// Returns the minimum number of single-character edits needed to transform one string into another.
/// 
/// Uses rapidfuzz for optimized implementation.
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    use rapidfuzz::distance::levenshtein;
    levenshtein::distance(s1.chars(), s2.chars())
}

/// Calculate similarity between two strings (0.0 to 1.0).
/// Uses rapidfuzz normalized Levenshtein similarity.
pub fn string_similarity(s1: &str, s2: &str) -> f64 {
    use rapidfuzz::distance::levenshtein;
    
    if s1.is_empty() && s2.is_empty() {
        return 1.0;
    }
    
    levenshtein::normalized_similarity(s1.chars(), s2.chars())
}

/// Calculate similarity between two show names.
/// Normalizes both names before comparison and uses multiple matching strategies.
/// 
/// Delegates to the filename_parser implementation which uses rapidfuzz with:
/// - Basic Levenshtein similarity
/// - Partial ratio (substring matching)
/// - Token sort ratio (word order invariant)
pub fn show_name_similarity(name1: &str, name2: &str) -> f64 {
    // Use the main implementation from filename_parser
    super::filename_parser::show_name_similarity(name1, name2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_quality() {
        assert_eq!(normalize_quality("1080p"), "1080p");
        assert_eq!(normalize_quality("1080P"), "1080p");
        assert_eq!(normalize_quality("x.264"), "x264");
        assert_eq!(normalize_quality("H-265"), "h265");
        assert_eq!(normalize_quality("DTS HD"), "dtshd");
    }

    #[test]
    fn test_normalize_show_name() {
        assert_eq!(normalize_show_name("Breaking.Bad"), "breaking bad");
        assert_eq!(normalize_show_name("The-100"), "the 100");
        assert_eq!(normalize_show_name("Game_of_Thrones"), "game of thrones");
    }

    #[test]
    fn test_normalize_show_name_no_articles() {
        assert_eq!(
            normalize_show_name_no_articles("The Walking Dead"),
            "walking dead"
        );
        assert_eq!(
            normalize_show_name_no_articles("A Series of Events"),
            "series of events"
        );
        assert_eq!(
            normalize_show_name_no_articles("Breaking Bad"),
            "breaking bad"
        );
    }

    #[test]
    fn test_normalize_title() {
        assert_eq!(normalize_title("The Lord's Return"), "the lords return");
        assert_eq!(
            normalize_title("Spider-Man: No Way Home"),
            "spiderman no way home"
        );
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
        assert_eq!(levenshtein_distance("", "hello"), 5);
        assert_eq!(levenshtein_distance("hello", ""), 5);
        assert_eq!(levenshtein_distance("", ""), 0);
    }

    #[test]
    fn test_string_similarity() {
        assert!((string_similarity("hello", "hello") - 1.0).abs() < 0.001);
        assert!((string_similarity("hello", "hallo") - 0.8).abs() < 0.001);
        assert!(string_similarity("abc", "xyz") < 0.5);
    }

    #[test]
    fn test_show_name_similarity() {
        assert!(show_name_similarity("Breaking Bad", "Breaking.Bad") > 0.9);
        assert!(show_name_similarity("Game of Thrones", "Game_of_Thrones") > 0.9);
    }
}
