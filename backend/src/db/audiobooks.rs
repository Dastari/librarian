//! Audiobook database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Audiobook record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AudiobookRecord {
    pub id: Uuid,
    pub author_id: Option<Uuid>,
    pub library_id: Uuid,
    pub user_id: Uuid,
    // Basic info
    pub title: String,
    pub sort_title: Option<String>,
    pub subtitle: Option<String>,
    // External IDs
    pub audible_id: Option<String>,
    pub asin: Option<String>,
    pub isbn: Option<String>,
    pub openlibrary_id: Option<String>,
    pub goodreads_id: Option<String>,
    // Metadata
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub publish_date: Option<chrono::NaiveDate>,
    pub language: Option<String>,
    // Narrators
    pub narrators: Vec<String>,
    // Series info
    pub series_name: Option<String>,
    pub series_position: Option<rust_decimal::Decimal>,
    // Duration
    pub duration_secs: Option<i32>,
    // Ratings
    pub audible_rating: Option<rust_decimal::Decimal>,
    pub audible_rating_count: Option<i32>,
    // Artwork
    pub cover_url: Option<String>,
    // File status
    pub has_files: bool,
    pub size_bytes: Option<i64>,
    pub is_finished: Option<bool>,
    pub last_played_at: Option<chrono::DateTime<chrono::Utc>>,
    pub path: Option<String>,
    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Audiobook author record from database (minimal for organization)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AudiobookAuthorRecord {
    pub id: Uuid,
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
    pub openlibrary_id: Option<String>,
}

/// Input for creating a new audiobook
#[derive(Debug, Clone)]
pub struct CreateAudiobook {
    pub author_id: Option<Uuid>,
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub sort_title: Option<String>,
    pub subtitle: Option<String>,
    pub openlibrary_id: Option<String>,
    pub isbn: Option<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub cover_url: Option<String>,
}

pub struct AudiobookRepository {
    pool: PgPool,
}

impl AudiobookRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get an audiobook by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<AudiobookRecord>> {
        let record = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT id, author_id, library_id, user_id, title, sort_title, subtitle,
                   audible_id, asin, isbn, openlibrary_id, goodreads_id,
                   description, publisher, publish_date, language, narrators,
                   series_name, series_position, duration_secs,
                   audible_rating, audible_rating_count, cover_url,
                   has_files, size_bytes, is_finished, last_played_at, path,
                   created_at, updated_at
            FROM audiobooks
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get an audiobook by OpenLibrary ID within a library
    pub async fn get_by_openlibrary_id(
        &self,
        library_id: Uuid,
        openlibrary_id: &str,
    ) -> Result<Option<AudiobookRecord>> {
        let record = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT id, author_id, library_id, user_id, title, sort_title, subtitle,
                   audible_id, asin, isbn, openlibrary_id, goodreads_id,
                   description, publisher, publish_date, language, narrators,
                   series_name, series_position, duration_secs,
                   audible_rating, audible_rating_count, cover_url,
                   has_files, size_bytes, is_finished, last_played_at, path,
                   created_at, updated_at
            FROM audiobooks
            WHERE library_id = $1 AND openlibrary_id = $2
            "#,
        )
        .bind(library_id)
        .bind(openlibrary_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get an author by ID
    pub async fn get_author_by_id(&self, id: Uuid) -> Result<Option<AudiobookAuthorRecord>> {
        let record = sqlx::query_as::<_, AudiobookAuthorRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, openlibrary_id
            FROM authors
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Find or create an author by name
    pub async fn find_or_create_author(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        name: &str,
        openlibrary_id: Option<&str>,
    ) -> Result<AudiobookAuthorRecord> {
        // First try to find by OpenLibrary ID if provided
        if let Some(ol_id) = openlibrary_id {
            let existing = sqlx::query_as::<_, AudiobookAuthorRecord>(
                r#"
                SELECT id, library_id, user_id, name, sort_name, openlibrary_id
                FROM authors
                WHERE library_id = $1 AND openlibrary_id = $2
                "#,
            )
            .bind(library_id)
            .bind(ol_id)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(author) = existing {
                return Ok(author);
            }
        }

        // Try to find by name
        let existing = sqlx::query_as::<_, AudiobookAuthorRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, openlibrary_id
            FROM authors
            WHERE library_id = $1 AND LOWER(name) = LOWER($2)
            "#,
        )
        .bind(library_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(author) = existing {
            return Ok(author);
        }

        // Create new author
        let author = sqlx::query_as::<_, AudiobookAuthorRecord>(
            r#"
            INSERT INTO authors (library_id, user_id, name, openlibrary_id)
            VALUES ($1, $2, $3, $4)
            RETURNING id, library_id, user_id, name, sort_name, openlibrary_id
            "#,
        )
        .bind(library_id)
        .bind(user_id)
        .bind(name)
        .bind(openlibrary_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(author)
    }

    /// Create a new audiobook
    pub async fn create(&self, input: CreateAudiobook) -> Result<AudiobookRecord> {
        let record = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            INSERT INTO audiobooks (
                author_id, library_id, user_id, title, sort_title, subtitle,
                openlibrary_id, isbn, description, publisher, language, cover_url
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, author_id, library_id, user_id, title, sort_title, subtitle,
                      audible_id, asin, isbn, openlibrary_id, goodreads_id,
                      description, publisher, publish_date, language, narrators,
                      series_name, series_position, duration_secs,
                      audible_rating, audible_rating_count, cover_url,
                      has_files, size_bytes, is_finished, last_played_at, path,
                      created_at, updated_at
            "#,
        )
        .bind(input.author_id)
        .bind(input.library_id)
        .bind(input.user_id)
        .bind(&input.title)
        .bind(&input.sort_title)
        .bind(&input.subtitle)
        .bind(&input.openlibrary_id)
        .bind(&input.isbn)
        .bind(&input.description)
        .bind(&input.publisher)
        .bind(&input.language)
        .bind(&input.cover_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// List all audiobooks in a library
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<AudiobookRecord>> {
        let records = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT id, author_id, library_id, user_id, title, sort_title, subtitle,
                   audible_id, asin, isbn, openlibrary_id, goodreads_id,
                   description, publisher, publish_date, language, narrators,
                   series_name, series_position, duration_secs,
                   audible_rating, audible_rating_count, cover_url,
                   has_files, size_bytes, is_finished, last_played_at, path,
                   created_at, updated_at
            FROM audiobooks
            WHERE library_id = $1
            ORDER BY COALESCE(sort_title, title)
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List audiobooks in a library with pagination and filtering
    #[allow(clippy::too_many_arguments)]
    pub async fn list_by_library_paginated(
        &self,
        library_id: Uuid,
        offset: i64,
        limit: i64,
        title_filter: Option<&str>,
        has_files_filter: Option<bool>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<AudiobookRecord>, i64)> {
        let mut conditions = vec!["library_id = $1".to_string()];
        let mut param_idx = 2;

        if title_filter.is_some() {
            conditions.push(format!("LOWER(title) LIKE ${}", param_idx));
            param_idx += 1;
        }
        if has_files_filter.is_some() {
            conditions.push(format!("has_files = ${}", param_idx));
        }

        let where_clause = conditions.join(" AND ");

        let valid_sort_columns = ["title", "sort_title", "created_at"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "sort_title"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        let order_clause = format!(
            "ORDER BY COALESCE({}, title) {} NULLS LAST",
            sort_col, order_dir
        );

        let count_query = format!("SELECT COUNT(*) FROM audiobooks WHERE {}", where_clause);
        let data_query = format!(
            r#"
            SELECT id, author_id, library_id, user_id, title, sort_title, subtitle,
                   audible_id, asin, isbn, openlibrary_id, goodreads_id,
                   description, publisher, publish_date, language, narrators,
                   series_name, series_position, duration_secs,
                   audible_rating, audible_rating_count, cover_url,
                   has_files, size_bytes, is_finished, last_played_at, path,
                   created_at, updated_at
            FROM audiobooks
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
        if let Some(has_files) = has_files_filter {
            count_builder = count_builder.bind(has_files);
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let mut data_builder = sqlx::query_as::<_, AudiobookRecord>(&data_query).bind(library_id);
        if let Some(title) = title_filter {
            data_builder = data_builder.bind(format!("%{}%", title.to_lowercase()));
        }
        if let Some(has_files) = has_files_filter {
            data_builder = data_builder.bind(has_files);
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// List all authors in a library
    pub async fn list_authors_by_library(
        &self,
        library_id: Uuid,
    ) -> Result<Vec<AudiobookAuthorRecord>> {
        let records = sqlx::query_as::<_, AudiobookAuthorRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, openlibrary_id
            FROM authors
            WHERE library_id = $1
            ORDER BY COALESCE(sort_name, name)
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List audiobook authors in a library with pagination and filtering
    pub async fn list_authors_by_library_paginated(
        &self,
        library_id: Uuid,
        offset: i64,
        limit: i64,
        name_filter: Option<&str>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<AudiobookAuthorRecord>, i64)> {
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
        let order_clause = format!(
            "ORDER BY COALESCE({}, name) {} NULLS LAST",
            sort_col, order_dir
        );

        let count_query = format!("SELECT COUNT(*) FROM authors WHERE {}", where_clause);
        let data_query = format!(
            r#"
            SELECT id, library_id, user_id, name, sort_name, openlibrary_id
            FROM authors
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

        let mut data_builder =
            sqlx::query_as::<_, AudiobookAuthorRecord>(&data_query).bind(library_id);
        if let Some(name) = name_filter {
            data_builder = data_builder.bind(format!("%{}%", name.to_lowercase()));
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// Update audiobook has_files status
    pub async fn update_has_files(&self, id: Uuid, has_files: bool) -> Result<()> {
        sqlx::query("UPDATE audiobooks SET has_files = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(has_files)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update audiobook path
    pub async fn update_path(&self, id: Uuid, path: &str) -> Result<()> {
        sqlx::query("UPDATE audiobooks SET path = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(path)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete an audiobook and its associated data
    ///
    /// This will also delete:
    /// - All media files linked to this audiobook
    /// - Torrent links to this audiobook
    /// - Watch progress for this audiobook
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        // Start a transaction to ensure all deletions are atomic
        let mut tx = self.pool.begin().await?;

        // Delete media files for this audiobook (torrent_file_matches handles cleanup via ON DELETE SET NULL)
        sqlx::query("DELETE FROM media_files WHERE audiobook_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete watch progress for this audiobook
        sqlx::query("DELETE FROM watch_progress WHERE audiobook_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // Delete the audiobook itself
        let result = sqlx::query("DELETE FROM audiobooks WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(result.rows_affected() > 0)
    }
}

// ============================================================================
// Audiobook Chapters
// ============================================================================

/// Audiobook chapter record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AudiobookChapterRecord {
    pub id: Uuid,
    pub audiobook_id: Uuid,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub start_secs: i32,
    pub end_secs: i32,
    pub duration_secs: Option<i32>,
    pub media_file_id: Option<Uuid>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Input for creating a new audiobook chapter
#[derive(Debug, Clone)]
pub struct CreateAudiobookChapter {
    pub audiobook_id: Uuid,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub start_secs: i32,
    pub end_secs: i32,
}

pub struct AudiobookChapterRepository {
    pool: PgPool,
}

impl AudiobookChapterRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a chapter by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<AudiobookChapterRecord>> {
        let record = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            SELECT id, audiobook_id, chapter_number, title, start_secs, end_secs,
                   duration_secs, media_file_id, status, created_at
            FROM chapters
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// List all chapters for an audiobook
    pub async fn list_by_audiobook(
        &self,
        audiobook_id: Uuid,
    ) -> Result<Vec<AudiobookChapterRecord>> {
        let records = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            SELECT id, audiobook_id, chapter_number, title, start_secs, end_secs,
                   duration_secs, media_file_id, status, created_at
            FROM chapters
            WHERE audiobook_id = $1
            ORDER BY chapter_number
            "#,
        )
        .bind(audiobook_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List chapters with pagination and filtering
    #[allow(clippy::too_many_arguments)]
    pub async fn list_by_audiobook_paginated(
        &self,
        audiobook_id: Uuid,
        offset: i64,
        limit: i64,
        status_filter: Option<&str>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<AudiobookChapterRecord>, i64)> {
        let mut conditions = vec!["audiobook_id = $1".to_string()];
        let param_idx = 2;

        if status_filter.is_some() {
            conditions.push(format!("status = ${}", param_idx));
        }

        let where_clause = conditions.join(" AND ");

        let valid_sort_columns = ["chapter_number", "title", "created_at"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "chapter_number"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        let order_clause = format!("ORDER BY {} {} NULLS LAST", sort_col, order_dir);

        let count_query = format!("SELECT COUNT(*) FROM chapters WHERE {}", where_clause);
        let data_query = format!(
            r#"
            SELECT id, audiobook_id, chapter_number, title, start_secs, end_secs,
                   duration_secs, media_file_id, status, created_at
            FROM chapters
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_clause, limit, offset
        );

        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(audiobook_id);
        if let Some(status) = status_filter {
            count_builder = count_builder.bind(status);
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let mut data_builder =
            sqlx::query_as::<_, AudiobookChapterRecord>(&data_query).bind(audiobook_id);
        if let Some(status) = status_filter {
            data_builder = data_builder.bind(status);
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// Create a new chapter
    pub async fn create(&self, input: CreateAudiobookChapter) -> Result<AudiobookChapterRecord> {
        let record = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            INSERT INTO chapters (audiobook_id, chapter_number, title, start_secs, end_secs)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, audiobook_id, chapter_number, title, start_secs, end_secs,
                      duration_secs, media_file_id, status, created_at
            "#,
        )
        .bind(input.audiobook_id)
        .bind(input.chapter_number)
        .bind(&input.title)
        .bind(input.start_secs)
        .bind(input.end_secs)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update chapter status
    pub async fn update_status(&self, id: Uuid, status: &str) -> Result<()> {
        sqlx::query("UPDATE chapters SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(status)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Link chapter to media file
    pub async fn link_media_file(&self, id: Uuid, media_file_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE chapters SET media_file_id = $2, status = 'downloaded' WHERE id = $1")
            .bind(id)
            .bind(media_file_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete all chapters for an audiobook
    pub async fn delete_by_audiobook(&self, audiobook_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM chapters WHERE audiobook_id = $1")
            .bind(audiobook_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
