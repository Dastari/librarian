//! Subtitle database repository
//!
//! Handles storage and retrieval of subtitle tracks - embedded in media files,
//! external files alongside media, and downloaded from services like OpenSubtitles.

use anyhow::Result;
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{
    bool_to_int, int_to_bool, str_to_datetime, str_to_uuid, uuid_to_str,
};

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
#[derive(Debug, Clone)]
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


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for SubtitleRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let media_file_id_str: String = row.try_get("media_file_id")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;
        let downloaded_at_str: Option<String> = row.try_get("downloaded_at")?;

        // Booleans stored as INTEGER
        let is_default: i32 = row.try_get("is_default")?;
        let is_forced: i32 = row.try_get("is_forced")?;
        let is_hearing_impaired: i32 = row.try_get("is_hearing_impaired")?;

        // Metadata stored as JSON TEXT
        let metadata_str: Option<String> = row.try_get("metadata")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            media_file_id: str_to_uuid(&media_file_id_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            source_type: row.try_get("source_type")?,
            stream_index: row.try_get("stream_index")?,
            file_path: row.try_get("file_path")?,
            codec: row.try_get("codec")?,
            codec_long_name: row.try_get("codec_long_name")?,
            language: row.try_get("language")?,
            title: row.try_get("title")?,
            is_default: int_to_bool(is_default),
            is_forced: int_to_bool(is_forced),
            is_hearing_impaired: int_to_bool(is_hearing_impaired),
            opensubtitles_id: row.try_get("opensubtitles_id")?,
            downloaded_at: downloaded_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            metadata: metadata_str
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e: serde_json::Error| sqlx::Error::Decode(Box::new(e)))?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
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
#[derive(Debug, Clone)]
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


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for VideoStreamRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let media_file_id_str: String = row.try_get("media_file_id")?;
        let created_str: String = row.try_get("created_at")?;
        let is_default: i32 = row.try_get("is_default")?;
        let metadata_str: Option<String> = row.try_get("metadata")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            media_file_id: str_to_uuid(&media_file_id_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            stream_index: row.try_get("stream_index")?,
            codec: row.try_get("codec")?,
            codec_long_name: row.try_get("codec_long_name")?,
            width: row.try_get("width")?,
            height: row.try_get("height")?,
            aspect_ratio: row.try_get("aspect_ratio")?,
            frame_rate: row.try_get("frame_rate")?,
            avg_frame_rate: row.try_get("avg_frame_rate")?,
            bitrate: row.try_get("bitrate")?,
            pixel_format: row.try_get("pixel_format")?,
            color_space: row.try_get("color_space")?,
            color_transfer: row.try_get("color_transfer")?,
            color_primaries: row.try_get("color_primaries")?,
            hdr_type: row.try_get("hdr_type")?,
            bit_depth: row.try_get("bit_depth")?,
            language: row.try_get("language")?,
            title: row.try_get("title")?,
            is_default: int_to_bool(is_default),
            metadata: metadata_str
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e: serde_json::Error| sqlx::Error::Decode(Box::new(e)))?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Audio stream record from database
#[derive(Debug, Clone)]
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


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for AudioStreamRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let media_file_id_str: String = row.try_get("media_file_id")?;
        let created_str: String = row.try_get("created_at")?;
        let is_default: i32 = row.try_get("is_default")?;
        let is_commentary: i32 = row.try_get("is_commentary")?;
        let metadata_str: Option<String> = row.try_get("metadata")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            media_file_id: str_to_uuid(&media_file_id_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            stream_index: row.try_get("stream_index")?,
            codec: row.try_get("codec")?,
            codec_long_name: row.try_get("codec_long_name")?,
            channels: row.try_get("channels")?,
            channel_layout: row.try_get("channel_layout")?,
            sample_rate: row.try_get("sample_rate")?,
            bitrate: row.try_get("bitrate")?,
            bit_depth: row.try_get("bit_depth")?,
            language: row.try_get("language")?,
            title: row.try_get("title")?,
            is_default: int_to_bool(is_default),
            is_commentary: int_to_bool(is_commentary),
            metadata: metadata_str
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e: serde_json::Error| sqlx::Error::Decode(Box::new(e)))?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Chapter record from database
#[derive(Debug, Clone)]
pub struct ChapterRecord {
    pub id: Uuid,
    pub media_file_id: Uuid,
    pub chapter_index: i32,
    pub start_secs: f64,
    pub end_secs: f64,
    pub title: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for ChapterRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let media_file_id_str: String = row.try_get("media_file_id")?;
        let created_str: String = row.try_get("created_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            media_file_id: str_to_uuid(&media_file_id_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            chapter_index: row.try_get("chapter_index")?,
            start_secs: row.try_get("start_secs")?,
            end_secs: row.try_get("end_secs")?,
            title: row.try_get("title")?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Subtitle repository for database operations
pub struct SubtitleRepository {
    pool: DbPool,
}

impl SubtitleRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get all subtitles for a media file

    #[cfg(feature = "sqlite")]
    pub async fn list_by_media_file(&self, media_file_id: Uuid) -> Result<Vec<SubtitleRecord>> {
        let records = sqlx::query_as::<_, SubtitleRecord>(
            r#"
            SELECT id, media_file_id, source_type, stream_index, file_path,
                   codec, codec_long_name, language, title, is_default,
                   is_forced, is_hearing_impaired, opensubtitles_id,
                   downloaded_at, metadata, created_at, updated_at
            FROM subtitles
            WHERE media_file_id = ?1
            ORDER BY 
                CASE source_type 
                    WHEN 'embedded' THEN 1 
                    WHEN 'external' THEN 2 
                    WHEN 'downloaded' THEN 3 
                END,
                CASE WHEN stream_index IS NULL THEN 1 ELSE 0 END,
                stream_index,
                CASE WHEN language IS NULL THEN 1 ELSE 0 END,
                language
            "#,
        )
        .bind(uuid_to_str(media_file_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get subtitles for an episode (via media file join)

    #[cfg(feature = "sqlite")]
    pub async fn list_by_episode(&self, episode_id: Uuid) -> Result<Vec<SubtitleRecord>> {
        let records = sqlx::query_as::<_, SubtitleRecord>(
            r#"
            SELECT s.id, s.media_file_id, s.source_type, s.stream_index, s.file_path,
                   s.codec, s.codec_long_name, s.language, s.title, s.is_default,
                   s.is_forced, s.is_hearing_impaired, s.opensubtitles_id,
                   s.downloaded_at, s.metadata, s.created_at, s.updated_at
            FROM subtitles s
            JOIN media_files mf ON mf.id = s.media_file_id
            WHERE mf.episode_id = ?1
            ORDER BY 
                CASE s.source_type 
                    WHEN 'embedded' THEN 1 
                    WHEN 'external' THEN 2 
                    WHEN 'downloaded' THEN 3 
                END,
                CASE WHEN s.stream_index IS NULL THEN 1 ELSE 0 END,
                s.stream_index,
                CASE WHEN s.language IS NULL THEN 1 ELSE 0 END,
                s.language
            "#,
        )
        .bind(uuid_to_str(episode_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get subtitles by language for a media file

    #[cfg(feature = "sqlite")]
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
            WHERE media_file_id = ?1 AND language = ?2
            ORDER BY source_type,
                CASE WHEN stream_index IS NULL THEN 1 ELSE 0 END,
                stream_index
            "#,
        )
        .bind(uuid_to_str(media_file_id))
        .bind(language)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Check if a media file has subtitles for a specific language

    #[cfg(feature = "sqlite")]
    pub async fn has_language(&self, media_file_id: Uuid, language: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM subtitles WHERE media_file_id = ?1 AND language = ?2",
        )
        .bind(uuid_to_str(media_file_id))
        .bind(language)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Get languages that are missing for a media file

    #[cfg(feature = "sqlite")]
    pub async fn get_missing_languages(
        &self,
        media_file_id: Uuid,
        wanted_languages: &[String],
    ) -> Result<Vec<String>> {
        if wanted_languages.is_empty() {
            return Ok(vec![]);
        }

        // SQLite doesn't support ANY(), so we build a dynamic IN clause
        let placeholders: Vec<String> = (0..wanted_languages.len())
            .map(|i| format!("?{}", i + 2))
            .collect();
        let query = format!(
            r#"
            SELECT DISTINCT language 
            FROM subtitles 
            WHERE media_file_id = ?1 AND language IN ({})
            "#,
            placeholders.join(", ")
        );

        let mut query_builder = sqlx::query_scalar::<_, String>(&query);
        query_builder = query_builder.bind(uuid_to_str(media_file_id));
        for lang in wanted_languages {
            query_builder = query_builder.bind(lang);
        }

        let existing: Vec<String> = query_builder.fetch_all(&self.pool).await?;

        let missing: Vec<String> = wanted_languages
            .iter()
            .filter(|lang| !existing.contains(lang))
            .cloned()
            .collect();

        Ok(missing)
    }

    /// Create an embedded subtitle record

    #[cfg(feature = "sqlite")]
    pub async fn create_embedded(&self, input: CreateEmbeddedSubtitle) -> Result<SubtitleRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        let metadata_json = input
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "null".to_string()));

        sqlx::query(
            r#"
            INSERT INTO subtitles (
                id, media_file_id, source_type, stream_index, codec, codec_long_name,
                language, title, is_default, is_forced, is_hearing_impaired, metadata,
                created_at, updated_at
            )
            VALUES (?1, ?2, 'embedded', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                    datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.media_file_id))
        .bind(input.stream_index)
        .bind(&input.codec)
        .bind(&input.codec_long_name)
        .bind(&input.language)
        .bind(&input.title)
        .bind(bool_to_int(input.is_default))
        .bind(bool_to_int(input.is_forced))
        .bind(bool_to_int(input.is_hearing_impaired))
        .bind(&metadata_json)
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve subtitle after insert"))
    }

    /// Create an external subtitle record

    #[cfg(feature = "sqlite")]
    pub async fn create_external(&self, input: CreateExternalSubtitle) -> Result<SubtitleRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        // Detect format from file extension
        let format = std::path::Path::new(&input.file_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        sqlx::query(
            r#"
            INSERT INTO subtitles (
                id, media_file_id, source_type, file_path, codec, language,
                is_forced, is_hearing_impaired, is_default,
                created_at, updated_at
            )
            VALUES (?1, ?2, 'external', ?3, ?4, ?5, ?6, ?7, 0,
                    datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.media_file_id))
        .bind(&input.file_path)
        .bind(&format)
        .bind(&input.language)
        .bind(bool_to_int(input.is_forced))
        .bind(bool_to_int(input.is_hearing_impaired))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve subtitle after insert"))
    }

    /// Create a downloaded subtitle record

    #[cfg(feature = "sqlite")]
    pub async fn create_downloaded(
        &self,
        input: CreateDownloadedSubtitle,
    ) -> Result<SubtitleRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        let format = std::path::Path::new(&input.file_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        sqlx::query(
            r#"
            INSERT INTO subtitles (
                id, media_file_id, source_type, file_path, codec, language,
                opensubtitles_id, is_hearing_impaired, is_default, is_forced,
                downloaded_at, created_at, updated_at
            )
            VALUES (?1, ?2, 'downloaded', ?3, ?4, ?5, ?6, ?7, 0, 0,
                    datetime('now'), datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.media_file_id))
        .bind(&input.file_path)
        .bind(&format)
        .bind(&input.language)
        .bind(&input.opensubtitles_id)
        .bind(bool_to_int(input.is_hearing_impaired))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve subtitle after insert"))
    }

    /// Delete all subtitles for a media file

    #[cfg(feature = "sqlite")]
    pub async fn delete_by_media_file(&self, media_file_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM subtitles WHERE media_file_id = ?1")
            .bind(uuid_to_str(media_file_id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete subtitles by source type for a media file

    #[cfg(feature = "sqlite")]
    pub async fn delete_by_source_type(
        &self,
        media_file_id: Uuid,
        source_type: SubtitleSourceType,
    ) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM subtitles WHERE media_file_id = ?1 AND source_type = ?2")
                .bind(uuid_to_str(media_file_id))
                .bind(source_type.as_str())
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected())
    }

    /// Delete a specific subtitle by ID

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM subtitles WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get a subtitle by ID

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<SubtitleRecord>> {
        let record = sqlx::query_as::<_, SubtitleRecord>(
            r#"
            SELECT id, media_file_id, source_type, stream_index, file_path,
                   codec, codec_long_name, language, title, is_default,
                   is_forced, is_hearing_impaired, opensubtitles_id,
                   downloaded_at, metadata, created_at, updated_at
            FROM subtitles
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }
}

/// Stream repository for video/audio streams
pub struct StreamRepository {
    pool: DbPool,
}

impl StreamRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get all video streams for a media file

    #[cfg(feature = "sqlite")]
    pub async fn list_video_streams(&self, media_file_id: Uuid) -> Result<Vec<VideoStreamRecord>> {
        let records = sqlx::query_as::<_, VideoStreamRecord>(
            r#"
            SELECT id, media_file_id, stream_index, codec, codec_long_name,
                   width, height, aspect_ratio, frame_rate, avg_frame_rate,
                   bitrate, pixel_format, color_space, color_transfer, color_primaries,
                   hdr_type, bit_depth, language, title, is_default, metadata, created_at
            FROM video_streams
            WHERE media_file_id = ?1
            ORDER BY stream_index
            "#,
        )
        .bind(uuid_to_str(media_file_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all audio streams for a media file

    #[cfg(feature = "sqlite")]
    pub async fn list_audio_streams(&self, media_file_id: Uuid) -> Result<Vec<AudioStreamRecord>> {
        let records = sqlx::query_as::<_, AudioStreamRecord>(
            r#"
            SELECT id, media_file_id, stream_index, codec, codec_long_name,
                   channels, channel_layout, sample_rate, bitrate, bit_depth,
                   language, title, is_default, is_commentary, metadata, created_at
            FROM audio_streams
            WHERE media_file_id = ?1
            ORDER BY stream_index
            "#,
        )
        .bind(uuid_to_str(media_file_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all chapters for a media file

    #[cfg(feature = "sqlite")]
    pub async fn list_chapters(&self, media_file_id: Uuid) -> Result<Vec<ChapterRecord>> {
        let records = sqlx::query_as::<_, ChapterRecord>(
            r#"
            SELECT id, media_file_id, chapter_index, start_secs, end_secs, title, created_at
            FROM media_chapters
            WHERE media_file_id = ?1
            ORDER BY chapter_index
            "#,
        )
        .bind(uuid_to_str(media_file_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}
