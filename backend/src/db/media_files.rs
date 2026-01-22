//! Media files database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Media file record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MediaFileRecord {
    pub id: Uuid,
    pub library_id: Uuid,
    pub path: String,
    pub size_bytes: i64,
    pub container: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration: Option<i32>,
    pub bitrate: Option<i32>,
    pub file_hash: Option<String>,
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    /// Link to track record if this file is a music track
    pub track_id: Option<Uuid>,
    /// Link to album for grouping music files
    pub album_id: Option<Uuid>,
    /// Link to audiobook record if this file is an audiobook
    pub audiobook_id: Option<Uuid>,
    /// Link to audiobook chapter if this file is a chapter
    pub chapter_id: Option<Uuid>,
    pub relative_path: Option<String>,
    pub original_name: Option<String>,
    pub video_bitrate: Option<i32>,
    pub audio_channels: Option<String>,
    pub audio_language: Option<String>,
    pub resolution: Option<String>,
    pub is_hdr: Option<bool>,
    pub hdr_type: Option<String>,
    pub organized: bool,
    pub organized_at: Option<chrono::DateTime<chrono::Utc>>,
    pub original_path: Option<String>,
    pub organize_status: Option<String>,
    pub organize_error: Option<String>,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Content type: episode, movie, track, or audiobook
    pub content_type: Option<String>,
    /// Quality relative to library/item target: unknown, optimal, suboptimal, exceeds
    pub quality_status: Option<String>,
    
    // Embedded metadata from ID3/Vorbis/container tags
    /// Artist name from embedded tags
    pub meta_artist: Option<String>,
    /// Album name from embedded tags
    pub meta_album: Option<String>,
    /// Track/episode title from embedded tags
    pub meta_title: Option<String>,
    /// Track number from embedded tags
    pub meta_track_number: Option<i32>,
    /// Disc number from embedded tags
    pub meta_disc_number: Option<i32>,
    /// Year from embedded tags
    pub meta_year: Option<i32>,
    /// Genre from embedded tags
    pub meta_genre: Option<String>,
    /// Show name from video container metadata
    pub meta_show_name: Option<String>,
    /// Season number from video container metadata
    pub meta_season: Option<i32>,
    /// Episode number from video container metadata
    pub meta_episode: Option<i32>,
    
    // Processing timestamps (null = not yet done)
    /// When FFprobe analysis was completed
    pub ffprobe_analyzed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When ID3/Vorbis metadata was extracted
    pub metadata_extracted_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When file was last matched to a library item
    pub matched_at: Option<chrono::DateTime<chrono::Utc>>,
    
    // Album art and lyrics
    /// Cover art image as base64-encoded string
    pub cover_art_base64: Option<String>,
    /// MIME type of the cover art (e.g., "image/jpeg", "image/png")
    pub cover_art_mime: Option<String>,
    /// Lyrics extracted from embedded tags
    pub lyrics: Option<String>,
}

/// Input for creating a media file
#[derive(Debug)]
#[derive(Default)]
pub struct CreateMediaFile {
    pub library_id: Uuid,
    pub path: String,
    pub size_bytes: i64,
    pub container: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration: Option<i32>,
    pub bitrate: Option<i32>,
    pub file_hash: Option<String>,
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    /// Link to track record if this file is a music track
    pub track_id: Option<Uuid>,
    /// Link to album for grouping music files
    pub album_id: Option<Uuid>,
    /// Link to audiobook record if this file is an audiobook
    pub audiobook_id: Option<Uuid>,
    /// Link to audiobook chapter if this file is a chapter
    pub chapter_id: Option<Uuid>,
    pub relative_path: Option<String>,
    pub original_name: Option<String>,
    pub resolution: Option<String>,
    pub is_hdr: Option<bool>,
    pub hdr_type: Option<String>,
}

pub struct MediaFileRepository {
    pool: PgPool,
}

impl MediaFileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all media files for a library
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE library_id = $1
            ORDER BY path
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Check if a file path already exists
    pub async fn exists_by_path(&self, path: &str) -> Result<bool> {
        let result =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM media_files WHERE path = $1")
                .bind(path)
                .fetch_one(&self.pool)
                .await?;

        Ok(result > 0)
    }

    /// Get a media file by path
    pub async fn get_by_path(&self, path: &str) -> Result<Option<MediaFileRecord>> {
        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE path = $1
            "#,
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// List all media files linked to an episode
    pub async fn list_by_episode(&self, episode_id: Uuid) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE episode_id = $1
            ORDER BY organized DESC, added_at ASC
            "#,
        )
        .bind(episode_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Create a new media file
    pub async fn create(&self, input: CreateMediaFile) -> Result<MediaFileRecord> {
        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            INSERT INTO media_files (
                library_id, path, size, container, video_codec, audio_codec,
                width, height, duration, bitrate, file_hash, episode_id, movie_id,
                relative_path, original_name, resolution, is_hdr, hdr_type
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING id, library_id, path, size as size_bytes, 
                      container, video_codec, audio_codec, width, height,
                      duration, bitrate, file_hash, episode_id, movie_id,
                      track_id, album_id, audiobook_id, chapter_id, relative_path,
                      original_name, video_bitrate, audio_channels, audio_language,
                      resolution, is_hdr, hdr_type, organized, organized_at,
                      original_path, organize_status, organize_error, added_at, modified_at,
                      content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            "#,
        )
        .bind(input.library_id)
        .bind(&input.path)
        .bind(input.size_bytes)
        .bind(&input.container)
        .bind(&input.video_codec)
        .bind(&input.audio_codec)
        .bind(input.width)
        .bind(input.height)
        .bind(input.duration)
        .bind(input.bitrate)
        .bind(&input.file_hash)
        .bind(input.episode_id)
        .bind(input.movie_id)
        .bind(&input.relative_path)
        .bind(&input.original_name)
        .bind(&input.resolution)
        .bind(input.is_hdr)
        .bind(&input.hdr_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Upsert a media file (insert or update if path exists)
    ///
    /// On conflict, updates file metadata AND links to the provided movie/episode.
    /// This ensures that when re-processing a download, the existing record gets
    /// properly linked to the content item.
    pub async fn upsert(&self, input: CreateMediaFile) -> Result<MediaFileRecord> {
        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            INSERT INTO media_files (
                library_id, path, size, container, video_codec, audio_codec,
                width, height, duration, bitrate, file_hash, episode_id, movie_id,
                track_id, album_id, audiobook_id, chapter_id,
                relative_path, original_name, resolution, is_hdr, hdr_type
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
            ON CONFLICT (path) DO UPDATE SET
                size = EXCLUDED.size,
                container = EXCLUDED.container,
                video_codec = EXCLUDED.video_codec,
                audio_codec = EXCLUDED.audio_codec,
                width = EXCLUDED.width,
                height = EXCLUDED.height,
                duration = EXCLUDED.duration,
                bitrate = EXCLUDED.bitrate,
                file_hash = EXCLUDED.file_hash,
                resolution = EXCLUDED.resolution,
                is_hdr = EXCLUDED.is_hdr,
                hdr_type = EXCLUDED.hdr_type,
                -- Only update movie_id/episode_id if the new value is not null
                movie_id = COALESCE(EXCLUDED.movie_id, media_files.movie_id),
                episode_id = COALESCE(EXCLUDED.episode_id, media_files.episode_id),
                track_id = COALESCE(EXCLUDED.track_id, media_files.track_id),
                album_id = COALESCE(EXCLUDED.album_id, media_files.album_id),
                audiobook_id = COALESCE(EXCLUDED.audiobook_id, media_files.audiobook_id),
                chapter_id = COALESCE(EXCLUDED.chapter_id, media_files.chapter_id),
                -- Update content_type based on what's being linked
                content_type = CASE 
                    WHEN EXCLUDED.movie_id IS NOT NULL THEN 'movie'
                    WHEN EXCLUDED.episode_id IS NOT NULL THEN 'episode'
                    WHEN EXCLUDED.track_id IS NOT NULL THEN 'track'
                    WHEN EXCLUDED.chapter_id IS NOT NULL THEN 'chapter'
                    WHEN EXCLUDED.audiobook_id IS NOT NULL THEN 'audiobook'
                    ELSE media_files.content_type
                END,
                modified_at = NOW()
            RETURNING id, library_id, path, size as size_bytes, 
                      container, video_codec, audio_codec, width, height,
                      duration, bitrate, file_hash, episode_id, movie_id,
                      track_id, album_id, audiobook_id, chapter_id, relative_path,
                      original_name, video_bitrate, audio_channels, audio_language,
                      resolution, is_hdr, hdr_type, organized, organized_at,
                      original_path, organize_status, organize_error, added_at, modified_at,
                      content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            "#,
        )
        .bind(input.library_id)
        .bind(&input.path)
        .bind(input.size_bytes)
        .bind(&input.container)
        .bind(&input.video_codec)
        .bind(&input.audio_codec)
        .bind(input.width)
        .bind(input.height)
        .bind(input.duration)
        .bind(input.bitrate)
        .bind(&input.file_hash)
        .bind(input.episode_id)
        .bind(input.movie_id)
        .bind(input.track_id)
        .bind(input.album_id)
        .bind(input.audiobook_id)
        .bind(input.chapter_id)
        .bind(&input.relative_path)
        .bind(&input.original_name)
        .bind(&input.resolution)
        .bind(input.is_hdr)
        .bind(&input.hdr_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete media files that no longer exist on disk
    pub async fn delete_missing(&self, library_id: Uuid, existing_paths: &[String]) -> Result<u64> {
        if existing_paths.is_empty() {
            // Delete all files for this library if none exist
            let result = sqlx::query("DELETE FROM media_files WHERE library_id = $1")
                .bind(library_id)
                .execute(&self.pool)
                .await?;
            return Ok(result.rows_affected());
        }

        let result =
            sqlx::query("DELETE FROM media_files WHERE library_id = $1 AND path != ALL($2)")
                .bind(library_id)
                .bind(existing_paths)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected())
    }

    /// Get count of files in a library
    pub async fn count_by_library(&self, library_id: Uuid) -> Result<i64> {
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM media_files WHERE library_id = $1")
                .bind(library_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count)
    }

    /// Link a media file to an episode
    pub async fn link_to_episode(&self, file_id: Uuid, episode_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE media_files SET episode_id = $2, content_type = 'episode' WHERE id = $1",
        )
        .bind(file_id)
        .bind(episode_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Link a media file to a movie
    pub async fn link_to_movie(&self, file_id: Uuid, movie_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE media_files SET movie_id = $2, content_type = 'movie' WHERE id = $1")
            .bind(file_id)
            .bind(movie_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Link a media file to a music album
    pub async fn link_to_album(&self, file_id: Uuid, album_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE media_files SET album_id = $2 WHERE id = $1")
            .bind(file_id)
            .bind(album_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Link a media file to a music track
    pub async fn link_to_track(&self, file_id: Uuid, track_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE media_files SET track_id = $2, content_type = 'track' WHERE id = $1")
            .bind(file_id)
            .bind(track_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Link a media file to an audiobook
    pub async fn link_to_audiobook(&self, file_id: Uuid, audiobook_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE media_files SET audiobook_id = $2, content_type = 'audiobook' WHERE id = $1",
        )
        .bind(file_id)
        .bind(audiobook_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a media file by episode ID (returns the first/primary file for an episode)
    pub async fn get_by_episode_id(&self, episode_id: Uuid) -> Result<Option<MediaFileRecord>> {
        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE episode_id = $1
            ORDER BY size_bytes DESC
            LIMIT 1
            "#,
        )
        .bind(episode_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get unorganized files for a library
    pub async fn list_unorganized_by_library(
        &self,
        library_id: Uuid,
    ) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE library_id = $1 AND organized = false
            ORDER BY path
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Mark a file as organized (moved to library structure)
    ///
    /// If another record already exists at the new_path, this will:
    /// 1. Transfer the movie_id/episode_id to the existing record if needed
    /// 2. Delete the source record (file_id)
    /// 3. Mark the existing record as organized
    ///
    /// This handles the case where a library scan created a record at the destination
    /// before the organizer could update the download record's path.
    pub async fn mark_organized(
        &self,
        file_id: Uuid,
        new_path: &str,
        original_path: &str,
    ) -> Result<()> {
        // Check if another record already exists at the destination path
        if let Some(existing) = self.get_by_path(new_path).await? {
            if existing.id != file_id {
                // Another record exists at the destination - we need to merge
                // Get the source record to transfer its links
                if let Some(source) = self.get_by_id(file_id).await? {
                    // Transfer movie/episode links to the existing record if not already set
                    if source.movie_id.is_some() && existing.movie_id.is_none() {
                        self.link_to_movie(existing.id, source.movie_id.unwrap())
                            .await?;
                    }
                    if source.episode_id.is_some() && existing.episode_id.is_none() {
                        self.link_to_episode(existing.id, source.episode_id.unwrap())
                            .await?;
                    }
                    if source.album_id.is_some() && existing.album_id.is_none() {
                        self.link_to_album(existing.id, source.album_id.unwrap())
                            .await?;
                    }
                    if source.audiobook_id.is_some() && existing.audiobook_id.is_none() {
                        self.link_to_audiobook(existing.id, source.audiobook_id.unwrap())
                            .await?;
                    }
                }

                // Delete the source record (the one from downloads)
                self.delete(file_id).await?;

                // Mark the existing destination record as organized
                sqlx::query(
                    r#"
                    UPDATE media_files SET 
                        original_path = COALESCE(original_path, $2),
                        organized = true, 
                        organized_at = NOW(),
                        organize_status = 'organized',
                        organize_error = NULL,
                        modified_at = NOW()
                    WHERE id = $1
                    "#,
                )
                .bind(existing.id)
                .bind(original_path)
                .execute(&self.pool)
                .await?;

                return Ok(());
            }
        }

        // No conflict - just update the path
        sqlx::query(
            r#"
            UPDATE media_files SET 
                path = $2, 
                original_path = $3,
                organized = true, 
                organized_at = NOW(),
                organize_status = 'organized',
                organize_error = NULL,
                modified_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(file_id)
        .bind(new_path)
        .bind(original_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a file as unorganized (needs to be re-organized)
    pub async fn mark_unorganized(&self, file_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET 
                organized = false, 
                organized_at = NULL,
                organize_status = 'pending',
                organize_error = NULL,
                modified_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(file_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a file as conflicted (cannot be organized due to conflict)
    pub async fn mark_conflicted(&self, file_id: Uuid, error_message: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET 
                organize_status = 'conflicted',
                organize_error = $2,
                modified_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(file_id)
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a file organization as failed with an error
    pub async fn mark_organize_error(&self, file_id: Uuid, error_message: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET 
                organize_status = 'error',
                organize_error = $2,
                modified_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(file_id)
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List conflicted files for a library
    pub async fn list_conflicted_by_library(
        &self,
        library_id: Uuid,
    ) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE library_id = $1 AND organize_status = 'conflicted'
            ORDER BY path
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Update only the path of a media file
    pub async fn update_path(&self, file_id: Uuid, new_path: &str) -> Result<()> {
        sqlx::query("UPDATE media_files SET path = $2, modified_at = NOW() WHERE id = $1")
            .bind(file_id)
            .bind(new_path)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get a media file by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MediaFileRecord>> {
        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get unmatched files for a library (files not linked to any episode or movie)
    ///
    /// Only returns files that are actually within the library's folder path,
    /// not files in other locations (like downloads) that happen to be linked to the library.
    pub async fn list_unmatched_by_library(
        &self,
        library_id: Uuid,
    ) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT mf.id, mf.library_id, mf.path, mf.size as size_bytes, 
                   mf.container, mf.video_codec, mf.audio_codec, mf.width, mf.height,
                   mf.duration, mf.bitrate, mf.file_hash, mf.episode_id, mf.movie_id,
                   mf.track_id, mf.album_id, mf.audiobook_id, mf.relative_path,
                   mf.original_name, mf.video_bitrate, mf.audio_channels, mf.audio_language,
                   mf.resolution, mf.is_hdr, mf.hdr_type, mf.organized, mf.organized_at,
                   mf.original_path, mf.organize_status, mf.organize_error, mf.added_at, mf.modified_at,
                   mf.content_type, mf.quality_status,
                   mf.meta_artist, mf.meta_album, mf.meta_title, mf.meta_track_number, mf.meta_disc_number,
                   mf.meta_year, mf.meta_genre, mf.meta_show_name, mf.meta_season, mf.meta_episode,
                   mf.ffprobe_analyzed_at, mf.metadata_extracted_at, mf.matched_at
            FROM media_files mf
            JOIN libraries l ON mf.library_id = l.id
            WHERE mf.library_id = $1 
              AND mf.episode_id IS NULL 
              AND mf.movie_id IS NULL
              AND mf.path LIKE l.path || '%'
            ORDER BY mf.path
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get count of unmatched files in a library
    ///
    /// Only counts files that are actually within the library's folder path.
    pub async fn count_unmatched_by_library(&self, library_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) 
            FROM media_files mf
            JOIN libraries l ON mf.library_id = l.id
            WHERE mf.library_id = $1 
              AND mf.episode_id IS NULL 
              AND mf.movie_id IS NULL
              AND mf.path LIKE l.path || '%'
            "#,
        )
        .bind(library_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// List all media files linked to a movie
    pub async fn list_by_movie(&self, movie_id: Uuid) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE movie_id = $1
            ORDER BY size_bytes DESC
            "#,
        )
        .bind(movie_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a media file by movie ID (returns the first/primary file for a movie)
    pub async fn get_by_movie_id(&self, movie_id: Uuid) -> Result<Option<MediaFileRecord>> {
        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE movie_id = $1
            ORDER BY size_bytes DESC
            LIMIT 1
            "#,
        )
        .bind(movie_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a media file by ID
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM media_files WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // =========================================================================
    // Metadata Storage Methods
    // =========================================================================

    /// Update FFprobe analysis results and mark as analyzed
    pub async fn update_ffprobe_results(
        &self,
        id: Uuid,
        container: Option<&str>,
        video_codec: Option<&str>,
        audio_codec: Option<&str>,
        width: Option<i32>,
        height: Option<i32>,
        duration: Option<i32>,
        bitrate: Option<i32>,
        resolution: Option<&str>,
        is_hdr: Option<bool>,
        hdr_type: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET
                container = $2,
                video_codec = $3,
                audio_codec = $4,
                width = $5,
                height = $6,
                duration = $7,
                bitrate = $8,
                resolution = $9,
                is_hdr = $10,
                hdr_type = $11,
                ffprobe_analyzed_at = NOW(),
                modified_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(container)
        .bind(video_codec)
        .bind(audio_codec)
        .bind(width)
        .bind(height)
        .bind(duration)
        .bind(bitrate)
        .bind(resolution)
        .bind(is_hdr)
        .bind(hdr_type)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update embedded metadata (ID3/Vorbis tags) and mark as extracted
    pub async fn update_embedded_metadata(
        &self,
        id: Uuid,
        metadata: &EmbeddedMetadata,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET
                meta_artist = $2,
                meta_album = $3,
                meta_title = $4,
                meta_track_number = $5,
                meta_disc_number = $6,
                meta_year = $7,
                meta_genre = $8,
                meta_show_name = $9,
                meta_season = $10,
                meta_episode = $11,
                cover_art_base64 = $12,
                cover_art_mime = $13,
                lyrics = $14,
                metadata_extracted_at = NOW(),
                modified_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(&metadata.artist)
        .bind(&metadata.album)
        .bind(&metadata.title)
        .bind(metadata.track_number)
        .bind(metadata.disc_number)
        .bind(metadata.year)
        .bind(&metadata.genre)
        .bind(&metadata.show_name)
        .bind(metadata.season)
        .bind(metadata.episode)
        .bind(&metadata.cover_art_base64)
        .bind(&metadata.cover_art_mime)
        .bind(&metadata.lyrics)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update the match (link to library item) and mark as matched
    pub async fn update_match(
        &self,
        id: Uuid,
        episode_id: Option<Uuid>,
        movie_id: Option<Uuid>,
        track_id: Option<Uuid>,
        album_id: Option<Uuid>,
        audiobook_id: Option<Uuid>,
        content_type: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET
                episode_id = $2,
                movie_id = $3,
                track_id = $4,
                album_id = $5,
                audiobook_id = $6,
                content_type = $7,
                matched_at = NOW(),
                modified_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(episode_id)
        .bind(movie_id)
        .bind(track_id)
        .bind(album_id)
        .bind(audiobook_id)
        .bind(content_type)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create an unmatched media file entry for a file in a library
    pub async fn create_unmatched(
        &self,
        library_id: Uuid,
        path: &str,
        size_bytes: i64,
    ) -> Result<MediaFileRecord> {
        let original_name = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            INSERT INTO media_files (library_id, path, size, original_name, organized)
            VALUES ($1, $2, $3, $4, false)
            RETURNING id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            "#,
        )
        .bind(library_id)
        .bind(path)
        .bind(size_bytes)
        .bind(original_name)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// List files that need FFprobe analysis
    pub async fn list_needing_ffprobe(&self, library_id: Uuid, limit: i32) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE library_id = $1 AND ffprobe_analyzed_at IS NULL
            ORDER BY added_at
            LIMIT $2
            "#,
        )
        .bind(library_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List files that need metadata extraction
    pub async fn list_needing_metadata(&self, library_id: Uuid, limit: i32) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE library_id = $1 AND metadata_extracted_at IS NULL
            ORDER BY added_at
            LIMIT $2
            "#,
        )
        .bind(library_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List unmatched files in a library
    pub async fn list_unmatched(&self, library_id: Uuid, limit: i32) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, movie_id,
                   track_id, album_id, audiobook_id, chapter_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, organize_status, organize_error, added_at, modified_at,
                   content_type, quality_status,
                   meta_artist, meta_album, meta_title, meta_track_number, meta_disc_number,
                   meta_year, meta_genre, meta_show_name, meta_season, meta_episode,
                   ffprobe_analyzed_at, metadata_extracted_at, matched_at,
                   cover_art_base64, cover_art_mime, lyrics
            FROM media_files
            WHERE library_id = $1 
              AND episode_id IS NULL 
              AND movie_id IS NULL 
              AND track_id IS NULL 
              AND audiobook_id IS NULL
            ORDER BY path
            LIMIT $2
            "#,
        )
        .bind(library_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}

/// Embedded metadata extracted from file tags (ID3/Vorbis/container)
#[derive(Debug, Clone, Default)]
pub struct EmbeddedMetadata {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub title: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub show_name: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    /// Cover art as base64-encoded string
    pub cover_art_base64: Option<String>,
    /// MIME type of the cover art
    pub cover_art_mime: Option<String>,
    /// Lyrics from embedded tags
    pub lyrics: Option<String>,
}
