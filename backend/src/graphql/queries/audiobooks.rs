use super::prelude::*;

#[derive(Default)]
pub struct AudiobookQueries;

#[Object]
impl AudiobookQueries {
    /// Get all audiobooks in a library
    async fn audiobooks(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<Audiobook>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .audiobooks()
            .list_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Audiobook::from).collect())
    }

    /// Get audiobooks in a library with cursor-based pagination and filtering
    async fn audiobooks_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<AudiobookWhereInput>,
        order_by: Option<AudiobookOrderByInput>,
    ) -> Result<AudiobookConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let (offset, limit) =
            parse_pagination_args(first, after).map_err(|e| async_graphql::Error::new(e))?;

        let title_filter = r#where
            .as_ref()
            .and_then(|w| w.title.as_ref().and_then(|f| f.contains.clone()));
        let has_files_filter = r#where
            .as_ref()
            .and_then(|w| w.has_files.as_ref().and_then(|f| f.eq));

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(AudiobookSortField::Title);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(OrderDirection::Asc);

        let (records, total) = db
            .audiobooks()
            .list_by_library_paginated(
                lib_id,
                offset,
                limit,
                title_filter.as_deref(),
                has_files_filter,
                &audiobook_sort_field_to_column(sort_field),
                sort_dir == OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let audiobooks: Vec<Audiobook> = records.into_iter().map(Audiobook::from).collect();
        let connection = Connection::from_items(audiobooks, offset, limit, total);

        Ok(AudiobookConnection::from_connection(connection))
    }

    /// Get a specific audiobook by ID
    async fn audiobook(&self, ctx: &Context<'_>, id: String) -> Result<Option<Audiobook>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let audiobook_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        let record = db
            .audiobooks()
            .get_by_id(audiobook_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(Audiobook::from))
    }

    /// Get an audiobook with all its chapters and status
    async fn audiobook_with_chapters(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<AudiobookWithChapters>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let audiobook_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        let audiobook_record = db
            .audiobooks()
            .get_by_id(audiobook_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let Some(audiobook_record) = audiobook_record else {
            return Ok(None);
        };

        // Fetch author if exists
        let author = if let Some(author_id) = audiobook_record.author_id {
            db.audiobooks()
                .get_author_by_id(author_id)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
                .map(AudiobookAuthor::from)
        } else {
            None
        };

        // Fetch all chapters
        let chapters = db
            .chapters()
            .list_by_audiobook(audiobook_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let chapter_count = chapters.len() as i32;
        let chapters_with_files = chapters
            .iter()
            .filter(|c| c.media_file_id.is_some())
            .count() as i32;
        let missing_chapters = chapter_count - chapters_with_files;
        let completion_percent = if chapter_count > 0 {
            (chapters_with_files as f64 / chapter_count as f64) * 100.0
        } else {
            0.0
        };

        Ok(Some(AudiobookWithChapters {
            audiobook: audiobook_record.into(),
            chapters: chapters.into_iter().map(AudiobookChapter::from).collect(),
            chapter_count,
            chapters_with_files,
            missing_chapters,
            completion_percent,
            author,
        }))
    }

    /// Get all audiobook authors in a library
    async fn audiobook_authors(
        &self,
        ctx: &Context<'_>,
        library_id: String,
    ) -> Result<Vec<AudiobookAuthor>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .audiobooks()
            .list_authors_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(AudiobookAuthor::from).collect())
    }

    /// Get audiobook authors in a library with cursor-based pagination and filtering
    async fn audiobook_authors_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<AudiobookAuthorWhereInput>,
        order_by: Option<AudiobookAuthorOrderByInput>,
    ) -> Result<AudiobookAuthorConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let (offset, limit) =
            parse_pagination_args(first, after).map_err(|e| async_graphql::Error::new(e))?;

        let name_filter = r#where
            .as_ref()
            .and_then(|w| w.name.as_ref().and_then(|f| f.contains.clone()));

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(AudiobookAuthorSortField::Name);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(OrderDirection::Asc);

        let (records, total) = db
            .audiobooks()
            .list_authors_by_library_paginated(
                lib_id,
                offset,
                limit,
                name_filter.as_deref(),
                &audiobook_author_sort_field_to_column(sort_field),
                sort_dir == OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let authors: Vec<AudiobookAuthor> =
            records.into_iter().map(AudiobookAuthor::from).collect();
        let connection = Connection::from_items(authors, offset, limit, total);

        Ok(AudiobookAuthorConnection::from_connection(connection))
    }

    /// Get all chapters for an audiobook
    async fn audiobook_chapters(
        &self,
        ctx: &Context<'_>,
        audiobook_id: String,
    ) -> Result<Vec<AudiobookChapter>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let book_id = Uuid::parse_str(&audiobook_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        let records = db
            .chapters()
            .list_by_audiobook(book_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(AudiobookChapter::from).collect())
    }

    /// Get audiobook chapters with cursor-based pagination and filtering
    async fn audiobook_chapters_connection(
        &self,
        ctx: &Context<'_>,
        audiobook_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<AudiobookChapterWhereInput>,
        order_by: Option<AudiobookChapterOrderByInput>,
    ) -> Result<AudiobookChapterConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let book_id = Uuid::parse_str(&audiobook_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        let (offset, limit) =
            parse_pagination_args(first, after).map_err(|e| async_graphql::Error::new(e))?;

        let status_filter = r#where
            .as_ref()
            .and_then(|w| w.status.as_ref().and_then(|f| f.eq.clone()));

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(AudiobookChapterSortField::ChapterNumber);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(OrderDirection::Asc);

        let (records, total) = db
            .chapters()
            .list_by_audiobook_paginated(
                book_id,
                offset,
                limit,
                status_filter.as_deref(),
                &audiobook_chapter_sort_field_to_column(sort_field),
                sort_dir == OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let chapters: Vec<AudiobookChapter> =
            records.into_iter().map(AudiobookChapter::from).collect();
        let connection = Connection::from_items(chapters, offset, limit, total);

        Ok(AudiobookChapterConnection::from_connection(connection))
    }

    /// Search for audiobooks on OpenLibrary
    async fn search_audiobooks(
        &self,
        ctx: &Context<'_>,
        query: String,
    ) -> Result<Vec<AudiobookSearchResult>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let results = metadata
            .search_audiobooks(&query)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|a| AudiobookSearchResult {
                provider: "openlibrary".to_string(),
                provider_id: a.provider_id,
                title: a.title,
                author_name: a.author_name,
                year: a.year,
                cover_url: a.cover_url,
                isbn: a.isbn,
                description: a.description,
            })
            .collect())
    }
}
