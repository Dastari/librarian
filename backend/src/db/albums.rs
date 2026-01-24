//! Album database repository

use anyhow::Result;
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// Album record from database
#[derive(Debug, Clone)]
pub struct AlbumRecord {
    pub id: Uuid,
    pub artist_id: Uuid,
    pub library_id: Uuid,
    pub user_id: Uuid,
    // Basic info
    pub name: String,
    pub sort_name: Option<String>,
    pub year: Option<i32>,
    // External IDs
    pub musicbrainz_id: Option<Uuid>,
    // Metadata
    pub album_type: Option<String>,
    pub genres: Vec<String>,
    pub label: Option<String>,
    pub country: Option<String>,
    pub release_date: Option<chrono::NaiveDate>,
    // Artwork
    pub cover_url: Option<String>,
    // Stats
    pub track_count: Option<i32>,
    pub disc_count: Option<i32>,
    pub total_duration_secs: Option<i32>,
    // File status
    pub has_files: bool,
    pub size_bytes: Option<i64>,
    pub path: Option<String>,
    /// Calculated: count of tracks with status = 'downloaded'
    pub downloaded_track_count: Option<i32>,
    /// When true, auto-hunt searches for individual tracks instead of complete album releases
    /// Set after a partial download completes
    pub hunt_individual_items: bool,
    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for AlbumRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use crate::db::sqlite_helpers::{int_to_bool, json_to_vec, str_to_datetime, str_to_uuid};
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let artist_id_str: String = row.try_get("artist_id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let musicbrainz_id_str: Option<String> = row.try_get("musicbrainz_id")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;

        // JSON arrays stored as TEXT
        let genres_json: String = row.try_get("genres")?;

        // Boolean stored as INTEGER
        let has_files: i32 = row.try_get("has_files")?;

        // NaiveDate stored as TEXT (YYYY-MM-DD)
        let release_date_str: Option<String> = row.try_get("release_date")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            artist_id: str_to_uuid(&artist_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_id: str_to_uuid(&library_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            name: row.try_get("name")?,
            sort_name: row.try_get("sort_name")?,
            year: row.try_get("year")?,
            musicbrainz_id: musicbrainz_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            album_type: row.try_get("album_type")?,
            genres: json_to_vec(&genres_json),
            label: row.try_get("label")?,
            country: row.try_get("country")?,
            release_date: release_date_str
                .map(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            cover_url: row.try_get("cover_url")?,
            track_count: row.try_get("track_count")?,
            disc_count: row.try_get("disc_count")?,
            total_duration_secs: row.try_get("total_duration_secs")?,
            has_files: int_to_bool(has_files),
            size_bytes: row.try_get("size_bytes")?,
            path: row.try_get("path")?,
            downloaded_track_count: row.try_get("downloaded_track_count")?,
            hunt_individual_items: {
                let v: i32 = row.try_get("hunt_individual_items").unwrap_or(0);
                int_to_bool(v)
            },
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Artist record from database
#[derive(Debug, Clone)]
pub struct ArtistRecord {
    pub id: Uuid,
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
    pub musicbrainz_id: Option<Uuid>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for ArtistRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use crate::db::sqlite_helpers::str_to_uuid;
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let musicbrainz_id_str: Option<String> = row.try_get("musicbrainz_id")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_id: str_to_uuid(&library_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            name: row.try_get("name")?,
            sort_name: row.try_get("sort_name")?,
            musicbrainz_id: musicbrainz_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Input for creating an album
#[derive(Debug)]
pub struct CreateAlbum {
    pub artist_id: Uuid,
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
    pub year: Option<i32>,
    pub musicbrainz_id: Option<Uuid>,
    pub album_type: Option<String>,
    pub genres: Vec<String>,
    pub label: Option<String>,
    pub country: Option<String>,
    pub release_date: Option<chrono::NaiveDate>,
    pub cover_url: Option<String>,
    pub track_count: Option<i32>,
    pub disc_count: Option<i32>,
}

pub struct AlbumRepository {
    pool: DbPool,
}

impl AlbumRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get an album by ID

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<AlbumRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, AlbumRecord>(
            r#"
            SELECT a.id, a.artist_id, a.library_id, a.user_id, a.name, a.sort_name, a.year,
                   a.musicbrainz_id, a.album_type, a.genres, a.label, a.country, a.release_date,
                   a.cover_url, a.track_count, a.disc_count, a.total_duration_secs,
                   a.has_files, a.size_bytes, a.path,
                   CAST((SELECT COUNT(*) FROM tracks t WHERE t.album_id = a.id AND t.media_file_id IS NOT NULL) AS INTEGER) as downloaded_track_count,
                   a.created_at, a.updated_at
            FROM albums a
            WHERE a.id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get an artist by ID

    #[cfg(feature = "sqlite")]
    pub async fn get_artist_by_id(&self, id: Uuid) -> Result<Option<ArtistRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, ArtistRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, musicbrainz_id
            FROM artists
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update album has_files status

    #[cfg(feature = "sqlite")]
    pub async fn update_has_files(&self, id: Uuid, has_files: bool) -> Result<()> {
        use crate::db::sqlite_helpers::{bool_to_int, uuid_to_str};

        sqlx::query("UPDATE albums SET has_files = ?2, updated_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(id))
            .bind(bool_to_int(has_files))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update album path

    #[cfg(feature = "sqlite")]
    pub async fn update_path(&self, id: Uuid, path: &str) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query("UPDATE albums SET path = ?2, updated_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(id))
            .bind(path)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get album by MusicBrainz ID within a library
    #[cfg(feature = "sqlite")]
    pub async fn get_by_musicbrainz_id(
        &self,
        library_id: Uuid,
        musicbrainz_id: Uuid,
    ) -> Result<Option<AlbumRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, AlbumRecord>(
            r#"
            SELECT a.id, a.artist_id, a.library_id, a.user_id, a.name, a.sort_name, a.year,
                   a.musicbrainz_id, a.album_type, a.genres, a.label, a.country, a.release_date,
                   a.cover_url, a.track_count, a.disc_count, a.total_duration_secs,
                   a.has_files, a.size_bytes, a.path,
                   CAST((SELECT COUNT(*) FROM tracks t WHERE t.album_id = a.id AND t.media_file_id IS NOT NULL) AS INTEGER) as downloaded_track_count,
                   a.created_at, a.updated_at
            FROM albums a
            WHERE a.library_id = ?1 AND a.musicbrainz_id = ?2
            "#,
        )
        .bind(uuid_to_str(library_id))
        .bind(uuid_to_str(musicbrainz_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Find or create an artist in the library
    #[cfg(feature = "sqlite")]
    pub async fn find_or_create_artist(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        name: &str,
        sort_name: Option<&str>,
        musicbrainz_id: Option<Uuid>,
    ) -> Result<ArtistRecord> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let library_id_str = uuid_to_str(library_id);

        // Try to find existing artist first by MusicBrainz ID
        if let Some(mbid) = musicbrainz_id {
            if let Some(artist) = sqlx::query_as::<_, ArtistRecord>(
                r#"
                SELECT id, library_id, user_id, name, sort_name, musicbrainz_id
                FROM artists
                WHERE library_id = ?1 AND musicbrainz_id = ?2
                "#,
            )
            .bind(&library_id_str)
            .bind(uuid_to_str(mbid))
            .fetch_optional(&self.pool)
            .await?
            {
                return Ok(artist);
            }
        }

        // Try to find by name if no MBID match
        if let Some(artist) = sqlx::query_as::<_, ArtistRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, musicbrainz_id
            FROM artists
            WHERE library_id = ?1 AND LOWER(name) = LOWER(?2)
            "#,
        )
        .bind(&library_id_str)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?
        {
            return Ok(artist);
        }

        // Create new artist
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO artists (id, library_id, user_id, name, sort_name, musicbrainz_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&id_str)
        .bind(&library_id_str)
        .bind(uuid_to_str(user_id))
        .bind(name)
        .bind(sort_name)
        .bind(musicbrainz_id.map(uuid_to_str))
        .execute(&self.pool)
        .await?;

        // Fetch the created artist
        self.get_artist_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve artist after insert"))
    }

    /// Create a new album

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateAlbum) -> Result<AlbumRecord> {
        use crate::db::sqlite_helpers::{uuid_to_str, vec_to_json};

        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO albums (
                id, artist_id, library_id, user_id, name, sort_name, year,
                musicbrainz_id, album_type, genres, label, country, release_date,
                cover_url, track_count, disc_count, has_files,
                created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, 0,
                    datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.artist_id))
        .bind(uuid_to_str(input.library_id))
        .bind(uuid_to_str(input.user_id))
        .bind(&input.name)
        .bind(&input.sort_name)
        .bind(input.year)
        .bind(input.musicbrainz_id.map(uuid_to_str))
        .bind(&input.album_type)
        .bind(vec_to_json(&input.genres))
        .bind(&input.label)
        .bind(&input.country)
        .bind(input.release_date.map(|d| d.format("%Y-%m-%d").to_string()))
        .bind(&input.cover_url)
        .bind(input.track_count)
        .bind(input.disc_count)
        .execute(&self.pool)
        .await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve album after insert"))
    }

    /// List albums by library

    #[cfg(feature = "sqlite")]
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<AlbumRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, AlbumRecord>(
            r#"
            SELECT a.id, a.artist_id, a.library_id, a.user_id, a.name, a.sort_name, a.year,
                   a.musicbrainz_id, a.album_type, a.genres, a.label, a.country, a.release_date,
                   a.cover_url, a.track_count, a.disc_count, a.total_duration_secs,
                   a.has_files, a.size_bytes, a.path,
                   CAST((SELECT COUNT(*) FROM tracks t WHERE t.album_id = a.id AND t.media_file_id IS NOT NULL) AS INTEGER) as downloaded_track_count,
                   a.created_at, a.updated_at
            FROM albums a
            WHERE a.library_id = ?1
            ORDER BY a.name ASC
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List albums in a library with pagination and filtering
    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "sqlite")]
    pub async fn list_by_library_paginated(
        &self,
        library_id: Uuid,
        offset: i64,
        limit: i64,
        name_filter: Option<&str>,
        year_filter: Option<i32>,
        has_files_filter: Option<bool>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<AlbumRecord>, i64)> {
        use crate::db::sqlite_helpers::{bool_to_int, uuid_to_str};

        let mut conditions = vec!["library_id = ?1".to_string()];
        let mut param_idx = 2;

        if name_filter.is_some() {
            conditions.push(format!("LOWER(name) LIKE ?{}", param_idx));
            param_idx += 1;
        }
        if year_filter.is_some() {
            conditions.push(format!("year = ?{}", param_idx));
            param_idx += 1;
        }
        if has_files_filter.is_some() {
            conditions.push(format!("has_files = ?{}", param_idx));
        }

        let where_clause = conditions.join(" AND ");

        let valid_sort_columns = ["name", "sort_name", "year", "created_at", "artist_id"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "name"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        // SQLite doesn't support NULLS LAST directly, use CASE expression
        let order_clause = format!(
            "ORDER BY CASE WHEN {} IS NULL THEN 1 ELSE 0 END, {} {}",
            sort_col, sort_col, order_dir
        );

        let count_query = format!("SELECT COUNT(*) FROM albums WHERE {}", where_clause);
        let data_query = format!(
            r#"
            SELECT a.id, a.artist_id, a.library_id, a.user_id, a.name, a.sort_name, a.year,
                   a.musicbrainz_id, a.album_type, a.genres, a.label, a.country, a.release_date,
                   a.cover_url, a.track_count, a.disc_count, a.total_duration_secs,
                   a.has_files, a.size_bytes, a.path,
                   CAST((SELECT COUNT(*) FROM tracks t WHERE t.album_id = a.id AND t.media_file_id IS NOT NULL) AS INTEGER) as downloaded_track_count,
                   a.created_at, a.updated_at
            FROM albums a
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause
                .replace("library_id", "a.library_id")
                .replace("name", "a.name")
                .replace("year", "a.year")
                .replace("has_files", "a.has_files"),
            order_clause
                .replace(sort_col, &format!("a.{}", sort_col)),
            limit,
            offset
        );

        let library_id_str = uuid_to_str(library_id);

        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(&library_id_str);
        if let Some(name) = name_filter {
            count_builder = count_builder.bind(format!("%{}%", name.to_lowercase()));
        }
        if let Some(year) = year_filter {
            count_builder = count_builder.bind(year);
        }
        if let Some(has_files) = has_files_filter {
            count_builder = count_builder.bind(bool_to_int(has_files));
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let mut data_builder =
            sqlx::query_as::<_, AlbumRecord>(&data_query).bind(&library_id_str);
        if let Some(name) = name_filter {
            data_builder = data_builder.bind(format!("%{}%", name.to_lowercase()));
        }
        if let Some(year) = year_filter {
            data_builder = data_builder.bind(year);
        }
        if let Some(has_files) = has_files_filter {
            data_builder = data_builder.bind(bool_to_int(has_files));
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// List artists by library

    #[cfg(feature = "sqlite")]
    pub async fn list_artists_by_library(&self, library_id: Uuid) -> Result<Vec<ArtistRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, ArtistRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, musicbrainz_id
            FROM artists
            WHERE library_id = ?1
            ORDER BY name ASC
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List artists in a library with pagination and filtering
    #[cfg(feature = "sqlite")]
    pub async fn list_artists_by_library_paginated(
        &self,
        library_id: Uuid,
        offset: i64,
        limit: i64,
        name_filter: Option<&str>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<ArtistRecord>, i64)> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let mut conditions = vec!["library_id = ?1".to_string()];

        if name_filter.is_some() {
            conditions.push("LOWER(name) LIKE ?2".to_string());
        }

        let where_clause = conditions.join(" AND ");

        let valid_sort_columns = ["name", "sort_name"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "name"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        // SQLite doesn't support NULLS LAST directly
        let order_clause = format!(
            "ORDER BY CASE WHEN {} IS NULL THEN 1 ELSE 0 END, {} {}",
            sort_col, sort_col, order_dir
        );

        let count_query = format!("SELECT COUNT(*) FROM artists WHERE {}", where_clause);
        let data_query = format!(
            r#"
            SELECT id, library_id, user_id, name, sort_name, musicbrainz_id
            FROM artists
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_clause, limit, offset
        );

        let library_id_str = uuid_to_str(library_id);

        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(&library_id_str);
        if let Some(name) = name_filter {
            count_builder = count_builder.bind(format!("%{}%", name.to_lowercase()));
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let mut data_builder =
            sqlx::query_as::<_, ArtistRecord>(&data_query).bind(&library_id_str);
        if let Some(name) = name_filter {
            data_builder = data_builder.bind(format!("%{}%", name.to_lowercase()));
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// List albums that need files (for auto-hunt)
    ///
    /// Returns albums in the library that don't have complete files.
    /// Album status is derived from track media_file_ids - an album needs files
    /// if it has no files or has tracks without media files.
    #[cfg(feature = "sqlite")]
    pub async fn list_needing_files(
        &self,
        library_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AlbumRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, AlbumRecord>(
            r#"
            SELECT a.id, a.artist_id, a.library_id, a.user_id, a.name, a.sort_name, a.year,
                   a.musicbrainz_id, a.album_type, a.genres, a.label, a.country, a.release_date,
                   a.cover_url, a.track_count, a.disc_count, a.total_duration_secs,
                   a.has_files, a.size_bytes, a.path,
                   CAST((SELECT COUNT(*) FROM tracks t WHERE t.album_id = a.id AND t.media_file_id IS NOT NULL) AS INTEGER) as downloaded_track_count,
                   a.created_at, a.updated_at
            FROM albums a
            WHERE a.library_id = ?1 
              AND (
                a.has_files = 0
                OR EXISTS (SELECT 1 FROM tracks t WHERE t.album_id = a.id AND t.media_file_id IS NULL)
              )
            ORDER BY a.created_at DESC
            LIMIT ?2
            "#,
        )
        .bind(uuid_to_str(library_id))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Delete an album and its associated data
    ///
    /// This will also delete:
    /// - All tracks for this album
    /// - All media files linked to this album's tracks
    /// - Torrent links to this album
    /// - Watch progress for this album's tracks

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let id_str = uuid_to_str(id);

        // Start a transaction to ensure all deletions are atomic
        let mut tx = self.pool.begin().await?;

        // Delete watch progress for tracks in this album
        sqlx::query(
            r#"
            DELETE FROM watch_progress 
            WHERE track_id IN (SELECT id FROM tracks WHERE album_id = ?1)
            "#,
        )
        .bind(&id_str)
        .execute(&mut *tx)
        .await?;

        // Delete media files for tracks in this album
        sqlx::query(
            r#"
            DELETE FROM media_files 
            WHERE track_id IN (SELECT id FROM tracks WHERE album_id = ?1)
            "#,
        )
        .bind(&id_str)
        .execute(&mut *tx)
        .await?;

        // Delete tracks for this album (pending_file_matches handles cleanup via ON DELETE CASCADE)
        sqlx::query("DELETE FROM tracks WHERE album_id = ?1")
            .bind(&id_str)
            .execute(&mut *tx)
            .await?;

        // Delete the album itself
        let result = sqlx::query("DELETE FROM albums WHERE id = ?1")
            .bind(&id_str)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(result.rows_affected() > 0)
    }

    /// Set the hunt_individual_items flag for an album
    ///
    /// When true, auto-hunt will search for individual tracks instead of complete album releases.
    /// This is set after a partial album download completes.
    #[cfg(feature = "sqlite")]
    pub async fn set_hunt_individual_items(&self, id: Uuid, value: bool) -> Result<()> {
        use crate::db::sqlite_helpers::{bool_to_int, uuid_to_str};

        sqlx::query(
            r#"
            UPDATE albums SET 
                hunt_individual_items = ?2,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(bool_to_int(value))
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
