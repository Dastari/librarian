//! Fuzzy/Similar text matching utilities
//!
//! Provides high-quality text matching using multiple algorithms:
//! - Levenshtein distance (edit distance)
//! - Jaro-Winkler similarity (good for names, handles transpositions)
//! - Normalized matching (handles case, punctuation, articles)
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::graphql::orm::fuzzy::{FuzzyMatcher, FuzzyMatch};
//!
//! let matcher = FuzzyMatcher::new("The Hunt for Red October")
//!     .with_threshold(0.6);
//!
//! // Score a single candidate
//! let score = matcher.score("Red.October.1990.1080p.BluRay");
//!
//! // Filter and sort a collection
//! let matches: Vec<FuzzyMatch<Movie>> = matcher.filter_and_score(
//!     movies,
//!     |m| &m.title,
//! );
//! ```

use strsim::{jaro_winkler, normalized_levenshtein};

/// Result of a fuzzy match with the matched entity and its score
#[derive(Debug, Clone)]
pub struct FuzzyMatch<T> {
    /// The matched entity
    pub entity: T,
    /// Similarity score (0.0 to 1.0, higher is better)
    pub score: f64,
}

impl<T> FuzzyMatch<T> {
    pub fn new(entity: T, score: f64) -> Self {
        Self { entity, score }
    }
}

/// Fuzzy text matcher with configurable threshold and algorithms
#[derive(Debug, Clone)]
pub struct FuzzyMatcher {
    /// The normalized query string
    query: String,
    /// Original query (for exact match bonus)
    original_query: String,
    /// Minimum similarity threshold (0.0 to 1.0)
    threshold: f64,
    /// Weight for Jaro-Winkler (vs Levenshtein)
    jaro_weight: f64,
}

impl FuzzyMatcher {
    /// Create a new fuzzy matcher with default settings
    pub fn new(query: impl Into<String>) -> Self {
        let original = query.into();
        Self {
            query: normalize_for_matching(&original),
            original_query: original,
            threshold: 0.6,
            jaro_weight: 0.6, // Favor Jaro-Winkler slightly
        }
    }

    /// Set the minimum similarity threshold (default: 0.6)
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Score a candidate string against the query
    /// Returns a score from 0.0 to 1.0
    pub fn score(&self, candidate: &str) -> f64 {
        if candidate.is_empty() || self.query.is_empty() {
            return 0.0;
        }

        let normalized = normalize_for_matching(candidate);

        // Exact match (after normalization)
        if normalized == self.query {
            return 1.0;
        }

        // Combined score from multiple algorithms
        let jaro = jaro_winkler(&self.query, &normalized);
        let lev = normalized_levenshtein(&self.query, &normalized);

        // Weighted combination
        let base_score = (self.jaro_weight * jaro) + ((1.0 - self.jaro_weight) * lev);

        // Bonus for substring containment
        let containment_bonus =
            if normalized.contains(&self.query) || self.query.contains(&normalized) {
                0.1
            } else {
                0.0
            };

        // Bonus for matching first word
        let first_word_bonus = if let (Some(q_first), Some(c_first)) = (
            self.query.split_whitespace().next(),
            normalized.split_whitespace().next(),
        ) {
            if q_first == c_first {
                0.1
            } else if jaro_winkler(q_first, c_first) > 0.9 {
                0.05
            } else {
                0.0
            }
        } else {
            0.0
        };

        (base_score + containment_bonus + first_word_bonus).min(1.0)
    }

    /// Check if a candidate meets the threshold
    pub fn matches(&self, candidate: &str) -> bool {
        self.score(candidate) >= self.threshold
    }

    /// Filter and score a collection, returning matches sorted by score (descending)
    pub fn filter_and_score<T, F>(&self, items: Vec<T>, get_text: F) -> Vec<FuzzyMatch<T>>
    where
        F: Fn(&T) -> Option<&str>,
    {
        let mut matches: Vec<FuzzyMatch<T>> = items
            .into_iter()
            .filter_map(|item| {
                let text = get_text(&item)?;
                let score = self.score(text);
                if score >= self.threshold {
                    Some(FuzzyMatch::new(item, score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        matches
    }

    /// Filter, score, and limit results
    pub fn filter_and_score_limit<T, F>(
        &self,
        items: Vec<T>,
        get_text: F,
        limit: usize,
    ) -> Vec<FuzzyMatch<T>>
    where
        F: Fn(&T) -> Option<&str>,
    {
        let mut matches = self.filter_and_score(items, get_text);
        matches.truncate(limit);
        matches
    }

    /// Get the threshold
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
}

/// Normalize a string for fuzzy matching
/// - Lowercase
/// - Remove common articles (the, a, an)
/// - Remove punctuation and extra whitespace
/// - Handle common media naming patterns (dots, underscores as spaces)
pub fn normalize_for_matching(s: &str) -> String {
    let s = s.to_lowercase();

    // Replace common separators with spaces
    let s = s
        .replace('.', " ")
        .replace('_', " ")
        .replace('-', " ")
        .replace('[', " ")
        .replace(']', " ")
        .replace('(', " ")
        .replace(')', " ");

    // Remove punctuation
    let s: String = s
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();

    // Split into words, remove articles, rejoin
    let words: Vec<&str> = s
        .split_whitespace()
        .filter(|w| !matches!(*w, "the" | "a" | "an"))
        .collect();

    words.join(" ")
}

/// Generate a broad SQL LIKE pattern for candidate filtering
/// This is used to reduce the dataset before Rust-side scoring
pub fn generate_candidate_pattern(query: &str) -> String {
    let normalized = normalize_for_matching(query);
    let words: Vec<&str> = normalized.split_whitespace().collect();

    if words.is_empty() {
        return "%".to_string();
    }

    // Use the longest word for best filtering
    let best_word = words.iter().max_by_key(|w| w.len()).unwrap_or(&"");

    if best_word.len() >= 3 {
        format!("%{}%", best_word)
    } else if !words.is_empty() {
        // Use first word if all words are short
        format!("%{}%", words[0])
    } else {
        "%".to_string()
    }
}

/// Score and filter results that were fetched with a Similar filter
/// This is called after fetching candidates from the database
pub fn apply_similar_filter<T, F>(
    items: Vec<T>,
    query: &str,
    threshold: f64,
    get_text: F,
) -> Vec<(T, f64)>
where
    F: Fn(&T) -> Option<&str>,
{
    let matcher = FuzzyMatcher::new(query).with_threshold(threshold);
    matcher
        .filter_and_score(items, get_text)
        .into_iter()
        .map(|m| (m.entity, m.score))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_for_matching() {
        assert_eq!(
            normalize_for_matching("The Hunt for Red October"),
            "hunt for red october"
        );
        assert_eq!(
            normalize_for_matching("Red.October.1990.1080p.BluRay"),
            "red october 1990 1080p bluray"
        );
        assert_eq!(normalize_for_matching("A_New_Hope"), "new hope");
    }

    #[test]
    fn test_fuzzy_matching() {
        let matcher = FuzzyMatcher::new("The Hunt for Red October");

        // Exact match (normalized)
        assert!(matcher.score("Hunt for Red October") > 0.95);

        // Close match
        assert!(matcher.score("Red October") > 0.6);

        // Torrent-style name
        assert!(matcher.score("The.Hunt.for.Red.October.1990.1080p.BluRay") > 0.7);

        // Poor match
        assert!(matcher.score("Crimson Tide") < 0.5);
    }

    #[test]
    fn test_generate_candidate_pattern() {
        assert_eq!(
            generate_candidate_pattern("The Hunt for Red October"),
            "%october%"
        );
        assert_eq!(generate_candidate_pattern("Red October"), "%october%");
    }
}
