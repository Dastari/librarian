//! Weighted fuzzy matching scorer for media files
//!
//! This module provides a unified scoring system for matching files to library items
//! across all media types (Music, TV, Movies, Audiobooks).
//!
//! ## Scoring Formula
//! ```text
//! final_score = Σ (field_weight × fuzzy_similarity)
//! ```
//!
//! Example: Album field worth 25 points, fuzzy match = 80% → contributes 20 points

use super::filename_parser;

/// Parsed info extracted from a filename (track number, cleaned title, etc.)
#[derive(Debug, Clone, Default)]
pub struct ParsedFileInfo {
    /// Track/chapter/episode number extracted from filename (e.g., "02 - Title.flac" → 2)
    pub number: Option<u32>,
    /// Disc number extracted from filename (e.g., "CD2/01 - Title.flac" → 2)
    pub disc_number: Option<u32>,
    /// Season number for TV (e.g., "S01E05" → 1)
    pub season: Option<u32>,
    /// Episode number for TV (e.g., "S01E05" → 5)
    pub episode: Option<u32>,
    /// Year extracted from filename (e.g., "Movie (2020).mkv" → 2020)
    pub year: Option<i32>,
    /// Cleaned title with numbers/extensions removed
    pub cleaned_title: String,
    /// Original filename for reference
    pub original: String,
}

/// Score breakdown showing how each field contributed to the total
#[derive(Debug, Clone, Default)]
pub struct ScoreBreakdown {
    /// Artist/Author comparison (similarity, weighted_score)
    pub artist: Option<(f64, f64)>,
    /// Album/Book/Show title comparison
    pub album_or_show: Option<(f64, f64)>,
    /// Track/Episode/Chapter title comparison
    pub title: Option<(f64, f64)>,
    /// Track/Episode/Chapter number comparison
    pub number: Option<(f64, f64)>,
    /// Season number comparison (TV only)
    pub season: Option<(f64, f64)>,
    /// Year comparison
    pub year: Option<(f64, f64)>,
    /// Total score (0-100)
    pub total: f64,
}

impl ScoreBreakdown {
    /// Create a human-readable summary of the scoring
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some((sim, score)) = self.artist {
            parts.push(format!("artist:{:.0}%→{:.1}", sim * 100.0, score));
        }
        if let Some((sim, score)) = self.album_or_show {
            parts.push(format!("album:{:.0}%→{:.1}", sim * 100.0, score));
        }
        if let Some((sim, score)) = self.title {
            parts.push(format!("title:{:.0}%→{:.1}", sim * 100.0, score));
        }
        if let Some((sim, score)) = self.number {
            parts.push(format!("num:{:.0}%→{:.1}", sim * 100.0, score));
        }
        if let Some((sim, score)) = self.season {
            parts.push(format!("season:{:.0}%→{:.1}", sim * 100.0, score));
        }
        if let Some((sim, score)) = self.year {
            parts.push(format!("year:{:.0}%→{:.1}", sim * 100.0, score));
        }
        format!("total:{:.1} [{}]", self.total, parts.join(", "))
    }
}

// ============================================================================
// Weight Constants
// ============================================================================

/// Music scoring weights (total = 100)
pub mod music_weights {
    pub const ARTIST: f64 = 30.0;
    pub const ALBUM: f64 = 25.0;
    pub const TRACK_TITLE: f64 = 25.0;
    pub const TRACK_NUMBER: f64 = 15.0;
    pub const YEAR: f64 = 5.0;
}

/// TV show scoring weights (total = 100)
pub mod tv_weights {
    pub const SHOW_NAME: f64 = 30.0;
    pub const SEASON: f64 = 25.0;
    pub const EPISODE: f64 = 25.0;
    pub const EPISODE_TITLE: f64 = 15.0;
    pub const YEAR: f64 = 5.0;
}

/// Movie scoring weights (total = 100)
pub mod movie_weights {
    pub const TITLE: f64 = 50.0;
    pub const YEAR: f64 = 30.0;
    pub const DIRECTOR: f64 = 20.0;
}

/// Audiobook scoring weights (total = 100)
pub mod audiobook_weights {
    pub const AUTHOR: f64 = 30.0;
    pub const BOOK_TITLE: f64 = 30.0;
    pub const CHAPTER_TITLE: f64 = 25.0;
    pub const CHAPTER_NUMBER: f64 = 15.0;
}

// ============================================================================
// Parsing Functions
// ============================================================================

/// Parse track/chapter info from a filename
///
/// Extracts:
/// - Track/chapter number from prefix (e.g., "02 - Title.flac" → 2)
/// - Disc number from prefix or path (e.g., "CD2/01 - Title.flac" → disc 2)
/// - Cleaned title without the number prefix
///
/// # Examples
/// ```
/// let info = parse_track_info("02 - It's So Easy.flac");
/// assert_eq!(info.number, Some(2));
/// assert_eq!(info.cleaned_title, "It's So Easy");
///
/// let info2 = parse_track_info("2-01 - Track Title.flac");
/// assert_eq!(info2.disc_number, Some(2));
/// assert_eq!(info2.number, Some(1));
/// ```
pub fn parse_track_info(filename: &str) -> ParsedFileInfo {
    let original = filename.to_string();
    
    // Remove extension first
    let name_without_ext = if let Some(pos) = filename.rfind('.') {
        &filename[..pos]
    } else {
        filename
    };
    
    // Extract disc number from filename
    let disc_number = extract_disc_number(filename);
    
    // Try disc-track format first: "1-01 - Title" or "2-05. Title"
    let disc_track_re = regex::Regex::new(
        r"^(\d)-(\d{2})[\s\-._]+(.*)$"
    ).unwrap();
    
    if let Some(caps) = disc_track_re.captures(name_without_ext) {
        let disc = caps.get(1).and_then(|m| m.as_str().parse().ok());
        let track = caps.get(2).and_then(|m| m.as_str().parse().ok());
        let title = caps.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
        
        return ParsedFileInfo {
            number: track,
            disc_number: disc.or(disc_number),
            cleaned_title: title,
            original,
            ..Default::default()
        };
    }
    
    // Try "CD1 - 05 - Title" or "Disc 2 - 03 - Title" format
    let cd_track_re = regex::Regex::new(
        r"(?i)^(?:cd|disc|disk)\s*\d[\s\-._]+(\d{1,3})[\s\-._]+(.*)$"
    ).unwrap();
    
    if let Some(caps) = cd_track_re.captures(name_without_ext) {
        let track = caps.get(1).and_then(|m| m.as_str().parse().ok());
        let title = caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
        
        return ParsedFileInfo {
            number: track,
            disc_number, // Already extracted via extract_disc_number
            cleaned_title: title,
            original,
            ..Default::default()
        };
    }
    
    // Try to extract leading track number
    // Patterns: "01 - Title", "01. Title", "01-Title", "01_Title", "Track 01 - Title"
    let track_number_re = regex::Regex::new(
        r"^(?:Track\s*)?(\d{1,3})[\s\-._]+(.*)$"
    ).unwrap();
    
    if let Some(caps) = track_number_re.captures(name_without_ext) {
        let number = caps.get(1).and_then(|m| m.as_str().parse().ok());
        let title = caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
        
        return ParsedFileInfo {
            number,
            disc_number,
            cleaned_title: title,
            original,
            ..Default::default()
        };
    }
    
    // No track number found - just clean the title
    ParsedFileInfo {
        number: None,
        disc_number,
        cleaned_title: name_without_ext.to_string(),
        original,
        ..Default::default()
    }
}

/// Extract disc number from a filename
///
/// Handles formats like:
/// - "CD1/01 - Track.flac" (from path)
/// - "1-01 - Track.flac" (disc-track format)
/// - "Disc 2 - 01 - Track.flac"
fn extract_disc_number(filename: &str) -> Option<u32> {
    // Pattern 1: "CD1", "Disc 1", "Disk1"
    let disc_re = regex::Regex::new(r"(?i)(?:cd|disc|disk)\s*(\d)").unwrap();
    if let Some(caps) = disc_re.captures(filename) {
        if let Some(num) = caps.get(1) {
            if let Ok(n) = num.as_str().parse::<u32>() {
                return Some(n);
            }
        }
    }

    // Pattern 2: Disc-Track format "1-01" at the start
    let disc_track_re = regex::Regex::new(r"^(\d)-\d{2}[\s._-]").unwrap();
    if let Some(caps) = disc_track_re.captures(filename) {
        if let Some(num) = caps.get(1) {
            if let Ok(n) = num.as_str().parse::<u32>() {
                return Some(n);
            }
        }
    }

    None
}

/// Parse TV episode info from a filename
///
/// Extracts:
/// - Season number (S01, Season 1, etc.)
/// - Episode number (E05, Episode 5, x05, etc.)
/// - Episode title (if present after the pattern)
///
/// # Examples
/// ```
/// let info = parse_episode_info("Show Name S01E05 Episode Title.mkv");
/// assert_eq!(info.season, Some(1));
/// assert_eq!(info.episode, Some(5));
/// ```
pub fn parse_episode_info(filename: &str) -> ParsedFileInfo {
    let original = filename.to_string();
    
    // Remove extension
    let name_without_ext = if let Some(pos) = filename.rfind('.') {
        &filename[..pos]
    } else {
        filename
    };
    
    // Standard S01E05 pattern
    let se_pattern = regex::Regex::new(
        r"[Ss](\d{1,2})[Ee](\d{1,3})"
    ).unwrap();
    
    // Alternative patterns: 1x05, Season 1 Episode 5
    let alt_pattern = regex::Regex::new(
        r"(\d{1,2})[xX](\d{1,3})"
    ).unwrap();
    
    let verbose_pattern = regex::Regex::new(
        r"[Ss]eason\s*(\d{1,2}).*?[Ee]pisode\s*(\d{1,3})"
    ).unwrap();
    
    let (season, episode) = if let Some(caps) = se_pattern.captures(name_without_ext) {
        (
            caps.get(1).and_then(|m| m.as_str().parse().ok()),
            caps.get(2).and_then(|m| m.as_str().parse().ok()),
        )
    } else if let Some(caps) = alt_pattern.captures(name_without_ext) {
        (
            caps.get(1).and_then(|m| m.as_str().parse().ok()),
            caps.get(2).and_then(|m| m.as_str().parse().ok()),
        )
    } else if let Some(caps) = verbose_pattern.captures(name_without_ext) {
        (
            caps.get(1).and_then(|m| m.as_str().parse().ok()),
            caps.get(2).and_then(|m| m.as_str().parse().ok()),
        )
    } else {
        (None, None)
    };
    
    // Try to extract episode title (text after S01E05 pattern)
    let title_after_pattern = regex::Regex::new(
        r"[Ss]\d{1,2}[Ee]\d{1,3}[\s\-._]*(.+?)(?:\s*\d{3,4}p|\s*(?:HDTV|WEB|BluRay)|$)"
    ).unwrap();
    
    let cleaned_title = title_after_pattern
        .captures(name_without_ext)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| name_without_ext.to_string());
    
    ParsedFileInfo {
        season,
        episode,
        cleaned_title,
        original,
        ..Default::default()
    }
}

/// Parse movie info from a filename
///
/// Extracts:
/// - Title
/// - Year (if present in parentheses or brackets)
///
/// # Examples
/// ```
/// let info = parse_movie_info("The Matrix (1999).mkv");
/// assert_eq!(info.year, Some(1999));
/// assert_eq!(info.cleaned_title, "The Matrix");
/// ```
pub fn parse_movie_info(filename: &str) -> ParsedFileInfo {
    let original = filename.to_string();
    
    // Remove extension
    let name_without_ext = if let Some(pos) = filename.rfind('.') {
        &filename[..pos]
    } else {
        filename
    };
    
    // Extract year in parentheses or brackets: (2020), [2020]
    let year_pattern = regex::Regex::new(r"[\(\[](\d{4})[\)\]]").unwrap();
    let year = year_pattern
        .captures(name_without_ext)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok());
    
    // Remove year and quality info to get clean title
    let clean_pattern = regex::Regex::new(
        r"[\(\[]\d{4}[\)\]]|(?:\d{3,4}p)|(?:BluRay|WEB-DL|WEBRip|HDTV|DVDRip|BRRip|HDRip|x264|x265|HEVC|AAC|DTS|REMUX|REPACK)"
    ).unwrap();
    
    let cleaned_title = clean_pattern
        .replace_all(name_without_ext, " ")
        .trim()
        .replace("  ", " ")
        .replace(".", " ")
        .trim()
        .to_string();
    
    ParsedFileInfo {
        year,
        cleaned_title,
        original,
        ..Default::default()
    }
}

// ============================================================================
// Scoring Functions
// ============================================================================

/// Calculate fuzzy similarity between two strings (0.0 - 1.0)
pub fn fuzzy_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    filename_parser::show_name_similarity(a, b)
}

/// Compare two numbers for equality (returns 1.0 if equal, 0.0 otherwise)
pub fn number_match(a: Option<u32>, b: i32) -> f64 {
    match a {
        Some(n) if n as i32 == b => 1.0,
        _ => 0.0,
    }
}

/// Compare two optional numbers for equality
pub fn number_match_opt(a: Option<u32>, b: Option<i32>) -> f64 {
    match (a, b) {
        (Some(an), Some(bn)) if an as i32 == bn => 1.0,
        _ => 0.0,
    }
}

/// Compare years with some tolerance (exact = 1.0, ±1 year = 0.8, else 0.0)
pub fn year_match(file_year: Option<i32>, db_year: Option<i32>) -> f64 {
    match (file_year, db_year) {
        (Some(fy), Some(dy)) => {
            let diff = (fy - dy).abs();
            if diff == 0 {
                1.0
            } else if diff == 1 {
                0.8 // Allow 1 year tolerance
            } else {
                0.0
            }
        }
        _ => 0.0, // Can't compare if either is missing
    }
}

/// Score a music track match
///
/// # Arguments
/// * `meta_artist` - Artist from file metadata
/// * `meta_album` - Album from file metadata  
/// * `meta_title` - Title from file metadata
/// * `meta_track` - Track number from metadata
/// * `meta_disc` - Disc number from metadata
/// * `meta_year` - Year from metadata
/// * `file_info` - Parsed info from filename
/// * `db_artist` - Artist name from database
/// * `db_album` - Album name from database
/// * `db_track_title` - Track title from database
/// * `db_track_number` - Track number from database
/// * `db_disc_number` - Disc number from database
/// * `db_year` - Year from database
pub fn score_music_match(
    meta_artist: Option<&str>,
    meta_album: Option<&str>,
    meta_title: Option<&str>,
    meta_track: Option<u32>,
    meta_disc: Option<u32>,
    meta_year: Option<i32>,
    file_info: &ParsedFileInfo,
    db_artist: &str,
    db_album: &str,
    db_track_title: &str,
    db_track_number: i32,
    db_disc_number: i32,
    db_year: Option<i32>,
) -> ScoreBreakdown {
    let mut breakdown = ScoreBreakdown::default();
    
    // Artist comparison (use metadata if available)
    if let Some(artist) = meta_artist {
        let sim = fuzzy_similarity(artist, db_artist);
        let score = sim * music_weights::ARTIST;
        breakdown.artist = Some((sim, score));
        breakdown.total += score;
    }
    
    // Album comparison (use metadata if available)
    if let Some(album) = meta_album {
        let sim = fuzzy_similarity(album, db_album);
        let score = sim * music_weights::ALBUM;
        breakdown.album_or_show = Some((sim, score));
        breakdown.total += score;
    }
    
    // Track title comparison (prefer metadata, fall back to filename)
    let title_to_compare = meta_title.unwrap_or(&file_info.cleaned_title);
    if !title_to_compare.is_empty() {
        let sim = fuzzy_similarity(title_to_compare, db_track_title);
        let score = sim * music_weights::TRACK_TITLE;
        breakdown.title = Some((sim, score));
        breakdown.total += score;
    }
    
    // Disc number comparison (prefer metadata, fall back to filename)
    // We get the file's disc number for use in track number comparison
    let file_disc = meta_disc.or(file_info.disc_number);
    
    // Track number comparison - ONLY award points if disc numbers match
    // This prevents CD2 Track 1 from matching CD1 Track 1
    let track_num = meta_track.or(file_info.number);
    let disc_matches = disc_number_matches(file_disc, db_disc_number);
    
    if disc_matches {
        let sim = number_match(track_num, db_track_number);
        if sim > 0.0 {
            let score = sim * music_weights::TRACK_NUMBER;
            breakdown.number = Some((sim, score));
            breakdown.total += score;
        }
    } else {
        // Disc number mismatch - record as 0 points for track number
        // This is important: file has explicit disc 2 but DB track is disc 1 (or vice versa)
        breakdown.number = Some((0.0, 0.0));
    }
    
    // Year comparison
    let sim = year_match(meta_year, db_year);
    if sim > 0.0 {
        let score = sim * music_weights::YEAR;
        breakdown.year = Some((sim, score));
        breakdown.total += score;
    }
    
    breakdown
}

/// Check if disc numbers match for track number scoring purposes
///
/// Returns true if:
/// - Both disc numbers are equal
/// - File has no disc number (treat as disc 1 or "any disc")
/// - DB track is disc 1 and file has no disc info (common single-disc albums)
fn disc_number_matches(file_disc: Option<u32>, db_disc: i32) -> bool {
    match file_disc {
        // File explicitly specifies a disc number - must match exactly
        Some(fd) => fd as i32 == db_disc,
        // File has no disc info - only match disc 1 (or allow if db also has no disc info, i.e., disc 1)
        // This handles the common case of single-disc albums where disc number isn't specified
        None => db_disc == 1,
    }
}

/// Score a TV episode match
pub fn score_tv_match(
    meta_show: Option<&str>,
    meta_season: Option<i32>,
    meta_episode: Option<i32>,
    meta_title: Option<&str>,
    file_info: &ParsedFileInfo,
    source_name: Option<&str>,
    db_show: &str,
    db_season: i32,
    db_episode: i32,
    db_episode_title: Option<&str>,
) -> ScoreBreakdown {
    let mut breakdown = ScoreBreakdown::default();
    
    // Show name comparison (prefer metadata, then source name)
    let show_to_compare = meta_show.or(source_name);
    if let Some(show) = show_to_compare {
        let sim = fuzzy_similarity(show, db_show);
        let score = sim * tv_weights::SHOW_NAME;
        breakdown.album_or_show = Some((sim, score));
        breakdown.total += score;
    }
    
    // Season number comparison (prefer metadata, fall back to filename)
    let season = meta_season.map(|s| s as u32).or(file_info.season);
    let sim = number_match(season, db_season);
    if sim > 0.0 {
        let score = sim * tv_weights::SEASON;
        breakdown.season = Some((sim, score));
        breakdown.total += score;
    }
    
    // Episode number comparison (prefer metadata, fall back to filename)
    let episode = meta_episode.map(|e| e as u32).or(file_info.episode);
    let sim = number_match(episode, db_episode);
    if sim > 0.0 {
        let score = sim * tv_weights::EPISODE;
        breakdown.number = Some((sim, score));
        breakdown.total += score;
    }
    
    // Episode title comparison (if available)
    if let (Some(meta_t), Some(db_t)) = (meta_title, db_episode_title) {
        let sim = fuzzy_similarity(meta_t, db_t);
        let score = sim * tv_weights::EPISODE_TITLE;
        breakdown.title = Some((sim, score));
        breakdown.total += score;
    } else if let Some(db_t) = db_episode_title {
        // Try matching cleaned filename title to episode title
        if !file_info.cleaned_title.is_empty() {
            let sim = fuzzy_similarity(&file_info.cleaned_title, db_t);
            let score = sim * tv_weights::EPISODE_TITLE;
            breakdown.title = Some((sim, score));
            breakdown.total += score;
        }
    }
    
    breakdown
}

/// Score a movie match
pub fn score_movie_match(
    meta_title: Option<&str>,
    meta_year: Option<i32>,
    meta_director: Option<&str>,
    file_info: &ParsedFileInfo,
    source_name: Option<&str>,
    db_title: &str,
    db_year: Option<i32>,
    db_director: Option<&str>,
) -> ScoreBreakdown {
    let mut breakdown = ScoreBreakdown::default();
    
    // Title comparison (prefer metadata, then source name, then filename)
    let title_to_compare = meta_title
        .or(source_name)
        .map(|s| s.to_string())
        .unwrap_or_else(|| file_info.cleaned_title.clone());
    
    if !title_to_compare.is_empty() {
        let sim = fuzzy_similarity(&title_to_compare, db_title);
        let score = sim * movie_weights::TITLE;
        breakdown.title = Some((sim, score));
        breakdown.total += score;
    }
    
    // Year comparison (prefer metadata, fall back to filename)
    let year = meta_year.or(file_info.year);
    let sim = year_match(year, db_year);
    if sim > 0.0 {
        let score = sim * movie_weights::YEAR;
        breakdown.year = Some((sim, score));
        breakdown.total += score;
    }
    
    // Director comparison (if available)
    if let (Some(meta_d), Some(db_d)) = (meta_director, db_director) {
        let sim = fuzzy_similarity(meta_d, db_d);
        let score = sim * movie_weights::DIRECTOR;
        breakdown.artist = Some((sim, score)); // Reuse artist field for director
        breakdown.total += score;
    }
    
    breakdown
}

/// Score an audiobook chapter match
pub fn score_audiobook_match(
    meta_author: Option<&str>,
    meta_book: Option<&str>,
    meta_chapter_title: Option<&str>,
    meta_chapter_number: Option<u32>,
    file_info: &ParsedFileInfo,
    source_name: Option<&str>,
    db_author: &str,
    db_book_title: &str,
    db_chapter_title: Option<&str>,
    db_chapter_number: i32,
) -> ScoreBreakdown {
    let mut breakdown = ScoreBreakdown::default();
    
    // Author comparison (artist tag in audiobooks)
    if let Some(author) = meta_author {
        let sim = fuzzy_similarity(author, db_author);
        let score = sim * audiobook_weights::AUTHOR;
        breakdown.artist = Some((sim, score));
        breakdown.total += score;
    }
    
    // Book title comparison (album tag in audiobooks, or source name)
    let book_to_compare = meta_book.or(source_name);
    if let Some(book) = book_to_compare {
        let sim = fuzzy_similarity(book, db_book_title);
        let score = sim * audiobook_weights::BOOK_TITLE;
        breakdown.album_or_show = Some((sim, score));
        breakdown.total += score;
    }
    
    // Chapter title comparison (title tag or filename)
    let chapter_title = meta_chapter_title.unwrap_or(&file_info.cleaned_title);
    if let Some(db_ct) = db_chapter_title {
        let sim = fuzzy_similarity(chapter_title, db_ct);
        let score = sim * audiobook_weights::CHAPTER_TITLE;
        breakdown.title = Some((sim, score));
        breakdown.total += score;
    }
    
    // Chapter number comparison (track number tag or filename)
    let chapter_num = meta_chapter_number.or(file_info.number);
    let sim = number_match(chapter_num, db_chapter_number);
    if sim > 0.0 {
        let score = sim * audiobook_weights::CHAPTER_NUMBER;
        breakdown.number = Some((sim, score));
        breakdown.total += score;
    }
    
    breakdown
}

// ============================================================================
// Threshold Constants
// ============================================================================

/// Minimum score to auto-link a file (high confidence)
pub const AUTO_LINK_THRESHOLD: f64 = 70.0;

/// Minimum score to suggest for manual review
pub const SUGGEST_THRESHOLD: f64 = 40.0;

/// Score is above auto-link threshold
pub fn is_auto_link(score: f64) -> bool {
    score >= AUTO_LINK_THRESHOLD
}

/// Score is in the uncertain range (suggest for review)
pub fn is_uncertain(score: f64) -> bool {
    score >= SUGGEST_THRESHOLD && score < AUTO_LINK_THRESHOLD
}

/// Score is too low to be a valid match
pub fn is_no_match(score: f64) -> bool {
    score < SUGGEST_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_track_info() {
        let info = parse_track_info("02 - It's So Easy.flac");
        assert_eq!(info.number, Some(2));
        assert_eq!(info.cleaned_title, "It's So Easy");
        
        let info2 = parse_track_info("01. Welcome to the Jungle.mp3");
        assert_eq!(info2.number, Some(1));
        assert_eq!(info2.cleaned_title, "Welcome to the Jungle");
        
        let info3 = parse_track_info("Track 05 - Paradise City.flac");
        assert_eq!(info3.number, Some(5));
        assert_eq!(info3.cleaned_title, "Paradise City");
    }
    
    #[test]
    fn test_parse_episode_info() {
        let info = parse_episode_info("Breaking Bad S01E05.mkv");
        assert_eq!(info.season, Some(1));
        assert_eq!(info.episode, Some(5));
        
        let info2 = parse_episode_info("The Office 2x03 - The Dundies.avi");
        assert_eq!(info2.season, Some(2));
        assert_eq!(info2.episode, Some(3));
    }
    
    #[test]
    fn test_parse_movie_info() {
        let info = parse_movie_info("The Matrix (1999).mkv");
        assert_eq!(info.year, Some(1999));
        assert!(info.cleaned_title.contains("Matrix"));
        
        let info2 = parse_movie_info("Inception [2010] 1080p BluRay.mkv");
        assert_eq!(info2.year, Some(2010));
    }
    
    #[test]
    fn test_fuzzy_artist_match() {
        // These should all match reasonably well
        let sim1 = fuzzy_similarity("Guns N' Roses", "Guns and Roses");
        let sim2 = fuzzy_similarity("Guns N' Roses", "Guns & Roses");
        let sim3 = fuzzy_similarity("Guns N' Roses", "Guns N Roses");
        
        assert!(sim1 > 0.7, "Expected > 0.7, got {}", sim1);
        assert!(sim2 > 0.7, "Expected > 0.7, got {}", sim2);
        assert!(sim3 > 0.8, "Expected > 0.8, got {}", sim3);
    }
    
    #[test]
    fn test_score_music_match() {
        let file_info = parse_track_info("02 - It's So Easy.flac");
        
        // Test matching against correct track (disc 1)
        let score = score_music_match(
            Some("Guns N' Roses"),
            Some("Appetite for Destruction"),
            Some("It's So Easy"),
            Some(2),
            Some(1), // disc 1
            Some(1987),
            &file_info,
            "Guns N' Roses",
            "Appetite for Destruction",
            "It's So Easy",
            2,
            1, // db disc 1
            Some(1987),
        );
        
        // Should be a very high score (near 100)
        assert!(score.total > 90.0, "Expected > 90, got {}", score.total);
    }
    
    #[test]
    fn test_disc_number_mismatch_prevents_track_number_score() {
        // File from CD2 should NOT match track on CD1
        let file_info = parse_track_info("2-01 - Some Song.flac");
        assert_eq!(file_info.disc_number, Some(2));
        assert_eq!(file_info.number, Some(1));
        
        // Try to match against track 1 from disc 1 - should get 0 for track number
        let score = score_music_match(
            Some("Artist"),
            Some("Album"),
            None, // different title
            Some(1), // meta track 1
            Some(2), // meta disc 2
            None,
            &file_info,
            "Artist",
            "Album",
            "Completely Different Song", // different track title
            1, // db track 1
            1, // db disc 1 - MISMATCH
            None,
        );
        
        // Should NOT get track number points due to disc mismatch
        assert_eq!(score.number, Some((0.0, 0.0)), "Disc mismatch should yield 0 track number points");
    }
    
    #[test]
    fn test_disc_number_match_awards_track_number_score() {
        // File from CD2 should match track on CD2
        let file_info = parse_track_info("2-01 - Some Song.flac");
        
        let score = score_music_match(
            Some("Artist"),
            Some("Album"),
            Some("Some Song"),
            Some(1), // meta track 1
            Some(2), // meta disc 2
            None,
            &file_info,
            "Artist",
            "Album",
            "Some Song",
            1, // db track 1
            2, // db disc 2 - MATCH
            None,
        );
        
        // Should get track number points since disc matches
        assert!(score.number.is_some());
        let (sim, points) = score.number.unwrap();
        assert_eq!(sim, 1.0, "Track number should match");
        assert!(points > 0.0, "Should get track number points when disc matches");
    }
    
    #[test]
    fn test_no_disc_info_matches_disc_1() {
        // File with no disc info should match disc 1 tracks
        let file_info = parse_track_info("01 - Welcome to the Jungle.flac");
        assert_eq!(file_info.disc_number, None);
        
        let score = score_music_match(
            None,
            None,
            Some("Welcome to the Jungle"),
            Some(1),
            None, // no disc in metadata
            None,
            &file_info,
            "Guns N' Roses",
            "Appetite for Destruction",
            "Welcome to the Jungle",
            1, // track 1
            1, // disc 1
            None,
        );
        
        // Should get track number points (file with no disc info defaults to disc 1)
        assert!(score.number.is_some());
        let (sim, points) = score.number.unwrap();
        assert_eq!(sim, 1.0, "Track number should match");
        assert!(points > 0.0, "Should get track number points when no disc info matches disc 1");
    }
    
    #[test]
    fn test_no_disc_info_does_not_match_disc_2() {
        // File with no disc info should NOT match disc 2 tracks
        let file_info = parse_track_info("01 - Welcome to the Jungle.flac");
        assert_eq!(file_info.disc_number, None);
        
        let score = score_music_match(
            None,
            None,
            Some("Some Bonus Track"),
            Some(1),
            None, // no disc in metadata
            None,
            &file_info,
            "Guns N' Roses",
            "Appetite for Destruction",
            "Some Bonus Track",
            1, // track 1
            2, // disc 2 - file has no disc, so shouldn't match disc 2
            None,
        );
        
        // Should NOT get track number points (file has no disc info but db track is disc 2)
        assert_eq!(score.number, Some((0.0, 0.0)), "No disc info should not match disc 2");
    }
    
    #[test]
    fn test_parse_track_info_with_disc() {
        // Test disc-track format
        let info = parse_track_info("2-01 - Track Title.flac");
        assert_eq!(info.disc_number, Some(2));
        assert_eq!(info.number, Some(1));
        assert_eq!(info.cleaned_title, "Track Title");
        
        // Test CD prefix format
        let info2 = parse_track_info("CD1 - 05 - Another Track.mp3");
        assert_eq!(info2.disc_number, Some(1));
        assert_eq!(info2.number, Some(5));
        
        // Test Disc prefix format
        let info3 = parse_track_info("Disc 2 - 03 - Track.flac");
        assert_eq!(info3.disc_number, Some(2));
    }
}
