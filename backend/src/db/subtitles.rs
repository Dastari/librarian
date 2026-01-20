//! Subtitle database repository
//!
//! Handles storage and retrieval of subtitle tracks - embedded in media files,
//! external files alongside media, and downloaded from services like OpenSubtitles.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Subtitle source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitleSourceType {
    /// Embedded in the media container (MKV, MP4, etc.)
    Embedded,
    /// External file alongside the media file (.srt, .ass, etc.)
    External,
    /// Downloaded from a subtitle service
    Downloaded,
}

impl SubtitleSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubtitleSourceType::Embedded => "embedded",
            SubtitleSourceType::External => "external",
            SubtitleSourceType::Downloaded => "downloaded",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "embedded" => Some(SubtitleSourceType::Embedded),
            "external" => Some(SubtitleSourceType::External),
            "downloaded" => Some(SubtitleSourceType::Downloaded),
            _ => None,
        }
    }
}

/// Subtitle record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubtitleRecord {
    pub id: Uuid,
    pub media_file_id: Uuid,
    pub source_type: String,
    pub stream_index: Option<i32>,
    pub file_path: Option<String>,
    pub codec: Option<String>,
    pub codec_long_name: Option<String>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_default: bool,
    pub is_forced: bool,
    pub is_hearing_impaired: bool,
    pub opensubtitles_id: Option<String>,
    pub downloaded_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl SubtitleRecord {
    /// Get the source type as enum
    pub fn source_type_enum(&self) -> Option<SubtitleSourceType> {
        SubtitleSourceType::from_str(&self.source_type)
    }
}

/// Input for creating an embedded subtitle record
#[derive(Debug)]
pub struct CreateEmbeddedSubtitle {
    pub media_file_id: Uuid,
    pub stream_index: i32,
    pub codec: Option<String>,
    pub codec_long_name: Option<String>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_default: bool,
    pub is_forced: bool,
    pub is_hearing_impaired: bool,
    pub metadata: Option<serde_json::Value>,
}

/// Input for creating an external subtitle record
#[derive(Debug)]
pub struct CreateExternalSubtitle {
    pub media_file_id: Uuid,
    pub file_path: String,
    pub language: Option<String>,
    pub is_forced: bool,
    pub is_hearing_impaired: bool,
}

/// Input for creating a downloaded subtitle record
#[derive(Debug)]
pub struct CreateDownloadedSubtitle {
    pub media_file_id: Uuid,
    pub file_path: String,
    pub language: Option<String>,
    pub opensubtitles_id: Option<String>,
    pub is_hearing_impaired: bool,
}

/// Video stream record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VideoStreamRecord {
    pub id: Uuid,
    pub media_file_id: Uuid,
    pub stream_index: i32,
    pub codec: String,
    pub codec_long_name: Option<String>,
    pub width: i32,
    pub height: i32,
    pub aspect_ratio: Option<String>,
    pub frame_rate: Option<String>,
    pub avg_frame_rate: Option<String>,
    pub bitrate: Option<i64>,
    pub pixel_format: Option<String>,
    pub color_space: Option<String>,
    pub color_transfer: Option<String>,
    pub color_primaries: Option<String>,
    pub hdr_type: Option<String>,
    pub bit_depth: Option<i32>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_default: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Audio stream record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AudioStreamRecord {
    pub id: Uuid,
    pub media_file_id: Uuid,
    pub stream_index: i32,
    pub codec: String,
    pub codec_long_name: Option<String>,
    pub channels: i32,
    pub channel_layout: Option<String>,
    pub sample_rate: Option<i32>,
    pub bitrate: Option<i64>,
    pub bit_depth: Option<i32>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_default: bool,
    pub is_commentary: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Chapter record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChapterRecord {
    pub id: Uuid,
    pub media_file_id: Uuid,
    pub chapter_index: i32,
    pub start_secs: f64,
    pub end_secs: f64,
    pub title: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Subtitle repository for database operations
pub struct SubtitleRepository {
    pool: PgPool,
}

impl SubtitleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all subtitles for a media file
    pub async fn list_by_media_file(&self, media_file_id: Uuid) -> Result<Vec<SubtitleRecord>> {
        let records = sqlx::query_as::<_, SubtitleRecord>(
            r#"
            SELECT id, media_file_id, source_type, stream_index, file_path,
                   codec, codec_long_name, language, title, is_default,
                   is_forced, is_hearing_impaired, opensubtitles_id,
                   downloaded_at, metadata, created_at, updated_at
            FROM subtitles
            WHERE media_file_id = $1
            ORDER BY 
                CASE source_type 
                    WHEN 'embedded' THEN 1 
                    WHEN 'external' THEN 2 
                    WHEN 'downloaded' THEN 3 
                END,
                stream_index NULLS LAST,
                language NULLS LAST
            "#,
        )
        .bind(media_file_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get subtitles for an episode (via media file join)
    pub async fn list_by_episode(&self, episode_id: Uuid) -> Result<Vec<SubtitleRecord>> {
        let records = sqlx::query_as::<_, SubtitleRecord>(
            r#"
            SELECT s.id, s.media_file_id, s.source_type, s.stream_index, s.file_path,
                   s.codec, s.codec_long_name, s.language, s.title, s.is_default,
                   s.is_forced, s.is_hearing_impaired, s.opensubtitles_id,
                   s.downloaded_at, s.metadata, s.created_at, s.updated_at
            FROM subtitles s
            JOIN media_files mf ON mf.id = s.media_file_id
            WHERE mf.episode_id = $1
            ORDER BY 
                CASE s.source_type 
                    WHEN 'embedded' THEN 1 
                    WHEN 'external' THEN 2 
                    WHEN 'downloaded' THEN 3 
                END,
                s.stream_index NULLS LAST,
                s.language NULLS LAST
            "#,
        )
        .bind(episode_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get subtitles by language for a media file
    pub async fn list_by_language(
        &self,
        media_file_id: Uuid,
        language: &str,
    ) -> Result<Vec<SubtitleRecord>> {
        let records = sqlx::query_as::<_, SubtitleRecord>(
            r#"
            SELECT id, media_file_id, source_type, stream_index, file_path,
                   codec, codec_long_name, language, title, is_default,
                   is_forced, is_hearing_impaired, opensubtitles_id,
                   downloaded_at, metadata, created_at, updated_at
            FROM subtitles
            WHERE media_file_id = $1 AND language = $2
            ORDER BY source_type, stream_index NULLS LAST
            "#,
        )
        .bind(media_file_id)
        .bind(language)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Check if a media file has subtitles for a specific language
    pub async fn has_language(&self, media_file_id: Uuid, language: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM subtitles WHERE media_file_id = $1 AND language = $2",
        )
        .bind(media_file_id)
        .bind(language)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Get languages that are missing for a media file
    pub async fn get_missing_languages(
        &self,
        media_file_id: Uuid,
        wanted_languages: &[String],
    ) -> Result<Vec<String>> {
        if wanted_languages.is_empty() {
            return Ok(vec![]);
        }

        let existing: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT language 
            FROM subtitles 
            WHERE media_file_id = $1 AND language = ANY($2)
            "#,
        )
        .bind(media_file_id)
        .bind(wanted_languages)
        .fetch_all(&self.pool)
        .await?;

        let missing: Vec<String> = wanted_languages
            .iter()
            .filter(|lang| !existing.contains(lang))
            .cloned()
            .collect();

        Ok(missing)
    }

    /// Create an embedded subtitle record
    pub async fn create_embedded(&self, input: CreateEmbeddedSubtitle) -> Result<SubtitleRecord> {
        let record = sqlx::query_as::<_, SubtitleRecord>(
            r#"
            INSERT INTO subtitles (
                media_file_id, source_type, stream_index, codec, codec_long_name,
                language, title, is_default, is_forced, is_hearing_impaired, metadata
            )
            VALUES ($1, 'embedded', $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, media_file_id, source_type, stream_index, file_path,
                      codec, codec_long_name, language, title, is_default,
                      is_forced, is_hearing_impaired, opensubtitles_id,
                      downloaded_at, metadata, created_at, updated_at
            "#,
        )
        .bind(input.media_file_id)
        .bind(input.stream_index)
        .bind(&input.codec)
        .bind(&input.codec_long_name)
        .bind(&input.language)
        .bind(&input.title)
        .bind(input.is_default)
        .bind(input.is_forced)
        .bind(input.is_hearing_impaired)
        .bind(&input.metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create an external subtitle record
    pub async fn create_external(&self, input: CreateExternalSubtitle) -> Result<SubtitleRecord> {
        // Detect format from file extension
        let format = std::path::Path::new(&input.file_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        let record = sqlx::query_as::<_, SubtitleRecord>(
            r#"
            INSERT INTO subtitles (
                media_file_id, source_type, file_path, codec, language,
                is_forced, is_hearing_impaired
            )
            VALUES ($1, 'external', $2, $3, $4, $5, $6)
            RETURNING id, media_file_id, source_type, stream_index, file_path,
                      codec, codec_long_name, language, title, is_default,
                      is_forced, is_hearing_impaired, opensubtitles_id,
                      downloaded_at, metadata, created_at, updated_at
            "#,
        )
        .bind(input.media_file_id)
        .bind(&input.file_path)
        .bind(&format)
        .bind(&input.language)
        .bind(input.is_forced)
        .bind(input.is_hearing_impaired)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a downloaded subtitle record
    pub async fn create_downloaded(
        &self,
        input: CreateDownloadedSubtitle,
    ) -> Result<SubtitleRecord> {
        let format = std::path::Path::new(&input.file_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        let record = sqlx::query_as::<_, SubtitleRecord>(
            r#"
            INSERT INTO subtitles (
                media_file_id, source_type, file_path, codec, language,
                opensubtitles_id, is_hearing_impaired, downloaded_at
            )
            VALUES ($1, 'downloaded', $2, $3, $4, $5, $6, NOW())
            RETURNING id, media_file_id, source_type, stream_index, file_path,
                      codec, codec_long_name, language, title, is_default,
                      is_forced, is_hearing_impaired, opensubtitles_id,
                      downloaded_at, metadata, created_at, updated_at
            "#,
        )
        .bind(input.media_file_id)
        .bind(&input.file_path)
        .bind(&format)
        .bind(&input.language)
        .bind(&input.opensubtitles_id)
        .bind(input.is_hearing_impaired)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete all subtitles for a media file
    pub async fn delete_by_media_file(&self, media_file_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM subtitles WHERE media_file_id = $1")
            .bind(media_file_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete subtitles by source type for a media file
    pub async fn delete_by_source_type(
        &self,
        media_file_id: Uuid,
        source_type: SubtitleSourceType,
    ) -> Result<u64> {
        let result = sqlx::query("DELETE FROM subtitles WHERE media_file_id = $1 AND source_type = $2")
            .bind(media_file_id)
            .bind(source_type.as_str())
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete a specific subtitle by ID
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM subtitles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get a subtitle by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<SubtitleRecord>> {
        let record = sqlx::query_as::<_, SubtitleRecord>(
            r#"
            SELECT id, media_file_id, source_type, stream_index, file_path,
                   codec, codec_long_name, language, title, is_default,
                   is_forced, is_hearing_impaired, opensubtitles_id,
                   downloaded_at, metadata, created_at, updated_at
            FROM subtitles
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }
}

/// Stream repository for video/audio streams
pub struct StreamRepository {
    pool: PgPool,
}

impl StreamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all video streams for a media file
    pub async fn list_video_streams(&self, media_file_id: Uuid) -> Result<Vec<VideoStreamRecord>> {
        let records = sqlx::query_as::<_, VideoStreamRecord>(
            r#"
            SELECT id, media_file_id, stream_index, codec, codec_long_name,
                   width, height, aspect_ratio, frame_rate, avg_frame_rate,
                   bitrate, pixel_format, color_space, color_transfer, color_primaries,
                   hdr_type, bit_depth, language, title, is_default, metadata, created_at
            FROM video_streams
            WHERE media_file_id = $1
            ORDER BY stream_index
            "#,
        )
        .bind(media_file_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all audio streams for a media file
    pub async fn list_audio_streams(&self, media_file_id: Uuid) -> Result<Vec<AudioStreamRecord>> {
        let records = sqlx::query_as::<_, AudioStreamRecord>(
            r#"
            SELECT id, media_file_id, stream_index, codec, codec_long_name,
                   channels, channel_layout, sample_rate, bitrate, bit_depth,
                   language, title, is_default, is_commentary, metadata, created_at
            FROM audio_streams
            WHERE media_file_id = $1
            ORDER BY stream_index
            "#,
        )
        .bind(media_file_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all chapters for a media file
    pub async fn list_chapters(&self, media_file_id: Uuid) -> Result<Vec<ChapterRecord>> {
        let records = sqlx::query_as::<_, ChapterRecord>(
            r#"
            SELECT id, media_file_id, chapter_index, start_secs, end_secs, title, created_at
            FROM media_chapters
            WHERE media_file_id = $1
            ORDER BY chapter_index
            "#,
        )
        .bind(media_file_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}
