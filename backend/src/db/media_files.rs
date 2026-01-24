//! Media files database repository

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

/// Media file record from database
#[derive(Debug, Clone)]
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

    // =========================================================================
    // Match Type Tracking
    // =========================================================================
    
    /// How the file was matched: 'automatic', 'manual', or None (unmatched)
    pub match_type: Option<String>,
    /// When the match was confirmed by a user
    pub match_confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// User ID who performed the manual match (None for automatic)
    pub matched_by_user_id: Option<Uuid>,

    // Album art and lyrics
    /// Cover art image as base64-encoded string
    pub cover_art_base64: Option<String>,
    /// MIME type of the cover art (e.g., "image/jpeg", "image/png")
    pub cover_art_mime: Option<String>,
    /// Lyrics extracted from embedded tags
    pub lyrics: Option<String>,

    // =========================================================================
    // Audio Fingerprinting
    // =========================================================================
    
    /// Chromaprint audio fingerprint for content identification
    pub audio_fingerprint: Option<String>,
    /// AcoustID recording ID if matched
    pub acoustid_id: Option<String>,
    /// MusicBrainz recording ID if matched via fingerprint
    pub musicbrainz_recording_id: Option<String>,
    /// When the fingerprint was generated
    pub fingerprint_generated_at: Option<chrono::DateTime<chrono::Utc>>,

    // =========================================================================
    // ReplayGain / Loudness Normalization
    // =========================================================================
    
    /// ReplayGain track gain in dB
    pub replaygain_track_gain: Option<f64>,
    /// ReplayGain track peak (linear)
    pub replaygain_track_peak: Option<f64>,
    /// ReplayGain album gain in dB
    pub replaygain_album_gain: Option<f64>,
    /// ReplayGain album peak (linear)
    pub replaygain_album_peak: Option<f64>,
    /// EBU R128 integrated loudness in LUFS
    pub r128_integrated_loudness: Option<f64>,
    /// EBU R128 true peak in dBTP
    pub r128_true_peak: Option<f64>,
    /// SoundCheck normalization value (iTunes format)
    pub soundcheck_value: Option<String>,

    // =========================================================================
    // Extended Audio Metadata
    // =========================================================================
    
    /// Album artist (distinct from track artist)
    pub meta_album_artist: Option<String>,
    /// Composer
    pub meta_composer: Option<String>,
    /// Conductor (classical music)
    pub meta_conductor: Option<String>,
    /// Label/Publisher
    pub meta_label: Option<String>,
    /// Catalog number
    pub meta_catalog_number: Option<String>,
    /// ISRC (International Standard Recording Code)
    pub meta_isrc: Option<String>,
    /// BPM (beats per minute)
    pub meta_bpm: Option<i32>,
    /// Initial key (musical key)
    pub meta_initial_key: Option<String>,
    /// Is this part of a compilation album?
    pub meta_is_compilation: Option<bool>,
    /// Gapless playback flag
    pub meta_gapless_playback: Option<bool>,
    /// Rating (0-100 scale)
    pub meta_rating: Option<i32>,
    /// Play count from embedded metadata
    pub meta_play_count: Option<i32>,

    // =========================================================================
    // Professional Audio (BWF - Broadcast Wave Format)
    // =========================================================================
    
    /// BWF originator
    pub bwf_originator: Option<String>,
    /// BWF originator reference
    pub bwf_originator_reference: Option<String>,
    /// BWF origination date (YYYY-MM-DD)
    pub bwf_origination_date: Option<String>,
    /// BWF origination time (HH:MM:SS)
    pub bwf_origination_time: Option<String>,
    /// BWF time reference (sample count since midnight)
    pub bwf_time_reference: Option<i64>,
    /// BWF UMID
    pub bwf_umid: Option<String>,
    /// BWF coding history
    pub bwf_coding_history: Option<String>,

    // =========================================================================
    // Video Container Metadata
    // =========================================================================
    
    /// Director
    pub meta_director: Option<String>,
    /// Producer
    pub meta_producer: Option<String>,
    /// Copyright notice
    pub meta_copyright: Option<String>,
    /// Encoder/creation tool
    pub meta_encoder: Option<String>,
    /// Content creation date
    pub meta_creation_date: Option<String>,

    // =========================================================================
    // DSD/High-Resolution Audio
    // =========================================================================
    
    /// DSD sample rate (2.8MHz, 5.6MHz, 11.2MHz, etc.)
    pub dsd_sample_rate: Option<i32>,
    /// Whether this is a DSD file
    pub is_dsd: Option<bool>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for MediaFileRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let episode_id_str: Option<String> = row.try_get("episode_id")?;
        let movie_id_str: Option<String> = row.try_get("movie_id")?;
        let track_id_str: Option<String> = row.try_get("track_id")?;
        let album_id_str: Option<String> = row.try_get("album_id")?;
        let audiobook_id_str: Option<String> = row.try_get("audiobook_id")?;
        let chapter_id_str: Option<String> = row.try_get("chapter_id")?;
        let organized_at_str: Option<String> = row.try_get("organized_at")?;
        let added_at_str: String = row.try_get("added_at")?;
        let modified_at_str: Option<String> = row.try_get("modified_at")?;
        let ffprobe_analyzed_at_str: Option<String> = row.try_get("ffprobe_analyzed_at")?;
        let metadata_extracted_at_str: Option<String> = row.try_get("metadata_extracted_at")?;
        let matched_at_str: Option<String> = row.try_get("matched_at")?;
        let organized_int: i32 = row.try_get("organized")?;
        let is_hdr_int: Option<i32> = row.try_get("is_hdr")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_id: str_to_uuid(&library_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            path: row.try_get("path")?,
            size_bytes: row.try_get("size_bytes")?,
            container: row.try_get("container")?,
            video_codec: row.try_get("video_codec")?,
            audio_codec: row.try_get("audio_codec")?,
            width: row.try_get("width")?,
            height: row.try_get("height")?,
            duration: row.try_get("duration")?,
            bitrate: row.try_get("bitrate")?,
            file_hash: row.try_get("file_hash")?,
            episode_id: episode_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            movie_id: movie_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            track_id: track_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            album_id: album_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            audiobook_id: audiobook_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            chapter_id: chapter_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            relative_path: row.try_get("relative_path")?,
            original_name: row.try_get("original_name")?,
            video_bitrate: row.try_get("video_bitrate")?,
            audio_channels: row.try_get("audio_channels")?,
            audio_language: row.try_get("audio_language")?,
            resolution: row.try_get("resolution")?,
            is_hdr: is_hdr_int.map(int_to_bool),
            hdr_type: row.try_get("hdr_type")?,
            organized: int_to_bool(organized_int),
            organized_at: organized_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            original_path: row.try_get("original_path")?,
            organize_status: row.try_get("organize_status")?,
            organize_error: row.try_get("organize_error")?,
            added_at: str_to_datetime(&added_at_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            modified_at: modified_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            content_type: row.try_get("content_type")?,
            quality_status: row.try_get("quality_status")?,
            meta_artist: row.try_get("meta_artist")?,
            meta_album: row.try_get("meta_album")?,
            meta_title: row.try_get("meta_title")?,
            meta_track_number: row.try_get("meta_track_number")?,
            meta_disc_number: row.try_get("meta_disc_number")?,
            meta_year: row.try_get("meta_year")?,
            meta_genre: row.try_get("meta_genre")?,
            meta_show_name: row.try_get("meta_show_name")?,
            meta_season: row.try_get("meta_season")?,
            meta_episode: row.try_get("meta_episode")?,
            ffprobe_analyzed_at: ffprobe_analyzed_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            metadata_extracted_at: metadata_extracted_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            matched_at: matched_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            match_type: row.try_get("match_type")?,
            match_confirmed_at: {
                let s: Option<String> = row.try_get("match_confirmed_at")?;
                s.map(|s| str_to_datetime(&s))
                    .transpose()
                    .map_err(|e| sqlx::Error::Decode(e.into()))?
            },
            matched_by_user_id: {
                let s: Option<String> = row.try_get("matched_by_user_id")?;
                s.map(|s| str_to_uuid(&s))
                    .transpose()
                    .map_err(|e| sqlx::Error::Decode(e.into()))?
            },
            cover_art_base64: row.try_get("cover_art_base64")?,
            cover_art_mime: row.try_get("cover_art_mime")?,
            lyrics: row.try_get("lyrics")?,
            
            // Fingerprinting fields
            audio_fingerprint: row.try_get("audio_fingerprint")?,
            acoustid_id: row.try_get("acoustid_id")?,
            musicbrainz_recording_id: row.try_get("musicbrainz_recording_id")?,
            fingerprint_generated_at: {
                let s: Option<String> = row.try_get("fingerprint_generated_at")?;
                s.map(|s| str_to_datetime(&s))
                    .transpose()
                    .map_err(|e| sqlx::Error::Decode(e.into()))?
            },
            
            // ReplayGain / Loudness fields
            replaygain_track_gain: row.try_get("replaygain_track_gain")?,
            replaygain_track_peak: row.try_get("replaygain_track_peak")?,
            replaygain_album_gain: row.try_get("replaygain_album_gain")?,
            replaygain_album_peak: row.try_get("replaygain_album_peak")?,
            r128_integrated_loudness: row.try_get("r128_integrated_loudness")?,
            r128_true_peak: row.try_get("r128_true_peak")?,
            soundcheck_value: row.try_get("soundcheck_value")?,
            
            // Extended audio metadata
            meta_album_artist: row.try_get("meta_album_artist")?,
            meta_composer: row.try_get("meta_composer")?,
            meta_conductor: row.try_get("meta_conductor")?,
            meta_label: row.try_get("meta_label")?,
            meta_catalog_number: row.try_get("meta_catalog_number")?,
            meta_isrc: row.try_get("meta_isrc")?,
            meta_bpm: row.try_get("meta_bpm")?,
            meta_initial_key: row.try_get("meta_initial_key")?,
            meta_is_compilation: {
                let i: Option<i32> = row.try_get("meta_is_compilation")?;
                i.map(int_to_bool)
            },
            meta_gapless_playback: {
                let i: Option<i32> = row.try_get("meta_gapless_playback")?;
                i.map(int_to_bool)
            },
            meta_rating: row.try_get("meta_rating")?,
            meta_play_count: row.try_get("meta_play_count")?,
            
            // BWF (Broadcast Wave Format) fields
            bwf_originator: row.try_get("bwf_originator")?,
            bwf_originator_reference: row.try_get("bwf_originator_reference")?,
            bwf_origination_date: row.try_get("bwf_origination_date")?,
            bwf_origination_time: row.try_get("bwf_origination_time")?,
            bwf_time_reference: row.try_get("bwf_time_reference")?,
            bwf_umid: row.try_get("bwf_umid")?,
            bwf_coding_history: row.try_get("bwf_coding_history")?,
            
            // Video container metadata
            meta_director: row.try_get("meta_director")?,
            meta_producer: row.try_get("meta_producer")?,
            meta_copyright: row.try_get("meta_copyright")?,
            meta_encoder: row.try_get("meta_encoder")?,
            meta_creation_date: row.try_get("meta_creation_date")?,
            
            // DSD/High-resolution audio
            dsd_sample_rate: row.try_get("dsd_sample_rate")?,
            is_dsd: {
                let i: Option<i32> = row.try_get("is_dsd")?;
                i.map(int_to_bool)
            },
        })
    }
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
    pool: DbPool,
}

/// Common SELECT columns for media files queries
const MEDIA_FILE_COLUMNS: &str = r#"
    id, library_id, path, size as size_bytes, 
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
    match_type, match_confirmed_at, matched_by_user_id,
    cover_art_base64, cover_art_mime, lyrics,
    audio_fingerprint, acoustid_id, musicbrainz_recording_id, fingerprint_generated_at,
    replaygain_track_gain, replaygain_track_peak, replaygain_album_gain, replaygain_album_peak,
    r128_integrated_loudness, r128_true_peak, soundcheck_value,
    meta_album_artist, meta_composer, meta_conductor, meta_label, meta_catalog_number,
    meta_isrc, meta_bpm, meta_initial_key, meta_is_compilation, meta_gapless_playback,
    meta_rating, meta_play_count,
    bwf_originator, bwf_originator_reference, bwf_origination_date, bwf_origination_time,
    bwf_time_reference, bwf_umid, bwf_coding_history,
    meta_director, meta_producer, meta_copyright, meta_encoder, meta_creation_date,
    dsd_sample_rate, is_dsd
"#;

impl MediaFileRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get all media files for a library

    #[cfg(feature = "sqlite")]
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE library_id = ?1 ORDER BY path",
            MEDIA_FILE_COLUMNS
        );
        let records = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(library_id))
            .fetch_all(&self.pool)
            .await?;

        Ok(records)
    }

    /// Check if a file path already exists

    #[cfg(feature = "sqlite")]
    pub async fn exists_by_path(&self, path: &str) -> Result<bool> {
        let result =
            sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM media_files WHERE path = ?1")
                .bind(path)
                .fetch_one(&self.pool)
                .await?;

        Ok(result > 0)
    }

    /// Get a media file by path

    #[cfg(feature = "sqlite")]
    pub async fn get_by_path(&self, path: &str) -> Result<Option<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE path = ?1",
            MEDIA_FILE_COLUMNS
        );
        let record = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(path)
            .fetch_optional(&self.pool)
            .await?;

        Ok(record)
    }

    /// List all media files linked to an episode

    #[cfg(feature = "sqlite")]
    pub async fn list_by_episode(&self, episode_id: Uuid) -> Result<Vec<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE episode_id = ?1 ORDER BY organized DESC, added_at ASC",
            MEDIA_FILE_COLUMNS
        );
        let records = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(episode_id))
            .fetch_all(&self.pool)
            .await?;

        Ok(records)
    }

    /// Create a new media file

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateMediaFile) -> Result<MediaFileRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO media_files (
                id, library_id, path, size, container, video_codec, audio_codec,
                width, height, duration, bitrate, file_hash, episode_id, movie_id,
                relative_path, original_name, resolution, is_hdr, hdr_type,
                organized, added_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, 0, datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.library_id))
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
        .bind(input.episode_id.map(uuid_to_str))
        .bind(input.movie_id.map(uuid_to_str))
        .bind(&input.relative_path)
        .bind(&input.original_name)
        .bind(&input.resolution)
        .bind(input.is_hdr.map(bool_to_int))
        .bind(&input.hdr_type)
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve media file after insert"))
    }

    /// Upsert a media file (insert or update if path exists)
    ///
    /// On conflict, updates file metadata AND links to the provided movie/episode.
    /// This ensures that when re-processing a download, the existing record gets
    /// properly linked to the content item.

    #[cfg(feature = "sqlite")]
    pub async fn upsert(&self, input: CreateMediaFile) -> Result<MediaFileRecord> {
        // Check if exists first
        let existing = self.get_by_path(&input.path).await?;

        if let Some(existing_record) = existing {
            // Determine content_type based on what's being linked
            let content_type = if input.movie_id.is_some() {
                Some("movie")
            } else if input.episode_id.is_some() {
                Some("episode")
            } else if input.track_id.is_some() {
                Some("track")
            } else if input.chapter_id.is_some() {
                Some("chapter")
            } else if input.audiobook_id.is_some() {
                Some("audiobook")
            } else {
                None
            };

            sqlx::query(
                r#"
                UPDATE media_files SET
                    size = ?2,
                    container = ?3,
                    video_codec = ?4,
                    audio_codec = ?5,
                    width = ?6,
                    height = ?7,
                    duration = ?8,
                    bitrate = ?9,
                    file_hash = ?10,
                    resolution = ?11,
                    is_hdr = ?12,
                    hdr_type = ?13,
                    movie_id = COALESCE(?14, movie_id),
                    episode_id = COALESCE(?15, episode_id),
                    track_id = COALESCE(?16, track_id),
                    album_id = COALESCE(?17, album_id),
                    audiobook_id = COALESCE(?18, audiobook_id),
                    chapter_id = COALESCE(?19, chapter_id),
                    content_type = COALESCE(?20, content_type),
                    modified_at = datetime('now')
                WHERE path = ?1
                "#,
            )
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
            .bind(&input.resolution)
            .bind(input.is_hdr.map(bool_to_int))
            .bind(&input.hdr_type)
            .bind(input.movie_id.map(uuid_to_str))
            .bind(input.episode_id.map(uuid_to_str))
            .bind(input.track_id.map(uuid_to_str))
            .bind(input.album_id.map(uuid_to_str))
            .bind(input.audiobook_id.map(uuid_to_str))
            .bind(input.chapter_id.map(uuid_to_str))
            .bind(content_type)
            .execute(&self.pool)
            .await?;

            return self.get_by_id(existing_record.id).await?
                .ok_or_else(|| anyhow::anyhow!("Failed to retrieve media file after update"));
        }

        // Insert new record
        let id = Uuid::new_v4();
        let content_type = if input.movie_id.is_some() {
            Some("movie")
        } else if input.episode_id.is_some() {
            Some("episode")
        } else if input.track_id.is_some() {
            Some("track")
        } else if input.chapter_id.is_some() {
            Some("chapter")
        } else if input.audiobook_id.is_some() {
            Some("audiobook")
        } else {
            None
        };

        sqlx::query(
            r#"
            INSERT INTO media_files (
                id, library_id, path, size, container, video_codec, audio_codec,
                width, height, duration, bitrate, file_hash, episode_id, movie_id,
                track_id, album_id, audiobook_id, chapter_id,
                relative_path, original_name, resolution, is_hdr, hdr_type,
                organized, content_type, added_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, 0, ?24, datetime('now'))
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(uuid_to_str(input.library_id))
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
        .bind(input.episode_id.map(uuid_to_str))
        .bind(input.movie_id.map(uuid_to_str))
        .bind(input.track_id.map(uuid_to_str))
        .bind(input.album_id.map(uuid_to_str))
        .bind(input.audiobook_id.map(uuid_to_str))
        .bind(input.chapter_id.map(uuid_to_str))
        .bind(&input.relative_path)
        .bind(&input.original_name)
        .bind(&input.resolution)
        .bind(input.is_hdr.map(bool_to_int))
        .bind(&input.hdr_type)
        .bind(content_type)
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve media file after insert"))
    }

    /// Delete media files that no longer exist on disk

    #[cfg(feature = "sqlite")]
    pub async fn delete_missing(&self, library_id: Uuid, existing_paths: &[String]) -> Result<u64> {
        if existing_paths.is_empty() {
            // Delete all files for this library if none exist
            let result = sqlx::query("DELETE FROM media_files WHERE library_id = ?1")
                .bind(uuid_to_str(library_id))
                .execute(&self.pool)
                .await?;
            return Ok(result.rows_affected());
        }

        // SQLite doesn't support != ALL($2) syntax
        // We need to fetch all paths and filter in Rust, or use NOT IN with dynamically built placeholders
        // For simplicity, fetch all current paths and delete those not in the list
        let all_files = self.list_by_library(library_id).await?;
        let existing_set: std::collections::HashSet<&str> =
            existing_paths.iter().map(|s| s.as_str()).collect();

        let mut deleted = 0u64;
        for file in all_files {
            if !existing_set.contains(file.path.as_str()) {
                if self.delete(file.id).await? {
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }

    /// Get count of files in a library

    #[cfg(feature = "sqlite")]
    pub async fn count_by_library(&self, library_id: Uuid) -> Result<i64> {
        let count =
            sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM media_files WHERE library_id = ?1")
                .bind(uuid_to_str(library_id))
                .fetch_one(&self.pool)
                .await?;

        Ok(count as i64)
    }

    /// Link a media file to an episode

    #[cfg(feature = "sqlite")]
    pub async fn link_to_episode(&self, file_id: Uuid, episode_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE media_files SET episode_id = ?2, content_type = 'episode' WHERE id = ?1",
        )
        .bind(uuid_to_str(file_id))
        .bind(uuid_to_str(episode_id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Link a media file to a movie

    #[cfg(feature = "sqlite")]
    pub async fn link_to_movie(&self, file_id: Uuid, movie_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE media_files SET movie_id = ?2, content_type = 'movie' WHERE id = ?1")
            .bind(uuid_to_str(file_id))
            .bind(uuid_to_str(movie_id))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Link a media file to a music album

    #[cfg(feature = "sqlite")]
    pub async fn link_to_album(&self, file_id: Uuid, album_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE media_files SET album_id = ?2 WHERE id = ?1")
            .bind(uuid_to_str(file_id))
            .bind(uuid_to_str(album_id))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Link a media file to a music track

    #[cfg(feature = "sqlite")]
    pub async fn link_to_track(&self, file_id: Uuid, track_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE media_files SET track_id = ?2, content_type = 'track' WHERE id = ?1")
            .bind(uuid_to_str(file_id))
            .bind(uuid_to_str(track_id))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Link a media file to an audiobook

    #[cfg(feature = "sqlite")]
    pub async fn link_to_audiobook(&self, file_id: Uuid, audiobook_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE media_files SET audiobook_id = ?2, content_type = 'audiobook' WHERE id = ?1",
        )
        .bind(uuid_to_str(file_id))
        .bind(uuid_to_str(audiobook_id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a media file by episode ID (returns the first/primary file for an episode)

    #[cfg(feature = "sqlite")]
    pub async fn get_by_episode_id(&self, episode_id: Uuid) -> Result<Option<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE episode_id = ?1 ORDER BY size_bytes DESC LIMIT 1",
            MEDIA_FILE_COLUMNS
        );
        let record = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(episode_id))
            .fetch_optional(&self.pool)
            .await?;

        Ok(record)
    }

    /// Get unorganized files for a library

    #[cfg(feature = "sqlite")]
    pub async fn list_unorganized_by_library(
        &self,
        library_id: Uuid,
    ) -> Result<Vec<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE library_id = ?1 AND organized = 0 ORDER BY path",
            MEDIA_FILE_COLUMNS
        );
        let records = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(library_id))
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

    #[cfg(feature = "sqlite")]
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
                        original_path = COALESCE(original_path, ?2),
                        organized = 1, 
                        organized_at = datetime('now'),
                        organize_status = 'organized',
                        organize_error = NULL,
                        modified_at = datetime('now')
                    WHERE id = ?1
                    "#,
                )
                .bind(uuid_to_str(existing.id))
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
                path = ?2, 
                original_path = ?3,
                organized = 1, 
                organized_at = datetime('now'),
                organize_status = 'organized',
                organize_error = NULL,
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(file_id))
        .bind(new_path)
        .bind(original_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a file as unorganized (needs to be re-organized)

    #[cfg(feature = "sqlite")]
    pub async fn mark_unorganized(&self, file_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET 
                organized = 0, 
                organized_at = NULL,
                organize_status = 'pending',
                organize_error = NULL,
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(file_id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a file as conflicted (cannot be organized due to conflict)

    #[cfg(feature = "sqlite")]
    pub async fn mark_conflicted(&self, file_id: Uuid, error_message: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET 
                organize_status = 'conflicted',
                organize_error = ?2,
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(file_id))
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a file organization as failed with an error

    #[cfg(feature = "sqlite")]
    pub async fn mark_organize_error(&self, file_id: Uuid, error_message: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET 
                organize_status = 'error',
                organize_error = ?2,
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(file_id))
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List conflicted files for a library

    #[cfg(feature = "sqlite")]
    pub async fn list_conflicted_by_library(
        &self,
        library_id: Uuid,
    ) -> Result<Vec<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE library_id = ?1 AND organize_status = 'conflicted' ORDER BY path",
            MEDIA_FILE_COLUMNS
        );
        let records = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(library_id))
            .fetch_all(&self.pool)
            .await?;

        Ok(records)
    }

    /// Update only the path of a media file

    #[cfg(feature = "sqlite")]
    pub async fn update_path(&self, file_id: Uuid, new_path: &str) -> Result<()> {
        sqlx::query("UPDATE media_files SET path = ?2, modified_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(file_id))
            .bind(new_path)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get a media file by ID

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE id = ?1",
            MEDIA_FILE_COLUMNS
        );
        let record = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(id))
            .fetch_optional(&self.pool)
            .await?;

        Ok(record)
    }

    /// Get unmatched files for a library (files not linked to any episode or movie)
    ///
    /// Only returns files that are actually within the library's folder path,
    /// not files in other locations (like downloads) that happen to be linked to the library.

    #[cfg(feature = "sqlite")]
    pub async fn list_unmatched_by_library(
        &self,
        library_id: Uuid,
    ) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT mf.id, mf.library_id, mf.path, mf.size as size_bytes, 
                   mf.container, mf.video_codec, mf.audio_codec, mf.width, mf.height,
                   mf.duration, mf.bitrate, mf.file_hash, mf.episode_id, mf.movie_id,
                   mf.track_id, mf.album_id, mf.audiobook_id, mf.chapter_id, mf.relative_path,
                   mf.original_name, mf.video_bitrate, mf.audio_channels, mf.audio_language,
                   mf.resolution, mf.is_hdr, mf.hdr_type, mf.organized, mf.organized_at,
                   mf.original_path, mf.organize_status, mf.organize_error, mf.added_at, mf.modified_at,
                   mf.content_type, mf.quality_status,
                   mf.meta_artist, mf.meta_album, mf.meta_title, mf.meta_track_number, mf.meta_disc_number,
                   mf.meta_year, mf.meta_genre, mf.meta_show_name, mf.meta_season, mf.meta_episode,
                   mf.ffprobe_analyzed_at, mf.metadata_extracted_at, mf.matched_at,
                   mf.cover_art_base64, mf.cover_art_mime, mf.lyrics
            FROM media_files mf
            JOIN libraries l ON mf.library_id = l.id
            WHERE mf.library_id = ?1 
              AND mf.episode_id IS NULL 
              AND mf.movie_id IS NULL
              AND mf.path LIKE l.path || '%'
            ORDER BY mf.path
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get count of unmatched files in a library
    ///
    /// Only counts files that are actually within the library's folder path.

    #[cfg(feature = "sqlite")]
    pub async fn count_unmatched_by_library(&self, library_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT COUNT(*) 
            FROM media_files mf
            JOIN libraries l ON mf.library_id = l.id
            WHERE mf.library_id = ?1 
              AND mf.episode_id IS NULL 
              AND mf.movie_id IS NULL
              AND mf.path LIKE l.path || '%'
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_one(&self.pool)
        .await?;

        Ok(count as i64)
    }

    /// List all media files linked to a movie

    #[cfg(feature = "sqlite")]
    pub async fn list_by_movie(&self, movie_id: Uuid) -> Result<Vec<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE movie_id = ?1 ORDER BY size_bytes DESC",
            MEDIA_FILE_COLUMNS
        );
        let records = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(movie_id))
            .fetch_all(&self.pool)
            .await?;

        Ok(records)
    }

    /// Get a media file by movie ID (returns the first/primary file for a movie)

    #[cfg(feature = "sqlite")]
    pub async fn get_by_movie_id(&self, movie_id: Uuid) -> Result<Option<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE movie_id = ?1 ORDER BY size_bytes DESC LIMIT 1",
            MEDIA_FILE_COLUMNS
        );
        let record = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(movie_id))
            .fetch_optional(&self.pool)
            .await?;

        Ok(record)
    }

    /// Delete a media file by ID

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM media_files WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // =========================================================================
    // Metadata Storage Methods
    // =========================================================================

    /// Update FFprobe analysis results and mark as analyzed

    #[cfg(feature = "sqlite")]
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
                container = ?2,
                video_codec = ?3,
                audio_codec = ?4,
                width = ?5,
                height = ?6,
                duration = ?7,
                bitrate = ?8,
                resolution = ?9,
                is_hdr = ?10,
                hdr_type = ?11,
                ffprobe_analyzed_at = datetime('now'),
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(container)
        .bind(video_codec)
        .bind(audio_codec)
        .bind(width)
        .bind(height)
        .bind(duration)
        .bind(bitrate)
        .bind(resolution)
        .bind(is_hdr.map(bool_to_int))
        .bind(hdr_type)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update embedded metadata (ID3/Vorbis tags) and mark as extracted

    #[cfg(feature = "sqlite")]
    pub async fn update_embedded_metadata(
        &self,
        id: Uuid,
        metadata: &EmbeddedMetadata,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET
                meta_artist = ?2,
                meta_album = ?3,
                meta_title = ?4,
                meta_track_number = ?5,
                meta_disc_number = ?6,
                meta_year = ?7,
                meta_genre = ?8,
                meta_show_name = ?9,
                meta_season = ?10,
                meta_episode = ?11,
                cover_art_base64 = ?12,
                cover_art_mime = ?13,
                lyrics = ?14,
                meta_album_artist = ?15,
                meta_composer = ?16,
                meta_conductor = ?17,
                meta_label = ?18,
                meta_catalog_number = ?19,
                meta_isrc = ?20,
                meta_bpm = ?21,
                meta_initial_key = ?22,
                meta_is_compilation = ?23,
                meta_gapless_playback = ?24,
                meta_rating = ?25,
                meta_play_count = ?26,
                replaygain_track_gain = ?27,
                replaygain_track_peak = ?28,
                replaygain_album_gain = ?29,
                replaygain_album_peak = ?30,
                soundcheck_value = ?31,
                meta_director = ?32,
                meta_producer = ?33,
                meta_copyright = ?34,
                meta_encoder = ?35,
                meta_creation_date = ?36,
                bwf_originator = ?37,
                bwf_originator_reference = ?38,
                bwf_origination_date = ?39,
                bwf_origination_time = ?40,
                bwf_time_reference = ?41,
                bwf_umid = ?42,
                bwf_coding_history = ?43,
                metadata_extracted_at = datetime('now'),
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
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
        .bind(&metadata.album_artist)
        .bind(&metadata.composer)
        .bind(&metadata.conductor)
        .bind(&metadata.label)
        .bind(&metadata.catalog_number)
        .bind(&metadata.isrc)
        .bind(metadata.bpm)
        .bind(&metadata.initial_key)
        .bind(metadata.is_compilation.map(bool_to_int))
        .bind(metadata.gapless_playback.map(bool_to_int))
        .bind(metadata.rating)
        .bind(metadata.play_count)
        .bind(metadata.replaygain_track_gain)
        .bind(metadata.replaygain_track_peak)
        .bind(metadata.replaygain_album_gain)
        .bind(metadata.replaygain_album_peak)
        .bind(&metadata.soundcheck_value)
        .bind(&metadata.director)
        .bind(&metadata.producer)
        .bind(&metadata.copyright)
        .bind(&metadata.encoder)
        .bind(&metadata.creation_date)
        .bind(&metadata.bwf_originator)
        .bind(&metadata.bwf_originator_reference)
        .bind(&metadata.bwf_origination_date)
        .bind(&metadata.bwf_origination_time)
        .bind(metadata.bwf_time_reference)
        .bind(&metadata.bwf_umid)
        .bind(&metadata.bwf_coding_history)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update the match (link to library item) and mark as matched automatically
    ///
    /// IMPORTANT: This will NOT update if the file was manually matched.
    /// Manual matches take precedence and should never be overwritten by automatic matching.

    #[cfg(feature = "sqlite")]
    pub async fn update_match(
        &self,
        id: Uuid,
        episode_id: Option<Uuid>,
        movie_id: Option<Uuid>,
        track_id: Option<Uuid>,
        album_id: Option<Uuid>,
        audiobook_id: Option<Uuid>,
        content_type: Option<&str>,
    ) -> Result<bool> {
        // Only update if match_type is not 'manual' (automatic matches don't override manual ones)
        let result = sqlx::query(
            r#"
            UPDATE media_files SET
                episode_id = ?2,
                movie_id = ?3,
                track_id = ?4,
                album_id = ?5,
                audiobook_id = ?6,
                content_type = ?7,
                match_type = 'automatic',
                matched_at = datetime('now'),
                modified_at = datetime('now')
            WHERE id = ?1 AND (match_type IS NULL OR match_type != 'manual')
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(episode_id.map(uuid_to_str))
        .bind(movie_id.map(uuid_to_str))
        .bind(track_id.map(uuid_to_str))
        .bind(album_id.map(uuid_to_str))
        .bind(audiobook_id.map(uuid_to_str))
        .bind(content_type)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Manually match a file to a library item
    ///
    /// Manual matches take precedence over automatic matches and will never be 
    /// overwritten by the scanner or automatic matching systems.

    #[cfg(feature = "sqlite")]
    pub async fn manual_match(
        &self,
        id: Uuid,
        user_id: Uuid,
        episode_id: Option<Uuid>,
        movie_id: Option<Uuid>,
        track_id: Option<Uuid>,
        album_id: Option<Uuid>,
        audiobook_id: Option<Uuid>,
        chapter_id: Option<Uuid>,
    ) -> Result<MediaFileRecord> {
        // Determine content_type based on what's being linked
        let content_type = if movie_id.is_some() {
            Some("movie")
        } else if episode_id.is_some() {
            Some("episode")
        } else if track_id.is_some() {
            Some("track")
        } else if chapter_id.is_some() {
            Some("chapter")
        } else if audiobook_id.is_some() {
            Some("audiobook")
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE media_files SET
                episode_id = ?2,
                movie_id = ?3,
                track_id = ?4,
                album_id = ?5,
                audiobook_id = ?6,
                chapter_id = ?7,
                content_type = ?8,
                match_type = 'manual',
                matched_at = datetime('now'),
                match_confirmed_at = datetime('now'),
                matched_by_user_id = ?9,
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(episode_id.map(uuid_to_str))
        .bind(movie_id.map(uuid_to_str))
        .bind(track_id.map(uuid_to_str))
        .bind(album_id.map(uuid_to_str))
        .bind(audiobook_id.map(uuid_to_str))
        .bind(chapter_id.map(uuid_to_str))
        .bind(content_type)
        .bind(uuid_to_str(user_id))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve media file after manual match"))
    }

    /// Unmatch a file (remove link to library item)
    ///
    /// This clears the match and allows the file to be re-matched.

    #[cfg(feature = "sqlite")]
    pub async fn unmatch(&self, id: Uuid) -> Result<MediaFileRecord> {
        sqlx::query(
            r#"
            UPDATE media_files SET
                episode_id = NULL,
                movie_id = NULL,
                track_id = NULL,
                album_id = NULL,
                audiobook_id = NULL,
                chapter_id = NULL,
                content_type = NULL,
                match_type = NULL,
                matched_at = NULL,
                match_confirmed_at = NULL,
                matched_by_user_id = NULL,
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve media file after unmatch"))
    }

    /// Check if a file was manually matched
    ///
    /// Returns true if the file has match_type = 'manual'

    #[cfg(feature = "sqlite")]
    pub async fn is_manually_matched(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query_scalar::<_, Option<String>>(
            "SELECT match_type FROM media_files WHERE id = ?1",
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.flatten() == Some("manual".to_string()))
    }

    /// Create an unmatched media file entry for a file in a library

    #[cfg(feature = "sqlite")]
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

        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO media_files (id, library_id, path, size, original_name, organized, added_at)
            VALUES (?1, ?2, ?3, ?4, ?5, 0, datetime('now'))
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(uuid_to_str(library_id))
        .bind(path)
        .bind(size_bytes)
        .bind(original_name)
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve media file after insert"))
    }

    /// List files that need FFprobe analysis

    #[cfg(feature = "sqlite")]
    pub async fn list_needing_ffprobe(
        &self,
        library_id: Uuid,
        limit: i32,
    ) -> Result<Vec<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE library_id = ?1 AND ffprobe_analyzed_at IS NULL ORDER BY added_at LIMIT ?2",
            MEDIA_FILE_COLUMNS
        );
        let records = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(library_id))
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        Ok(records)
    }

    /// List files that need metadata extraction

    #[cfg(feature = "sqlite")]
    pub async fn list_needing_metadata(
        &self,
        library_id: Uuid,
        limit: i32,
    ) -> Result<Vec<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE library_id = ?1 AND metadata_extracted_at IS NULL ORDER BY added_at LIMIT ?2",
            MEDIA_FILE_COLUMNS
        );
        let records = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(library_id))
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        Ok(records)
    }

    /// List unmatched files in a library

    #[cfg(feature = "sqlite")]
    pub async fn list_unmatched(
        &self,
        library_id: Uuid,
        limit: i32,
    ) -> Result<Vec<MediaFileRecord>> {
        let query = format!(
            r#"
            SELECT {} FROM media_files
            WHERE library_id = ?1 
              AND episode_id IS NULL 
              AND movie_id IS NULL 
              AND track_id IS NULL 
              AND audiobook_id IS NULL
            ORDER BY path
            LIMIT ?2
            "#,
            MEDIA_FILE_COLUMNS
        );
        let records = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(library_id))
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        Ok(records)
    }

    // =========================================================================
    // Fingerprinting Methods
    // =========================================================================

    /// Update audio fingerprint data for a media file
    #[cfg(feature = "sqlite")]
    pub async fn update_fingerprint(
        &self,
        id: Uuid,
        fingerprint: &str,
        acoustid_id: Option<&str>,
        musicbrainz_recording_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET
                audio_fingerprint = ?2,
                acoustid_id = ?3,
                musicbrainz_recording_id = ?4,
                fingerprint_generated_at = datetime('now'),
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(fingerprint)
        .bind(acoustid_id)
        .bind(musicbrainz_recording_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List files that need fingerprint generation
    #[cfg(feature = "sqlite")]
    pub async fn list_needing_fingerprint(
        &self,
        library_id: Uuid,
        limit: i32,
    ) -> Result<Vec<MediaFileRecord>> {
        let query = format!(
            "SELECT {} FROM media_files WHERE library_id = ?1 AND audio_fingerprint IS NULL AND (content_type = 'track' OR content_type IS NULL) ORDER BY added_at LIMIT ?2",
            MEDIA_FILE_COLUMNS
        );
        let records = sqlx::query_as::<_, MediaFileRecord>(&query)
            .bind(uuid_to_str(library_id))
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        Ok(records)
    }

    /// Update R128 loudness data (from FFmpeg analysis)
    #[cfg(feature = "sqlite")]
    pub async fn update_loudness(
        &self,
        id: Uuid,
        integrated_loudness: Option<f64>,
        true_peak: Option<f64>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET
                r128_integrated_loudness = ?2,
                r128_true_peak = ?3,
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(integrated_loudness)
        .bind(true_peak)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update DSD-specific metadata
    #[cfg(feature = "sqlite")]
    pub async fn update_dsd_info(
        &self,
        id: Uuid,
        is_dsd: bool,
        dsd_sample_rate: Option<i32>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET
                is_dsd = ?2,
                dsd_sample_rate = ?3,
                modified_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(bool_to_int(is_dsd))
        .bind(dsd_sample_rate)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Embedded metadata extracted from file tags (ID3/Vorbis/container)
#[derive(Debug, Clone, Default)]
pub struct EmbeddedMetadata {
    // Basic tags
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
    
    // Extended audio metadata
    /// Album artist (distinct from track artist)
    pub album_artist: Option<String>,
    /// Composer
    pub composer: Option<String>,
    /// Conductor
    pub conductor: Option<String>,
    /// Label/Publisher
    pub label: Option<String>,
    /// Catalog number
    pub catalog_number: Option<String>,
    /// ISRC (International Standard Recording Code)
    pub isrc: Option<String>,
    /// BPM (beats per minute)
    pub bpm: Option<i32>,
    /// Initial key (musical key)
    pub initial_key: Option<String>,
    /// Is this part of a compilation album?
    pub is_compilation: Option<bool>,
    /// Gapless playback flag
    pub gapless_playback: Option<bool>,
    /// Rating (0-100 scale)
    pub rating: Option<i32>,
    /// Play count
    pub play_count: Option<i32>,

    // ReplayGain / Loudness
    /// ReplayGain track gain in dB
    pub replaygain_track_gain: Option<f64>,
    /// ReplayGain track peak
    pub replaygain_track_peak: Option<f64>,
    /// ReplayGain album gain in dB
    pub replaygain_album_gain: Option<f64>,
    /// ReplayGain album peak
    pub replaygain_album_peak: Option<f64>,
    /// SoundCheck value (iTunes format)
    pub soundcheck_value: Option<String>,

    // Video metadata
    /// Director
    pub director: Option<String>,
    /// Producer
    pub producer: Option<String>,
    /// Copyright notice
    pub copyright: Option<String>,
    /// Encoder/creation tool
    pub encoder: Option<String>,
    /// Content creation date
    pub creation_date: Option<String>,

    // BWF (Broadcast Wave Format)
    /// BWF originator
    pub bwf_originator: Option<String>,
    /// BWF originator reference
    pub bwf_originator_reference: Option<String>,
    /// BWF origination date
    pub bwf_origination_date: Option<String>,
    /// BWF origination time
    pub bwf_origination_time: Option<String>,
    /// BWF time reference
    pub bwf_time_reference: Option<i64>,
    /// BWF UMID
    pub bwf_umid: Option<String>,
    /// BWF coding history
    pub bwf_coding_history: Option<String>,
}
