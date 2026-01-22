//! File matcher service
//!
//! Source-agnostic file-to-library matching service.
//! This is THE ONLY place in the codebase where file matching logic exists.
//!
//! Used by:
//! - Torrent completion handlers
//! - Usenet completion handlers (future)
//! - Library scanners
//! - Manual file matching UI

use std::path::Path;

use anyhow::Result;
use rust_decimal::prelude::*;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::{
    CreatePendingFileMatch, Database, LibraryRecord, MatchTarget,
    PendingFileMatchRecord,
};
use crate::services::file_utils::{is_audio_file, is_video_file};
use crate::services::filename_parser::{self, ParsedEpisode, ParsedQuality};

// =========================================================================
// Embedded Metadata Types and Readers
// =========================================================================

/// Unified media metadata extracted from embedded tags
///
/// Works for both audio files (ID3/Vorbis via lofty) and video files (container tags via ffprobe).
#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    /// Title of the track/episode/movie
    pub title: Option<String>,
    /// Artist name (audio) or director (video)
    pub artist: Option<String>,
    /// Album name (audio) or show/series name (video)
    pub album: Option<String>,
    /// Release year
    pub year: Option<i32>,
    /// Track number (audio)
    pub track_number: Option<u32>,
    /// Disc number (audio)
    pub disc_number: Option<u32>,
    /// Season number (video - TV)
    pub season: Option<i32>,
    /// Episode number (video - TV)
    pub episode: Option<i32>,
}

impl MediaMetadata {
    /// Returns true if this metadata has useful matching info
    pub fn has_audio_tags(&self) -> bool {
        self.title.is_some() || self.album.is_some() || self.artist.is_some()
    }

    /// Returns true if this metadata has video matching info
    pub fn has_video_tags(&self) -> bool {
        self.title.is_some() || self.album.is_some() || (self.season.is_some() && self.episode.is_some())
    }
}

/// Stored metadata from database (mirrors EmbeddedMetadata from db)
/// Used for matching without re-reading from disk
#[derive(Debug, Clone, Default)]
pub struct StoredMetadata {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub title: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub show_name: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
}

impl StoredMetadata {
    /// Returns true if this has audio metadata useful for matching
    pub fn has_audio_tags(&self) -> bool {
        self.title.is_some() || self.album.is_some() || self.artist.is_some()
    }

    /// Returns true if this has video metadata useful for matching
    pub fn has_video_tags(&self) -> bool {
        self.title.is_some() || self.show_name.is_some() || (self.season.is_some() && self.episode.is_some())
    }
}

/// Read embedded metadata from an audio file using lofty
///
/// Fast (~1-5ms per file), reads only the tag header, not the full file.
/// Supports: FLAC, MP3, OGG, Opus, AAC, WAV, AIFF, APE, MPC, WavPack, etc.
pub fn read_audio_metadata(path: &str) -> Option<MediaMetadata> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let tagged_file = Probe::open(path).ok()?.read().ok()?;
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag())?;

    Some(MediaMetadata {
        title: tag.title().map(|s| s.to_string()),
        artist: tag.artist().map(|s| s.to_string()),
        album: tag.album().map(|s| s.to_string()),
        year: tag.year().map(|y| y as i32),
        track_number: tag.track(),
        disc_number: tag.disk(),
        ..Default::default()
    })
}

/// Read embedded metadata from a video file using ffprobe
///
/// Slower (~50-200ms per file) as it spawns a subprocess.
/// Supports: MKV, MP4, MOV, AVI, and other containers ffprobe understands.
pub async fn read_video_metadata(path: &str) -> Option<MediaMetadata> {
    use crate::services::ffmpeg::FfmpegService;

    let ffmpeg = FfmpegService::new();
    let analysis = ffmpeg.analyze(Path::new(path)).await.ok()?;

    // Extract from container metadata HashMap
    // Different containers use different tag names, so we try multiple variants
    let title = analysis.metadata.get("title")
        .or_else(|| analysis.metadata.get("TITLE"))
        .cloned();
    
    let artist = analysis.metadata.get("artist")
        .or_else(|| analysis.metadata.get("ARTIST"))
        .or_else(|| analysis.metadata.get("director"))
        .or_else(|| analysis.metadata.get("DIRECTOR"))
        .cloned();
    
    let album = analysis.metadata.get("album")
        .or_else(|| analysis.metadata.get("ALBUM"))
        .or_else(|| analysis.metadata.get("show"))
        .or_else(|| analysis.metadata.get("SHOW"))
        .or_else(|| analysis.metadata.get("series"))
        .or_else(|| analysis.metadata.get("SERIES"))
        .cloned();
    
    let year: Option<i32> = analysis.metadata.get("date")
        .or_else(|| analysis.metadata.get("DATE"))
        .or_else(|| analysis.metadata.get("year"))
        .or_else(|| analysis.metadata.get("YEAR"))
        .and_then(|s: &String| s.get(..4))
        .and_then(|s: &str| s.parse().ok());
    
    let season: Option<i32> = analysis.metadata.get("season")
        .or_else(|| analysis.metadata.get("SEASON"))
        .or_else(|| analysis.metadata.get("season_number"))
        .and_then(|s: &String| s.parse().ok());
    
    let episode: Option<i32> = analysis.metadata.get("episode")
        .or_else(|| analysis.metadata.get("EPISODE"))
        .or_else(|| analysis.metadata.get("episode_sort"))
        .or_else(|| analysis.metadata.get("episode_id"))
        .and_then(|s: &String| s.parse().ok());

    Some(MediaMetadata {
        title,
        artist,
        album,
        year,
        season,
        episode,
        ..Default::default()
    })
}

// =========================================================================
// File Matching Types
// =========================================================================

/// Information about a file to match
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub size: i64,
    /// For multi-file sources (torrents), the file index
    pub file_index: Option<i32>,
    /// Source name (e.g., torrent name) for context-aware matching
    pub source_name: Option<String>,
}

/// Result of matching a single file
#[derive(Debug, Clone)]
pub struct FileMatchResult {
    pub file_path: String,
    pub file_size: i64,
    pub file_index: Option<i32>,
    /// What type of match this is
    pub match_target: FileMatchTarget,
    /// How we matched it (auto/manual)
    pub match_type: FileMatchType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Parsed quality info from filename
    pub quality: ParsedQuality,
}

/// Known match target for when the library item is already identified
/// Used by auto-hunt/auto-download when we know what album/episode/movie we're downloading
#[derive(Debug, Clone)]
pub enum KnownMatchTarget {
    /// Match audio files to tracks within this album
    Album(Uuid),
    /// Match the largest video file to this episode
    Episode(Uuid),
    /// Match the largest video file to this movie
    Movie(Uuid),
    /// Match audio files to chapters within this audiobook
    Audiobook(Uuid),
}

/// What a file matched to
#[derive(Debug, Clone)]
pub enum FileMatchTarget {
    /// Matched to a TV episode
    Episode {
        episode_id: Uuid,
        show_id: Uuid,
        show_name: String,
        season: i32,
        episode: i32,
        library_id: Uuid,
    },
    /// Matched to a movie
    Movie {
        movie_id: Uuid,
        title: String,
        year: Option<i32>,
        library_id: Uuid,
    },
    /// Matched to a music track
    Track {
        track_id: Uuid,
        album_id: Uuid,
        title: String,
        track_number: i32,
        library_id: Uuid,
    },
    /// Matched to an audiobook chapter
    Chapter {
        chapter_id: Uuid,
        audiobook_id: Uuid,
        chapter_number: i32,
        library_id: Uuid,
    },
    /// No match found
    Unmatched { reason: String },
    /// Sample file - should not be processed
    Sample,
}

impl FileMatchTarget {
    /// Returns true if this is a successful match (not Unmatched or Sample)
    pub fn is_matched(&self) -> bool {
        !matches!(self, FileMatchTarget::Unmatched { .. } | FileMatchTarget::Sample)
    }

    /// Get the library ID if matched
    pub fn library_id(&self) -> Option<Uuid> {
        match self {
            FileMatchTarget::Episode { library_id, .. } => Some(*library_id),
            FileMatchTarget::Movie { library_id, .. } => Some(*library_id),
            FileMatchTarget::Track { library_id, .. } => Some(*library_id),
            FileMatchTarget::Chapter { library_id, .. } => Some(*library_id),
            _ => None,
        }
    }

    /// Convert to MatchTarget for database storage
    pub fn to_match_target(&self) -> Option<MatchTarget> {
        match self {
            FileMatchTarget::Episode { episode_id, .. } => Some(MatchTarget::Episode(*episode_id)),
            FileMatchTarget::Movie { movie_id, .. } => Some(MatchTarget::Movie(*movie_id)),
            FileMatchTarget::Track { track_id, .. } => Some(MatchTarget::Track(*track_id)),
            FileMatchTarget::Chapter { chapter_id, .. } => Some(MatchTarget::Chapter(*chapter_id)),
            _ => None,
        }
    }
}

/// How a file was matched
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMatchType {
    /// Automatically matched by filename parsing
    Auto,
    /// Manually linked by user
    Manual,
}

impl std::fmt::Display for FileMatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileMatchType::Auto => write!(f, "auto"),
            FileMatchType::Manual => write!(f, "manual"),
        }
    }
}

/// Summary of matching results
#[derive(Debug, Default)]
pub struct MatchSummary {
    pub total_files: usize,
    pub matched: usize,
    pub unmatched: usize,
    pub samples: usize,
    pub episodes: usize,
    pub movies: usize,
    pub tracks: usize,
    pub chapters: usize,
}

/// Result of verifying matches with embedded metadata
#[derive(Debug, Default)]
pub struct VerificationResult {
    /// Total matches checked
    pub total: usize,
    /// Matches that were verified as correct
    pub verified: usize,
    /// Matches that were auto-corrected (high confidence)
    pub corrected: usize,
    /// Matches flagged for manual review (low confidence correction)
    pub flagged: usize,
    /// Matches skipped (file not found, no metadata, etc.)
    pub skipped: usize,
}

/// Result of verifying a single match
#[derive(Debug)]
enum MatchVerification {
    /// Match is correct according to embedded metadata
    Verified,
    /// Match was wrong, auto-corrected to new target
    Corrected {
        new_target: MatchTarget,
        confidence: f64,
    },
    /// Mismatch detected but no high-confidence alternative found
    Flagged {
        reason: String,
    },
    /// No embedded metadata available to verify
    NoMetadata,
}

/// Source-agnostic file matcher service
///
/// This is THE ONLY place file matching logic should exist in the codebase.
pub struct FileMatcher {
    db: Database,
}

impl FileMatcher {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // =========================================================================
    // Main Public API
    // =========================================================================

    /// Match a single file to library items
    ///
    /// Returns all matching library items (can be multiple across different libraries).
    /// If `target_library_id` is provided, only searches that library.
    pub async fn match_file(
        &self,
        user_id: Uuid,
        file: &FileInfo,
        target_library_id: Option<Uuid>,
    ) -> Result<Vec<FileMatchResult>> {
        let libraries = self.get_libraries(user_id, target_library_id).await?;
        if libraries.is_empty() {
            return Ok(vec![FileMatchResult {
                file_path: file.path.clone(),
                file_size: file.size,
                file_index: file.file_index,
                match_target: FileMatchTarget::Unmatched {
                    reason: "No libraries found".to_string(),
                },
                match_type: FileMatchType::Auto,
                confidence: 0.0,
                quality: ParsedQuality::default(),
            }]);
        }

        self.match_file_against_libraries(file, &libraries).await
    }

    /// Match multiple files at once (batch operation)
    ///
    /// Returns results for all files. Each file may have multiple matches
    /// if it matches items in different libraries.
    pub async fn match_files(
        &self,
        user_id: Uuid,
        files: Vec<FileInfo>,
        target_library_id: Option<Uuid>,
    ) -> Result<Vec<FileMatchResult>> {
        let libraries = self.get_libraries(user_id, target_library_id).await?;
        if libraries.is_empty() {
            return Ok(files
                .into_iter()
                .map(|f| FileMatchResult {
                    file_path: f.path,
                    file_size: f.size,
                    file_index: f.file_index,
                    match_target: FileMatchTarget::Unmatched {
                        reason: "No libraries found".to_string(),
                    },
                    match_type: FileMatchType::Auto,
                    confidence: 0.0,
                    quality: ParsedQuality::default(),
                })
                .collect());
        }

        let mut results = Vec::new();
        for file in &files {
            let file_results = self.match_file_against_libraries(file, &libraries).await?;
            results.extend(file_results);
        }

        Ok(results)
    }

    /// Save match results to the database as pending file matches
    ///
    /// Only saves successful matches (not Unmatched or Sample).
    /// Returns the created records.
    pub async fn save_matches(
        &self,
        user_id: Uuid,
        source_type: &str,
        source_id: Option<Uuid>,
        matches: &[FileMatchResult],
    ) -> Result<Vec<PendingFileMatchRecord>> {
        let mut records = Vec::new();

        for m in matches {
            if let Some(target) = m.match_target.to_match_target() {
                let input = CreatePendingFileMatch {
                    user_id,
                    source_path: m.file_path.clone(),
                    source_type: source_type.to_string(),
                    source_id,
                    source_file_index: m.file_index,
                    file_size: m.file_size,
                    target,
                    match_type: m.match_type.to_string(),
                    match_confidence: Some(Decimal::from_f64(m.confidence).unwrap_or_default()),
                    parsed_resolution: m.quality.resolution.clone(),
                    parsed_codec: m.quality.codec.clone(),
                    parsed_source: m.quality.source.clone(),
                    parsed_audio: m.quality.audio.clone(),
                };

                match self.db.pending_file_matches().create(input).await {
                    Ok(record) => {
                        // Status is derived from pending_file_matches - no direct update needed
                        records.push(record);
                    }
                    Err(e) => {
                        warn!(
                            file_path = %m.file_path,
                            error = %e,
                            "Failed to save pending file match"
                        );
                    }
                }
            }
        }

        if !records.is_empty() {
            info!(
                source_type = %source_type,
                source_id = ?source_id,
                matches_saved = records.len(),
                "Saved pending file matches"
            );
        }

        Ok(records)
    }

    /// Get a summary of match results
    pub fn summarize_matches(matches: &[FileMatchResult]) -> MatchSummary {
        let mut summary = MatchSummary {
            total_files: matches.len(),
            ..Default::default()
        };

        for m in matches {
            match &m.match_target {
                FileMatchTarget::Episode { .. } => {
                    summary.matched += 1;
                    summary.episodes += 1;
                }
                FileMatchTarget::Movie { .. } => {
                    summary.matched += 1;
                    summary.movies += 1;
                }
                FileMatchTarget::Track { .. } => {
                    summary.matched += 1;
                    summary.tracks += 1;
                }
                FileMatchTarget::Chapter { .. } => {
                    summary.matched += 1;
                    summary.chapters += 1;
                }
                FileMatchTarget::Unmatched { .. } => {
                    summary.unmatched += 1;
                }
                FileMatchTarget::Sample => {
                    summary.samples += 1;
                }
            }
        }

        summary
    }

    // =========================================================================
    // Match Using Stored Metadata (for scanner re-matching)
    // =========================================================================

    /// Match an existing media_file record using its stored embedded metadata
    ///
    /// This does NOT read from disk - it uses metadata already stored in the database.
    /// This allows re-matching without re-extraction.
    ///
    /// Matching priority:
    /// 1. Embedded metadata (ID3/Vorbis tags stored in meta_* fields)
    /// 2. Original filename (if different from current - useful after incorrect renames)
    /// 3. Current filename (fallback)
    ///
    /// IMPORTANT: This function only matches within the specified library.
    /// For files without a library, use a different entry point that searches all libraries.
    ///
    /// Returns the match result which can be used to update the media_file's links.
    pub async fn match_media_file(
        &self,
        media_file: &crate::db::MediaFileRecord,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<FileMatchResult> {
        let file_path = &media_file.path;
        let current_filename = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);

        // Check if we have an original filename that differs from current
        // (useful when file was incorrectly renamed)
        let original_filename = media_file.original_name.as_deref();
        let has_different_original = original_filename
            .map(|orig| orig != current_filename)
            .unwrap_or(false);

        // Build stored metadata from the media_file record
        let stored_meta = StoredMetadata {
            artist: media_file.meta_artist.clone(),
            album: media_file.meta_album.clone(),
            title: media_file.meta_title.clone(),
            track_number: media_file.meta_track_number.map(|n| n as u32),
            disc_number: media_file.meta_disc_number.map(|n| n as u32),
            year: media_file.meta_year,
            genre: media_file.meta_genre.clone(),
            show_name: media_file.meta_show_name.clone(),
            season: media_file.meta_season,
            episode: media_file.meta_episode,
        };

        let has_metadata = stored_meta.has_audio_tags() || stored_meta.has_video_tags();
        let _quality = filename_parser::parse_quality(file_path);

        // Match based on library type
        let library_type = library.library_type.to_lowercase();
        
        match library_type.as_str() {
            "music" => {
                // Priority 1: Try metadata-first matching
                if has_metadata && stored_meta.has_audio_tags() {
                    if let Some(result) = self
                        .try_match_track_by_stored_metadata(&stored_meta, library, file_path, media_file.size_bytes)
                        .await?
                    {
                        info!(
                            path = %file_path,
                            "Matched via embedded metadata"
                        );
                        return Ok(result);
                    }
                }
                
                // Priority 2: Try original filename if different from current
                if has_different_original {
                    let orig_name = original_filename.unwrap();
                    debug!(
                        current = %current_filename,
                        original = %orig_name,
                        "Trying match with original filename"
                    );
                    let file_info = FileInfo {
                        path: file_path.to_string(),
                        size: media_file.size_bytes,
                        file_index: None,
                        source_name: None,
                    };
                    let results = self.match_audio_file(&file_info, orig_name, &[library.clone()]).await?;
                    if let Some(result) = results.into_iter().next() {
                        if result.match_target.is_matched() {
                            info!(
                                path = %file_path,
                                original = %orig_name,
                                "Matched via original filename"
                            );
                            return Ok(result);
                        }
                    }
                }
                
                // Priority 3: Fall back to current filename matching
                let file_info = FileInfo {
                    path: file_path.to_string(),
                    size: media_file.size_bytes,
                    file_index: None,
                    source_name: None,
                };
                let results = self.match_audio_file(&file_info, current_filename, &[library.clone()]).await?;
                results.into_iter().next().ok_or_else(|| anyhow::anyhow!("No match result"))
            }
            "tv" | "tv shows" | "television" => {
                // Priority 1: Try metadata-first matching
                if has_metadata && stored_meta.has_video_tags() {
                    if let Some(result) = self
                        .try_match_episode_by_stored_metadata(&stored_meta, library, file_path, media_file.size_bytes)
                        .await?
                    {
                        info!(
                            path = %file_path,
                            "Matched via embedded metadata"
                        );
                        return Ok(result);
                    }
                }
                
                // Priority 2: Try original filename if different from current
                if has_different_original {
                    let orig_name = original_filename.unwrap();
                    debug!(
                        current = %current_filename,
                        original = %orig_name,
                        "Trying match with original filename"
                    );
                    let file_info = FileInfo {
                        path: file_path.to_string(),
                        size: media_file.size_bytes,
                        file_index: None,
                        source_name: None,
                    };
                    let results = self.match_video_file(&file_info, orig_name, &[library.clone()]).await?;
                    if let Some(result) = results.into_iter().next() {
                        if result.match_target.is_matched() {
                            info!(
                                path = %file_path,
                                original = %orig_name,
                                "Matched via original filename"
                            );
                            return Ok(result);
                        }
                    }
                }
                
                // Priority 3: Fall back to current filename matching
                let file_info = FileInfo {
                    path: file_path.to_string(),
                    size: media_file.size_bytes,
                    file_index: None,
                    source_name: None,
                };
                let results = self.match_video_file(&file_info, current_filename, &[library.clone()]).await?;
                results.into_iter().next().ok_or_else(|| anyhow::anyhow!("No match result"))
            }
            "movies" => {
                // Priority 1: Try metadata-first matching
                if has_metadata && stored_meta.has_video_tags() {
                    if let Some(result) = self
                        .try_match_movie_by_stored_metadata(&stored_meta, library, file_path, media_file.size_bytes)
                        .await?
                    {
                        info!(
                            path = %file_path,
                            "Matched via embedded metadata"
                        );
                        return Ok(result);
                    }
                }
                
                // Priority 2: Try original filename if different from current
                if has_different_original {
                    let orig_name = original_filename.unwrap();
                    debug!(
                        current = %current_filename,
                        original = %orig_name,
                        "Trying match with original filename"
                    );
                    let file_info = FileInfo {
                        path: file_path.to_string(),
                        size: media_file.size_bytes,
                        file_index: None,
                        source_name: None,
                    };
                    let results = self.match_video_file(&file_info, orig_name, &[library.clone()]).await?;
                    if let Some(result) = results.into_iter().next() {
                        if result.match_target.is_matched() {
                            info!(
                                path = %file_path,
                                original = %orig_name,
                                "Matched via original filename"
                            );
                            return Ok(result);
                        }
                    }
                }
                
                // Priority 3: Fall back to current filename matching
                let file_info = FileInfo {
                    path: file_path.to_string(),
                    size: media_file.size_bytes,
                    file_index: None,
                    source_name: None,
                };
                let results = self.match_video_file(&file_info, current_filename, &[library.clone()]).await?;
                results.into_iter().next().ok_or_else(|| anyhow::anyhow!("No match result"))
            }
            "audiobooks" => {
                // Priority 1: Try metadata-first matching
                if has_metadata && stored_meta.has_audio_tags() {
                    if let Some(result) = self
                        .try_match_chapter_by_stored_metadata(&stored_meta, library, file_path, media_file.size_bytes)
                        .await?
                    {
                        info!(
                            path = %file_path,
                            "Matched via embedded metadata"
                        );
                        return Ok(result);
                    }
                }
                
                // Priority 2: Try original filename if different from current
                if has_different_original {
                    let orig_name = original_filename.unwrap();
                    debug!(
                        current = %current_filename,
                        original = %orig_name,
                        "Trying match with original filename"
                    );
                    let file_info = FileInfo {
                        path: file_path.to_string(),
                        size: media_file.size_bytes,
                        file_index: None,
                        source_name: None,
                    };
                    let results = self.match_audio_file(&file_info, orig_name, &[library.clone()]).await?;
                    if let Some(result) = results.into_iter().next() {
                        if result.match_target.is_matched() {
                            info!(
                                path = %file_path,
                                original = %orig_name,
                                "Matched via original filename"
                            );
                            return Ok(result);
                        }
                    }
                }
                
                // Priority 3: Fall back to current filename matching
                let file_info = FileInfo {
                    path: file_path.to_string(),
                    size: media_file.size_bytes,
                    file_index: None,
                    source_name: None,
                };
                let results = self.match_audio_file(&file_info, current_filename, &[library.clone()]).await?;
                results.into_iter().next().ok_or_else(|| anyhow::anyhow!("No match result"))
            }
            _ => {
                let quality = filename_parser::parse_quality(file_path);
                Ok(FileMatchResult {
                    file_path: file_path.to_string(),
                    file_size: media_file.size_bytes,
                    file_index: None,
                    match_target: FileMatchTarget::Unmatched {
                        reason: format!("Unknown library type: {}", library_type),
                    },
                    match_type: FileMatchType::Auto,
                    confidence: 0.0,
                    quality,
                })
            }
        }
    }

    /// Try to match a track using stored metadata (not reading from file)
    async fn try_match_track_by_stored_metadata(
        &self,
        meta: &StoredMetadata,
        library: &crate::db::libraries::LibraryRecord,
        file_path: &str,
        file_size: i64,
    ) -> Result<Option<FileMatchResult>> {
        use super::match_scorer::{score_music_match, parse_track_info, AUTO_LINK_THRESHOLD};
        
        let quality = filename_parser::parse_quality(file_path);

        // Need at least album to match
        let meta_album = match &meta.album {
            Some(a) if !a.is_empty() => Some(a.as_str()),
            _ => None,
        };
        
        // If no metadata, can't match by metadata
        if meta_album.is_none() && meta.artist.is_none() && meta.title.is_none() {
            return Ok(None);
        }

        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);
        let file_info = parse_track_info(file_name);

        // Get all albums in the library and score each track
        let albums = self.db.albums().list_by_library(library.id).await?;
        
        // Store best match by value (IDs + cloned strings) to avoid lifetime issues
        struct BestMatch {
            track_id: Uuid,
            album_id: Uuid,
            track_title: String,
            track_number: i32,
            album_name: String,
            artist_name: String,
            score: f64,
        }
        let mut best_match: Option<BestMatch> = None;

        for album in &albums {
            // Get artist for this album
            let artist_name = self.db.albums().get_artist_by_id(album.artist_id).await?
                .map(|a| a.name)
                .unwrap_or_default();

            // Get tracks for this album
            let tracks = self.db.tracks().list_by_album(album.id).await?;

            for track in &tracks {
                // Use the weighted scorer
                let breakdown = score_music_match(
                    meta.artist.as_deref(),
                    meta_album,
                    meta.title.as_deref(),
                    meta.track_number.map(|n| n as u32),
                    meta.disc_number.map(|n| n as u32),
                    meta.year,
                    &file_info,
                    &artist_name,
                    &album.name,
                    &track.title,
                    track.track_number,
                    track.disc_number,
                    album.year,
                );

                if breakdown.total > best_match.as_ref().map(|m| m.score).unwrap_or(0.0) {
                    best_match = Some(BestMatch {
                        track_id: track.id,
                        album_id: album.id,
                        track_title: track.title.clone(),
                        track_number: track.track_number,
                        album_name: album.name.clone(),
                        artist_name: artist_name.clone(),
                        score: breakdown.total,
                    });
                }
            }
        }

        // Check if we found a match above the threshold
        if let Some(m) = best_match {
            if m.score >= AUTO_LINK_THRESHOLD {
                info!(
                    "Matched '{}' to '{}' - '{}' by '{}' (score: {:.1})",
                    file_name, m.album_name, m.track_title, m.artist_name, m.score
                );

                let confidence = (m.score / 100.0).min(1.0);
                return Ok(Some(FileMatchResult {
                    file_path: file_path.to_string(),
                    file_size,
                    file_index: None,
                    match_target: FileMatchTarget::Track {
                        track_id: m.track_id,
                        album_id: m.album_id,
                        title: m.track_title,
                        track_number: m.track_number,
                        library_id: library.id,
                    },
                    match_type: FileMatchType::Auto,
                    confidence,
                    quality,
                }));
            } else {
                debug!(
                    "Best match for '{}' was '{}' - '{}' with score {:.1}, below threshold {}",
                    file_name, m.album_name, m.track_title, m.score, AUTO_LINK_THRESHOLD
                );
            }
        }

        Ok(None)
    }


    /// Try to match an episode using stored metadata
    async fn try_match_episode_by_stored_metadata(
        &self,
        meta: &StoredMetadata,
        library: &crate::db::libraries::LibraryRecord,
        file_path: &str,
        file_size: i64,
    ) -> Result<Option<FileMatchResult>> {
        let quality = filename_parser::parse_quality(file_path);

        // Need show name and season/episode to match
        let meta_show = match &meta.show_name {
            Some(s) if !s.is_empty() => s.as_str(),
            _ => return Ok(None),
        };
        let meta_season = match meta.season {
            Some(s) => s,
            None => return Ok(None),
        };
        let meta_episode = match meta.episode {
            Some(e) => e,
            None => return Ok(None),
        };

        // Get all shows in the library
        let shows = self.db.tv_shows().list_by_library(library.id).await?;

        // Find best matching show
        let mut best_show: Option<(&crate::db::tv_shows::TvShowRecord, f64)> = None;
        for show in &shows {
            let similarity = filename_parser::show_name_similarity(&show.name, meta_show);
            if similarity >= 0.7 {
                if best_show.is_none() || similarity > best_show.unwrap().1 {
                    best_show = Some((show, similarity));
                }
            }
        }

        let (show, show_similarity) = match best_show {
            Some(s) => s,
            None => return Ok(None),
        };

        // Find episode by season/episode number
        let episode = self
            .db
            .episodes()
            .get_by_show_season_episode(show.id, meta_season, meta_episode)
            .await?;

        if let Some(ep) = episode {
            let confidence = show_similarity * 0.95; // High confidence when we have exact S/E match
            
            info!(
                file = %file_path,
                meta_show = %meta_show,
                meta_se = %format!("S{:02}E{:02}", meta_season, meta_episode),
                matched_show = %show.name,
                confidence = confidence,
                "Matched episode using stored metadata"
            );

            return Ok(Some(FileMatchResult {
                file_path: file_path.to_string(),
                file_size,
                file_index: None,
                match_target: FileMatchTarget::Episode {
                    episode_id: ep.id,
                    show_id: show.id,
                    show_name: show.name.clone(),
                    season: meta_season,
                    episode: meta_episode,
                    library_id: library.id,
                },
                match_type: FileMatchType::Auto,
                confidence,
                quality,
            }));
        }

        Ok(None)
    }

    /// Try to match a movie using stored metadata
    async fn try_match_movie_by_stored_metadata(
        &self,
        meta: &StoredMetadata,
        library: &crate::db::libraries::LibraryRecord,
        file_path: &str,
        file_size: i64,
    ) -> Result<Option<FileMatchResult>> {
        let quality = filename_parser::parse_quality(file_path);

        // Need title to match movies
        let meta_title = match &meta.title {
            Some(t) if !t.is_empty() => t.as_str(),
            _ => return Ok(None),
        };

        // Get all movies in the library
        let movies = self.db.movies().list_by_library(library.id).await?;

        // Find best matching movie
        let mut best_movie: Option<(&crate::db::movies::MovieRecord, f64)> = None;
        for movie in &movies {
            let similarity = filename_parser::show_name_similarity(&movie.title, meta_title);
            
            // Boost if year matches
            let year_boost = if let (Some(meta_year), Some(movie_year)) = (meta.year, movie.year) {
                if meta_year == movie_year { 0.1 } else { 0.0 }
            } else {
                0.0
            };

            let total_sim = (similarity + year_boost).min(1.0);
            if total_sim >= 0.7 {
                if best_movie.is_none() || total_sim > best_movie.unwrap().1 {
                    best_movie = Some((movie, total_sim));
                }
            }
        }

        if let Some((movie, confidence)) = best_movie {
            info!(
                file = %file_path,
                meta_title = %meta_title,
                matched_movie = %movie.title,
                confidence = confidence,
                "Matched movie using stored metadata"
            );

            return Ok(Some(FileMatchResult {
                file_path: file_path.to_string(),
                file_size,
                file_index: None,
                match_target: FileMatchTarget::Movie {
                    movie_id: movie.id,
                    title: movie.title.clone(),
                    year: movie.year,
                    library_id: library.id,
                },
                match_type: FileMatchType::Auto,
                confidence,
                quality,
            }));
        }

        Ok(None)
    }

    /// Try to match an audiobook chapter using stored metadata
    async fn try_match_chapter_by_stored_metadata(
        &self,
        meta: &StoredMetadata,
        library: &crate::db::libraries::LibraryRecord,
        file_path: &str,
        file_size: i64,
    ) -> Result<Option<FileMatchResult>> {
        let quality = filename_parser::parse_quality(file_path);

        // Need album (book title) to match
        let meta_album = match &meta.album {
            Some(a) if !a.is_empty() => a.as_str(),
            _ => return Ok(None),
        };

        // Get all audiobooks in the library
        let audiobooks = self.db.audiobooks().list_by_library(library.id).await?;

        // Find best matching audiobook
        let mut best_book: Option<(&crate::db::audiobooks::AudiobookRecord, f64)> = None;
        for book in &audiobooks {
            let similarity = filename_parser::show_name_similarity(&book.title, meta_album);
            if similarity >= 0.7 {
                if best_book.is_none() || similarity > best_book.unwrap().1 {
                    best_book = Some((book, similarity));
                }
            }
        }

        let (book, book_similarity) = match best_book {
            Some(b) => b,
            None => return Ok(None),
        };

        // Get chapters and try to match by track number
        let chapters = self.db.chapters().list_by_audiobook(book.id).await?;

        if let Some(meta_track) = meta.track_number {
            if let Some(chapter) = chapters.iter().find(|c| c.chapter_number == meta_track as i32) {
                let confidence = book_similarity * 0.9;
                
                info!(
                    file = %file_path,
                    meta_album = %meta_album,
                    meta_track = meta_track,
                    matched_book = %book.title,
                    chapter_number = chapter.chapter_number,
                    confidence = confidence,
                    "Matched chapter using stored metadata"
                );

                return Ok(Some(FileMatchResult {
                    file_path: file_path.to_string(),
                    file_size,
                    file_index: None,
                    match_target: FileMatchTarget::Chapter {
                        chapter_id: chapter.id,
                        audiobook_id: book.id,
                        chapter_number: chapter.chapter_number,
                        library_id: library.id,
                    },
                    match_type: FileMatchType::Auto,
                    confidence,
                    quality,
                }));
            }
        }

        Ok(None)
    }

    // =========================================================================
    // Known Target Matching (for auto-hunt/auto-download)
    // =========================================================================

    /// Create matches for files when the target library item is already known
    ///
    /// This is used by auto-hunt and auto-download when we already know which
    /// album/episode/movie/audiobook the files belong to. It:
    /// - Clears any existing matches for this source first
    /// - For albums: Fuzzy-matches audio files to tracks within that album
    /// - For episodes/movies: Matches the largest video file
    /// - For audiobooks: Fuzzy-matches audio files to chapters
    ///
    /// Returns the created match records.
    pub async fn create_matches_for_target(
        &self,
        user_id: Uuid,
        source_type: &str,
        source_id: Uuid,
        files: Vec<FileInfo>,
        target: KnownMatchTarget,
    ) -> Result<Vec<PendingFileMatchRecord>> {
        // Clear any existing matches for this source first to avoid duplicates
        let deleted = self
            .db
            .pending_file_matches()
            .delete_by_source(source_type, source_id)
            .await
            .unwrap_or(0);

        if deleted > 0 {
            debug!(
                source_type = %source_type,
                source_id = %source_id,
                deleted = deleted,
                "Cleared existing pending matches before creating new ones"
            );
        }

        match target {
            KnownMatchTarget::Album(album_id) => {
                self.create_matches_for_album(user_id, source_type, source_id, files, album_id)
                    .await
            }
            KnownMatchTarget::Episode(episode_id) => {
                self.create_matches_for_episode(user_id, source_type, source_id, files, episode_id)
                    .await
            }
            KnownMatchTarget::Movie(movie_id) => {
                self.create_matches_for_movie(user_id, source_type, source_id, files, movie_id)
                    .await
            }
            KnownMatchTarget::Audiobook(audiobook_id) => {
                self.create_matches_for_audiobook(user_id, source_type, source_id, files, audiobook_id)
                    .await
            }
        }
    }

    /// Create matches for an album - match audio files to tracks
    async fn create_matches_for_album(
        &self,
        user_id: Uuid,
        source_type: &str,
        source_id: Uuid,
        files: Vec<FileInfo>,
        album_id: Uuid,
    ) -> Result<Vec<PendingFileMatchRecord>> {
        let mut records = Vec::new();

        // Get the album and its library
        let album = self.db.albums().get_by_id(album_id).await?
            .ok_or_else(|| anyhow::anyhow!("Album not found: {}", album_id))?;

        // Get tracks for this album
        let tracks = self.db.tracks().list_by_album(album_id).await?;

        // Filter to audio files only
        let audio_files: Vec<_> = files
            .iter()
            .filter(|f| is_audio_file(&f.path))
            .collect();

        info!(
            source_type = %source_type,
            source_id = %source_id,
            "Matching {} audio files to {} tracks for album '{}'",
            audio_files.len(),
            tracks.len(),
            album.name
        );

        // Match each audio file to a track using fuzzy matching
        for file in &audio_files {
            let file_name = Path::new(&file.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file.path);

            // Find best matching track (only match to tracks without files)
            let mut best_match: Option<(Uuid, f64, &str)> = None;
            for track in &tracks {
                // Skip tracks that already have a linked media file
                if track.media_file_id.is_some() {
                    continue;
                }

                let similarity = self.calculate_track_similarity(file_name, &track.title);
                if similarity >= 0.5 {
                    if best_match.is_none() || similarity > best_match.unwrap().1 {
                        best_match = Some((track.id, similarity, &track.title));
                    }
                }
            }

            if let Some((track_id, confidence, track_title)) = best_match {
                let quality = filename_parser::parse_quality(&file.path);

                let input = CreatePendingFileMatch {
                    user_id,
                    source_path: file.path.clone(),
                    source_type: source_type.to_string(),
                    source_id: Some(source_id),
                    source_file_index: file.file_index,
                    file_size: file.size,
                    target: MatchTarget::Track(track_id),
                    match_type: "auto".to_string(),
                    match_confidence: Some(Decimal::from_f64(confidence).unwrap_or_default()),
                    parsed_resolution: quality.resolution,
                    parsed_codec: quality.codec,
                    parsed_source: quality.source,
                    parsed_audio: quality.audio,
                };

                match self.db.pending_file_matches().create(input).await {
                    Ok(record) => {
                        debug!(
                            file = %file_name,
                            track = %track_title,
                            confidence = confidence,
                            "Matched file to track"
                        );
                        records.push(record);
                    }
                    Err(e) => {
                        warn!(
                            file = %file_name,
                            error = %e,
                            "Failed to create pending file match for track"
                        );
                    }
                }
            } else {
                debug!(
                    file = %file_name,
                    "No matching track found for audio file"
                );
            }
        }

        info!(
            source_type = %source_type,
            source_id = %source_id,
            "Created {} file-level matches for album '{}'",
            records.len(),
            album.name
        );

        Ok(records)
    }

    /// Create matches for an episode - match largest video file
    async fn create_matches_for_episode(
        &self,
        user_id: Uuid,
        source_type: &str,
        source_id: Uuid,
        files: Vec<FileInfo>,
        episode_id: Uuid,
    ) -> Result<Vec<PendingFileMatchRecord>> {
        let mut records = Vec::new();

        // Get the episode
        let episode = self.db.episodes().get_by_id(episode_id).await?
            .ok_or_else(|| anyhow::anyhow!("Episode not found: {}", episode_id))?;

        // Filter to video files and find the largest
        let video_files: Vec<_> = files
            .iter()
            .filter(|f| is_video_file(&f.path) && !self.is_sample_file(&f.path))
            .collect();

        let largest_video = video_files.iter().max_by_key(|f| f.size);

        if let Some(file) = largest_video {
            let quality = filename_parser::parse_quality(&file.path);

            let input = CreatePendingFileMatch {
                user_id,
                source_path: file.path.clone(),
                source_type: source_type.to_string(),
                source_id: Some(source_id),
                source_file_index: file.file_index,
                file_size: file.size,
                target: MatchTarget::Episode(episode_id),
                match_type: "auto".to_string(),
                match_confidence: Some(Decimal::from(1)), // Known target = 100% confidence
                parsed_resolution: quality.resolution,
                parsed_codec: quality.codec,
                parsed_source: quality.source,
                parsed_audio: quality.audio,
            };

            match self.db.pending_file_matches().create(input).await {
                Ok(record) => {
                    // Episode status is now computed from media_file_id - no need to update
                    records.push(record);
                    
                    info!(
                        episode_id = %episode_id,
                        season = episode.season,
                        episode = episode.episode,
                        file = %file.path,
                        "Created match for episode"
                    );
                }
                Err(e) => {
                    warn!(
                        episode_id = %episode_id,
                        error = %e,
                        "Failed to create pending file match for episode"
                    );
                }
            }
        } else {
            warn!(
                episode_id = %episode_id,
                video_files = video_files.len(),
                "No video files found for episode"
            );
        }

        Ok(records)
    }

    /// Create matches for a movie - match largest video file
    async fn create_matches_for_movie(
        &self,
        user_id: Uuid,
        source_type: &str,
        source_id: Uuid,
        files: Vec<FileInfo>,
        movie_id: Uuid,
    ) -> Result<Vec<PendingFileMatchRecord>> {
        let mut records = Vec::new();

        // Get the movie
        let movie = self.db.movies().get_by_id(movie_id).await?
            .ok_or_else(|| anyhow::anyhow!("Movie not found: {}", movie_id))?;

        // Filter to video files and find the largest (exclude samples)
        let video_files: Vec<_> = files
            .iter()
            .filter(|f| is_video_file(&f.path) && !self.is_sample_file(&f.path))
            .collect();

        let largest_video = video_files.iter().max_by_key(|f| f.size);

        if let Some(file) = largest_video {
            let quality = filename_parser::parse_quality(&file.path);

            let input = CreatePendingFileMatch {
                user_id,
                source_path: file.path.clone(),
                source_type: source_type.to_string(),
                source_id: Some(source_id),
                source_file_index: file.file_index,
                file_size: file.size,
                target: MatchTarget::Movie(movie_id),
                match_type: "auto".to_string(),
                match_confidence: Some(Decimal::from(1)), // Known target = 100% confidence
                parsed_resolution: quality.resolution,
                parsed_codec: quality.codec,
                parsed_source: quality.source,
                parsed_audio: quality.audio,
            };

                match self.db.pending_file_matches().create(input).await {
                Ok(record) => {
                    // Status is derived from pending_file_matches - no direct update needed
                    records.push(record);
                    
                    info!(
                        movie_id = %movie_id,
                        title = %movie.title,
                        file = %file.path,
                        "Created match for movie"
                    );
                }
                Err(e) => {
                    warn!(
                        movie_id = %movie_id,
                        error = %e,
                        "Failed to create pending file match for movie"
                    );
                }
            }
        } else {
            warn!(
                movie_id = %movie_id,
                video_files = video_files.len(),
                "No video files found for movie"
            );
        }

        Ok(records)
    }

    /// Create matches for an audiobook - match audio files to chapters
    async fn create_matches_for_audiobook(
        &self,
        user_id: Uuid,
        source_type: &str,
        source_id: Uuid,
        files: Vec<FileInfo>,
        audiobook_id: Uuid,
    ) -> Result<Vec<PendingFileMatchRecord>> {
        let mut records = Vec::new();

        // Get the audiobook
        let audiobook = self.db.audiobooks().get_by_id(audiobook_id).await?
            .ok_or_else(|| anyhow::anyhow!("Audiobook not found: {}", audiobook_id))?;

        // Get chapters for this audiobook
        let chapters = self.db.chapters().list_by_audiobook(audiobook_id).await?;

        // Filter to audio files only
        let audio_files: Vec<_> = files
            .iter()
            .filter(|f| is_audio_file(&f.path))
            .collect();

        info!(
            source_type = %source_type,
            source_id = %source_id,
            "Matching {} audio files to {} chapters for audiobook '{}'",
            audio_files.len(),
            chapters.len(),
            audiobook.title
        );

        // Match by chapter number from filename
        for file in &audio_files {
            let file_name = Path::new(&file.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file.path);

            // Try to extract chapter number from filename
            if let Some(chapter_num) = self.extract_chapter_number(file_name) {
                if let Some(chapter) = chapters.iter().find(|c| c.chapter_number == chapter_num) {
                    // Skip chapters that already have a media file
                    if chapter.media_file_id.is_some() {
                        continue;
                    }

                    let quality = filename_parser::parse_quality(&file.path);

                    let input = CreatePendingFileMatch {
                        user_id,
                        source_path: file.path.clone(),
                        source_type: source_type.to_string(),
                        source_id: Some(source_id),
                        source_file_index: file.file_index,
                        file_size: file.size,
                        target: MatchTarget::Chapter(chapter.id),
                        match_type: "auto".to_string(),
                        match_confidence: Some(Decimal::from_f64(0.9).unwrap_or_default()),
                        parsed_resolution: quality.resolution,
                        parsed_codec: quality.codec,
                        parsed_source: quality.source,
                        parsed_audio: quality.audio,
                    };

                    match self.db.pending_file_matches().create(input).await {
                        Ok(record) => {
                            records.push(record);
                        }
                        Err(e) => {
                            warn!(
                                file = %file_name,
                                error = %e,
                                "Failed to create pending file match for chapter"
                            );
                        }
                    }
                }
            }
        }

        info!(
            source_type = %source_type,
            source_id = %source_id,
            audiobook = %audiobook.title,
            matches_created = records.len(),
            "Created file-level matches for audiobook"
        );

        Ok(records)
    }

    /// Verify existing matches using embedded metadata (call after download completes)
    ///
    /// This function reads actual file tags and compares them with the current matches.
    /// - Auto-corrects if a better match is found with confidence > 0.9
    /// - Flags for review if mismatch detected but no high-confidence alternative
    ///
    /// Returns a summary of verification results.
    pub async fn verify_matches_with_metadata(
        &self,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<VerificationResult> {
        let mut result = VerificationResult::default();

        // Get all uncopied matches for this source
        let matches = self
            .db
            .pending_file_matches()
            .list_uncopied_by_source(source_type, source_id)
            .await?;

        if matches.is_empty() {
            return Ok(result);
        }

        info!(
            source_type = %source_type,
            source_id = %source_id,
            match_count = matches.len(),
            "Verifying matches with embedded metadata"
        );

        for pending_match in matches {
            result.total += 1;

            // Check if file exists
            if !Path::new(&pending_match.source_path).exists() {
                debug!(
                    path = %pending_match.source_path,
                    "File not found, skipping verification"
                );
                result.skipped += 1;
                continue;
            }

            // Read metadata based on file type
            let is_audio = is_audio_file(&pending_match.source_path);
            let is_video = is_video_file(&pending_match.source_path);

            if !is_audio && !is_video {
                result.skipped += 1;
                continue;
            }

            // Verify the match
            let verification = if is_audio {
                self.verify_audio_match(&pending_match).await?
            } else {
                self.verify_video_match(&pending_match).await?
            };

            match verification {
                MatchVerification::Verified => {
                    result.verified += 1;
                }
                MatchVerification::Corrected { new_target, confidence } => {
                    // Update the pending match with the correct target
                    if let Err(e) = self.db.pending_file_matches()
                        .update_target(pending_match.id, new_target)
                        .await
                    {
                        warn!(
                            match_id = %pending_match.id,
                            error = %e,
                            "Failed to update corrected match"
                        );
                    } else {
                        info!(
                            path = %pending_match.source_path,
                            confidence = confidence,
                            "Auto-corrected match using embedded metadata"
                        );
                        result.corrected += 1;
                    }
                }
                MatchVerification::Flagged { reason } => {
                    warn!(
                        path = %pending_match.source_path,
                        reason = %reason,
                        "Match flagged for review"
                    );
                    // TODO: When db migration adds verification_status column, update it here
                    result.flagged += 1;
                }
                MatchVerification::NoMetadata => {
                    // No metadata available, can't verify
                    result.skipped += 1;
                }
            }
        }

        if result.corrected > 0 || result.flagged > 0 {
            info!(
                source_type = %source_type,
                source_id = %source_id,
                total = result.total,
                verified = result.verified,
                corrected = result.corrected,
                flagged = result.flagged,
                "Verification complete"
            );
        }

        Ok(result)
    }

    /// Verify a single audio file match against its embedded tags
    async fn verify_audio_match(
        &self,
        pending_match: &PendingFileMatchRecord,
    ) -> Result<MatchVerification> {
        // Read embedded tags
        let metadata = match read_audio_metadata(&pending_match.source_path) {
            Some(m) if m.has_audio_tags() => m,
            _ => return Ok(MatchVerification::NoMetadata),
        };

        // Get the currently matched item
        let (current_album_name, current_track_title) = match &pending_match.target_type() {
            Some(MatchTarget::Track(track_id)) => {
                let track = self.db.tracks().get_by_id(*track_id).await?;
                if let Some(track) = track {
                    let album = self.db.albums().get_by_id(track.album_id).await?;
                    let album_name = album.map(|a| a.name).unwrap_or_default();
                    (album_name, track.title)
                } else {
                    return Ok(MatchVerification::Verified); // Track not found, assume OK
                }
            }
            Some(MatchTarget::Chapter(chapter_id)) => {
                let chapter = self.db.chapters().get_by_id(*chapter_id).await?;
                if let Some(chapter) = chapter {
                    let audiobook = self.db.audiobooks().get_by_id(chapter.audiobook_id).await?;
                    let book_title = audiobook.map(|a| a.title).unwrap_or_default();
                    let chapter_title = chapter.title.unwrap_or_else(|| format!("Chapter {}", chapter.chapter_number));
                    (book_title, chapter_title)
                } else {
                    return Ok(MatchVerification::Verified);
                }
            }
            _ => return Ok(MatchVerification::Verified), // Not an audio match
        };

        // Compare embedded tags with current match
        let album_match = if let Some(ref meta_album) = metadata.album {
            filename_parser::show_name_similarity(&current_album_name, meta_album) >= 0.6
        } else {
            true // No album tag, can't compare
        };

        let title_match = if let Some(ref meta_title) = metadata.title {
            filename_parser::show_name_similarity(&current_track_title, meta_title) >= 0.6
        } else {
            true // No title tag, can't compare
        };

        if album_match && title_match {
            return Ok(MatchVerification::Verified);
        }

        // Mismatch detected - try to find a better match
        debug!(
            path = %pending_match.source_path,
            current_album = %current_album_name,
            meta_album = ?metadata.album,
            current_title = %current_track_title,
            meta_title = ?metadata.title,
            "Mismatch detected, searching for correct match"
        );

        // Get libraries and try to find correct match
        let libraries = self.db.libraries().list_by_user(pending_match.user_id).await?;
        let music_libs: Vec<_> = libraries.iter().filter(|l| l.library_type == "music").collect();
        let audiobook_libs: Vec<_> = libraries.iter().filter(|l| l.library_type == "audiobooks").collect();

        let file = FileInfo {
            path: pending_match.source_path.clone(),
            size: pending_match.file_size,
            file_index: pending_match.source_file_index,
            source_name: None,
        };
        let quality = ParsedQuality::default();

        // Try to find correct track
        for lib in music_libs {
            if let Some(result) = self.try_match_track_by_metadata(&file, &metadata, &quality, lib).await? {
                if result.confidence >= 0.9 {
                    if let Some(target) = result.match_target.to_match_target() {
                        return Ok(MatchVerification::Corrected {
                            new_target: target,
                            confidence: result.confidence,
                        });
                    }
                }
            }
        }

        // Try audiobooks
        for lib in audiobook_libs {
            if let Some(result) = self.try_match_chapter_by_metadata(&file, &metadata, &quality, lib).await? {
                if result.confidence >= 0.9 {
                    if let Some(target) = result.match_target.to_match_target() {
                        return Ok(MatchVerification::Corrected {
                            new_target: target,
                            confidence: result.confidence,
                        });
                    }
                }
            }
        }

        // No high-confidence alternative found, flag for review
        let reason = format!(
            "Album mismatch: file='{}' vs db='{}'",
            metadata.album.as_deref().unwrap_or("?"),
            current_album_name
        );
        Ok(MatchVerification::Flagged { reason })
    }

    /// Verify a single video file match against its embedded tags
    async fn verify_video_match(
        &self,
        pending_match: &PendingFileMatchRecord,
    ) -> Result<MatchVerification> {
        // Read embedded tags (async since ffprobe is used)
        let metadata = match read_video_metadata(&pending_match.source_path).await {
            Some(m) if m.has_video_tags() => m,
            _ => return Ok(MatchVerification::NoMetadata),
        };

        // Get the currently matched item
        match &pending_match.target_type() {
            Some(MatchTarget::Episode(episode_id)) => {
                let episode = self.db.episodes().get_by_id(*episode_id).await?;
                if let Some(ep) = episode {
                    let show = self.db.tv_shows().get_by_id(ep.tv_show_id).await?;
                    let show_name = show.map(|s| s.name).unwrap_or_default();

                    // Check if metadata matches
                    let show_match = if let Some(ref meta_show) = metadata.album {
                        filename_parser::show_name_similarity(&show_name, meta_show) >= 0.6
                    } else {
                        true
                    };

                    let episode_match = match (metadata.season, metadata.episode) {
                        (Some(s), Some(e)) => s == ep.season && e == ep.episode,
                        _ => true, // No S/E tags, can't compare
                    };

                    if show_match && episode_match {
                        return Ok(MatchVerification::Verified);
                    }

                    // Mismatch - for video, we flag rather than auto-correct
                    // (video metadata is less reliable than audio tags)
                    let reason = format!(
                        "Episode mismatch: meta S{:02}E{:02} vs db S{:02}E{:02}",
                        metadata.season.unwrap_or(0),
                        metadata.episode.unwrap_or(0),
                        ep.season,
                        ep.episode
                    );
                    return Ok(MatchVerification::Flagged { reason });
                }
            }
            Some(MatchTarget::Movie(movie_id)) => {
                let movie = self.db.movies().get_by_id(*movie_id).await?;
                if let Some(movie) = movie {
                    // Check if metadata matches
                    let title_match = if let Some(ref meta_title) = metadata.title {
                        filename_parser::show_name_similarity(&movie.title, meta_title) >= 0.6
                    } else {
                        true
                    };

                    let year_match = match (metadata.year, movie.year) {
                        (Some(my), Some(dy)) => (my - dy).abs() <= 1,
                        _ => true,
                    };

                    if title_match && year_match {
                        return Ok(MatchVerification::Verified);
                    }

                    let reason = format!(
                        "Movie mismatch: meta='{}' vs db='{}'",
                        metadata.title.as_deref().unwrap_or("?"),
                        movie.title
                    );
                    return Ok(MatchVerification::Flagged { reason });
                }
            }
            _ => {}
        }

        Ok(MatchVerification::Verified)
    }

    // =========================================================================
    // Internal Matching Logic
    // =========================================================================

    async fn get_libraries(
        &self,
        user_id: Uuid,
        target_library_id: Option<Uuid>,
    ) -> Result<Vec<LibraryRecord>> {
        if let Some(lib_id) = target_library_id {
            // Get specific library
            if let Some(lib) = self.db.libraries().get_by_id(lib_id).await? {
                Ok(vec![lib])
            } else {
                Ok(vec![])
            }
        } else {
            // Get all user libraries with auto_download enabled
            let all_libs = self.db.libraries().list_by_user(user_id).await?;
            Ok(all_libs
                .into_iter()
                .filter(|l| l.auto_download)
                .collect())
        }
    }

    async fn match_file_against_libraries(
        &self,
        file: &FileInfo,
        libraries: &[LibraryRecord],
    ) -> Result<Vec<FileMatchResult>> {
        let file_name = Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);

        // Check if this is a sample file
        if self.is_sample_file(file_name) {
            return Ok(vec![FileMatchResult {
                file_path: file.path.clone(),
                file_size: file.size,
                file_index: file.file_index,
                match_target: FileMatchTarget::Sample,
                match_type: FileMatchType::Auto,
                confidence: 1.0,
                quality: ParsedQuality::default(),
            }]);
        }

        // Route based on file type
        if is_video_file(&file.path) {
            self.match_video_file(file, file_name, libraries).await
        } else if is_audio_file(&file.path) {
            self.match_audio_file(file, file_name, libraries).await
        } else {
            // Non-media file
            Ok(vec![FileMatchResult {
                file_path: file.path.clone(),
                file_size: file.size,
                file_index: file.file_index,
                match_target: FileMatchTarget::Unmatched {
                    reason: "Not a media file".to_string(),
                },
                match_type: FileMatchType::Auto,
                confidence: 0.0,
                quality: ParsedQuality::default(),
            }])
        }
    }

    fn is_sample_file(&self, file_name: &str) -> bool {
        let lower = file_name.to_lowercase();
        lower.contains("sample") || lower.contains("trailer") || lower.contains("preview")
    }

    // =========================================================================
    // Video Matching (TV Episodes & Movies)
    // =========================================================================

    /// Match a video file to TV episodes or movies in the given libraries.
    /// 
    /// This is public for use by the scanner which already has library context.
    pub async fn match_video_file(
        &self,
        file: &FileInfo,
        file_name: &str,
        libraries: &[LibraryRecord],
    ) -> Result<Vec<FileMatchResult>> {
        let quality = filename_parser::parse_quality(file_name);
        let mut results = Vec::new();

        // Try to read embedded metadata if file exists on disk
        let metadata = if Path::new(&file.path).exists() {
            read_video_metadata(&file.path).await
        } else {
            None
        };

        // Try to parse as episode first
        let parsed = filename_parser::parse_episode(file_name);

        if parsed.season.is_some() && parsed.episode.is_some() {
            // This looks like a TV episode - try TV libraries
            let tv_libs: Vec<_> = libraries.iter().filter(|l| l.library_type == "tv").collect();
            for lib in tv_libs {
                // Try metadata-based matching first
                if let Some(ref meta) = metadata {
                    if meta.has_video_tags() {
                        if let Some(result) = self
                            .try_match_episode_by_metadata(file, meta, &quality, lib)
                            .await?
                        {
                            results.push(result);
                            continue;
                        }
                    }
                }

                // Fall back to filename parsing
                if let Some(result) = self
                    .try_match_episode(&parsed, file, &quality, lib)
                    .await?
                {
                    results.push(result);
                }
            }
        }

        // If no episode match, or if it didn't look like an episode, try movies
        if results.is_empty() {
            let movie_parsed = filename_parser::parse_movie(file_name);
            let movie_libs: Vec<_> = libraries
                .iter()
                .filter(|l| l.library_type == "movies")
                .collect();
            for lib in movie_libs {
                // Try metadata-based matching first
                if let Some(ref meta) = metadata {
                    if meta.has_video_tags() {
                        if let Some(result) = self
                            .try_match_movie_by_metadata(file, meta, &quality, lib)
                            .await?
                        {
                            results.push(result);
                            continue;
                        }
                    }
                }

                // Fall back to filename parsing
                if let Some(result) = self
                    .try_match_movie(&movie_parsed, file, &quality, lib)
                    .await?
                {
                    results.push(result);
                }
            }
        }

        // If still no match, return unmatched
        if results.is_empty() {
            results.push(FileMatchResult {
                file_path: file.path.clone(),
                file_size: file.size,
                file_index: file.file_index,
                match_target: FileMatchTarget::Unmatched {
                    reason: "Could not match to any library item".to_string(),
                },
                match_type: FileMatchType::Auto,
                confidence: 0.0,
                quality,
            });
        }

        Ok(results)
    }

    /// Match a TV episode using embedded container metadata
    /// Match a TV episode using embedded metadata
    ///
    /// Uses weighted scoring: Show (30), Season (25), Episode (25), Title (15), Year (5)
    async fn try_match_episode_by_metadata(
        &self,
        file: &FileInfo,
        metadata: &MediaMetadata,
        quality: &ParsedQuality,
        library: &LibraryRecord,
    ) -> Result<Option<FileMatchResult>> {
        use super::match_scorer::{self, parse_episode_info, score_tv_match, AUTO_LINK_THRESHOLD};

        let shows = self.db.tv_shows().list_by_library(library.id).await?;
        
        // Parse filename for fallback info
        let filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);
        let file_info = parse_episode_info(filename);
        
        // Get metadata values
        let meta_show = metadata.album.as_deref(); // Show name stored in album tag
        let meta_season = metadata.season;
        let meta_episode = metadata.episode;
        let meta_title = metadata.title.as_deref();
        
        // Collect all candidates with scores
        let mut candidates: Vec<(f64, match_scorer::ScoreBreakdown, FileMatchResult)> = Vec::new();

        for show in &shows {
            let episodes = self.db.episodes().list_by_show(show.id).await?;
            
            for ep in episodes {
                // Skip episodes that already have a media file
                if ep.media_file_id.is_some() {
                    continue;
                }

                // Calculate weighted score
                let breakdown = score_tv_match(
                    meta_show,
                    meta_season,
                    meta_episode,
                    meta_title,
                    &file_info,
                    file.source_name.as_deref(),
                    &show.name,
                    ep.season,
                    ep.episode,
                    ep.title.as_deref(),
                );

                if breakdown.total >= match_scorer::SUGGEST_THRESHOLD {
                    let result = FileMatchResult {
                        file_path: file.path.clone(),
                        file_size: file.size,
                        file_index: file.file_index,
                        match_target: FileMatchTarget::Episode {
                            episode_id: ep.id,
                            show_id: show.id,
                            show_name: show.name.clone(),
                            season: ep.season,
                            episode: ep.episode,
                            library_id: library.id,
                        },
                        match_type: FileMatchType::Auto,
                        confidence: breakdown.total / 100.0,
                        quality: quality.clone(),
                    };
                    candidates.push((breakdown.total, breakdown, result));
                }
            }
        }

        // Sort by score descending and pick the best
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((score, breakdown, result)) = candidates.into_iter().next() {
            if score >= AUTO_LINK_THRESHOLD {
                if let FileMatchTarget::Episode { ref show_name, season, episode, .. } = result.match_target {
                    debug!(
                        show = %show_name,
                        season = season,
                        episode = episode,
                        score = score,
                        breakdown = %breakdown.summary(),
                        "Matched episode by weighted scoring"
                    );
                }
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    /// Match a movie using embedded container metadata
    ///
    /// Uses weighted scoring: Title (50), Year (30), Director (20)
    async fn try_match_movie_by_metadata(
        &self,
        file: &FileInfo,
        metadata: &MediaMetadata,
        quality: &ParsedQuality,
        library: &LibraryRecord,
    ) -> Result<Option<FileMatchResult>> {
        use super::match_scorer::{self, parse_movie_info, score_movie_match, AUTO_LINK_THRESHOLD};

        let meta_title = metadata.title.as_deref();
        let meta_year = metadata.year;
        
        // Parse filename for fallback info
        let filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);
        let file_info = parse_movie_info(filename);

        let movies = self.db.movies().list_by_library(library.id).await?;
        
        // Collect all candidates with scores
        let mut candidates: Vec<(f64, match_scorer::ScoreBreakdown, FileMatchResult)> = Vec::new();

        for movie in movies {
            // Skip movies that already have a media file (status would be "downloaded")
            if movie.media_file_id.is_some() {
                continue;
            }

            // Calculate weighted score
            let breakdown = score_movie_match(
                meta_title,
                meta_year,
                None, // No director in metadata typically
                &file_info,
                file.source_name.as_deref(),
                &movie.title,
                movie.year,
                None, // No director in movie record typically
            );

            if breakdown.total >= match_scorer::SUGGEST_THRESHOLD {
                let result = FileMatchResult {
                    file_path: file.path.clone(),
                    file_size: file.size,
                    file_index: file.file_index,
                    match_target: FileMatchTarget::Movie {
                        movie_id: movie.id,
                        title: movie.title.clone(),
                        year: movie.year,
                        library_id: library.id,
                    },
                    match_type: FileMatchType::Auto,
                    confidence: breakdown.total / 100.0,
                    quality: quality.clone(),
                };
                candidates.push((breakdown.total, breakdown, result));
            }
        }

        // Sort by score descending and pick the best
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((score, breakdown, result)) = candidates.into_iter().next() {
            if score >= AUTO_LINK_THRESHOLD {
                if let FileMatchTarget::Movie { ref title, year, .. } = result.match_target {
                    debug!(
                        movie = %title,
                        year = ?year,
                        score = score,
                        breakdown = %breakdown.summary(),
                        "Matched movie by weighted scoring"
                    );
                }
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    /// Match a TV episode using filename parsing
    ///
    /// Uses weighted scoring: Show (30), Season (25), Episode (25), Title (15), Year (5)
    async fn try_match_episode(
        &self,
        parsed: &ParsedEpisode,
        file: &FileInfo,
        quality: &ParsedQuality,
        library: &LibraryRecord,
    ) -> Result<Option<FileMatchResult>> {
        use super::match_scorer::{self, parse_episode_info, score_tv_match, AUTO_LINK_THRESHOLD};

        // Parse filename for additional info
        let filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);
        let file_info = parse_episode_info(filename);
        
        // Use parsed values, falling back to file_info
        let parsed_season = parsed.season.map(|s| s as i32);
        let parsed_episode = parsed.episode.map(|e| e as i32);

        let shows = self.db.tv_shows().list_by_library(library.id).await?;
        
        // Collect all candidates with scores
        let mut candidates: Vec<(f64, match_scorer::ScoreBreakdown, FileMatchResult)> = Vec::new();

        for show in shows {
            let episodes = self.db.episodes().list_by_show(show.id).await?;
            
            for ep in episodes {
                // Skip episodes that already have a media file
                if ep.media_file_id.is_some() {
                    continue;
                }

                // Calculate weighted score
                let breakdown = score_tv_match(
                    parsed.show_name.as_deref(),
                    parsed_season,
                    parsed_episode,
                    None, // No title from filename parser
                    &file_info,
                    file.source_name.as_deref(),
                    &show.name,
                    ep.season,
                    ep.episode,
                    ep.title.as_deref(),
                );

                if breakdown.total >= match_scorer::SUGGEST_THRESHOLD {
                    let result = FileMatchResult {
                        file_path: file.path.clone(),
                        file_size: file.size,
                        file_index: file.file_index,
                        match_target: FileMatchTarget::Episode {
                            episode_id: ep.id,
                            show_id: show.id,
                            show_name: show.name.clone(),
                            season: ep.season,
                            episode: ep.episode,
                            library_id: library.id,
                        },
                        match_type: FileMatchType::Auto,
                        confidence: breakdown.total / 100.0,
                        quality: quality.clone(),
                    };
                    candidates.push((breakdown.total, breakdown, result));
                }
            }
        }

        // Sort by score descending and pick the best
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((score, breakdown, result)) = candidates.into_iter().next() {
            if score >= AUTO_LINK_THRESHOLD {
                if let FileMatchTarget::Episode { ref show_name, season, episode, .. } = result.match_target {
                    debug!(
                        show = %show_name,
                        season = season,
                        episode = episode,
                        score = score,
                        breakdown = %breakdown.summary(),
                        "Matched episode by filename scoring"
                    );
                }
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    /// Match a movie using filename parsing
    ///
    /// Uses weighted scoring: Title (50), Year (30), Director (20)
    async fn try_match_movie(
        &self,
        parsed: &ParsedEpisode,  // parse_movie returns ParsedEpisode
        file: &FileInfo,
        quality: &ParsedQuality,
        library: &LibraryRecord,
    ) -> Result<Option<FileMatchResult>> {
        use super::match_scorer::{self, parse_movie_info, score_movie_match, AUTO_LINK_THRESHOLD};

        // Parse filename for additional info
        let filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);
        let file_info = parse_movie_info(filename);
        
        let parsed_year = parsed.year.map(|y| y as i32);

        let movies = self.db.movies().list_by_library(library.id).await?;
        
        // Collect all candidates with scores
        let mut candidates: Vec<(f64, match_scorer::ScoreBreakdown, FileMatchResult)> = Vec::new();

        for movie in movies {
            // Skip movies that already have a media file (status would be "downloaded")
            if movie.media_file_id.is_some() {
                continue;
            }

            // Calculate weighted score
            let breakdown = score_movie_match(
                None, // No metadata title
                parsed_year,
                None, // No director
                &file_info,
                file.source_name.as_deref(),
                &movie.title,
                movie.year,
                None, // No director in DB
            );

            if breakdown.total >= match_scorer::SUGGEST_THRESHOLD {
                let result = FileMatchResult {
                    file_path: file.path.clone(),
                    file_size: file.size,
                    file_index: file.file_index,
                    match_target: FileMatchTarget::Movie {
                        movie_id: movie.id,
                        title: movie.title.clone(),
                        year: movie.year,
                        library_id: library.id,
                    },
                    match_type: FileMatchType::Auto,
                    confidence: breakdown.total / 100.0,
                    quality: quality.clone(),
                };
                candidates.push((breakdown.total, breakdown, result));
            }
        }

        // Sort by score descending and pick the best
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((score, breakdown, result)) = candidates.into_iter().next() {
            if score >= AUTO_LINK_THRESHOLD {
                if let FileMatchTarget::Movie { ref title, year, .. } = result.match_target {
                    debug!(
                        movie = %title,
                        year = ?year,
                        score = score,
                        breakdown = %breakdown.summary(),
                        "Matched movie by filename scoring"
                    );
                }
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    // =========================================================================
    // Audio Matching (Music Tracks & Audiobook Chapters)
    // =========================================================================

    /// Match an audio file to music tracks or audiobook chapters in the given libraries.
    /// 
    /// This is public for use by the scanner which already has library context.
    pub async fn match_audio_file(
        &self,
        file: &FileInfo,
        file_name: &str,
        libraries: &[LibraryRecord],
    ) -> Result<Vec<FileMatchResult>> {
        let quality = filename_parser::parse_quality(file_name);
        let mut results = Vec::new();

        // Try to read embedded metadata if file exists on disk
        // This is the preferred matching method when available
        let metadata = if Path::new(&file.path).exists() {
            read_audio_metadata(&file.path)
        } else {
            None
        };

        // Try music libraries first
        let music_libs: Vec<_> = libraries
            .iter()
            .filter(|l| l.library_type == "music")
            .collect();
        
        for lib in &music_libs {
            // Try metadata-based matching first (highest confidence)
            if let Some(ref meta) = metadata {
                if meta.has_audio_tags() {
                    if let Some(result) = self
                        .try_match_track_by_metadata(file, meta, &quality, lib)
                        .await?
                    {
                        results.push(result);
                        continue; // Found match for this library
                    }
                }
            }

            // Fall back to source-context + filename matching
            if let Some(result) = self.try_match_track(file, file_name, &quality, lib).await? {
                results.push(result);
            }
        }

        // Try audiobook libraries
        let audiobook_libs: Vec<_> = libraries
            .iter()
            .filter(|l| l.library_type == "audiobooks")
            .collect();
        
        for lib in &audiobook_libs {
            // Try metadata-based matching first for audiobooks too
            if let Some(ref meta) = metadata {
                if meta.has_audio_tags() {
                    if let Some(result) = self
                        .try_match_chapter_by_metadata(file, meta, &quality, lib)
                        .await?
                    {
                        results.push(result);
                        continue;
                    }
                }
            }

            // Fall back to filename matching
            if let Some(result) = self
                .try_match_chapter(file, file_name, &quality, lib)
                .await?
            {
                results.push(result);
            }
        }

        // If no match, return unmatched
        if results.is_empty() {
            results.push(FileMatchResult {
                file_path: file.path.clone(),
                file_size: file.size,
                file_index: file.file_index,
                match_target: FileMatchTarget::Unmatched {
                    reason: "Could not match to any library item".to_string(),
                },
                match_type: FileMatchType::Auto,
                confidence: 0.0,
                quality,
            });
        }

        Ok(results)
    }

    /// Match a track using embedded metadata (ID3/Vorbis tags)
    ///
    /// Uses weighted scoring: Artist (30), Album (25), Track Title (25), Track Number (15), Year (5)
    /// Collects ALL candidates and picks the best match.
    async fn try_match_track_by_metadata(
        &self,
        file: &FileInfo,
        metadata: &MediaMetadata,
        quality: &ParsedQuality,
        library: &LibraryRecord,
    ) -> Result<Option<FileMatchResult>> {
        use super::match_scorer::{self, parse_track_info, score_music_match, AUTO_LINK_THRESHOLD};
        
        let albums = self.db.albums().list_by_library(library.id).await?;

        // Get metadata values
        let meta_album = metadata.album.as_deref();
        let meta_title = metadata.title.as_deref();
        let meta_artist = metadata.artist.as_deref();
        let meta_track = metadata.track_number;
        let meta_disc = metadata.disc_number;
        let meta_year = metadata.year;
        
        // Parse filename for fallback info
        let filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);
        let file_info = parse_track_info(filename);
        
        // Collect all candidates with scores
        let mut candidates: Vec<(f64, match_scorer::ScoreBreakdown, FileMatchResult)> = Vec::new();

        for album in albums {
            // Get artist name for this album
            let artist = self.db.albums().get_artist_by_id(album.artist_id).await?;
            let artist_name = artist.map(|a| a.name).unwrap_or_default();

            // Get tracks for this album
            let tracks = self.db.tracks().list_by_album(album.id).await?;

            for track in tracks {
                // Skip tracks that already have a media file (status would be "downloaded")
                if track.media_file_id.is_some() {
                    continue;
                }

                // Calculate weighted score
                let breakdown = score_music_match(
                    meta_artist,
                    meta_album,
                    meta_title,
                    meta_track,
                    meta_disc,
                    meta_year,
                    &file_info,
                    &artist_name,
                    &album.name,
                    &track.title,
                    track.track_number,
                    track.disc_number,
                    album.year,
                );

                // Only consider candidates above minimum threshold
                if breakdown.total >= match_scorer::SUGGEST_THRESHOLD {
                    let result = FileMatchResult {
                        file_path: file.path.clone(),
                        file_size: file.size,
                        file_index: file.file_index,
                        match_target: FileMatchTarget::Track {
                            track_id: track.id,
                            album_id: album.id,
                            title: track.title.clone(),
                            track_number: track.track_number,
                            library_id: library.id,
                        },
                        match_type: FileMatchType::Auto,
                        confidence: breakdown.total / 100.0, // Normalize to 0-1
                        quality: quality.clone(),
                    };
                    candidates.push((breakdown.total, breakdown, result));
                }
            }
        }

        // Sort by score descending and pick the best
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((score, breakdown, result)) = candidates.into_iter().next() {
            // Only auto-link if above threshold
            if score >= AUTO_LINK_THRESHOLD {
                if let FileMatchTarget::Track { ref title, .. } = result.match_target {
                    debug!(
                        track = %title,
                        score = score,
                        breakdown = %breakdown.summary(),
                        file = %filename,
                        "Matched track by weighted scoring"
                    );
                }
                return Ok(Some(result));
            } else {
                debug!(
                    score = score,
                    threshold = AUTO_LINK_THRESHOLD,
                    file = %filename,
                    "Best match below auto-link threshold"
                );
            }
        }

        Ok(None)
    }

    /// Match an audiobook chapter using embedded metadata
    ///
    /// Uses weighted scoring: Author (30), Book (30), Chapter Title (25), Chapter Number (15)
    async fn try_match_chapter_by_metadata(
        &self,
        file: &FileInfo,
        metadata: &MediaMetadata,
        quality: &ParsedQuality,
        library: &LibraryRecord,
    ) -> Result<Option<FileMatchResult>> {
        use super::match_scorer::{self, parse_track_info, score_audiobook_match, AUTO_LINK_THRESHOLD};
        
        let audiobooks = self.db.audiobooks().list_by_library(library.id).await?;

        // Get metadata values - for audiobooks: artist = author, album = book title, title = chapter title
        let meta_author = metadata.artist.as_deref();
        let meta_book = metadata.album.as_deref();
        let meta_chapter_title = metadata.title.as_deref();
        let meta_track = metadata.track_number;
        
        // Parse filename for fallback info
        let filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);
        let file_info = parse_track_info(filename);
        
        // Collect all candidates with scores
        let mut candidates: Vec<(f64, match_scorer::ScoreBreakdown, FileMatchResult)> = Vec::new();

        for audiobook in audiobooks {
            // Get author name for this audiobook
            let author_name = if let Some(author_id) = audiobook.author_id {
                self.db.audiobooks().get_author_by_id(author_id).await?
                    .map(|a| a.name)
                    .unwrap_or_default()
            } else {
                String::new()
            };
            
            let chapters = self.db.chapters().list_by_audiobook(audiobook.id).await?;

            for chapter in chapters {
                // Skip chapters that already have a media file
                if chapter.media_file_id.is_some() {
                    continue;
                }

                // Calculate weighted score
                let breakdown = score_audiobook_match(
                    meta_author,
                    meta_book,
                    meta_chapter_title,
                    meta_track,
                    &file_info,
                    file.source_name.as_deref(),
                    &author_name,
                    &audiobook.title,
                    chapter.title.as_deref(),
                    chapter.chapter_number,
                );

                if breakdown.total >= match_scorer::SUGGEST_THRESHOLD {
                    let result = FileMatchResult {
                        file_path: file.path.clone(),
                        file_size: file.size,
                        file_index: file.file_index,
                        match_target: FileMatchTarget::Chapter {
                            chapter_id: chapter.id,
                            audiobook_id: audiobook.id,
                            chapter_number: chapter.chapter_number,
                            library_id: library.id,
                        },
                        match_type: FileMatchType::Auto,
                        confidence: breakdown.total / 100.0,
                        quality: quality.clone(),
                    };
                    candidates.push((breakdown.total, breakdown, result));
                }
            }
        }

        // Sort by score descending and pick the best
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((score, breakdown, result)) = candidates.into_iter().next() {
            if score >= AUTO_LINK_THRESHOLD {
                if let FileMatchTarget::Chapter { chapter_number, .. } = result.match_target {
                    debug!(
                        chapter = chapter_number,
                        score = score,
                        breakdown = %breakdown.summary(),
                        "Matched chapter by weighted scoring"
                    );
                }
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    /// Match a track using filename parsing (when metadata is unavailable)
    ///
    /// Uses weighted scoring with source context for album matching.
    /// Collects ALL candidates and picks the best match.
    async fn try_match_track(
        &self,
        file: &FileInfo,
        file_name: &str,
        quality: &ParsedQuality,
        library: &LibraryRecord,
    ) -> Result<Option<FileMatchResult>> {
        use super::match_scorer::{self, parse_track_info, score_music_match, AUTO_LINK_THRESHOLD};
        
        // Get all albums in this library
        let all_albums = self.db.albums().list_by_library(library.id).await?;

        // Parse filename for track number and cleaned title
        let file_info = parse_track_info(file_name);
        
        // Collect all candidates with scores
        let mut candidates: Vec<(f64, match_scorer::ScoreBreakdown, FileMatchResult)> = Vec::new();

        for album in &all_albums {
            // Get artist name for this album
            let artist = self.db.albums().get_artist_by_id(album.artist_id).await?;
            let artist_name = artist.map(|a| a.name).unwrap_or_default();

            // Get tracks for this album
            let tracks = self.db.tracks().list_by_album(album.id).await?;

            for track in tracks {
                // Skip tracks that already have a media file
                if track.media_file_id.is_some() {
                    continue;
                }

                // Calculate weighted score using filename info only
                // When matching by filename, we don't have metadata, so use source_name for album matching
                let breakdown = score_music_match(
                    None, // No metadata artist
                    file.source_name.as_deref(), // Use source name as pseudo-album
                    None, // No metadata title - use file_info.cleaned_title
                    None, // No metadata track number - use file_info.number
                    None, // No metadata disc number - use file_info.disc_number
                    None, // No metadata year
                    &file_info,
                    &artist_name,
                    &album.name,
                    &track.title,
                    track.track_number,
                    track.disc_number,
                    album.year,
                );

                // Only consider candidates above minimum threshold
                if breakdown.total >= match_scorer::SUGGEST_THRESHOLD {
                    let result = FileMatchResult {
                        file_path: file.path.clone(),
                        file_size: file.size,
                        file_index: file.file_index,
                        match_target: FileMatchTarget::Track {
                            track_id: track.id,
                            album_id: album.id,
                            title: track.title.clone(),
                            track_number: track.track_number,
                            library_id: library.id,
                        },
                        match_type: FileMatchType::Auto,
                        confidence: breakdown.total / 100.0,
                        quality: quality.clone(),
                    };
                    candidates.push((breakdown.total, breakdown, result));
                }
            }
        }

        // Sort by score descending and pick the best
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((score, breakdown, result)) = candidates.into_iter().next() {
            if score >= AUTO_LINK_THRESHOLD {
                if let FileMatchTarget::Track { ref title, .. } = result.match_target {
                    debug!(
                        track = %title,
                        score = score,
                        breakdown = %breakdown.summary(),
                        file = %file_name,
                        "Matched track by filename scoring"
                    );
                }
                return Ok(Some(result));
            } else {
                debug!(
                    score = score,
                    threshold = AUTO_LINK_THRESHOLD,
                    file = %file_name,
                    "Best filename match below auto-link threshold"
                );
            }
        }

        Ok(None)
    }

    /// Filter albums into priority (matching source name) and others
    ///
    /// Parses album info from torrent name and returns albums sorted by match likelihood.
    fn filter_albums_by_source<'a>(
        &self,
        source_name: &str,
        albums: &'a [crate::db::albums::AlbumRecord],
    ) -> (Vec<&'a crate::db::albums::AlbumRecord>, Vec<&'a crate::db::albums::AlbumRecord>) {
        let mut priority = Vec::new();
        let mut others = Vec::new();

        // Clean up source name for matching
        let clean_source = self.clean_source_name(source_name);

        for album in albums {
            // Check if album name is contained in source name or vice versa
            let album_similarity = filename_parser::show_name_similarity(&clean_source, &album.name);
            
            // Note: Could also check artist name, but would need to load artist by album.artist_id
            // For now, we rely on album name matching
            if album_similarity >= 0.5 {
                priority.push(album);
            } else {
                others.push(album);
            }
        }

        // Sort priority albums by similarity (highest first)
        priority.sort_by(|a, b| {
            let sim_a = filename_parser::show_name_similarity(&clean_source, &a.name);
            let sim_b = filename_parser::show_name_similarity(&clean_source, &b.name);
            sim_b.partial_cmp(&sim_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        (priority, others)
    }

    /// Clean up a source name (torrent name) for matching
    fn clean_source_name(&self, source_name: &str) -> String {
        let mut name = source_name.to_string();

        // Remove common torrent name patterns
        let patterns = [
            r"\[.*?\]",           // [WEB-FLAC], [24bit], etc.
            r"\(.*?\)",           // (2023), (Deluxe), etc.
            r"-\s*[A-Z0-9]+$",    // -SCENE, -GROUP, etc.
            r"(?i)\bflac\b",
            r"(?i)\bmp3\b",
            r"(?i)\b320\b",
            r"(?i)\bweb\b",
            r"(?i)\b24bit\b",
            r"(?i)\b16bit\b",
            r"(?i)\b44\.1\b",
            r"(?i)\b48\b",
            r"(?i)\blossless\b",
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                name = re.replace_all(&name, " ").to_string();
            }
        }

        // Replace separators with spaces
        name = name.replace(['.', '_', '-'], " ");

        // Collapse multiple spaces
        if let Ok(re) = regex::Regex::new(r"\s+") {
            name = re.replace_all(&name, " ").trim().to_string();
        }

        name
    }

    async fn try_match_chapter(
        &self,
        file: &FileInfo,
        file_name: &str,
        quality: &ParsedQuality,
        library: &LibraryRecord,
    ) -> Result<Option<FileMatchResult>> {
        // Get all audiobooks in this library
        let audiobooks = self.db.audiobooks().list_by_library(library.id).await?;

        for audiobook in audiobooks {
            // Get chapters for this audiobook
            let chapters = self.db.chapters().list_by_audiobook(audiobook.id).await?;

            for chapter in chapters {
                // Skip chapters that already have a media file
                if chapter.media_file_id.is_some() {
                    continue;
                }

                // Try to match by chapter number from filename
                if let Some(chapter_num) = self.extract_chapter_number(file_name) {
                    if chapter_num == chapter.chapter_number {
                        debug!(
                            chapter = chapter.chapter_number,
                            audiobook = %audiobook.title,
                            file = %file_name,
                            "Matched chapter by number"
                        );

                        return Ok(Some(FileMatchResult {
                            file_path: file.path.clone(),
                            file_size: file.size,
                            file_index: file.file_index,
                            match_target: FileMatchTarget::Chapter {
                                chapter_id: chapter.id,
                                audiobook_id: audiobook.id,
                                chapter_number: chapter.chapter_number,
                                library_id: library.id,
                            },
                            match_type: FileMatchType::Auto,
                            confidence: 0.9,
                            quality: quality.clone(),
                        }));
                    }
                }

                // Try title similarity if chapter has a title
                if let Some(ref chapter_title) = chapter.title {
                    let similarity =
                        filename_parser::show_name_similarity(file_name, chapter_title);
                    if similarity >= 0.6 {
                        debug!(
                            chapter = chapter.chapter_number,
                            title = %chapter_title,
                            similarity = similarity,
                            "Matched chapter by title"
                        );

                        return Ok(Some(FileMatchResult {
                            file_path: file.path.clone(),
                            file_size: file.size,
                            file_index: file.file_index,
                            match_target: FileMatchTarget::Chapter {
                                chapter_id: chapter.id,
                                audiobook_id: audiobook.id,
                                chapter_number: chapter.chapter_number,
                                library_id: library.id,
                            },
                            match_type: FileMatchType::Auto,
                            confidence: similarity,
                            quality: quality.clone(),
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    // =========================================================================
    // Helper Functions
    // =========================================================================

    fn calculate_track_similarity(&self, file_name: &str, track_title: &str) -> f64 {
        // Clean up filename
        let clean_file = self.clean_track_filename(file_name);
        let clean_title = self.normalize_track_title(track_title);

        // Use the rapidfuzz-based similarity
        let similarity = filename_parser::show_name_similarity(&clean_file, &clean_title);

        // Bonus if track title is fully contained in filename
        let contains_bonus = if clean_file
            .to_lowercase()
            .contains(&clean_title.to_lowercase())
        {
            0.3
        } else {
            0.0
        };

        (similarity + contains_bonus).min(1.0)
    }

    fn clean_track_filename(&self, file_name: &str) -> String {
        let mut name = file_name.to_string();

        // Remove file extension
        if let Some(pos) = name.rfind('.') {
            name = name[..pos].to_string();
        }

        // Remove leading track numbers (e.g., "01-", "01.", "01 -", etc.)
        let re = regex::Regex::new(r"^\d{1,3}[\s\-._]+").unwrap();
        name = re.replace(&name, "").to_string();

        // Remove common parenthetical suffixes
        let paren_re =
            regex::Regex::new(r"\s*\([^)]*(?:original|remaster|version|mix|edit|live)\s*\)", )
                .unwrap();
        name = paren_re.replace_all(&name, "").to_string();

        // Normalize curly apostrophes to straight
        name = name.replace('\u{2019}', "'").replace('\u{2018}', "'");
        name = name.replace('\u{201C}', "\"").replace('\u{201D}', "\"");

        // Replace underscores with spaces
        name = name.replace('_', " ");

        // Collapse multiple spaces
        let space_re = regex::Regex::new(r"\s+").unwrap();
        name = space_re.replace_all(&name, " ").trim().to_string();

        name
    }

    fn normalize_track_title(&self, title: &str) -> String {
        let mut name = title.to_string();

        // Normalize curly apostrophes
        name = name.replace('\u{2019}', "'").replace('\u{2018}', "'");
        name = name.replace('\u{201C}', "\"").replace('\u{201D}', "\"");

        // Replace underscores with spaces
        name = name.replace('_', " ");

        // Collapse multiple spaces
        let space_re = regex::Regex::new(r"\s+").unwrap();
        name = space_re.replace_all(&name, " ").trim().to_string();

        name
    }

    fn extract_chapter_number(&self, file_name: &str) -> Option<i32> {
        // Try patterns like "Chapter 01", "Ch. 1", "Part 1", etc.
        let patterns = [
            r"(?i)chapter\s*(\d+)",
            r"(?i)ch\.?\s*(\d+)",
            r"(?i)part\s*(\d+)",
            r"^\s*(\d{1,3})\s*[-._]", // Leading number like "01-..."
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(file_name) {
                    if let Some(num_match) = caps.get(1) {
                        if let Ok(num) = num_match.as_str().parse::<i32>() {
                            return Some(num);
                        }
                    }
                }
            }
        }

        None
    }

    // NOTE: Status updates are no longer needed on items.
    // Status is now derived from:
    // - media_file_id IS NOT NULL → downloaded
    // - Entry in pending_file_matches with copied_at IS NULL → downloading
    // - Otherwise → missing/wanted
}
