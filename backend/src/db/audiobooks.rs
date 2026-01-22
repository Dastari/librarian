//! Audiobook database repository

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

/// Audiobook record from database
#[derive(Debug, Clone)]
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
    /// Total number of chapters
    pub chapter_count: Option<i32>,
    /// Number of chapters with media files
    pub downloaded_chapter_count: Option<i32>,
    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for AudiobookRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            author_id: row.try_get("author_id")?,
            library_id: row.try_get("library_id")?,
            user_id: row.try_get("user_id")?,
            title: row.try_get("title")?,
            sort_title: row.try_get("sort_title")?,
            subtitle: row.try_get("subtitle")?,
            audible_id: row.try_get("audible_id")?,
            asin: row.try_get("asin")?,
            isbn: row.try_get("isbn")?,
            openlibrary_id: row.try_get("openlibrary_id")?,
            goodreads_id: row.try_get("goodreads_id")?,
            description: row.try_get("description")?,
            publisher: row.try_get("publisher")?,
            publish_date: row.try_get("publish_date")?,
            language: row.try_get("language")?,
            narrators: row.try_get("narrators")?,
            series_name: row.try_get("series_name")?,
            series_position: row.try_get("series_position")?,
            duration_secs: row.try_get("duration_secs")?,
            audible_rating: row.try_get("audible_rating")?,
            audible_rating_count: row.try_get("audible_rating_count")?,
            cover_url: row.try_get("cover_url")?,
            has_files: row.try_get("has_files")?,
            size_bytes: row.try_get("size_bytes")?,
            is_finished: row.try_get("is_finished")?,
            last_played_at: row.try_get("last_played_at")?,
            path: row.try_get("path")?,
            chapter_count: row.try_get("chapter_count")?,
            downloaded_chapter_count: row.try_get("downloaded_chapter_count")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for AudiobookRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use crate::db::sqlite_helpers::{str_to_uuid, str_to_datetime, int_to_bool, json_to_vec};
        use std::str::FromStr;
        
        let id_str: String = row.try_get("id")?;
        let author_id_str: Option<String> = row.try_get("author_id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;
        
        // JSON array stored as TEXT
        let narrators_json: String = row.try_get("narrators")?;
        
        // Decimals stored as TEXT
        let series_position_str: Option<String> = row.try_get("series_position")?;
        let audible_rating_str: Option<String> = row.try_get("audible_rating")?;
        
        // Booleans stored as INTEGER
        let has_files: i32 = row.try_get("has_files")?;
        let is_finished: Option<i32> = row.try_get("is_finished")?;
        
        // NaiveDate stored as TEXT (YYYY-MM-DD)
        let publish_date_str: Option<String> = row.try_get("publish_date")?;
        
        // DateTime stored as TEXT
        let last_played_at_str: Option<String> = row.try_get("last_played_at")?;
        
        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            author_id: author_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_id: str_to_uuid(&library_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            title: row.try_get("title")?,
            sort_title: row.try_get("sort_title")?,
            subtitle: row.try_get("subtitle")?,
            audible_id: row.try_get("audible_id")?,
            asin: row.try_get("asin")?,
            isbn: row.try_get("isbn")?,
            openlibrary_id: row.try_get("openlibrary_id")?,
            goodreads_id: row.try_get("goodreads_id")?,
            description: row.try_get("description")?,
            publisher: row.try_get("publisher")?,
            publish_date: publish_date_str
                .map(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            language: row.try_get("language")?,
            narrators: json_to_vec(&narrators_json),
            series_name: row.try_get("series_name")?,
            series_position: series_position_str
                .map(|s| rust_decimal::Decimal::from_str(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            duration_secs: row.try_get("duration_secs")?,
            audible_rating: audible_rating_str
                .map(|s| rust_decimal::Decimal::from_str(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            audible_rating_count: row.try_get("audible_rating_count")?,
            cover_url: row.try_get("cover_url")?,
            has_files: int_to_bool(has_files),
            size_bytes: row.try_get("size_bytes")?,
            is_finished: is_finished.map(int_to_bool),
            last_played_at: last_played_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            path: row.try_get("path")?,
            chapter_count: row.try_get("chapter_count")?,
            downloaded_chapter_count: row.try_get("downloaded_chapter_count")?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Audiobook author record from database (minimal for organization)
#[derive(Debug, Clone)]
pub struct AudiobookAuthorRecord {
    pub id: Uuid,
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
    pub openlibrary_id: Option<String>,
}

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for AudiobookAuthorRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            library_id: row.try_get("library_id")?,
            user_id: row.try_get("user_id")?,
            name: row.try_get("name")?,
            sort_name: row.try_get("sort_name")?,
            openlibrary_id: row.try_get("openlibrary_id")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for AudiobookAuthorRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use crate::db::sqlite_helpers::str_to_uuid;
        
        let id_str: String = row.try_get("id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let user_id_str: String = row.try_get("user_id")?;
        
        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_id: str_to_uuid(&library_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            name: row.try_get("name")?,
            sort_name: row.try_get("sort_name")?,
            openlibrary_id: row.try_get("openlibrary_id")?,
        })
    }
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
    pool: DbPool,
}

impl AudiobookRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get an audiobook by ID
    #[cfg(feature = "postgres")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<AudiobookRecord>> {
        let record = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT a.id, a.author_id, a.library_id, a.user_id, a.title, a.sort_title, a.subtitle,
                   a.audible_id, a.asin, a.isbn, a.openlibrary_id, a.goodreads_id,
                   a.description, a.publisher, a.publish_date, a.language, a.narrators,
                   a.series_name, a.series_position, a.duration_secs,
                   a.audible_rating, a.audible_rating_count, a.cover_url,
                   a.has_files, a.size_bytes, a.is_finished, a.last_played_at, a.path,
                   (SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id) as chapter_count,
                   (SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id AND c.media_file_id IS NOT NULL) as downloaded_chapter_count,
                   a.created_at, a.updated_at
            FROM audiobooks a
            WHERE a.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<AudiobookRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let record = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT a.id, a.author_id, a.library_id, a.user_id, a.title, a.sort_title, a.subtitle,
                   a.audible_id, a.asin, a.isbn, a.openlibrary_id, a.goodreads_id,
                   a.description, a.publisher, a.publish_date, a.language, a.narrators,
                   a.series_name, a.series_position, a.duration_secs,
                   a.audible_rating, a.audible_rating_count, a.cover_url,
                   a.has_files, a.size_bytes, a.is_finished, a.last_played_at, a.path,
                   (SELECT CAST(COUNT(*) AS INTEGER) FROM chapters c WHERE c.audiobook_id = a.id) as chapter_count,
                   (SELECT CAST(COUNT(*) AS INTEGER) FROM chapters c WHERE c.audiobook_id = a.id AND c.media_file_id IS NOT NULL) as downloaded_chapter_count,
                   a.created_at, a.updated_at
            FROM audiobooks a
            WHERE a.id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get an audiobook by OpenLibrary ID within a library
    #[cfg(feature = "postgres")]
    pub async fn get_by_openlibrary_id(
        &self,
        library_id: Uuid,
        openlibrary_id: &str,
    ) -> Result<Option<AudiobookRecord>> {
        let record = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT a.id, a.author_id, a.library_id, a.user_id, a.title, a.sort_title, a.subtitle,
                   a.audible_id, a.asin, a.isbn, a.openlibrary_id, a.goodreads_id,
                   a.description, a.publisher, a.publish_date, a.language, a.narrators,
                   a.series_name, a.series_position, a.duration_secs,
                   a.audible_rating, a.audible_rating_count, a.cover_url,
                   a.has_files, a.size_bytes, a.is_finished, a.last_played_at, a.path,
                   (SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id) as chapter_count,
                   (SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id AND c.media_file_id IS NOT NULL) as downloaded_chapter_count,
                   a.created_at, a.updated_at
            FROM audiobooks a
            WHERE a.library_id = $1 AND a.openlibrary_id = $2
            "#,
        )
        .bind(library_id)
        .bind(openlibrary_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_by_openlibrary_id(
        &self,
        library_id: Uuid,
        openlibrary_id: &str,
    ) -> Result<Option<AudiobookRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let record = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT a.id, a.author_id, a.library_id, a.user_id, a.title, a.sort_title, a.subtitle,
                   a.audible_id, a.asin, a.isbn, a.openlibrary_id, a.goodreads_id,
                   a.description, a.publisher, a.publish_date, a.language, a.narrators,
                   a.series_name, a.series_position, a.duration_secs,
                   a.audible_rating, a.audible_rating_count, a.cover_url,
                   a.has_files, a.size_bytes, a.is_finished, a.last_played_at, a.path,
                   (SELECT CAST(COUNT(*) AS INTEGER) FROM chapters c WHERE c.audiobook_id = a.id) as chapter_count,
                   (SELECT CAST(COUNT(*) AS INTEGER) FROM chapters c WHERE c.audiobook_id = a.id AND c.media_file_id IS NOT NULL) as downloaded_chapter_count,
                   a.created_at, a.updated_at
            FROM audiobooks a
            WHERE a.library_id = ?1 AND a.openlibrary_id = ?2
            "#,
        )
        .bind(uuid_to_str(library_id))
        .bind(openlibrary_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get an author by ID
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get_author_by_id(&self, id: Uuid) -> Result<Option<AudiobookAuthorRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let record = sqlx::query_as::<_, AudiobookAuthorRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, openlibrary_id
            FROM authors
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Find or create an author by name
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn find_or_create_author(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        name: &str,
        openlibrary_id: Option<&str>,
    ) -> Result<AudiobookAuthorRecord> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let library_id_str = uuid_to_str(library_id);
        
        // First try to find by OpenLibrary ID if provided
        if let Some(ol_id) = openlibrary_id {
            let existing = sqlx::query_as::<_, AudiobookAuthorRecord>(
                r#"
                SELECT id, library_id, user_id, name, sort_name, openlibrary_id
                FROM authors
                WHERE library_id = ?1 AND openlibrary_id = ?2
                "#,
            )
            .bind(&library_id_str)
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
            WHERE library_id = ?1 AND LOWER(name) = LOWER(?2)
            "#,
        )
        .bind(&library_id_str)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(author) = existing {
            return Ok(author);
        }

        // Create new author
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);
        
        sqlx::query(
            r#"
            INSERT INTO authors (id, library_id, user_id, name, openlibrary_id)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )
        .bind(&id_str)
        .bind(&library_id_str)
        .bind(uuid_to_str(user_id))
        .bind(name)
        .bind(openlibrary_id)
        .execute(&self.pool)
        .await?;

        self.get_author_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to retrieve author after insert"))
    }

    /// Create a new audiobook
    #[cfg(feature = "postgres")]
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
                      0 as chapter_count, 0 as downloaded_chapter_count,
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

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateAudiobook) -> Result<AudiobookRecord> {
        use crate::db::sqlite_helpers::{uuid_to_str, vec_to_json, bool_to_int};
        
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);
        
        sqlx::query(
            r#"
            INSERT INTO audiobooks (
                id, author_id, library_id, user_id, title, sort_title, subtitle,
                openlibrary_id, isbn, description, publisher, language, cover_url,
                narrators, has_files, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                    ?14, ?15, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(input.author_id.map(uuid_to_str))
        .bind(uuid_to_str(input.library_id))
        .bind(uuid_to_str(input.user_id))
        .bind(&input.title)
        .bind(&input.sort_title)
        .bind(&input.subtitle)
        .bind(&input.openlibrary_id)
        .bind(&input.isbn)
        .bind(&input.description)
        .bind(&input.publisher)
        .bind(&input.language)
        .bind(&input.cover_url)
        .bind(vec_to_json::<String>(&[]))  // Empty narrators array
        .bind(bool_to_int(false))  // has_files default
        .execute(&self.pool)
        .await?;

        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to retrieve audiobook after insert"))
    }

    /// List all audiobooks in a library
    #[cfg(feature = "postgres")]
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<AudiobookRecord>> {
        let records = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT a.id, a.author_id, a.library_id, a.user_id, a.title, a.sort_title, a.subtitle,
                   a.audible_id, a.asin, a.isbn, a.openlibrary_id, a.goodreads_id,
                   a.description, a.publisher, a.publish_date, a.language, a.narrators,
                   a.series_name, a.series_position, a.duration_secs,
                   a.audible_rating, a.audible_rating_count, a.cover_url,
                   a.has_files, a.size_bytes, a.is_finished, a.last_played_at, a.path,
                   (SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id) as chapter_count,
                   (SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id AND c.media_file_id IS NOT NULL) as downloaded_chapter_count,
                   a.created_at, a.updated_at
            FROM audiobooks a
            WHERE a.library_id = $1
            ORDER BY COALESCE(a.sort_title, a.title)
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    #[cfg(feature = "sqlite")]
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<AudiobookRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let records = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT a.id, a.author_id, a.library_id, a.user_id, a.title, a.sort_title, a.subtitle,
                   a.audible_id, a.asin, a.isbn, a.openlibrary_id, a.goodreads_id,
                   a.description, a.publisher, a.publish_date, a.language, a.narrators,
                   a.series_name, a.series_position, a.duration_secs,
                   a.audible_rating, a.audible_rating_count, a.cover_url,
                   a.has_files, a.size_bytes, a.is_finished, a.last_played_at, a.path,
                   (SELECT CAST(COUNT(*) AS INTEGER) FROM chapters c WHERE c.audiobook_id = a.id) as chapter_count,
                   (SELECT CAST(COUNT(*) AS INTEGER) FROM chapters c WHERE c.audiobook_id = a.id AND c.media_file_id IS NOT NULL) as downloaded_chapter_count,
                   a.created_at, a.updated_at
            FROM audiobooks a
            WHERE a.library_id = ?1
            ORDER BY COALESCE(a.sort_title, a.title)
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List audiobooks that need files (for auto-hunt)
    /// Returns audiobooks without has_files=true, excluding those with active downloads
    #[cfg(feature = "postgres")]
    pub async fn list_needing_files(
        &self,
        library_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AudiobookRecord>> {
        let records = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT a.id, a.author_id, a.library_id, a.user_id, a.title, a.sort_title, a.subtitle,
                   a.audible_id, a.asin, a.isbn, a.openlibrary_id, a.goodreads_id,
                   a.description, a.publisher, a.publish_date, a.language, a.narrators,
                   a.series_name, a.series_position, a.duration_secs,
                   a.audible_rating, a.audible_rating_count, a.cover_url,
                   a.has_files, a.size_bytes, a.is_finished, a.last_played_at, a.path,
                   (SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id) as chapter_count,
                   (SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id AND c.media_file_id IS NOT NULL) as downloaded_chapter_count,
                   a.created_at, a.updated_at
            FROM audiobooks a
            WHERE a.library_id = $1
              AND a.has_files = false
              AND NOT EXISTS (
                  SELECT 1 FROM pending_file_matches pfm
                  WHERE pfm.chapter_id IN (SELECT id FROM chapters WHERE audiobook_id = a.id)
                    AND pfm.copied_at IS NULL
              )
            ORDER BY a.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(library_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    #[cfg(feature = "sqlite")]
    pub async fn list_needing_files(
        &self,
        library_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AudiobookRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let records = sqlx::query_as::<_, AudiobookRecord>(
            r#"
            SELECT a.id, a.author_id, a.library_id, a.user_id, a.title, a.sort_title, a.subtitle,
                   a.audible_id, a.asin, a.isbn, a.openlibrary_id, a.goodreads_id,
                   a.description, a.publisher, a.publish_date, a.language, a.narrators,
                   a.series_name, a.series_position, a.duration_secs,
                   a.audible_rating, a.audible_rating_count, a.cover_url,
                   a.has_files, a.size_bytes, a.is_finished, a.last_played_at, a.path,
                   (SELECT CAST(COUNT(*) AS INTEGER) FROM chapters c WHERE c.audiobook_id = a.id) as chapter_count,
                   (SELECT CAST(COUNT(*) AS INTEGER) FROM chapters c WHERE c.audiobook_id = a.id AND c.media_file_id IS NOT NULL) as downloaded_chapter_count,
                   a.created_at, a.updated_at
            FROM audiobooks a
            WHERE a.library_id = ?1
              AND a.has_files = 0
              AND NOT EXISTS (
                  SELECT 1 FROM pending_file_matches pfm
                  WHERE pfm.chapter_id IN (SELECT id FROM chapters WHERE audiobook_id = a.id)
                    AND pfm.copied_at IS NULL
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

    /// List audiobooks in a library with pagination and filtering
    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "postgres")]
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
        let _ = param_idx;

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
            SELECT a.id, a.author_id, a.library_id, a.user_id, a.title, a.sort_title, a.subtitle,
                   a.audible_id, a.asin, a.isbn, a.openlibrary_id, a.goodreads_id,
                   a.description, a.publisher, a.publish_date, a.language, a.narrators,
                   a.series_name, a.series_position, a.duration_secs,
                   a.audible_rating, a.audible_rating_count, a.cover_url,
                   a.has_files, a.size_bytes, a.is_finished, a.last_played_at, a.path,
                   (SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id) as chapter_count,
                   (SELECT COUNT(*)::int FROM chapters c WHERE c.audiobook_id = a.id AND c.media_file_id IS NOT NULL) as downloaded_chapter_count,
                   a.created_at, a.updated_at
            FROM audiobooks a
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause.replace("library_id", "a.library_id")
                .replace("title", "a.title")
                .replace("has_files", "a.has_files"),
            order_clause.replace(sort_col, &format!("a.{}", sort_col)),
            limit, offset
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

    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "sqlite")]
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
        use crate::db::sqlite_helpers::{uuid_to_str, bool_to_int};
        
        let mut conditions = vec!["library_id = ?1".to_string()];
        let mut param_idx = 2;

        if title_filter.is_some() {
            conditions.push(format!("LOWER(title) LIKE ?{}", param_idx));
            param_idx += 1;
        }
        if has_files_filter.is_some() {
            conditions.push(format!("has_files = ?{}", param_idx));
        }
        let _ = param_idx;

        let where_clause = conditions.join(" AND ");

        let valid_sort_columns = ["title", "sort_title", "created_at"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "sort_title"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        // SQLite doesn't support NULLS LAST directly
        let order_clause = format!(
            "ORDER BY CASE WHEN COALESCE({}, title) IS NULL THEN 1 ELSE 0 END, COALESCE({}, title) {}",
            sort_col, sort_col, order_dir
        );

        let count_query = format!("SELECT COUNT(*) FROM audiobooks WHERE {}", where_clause);
        let data_query = format!(
            r#"
            SELECT a.id, a.author_id, a.library_id, a.user_id, a.title, a.sort_title, a.subtitle,
                   a.audible_id, a.asin, a.isbn, a.openlibrary_id, a.goodreads_id,
                   a.description, a.publisher, a.publish_date, a.language, a.narrators,
                   a.series_name, a.series_position, a.duration_secs,
                   a.audible_rating, a.audible_rating_count, a.cover_url,
                   a.has_files, a.size_bytes, a.is_finished, a.last_played_at, a.path,
                   (SELECT CAST(COUNT(*) AS INTEGER) FROM chapters c WHERE c.audiobook_id = a.id) as chapter_count,
                   (SELECT CAST(COUNT(*) AS INTEGER) FROM chapters c WHERE c.audiobook_id = a.id AND c.media_file_id IS NOT NULL) as downloaded_chapter_count,
                   a.created_at, a.updated_at
            FROM audiobooks a
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause.replace("library_id", "a.library_id")
                .replace("title", "a.title")
                .replace("has_files", "a.has_files"),
            order_clause.replace(sort_col, &format!("a.{}", sort_col)),
            limit, offset
        );

        let library_id_str = uuid_to_str(library_id);
        
        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(&library_id_str);
        if let Some(title) = title_filter {
            count_builder = count_builder.bind(format!("%{}%", title.to_lowercase()));
        }
        if let Some(has_files) = has_files_filter {
            count_builder = count_builder.bind(bool_to_int(has_files));
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let mut data_builder = sqlx::query_as::<_, AudiobookRecord>(&data_query).bind(&library_id_str);
        if let Some(title) = title_filter {
            data_builder = data_builder.bind(format!("%{}%", title.to_lowercase()));
        }
        if let Some(has_files) = has_files_filter {
            data_builder = data_builder.bind(bool_to_int(has_files));
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// List all authors in a library
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn list_authors_by_library(
        &self,
        library_id: Uuid,
    ) -> Result<Vec<AudiobookAuthorRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let records = sqlx::query_as::<_, AudiobookAuthorRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, openlibrary_id
            FROM authors
            WHERE library_id = ?1
            ORDER BY COALESCE(sort_name, name)
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List audiobook authors in a library with pagination and filtering
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn list_authors_by_library_paginated(
        &self,
        library_id: Uuid,
        offset: i64,
        limit: i64,
        name_filter: Option<&str>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<AudiobookAuthorRecord>, i64)> {
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
            "ORDER BY CASE WHEN COALESCE({}, name) IS NULL THEN 1 ELSE 0 END, COALESCE({}, name) {}",
            sort_col, sort_col, order_dir
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

        let library_id_str = uuid_to_str(library_id);
        
        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(&library_id_str);
        if let Some(name) = name_filter {
            count_builder = count_builder.bind(format!("%{}%", name.to_lowercase()));
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let mut data_builder =
            sqlx::query_as::<_, AudiobookAuthorRecord>(&data_query).bind(&library_id_str);
        if let Some(name) = name_filter {
            data_builder = data_builder.bind(format!("%{}%", name.to_lowercase()));
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// Update audiobook has_files status
    #[cfg(feature = "postgres")]
    pub async fn update_has_files(&self, id: Uuid, has_files: bool) -> Result<()> {
        sqlx::query("UPDATE audiobooks SET has_files = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(has_files)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn update_has_files(&self, id: Uuid, has_files: bool) -> Result<()> {
        use crate::db::sqlite_helpers::{uuid_to_str, bool_to_int};
        
        sqlx::query("UPDATE audiobooks SET has_files = ?2, updated_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(id))
            .bind(bool_to_int(has_files))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update audiobook path
    #[cfg(feature = "postgres")]
    pub async fn update_path(&self, id: Uuid, path: &str) -> Result<()> {
        sqlx::query("UPDATE audiobooks SET path = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(path)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn update_path(&self, id: Uuid, path: &str) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        sqlx::query("UPDATE audiobooks SET path = ?2, updated_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(id))
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
    #[cfg(feature = "postgres")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        // Start a transaction to ensure all deletions are atomic
        let mut tx = self.pool.begin().await?;

        // Delete media files for this audiobook (pending_file_matches handles cleanup via ON DELETE CASCADE)
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

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let id_str = uuid_to_str(id);
        
        // Start a transaction to ensure all deletions are atomic
        let mut tx = self.pool.begin().await?;

        // Delete media files for this audiobook (pending_file_matches handles cleanup via ON DELETE CASCADE)
        sqlx::query("DELETE FROM media_files WHERE audiobook_id = ?1")
            .bind(&id_str)
            .execute(&mut *tx)
            .await?;

        // Delete watch progress for this audiobook
        sqlx::query("DELETE FROM watch_progress WHERE audiobook_id = ?1")
            .bind(&id_str)
            .execute(&mut *tx)
            .await?;

        // Delete the audiobook itself
        let result = sqlx::query("DELETE FROM audiobooks WHERE id = ?1")
            .bind(&id_str)
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
#[derive(Debug, Clone)]
pub struct AudiobookChapterRecord {
    pub id: Uuid,
    pub audiobook_id: Uuid,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub start_secs: i32,
    pub end_secs: i32,
    pub duration_secs: Option<i32>,
    pub media_file_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for AudiobookChapterRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            audiobook_id: row.try_get("audiobook_id")?,
            chapter_number: row.try_get("chapter_number")?,
            title: row.try_get("title")?,
            start_secs: row.try_get("start_secs")?,
            end_secs: row.try_get("end_secs")?,
            duration_secs: row.try_get("duration_secs")?,
            media_file_id: row.try_get("media_file_id")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for AudiobookChapterRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use crate::db::sqlite_helpers::{str_to_uuid, str_to_datetime};
        
        let id_str: String = row.try_get("id")?;
        let audiobook_id_str: String = row.try_get("audiobook_id")?;
        let media_file_id_str: Option<String> = row.try_get("media_file_id")?;
        let created_str: String = row.try_get("created_at")?;
        
        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            audiobook_id: str_to_uuid(&audiobook_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            chapter_number: row.try_get("chapter_number")?,
            title: row.try_get("title")?,
            start_secs: row.try_get("start_secs")?,
            end_secs: row.try_get("end_secs")?,
            duration_secs: row.try_get("duration_secs")?,
            media_file_id: media_file_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
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
    pool: DbPool,
}

impl AudiobookChapterRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get a chapter by ID
    #[cfg(feature = "postgres")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<AudiobookChapterRecord>> {
        let record = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            SELECT id, audiobook_id, chapter_number, title, start_secs, end_secs,
                   duration_secs, media_file_id, created_at
            FROM chapters
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<AudiobookChapterRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let record = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            SELECT id, audiobook_id, chapter_number, title, start_secs, end_secs,
                   duration_secs, media_file_id, created_at
            FROM chapters
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// List all chapters for an audiobook
    #[cfg(feature = "postgres")]
    pub async fn list_by_audiobook(
        &self,
        audiobook_id: Uuid,
    ) -> Result<Vec<AudiobookChapterRecord>> {
        let records = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            SELECT id, audiobook_id, chapter_number, title, start_secs, end_secs,
                   duration_secs, media_file_id, created_at
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

    #[cfg(feature = "sqlite")]
    pub async fn list_by_audiobook(
        &self,
        audiobook_id: Uuid,
    ) -> Result<Vec<AudiobookChapterRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let records = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            SELECT id, audiobook_id, chapter_number, title, start_secs, end_secs,
                   duration_secs, media_file_id, created_at
            FROM chapters
            WHERE audiobook_id = ?1
            ORDER BY chapter_number
            "#,
        )
        .bind(uuid_to_str(audiobook_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List chapters with pagination and filtering
    ///
    /// `has_media_file_filter`: If Some(true), only chapters with media_file_id set.
    ///                          If Some(false), only chapters without media_file_id.
    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "postgres")]
    pub async fn list_by_audiobook_paginated(
        &self,
        audiobook_id: Uuid,
        offset: i64,
        limit: i64,
        has_media_file_filter: Option<bool>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<AudiobookChapterRecord>, i64)> {
        let mut conditions = vec!["audiobook_id = $1".to_string()];

        if let Some(has_file) = has_media_file_filter {
            if has_file {
                conditions.push("media_file_id IS NOT NULL".to_string());
            } else {
                conditions.push("media_file_id IS NULL".to_string());
            }
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
                   duration_secs, media_file_id, created_at
            FROM chapters
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_clause, limit, offset
        );

        let count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(audiobook_id);
        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let data_builder =
            sqlx::query_as::<_, AudiobookChapterRecord>(&data_query).bind(audiobook_id);
        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "sqlite")]
    pub async fn list_by_audiobook_paginated(
        &self,
        audiobook_id: Uuid,
        offset: i64,
        limit: i64,
        has_media_file_filter: Option<bool>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<AudiobookChapterRecord>, i64)> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let mut conditions = vec!["audiobook_id = ?1".to_string()];

        if let Some(has_file) = has_media_file_filter {
            if has_file {
                conditions.push("media_file_id IS NOT NULL".to_string());
            } else {
                conditions.push("media_file_id IS NULL".to_string());
            }
        }

        let where_clause = conditions.join(" AND ");

        let valid_sort_columns = ["chapter_number", "title", "created_at"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "chapter_number"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        // SQLite doesn't support NULLS LAST directly
        let order_clause = format!(
            "ORDER BY CASE WHEN {} IS NULL THEN 1 ELSE 0 END, {} {}",
            sort_col, sort_col, order_dir
        );

        let count_query = format!("SELECT COUNT(*) FROM chapters WHERE {}", where_clause);
        let data_query = format!(
            r#"
            SELECT id, audiobook_id, chapter_number, title, start_secs, end_secs,
                   duration_secs, media_file_id, created_at
            FROM chapters
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_clause, limit, offset
        );

        let audiobook_id_str = uuid_to_str(audiobook_id);
        
        let count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(&audiobook_id_str);
        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        let data_builder =
            sqlx::query_as::<_, AudiobookChapterRecord>(&data_query).bind(&audiobook_id_str);
        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// Create a new chapter
    #[cfg(feature = "postgres")]
    pub async fn create(&self, input: CreateAudiobookChapter) -> Result<AudiobookChapterRecord> {
        let record = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            INSERT INTO chapters (audiobook_id, chapter_number, title, start_secs, end_secs)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, audiobook_id, chapter_number, title, start_secs, end_secs,
                      duration_secs, media_file_id, created_at
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

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateAudiobookChapter) -> Result<AudiobookChapterRecord> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);
        
        sqlx::query(
            r#"
            INSERT INTO chapters (id, audiobook_id, chapter_number, title, start_secs, end_secs, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.audiobook_id))
        .bind(input.chapter_number)
        .bind(&input.title)
        .bind(input.start_secs)
        .bind(input.end_secs)
        .execute(&self.pool)
        .await?;

        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to retrieve chapter after insert"))
    }

    /// Link chapter to media file
    #[cfg(feature = "postgres")]
    pub async fn link_media_file(&self, id: Uuid, media_file_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE chapters SET media_file_id = $2 WHERE id = $1")
            .bind(id)
            .bind(media_file_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn link_media_file(&self, id: Uuid, media_file_id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        sqlx::query("UPDATE chapters SET media_file_id = ?2 WHERE id = ?1")
            .bind(uuid_to_str(id))
            .bind(uuid_to_str(media_file_id))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete all chapters for an audiobook
    #[cfg(feature = "postgres")]
    pub async fn delete_by_audiobook(&self, audiobook_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM chapters WHERE audiobook_id = $1")
            .bind(audiobook_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    #[cfg(feature = "sqlite")]
    pub async fn delete_by_audiobook(&self, audiobook_id: Uuid) -> Result<u64> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let result = sqlx::query("DELETE FROM chapters WHERE audiobook_id = ?1")
            .bind(uuid_to_str(audiobook_id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Get or create a chapter by audiobook ID and chapter number
    ///
    /// Returns the existing chapter if it exists, or creates a new one if not.
    #[cfg(feature = "postgres")]
    pub async fn get_or_create_by_number(
        &self,
        audiobook_id: Uuid,
        chapter_number: i32,
    ) -> Result<AudiobookChapterRecord> {
        // First try to find existing chapter
        let existing = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            SELECT id, audiobook_id, chapter_number, title, start_secs, end_secs,
                   duration_secs, media_file_id, created_at
            FROM chapters
            WHERE audiobook_id = $1 AND chapter_number = $2
            "#,
        )
        .bind(audiobook_id)
        .bind(chapter_number)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(chapter) = existing {
            return Ok(chapter);
        }

        // Create new chapter
        let record = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            INSERT INTO chapters (audiobook_id, chapter_number, title)
            VALUES ($1, $2, $3)
            RETURNING id, audiobook_id, chapter_number, title, start_secs, end_secs,
                      duration_secs, media_file_id, created_at
            "#,
        )
        .bind(audiobook_id)
        .bind(chapter_number)
        .bind(format!("Chapter {}", chapter_number))
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_or_create_by_number(
        &self,
        audiobook_id: Uuid,
        chapter_number: i32,
    ) -> Result<AudiobookChapterRecord> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let audiobook_id_str = uuid_to_str(audiobook_id);
        
        // First try to find existing chapter
        let existing = sqlx::query_as::<_, AudiobookChapterRecord>(
            r#"
            SELECT id, audiobook_id, chapter_number, title, start_secs, end_secs,
                   duration_secs, media_file_id, created_at
            FROM chapters
            WHERE audiobook_id = ?1 AND chapter_number = ?2
            "#,
        )
        .bind(&audiobook_id_str)
        .bind(chapter_number)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(chapter) = existing {
            return Ok(chapter);
        }

        // Create new chapter
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);
        
        sqlx::query(
            r#"
            INSERT INTO chapters (id, audiobook_id, chapter_number, title, start_secs, end_secs, created_at)
            VALUES (?1, ?2, ?3, ?4, 0, 0, datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(&audiobook_id_str)
        .bind(chapter_number)
        .bind(format!("Chapter {}", chapter_number))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to retrieve chapter after insert"))
    }
}
