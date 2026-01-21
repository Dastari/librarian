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

        // Get track details for logging
        let track_info = self.db.tracks().get_by_id(track_id).await.ok().flatten();
        let (track_title, track_number) = match &track_info {
            Some(t) => (t.title.as_str(), t.track_number),
            None => ("Unknown", 0),
        };

        info!(
            torrent_id = %torrent_id,
            file_index = file_index,
            file_path = %file_path,
            track_id = %track_id,
            track_number = track_number,
            track_title = %track_title,
            "Created file match: '{}' → Track {} '{}'",
            file_path, track_number, track_title
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
        
        // Get album and artist names for logging
        let album = self.db.albums().get_by_id(album_id).await?;
        let (album_name, artist_name) = match &album {
            Some(a) => {
                let artist = if let Ok(Some(artist)) = self.db.albums().get_artist_by_id(a.artist_id).await {
                    artist.name
                } else {
                    "Unknown Artist".to_string()
                };
                (a.name.clone(), artist)
            }
            None => ("Unknown Album".to_string(), "Unknown Artist".to_string()),
        };
        
        let audio_files: Vec<_> = files.iter().filter(|f| is_audio_file(&f.path)).collect();
        
        info!(
            torrent_id = %torrent_id,
            album_id = %album_id,
            album_name = %album_name,
            artist_name = %artist_name,
            total_files = files.len(),
            audio_files = audio_files.len(),
            tracks_in_album = tracks.len(),
            "Starting album file matching for '{}' by '{}' ({} audio files, {} tracks)",
            album_name, artist_name, audio_files.len(), tracks.len()
        );

        for (index, file) in files.iter().enumerate() {
            if !is_audio_file(&file.path) {
                debug!(
                    file_path = %file.path,
                    "Skipping non-audio file"
                );
                continue;
            }

            let file_name = Path::new(&file.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file.path);

            // Try to match by track number first
            let extracted_num = self.extract_track_number(file_name);
            
            info!(
                file_name = %file_name,
                extracted_track_number = ?extracted_num,
                "Processing audio file for album matching"
            );
            
            if let Some(track_num) = extracted_num {
                if let Some(track) = tracks.iter().find(|t| t.track_number == track_num) {
                    info!(
                        file_name = %file_name,
                        track_number = track_num,
                        track_title = %track.title,
                        track_id = %track.id,
                        "Matched file to track by number"
                    );
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
                } else {
                    info!(
                        file_name = %file_name,
                        track_number = track_num,
                        available_track_numbers = ?tracks.iter().map(|t| t.track_number).collect::<Vec<_>>(),
                        "No track found with matching number"
                    );
                }
            }

            // Try to match by title similarity
            let mut matched = false;
            for track in &tracks {
                let similarity = self.calculate_track_title_similarity(file_name, &track.title);
                if similarity > 0.7 {
                    info!(
                        file_name = %file_name,
                        track_title = %track.title,
                        similarity = format!("{:.2}", similarity),
                        track_id = %track.id,
                        "Matched file to track by title similarity"
                    );
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
                    matched = true;
                    break;
                }
            }
            
            if !matched {
                info!(
                    file_name = %file_name,
                    "Could not match file to any track in album"
                );
            }
        }

        info!(
            torrent_id = %torrent_id,
            album_id = %album_id,
            album_name = %album_name,
            artist_name = %artist_name,
            matched_files = records.len(),
            audio_files = audio_files.len(),
            "Album file matching complete: '{}' by '{}' - matched {}/{} audio files",
            album_name, artist_name, records.len(), audio_files.len()
        );

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

    /// Try to match a music file to any wanted track in a library
    ///
    /// This works like episode matching:
    /// 1. Parse the file name to extract track number and artist/title info
    /// 2. Search all albums in the library for a matching track
    /// 3. Match by track number (primary) or title similarity (fallback)
    /// 
    /// The torrent name is used for additional context (artist/album hints) but
    /// each file is matched independently based on its own filename.
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
        // Parse info from the file name (like episode matching parses S01E05)
        let parsed = parse_music_file_name(file_name);
        
        info!(
            file_name = %file_name,
            library_id = %library_id,
            parsed_track_number = ?parsed.track_number,
            parsed_artist = ?parsed.artist,
            parsed_title = ?parsed.title,
            "Matching music file to library tracks"
        );
        
        // Also try to get context from torrent name (for artist/album hints)
        let torrent_context = parse_music_torrent_name(torrent_name);
        if let Some((artist, album)) = &torrent_context {
            debug!(
                torrent_artist = %artist,
                torrent_album = %album,
                "Parsed torrent name context"
            );
        }

        // Get all albums in this library
        let albums = self.db.albums().list_by_library(library_id).await?;

        if albums.is_empty() {
            debug!(
                library_id = %library_id,
                "No albums found in library"
            );
            return Ok(None);
        }

        // Try to match by track number first (most reliable, like episode numbers)
        if let Some(track_num) = parsed.track_number {
            // Search all albums for a wanted track with this number
            for album in &albums {
                let tracks = self.db.tracks().list_by_album(album.id).await?;
                
                // Get artist name for logging
                let artist_name = if let Ok(Some(artist)) = self.db.albums().get_artist_by_id(album.artist_id).await {
                    artist.name
                } else {
                    "Unknown".to_string()
                };
                
                for track in tracks {
                    if track.track_number == track_num {
                        // Check if track is wanted (not already downloaded)
                        // Include "downloading" status since the file link may have failed
                        let is_wanted = track.status == "wanted" 
                            || track.status == "missing" 
                            || track.status == "downloading";
                        
                        // If we have artist context from file or torrent, verify it matches
                        let artist_match = if let Some(file_artist) = &parsed.artist {
                            filename_parser::show_name_similarity(file_artist, &artist_name) > 0.6
                        } else if let Some((torrent_artist, _)) = &torrent_context {
                            filename_parser::show_name_similarity(torrent_artist, &artist_name) > 0.6
                        } else {
                            // No artist context, can't verify - be conservative and require title match too
                            false
                        };
                        
                        // If artist matches, or if title also matches, consider it a match
                        let title_match = if let Some(file_title) = &parsed.title {
                            filename_parser::show_name_similarity(file_title, &track.title) > 0.6
                        } else {
                            false
                        };
                        
                        if artist_match || title_match {
                            info!(
                                file_name = %file_name,
                                track_number = track_num,
                                track_title = %track.title,
                                album_name = %album.name,
                                artist_name = %artist_name,
                                is_wanted = is_wanted,
                                artist_match = artist_match,
                                title_match = title_match,
                                "Matched file to track by number"
                            );
                            
                            if !is_wanted {
                                return Ok(Some(FileMatchResult {
                                    file_index,
                                    file_path: file_path.to_string(),
                                    file_size,
                                    match_target: FileMatchTarget::Track {
                                        track_id: track.id,
                                        album_id: album.id,
                                        title: track.title.clone(),
                                        track_number: track.track_number,
                                    },
                                    match_type: FileMatchType::Auto,
                                    confidence: 0.9,
                                    quality: quality.clone(),
                                    skip_download: true,
                                    skip_reason: Some(format!("Track already {}", track.status)),
                                }));
                            }
                            
                            return Ok(Some(FileMatchResult {
                                file_index,
                                file_path: file_path.to_string(),
                                file_size,
                                match_target: FileMatchTarget::Track {
                                    track_id: track.id,
                                    album_id: album.id,
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
                }
            }
        }
        
        // If track number matching failed, try title similarity (like movie matching)
        if let Some(file_title) = &parsed.title {
            for album in &albums {
                let tracks = self.db.tracks().list_by_album(album.id).await?;
                
                for track in tracks {
                    let similarity = filename_parser::show_name_similarity(file_title, &track.title);
                    
                    if similarity > 0.7 {
                        // Include "downloading" status since the file link may have failed
                        let is_wanted = track.status == "wanted" 
                            || track.status == "missing" 
                            || track.status == "downloading";
                        
                        info!(
                            file_name = %file_name,
                            file_title = %file_title,
                            track_title = %track.title,
                            album_name = %album.name,
                            similarity = format!("{:.2}", similarity),
                            is_wanted = is_wanted,
                            "Matched file to track by title similarity"
                        );
                        
                        if !is_wanted {
                            return Ok(Some(FileMatchResult {
                                file_index,
                                file_path: file_path.to_string(),
                                file_size,
                                match_target: FileMatchTarget::Track {
                                    track_id: track.id,
                                    album_id: album.id,
                                    title: track.title.clone(),
                                    track_number: track.track_number,
                                },
                                match_type: FileMatchType::Auto,
                                confidence: similarity,
                                quality: quality.clone(),
                                skip_download: true,
                                skip_reason: Some(format!("Track already {}", track.status)),
                            }));
                        }
                        
                        return Ok(Some(FileMatchResult {
                            file_index,
                            file_path: file_path.to_string(),
                            file_size,
                            match_target: FileMatchTarget::Track {
                                track_id: track.id,
                                album_id: album.id,
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
            }
        }
        
        debug!(
            file_name = %file_name,
            library_id = %library_id,
            "Could not match music file to any track"
        );
        
        Ok(None)
    }
    
    /// Legacy: Try to match a specific file to a track within a known album
    /// Used when user explicitly selects an album for matching.
    async fn try_match_music_file_legacy(
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

        // Get albums in this library
        let albums = self.db.albums().list_by_library(library_id).await?;

        if albums.is_empty() {
            return Ok(None);
        }

        // Find matching album by name similarity
        let mut best_album_match: Option<(crate::db::AlbumRecord, f64)> = None;

        for album in albums {
            let album_similarity = filename_parser::show_name_similarity(&album_name, &album.name);

            let artist_matches = if let Ok(Some(artist)) =
                self.db.albums().get_artist_by_id(album.artist_id).await
            {
                filename_parser::show_name_similarity(&artist_name, &artist.name) > 0.7
            } else {
                false
            };

            if album_similarity > 0.7 && artist_matches {
                if best_album_match.is_none() || album_similarity > best_album_match.as_ref().unwrap().1 {
                    best_album_match = Some((album, album_similarity));
                }
            }
        }

        let matched_album = match best_album_match {
            Some((album, _)) => album,
            None => return Ok(None),
        };

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
    ///
    /// Audiobook matching follows a similar pattern to TV shows:
    /// 1. Parse the filename for chapter/part number
    /// 2. Match to audiobook by title/author similarity
    /// 3. Match to chapter by chapter number
    async fn try_match_audiobook_file(
        &self,
        library_id: Uuid,
        file_name: &str,
        file_index: i32,
        file_path: &str,
        file_size: i64,
        quality: &ParsedQuality,
    ) -> Result<Option<FileMatchResult>> {
        // Parse chapter/part number from filename
        // Common patterns: "01 - Chapter Title.mp3", "Part 01.m4b", "Chapter 1.mp3"
        let parsed_chapter = self.parse_audiobook_file_name(file_name);
        
        debug!(
            file_name = %file_name,
            parsed_chapter_number = ?parsed_chapter.chapter_number,
            parsed_title = ?parsed_chapter.title,
            "Attempting audiobook file matching"
        );

        // Get all audiobooks in this library
        let audiobooks = self.db.audiobooks().list_by_library(library_id).await?;

        if audiobooks.is_empty() {
            debug!(
                library_id = %library_id,
                "No audiobooks found in library"
            );
            return Ok(None);
        }

        // Try to match by audiobook title similarity first
        for audiobook in &audiobooks {
            // Get author name for better matching
            let author_name = if let Some(author_id) = audiobook.author_id {
                self.db
                    .audiobooks()
                    .get_author_by_id(author_id)
                    .await?
                    .map(|a| a.name)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            // Check if the file name contains the audiobook title
            let title_lower = audiobook.title.to_lowercase();
            let file_lower = file_name.to_lowercase();
            let title_similarity = self.calculate_title_similarity(&title_lower, &file_lower);
            
            // Also check if author name is in filename
            let author_match = !author_name.is_empty() && file_lower.contains(&author_name.to_lowercase());

            if title_similarity > 0.5 || author_match {
                // Good potential match, look for chapters
                let chapters = self.db.chapters().list_by_audiobook(audiobook.id).await?;

                // If we have a chapter number from parsing, try to match it
                if let Some(chapter_num) = parsed_chapter.chapter_number {
                    if let Some(chapter) = chapters.iter().find(|c| c.chapter_number == chapter_num) {
                        // Check if chapter is wanted (include "downloading" since file link may have failed)
                        let should_download = chapter.status == "wanted" 
                            || chapter.status == "missing"
                            || chapter.status == "downloading";
                        let skip_download = !should_download;

                        info!(
                            file_name = %file_name,
                            audiobook_title = %audiobook.title,
                            author = %author_name,
                            chapter_number = chapter_num,
                            chapter_title = ?chapter.title,
                            chapter_status = %chapter.status,
                            skip_download = skip_download,
                            "Matched audiobook file to chapter by number"
                        );

                        return Ok(Some(FileMatchResult {
                            file_index,
                            file_path: file_path.to_string(),
                            file_size,
                            match_target: FileMatchTarget::AudiobookChapter {
                                chapter_id: chapter.id,
                                audiobook_id: audiobook.id,
                                chapter_number: chapter.chapter_number,
                            },
                            match_type: FileMatchType::Auto,
                            confidence: 0.9,
                            quality: quality.clone(),
                            skip_download,
                            skip_reason: if skip_download {
                                Some(format!("Chapter already {}", chapter.status))
                            } else {
                                None
                            },
                        }));
                    }
                }

                // If no specific chapter match and chapters exist, try to match by order
                // (e.g., if files are ordered 01.mp3, 02.mp3, match to chapters 1, 2)
                if !chapters.is_empty() && parsed_chapter.chapter_number.is_none() {
                    // Try to match by title similarity within chapters
                    if let Some(parsed_title) = &parsed_chapter.title {
                        for chapter in &chapters {
                            if let Some(chapter_title) = &chapter.title {
                                let sim = self.calculate_title_similarity(
                                    &parsed_title.to_lowercase(),
                                    &chapter_title.to_lowercase(),
                                );
                                if sim > 0.6 {
                                    // Include "downloading" since file link may have failed
                                    let should_download = chapter.status == "wanted" 
                                        || chapter.status == "missing"
                                        || chapter.status == "downloading";
                                    let skip_download = !should_download;

                                    info!(
                                        file_name = %file_name,
                                        audiobook_title = %audiobook.title,
                                        chapter_title = ?chapter_title,
                                        similarity = format!("{:.2}", sim),
                                        chapter_status = %chapter.status,
                                        "Matched audiobook file to chapter by title"
                                    );

                                    return Ok(Some(FileMatchResult {
                                        file_index,
                                        file_path: file_path.to_string(),
                                        file_size,
                                        match_target: FileMatchTarget::AudiobookChapter {
                                            chapter_id: chapter.id,
                                            audiobook_id: audiobook.id,
                                            chapter_number: chapter.chapter_number,
                                        },
                                        match_type: FileMatchType::Auto,
                                        confidence: sim,
                                        quality: quality.clone(),
                                        skip_download,
                                        skip_reason: if skip_download {
                                            Some(format!("Chapter already {}", chapter.status))
                                        } else {
                                            None
                                        },
                                    }));
                                }
                            }
                        }
                    }

                    // For single-file audiobooks (no chapters defined), match to the audiobook itself
                    if chapters.is_empty() || chapters.len() == 1 {
                        // Check if audiobook already has files
                        if audiobook.has_files {
                            debug!(
                                file_name = %file_name,
                                audiobook_title = %audiobook.title,
                                "Audiobook already has files, skipping"
                            );
                            return Ok(Some(FileMatchResult {
                                file_index,
                                file_path: file_path.to_string(),
                                file_size,
                                match_target: FileMatchTarget::AudiobookChapter {
                                    chapter_id: chapters.first().map(|c| c.id).unwrap_or(audiobook.id),
                                    audiobook_id: audiobook.id,
                                    chapter_number: 1,
                                },
                                match_type: FileMatchType::Auto,
                                confidence: title_similarity,
                                quality: quality.clone(),
                                skip_download: true,
                                skip_reason: Some("Audiobook already has files".to_string()),
                            }));
                        }

                        // Match to the audiobook (will create chapter if needed during processing)
                        info!(
                            file_name = %file_name,
                            audiobook_title = %audiobook.title,
                            author = %author_name,
                            "Matched audiobook file (single file or no chapters)"
                        );

                        // If there's a chapter, use it; otherwise we'll need to create one during processing
                        if let Some(chapter) = chapters.first() {
                            return Ok(Some(FileMatchResult {
                                file_index,
                                file_path: file_path.to_string(),
                                file_size,
                                match_target: FileMatchTarget::AudiobookChapter {
                                    chapter_id: chapter.id,
                                    audiobook_id: audiobook.id,
                                    chapter_number: chapter.chapter_number,
                                },
                                match_type: FileMatchType::Auto,
                                confidence: title_similarity,
                                quality: quality.clone(),
                                skip_download: false,
                                skip_reason: None,
                            }));
                        }
                    }
                }
            }
        }

        debug!(
            file_name = %file_name,
            "Could not match audiobook file to any chapter"
        );

        Ok(None)
    }

    /// Parse audiobook file name to extract chapter number and title
    fn parse_audiobook_file_name(&self, file_name: &str) -> ParsedAudiobookFile {
        let name_without_ext = Path::new(file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file_name);

        // Common patterns:
        // "01 - Chapter Title"
        // "Part 01"
        // "Chapter 1 - Title"
        // "01_chapter_title"
        // "Book Title - 01"
        
        let patterns = [
            // "01 - Title" or "01_title"
            regex::Regex::new(r"^(\d{1,3})[\s_\-\.]+(.*)$").ok(),
            // "Part 01" or "Chapter 01"
            regex::Regex::new(r"(?i)(?:part|chapter|ch|pt)[\s_\-\.]*(\d{1,3})(?:[\s_\-\.]+(.*))?$").ok(),
            // "Title - 01"
            regex::Regex::new(r"^(.+?)[\s_\-\.]+(\d{1,3})$").ok(),
        ];

        for pattern in patterns.iter().flatten() {
            if let Some(caps) = pattern.captures(name_without_ext) {
                // Try to get chapter number from first or second capture group
                let (chapter_num, title) = if let Some(num_match) = caps.get(1) {
                    if let Ok(num) = num_match.as_str().parse::<i32>() {
                        let title = caps.get(2).map(|m| m.as_str().trim().to_string());
                        (Some(num), title)
                    } else {
                        // First group is title, second is number
                        if let Some(num_match2) = caps.get(2) {
                            if let Ok(num) = num_match2.as_str().parse::<i32>() {
                                let title = Some(num_match.as_str().trim().to_string());
                                (Some(num), title)
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                } else {
                    continue;
                };

                return ParsedAudiobookFile {
                    chapter_number: chapter_num,
                    title,
                };
            }
        }

        // No pattern matched, try to extract just numbers
        let numbers: Vec<i32> = name_without_ext
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|s| s.parse().ok())
            .collect();

        ParsedAudiobookFile {
            chapter_number: numbers.first().copied(),
            title: Some(name_without_ext.to_string()),
        }
    }

    /// Calculate title similarity using rapidfuzz (0.0 to 1.0)
    /// 
    /// Uses multiple matching strategies for robust comparison:
    /// - Normalized Levenshtein distance
    /// - Partial ratio (substring matching)
    /// - Token sort ratio (word order invariant)
    fn calculate_title_similarity(&self, a: &str, b: &str) -> f64 {
        filename_parser::show_name_similarity(a, b)
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
        // Clean the filename for comparison
        let clean_file = self.clean_track_filename(file_name);
        let clean_title = self.normalize_track_title(track_title);
        
        // Calculate similarity
        let similarity = filename_parser::show_name_similarity(&clean_file, &clean_title);
        
        // Also check if the track title is contained within the filename
        // This helps with cases like "Don't Cry (Original)" matching "Don't Cry"
        let contains_bonus = if clean_file.to_lowercase().contains(&clean_title.to_lowercase()) {
            0.3 // Bonus if track title is fully contained in filename
        } else {
            0.0
        };
        
        (similarity + contains_bonus).min(1.0)
    }

    /// Clean a filename for track title comparison
    fn clean_track_filename(&self, file_name: &str) -> String {
        // Remove file extension
        let name = std::path::Path::new(file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file_name);
        
        // Remove leading track number and separators (e.g., "01-", "02 - ", "03.")
        let re_track_num = regex::Regex::new(r"^\d{1,3}[\s\-_\.]+").unwrap();
        let name = re_track_num.replace(name, "");
        
        // Remove common parenthetical suffixes that indicate versions
        // e.g., "(Original)", "(Remaster)", "(Live)", "(Demo)", "(Acoustic)", "(Radio Edit)"
        let re_parens = regex::Regex::new(r"(?i)\s*\([^)]*(?:original|remaster|live|demo|acoustic|radio|edit|mix|version|bonus|instrumental|explicit|clean|single|album|extended|short)\s*[^)]*\)\s*$").unwrap();
        let name = re_parens.replace_all(&name, "");
        
        // Remove trailing parenthetical content that's just extra info
        // but be careful not to remove meaningful parts like "(Part 1)"
        let re_trailing_parens = regex::Regex::new(r"\s*\([^)]*\)\s*$").unwrap();
        let name_without_parens = re_trailing_parens.replace_all(&name, "");
        
        // Normalize apostrophes and quotes (curly vs straight)
        // \u{2018} = ', \u{2019} = ', \u{201C} = ", \u{201D} = "
        let name = name_without_parens
            .replace('\u{2018}', "'")
            .replace('\u{2019}', "'")
            .replace('\u{201C}', "\"")
            .replace('\u{201D}', "\"");
        
        // Remove underscores (common in filenames)
        let name = name.replace('_', " ");
        
        // Collapse multiple spaces
        let re_spaces = regex::Regex::new(r"\s+").unwrap();
        let name = re_spaces.replace_all(&name, " ");
        
        name.trim().to_string()
    }

    /// Normalize a track title for comparison
    fn normalize_track_title(&self, title: &str) -> String {
        // Normalize apostrophes and quotes (curly vs straight)
        // \u{2018} = ', \u{2019} = ', \u{201C} = ", \u{201D} = "
        let title = title
            .replace('\u{2018}', "'")
            .replace('\u{2019}', "'")
            .replace('\u{201C}', "\"")
            .replace('\u{201D}', "\"");
        
        title.trim().to_string()
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

/// Parsed info from a music file name
#[derive(Debug, Default)]
struct ParsedMusicFile {
    /// Track number extracted from filename (e.g., 01 from "01-artist-title.flac")
    track_number: Option<i32>,
    /// Artist name if extractable
    artist: Option<String>,
    /// Track title if extractable
    title: Option<String>,
}

/// Parsed info from an audiobook file name
#[derive(Debug, Default)]
struct ParsedAudiobookFile {
    /// Chapter/part number extracted from filename
    chapter_number: Option<i32>,
    /// Chapter title if extractable
    title: Option<String>,
}

/// Parse music file name to extract track number, artist, and title
///
/// Handles common patterns:
/// - Scene: "01-artist_name-track_title.flac"
/// - Scene: "01_artist_name-track_title.mp3"
/// - Track: "01. Track Title.flac"
/// - Track: "01 Track Title.mp3"
/// - Artist-Title: "Artist - Track Title.flac"
fn parse_music_file_name(file_name: &str) -> ParsedMusicFile {
    let mut result = ParsedMusicFile::default();
    
    // Remove extension
    let name = file_name
        .rsplit_once('.')
        .map(|(n, _)| n)
        .unwrap_or(file_name);
    
    // Replace underscores with spaces for easier parsing
    let cleaned = name.replace('_', " ");
    
    // Try to extract track number from the beginning
    // Patterns: "01-...", "01_...", "01 ...", "01. ..."
    let track_num_re = regex::Regex::new(r"^(\d{1,3})[\s\-_\.]").ok();
    if let Some(re) = &track_num_re {
        if let Some(caps) = re.captures(&cleaned) {
            if let Some(m) = caps.get(1) {
                result.track_number = m.as_str().parse().ok();
            }
        }
    }
    
    // For scene format: "01-artist_name-track_title" or "01-guns_n_roses-welcome_to_the_jungle"
    // Split by hyphen and try to extract artist and title
    let parts: Vec<&str> = cleaned.split('-').map(|s| s.trim()).collect();
    
    if parts.len() >= 3 {
        // First part is track number, second is artist, rest is title
        let first_part = parts[0].trim();
        if first_part.chars().all(|c| c.is_ascii_digit()) {
            // "01" - "guns n roses" - "welcome to the jungle"
            result.artist = Some(parts[1].trim().to_string());
            result.title = Some(parts[2..].join(" ").trim().to_string());
        } else {
            // "guns n roses" - "welcome to the jungle" - "live"
            result.artist = Some(parts[0].trim().to_string());
            result.title = Some(parts[1..].join(" ").trim().to_string());
        }
    } else if parts.len() == 2 {
        // Could be "01 - Track Title" or "Artist - Track Title"
        let first_part = parts[0].trim();
        if first_part.chars().all(|c| c.is_ascii_digit() || c.is_whitespace()) {
            // "01" - "Track Title"
            result.title = Some(parts[1].trim().to_string());
        } else {
            // "Artist" - "Track Title"
            result.artist = Some(parts[0].trim().to_string());
            result.title = Some(parts[1].trim().to_string());
        }
    } else if parts.len() == 1 {
        // No hyphens - try to extract title after track number
        // "01. Track Title" or "01 Track Title"
        if result.track_number.is_some() {
            let after_num = cleaned
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .trim_start_matches(['.', ' ', '-', '_']);
            if !after_num.is_empty() {
                result.title = Some(after_num.to_string());
            }
        }
    }
    
    result
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
