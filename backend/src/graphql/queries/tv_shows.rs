use super::prelude::*;

#[derive(Default)]
pub struct TvShowQueries;

#[Object]
impl TvShowQueries {
    /// Get all TV shows for the current user (across all libraries)
    async fn all_tv_shows(&self, ctx: &Context<'_>) -> Result<Vec<TvShow>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .tv_shows()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(TvShow::from).collect())
    }

    /// Get all TV shows in a library
    async fn tv_shows(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<TvShow>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .tv_shows()
            .list_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(TvShow::from).collect())
    }

    /// Get TV shows in a library with cursor-based pagination and filtering
    async fn tv_shows_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<TvShowWhereInput>,
        order_by: Option<TvShowOrderByInput>,
    ) -> Result<TvShowConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let (offset, limit) =
            parse_pagination_args(first, after).map_err(|e| async_graphql::Error::new(e))?;

        // Build filter conditions
        let name_filter = r#where
            .as_ref()
            .and_then(|w| w.name.as_ref().and_then(|f| f.contains.clone()));
        let year_filter = r#where
            .as_ref()
            .and_then(|w| w.year.as_ref().and_then(|f| f.eq));
        let monitored_filter = r#where
            .as_ref()
            .and_then(|w| w.monitored.as_ref().and_then(|f| f.eq));
        let status_filter = r#where
            .as_ref()
            .and_then(|w| w.status.as_ref().and_then(|f| f.eq.clone()));

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(TvShowSortField::SortName);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(OrderDirection::Asc);

        let (records, total) = db
            .tv_shows()
            .list_by_library_paginated(
                lib_id,
                offset,
                limit,
                name_filter.as_deref(),
                year_filter,
                monitored_filter,
                status_filter.as_deref(),
                &tv_sort_field_to_column(sort_field),
                sort_dir == OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let shows: Vec<TvShow> = records.into_iter().map(TvShow::from).collect();
        let connection = Connection::from_items(shows, offset, limit, total);

        Ok(TvShowConnection::from_connection(connection))
    }

    /// Get a specific TV show by ID
    async fn tv_show(&self, ctx: &Context<'_>, id: String) -> Result<Option<TvShow>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let show_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid show ID: {}", e)))?;

        let record = db
            .tv_shows()
            .get_by_id(show_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(TvShow::from))
    }

    /// Search for TV shows from metadata providers
    async fn search_tv_shows(
        &self,
        ctx: &Context<'_>,
        query: String,
    ) -> Result<Vec<TvShowSearchResult>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let results = metadata
            .search_shows(&query)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| TvShowSearchResult {
                provider: format!("{:?}", r.provider).to_lowercase(),
                provider_id: r.provider_id as i32,
                name: r.name,
                year: r.year,
                status: r.status,
                network: r.network,
                overview: r.overview,
                poster_url: r.poster_url,
                tvdb_id: r.tvdb_id.map(|id| id as i32),
                imdb_id: r.imdb_id,
                score: r.score,
            })
            .collect())
    }
}
