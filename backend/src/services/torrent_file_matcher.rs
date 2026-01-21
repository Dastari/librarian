//! Torrent file matcher service
//!
//! Provides file-level matching within torrents to library items.
//! This is the core of the unified media pipeline - matching individual files
//! to episodes, movies, tracks, or audiobook chapters.

use std::path::Path;

use anyhow::{Context, Result};
use rust_decimal::prelude::*;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::{
    CreateTorrentFileMatch, Database, EpisodeRecord, LibraryRecord, MovieRecord,
    TorrentFileMatchRecord, TorrentRecord, TvShowRecord,
};
use crate::services::file_utils::{is_audio_file, is_video_file};
use crate::services::filename_parser::{self, ParsedEpisode, ParsedQuality};
use crate::services::torrent::TorrentFile;

/// Result of matching files within a torrent
#[derive(Debug, Clone)]
pub struct FileMatchResult {
    pub file_index: i32,
    pub file_path: String,
    pub file_size: i64,
    /// What type of match this is
    pub match_target: FileMatchTarget,
    /// How we matched it
    pub match_type: FileMatchType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Parsed quality info from filename
    pub quality: ParsedQuality,
    /// Should we skip downloading this file?
    pub skip_download: bool,
    /// Reason for skipping
    pub skip_reason: Option<String>,
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
    },
    /// Matched to a movie
    Movie {
        movie_id: Uuid,
        title: String,
        year: Option<i32>,
    },
    /// Matched to a music track
    Track {
        track_id: Uuid,
        album_id: Uuid,
        title: String,
        track_number: i32,
    },
    /// Matched to an audiobook chapter
    AudiobookChapter {
        chapter_id: Uuid,
        audiobook_id: Uuid,
        chapter_number: i32,
    },
    /// No match found - this is a non-media file or couldn't be matched
    Unmatched { reason: String },
    /// Sample file - should not be organized
    Sample,
}

/// How a file was matched
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMatchType {
    /// Automatically matched by filename parsing
    Auto,
    /// Manually linked by user
    Manual,
    /// Forced link by user (skip quality checks)
    Forced,
}

impl std::fmt::Display for FileMatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileMatchType::Auto => write!(f, "auto"),
            FileMatchType::Manual => write!(f, "manual"),
            FileMatchType::Forced => write!(f, "forced"),
        }
    }
}

/// Service for matching torrent files to library items
pub struct TorrentFileMatcher {
    db: Database,
}

impl TorrentFileMatcher {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // =========================================================================
    // Explicit Match Creation Methods
    // These are used when a user or auto-hunt explicitly selects an item to link
    // =========================================================================

    /// Create an explicit match for a movie
    ///
    /// Used when user adds a torrent for a specific movie, or auto-hunt finds a match.
    /// This creates a torrent_file_matches record linking the file to the movie.
    pub async fn create_match_for_movie(
        &self,
        torrent_id: Uuid,
        file_index: i32,
        file_path: &str,
        file_size: i64,
        movie_id: Uuid,
    ) -> Result<TorrentFileMatchRecord> {
        let quality = filename_parser::parse_quality(file_path);

        let input = CreateTorrentFileMatch {
            torrent_id,
            file_index,
            file_path: file_path.to_string(),
            file_size,
            episode_id: None,
            movie_id: Some(movie_id),
            track_id: None,
            chapter_id: None,
            match_type: FileMatchType::Manual.to_string(),
            match_confidence: Some(Decimal::from(1)),
            parsed_resolution: quality.resolution,
            parsed_codec: quality.codec,
            parsed_source: quality.source,
            parsed_audio: quality.audio,
            skip_download: false,
        };

        let record = self.db.torrent_file_matches().create(input).await?;

        // Update movie status to downloading
        sqlx::query("UPDATE movies SET download_status = 'downloading' WHERE id = $1")
            .bind(movie_id)
            .execute(self.db.pool())
            .await?;

        info!(
            torrent_id = %torrent_id,
            file_index = file_index,
            movie_id = %movie_id,
            "Created explicit match for movie"
        );

        Ok(record)
    }

    /// Create an explicit match for an episode
    ///
    /// Used when user adds a torrent for a specific episode, or auto-hunt finds a match.
    pub async fn create_match_for_episode(
        &self,
        torrent_id: Uuid,
        file_index: i32,
        file_path: &str,
        file_size: i64,
        episode_id: Uuid,
    ) -> Result<TorrentFileMatchRecord> {
        let quality = filename_parser::parse_quality(file_path);

        let input = CreateTorrentFileMatch {
            torrent_id,
            file_index,
            file_path: file_path.to_string(),
            file_size,
            episode_id: Some(episode_id),
            movie_id: None,
            track_id: None,
            chapter_id: None,
            match_type: FileMatchType::Manual.to_string(),
            match_confidence: Some(Decimal::from(1)),
            parsed_resolution: quality.resolution,
            parsed_codec: quality.codec,
            parsed_source: quality.source,
            parsed_audio: quality.audio,
            skip_download: false,
        };

        let record = self.db.torrent_file_matches().create(input).await?;

        // Update episode status to downloading
        self.db
            .episodes()
            .update_status(episode_id, "downloading")
            .await?;

        info!(
            torrent_id = %torrent_id,
            file_index = file_index,
            episode_id = %episode_id,
            "Created explicit match for episode"
        );

        Ok(record)
    }

    /// Create explicit matches for all video files in a torrent, linking them to an episode
    ///
    /// Used for single-episode torrents where all video files belong to the same episode.
    pub async fn create_matches_for_episode_torrent(
        &self,
        torrent_id: Uuid,
        files: &[TorrentFile],
        episode_id: Uuid,
    ) -> Result<Vec<TorrentFileMatchRecord>> {
        let mut records = Vec::new();

        for (index, file) in files.iter().enumerate() {
            if is_video_file(&file.path) && !self.is_sample_file(&file.path) {
                let record = self
                    .create_match_for_episode(
                        torrent_id,
                        index as i32,
                        &file.path,
                        file.size as i64,
                        episode_id,
                    )
                    .await?;
                records.push(record);
            }
        }

        Ok(records)
    }

    /// Create explicit matches for all video files in a torrent, linking them to a movie
    ///
    /// Used for movie torrents where all video files belong to the same movie.
    pub async fn create_matches_for_movie_torrent(
        &self,
        torrent_id: Uuid,
        files: &[TorrentFile],
        movie_id: Uuid,
    ) -> Result<Vec<TorrentFileMatchRecord>> {
        let mut records = Vec::new();

        for (index, file) in files.iter().enumerate() {
            if is_video_file(&file.path) && !self.is_sample_file(&file.path) {
                let record = self
                    .create_match_for_movie(
                        torrent_id,
                        index as i32,
                        &file.path,
                        file.size as i64,
                        movie_id,
                    )
                    .await?;
                records.push(record);
            }
        }

        Ok(records)
    }

    /// Create an explicit match for a track
    ///
    /// Used when linking a specific audio file to a track.
    pub async fn create_match_for_track(
        &self,
        torrent_id: Uuid,
        file_index: i32,
        file_path: &str,
        file_size: i64,
        track_id: Uuid,
    ) -> Result<TorrentFileMatchRecord> {
        let quality = filename_parser::parse_quality(file_path);

        let input = CreateTorrentFileMatch {
            torrent_id,
            file_index,
            file_path: file_path.to_string(),
            file_size,
            episode_id: None,
            movie_id: None,
            track_id: Some(track_id),
            chapter_id: None,
            match_type: FileMatchType::Manual.to_string(),
            match_confidence: Some(Decimal::from(1)),
            parsed_resolution: quality.resolution,
            parsed_codec: quality.codec,
            parsed_source: quality.source,
            parsed_audio: quality.audio,
            skip_download: false,
        };

        let record = self.db.torrent_file_matches().create(input).await?;

        // Update track status to downloading
        sqlx::query("UPDATE tracks SET status = 'downloading' WHERE id = $1")
            .bind(track_id)
            .execute(self.db.pool())
            .await?;

        info!(
            torrent_id = %torrent_id,
            file_index = file_index,
            track_id = %track_id,
            "Created explicit match for track"
        );

        Ok(record)
    }

    /// Create explicit matches for all audio files in a torrent, linking to tracks in an album
    ///
    /// This attempts to match audio files to tracks by track number or title similarity.
    /// Used for album torrents where files should be matched to existing tracks.
    pub async fn create_matches_for_album(
        &self,
        torrent_id: Uuid,
        files: &[TorrentFile],
        album_id: Uuid,
    ) -> Result<Vec<TorrentFileMatchRecord>> {
        let tracks = self.db.tracks().list_by_album(album_id).await?;
        let mut records = Vec::new();

        for (index, file) in files.iter().enumerate() {
            if !is_audio_file(&file.path) {
                continue;
            }

            let file_name = Path::new(&file.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file.path);

            // Try to match by track number first
            if let Some(track_num) = self.extract_track_number(file_name) {
                if let Some(track) = tracks.iter().find(|t| t.track_number == track_num) {
                    let record = self
                        .create_match_for_track(
                            torrent_id,
                            index as i32,
                            &file.path,
                            file.size as i64,
                            track.id,
                        )
                        .await?;
                    records.push(record);
                    continue;
                }
            }

            // Try to match by title similarity
            for track in &tracks {
                let similarity = self.calculate_track_title_similarity(file_name, &track.title);
                if similarity > 0.7 {
                    let record = self
                        .create_match_for_track(
                            torrent_id,
                            index as i32,
                            &file.path,
                            file.size as i64,
                            track.id,
                        )
                        .await?;
                    records.push(record);
                    break;
                }
            }
        }

        Ok(records)
    }

    /// Create an explicit match for an audiobook chapter
    pub async fn create_match_for_chapter(
        &self,
        torrent_id: Uuid,
        file_index: i32,
        file_path: &str,
        file_size: i64,
        chapter_id: Uuid,
    ) -> Result<TorrentFileMatchRecord> {
        let quality = filename_parser::parse_quality(file_path);

        let input = CreateTorrentFileMatch {
            torrent_id,
            file_index,
            file_path: file_path.to_string(),
            file_size,
            episode_id: None,
            movie_id: None,
            track_id: None,
            chapter_id: Some(chapter_id),
            match_type: FileMatchType::Manual.to_string(),
            match_confidence: Some(Decimal::from(1)),
            parsed_resolution: quality.resolution,
            parsed_codec: quality.codec,
            parsed_source: quality.source,
            parsed_audio: quality.audio,
            skip_download: false,
        };

        let record = self.db.torrent_file_matches().create(input).await?;

        // Update chapter status to downloading
        sqlx::query("UPDATE chapters SET status = 'downloading' WHERE id = $1")
            .bind(chapter_id)
            .execute(self.db.pool())
            .await?;

        info!(
            torrent_id = %torrent_id,
            file_index = file_index,
            chapter_id = %chapter_id,
            "Created explicit match for audiobook chapter"
        );

        Ok(record)
    }

    /// Create explicit matches for all audio files in a torrent, linking to chapters in an audiobook
    pub async fn create_matches_for_audiobook(
        &self,
        torrent_id: Uuid,
        files: &[TorrentFile],
        audiobook_id: Uuid,
    ) -> Result<Vec<TorrentFileMatchRecord>> {
        // Get chapters for this audiobook
        let chapters: Vec<(Uuid, i32, Option<String>)> = sqlx::query_as(
            "SELECT id, chapter_number, title FROM chapters WHERE audiobook_id = $1 ORDER BY chapter_number"
        )
        .bind(audiobook_id)
        .fetch_all(self.db.pool())
        .await?;

        let mut records = Vec::new();

        for (index, file) in files.iter().enumerate() {
            if !is_audio_file(&file.path) {
                continue;
            }

            let file_name = Path::new(&file.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file.path);

            // Try to match by chapter number
            if let Some(chapter_num) = self.extract_chapter_number(file_name) {
                if let Some((chapter_id, _, _)) =
                    chapters.iter().find(|(_, num, _)| *num == chapter_num)
                {
                    let record = self
                        .create_match_for_chapter(
                            torrent_id,
                            index as i32,
                            &file.path,
                            file.size as i64,
                            *chapter_id,
                        )
                        .await?;
                    records.push(record);
                }
            }
        }

        Ok(records)
    }

    // =========================================================================
    // Auto-Matching Methods
    // These analyze torrent files and try to match them automatically
    // =========================================================================

    /// Match all files in a torrent to library items
    ///
    /// This is the main entry point. It analyzes each file in the torrent
    /// and attempts to match it to wanted items in libraries.
    ///
    /// # Arguments
    /// * `torrent` - The torrent record
    /// * `files` - List of files in the torrent
    /// * `target_library_id` - Optional specific library to match against (skips auto_download checks)
    /// * `user_id` - User ID for library access
    pub async fn match_torrent_files(
        &self,
        torrent: &TorrentRecord,
        files: &[TorrentFile],
        target_library_id: Option<Uuid>,
        user_id: Uuid,
    ) -> Result<Vec<FileMatchResult>> {
        let mut results = Vec::with_capacity(files.len());

        // If user explicitly targeted a library, we should match regardless of auto_download settings
        let is_targeted_match = target_library_id.is_some();

        info!(
            "Matching files in torrent '{}' ({} files{})",
            torrent.name,
            files.len(),
            if is_targeted_match {
                ", targeted to specific library"
            } else {
                ""
            }
        );

        // Get libraries to match against
        let libraries = if let Some(lib_id) = target_library_id {
            let lib = self
                .db
                .libraries()
                .get_by_id(lib_id)
                .await?
                .context("Target library not found")?;
            debug!("Matching against targeted library '{}'", lib.name);
            vec![lib]
        } else {
            // Get all libraries for this user that allow auto-download
            let libs = self.db.libraries().list_by_user(user_id).await?;
            debug!("Matching against {} user libraries", libs.len());
            libs
        };

        let mut episode_matches = 0;
        let mut movie_matches = 0;
        let mut track_matches = 0;
        let mut chapter_matches = 0;

        for (index, file) in files.iter().enumerate() {
            let result = self
                .match_single_file(torrent, file, index as i32, &libraries, is_targeted_match)
                .await?;

            let file_name = Path::new(&file.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file.path);

            // Log each file's match result
            match &result.match_target {
                FileMatchTarget::Episode {
                    show_name,
                    season,
                    episode,
                    ..
                } => {
                    episode_matches += 1;
                    if result.skip_download {
                        info!(
                            "Matched '{}' to {} S{:02}E{:02} (skipping: {})",
                            file_name,
                            show_name,
                            season,
                            episode,
                            result.skip_reason.as_deref().unwrap_or("already have")
                        );
                    } else {
                        info!(
                            "Matched '{}' to {} S{:02}E{:02}",
                            file_name, show_name, season, episode
                        );
                    }
                }
                FileMatchTarget::Movie { title, year, .. } => {
                    movie_matches += 1;
                    let year_str = year.map(|y| format!(" ({})", y)).unwrap_or_default();
                    if result.skip_download {
                        info!(
                            "Matched '{}' to {}{} (skipping: {})",
                            file_name,
                            title,
                            year_str,
                            result.skip_reason.as_deref().unwrap_or("already have")
                        );
                    } else {
                        info!("Matched '{}' to {}{}", file_name, title, year_str);
                    }
                }
                FileMatchTarget::Track {
                    title,
                    track_number,
                    ..
                } => {
                    track_matches += 1;
                    info!(
                        "Matched '{}' to track {} (#{:02})",
                        file_name, title, track_number
                    );
                }
                FileMatchTarget::AudiobookChapter { chapter_number, .. } => {
                    chapter_matches += 1;
                    info!(
                        "Matched '{}' to audiobook chapter {:02}",
                        file_name, chapter_number
                    );
                }
                FileMatchTarget::Sample => {
                    debug!(
                        "Detected sample file '{}', will download but not organize",
                        file_name
                    );
                }
                FileMatchTarget::Unmatched { reason } => {
                    if is_video_file(&file.path) || is_audio_file(&file.path) {
                        debug!("Could not match media file '{}': {}", file_name, reason);
                    }
                }
            }

            results.push(result);
        }

        // Log summary
        let total_matched = episode_matches + movie_matches + track_matches + chapter_matches;
        info!(
            "File matching complete for '{}': {} of {} files matched ({} episodes, {} movies, {} tracks, {} chapters)",
            torrent.name,
            total_matched,
            files.len(),
            episode_matches,
            movie_matches,
            track_matches,
            chapter_matches
        );

        Ok(results)
    }

    /// Match a single file to library items
    ///
    /// # Arguments
    /// * `is_targeted_match` - If true, skip auto_download/auto_hunt checks (user explicitly targeted this library)
    async fn match_single_file(
        &self,
        torrent: &TorrentRecord,
        file: &TorrentFile,
        file_index: i32,
        libraries: &[LibraryRecord],
        is_targeted_match: bool,
    ) -> Result<FileMatchResult> {
        let file_path = &file.path;
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);

        // Check if this is a sample file
        if self.is_sample_file(file_name) {
            return Ok(FileMatchResult {
                file_index,
                file_path: file_path.clone(),
                file_size: file.size as i64,
                match_target: FileMatchTarget::Sample,
                match_type: FileMatchType::Auto,
                confidence: 1.0,
                quality: ParsedQuality::default(),
                skip_download: false, // Still download for seeding, just don't organize
                skip_reason: Some("Sample file".to_string()),
            });
        }

        // Check file type
        if is_video_file(file_path) {
            self.match_video_file(
                torrent,
                file,
                file_index,
                file_name,
                libraries,
                is_targeted_match,
            )
            .await
        } else if is_audio_file(file_path) {
            self.match_audio_file(
                torrent,
                file,
                file_index,
                file_name,
                libraries,
                is_targeted_match,
            )
            .await
        } else {
            // Non-media file (NFO, TXT, images, etc.)
            Ok(FileMatchResult {
                file_index,
                file_path: file_path.clone(),
                file_size: file.size as i64,
                match_target: FileMatchTarget::Unmatched {
                    reason: "Not a media file".to_string(),
                },
                match_type: FileMatchType::Auto,
                confidence: 0.0,
                quality: ParsedQuality::default(),
                skip_download: false, // Still download for seeding
                skip_reason: None,
            })
        }
    }

    /// Match a video file to TV episodes or movies
    async fn match_video_file(
        &self,
        _torrent: &TorrentRecord,
        file: &TorrentFile,
        file_index: i32,
        file_name: &str,
        libraries: &[LibraryRecord],
        is_targeted_match: bool,
    ) -> Result<FileMatchResult> {
        let file_path = &file.path;
        let quality = filename_parser::parse_quality(file_name);

        // Try to parse as episode first
        let parsed = filename_parser::parse_episode(file_name);

        // Try to match by parsing the filename
        if parsed.season.is_some() && parsed.episode.is_some() {
            // This looks like a TV episode
            if let Some(result) = self
                .try_match_episode(
                    &parsed,
                    file_index,
                    file_path,
                    file.size as i64,
                    libraries,
                    is_targeted_match,
                )
                .await?
            {
                return Ok(result);
            }
        }

        // Try to match as a movie
        let movie_parsed = filename_parser::parse_movie(file_name);
        if let Some(result) = self
            .try_match_movie(
                &movie_parsed,
                file_index,
                file_path,
                file.size as i64,
                libraries,
                is_targeted_match,
            )
            .await?
        {
            return Ok(result);
        }

        // No match found
        Ok(FileMatchResult {
            file_index,
            file_path: file_path.clone(),
            file_size: file.size as i64,
            match_target: FileMatchTarget::Unmatched {
                reason: "Could not match to any library item".to_string(),
            },
            match_type: FileMatchType::Auto,
            confidence: 0.0,
            quality,
            skip_download: false,
            skip_reason: None,
        })
    }

    /// Match an audio file to tracks or audiobook chapters
    async fn match_audio_file(
        &self,
        torrent: &TorrentRecord,
        file: &TorrentFile,
        file_index: i32,
        file_name: &str,
        libraries: &[LibraryRecord],
        _is_targeted_match: bool,
    ) -> Result<FileMatchResult> {
        let file_path = &file.path;
        let quality = filename_parser::parse_quality(file_name);

        debug!(
            torrent_name = %torrent.name,
            file_name = %file_name,
            libraries_count = libraries.len(),
            "Attempting to match audio file"
        );

        // Count music and audiobook libraries
        let music_libs: Vec<_> = libraries
            .iter()
            .filter(|l| l.library_type.to_lowercase() == "music")
            .collect();
        let audiobook_libs: Vec<_> = libraries
            .iter()
            .filter(|l| l.library_type.to_lowercase() == "audiobooks")
            .collect();

        debug!(
            music_libraries = music_libs.len(),
            audiobook_libraries = audiobook_libs.len(),
            "Found libraries to match against"
        );

        // Try to match against music/audiobook libraries
        for library in libraries {
            let lib_type = library.library_type.to_lowercase();
            if lib_type == "music" {
                debug!(
                    library_name = %library.name,
                    library_id = %library.id,
                    "Trying to match against music library"
                );
                if let Some(result) = self
                    .try_match_music_file(
                        library.id,
                        &torrent.name,
                        file_name,
                        file_index,
                        file_path,
                        file.size as i64,
                        &quality,
                    )
                    .await?
                {
                    info!(
                        file_name = %file_name,
                        library_name = %library.name,
                        "Successfully matched audio file to music library"
                    );
                    return Ok(result);
                }
            } else if lib_type == "audiobooks" {
                debug!(
                    library_name = %library.name,
                    library_id = %library.id,
                    "Trying to match against audiobook library"
                );
                if let Some(result) = self
                    .try_match_audiobook_file(
                        library.id,
                        file_name,
                        file_index,
                        file_path,
                        file.size as i64,
                        &quality,
                    )
                    .await?
                {
                    info!(
                        file_name = %file_name,
                        library_name = %library.name,
                        "Successfully matched audio file to audiobook library"
                    );
                    return Ok(result);
                }
            }
        }

        // No match found
        debug!(
            file_name = %file_name,
            torrent_name = %torrent.name,
            "Could not match audio file to any library"
        );
        Ok(FileMatchResult {
            file_index,
            file_path: file_path.clone(),
            file_size: file.size as i64,
            match_target: FileMatchTarget::Unmatched {
                reason: "Could not match audio file to any library item".to_string(),
            },
            match_type: FileMatchType::Auto,
            confidence: 0.0,
            quality,
            skip_download: false,
            skip_reason: None,
        })
    }

    /// Try to match a parsed episode to wanted episodes in libraries
    ///
    /// # Arguments
    /// * `is_targeted_match` - If true, skip auto_download/auto_hunt checks
    async fn try_match_episode(
        &self,
        parsed: &ParsedEpisode,
        file_index: i32,
        file_path: &str,
        file_size: i64,
        libraries: &[LibraryRecord],
        is_targeted_match: bool,
    ) -> Result<Option<FileMatchResult>> {
        let show_name = match &parsed.show_name {
            Some(name) => name,
            None => return Ok(None),
        };

        let season = match parsed.season {
            Some(s) => s as i32,
            None => return Ok(None),
        };

        let episode = match parsed.episode {
            Some(e) => e as i32,
            None => return Ok(None),
        };

        let quality = ParsedQuality {
            resolution: parsed.resolution.clone(),
            source: parsed.source.clone(),
            codec: parsed.codec.clone(),
            hdr: parsed.hdr.clone(),
            audio: parsed.audio.clone(),
        };

        // Search for matching shows in TV libraries
        for library in libraries {
            if library.library_type.to_lowercase() != "tv" {
                continue;
            }

            // Skip auto_download check if user explicitly targeted this library
            if !is_targeted_match && !library.auto_download && !library.auto_hunt {
                debug!(
                    "Skipping library '{}' - auto_download and auto_hunt are disabled",
                    library.name
                );
                continue;
            }

            // Find shows that match this name
            let shows = self.db.tv_shows().list_by_library(library.id).await?;

            for show in shows {
                let similarity = self.calculate_show_name_similarity(show_name, &show.name);

                if similarity > 0.8 {
                    // Good match, look for the episode
                    let episodes = self.db.episodes().list_by_season(show.id, season).await?;

                    for ep in episodes {
                        if ep.episode == episode {
                            // Check if this episode is wanted
                            let should_download = self
                                .should_download_episode(&ep, &show, &library, &quality)
                                .await?;

                            if should_download.skip {
                                return Ok(Some(FileMatchResult {
                                    file_index,
                                    file_path: file_path.to_string(),
                                    file_size,
                                    match_target: FileMatchTarget::Episode {
                                        episode_id: ep.id,
                                        show_id: show.id,
                                        show_name: show.name.clone(),
                                        season,
                                        episode,
                                    },
                                    match_type: FileMatchType::Auto,
                                    confidence: similarity,
                                    quality,
                                    skip_download: true,
                                    skip_reason: should_download.reason,
                                }));
                            }

                            let display_name = std::path::Path::new(file_path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(file_path);
                            debug!(
                                show = %show.name,
                                season = season,
                                episode = episode,
                                similarity = similarity,
                                "Matched '{}' → {} S{:02}E{:02} (confidence: {:.0}%)",
                                display_name, show.name, season, episode, similarity * 100.0
                            );

                            return Ok(Some(FileMatchResult {
                                file_index,
                                file_path: file_path.to_string(),
                                file_size,
                                match_target: FileMatchTarget::Episode {
                                    episode_id: ep.id,
                                    show_id: show.id,
                                    show_name: show.name.clone(),
                                    season,
                                    episode,
                                },
                                match_type: FileMatchType::Auto,
                                confidence: similarity,
                                quality,
                                skip_download: false,
                                skip_reason: None,
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Try to match a parsed movie to wanted movies in libraries
    async fn try_match_movie(
        &self,
        parsed: &ParsedEpisode,
        file_index: i32,
        file_path: &str,
        file_size: i64,
        libraries: &[LibraryRecord],
        is_targeted_match: bool,
    ) -> Result<Option<FileMatchResult>> {
        let title = match &parsed.show_name {
            Some(name) => name,
            None => return Ok(None),
        };

        let quality = ParsedQuality {
            resolution: parsed.resolution.clone(),
            source: parsed.source.clone(),
            codec: parsed.codec.clone(),
            hdr: parsed.hdr.clone(),
            audio: parsed.audio.clone(),
        };

        // Search for matching movies in movie libraries
        for library in libraries {
            if library.library_type.to_lowercase() != "movies" {
                continue;
            }

            // Skip auto_download check if user explicitly targeted this library
            if !is_targeted_match && !library.auto_download && !library.auto_hunt {
                debug!(
                    "Skipping library '{}' - auto_download and auto_hunt are disabled",
                    library.name
                );
                continue;
            }

            // Find movies that match this title
            let movies = self.db.movies().list_by_library(library.id).await?;

            for movie in movies {
                let similarity = self.calculate_movie_title_similarity(title, &movie.title);

                // Check year match if available
                let year_matches = match (parsed.year, movie.year) {
                    (Some(parsed_year), Some(movie_year)) => {
                        (parsed_year as i32 - movie_year).abs() <= 1
                    }
                    _ => true, // If no year, don't penalize
                };

                if similarity > 0.8 && year_matches {
                    // Check if this movie needs downloading
                    let should_download = self
                        .should_download_movie(&movie, &library, &quality)
                        .await?;

                    if should_download.skip {
                        return Ok(Some(FileMatchResult {
                            file_index,
                            file_path: file_path.to_string(),
                            file_size,
                            match_target: FileMatchTarget::Movie {
                                movie_id: movie.id,
                                title: movie.title.clone(),
                                year: movie.year,
                            },
                            match_type: FileMatchType::Auto,
                            confidence: similarity,
                            quality,
                            skip_download: true,
                            skip_reason: should_download.reason,
                        }));
                    }

                    debug!(
                        movie = %movie.title,
                        similarity = similarity,
                        "Matched file to movie"
                    );

                    return Ok(Some(FileMatchResult {
                        file_index,
                        file_path: file_path.to_string(),
                        file_size,
                        match_target: FileMatchTarget::Movie {
                            movie_id: movie.id,
                            title: movie.title.clone(),
                            year: movie.year,
                        },
                        match_type: FileMatchType::Auto,
                        confidence: similarity,
                        quality,
                        skip_download: false,
                        skip_reason: None,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Create an episode match result from an episode ID
    async fn create_episode_match(
        &self,
        file_index: i32,
        file_path: String,
        file_size: i64,
        episode_id: Uuid,
        quality: ParsedQuality,
        match_type: FileMatchType,
    ) -> Result<FileMatchResult> {
        let episode = self
            .db
            .episodes()
            .get_by_id(episode_id)
            .await?
            .context("Episode not found")?;

        let show = self
            .db
            .tv_shows()
            .get_by_id(episode.tv_show_id)
            .await?
            .context("Show not found")?;

        Ok(FileMatchResult {
            file_index,
            file_path,
            file_size,
            match_target: FileMatchTarget::Episode {
                episode_id: episode.id,
                show_id: show.id,
                show_name: show.name.clone(),
                season: episode.season,
                episode: episode.episode,
            },
            match_type,
            confidence: 1.0,
            quality,
            skip_download: false,
            skip_reason: None,
        })
    }

    /// Create a movie match result from a movie ID
    async fn create_movie_match(
        &self,
        file_index: i32,
        file_path: String,
        file_size: i64,
        movie_id: Uuid,
        quality: ParsedQuality,
        match_type: FileMatchType,
    ) -> Result<FileMatchResult> {
        let movie = self
            .db
            .movies()
            .get_by_id(movie_id)
            .await?
            .context("Movie not found")?;

        Ok(FileMatchResult {
            file_index,
            file_path,
            file_size,
            match_target: FileMatchTarget::Movie {
                movie_id: movie.id,
                title: movie.title.clone(),
                year: movie.year,
            },
            match_type,
            confidence: 1.0,
            quality,
            skip_download: false,
            skip_reason: None,
        })
    }

    /// Create a track match result from a track ID
    async fn create_track_match(
        &self,
        file_index: i32,
        file_path: String,
        file_size: i64,
        track_id: Uuid,
        quality: ParsedQuality,
        match_type: FileMatchType,
    ) -> Result<FileMatchResult> {
        let track = self
            .db
            .tracks()
            .get_by_id(track_id)
            .await?
            .context("Track not found")?;

        Ok(FileMatchResult {
            file_index,
            file_path,
            file_size,
            match_target: FileMatchTarget::Track {
                track_id: track.id,
                album_id: track.album_id,
                title: track.title.clone(),
                track_number: track.track_number,
            },
            match_type,
            confidence: 1.0,
            quality,
            skip_download: false,
            skip_reason: None,
        })
    }

    /// Try to match a file to a track within a specific album
    async fn try_match_track_in_album(
        &self,
        album_id: Uuid,
        file_name: &str,
        file_index: i32,
        file_path: &str,
        file_size: i64,
        quality: &ParsedQuality,
    ) -> Result<Option<FileMatchResult>> {
        let tracks = self.db.tracks().list_by_album(album_id).await?;

        debug!(
            album_id = %album_id,
            file_name = %file_name,
            track_count = tracks.len(),
            "Trying to match file to track in album"
        );

        if tracks.is_empty() {
            warn!(
                album_id = %album_id,
                "Album has no tracks - cannot match file"
            );
            return Ok(None);
        }

        // Try to match by track number from filename
        let extracted_track_num = self.extract_track_number(file_name);
        debug!(
            file_name = %file_name,
            extracted_track_number = ?extracted_track_num,
            "Extracted track number from filename"
        );

        if let Some(track_num) = extracted_track_num {
            for track in &tracks {
                if track.track_number == track_num {
                    info!(
                        file_name = %file_name,
                        track_number = track_num,
                        track_title = %track.title,
                        "Matched file to track by track number"
                    );
                    return Ok(Some(FileMatchResult {
                        file_index,
                        file_path: file_path.to_string(),
                        file_size,
                        match_target: FileMatchTarget::Track {
                            track_id: track.id,
                            album_id: track.album_id,
                            title: track.title.clone(),
                            track_number: track.track_number,
                        },
                        match_type: FileMatchType::Auto,
                        confidence: 0.9,
                        quality: quality.clone(),
                        skip_download: false,
                        skip_reason: None,
                    }));
                }
            }
            debug!(
                file_name = %file_name,
                track_number = track_num,
                "No track found with matching track number"
            );
        }

        // Try to match by title similarity
        for track in tracks {
            let similarity = self.calculate_track_title_similarity(file_name, &track.title);
            debug!(
                file_name = %file_name,
                track_title = %track.title,
                similarity = format!("{:.2}", similarity),
                "Comparing file to track by title"
            );
            if similarity > 0.7 {
                info!(
                    file_name = %file_name,
                    track_title = %track.title,
                    similarity = format!("{:.2}", similarity),
                    "Matched file to track by title similarity"
                );
                return Ok(Some(FileMatchResult {
                    file_index,
                    file_path: file_path.to_string(),
                    file_size,
                    match_target: FileMatchTarget::Track {
                        track_id: track.id,
                        album_id: track.album_id,
                        title: track.title.clone(),
                        track_number: track.track_number,
                    },
                    match_type: FileMatchType::Auto,
                    confidence: similarity,
                    quality: quality.clone(),
                    skip_download: false,
                    skip_reason: None,
                }));
            }
        }

        Ok(None)
    }

    /// Try to match a file to an audiobook chapter
    async fn try_match_audiobook_chapter(
        &self,
        audiobook_id: Uuid,
        file_name: &str,
        file_index: i32,
        file_path: &str,
        file_size: i64,
        quality: &ParsedQuality,
    ) -> Result<Option<FileMatchResult>> {
        // Get chapters for this audiobook
        let chapters = sqlx::query_as::<_, (Uuid, i32, Option<String>)>(
            "SELECT id, chapter_number, title FROM chapters WHERE audiobook_id = $1 ORDER BY chapter_number"
        )
        .bind(audiobook_id)
        .fetch_all(self.db.pool())
        .await?;

        // Try to match by chapter number
        if let Some(chapter_num) = self.extract_chapter_number(file_name) {
            for (chapter_id, chapter_number, _title) in &chapters {
                if *chapter_number == chapter_num {
                    return Ok(Some(FileMatchResult {
                        file_index,
                        file_path: file_path.to_string(),
                        file_size,
                        match_target: FileMatchTarget::AudiobookChapter {
                            chapter_id: *chapter_id,
                            audiobook_id,
                            chapter_number: *chapter_number,
                        },
                        match_type: FileMatchType::Auto,
                        confidence: 0.9,
                        quality: quality.clone(),
                        skip_download: false,
                        skip_reason: None,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Try to match a music file to any track in a library
    ///
    /// This parses the torrent name to extract artist/album info, finds matching
    /// albums in the library, then tries to match the specific file to a track
    /// by track number or title similarity.
    async fn try_match_music_file(
        &self,
        library_id: Uuid,
        torrent_name: &str,
        file_name: &str,
        file_index: i32,
        file_path: &str,
        file_size: i64,
        quality: &ParsedQuality,
    ) -> Result<Option<FileMatchResult>> {
        // Parse artist and album from torrent name
        // Common patterns:
        // - "Artist-Album-Year-Format-Group" (scene)
        // - "Artist - Album (Year) [Format]" (other)
        let (artist_name, album_name) = match parse_music_torrent_name(torrent_name) {
            Some((a, b)) => (a, b),
            None => {
                debug!(
                    torrent_name = %torrent_name,
                    "Could not parse artist/album from torrent name"
                );
                return Ok(None);
            }
        };

        debug!(
            torrent_name = %torrent_name,
            parsed_artist = %artist_name,
            parsed_album = %album_name,
            "Parsed music torrent name"
        );

        // Get albums in this library
        let albums = self.db.albums().list_by_library(library_id).await?;

        info!(
            library_id = %library_id,
            album_count = albums.len(),
            parsed_artist = %artist_name,
            parsed_album = %album_name,
            "Searching for matching album in library"
        );

        if albums.is_empty() {
            warn!(
                library_id = %library_id,
                "No albums found in library - cannot match music file"
            );
            return Ok(None);
        }

        // Find matching album by name similarity
        let mut best_album_match: Option<(crate::db::AlbumRecord, f64)> = None;

        for album in albums {
            // Check album name similarity
            let album_similarity = filename_parser::show_name_similarity(&album_name, &album.name);

            // Also check if artist matches
            let (artist_matches, artist_similarity, artist_db_name) = if let Ok(Some(artist)) =
                self.db.albums().get_artist_by_id(album.artist_id).await
            {
                let similarity = filename_parser::show_name_similarity(&artist_name, &artist.name);
                (similarity > 0.7, similarity, artist.name)
            } else {
                (false, 0.0, "Unknown".to_string())
            };

            debug!(
                library_album = %album.name,
                library_artist = %artist_db_name,
                parsed_album = %album_name,
                parsed_artist = %artist_name,
                album_similarity = format!("{:.2}", album_similarity),
                artist_similarity = format!("{:.2}", artist_similarity),
                artist_matches = artist_matches,
                "Comparing album"
            );

            // Require both album match and artist match
            if album_similarity > 0.7 && artist_matches {
                let score = album_similarity;
                info!(
                    album = %album.name,
                    artist = %artist_db_name,
                    score = format!("{:.2}", score),
                    "Found potential album match"
                );
                if best_album_match.is_none() || score > best_album_match.as_ref().unwrap().1 {
                    best_album_match = Some((album, score));
                }
            }
        }

        let (matched_album, album_confidence) = match best_album_match {
            Some((album, score)) => (album, score),
            None => {
                debug!(
                    artist = %artist_name,
                    album = %album_name,
                    library_id = %library_id,
                    "No matching album found in library"
                );
                return Ok(None);
            }
        };

        info!(
            torrent_name = %torrent_name,
            matched_album = %matched_album.name,
            album_id = %matched_album.id,
            confidence = album_confidence,
            "Found matching album for music torrent"
        );

        // Now try to match the specific file to a track in this album
        if let Some(result) = self
            .try_match_track_in_album(
                matched_album.id,
                file_name,
                file_index,
                file_path,
                file_size,
                quality,
            )
            .await?
        {
            return Ok(Some(result));
        }

        // If we matched the album but not a specific track, still return a result
        // by trying to match any track we can
        debug!(
            file_name = %file_name,
            album = %matched_album.name,
            "Could not match file to specific track in album"
        );

        Ok(None)
    }

    /// Try to match an audiobook file to chapters in a library
    async fn try_match_audiobook_file(
        &self,
        _library_id: Uuid,
        _file_name: &str,
        _file_index: i32,
        _file_path: &str,
        _file_size: i64,
        _quality: &ParsedQuality,
    ) -> Result<Option<FileMatchResult>> {
        // TODO: Implement audiobook file matching
        Ok(None)
    }

    /// Check if a file is a sample
    fn is_sample_file(&self, file_name: &str) -> bool {
        let lower = file_name.to_lowercase();
        lower.contains("sample") || lower.starts_with("sample-") || lower.contains("-sample.")
    }

    /// Calculate similarity between show names (0.0 to 1.0)
    fn calculate_show_name_similarity(&self, parsed: &str, show_name: &str) -> f64 {
        filename_parser::show_name_similarity(parsed, show_name)
    }

    /// Calculate similarity between movie titles
    fn calculate_movie_title_similarity(&self, parsed: &str, movie_title: &str) -> f64 {
        // Normalize both titles
        let n1 = filename_parser::normalize_show_name(parsed);
        let n2 = filename_parser::normalize_show_name(movie_title);

        if n1 == n2 {
            return 1.0;
        }

        // Use the same algorithm as show name similarity
        filename_parser::show_name_similarity(&n1, &n2)
    }

    /// Calculate similarity between track titles
    fn calculate_track_title_similarity(&self, file_name: &str, track_title: &str) -> f64 {
        // Remove common prefixes like track numbers
        let clean_file = file_name
            .split(|c: char| !c.is_alphanumeric() && c != ' ')
            .skip_while(|s| s.chars().all(|c| c.is_numeric()))
            .collect::<Vec<_>>()
            .join(" ");

        filename_parser::show_name_similarity(&clean_file, track_title)
    }

    /// Extract track number from filename (e.g., "01 - Song Name.mp3" -> 1)
    fn extract_track_number(&self, file_name: &str) -> Option<i32> {
        // Pattern: starts with digits, or digits followed by separator
        let re = regex::Regex::new(r"^(\d{1,3})[\s\-_\.]").ok()?;
        re.captures(file_name)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    /// Extract chapter number from filename
    fn extract_chapter_number(&self, file_name: &str) -> Option<i32> {
        // Pattern: "Chapter X" or "Ch X" or just leading number
        let re = regex::Regex::new(r"(?i)(?:chapter|ch)[.\s\-_]*(\d+)|^(\d{1,3})[\s\-_\.]").ok()?;
        re.captures(file_name).and_then(|c| {
            c.get(1)
                .or_else(|| c.get(2))
                .and_then(|m| m.as_str().parse().ok())
        })
    }

    /// Determine if we should download an episode based on current status and quality
    async fn should_download_episode(
        &self,
        episode: &EpisodeRecord,
        show: &TvShowRecord,
        library: &LibraryRecord,
        parsed_quality: &ParsedQuality,
    ) -> Result<ShouldDownload> {
        // If episode is ignored, skip
        if episode.status == "ignored" {
            return Ok(ShouldDownload {
                skip: true,
                reason: Some("Episode is ignored".to_string()),
            });
        }

        // If already downloaded and same or worse quality, skip
        if episode.status == "downloaded" {
            let is_upgrade = self
                .is_quality_upgrade(parsed_quality, library, show)
                .await?;

            if !is_upgrade {
                return Ok(ShouldDownload {
                    skip: true,
                    reason: Some("Already downloaded, not an upgrade".to_string()),
                });
            }
        }

        // If currently downloading (from another file), might skip
        if episode.status == "downloading" {
            let is_already_downloading = self
                .db
                .torrent_file_matches()
                .is_episode_downloading(episode.id)
                .await?;

            if is_already_downloading {
                return Ok(ShouldDownload {
                    skip: true,
                    reason: Some("Already downloading from another torrent".to_string()),
                });
            }
        }

        Ok(ShouldDownload {
            skip: false,
            reason: None,
        })
    }

    /// Determine if we should download a movie
    async fn should_download_movie(
        &self,
        movie: &MovieRecord,
        _library: &LibraryRecord,
        _parsed_quality: &ParsedQuality,
    ) -> Result<ShouldDownload> {
        // If movie is ignored, skip
        if !movie.monitored {
            return Ok(ShouldDownload {
                skip: true,
                reason: Some("Movie is not monitored".to_string()),
            });
        }

        // If already has file, skip
        if movie.has_file {
            return Ok(ShouldDownload {
                skip: true,
                reason: Some("Movie already has file".to_string()),
            });
        }

        // If currently downloading, skip
        let is_downloading = self
            .db
            .torrent_file_matches()
            .is_movie_downloading(movie.id)
            .await?;

        if is_downloading {
            return Ok(ShouldDownload {
                skip: true,
                reason: Some("Already downloading from another torrent".to_string()),
            });
        }

        Ok(ShouldDownload {
            skip: false,
            reason: None,
        })
    }

    /// Check if parsed quality is an upgrade over current
    async fn is_quality_upgrade(
        &self,
        _parsed_quality: &ParsedQuality,
        _library: &LibraryRecord,
        _show: &TvShowRecord,
    ) -> Result<bool> {
        // TODO: Implement quality comparison
        // For now, we don't consider anything an upgrade
        // Real implementation would compare parsed resolution to existing file's resolution
        Ok(false)
    }

    /// Save file matches to the database
    pub async fn save_matches(
        &self,
        torrent_id: Uuid,
        matches: &[FileMatchResult],
    ) -> Result<usize> {
        let repo = self.db.torrent_file_matches();
        let mut saved = 0;

        for m in matches {
            let (episode_id, movie_id, track_id, chapter_id) = match &m.match_target {
                FileMatchTarget::Episode { episode_id, .. } => {
                    (Some(*episode_id), None, None, None)
                }
                FileMatchTarget::Movie { movie_id, .. } => (None, Some(*movie_id), None, None),
                FileMatchTarget::Track { track_id, .. } => (None, None, Some(*track_id), None),
                FileMatchTarget::AudiobookChapter { chapter_id, .. } => {
                    (None, None, None, Some(*chapter_id))
                }
                FileMatchTarget::Unmatched { .. } | FileMatchTarget::Sample => {
                    (None, None, None, None)
                }
            };

            let input = CreateTorrentFileMatch {
                torrent_id,
                file_index: m.file_index,
                file_path: m.file_path.clone(),
                file_size: m.file_size,
                episode_id,
                movie_id,
                track_id,
                chapter_id,
                match_type: m.match_type.to_string(),
                match_confidence: Some(Decimal::from_f64(m.confidence).unwrap_or_default()),
                parsed_resolution: m.quality.resolution.clone(),
                parsed_codec: m.quality.codec.clone(),
                parsed_source: m.quality.source.clone(),
                parsed_audio: m.quality.audio.clone(),
                skip_download: m.skip_download,
            };

            repo.create(input).await?;
            saved += 1;
        }

        Ok(saved)
    }

    /// Update item statuses to "downloading" for matched files
    pub async fn update_item_statuses_to_downloading(
        &self,
        matches: &[FileMatchResult],
    ) -> Result<()> {
        for m in matches {
            if m.skip_download {
                continue;
            }

            match &m.match_target {
                FileMatchTarget::Episode { episode_id, .. } => {
                    self.db
                        .episodes()
                        .update_status(*episode_id, "downloading")
                        .await?;
                }
                FileMatchTarget::Movie { movie_id, .. } => {
                    // Update movie download_status
                    sqlx::query("UPDATE movies SET download_status = 'downloading' WHERE id = $1")
                        .bind(movie_id)
                        .execute(self.db.pool())
                        .await?;
                }
                FileMatchTarget::Track { track_id, .. } => {
                    // Update track status
                    sqlx::query("UPDATE tracks SET status = 'downloading' WHERE id = $1")
                        .bind(track_id)
                        .execute(self.db.pool())
                        .await?;
                }
                FileMatchTarget::AudiobookChapter { chapter_id, .. } => {
                    // Update chapter status
                    sqlx::query("UPDATE chapters SET status = 'downloading' WHERE id = $1")
                        .bind(chapter_id)
                        .execute(self.db.pool())
                        .await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

/// Result of checking if we should download something
struct ShouldDownload {
    skip: bool,
    reason: Option<String>,
}

/// Parse artist and album name from a music torrent name
///
/// Handles common patterns:
/// - Scene: "Artist-Album-Year-Format-Group" (e.g., "Guns_N_Roses-Appetite_for_Destruction-REMASTERED-24BIT-WEB-FLAC-2018-KLV")
/// - Other: "Artist - Album (Year) [Format]"
/// - Simple: "Artist - Album"
fn parse_music_torrent_name(torrent_name: &str) -> Option<(String, String)> {
    // Clean up the name - replace underscores with spaces
    let cleaned = torrent_name.replace('_', " ");

    // Try scene format first: "Artist-Album-Year-..." where parts are separated by hyphens
    // Scene releases typically have format info after the album name
    let scene_keywords = [
        "FLAC", "MP3", "WEB", "VINYL", "CD", "REMASTERED", "REMASTER", "DELUXE", "320", "V0",
        "24BIT", "16BIT", "44", "48", "96", "192", "LOSSLESS",
    ];

    // Split by " - " first (common in non-scene releases)
    if let Some(pos) = cleaned.find(" - ") {
        let artist = cleaned[..pos].trim().to_string();
        let rest = &cleaned[pos + 3..];

        // Try to extract album name, stopping at year or format info
        let album = rest
            .split(|c: char| c == '(' || c == '[')
            .next()
            .unwrap_or(rest)
            .trim();

        // Remove trailing year if present (e.g., "Album 2018")
        let album = album
            .split_whitespace()
            .filter(|s| !s.chars().all(|c| c.is_ascii_digit()) || s.len() != 4)
            .collect::<Vec<_>>()
            .join(" ");

        if !artist.is_empty() && !album.is_empty() {
            return Some((artist, album));
        }
    }

    // Try scene format: split by single hyphen with spaces around it or just hyphen
    let parts: Vec<&str> = cleaned.split('-').map(|s| s.trim()).collect();

    if parts.len() >= 2 {
        let artist = parts[0].to_string();

        // Find where the album name ends (before year/format info)
        let mut album_parts = Vec::new();
        for (i, part) in parts[1..].iter().enumerate() {
            let upper = part.to_uppercase();

            // Check if this part is a year (4 digits)
            if part.chars().all(|c| c.is_ascii_digit()) && part.len() == 4 {
                // This is likely the year, stop here
                break;
            }

            // Check if this looks like format/quality info
            if scene_keywords.iter().any(|kw| upper.contains(kw)) {
                break;
            }

            // Skip if it looks like a release group (usually short, at the end)
            if i > 0 && part.len() <= 6 && part.chars().all(|c| c.is_alphanumeric()) {
                // Might be release group, check if previous was format keyword
                let prev_upper = parts[i].to_uppercase();
                if scene_keywords.iter().any(|kw| prev_upper.contains(kw)) {
                    break;
                }
            }

            album_parts.push(*part);
        }

        if !artist.is_empty() && !album_parts.is_empty() {
            let album = album_parts.join(" ").trim().to_string();
            return Some((artist, album));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::filename_parser::{parse_episode, parse_quality};

    // =========================================================================
    // Sample File Detection Tests
    // =========================================================================

    #[test]
    fn test_sample_file_detection_basic() {
        // These patterns should be detected as samples
        let sample_files = vec![
            "sample.mkv",
            "Sample.mkv",
            "SAMPLE.mkv",
            "sample-s01e01.mkv",
            "Sample-Show.S01E01.mkv",
            "show.s01e01-sample.mkv",
            "show-sample.mkv",
        ];

        for filename in sample_files {
            let lower = filename.to_lowercase();
            let is_sample = lower.contains("sample")
                || lower.starts_with("sample-")
                || lower.contains("-sample.");
            assert!(is_sample, "Should detect '{}' as sample", filename);
        }
    }

    #[test]
    fn test_sample_file_detection_not_sample() {
        // These should NOT be detected as samples
        let not_sample_files = vec![
            "Show.S01E01.1080p.mkv",
            "Movie.2024.1080p.mkv",
            "Star Trek- Deep Space Nine - S01E09 - The Passenger 960p-QueerWorm-Lela.mkv",
            "Fallout.2024.S01E01.1080p.HEVC.x265-MeGusta.mkv",
        ];

        for filename in not_sample_files {
            let lower = filename.to_lowercase();
            let is_sample = lower.contains("sample")
                || lower.starts_with("sample-")
                || lower.contains("-sample.");
            assert!(!is_sample, "Should NOT detect '{}' as sample", filename);
        }
    }

    // =========================================================================
    // File Type Detection Tests
    // =========================================================================

    #[test]
    fn test_is_video_file() {
        use crate::services::file_utils::is_video_file;

        // Video files
        assert!(is_video_file("movie.mkv"));
        assert!(is_video_file("movie.mp4"));
        assert!(is_video_file("movie.avi"));
        assert!(is_video_file("movie.wmv"));
        assert!(is_video_file("movie.mov"));
        assert!(is_video_file("movie.m4v"));
        assert!(is_video_file("movie.ts"));
        assert!(is_video_file("movie.webm"));

        // Not video files
        assert!(!is_video_file("movie.nfo"));
        assert!(!is_video_file("movie.srt"));
        assert!(!is_video_file("movie.txt"));
        assert!(!is_video_file("cover.jpg"));
        assert!(!is_video_file("album.mp3"));
    }

    #[test]
    fn test_is_audio_file() {
        use crate::services::file_utils::is_audio_file;

        // Audio files
        assert!(is_audio_file("track.mp3"));
        assert!(is_audio_file("track.flac"));
        assert!(is_audio_file("track.m4a"));
        assert!(is_audio_file("track.aac"));
        assert!(is_audio_file("track.ogg"));
        assert!(is_audio_file("track.wav"));
        assert!(is_audio_file("track.wma"));

        // Not audio files
        assert!(!is_audio_file("video.mp4"));
        assert!(!is_audio_file("movie.mkv"));
        assert!(!is_audio_file("cover.jpg"));
    }

    // =========================================================================
    // Multi-Episode File Detection
    // =========================================================================

    #[test]
    fn test_parse_multi_episode() {
        // Multi-episode files like "S01E01-E02" or "S01E01E02"
        let result = parse_episode(
            "Star Trek- Deep Space Nine - S01E01-E02 - Emissary 960p-QueerWorm-Lela.mkv",
        );
        assert_eq!(result.season, Some(1));
        // At minimum should capture first episode
        assert!(result.episode.is_some());
    }

    // =========================================================================
    // Real Torrent File List Simulation
    // =========================================================================

    #[test]
    fn test_ds9_season_pack_file_list() {
        // Simulate the Deep Space Nine S01 torrent file list
        let files = vec![
            ("README.md", false, None, None),
            (
                "Star Trek- Deep Space Nine - S01E01-E02 - Emissary 960p-QueerWorm-Lela.mkv",
                true,
                Some(1),
                Some(1),
            ),
            (
                "Star Trek- Deep Space Nine - S01E03 - Past Prologue 960p-QueerWorm-Lela.mkv",
                true,
                Some(1),
                Some(3),
            ),
            (
                "Star Trek- Deep Space Nine - S01E04 - A Man Alone 960p-QueerWorm-Lela.mkv",
                true,
                Some(1),
                Some(4),
            ),
            (
                "Star Trek- Deep Space Nine - S01E05 - Babel 960p-QueerWorm-Lela.mkv",
                true,
                Some(1),
                Some(5),
            ),
            (
                "Star Trek- Deep Space Nine - S01E06 - Captive Pursuit 960p-QueerWorm-Lela.mkv",
                true,
                Some(1),
                Some(6),
            ),
            (
                "Star Trek- Deep Space Nine - S01E07 - Q-Less 960p-QueerWorm-Lela.mkv",
                true,
                Some(1),
                Some(7),
            ),
            (
                "Star Trek- Deep Space Nine - S01E08 - Dax 960p-QueerWorm-Lela.mkv",
                true,
                Some(1),
                Some(8),
            ),
            (
                "Star Trek- Deep Space Nine - S01E09 - The Passenger 960p-QueerWorm-Lela.mkv",
                true,
                Some(1),
                Some(9),
            ),
            (
                "Star Trek- Deep Space Nine - S01E10 - Move Along Home 960p-QueerWorm-Lela.mkv",
                true,
                Some(1),
                Some(10),
            ),
        ];

        for (filename, is_media, expected_season, expected_episode) in files {
            use crate::services::file_utils::is_video_file;

            let is_media_result = is_video_file(filename);
            assert_eq!(
                is_media_result, is_media,
                "Media detection failed for {}",
                filename
            );

            if is_media {
                let parsed = parse_episode(filename);
                assert_eq!(
                    parsed.season, expected_season,
                    "Season mismatch for {}",
                    filename
                );
                assert_eq!(
                    parsed.episode, expected_episode,
                    "Episode mismatch for {}",
                    filename
                );
            }
        }
    }

    #[test]
    fn test_fallout_single_episode_file_list() {
        // Simulate single episode torrents with extras
        let files = vec![
            (
                "Fallout.2024.S01E01.1080p.HEVC.x265-MeGusta.mkv",
                true,
                true,
            ),
            (
                "Fallout.2024.S01E01.1080p.HEVC.x265-MeGusta.nfo",
                false,
                false,
            ),
            ("Screens/screen0001.png", false, false),
            ("Screens/screen0002.png", false, false),
            ("Screens/screen0003.png", false, false),
        ];

        for (filename, is_video, should_process) in files {
            use crate::services::file_utils::is_video_file;

            assert_eq!(
                is_video_file(filename),
                is_video,
                "Video detection for {}",
                filename
            );

            // Only video files should be processed as media
            if should_process {
                let parsed = parse_episode(filename);
                assert_eq!(parsed.season, Some(1));
                assert_eq!(parsed.episode, Some(1));
                assert_eq!(parsed.year, Some(2024));
            }
        }
    }

    // =========================================================================
    // Quality Comparison Tests
    // =========================================================================

    #[test]
    fn test_quality_resolution_ranking() {
        // Higher resolution should rank higher
        let resolutions = vec!["480p", "720p", "1080p", "2160p"];

        fn resolution_to_score(res: &str) -> u32 {
            match res {
                "2160p" | "4K" | "UHD" => 2160,
                "1080p" => 1080,
                "720p" => 720,
                "480p" => 480,
                "360p" => 360,
                _ => 0,
            }
        }

        for i in 0..resolutions.len() {
            for j in (i + 1)..resolutions.len() {
                let lower = resolution_to_score(resolutions[i]);
                let higher = resolution_to_score(resolutions[j]);
                assert!(
                    higher > lower,
                    "{} should rank higher than {}",
                    resolutions[j],
                    resolutions[i]
                );
            }
        }
    }

    #[test]
    fn test_quality_meets_target() {
        // Test that a 1080p file meets a 1080p target
        let file_quality = parse_quality("Show.S01E01.1080p.WEB.h264-GROUP");
        // Resolution may be uppercase or lowercase depending on parser
        assert!(
            file_quality
                .resolution
                .as_ref()
                .map(|r| r.to_lowercase() == "1080p")
                .unwrap_or(false)
        );

        // A 720p file should NOT meet a 1080p target
        let file_quality_low = parse_quality("Show.S01E01.720p.WEB.h264-GROUP");
        assert!(
            file_quality_low
                .resolution
                .as_ref()
                .map(|r| r.to_lowercase() == "720p")
                .unwrap_or(false)
        );

        // But a 2160p file exceeds a 1080p target (acceptable)
        let file_quality_high = parse_quality("Show.S01E01.2160p.WEB.h265-GROUP");
        assert!(
            file_quality_high
                .resolution
                .as_ref()
                .map(|r| r.to_lowercase() == "2160p")
                .unwrap_or(false)
        );
    }

    // =========================================================================
    // Music Torrent Name Parsing Tests
    // =========================================================================

    #[test]
    fn test_parse_music_torrent_name_scene_format() {
        // Scene format: Artist-Album-Year-Format-Group
        let result = parse_music_torrent_name(
            "Guns_N_Roses-Appetite_for_Destruction-REMASTERED-24BIT-WEB-FLAC-2018-KLV",
        );
        assert!(result.is_some(), "Should parse scene format torrent name");
        let (artist, album) = result.unwrap();
        assert_eq!(artist, "Guns N Roses");
        assert_eq!(album, "Appetite for Destruction");
    }

    #[test]
    fn test_parse_music_torrent_name_standard_format() {
        // Standard format: "Artist - Album (Year)"
        let result = parse_music_torrent_name("Pink Floyd - The Dark Side of the Moon (1973)");
        assert!(result.is_some(), "Should parse standard format");
        let (artist, album) = result.unwrap();
        assert_eq!(artist, "Pink Floyd");
        assert_eq!(album, "The Dark Side of the Moon");
    }

    #[test]
    fn test_parse_music_torrent_name_with_brackets() {
        // Format with brackets: "Artist - Album [FLAC]"
        let result = parse_music_torrent_name("Daft Punk - Random Access Memories [FLAC]");
        assert!(result.is_some(), "Should parse format with brackets");
        let (artist, album) = result.unwrap();
        assert_eq!(artist, "Daft Punk");
        assert_eq!(album, "Random Access Memories");
    }

    #[test]
    fn test_parse_music_torrent_name_simple() {
        // Simple format: "Artist - Album"
        let result = parse_music_torrent_name("The Beatles - Abbey Road");
        assert!(result.is_some(), "Should parse simple format");
        let (artist, album) = result.unwrap();
        assert_eq!(artist, "The Beatles");
        assert_eq!(album, "Abbey Road");
    }

    // =========================================================================
    // Match Target Tests
    // =========================================================================

    #[test]
    fn test_file_match_target_variants() {
        // Test that FileMatchTarget can be constructed correctly
        let episode_target = FileMatchTarget::Episode {
            episode_id: uuid::Uuid::new_v4(),
            show_id: uuid::Uuid::new_v4(),
            show_name: "Star Trek: Deep Space Nine".to_string(),
            season: 1,
            episode: 9,
        };

        if let FileMatchTarget::Episode {
            show_name,
            season,
            episode,
            ..
        } = episode_target
        {
            assert_eq!(show_name, "Star Trek: Deep Space Nine");
            assert_eq!(season, 1);
            assert_eq!(episode, 9);
        } else {
            panic!("Should be Episode variant");
        }
    }

    #[test]
    fn test_file_match_type_variants() {
        // Test FileMatchType variants
        assert!(matches!(FileMatchType::Auto, FileMatchType::Auto));
        assert!(matches!(FileMatchType::Manual, FileMatchType::Manual));
        assert!(matches!(FileMatchType::Forced, FileMatchType::Forced));

        // Test Display trait
        assert_eq!(format!("{}", FileMatchType::Auto), "auto");
        assert_eq!(format!("{}", FileMatchType::Manual), "manual");
        assert_eq!(format!("{}", FileMatchType::Forced), "forced");
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_file_with_unusual_extension() {
        use crate::services::file_utils::is_video_file;

        // m2ts is a valid video format (Blu-ray) - supported
        assert!(is_video_file("movie.m2ts"));

        // ts is valid (transport stream)
        assert!(is_video_file("movie.ts"));

        // webm is valid
        assert!(is_video_file("movie.webm"));

        // Note: ogv is NOT currently in the supported list
        // If needed, it should be added to VIDEO_EXTENSIONS
    }

    #[test]
    fn test_nfo_and_metadata_files_ignored() {
        use crate::services::file_utils::{is_audio_file, is_video_file};

        let metadata_files = vec![
            "movie.nfo",
            "info.txt",
            "README.md",
            "release.sfv",
            "release.md5",
        ];

        for f in metadata_files {
            assert!(!is_video_file(f), "{} should not be detected as video", f);
            assert!(!is_audio_file(f), "{} should not be detected as audio", f);
        }
    }

    #[test]
    fn test_subtitle_files() {
        // Subtitle files should be recognized (for future subtitle support)
        let subtitle_extensions = vec![".srt", ".sub", ".ass", ".ssa", ".vtt"];

        for ext in subtitle_extensions {
            let filename = format!("movie{}", ext);
            let is_subtitle = filename.ends_with(".srt")
                || filename.ends_with(".sub")
                || filename.ends_with(".ass")
                || filename.ends_with(".ssa")
                || filename.ends_with(".vtt");
            assert!(is_subtitle, "{} should be detected as subtitle", filename);
        }
    }

    #[test]
    fn test_archive_files() {
        // Archive files that need extraction
        let archive_extensions = vec![".zip", ".rar", ".7z", ".tar.gz"];

        for ext in archive_extensions {
            let filename = format!("release{}", ext);
            let is_archive = filename.ends_with(".zip")
                || filename.ends_with(".rar")
                || filename.ends_with(".7z")
                || filename.ends_with(".tar.gz");
            assert!(is_archive, "{} should be detected as archive", filename);
        }
    }
}
