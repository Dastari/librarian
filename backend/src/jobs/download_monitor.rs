//! Download monitoring job
//!
//! This job monitors torrent completions and triggers post-download processing:
//! 1. Detects completed torrents (state = 'seeding', post_process_status IS NULL)
//! 2. Extracts archives if needed (RAR, ZIP, 7z)
//! 3. Identifies files belonging to media items (TV episodes, movies, music, audiobooks)
//! 4. Creates media file entries in database
//! 5. Runs file organization (if enabled for library)
//! 6. Updates item status to 'downloaded'
//!
//! Supports multiple media types:
//! - TV Shows: Linked via episode_id
//! - Movies: Linked via movie_id
//! - Music: Linked via album_id or track_id
//! - Audiobooks: Linked via audiobook_id
//!
//! Show-level and library-level overrides are respected for organization settings.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use crate::db::{Database, TorrentRecord};
use crate::services::extractor::ExtractorService;
use crate::services::file_utils::{get_container, is_audio_file, is_video_file};
use crate::services::{OrganizerService, TorrentService};

/// Process completed torrents and organize files
///
/// Called every minute by the job scheduler. Finds torrents that have completed
/// downloading (seeding) but haven't been processed yet, then:
/// 1. Extracts archives if present
/// 2. Creates media file records for video/audio files
/// 3. Organizes files into library structure (if enabled)
/// 4. Updates item status to 'downloaded'
/// 5. Marks the torrent as processed
pub async fn process_completed_torrents(
    pool: PgPool,
    torrent_service: Arc<TorrentService>,
) -> Result<()> {
    let db = Database::new(pool);

    // Get all completed torrents that need processing
    let completed_torrents = db.torrents().list_pending_processing().await?;

    if completed_torrents.is_empty() {
        debug!(job = "download_monitor", "No completed torrents to process");
        return Ok(());
    }

    info!(
        job = "download_monitor",
        torrent_count = completed_torrents.len(),
        "Processing completed torrents"
    );

    let organizer = OrganizerService::new(db.clone());
    
    // Create extractor service with temp directory
    let temp_dir = std::env::var("EXTRACT_TEMP_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("librarian_extract"));
    let extractor = ExtractorService::new(temp_dir);

    for torrent in completed_torrents {
        if let Err(e) = process_single_torrent(&db, &torrent_service, &organizer, &extractor, &torrent).await {
            error!(
                job = "download_monitor",
                info_hash = %torrent.info_hash,
                torrent_name = %torrent.name,
                error = %e,
                "Failed to process completed torrent: {}",
                torrent.name
            );
        }
    }

    Ok(())
}

async fn process_single_torrent(
    db: &Database,
    torrent_service: &Arc<TorrentService>,
    organizer: &OrganizerService,
    extractor: &ExtractorService,
    torrent: &TorrentRecord,
) -> Result<()> {
    info!(
        job = "download_monitor",
        info_hash = %torrent.info_hash,
        torrent_name = %torrent.name,
        episode_id = ?torrent.episode_id,
        movie_id = ?torrent.movie_id,
        album_id = ?torrent.album_id,
        audiobook_id = ?torrent.audiobook_id,
        "Processing completed torrent: {}",
        torrent.name
    );

    // Get files from the torrent
    let files = match torrent_service
        .get_files_for_torrent(&torrent.info_hash)
        .await
    {
        Ok(f) => f,
        Err(e) => {
            warn!(
                job = "download_monitor",
                info_hash = %torrent.info_hash,
                error = %e,
                "Could not get files for torrent"
            );
            db.torrents().mark_processed(&torrent.info_hash).await?;
            return Ok(());
        }
    };

    // Determine the torrent's save path
    let save_path = Path::new(&torrent.save_path);
    
    // Check if extraction is needed
    let (process_path, needs_cleanup) = if ExtractorService::needs_extraction(save_path) {
        info!(
            job = "download_monitor",
            info_hash = %torrent.info_hash,
            "Archive detected, extracting..."
        );
        match extractor.extract_archives(save_path).await {
            Ok(extracted_path) => {
                if extracted_path != save_path {
                    (extracted_path, true)
                } else {
                    (save_path.to_path_buf(), false)
                }
            }
            Err(e) => {
                error!(
                    job = "download_monitor",
                    info_hash = %torrent.info_hash,
                    error = %e,
                    "Failed to extract archives"
                );
                (save_path.to_path_buf(), false)
            }
        }
    } else {
        (save_path.to_path_buf(), false)
    };

    // Route processing based on linked item type
    let result = if torrent.episode_id.is_some() {
        process_tv_episode(db, organizer, torrent, &files).await
    } else if torrent.movie_id.is_some() {
        process_movie(db, organizer, torrent, &files).await
    } else if torrent.album_id.is_some() || torrent.track_id.is_some() {
        process_music(db, torrent, &files).await
    } else if torrent.audiobook_id.is_some() {
        process_audiobook(db, torrent, &files).await
    } else if torrent.library_id.is_some() {
        // No specific item linked - process as unlinked
        process_unlinked(db, organizer, torrent, &files).await
    } else {
        // No library or item linked - try to auto-match against all libraries
        info!(
            job = "download_monitor",
            info_hash = %torrent.info_hash,
            "No library linked, attempting to auto-match against all libraries"
        );
        process_no_library(db, organizer, torrent, &files).await
    };

    // Cleanup extracted files
    if needs_cleanup {
        if let Err(e) = extractor.cleanup(&process_path).await {
            warn!(
                job = "download_monitor",
                path = %process_path.display(),
                error = %e,
                "Failed to cleanup extracted files"
            );
        }
    }

    // Mark torrent as processed regardless of result
    db.torrents().mark_processed(&torrent.info_hash).await?;

    result?;

    info!(
        job = "download_monitor",
        info_hash = %torrent.info_hash,
        torrent_name = %torrent.name,
        "Torrent processing complete"
    );

    Ok(())
}

/// Process torrent linked to a TV episode
async fn process_tv_episode(
    db: &Database,
    organizer: &OrganizerService,
    torrent: &TorrentRecord,
    files: &[crate::services::torrent::TorrentFile],
) -> Result<()> {
    let episode_id = torrent.episode_id.unwrap();
    let episode = db.episodes().get_by_id(episode_id).await?;

    let (show, library) = if let Some(ref ep) = episode {
        let show = db.tv_shows().get_by_id(ep.tv_show_id).await?;
        let lib = if let Some(ref s) = show {
            db.libraries().get_by_id(s.library_id).await?
        } else {
            None
        };
        (show, lib)
    } else {
        warn!(
            job = "download_monitor",
            episode_id = %episode_id,
            "Episode not found"
        );
        return Ok(());
    };

    // Process each video file
    for file_info in files {
        if !is_video_file(&file_info.path) {
            continue;
        }

        let file_path = &file_info.path;
        if db.media_files().exists_by_path(file_path).await? {
            debug!(path = %file_path, "Media file already exists");
            continue;
        }

        if let Some(ref lib) = library {
            let media_file = db
                .media_files()
                .create(crate::db::CreateMediaFile {
                    library_id: lib.id,
                    path: file_path.clone(),
                    size_bytes: file_info.size as i64,
                    container: get_container(file_path),
                    video_codec: None,
                    audio_codec: None,
                    width: None,
                    height: None,
                    duration: None,
                    bitrate: None,
                    file_hash: None,
                    episode_id: episode.as_ref().map(|e| e.id),
                    relative_path: None,
                    original_name: Path::new(file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string()),
                    resolution: None,
                    is_hdr: None,
                    hdr_type: None,
                })
                .await?;

            info!(
                job = "download_monitor",
                file_id = %media_file.id,
                path = %file_path,
                "Created media file record"
            );

            // Organize if enabled
            if let (Some(ep), Some(s)) = (&episode, &show) {
                let (organize_enabled, rename_style, action) =
                    organizer.get_full_organize_settings(s).await?;

                if organize_enabled {
                    match organizer
                        .organize_file(
                            &media_file,
                            s,
                            ep,
                            &lib.path,
                            rename_style,
                            lib.naming_pattern.as_deref(),
                            &action,
                            false,
                        )
                        .await
                    {
                        Ok(result) if result.success => {
                            info!(
                                job = "download_monitor",
                                new_path = %result.new_path,
                                "File organized successfully"
                            );
                        }
                        Ok(result) => {
                            warn!(
                                job = "download_monitor",
                                error = ?result.error,
                                "Failed to organize file"
                            );
                        }
                        Err(e) => {
                            error!(job = "download_monitor", error = %e, "Error organizing file");
                        }
                    }
                }
            }
        }
    }

    // Update episode status
    if let Some(ref ep) = episode {
        db.episodes().update_status(ep.id, "downloaded").await?;
        if let Some(ref s) = show {
            db.tv_shows().update_stats(s.id).await?;
            info!(
                job = "download_monitor",
                show = %s.name,
                season = ep.season,
                episode = ep.episode,
                "Episode marked as downloaded"
            );
        }
    }

    Ok(())
}

/// Process torrent linked to a movie
async fn process_movie(
    db: &Database,
    _organizer: &OrganizerService,
    torrent: &TorrentRecord,
    files: &[crate::services::torrent::TorrentFile],
) -> Result<()> {
    let movie_id = torrent.movie_id.unwrap();
    let movie = db.movies().get_by_id(movie_id).await?;

    let library = if let Some(ref m) = movie {
        db.libraries().get_by_id(m.library_id).await?
    } else {
        warn!(job = "download_monitor", movie_id = %movie_id, "Movie not found");
        return Ok(());
    };

    // Process each video file
    for file_info in files {
        if !is_video_file(&file_info.path) {
            continue;
        }

        let file_path = &file_info.path;
        if db.media_files().exists_by_path(file_path).await? {
            continue;
        }

        if let Some(ref lib) = library {
            let media_file = db
                .media_files()
                .create(crate::db::CreateMediaFile {
                    library_id: lib.id,
                    path: file_path.clone(),
                    size_bytes: file_info.size as i64,
                    container: get_container(file_path),
                    video_codec: None,
                    audio_codec: None,
                    width: None,
                    height: None,
                    duration: None,
                    bitrate: None,
                    file_hash: None,
                    episode_id: None,
                    relative_path: None,
                    original_name: Path::new(file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string()),
                    resolution: None,
                    is_hdr: None,
                    hdr_type: None,
                })
                .await?;

            // Link to movie
            db.media_files()
                .link_to_movie(media_file.id, movie_id)
                .await?;

            info!(
                job = "download_monitor",
                file_id = %media_file.id,
                movie_id = %movie_id,
                "Created media file for movie"
            );

            // TODO: Organize movie files into Movie (Year) folder
        }
    }

    // Update movie status
    if let Some(ref m) = movie {
        db.movies().update_has_file(m.id, true).await?;
        info!(
            job = "download_monitor",
            movie = %m.title,
            "Movie marked as downloaded"
        );
    }

    Ok(())
}

/// Process torrent linked to music (album or track)
async fn process_music(
    db: &Database,
    torrent: &TorrentRecord,
    files: &[crate::services::torrent::TorrentFile],
) -> Result<()> {
    let library_id = torrent.library_id;

    // Process each audio file
    for file_info in files {
        if !is_audio_file(&file_info.path) {
            continue;
        }

        let file_path = &file_info.path;
        if db.media_files().exists_by_path(file_path).await? {
            continue;
        }

        if let Some(lib_id) = library_id {
            let media_file = db
                .media_files()
                .create(crate::db::CreateMediaFile {
                    library_id: lib_id,
                    path: file_path.clone(),
                    size_bytes: file_info.size as i64,
                    container: get_container(file_path),
                    video_codec: None,
                    audio_codec: None,
                    width: None,
                    height: None,
                    duration: None,
                    bitrate: None,
                    file_hash: None,
                    episode_id: None,
                    relative_path: None,
                    original_name: Path::new(file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string()),
                    resolution: None,
                    is_hdr: None,
                    hdr_type: None,
                })
                .await?;

            // Link to album or track
            if let Some(album_id) = torrent.album_id {
                db.media_files()
                    .link_to_album(media_file.id, album_id)
                    .await?;
            }
            if let Some(track_id) = torrent.track_id {
                db.media_files()
                    .link_to_track(media_file.id, track_id)
                    .await?;
            }

            info!(
                job = "download_monitor",
                file_id = %media_file.id,
                "Created media file for music"
            );
        }
    }

    // TODO: Update album/track status

    Ok(())
}

/// Process torrent linked to an audiobook
async fn process_audiobook(
    db: &Database,
    torrent: &TorrentRecord,
    files: &[crate::services::torrent::TorrentFile],
) -> Result<()> {
    let audiobook_id = torrent.audiobook_id.unwrap();
    let library_id = torrent.library_id;

    // Process each audio file
    for file_info in files {
        if !is_audio_file(&file_info.path) {
            continue;
        }

        let file_path = &file_info.path;
        if db.media_files().exists_by_path(file_path).await? {
            continue;
        }

        if let Some(lib_id) = library_id {
            let media_file = db
                .media_files()
                .create(crate::db::CreateMediaFile {
                    library_id: lib_id,
                    path: file_path.clone(),
                    size_bytes: file_info.size as i64,
                    container: get_container(file_path),
                    video_codec: None,
                    audio_codec: None,
                    width: None,
                    height: None,
                    duration: None,
                    bitrate: None,
                    file_hash: None,
                    episode_id: None,
                    relative_path: None,
                    original_name: Path::new(file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string()),
                    resolution: None,
                    is_hdr: None,
                    hdr_type: None,
                })
                .await?;

            // Link to audiobook
            db.media_files()
                .link_to_audiobook(media_file.id, audiobook_id)
                .await?;

            info!(
                job = "download_monitor",
                file_id = %media_file.id,
                audiobook_id = %audiobook_id,
                "Created media file for audiobook"
            );
        }
    }

    // TODO: Update audiobook status

    Ok(())
}

/// Process torrent with library but no specific item linked
/// Tries to auto-match based on filename and organize if successful
async fn process_unlinked(
    db: &Database,
    organizer: &OrganizerService,
    torrent: &TorrentRecord,
    files: &[crate::services::torrent::TorrentFile],
) -> Result<()> {
    let library_id = torrent.library_id.unwrap();
    let library = db.libraries().get_by_id(library_id).await?;

    let Some(lib) = library else {
        warn!(job = "download_monitor", library_id = %library_id, "Library not found");
        return Ok(());
    };

    // Determine which files to process based on library type
    let is_audio_library = matches!(lib.library_type.as_str(), "music" | "audiobooks");

    for file_info in files {
        let should_process = if is_audio_library {
            is_audio_file(&file_info.path)
        } else {
            is_video_file(&file_info.path)
        };

        if !should_process {
            continue;
        }

        let file_path = &file_info.path;
        if db.media_files().exists_by_path(file_path).await? {
            debug!(path = %file_path, "Media file already exists");
            continue;
        }

        // Try to auto-match based on filename for TV shows
        let mut matched_episode = None;
        let mut matched_show = None;
        
        if lib.library_type == "tv" {
            // Parse the filename or torrent name
            let parse_name = Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&torrent.name);
            
            let parsed = crate::services::filename_parser::parse_episode(parse_name);
            
            if let Some(ref show_name) = parsed.show_name {
                if let (Some(season), Some(episode)) = (parsed.season, parsed.episode) {
                    // Find matching show in library
                    if let Ok(Some(tv_show)) = db.tv_shows()
                        .find_by_name_in_library(lib.id, show_name)
                        .await
                    {
                        // Find matching episode
                        if let Ok(Some(ep)) = db.episodes()
                            .get_by_show_season_episode(tv_show.id, season as i32, episode as i32)
                            .await
                        {
                            info!(
                                job = "download_monitor",
                                show_name = %tv_show.name,
                                season = season,
                                episode = episode,
                                "Auto-matched file to episode"
                            );
                            matched_episode = Some(ep);
                            matched_show = Some(tv_show);
                        }
                    }
                }
            }
        }

        // Create media file record
        let media_file = db
            .media_files()
            .create(crate::db::CreateMediaFile {
                library_id: lib.id,
                path: file_path.clone(),
                size_bytes: file_info.size as i64,
                container: get_container(file_path),
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: matched_episode.as_ref().map(|e| e.id),
                relative_path: None,
                original_name: Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: None,
                is_hdr: None,
                hdr_type: None,
            })
            .await?;

        info!(
            job = "download_monitor",
            file_id = %media_file.id,
            library = %lib.name,
            matched = matched_episode.is_some(),
            "Created media file record"
        );

        // If matched to an episode, organize and update status
        if let (Some(ep), Some(show)) = (&matched_episode, &matched_show) {
            // Get organize settings (respecting show-level overrides)
            let (organize_enabled, rename_style, action) =
                organizer.get_full_organize_settings(show).await?;

            if organize_enabled {
                match organizer
                    .organize_file(
                        &media_file,
                        show,
                        ep,
                        &lib.path,
                        rename_style,
                        lib.naming_pattern.as_deref(),
                        &action,
                        false,
                    )
                    .await
                {
                    Ok(result) if result.success => {
                        info!(
                            job = "download_monitor",
                            new_path = %result.new_path,
                            "File organized successfully"
                        );
                    }
                    Ok(result) => {
                        warn!(
                            job = "download_monitor",
                            error = ?result.error,
                            "Failed to organize file"
                        );
                    }
                    Err(e) => {
                        error!(job = "download_monitor", error = %e, "Error organizing file");
                    }
                }
            }

            // Update episode status
            db.episodes().update_status(ep.id, "downloaded").await?;
            db.tv_shows().update_stats(show.id).await?;
            
            // Link torrent to episode for future reference
            let _ = db.torrents().link_to_episode(&torrent.info_hash, ep.id).await;
            
            info!(
                job = "download_monitor",
                show = %show.name,
                season = ep.season,
                episode = ep.episode,
                "Episode marked as downloaded"
            );
        }
    }

    Ok(())
}

/// Process torrent with no library linked - try to match against all libraries
/// This handles the case where a torrent was downloaded from Hunt without specifying a library
async fn process_no_library(
    db: &Database,
    organizer: &OrganizerService,
    torrent: &TorrentRecord,
    files: &[crate::services::torrent::TorrentFile],
) -> Result<()> {
    // Get user's libraries
    let user_id = torrent.user_id;
    let libraries = db.libraries().list_by_user(user_id).await?;
    
    if libraries.is_empty() {
        debug!(job = "download_monitor", "No libraries found for user");
        return Ok(());
    }

    // Parse the torrent name to determine media type
    let parsed = crate::services::filename_parser::parse_episode(&torrent.name);
    
    for file_info in files {
        if !is_video_file(&file_info.path) {
            continue;
        }

        let file_path = &file_info.path;
        if db.media_files().exists_by_path(file_path).await? {
            debug!(path = %file_path, "Media file already exists");
            continue;
        }

        // Try to match against TV libraries first
        let parse_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&torrent.name);
        
        let parsed = crate::services::filename_parser::parse_episode(parse_name);
        
        let mut matched = false;
        
        if let Some(ref show_name) = parsed.show_name {
            if let (Some(season), Some(episode)) = (parsed.season, parsed.episode) {
                // Try each TV library
                for lib in libraries.iter().filter(|l| l.library_type == "tv") {
                    if let Ok(Some(tv_show)) = db.tv_shows()
                        .find_by_name_in_library(lib.id, show_name)
                        .await
                    {
                        if let Ok(Some(ep)) = db.episodes()
                            .get_by_show_season_episode(tv_show.id, season as i32, episode as i32)
                            .await
                        {
                            info!(
                                job = "download_monitor",
                                show_name = %tv_show.name,
                                season = season,
                                episode = episode,
                                library = %lib.name,
                                "Auto-matched file to episode (no library was specified)"
                            );

                            // Create media file record
                            let media_file = db
                                .media_files()
                                .create(crate::db::CreateMediaFile {
                                    library_id: lib.id,
                                    path: file_path.clone(),
                                    size_bytes: file_info.size as i64,
                                    container: get_container(file_path),
                                    video_codec: None,
                                    audio_codec: None,
                                    width: None,
                                    height: None,
                                    duration: None,
                                    bitrate: None,
                                    file_hash: None,
                                    episode_id: Some(ep.id),
                                    relative_path: None,
                                    original_name: Path::new(file_path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .map(|s| s.to_string()),
                                    resolution: None,
                                    is_hdr: None,
                                    hdr_type: None,
                                })
                                .await?;

                            info!(
                                job = "download_monitor",
                                file_id = %media_file.id,
                                library = %lib.name,
                                "Created media file record"
                            );

                            // Link torrent to library and episode
                            let _ = db.torrents().link_to_library(&torrent.info_hash, lib.id).await;
                            let _ = db.torrents().link_to_episode(&torrent.info_hash, ep.id).await;

                            // Get organize settings (respecting show-level overrides)
                            let (organize_enabled, rename_style, action) =
                                organizer.get_full_organize_settings(&tv_show).await?;

                            if organize_enabled {
                                match organizer
                                    .organize_file(
                                        &media_file,
                                        &tv_show,
                                        &ep,
                                        &lib.path,
                                        rename_style,
                                        lib.naming_pattern.as_deref(),
                                        &action,
                                        false,
                                    )
                                    .await
                                {
                                    Ok(result) if result.success => {
                                        info!(
                                            job = "download_monitor",
                                            new_path = %result.new_path,
                                            "File organized successfully"
                                        );
                                    }
                                    Ok(result) => {
                                        warn!(
                                            job = "download_monitor",
                                            error = ?result.error,
                                            "Failed to organize file"
                                        );
                                    }
                                    Err(e) => {
                                        error!(job = "download_monitor", error = %e, "Error organizing file");
                                    }
                                }
                            }

                            // Update episode status
                            db.episodes().update_status(ep.id, "downloaded").await?;
                            db.tv_shows().update_stats(tv_show.id).await?;
                            
                            info!(
                                job = "download_monitor",
                                show = %tv_show.name,
                                season = ep.season,
                                episode = ep.episode,
                                "Episode marked as downloaded"
                            );
                            
                            matched = true;
                            break;
                        }
                    }
                }
            }
        }

        if !matched {
            debug!(
                job = "download_monitor",
                path = %file_path,
                "Could not auto-match file to any library"
            );
        }
    }

    Ok(())
}

