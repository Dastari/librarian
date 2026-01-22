//! Track database repository

use anyhow::Result;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// Track record from database
#[derive(Debug, Clone)]
pub struct TrackRecord {
    pub id: Uuid,
    pub album_id: Uuid,
    pub library_id: Uuid,
    // Basic info
    pub title: String,
    pub track_number: i32,
    pub disc_number: i32,
    // External IDs
    pub musicbrainz_id: Option<Uuid>,
    pub isrc: Option<String>,
    // Metadata
    pub duration_secs: Option<i32>,
    pub explicit: bool,
    // Artist info (for featured artists, may differ from album artist)
    pub artist_name: Option<String>,
    pub artist_id: Option<Uuid>,
    // File link
    pub media_file_id: Option<Uuid>,
    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for TrackRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            album_id: row.try_get("album_id")?,
            library_id: row.try_get("library_id")?,
            title: row.try_get("title")?,
            track_number: row.try_get("track_number")?,
            disc_number: row.try_get("disc_number")?,
            musicbrainz_id: row.try_get("musicbrainz_id")?,
            isrc: row.try_get("isrc")?,
            duration_secs: row.try_get("duration_secs")?,
            explicit: row.try_get("explicit")?,
            artist_name: row.try_get("artist_name")?,
            artist_id: row.try_get("artist_id")?,
            media_file_id: row.try_get("media_file_id")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for TrackRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use crate::db::sqlite_helpers::{int_to_bool, str_to_datetime, str_to_uuid};
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let album_id_str: String = row.try_get("album_id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let musicbrainz_id_str: Option<String> = row.try_get("musicbrainz_id")?;
        let artist_id_str: Option<String> = row.try_get("artist_id")?;
        let media_file_id_str: Option<String> = row.try_get("media_file_id")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;

        // Boolean stored as INTEGER
        let explicit: i32 = row.try_get("explicit")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            album_id: str_to_uuid(&album_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_id: str_to_uuid(&library_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            title: row.try_get("title")?,
            track_number: row.try_get("track_number")?,
            disc_number: row.try_get("disc_number")?,
            musicbrainz_id: musicbrainz_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            isrc: row.try_get("isrc")?,
            duration_secs: row.try_get("duration_secs")?,
            explicit: int_to_bool(explicit),
            artist_name: row.try_get("artist_name")?,
            artist_id: artist_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            media_file_id: media_file_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Input for creating a track
#[derive(Debug)]
pub struct CreateTrack {
    pub album_id: Uuid,
    pub library_id: Uuid,
    pub title: String,
    pub track_number: i32,
    pub disc_number: i32,
    pub musicbrainz_id: Option<Uuid>,
    pub isrc: Option<String>,
    pub duration_secs: Option<i32>,
    pub explicit: bool,
    pub artist_name: Option<String>,
    pub artist_id: Option<Uuid>,
}

/// Input for updating a track
#[derive(Debug, Default)]
pub struct UpdateTrack {
    pub title: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub duration_secs: Option<i32>,
    pub explicit: Option<bool>,
    pub artist_name: Option<String>,
    pub media_file_id: Option<Uuid>,
}

/// Track with file status for display
#[derive(Debug, Clone)]
pub struct TrackWithStatus {
    pub track: TrackRecord,
    pub has_file: bool,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    // Audio quality info from media file
    pub audio_codec: Option<String>,
    pub bitrate: Option<i32>,
    pub audio_channels: Option<String>,
}

/// Flat row struct for joined query (PostgreSQL)
#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
struct TrackWithFileRow {
    id: Uuid,
    album_id: Uuid,
    library_id: Uuid,
    title: String,
    track_number: i32,
    disc_number: i32,
    musicbrainz_id: Option<Uuid>,
    isrc: Option<String>,
    duration_secs: Option<i32>,
    explicit: bool,
    artist_name: Option<String>,
    artist_id: Option<Uuid>,
    media_file_id: Option<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    file_path: Option<String>,
    file_size: Option<i64>,
    // Audio quality info
    audio_codec: Option<String>,
    bitrate: Option<i32>,
    audio_channels: Option<String>,
}

/// Flat row struct for joined query (SQLite)
#[cfg(feature = "sqlite")]
struct TrackWithFileRowSqlite {
    id: Uuid,
    album_id: Uuid,
    library_id: Uuid,
    title: String,
    track_number: i32,
    disc_number: i32,
    musicbrainz_id: Option<Uuid>,
    isrc: Option<String>,
    duration_secs: Option<i32>,
    explicit: bool,
    artist_name: Option<String>,
    artist_id: Option<Uuid>,
    media_file_id: Option<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    file_path: Option<String>,
    file_size: Option<i64>,
    // Audio quality info
    audio_codec: Option<String>,
    bitrate: Option<i32>,
    audio_channels: Option<String>,
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for TrackWithFileRowSqlite {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use crate::db::sqlite_helpers::{int_to_bool, str_to_datetime, str_to_uuid};
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let album_id_str: String = row.try_get("album_id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let musicbrainz_id_str: Option<String> = row.try_get("musicbrainz_id")?;
        let artist_id_str: Option<String> = row.try_get("artist_id")?;
        let media_file_id_str: Option<String> = row.try_get("media_file_id")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;

        // Boolean stored as INTEGER
        let explicit: i32 = row.try_get("explicit")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            album_id: str_to_uuid(&album_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_id: str_to_uuid(&library_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            title: row.try_get("title")?,
            track_number: row.try_get("track_number")?,
            disc_number: row.try_get("disc_number")?,
            musicbrainz_id: musicbrainz_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            isrc: row.try_get("isrc")?,
            duration_secs: row.try_get("duration_secs")?,
            explicit: int_to_bool(explicit),
            artist_name: row.try_get("artist_name")?,
            artist_id: artist_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            media_file_id: media_file_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            file_path: row.try_get("file_path")?,
            file_size: row.try_get("file_size")?,
            audio_codec: row.try_get("audio_codec")?,
            bitrate: row.try_get("bitrate")?,
            audio_channels: row.try_get("audio_channels")?,
        })
    }
}

pub struct TrackRepository {
    pool: DbPool,
}

impl TrackRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get a track by ID
    #[cfg(feature = "postgres")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<TrackRecord>> {
        let record = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<TrackRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// List all tracks for an album
    #[cfg(feature = "postgres")]
    pub async fn list_by_album(&self, album_id: Uuid) -> Result<Vec<TrackRecord>> {
        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE album_id = $1
            ORDER BY disc_number, track_number
            "#,
        )
        .bind(album_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    #[cfg(feature = "sqlite")]
    pub async fn list_by_album(&self, album_id: Uuid) -> Result<Vec<TrackRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE album_id = ?1
            ORDER BY disc_number, track_number
            "#,
        )
        .bind(uuid_to_str(album_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List tracks with file status for an album
    #[cfg(feature = "postgres")]
    pub async fn list_with_status(&self, album_id: Uuid) -> Result<Vec<TrackWithStatus>> {
        let rows: Vec<TrackWithFileRow> = sqlx::query_as(
            r#"
            SELECT 
                t.id, t.album_id, t.library_id, t.title, t.track_number, t.disc_number,
                t.musicbrainz_id, t.isrc, t.duration_secs, t.explicit,
                t.artist_name, t.artist_id, t.media_file_id, t.created_at, t.updated_at,
                mf.path as file_path,
                mf.size as file_size,
                mf.audio_codec,
                mf.bitrate,
                mf.audio_channels
            FROM tracks t
            LEFT JOIN media_files mf ON t.media_file_id = mf.id
            WHERE t.album_id = $1
            ORDER BY t.disc_number, t.track_number
            "#,
        )
        .bind(album_id)
        .fetch_all(&self.pool)
        .await?;

        let tracks = rows
            .into_iter()
            .map(|row| TrackWithStatus {
                has_file: row.media_file_id.is_some(),
                file_path: row.file_path,
                file_size: row.file_size,
                audio_codec: row.audio_codec,
                bitrate: row.bitrate,
                audio_channels: row.audio_channels,
                track: TrackRecord {
                    id: row.id,
                    album_id: row.album_id,
                    library_id: row.library_id,
                    title: row.title,
                    track_number: row.track_number,
                    disc_number: row.disc_number,
                    musicbrainz_id: row.musicbrainz_id,
                    isrc: row.isrc,
                    duration_secs: row.duration_secs,
                    explicit: row.explicit,
                    artist_name: row.artist_name,
                    artist_id: row.artist_id,
                    media_file_id: row.media_file_id,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                },
            })
            .collect();

        Ok(tracks)
    }

    #[cfg(feature = "sqlite")]
    pub async fn list_with_status(&self, album_id: Uuid) -> Result<Vec<TrackWithStatus>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let rows: Vec<TrackWithFileRowSqlite> = sqlx::query_as(
            r#"
            SELECT 
                t.id, t.album_id, t.library_id, t.title, t.track_number, t.disc_number,
                t.musicbrainz_id, t.isrc, t.duration_secs, t.explicit,
                t.artist_name, t.artist_id, t.media_file_id, t.created_at, t.updated_at,
                mf.path as file_path,
                mf.size as file_size,
                mf.audio_codec,
                mf.bitrate,
                mf.audio_channels
            FROM tracks t
            LEFT JOIN media_files mf ON t.media_file_id = mf.id
            WHERE t.album_id = ?1
            ORDER BY t.disc_number, t.track_number
            "#,
        )
        .bind(uuid_to_str(album_id))
        .fetch_all(&self.pool)
        .await?;

        let tracks = rows
            .into_iter()
            .map(|row| TrackWithStatus {
                has_file: row.media_file_id.is_some(),
                file_path: row.file_path,
                file_size: row.file_size,
                audio_codec: row.audio_codec,
                bitrate: row.bitrate,
                audio_channels: row.audio_channels,
                track: TrackRecord {
                    id: row.id,
                    album_id: row.album_id,
                    library_id: row.library_id,
                    title: row.title,
                    track_number: row.track_number,
                    disc_number: row.disc_number,
                    musicbrainz_id: row.musicbrainz_id,
                    isrc: row.isrc,
                    duration_secs: row.duration_secs,
                    explicit: row.explicit,
                    artist_name: row.artist_name,
                    artist_id: row.artist_id,
                    media_file_id: row.media_file_id,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                },
            })
            .collect();

        Ok(tracks)
    }

    /// Count tracks in an album
    #[cfg(feature = "postgres")]
    pub async fn count_by_album(&self, album_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tracks WHERE album_id = $1")
            .bind(album_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    #[cfg(feature = "sqlite")]
    pub async fn count_by_album(&self, album_id: Uuid) -> Result<i64> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tracks WHERE album_id = ?1")
            .bind(uuid_to_str(album_id))
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// Count tracks with files in an album
    #[cfg(feature = "postgres")]
    pub async fn count_with_files(&self, album_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM tracks WHERE album_id = $1 AND media_file_id IS NOT NULL",
        )
        .bind(album_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    #[cfg(feature = "sqlite")]
    pub async fn count_with_files(&self, album_id: Uuid) -> Result<i64> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM tracks WHERE album_id = ?1 AND media_file_id IS NOT NULL",
        )
        .bind(uuid_to_str(album_id))
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    /// Create a new track
    #[cfg(feature = "postgres")]
    pub async fn create(&self, input: CreateTrack) -> Result<TrackRecord> {
        let record = sqlx::query_as::<_, TrackRecord>(
            r#"
            INSERT INTO tracks (
                album_id, library_id, title, track_number, disc_number,
                musicbrainz_id, isrc, duration_secs, explicit,
                artist_name, artist_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, album_id, library_id, title, track_number, disc_number,
                      musicbrainz_id, isrc, duration_secs, explicit,
                      artist_name, artist_id, media_file_id, created_at, updated_at
            "#,
        )
        .bind(input.album_id)
        .bind(input.library_id)
        .bind(&input.title)
        .bind(input.track_number)
        .bind(input.disc_number)
        .bind(input.musicbrainz_id)
        .bind(&input.isrc)
        .bind(input.duration_secs)
        .bind(input.explicit)
        .bind(&input.artist_name)
        .bind(input.artist_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateTrack) -> Result<TrackRecord> {
        use crate::db::sqlite_helpers::{bool_to_int, uuid_to_str};

        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO tracks (
                id, album_id, library_id, title, track_number, disc_number,
                musicbrainz_id, isrc, duration_secs, explicit,
                artist_name, artist_id, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.album_id))
        .bind(uuid_to_str(input.library_id))
        .bind(&input.title)
        .bind(input.track_number)
        .bind(input.disc_number)
        .bind(input.musicbrainz_id.map(uuid_to_str))
        .bind(&input.isrc)
        .bind(input.duration_secs)
        .bind(bool_to_int(input.explicit))
        .bind(&input.artist_name)
        .bind(input.artist_id.map(uuid_to_str))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve track after insert"))
    }

    /// Create multiple tracks at once
    pub async fn create_many(&self, tracks: Vec<CreateTrack>) -> Result<Vec<TrackRecord>> {
        let mut created = Vec::with_capacity(tracks.len());

        for input in tracks {
            let record = self.create(input).await?;
            created.push(record);
        }

        Ok(created)
    }

    /// Update a track
    #[cfg(feature = "postgres")]
    pub async fn update(&self, id: Uuid, input: UpdateTrack) -> Result<Option<TrackRecord>> {
        let record = sqlx::query_as::<_, TrackRecord>(
            r#"
            UPDATE tracks SET
                title = COALESCE($2, title),
                track_number = COALESCE($3, track_number),
                disc_number = COALESCE($4, disc_number),
                duration_secs = COALESCE($5, duration_secs),
                explicit = COALESCE($6, explicit),
                artist_name = COALESCE($7, artist_name),
                media_file_id = COALESCE($8, media_file_id),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, album_id, library_id, title, track_number, disc_number,
                      musicbrainz_id, isrc, duration_secs, explicit,
                      artist_name, artist_id, media_file_id, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&input.title)
        .bind(input.track_number)
        .bind(input.disc_number)
        .bind(input.duration_secs)
        .bind(input.explicit)
        .bind(&input.artist_name)
        .bind(input.media_file_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn update(&self, id: Uuid, input: UpdateTrack) -> Result<Option<TrackRecord>> {
        use crate::db::sqlite_helpers::{bool_to_int, uuid_to_str};

        let id_str = uuid_to_str(id);

        // SQLite doesn't support RETURNING, so we update then fetch
        sqlx::query(
            r#"
            UPDATE tracks SET
                title = COALESCE(?2, title),
                track_number = COALESCE(?3, track_number),
                disc_number = COALESCE(?4, disc_number),
                duration_secs = COALESCE(?5, duration_secs),
                explicit = COALESCE(?6, explicit),
                artist_name = COALESCE(?7, artist_name),
                media_file_id = COALESCE(?8, media_file_id),
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(&id_str)
        .bind(&input.title)
        .bind(input.track_number)
        .bind(input.disc_number)
        .bind(input.duration_secs)
        .bind(input.explicit.map(bool_to_int))
        .bind(&input.artist_name)
        .bind(input.media_file_id.map(uuid_to_str))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id).await
    }

    /// Link a media file to a track
    #[cfg(feature = "postgres")]
    pub async fn link_media_file(&self, track_id: Uuid, media_file_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tracks SET media_file_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(track_id)
        .bind(media_file_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn link_media_file(&self, track_id: Uuid, media_file_id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query(
            r#"
            UPDATE tracks SET media_file_id = ?2, updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(track_id))
        .bind(uuid_to_str(media_file_id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Unlink a media file from a track
    #[cfg(feature = "postgres")]
    pub async fn unlink_media_file(&self, track_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tracks SET media_file_id = NULL, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(track_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn unlink_media_file(&self, track_id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query(
            r#"
            UPDATE tracks SET media_file_id = NULL, updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(track_id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a track
    #[cfg(feature = "postgres")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM tracks WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let result = sqlx::query("DELETE FROM tracks WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all tracks for an album
    #[cfg(feature = "postgres")]
    pub async fn delete_by_album(&self, album_id: Uuid) -> Result<i64> {
        let result = sqlx::query("DELETE FROM tracks WHERE album_id = $1")
            .bind(album_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as i64)
    }

    #[cfg(feature = "sqlite")]
    pub async fn delete_by_album(&self, album_id: Uuid) -> Result<i64> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let result = sqlx::query("DELETE FROM tracks WHERE album_id = ?1")
            .bind(uuid_to_str(album_id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Find a track by MusicBrainz ID
    #[cfg(feature = "postgres")]
    pub async fn get_by_musicbrainz_id(
        &self,
        album_id: Uuid,
        mbid: Uuid,
    ) -> Result<Option<TrackRecord>> {
        let record = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE album_id = $1 AND musicbrainz_id = $2
            "#,
        )
        .bind(album_id)
        .bind(mbid)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_by_musicbrainz_id(
        &self,
        album_id: Uuid,
        mbid: Uuid,
    ) -> Result<Option<TrackRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE album_id = ?1 AND musicbrainz_id = ?2
            "#,
        )
        .bind(uuid_to_str(album_id))
        .bind(uuid_to_str(mbid))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Find tracks without files (missing tracks)
    #[cfg(feature = "postgres")]
    pub async fn list_missing(&self, album_id: Uuid) -> Result<Vec<TrackRecord>> {
        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE album_id = $1 AND media_file_id IS NULL
            ORDER BY disc_number, track_number
            "#,
        )
        .bind(album_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    #[cfg(feature = "sqlite")]
    pub async fn list_missing(&self, album_id: Uuid) -> Result<Vec<TrackRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE album_id = ?1 AND media_file_id IS NULL
            ORDER BY disc_number, track_number
            "#,
        )
        .bind(uuid_to_str(album_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Find track by album and track/disc number
    #[cfg(feature = "postgres")]
    pub async fn get_by_number(
        &self,
        album_id: Uuid,
        disc_number: i32,
        track_number: i32,
    ) -> Result<Option<TrackRecord>> {
        let record = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE album_id = $1 AND disc_number = $2 AND track_number = $3
            "#,
        )
        .bind(album_id)
        .bind(disc_number)
        .bind(track_number)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_by_number(
        &self,
        album_id: Uuid,
        disc_number: i32,
        track_number: i32,
    ) -> Result<Option<TrackRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE album_id = ?1 AND disc_number = ?2 AND track_number = ?3
            "#,
        )
        .bind(uuid_to_str(album_id))
        .bind(disc_number)
        .bind(track_number)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Search tracks by title within an album
    #[cfg(feature = "postgres")]
    pub async fn search_by_title(&self, album_id: Uuid, query: &str) -> Result<Vec<TrackRecord>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE album_id = $1 AND LOWER(title) LIKE $2
            ORDER BY disc_number, track_number
            "#,
        )
        .bind(album_id)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    #[cfg(feature = "sqlite")]
    pub async fn search_by_title(&self, album_id: Uuid, query: &str) -> Result<Vec<TrackRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let pattern = format!("%{}%", query.to_lowercase());
        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE album_id = ?1 AND LOWER(title) LIKE ?2
            ORDER BY disc_number, track_number
            "#,
        )
        .bind(uuid_to_str(album_id))
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List tracks without files across a library (wanted/missing tracks)
    #[cfg(feature = "postgres")]
    pub async fn list_wanted_by_library(&self, library_id: Uuid) -> Result<Vec<TrackRecord>> {
        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE library_id = $1 AND media_file_id IS NULL
            ORDER BY album_id, disc_number, track_number
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    #[cfg(feature = "sqlite")]
    pub async fn list_wanted_by_library(&self, library_id: Uuid) -> Result<Vec<TrackRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE library_id = ?1 AND media_file_id IS NULL
            ORDER BY album_id, disc_number, track_number
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List tracks in a library with pagination and filtering
    ///
    /// Returns (records, total_count)
    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "postgres")]
    pub async fn list_by_library_paginated(
        &self,
        library_id: Uuid,
        offset: i64,
        limit: i64,
        title_filter: Option<&str>,
        has_file_filter: Option<bool>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<TrackRecord>, i64)> {
        let mut conditions = vec!["library_id = $1".to_string()];
        let mut param_idx = 2;

        if title_filter.is_some() {
            conditions.push(format!("LOWER(title) LIKE ${}", param_idx));
            param_idx += 1;
        }
        let _ = param_idx; // Suppress unused variable warning
        if has_file_filter.is_some() {
            conditions.push(format!(
                "media_file_id IS {} NULL",
                if has_file_filter.unwrap() {
                    "NOT"
                } else {
                    ""
                }
            ));
            // No param needed, we use IS NULL / IS NOT NULL
        }

        let where_clause = conditions.join(" AND ");

        let valid_sort_columns = [
            "title",
            "track_number",
            "disc_number",
            "created_at",
            "artist_name",
            "duration_secs",
        ];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "title"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        let order_clause = format!("ORDER BY {} {} NULLS LAST", sort_col, order_dir);

        let count_query = format!("SELECT COUNT(*) FROM tracks WHERE {}", where_clause);
        let data_query = format!(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_clause, limit, offset
        );

        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(library_id);
        if let Some(title) = title_filter {
            count_builder = count_builder.bind(format!("%{}%", title.to_lowercase()));
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let mut data_builder = sqlx::query_as::<_, TrackRecord>(&data_query).bind(library_id);
        if let Some(title) = title_filter {
            data_builder = data_builder.bind(format!("%{}%", title.to_lowercase()));
        }

        let records: Vec<TrackRecord> = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "sqlite")]
    pub async fn list_by_library_paginated(
        &self,
        library_id: Uuid,
        offset: i64,
        limit: i64,
        title_filter: Option<&str>,
        has_file_filter: Option<bool>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<TrackRecord>, i64)> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let mut conditions = vec!["library_id = ?1".to_string()];
        let mut param_idx = 2;

        if title_filter.is_some() {
            conditions.push(format!("LOWER(title) LIKE ?{}", param_idx));
            param_idx += 1;
        }
        let _ = param_idx; // Suppress unused variable warning
        if has_file_filter.is_some() {
            conditions.push(format!(
                "media_file_id IS {} NULL",
                if has_file_filter.unwrap() {
                    "NOT"
                } else {
                    ""
                }
            ));
            // No param needed, we use IS NULL / IS NOT NULL
        }

        let where_clause = conditions.join(" AND ");

        let valid_sort_columns = [
            "title",
            "track_number",
            "disc_number",
            "created_at",
            "artist_name",
            "duration_secs",
        ];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "title"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        // SQLite doesn't support NULLS LAST directly, use CASE expression
        let order_clause = format!(
            "ORDER BY CASE WHEN {} IS NULL THEN 1 ELSE 0 END, {} {}",
            sort_col, sort_col, order_dir
        );

        let count_query = format!("SELECT COUNT(*) FROM tracks WHERE {}", where_clause);
        let data_query = format!(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, created_at, updated_at
            FROM tracks
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_clause, limit, offset
        );

        let library_id_str = uuid_to_str(library_id);

        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(&library_id_str);
        if let Some(title) = title_filter {
            count_builder = count_builder.bind(format!("%{}%", title.to_lowercase()));
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let mut data_builder =
            sqlx::query_as::<_, TrackRecord>(&data_query).bind(&library_id_str);
        if let Some(title) = title_filter {
            data_builder = data_builder.bind(format!("%{}%", title.to_lowercase()));
        }

        let records: Vec<TrackRecord> = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }
}
