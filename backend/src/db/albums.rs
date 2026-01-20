//! Album database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Album record from database
#[derive(Debug, Clone, sqlx::FromRow)]
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
    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Artist record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArtistRecord {
    pub id: Uuid,
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
    pub musicbrainz_id: Option<Uuid>,
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
    pool: PgPool,
}

impl AlbumRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get an album by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<AlbumRecord>> {
        let record = sqlx::query_as::<_, AlbumRecord>(
            r#"
            SELECT id, artist_id, library_id, user_id, name, sort_name, year,
                   musicbrainz_id, album_type, genres, label, country, release_date,
                   cover_url, track_count, disc_count, total_duration_secs,
                   has_files, size_bytes, path, created_at, updated_at
            FROM albums
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get an artist by ID
    pub async fn get_artist_by_id(&self, id: Uuid) -> Result<Option<ArtistRecord>> {
        let record = sqlx::query_as::<_, ArtistRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, musicbrainz_id
            FROM artists
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update album has_files status
    pub async fn update_has_files(&self, id: Uuid, has_files: bool) -> Result<()> {
        sqlx::query("UPDATE albums SET has_files = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(has_files)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update album path
    pub async fn update_path(&self, id: Uuid, path: &str) -> Result<()> {
        sqlx::query("UPDATE albums SET path = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(path)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get album by MusicBrainz ID within a library
    pub async fn get_by_musicbrainz_id(
        &self,
        library_id: Uuid,
        musicbrainz_id: Uuid,
    ) -> Result<Option<AlbumRecord>> {
        let record = sqlx::query_as::<_, AlbumRecord>(
            r#"
            SELECT id, artist_id, library_id, user_id, name, sort_name, year,
                   musicbrainz_id, album_type, genres, label, country, release_date,
                   cover_url, track_count, disc_count, total_duration_secs,
                   has_files, size_bytes, path, created_at, updated_at
            FROM albums
            WHERE library_id = $1 AND musicbrainz_id = $2
            "#,
        )
        .bind(library_id)
        .bind(musicbrainz_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Find or create an artist in the library
    pub async fn find_or_create_artist(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        name: &str,
        sort_name: Option<&str>,
        musicbrainz_id: Option<Uuid>,
    ) -> Result<ArtistRecord> {
        // Try to find existing artist first by MusicBrainz ID
        if let Some(mbid) = musicbrainz_id {
            if let Some(artist) = sqlx::query_as::<_, ArtistRecord>(
                r#"
                SELECT id, library_id, user_id, name, sort_name, musicbrainz_id
                FROM artists
                WHERE library_id = $1 AND musicbrainz_id = $2
                "#,
            )
            .bind(library_id)
            .bind(mbid)
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
            WHERE library_id = $1 AND LOWER(name) = LOWER($2)
            "#,
        )
        .bind(library_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?
        {
            return Ok(artist);
        }

        // Create new artist
        let artist = sqlx::query_as::<_, ArtistRecord>(
            r#"
            INSERT INTO artists (library_id, user_id, name, sort_name, musicbrainz_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, library_id, user_id, name, sort_name, musicbrainz_id
            "#,
        )
        .bind(library_id)
        .bind(user_id)
        .bind(name)
        .bind(sort_name)
        .bind(musicbrainz_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(artist)
    }

    /// Create a new album
    pub async fn create(&self, input: CreateAlbum) -> Result<AlbumRecord> {
        let album = sqlx::query_as::<_, AlbumRecord>(
            r#"
            INSERT INTO albums (
                artist_id, library_id, user_id, name, sort_name, year,
                musicbrainz_id, album_type, genres, label, country, release_date,
                cover_url, track_count, disc_count
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING id, artist_id, library_id, user_id, name, sort_name, year,
                      musicbrainz_id, album_type, genres, label, country, release_date,
                      cover_url, track_count, disc_count, total_duration_secs,
                      has_files, size_bytes, path, created_at, updated_at
            "#,
        )
        .bind(input.artist_id)
        .bind(input.library_id)
        .bind(input.user_id)
        .bind(&input.name)
        .bind(&input.sort_name)
        .bind(input.year)
        .bind(input.musicbrainz_id)
        .bind(&input.album_type)
        .bind(&input.genres)
        .bind(&input.label)
        .bind(&input.country)
        .bind(input.release_date)
        .bind(&input.cover_url)
        .bind(input.track_count)
        .bind(input.disc_count)
        .fetch_one(&self.pool)
        .await?;

        Ok(album)
    }

    /// List albums by library
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<AlbumRecord>> {
        let records = sqlx::query_as::<_, AlbumRecord>(
            r#"
            SELECT id, artist_id, library_id, user_id, name, sort_name, year,
                   musicbrainz_id, album_type, genres, label, country, release_date,
                   cover_url, track_count, disc_count, total_duration_secs,
                   has_files, size_bytes, path, created_at, updated_at
            FROM albums
            WHERE library_id = $1
            ORDER BY name ASC
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List albums in a library with pagination and filtering
    #[allow(clippy::too_many_arguments)]
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
        let mut conditions = vec!["library_id = $1".to_string()];
        let mut param_idx = 2;

        if name_filter.is_some() {
            conditions.push(format!("LOWER(name) LIKE ${}", param_idx));
            param_idx += 1;
        }
        if year_filter.is_some() {
            conditions.push(format!("year = ${}", param_idx));
            param_idx += 1;
        }
        if has_files_filter.is_some() {
            conditions.push(format!("has_files = ${}", param_idx));
        }

        let where_clause = conditions.join(" AND ");

        let valid_sort_columns = ["name", "sort_name", "year", "created_at", "artist_id"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "name"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        let order_clause = format!("ORDER BY {} {} NULLS LAST", sort_col, order_dir);

        let count_query = format!("SELECT COUNT(*) FROM albums WHERE {}", where_clause);
        let data_query = format!(
            r#"
            SELECT id, artist_id, library_id, user_id, name, sort_name, year,
                   musicbrainz_id, album_type, genres, label, country, release_date,
                   cover_url, track_count, disc_count, total_duration_secs,
                   has_files, size_bytes, path, created_at, updated_at
            FROM albums
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_clause, limit, offset
        );

        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(library_id);
        if let Some(name) = name_filter {
            count_builder = count_builder.bind(format!("%{}%", name.to_lowercase()));
        }
        if let Some(year) = year_filter {
            count_builder = count_builder.bind(year);
        }
        if let Some(has_files) = has_files_filter {
            count_builder = count_builder.bind(has_files);
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let mut data_builder = sqlx::query_as::<_, AlbumRecord>(&data_query).bind(library_id);
        if let Some(name) = name_filter {
            data_builder = data_builder.bind(format!("%{}%", name.to_lowercase()));
        }
        if let Some(year) = year_filter {
            data_builder = data_builder.bind(year);
        }
        if let Some(has_files) = has_files_filter {
            data_builder = data_builder.bind(has_files);
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// List artists by library
    pub async fn list_artists_by_library(&self, library_id: Uuid) -> Result<Vec<ArtistRecord>> {
        let records = sqlx::query_as::<_, ArtistRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, musicbrainz_id
            FROM artists
            WHERE library_id = $1
            ORDER BY name ASC
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List artists in a library with pagination and filtering
    pub async fn list_artists_by_library_paginated(
        &self,
        library_id: Uuid,
        offset: i64,
        limit: i64,
        name_filter: Option<&str>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<ArtistRecord>, i64)> {
        let mut conditions = vec!["library_id = $1".to_string()];

        if name_filter.is_some() {
            conditions.push("LOWER(name) LIKE $2".to_string());
        }

        let where_clause = conditions.join(" AND ");

        let valid_sort_columns = ["name", "sort_name"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "name"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        let order_clause = format!("ORDER BY {} {} NULLS LAST", sort_col, order_dir);

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

        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(library_id);
        if let Some(name) = name_filter {
            count_builder = count_builder.bind(format!("%{}%", name.to_lowercase()));
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let mut data_builder = sqlx::query_as::<_, ArtistRecord>(&data_query).bind(library_id);
        if let Some(name) = name_filter {
            data_builder = data_builder.bind(format!("%{}%", name.to_lowercase()));
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// List albums that need files (for auto-hunt)
    /// 
    /// Returns albums in the library that don't have complete files.
    /// For now, this returns albums where has_files = false.
    pub async fn list_needing_files(&self, library_id: Uuid, limit: i64) -> Result<Vec<AlbumRecord>> {
        let records = sqlx::query_as::<_, AlbumRecord>(
            r#"
            SELECT id, artist_id, library_id, user_id, name, sort_name, year,
                   musicbrainz_id, album_type, genres, label, country, release_date,
                   cover_url, track_count, disc_count, total_duration_secs,
                   has_files, size_bytes, path, created_at, updated_at
            FROM albums
            WHERE library_id = $1 
              AND (download_status IN ('missing', 'wanted', 'suboptimal') OR (download_status IS NULL AND has_files = false))
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(library_id)
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
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        // Start a transaction to ensure all deletions are atomic
        let mut tx = self.pool.begin().await?;

        // Delete watch progress for tracks in this album
        sqlx::query(
            r#"
            DELETE FROM watch_progress 
            WHERE track_id IN (SELECT id FROM tracks WHERE album_id = $1)
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete media files for tracks in this album
        sqlx::query(
            r#"
            DELETE FROM media_files 
            WHERE track_id IN (SELECT id FROM tracks WHERE album_id = $1)
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Delete tracks for this album
        sqlx::query("DELETE FROM tracks WHERE album_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Unlink any torrents associated with this album
        sqlx::query("UPDATE torrents SET album_id = NULL WHERE album_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete the album itself
        let result = sqlx::query("DELETE FROM albums WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(result.rows_affected() > 0)
    }
}
