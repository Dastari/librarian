//! Track database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Track record from database
#[derive(Debug, Clone, sqlx::FromRow)]
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
    // Download status
    pub status: String,
    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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
}

pub struct TrackRepository {
    pool: PgPool,
}

impl TrackRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a track by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<TrackRecord>> {
        let record = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, status, created_at, updated_at
            FROM tracks
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// List all tracks for an album
    pub async fn list_by_album(&self, album_id: Uuid) -> Result<Vec<TrackRecord>> {
        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, status, created_at, updated_at
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

    /// List tracks with file status for an album
    pub async fn list_with_status(&self, album_id: Uuid) -> Result<Vec<TrackWithStatus>> {
        // Use a flat row struct for the joined query
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
            status: String,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
            file_path: Option<String>,
            file_size: Option<i64>,
        }

        let rows: Vec<TrackWithFileRow> = sqlx::query_as(
            r#"
            SELECT 
                t.id, t.album_id, t.library_id, t.title, t.track_number, t.disc_number,
                t.musicbrainz_id, t.isrc, t.duration_secs, t.explicit,
                t.artist_name, t.artist_id, t.media_file_id, t.status, t.created_at, t.updated_at,
                mf.path as file_path,
                mf.size as file_size
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
                    status: row.status,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                },
            })
            .collect();

        Ok(tracks)
    }

    /// Count tracks in an album
    pub async fn count_by_album(&self, album_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tracks WHERE album_id = $1")
            .bind(album_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// Count tracks with files in an album
    pub async fn count_with_files(&self, album_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM tracks WHERE album_id = $1 AND media_file_id IS NOT NULL",
        )
        .bind(album_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    /// Create a new track
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
                      artist_name, artist_id, media_file_id, status, created_at, updated_at
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
                      artist_name, artist_id, media_file_id, status, created_at, updated_at
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

    /// Link a media file to a track
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

    /// Unlink a media file from a track
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

    /// Delete a track
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM tracks WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all tracks for an album
    pub async fn delete_by_album(&self, album_id: Uuid) -> Result<i64> {
        let result = sqlx::query("DELETE FROM tracks WHERE album_id = $1")
            .bind(album_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Find a track by MusicBrainz ID
    pub async fn get_by_musicbrainz_id(
        &self,
        album_id: Uuid,
        mbid: Uuid,
    ) -> Result<Option<TrackRecord>> {
        let record = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, status, created_at, updated_at
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

    /// Find tracks without files (missing tracks)
    pub async fn list_missing(&self, album_id: Uuid) -> Result<Vec<TrackRecord>> {
        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, status, created_at, updated_at
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

    /// Find track by album and track/disc number
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
                   artist_name, artist_id, media_file_id, status, created_at, updated_at
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

    /// Search tracks by title within an album
    pub async fn search_by_title(&self, album_id: Uuid, query: &str) -> Result<Vec<TrackRecord>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, status, created_at, updated_at
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

    /// Update a track's download status
    pub async fn update_status(&self, id: Uuid, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tracks SET status = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List tracks by status across a library
    pub async fn list_wanted_by_library(&self, library_id: Uuid) -> Result<Vec<TrackRecord>> {
        let records = sqlx::query_as::<_, TrackRecord>(
            r#"
            SELECT id, album_id, library_id, title, track_number, disc_number,
                   musicbrainz_id, isrc, duration_secs, explicit,
                   artist_name, artist_id, media_file_id, status, created_at, updated_at
            FROM tracks
            WHERE library_id = $1 AND status IN ('missing', 'wanted')
            ORDER BY album_id, disc_number, track_number
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}
