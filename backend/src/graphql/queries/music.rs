use super::prelude::*;

#[derive(Default)]
pub struct MusicQueries;

#[Object]
impl MusicQueries {
    /// Get all albums in a library
    async fn albums(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<Album>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .albums()
            .list_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Album::from).collect())
    }

    /// Get albums in a library with cursor-based pagination and filtering
    async fn albums_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<AlbumWhereInput>,
        order_by: Option<AlbumOrderByInput>,
    ) -> Result<AlbumConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let (offset, limit) =
            parse_pagination_args(first, after).map_err(|e| async_graphql::Error::new(e))?;

        let name_filter = r#where
            .as_ref()
            .and_then(|w| w.name.as_ref().and_then(|f| f.contains.clone()));
        let year_filter = r#where
            .as_ref()
            .and_then(|w| w.year.as_ref().and_then(|f| f.eq));
        let has_files_filter = r#where
            .as_ref()
            .and_then(|w| w.has_files.as_ref().and_then(|f| f.eq));

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(AlbumSortField::Name);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(OrderDirection::Asc);

        let (records, total) = db
            .albums()
            .list_by_library_paginated(
                lib_id,
                offset,
                limit,
                name_filter.as_deref(),
                year_filter,
                has_files_filter,
                &album_sort_field_to_column(sort_field),
                sort_dir == OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let albums: Vec<Album> = records.into_iter().map(Album::from).collect();
        let connection = Connection::from_items(albums, offset, limit, total);

        Ok(AlbumConnection::from_connection(connection))
    }

    /// Get a specific album by ID
    async fn album(&self, ctx: &Context<'_>, id: String) -> Result<Option<Album>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let album_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid album ID: {}", e)))?;

        let record = db
            .albums()
            .get_by_id(album_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(Album::from))
    }

    /// Get all artists in a library
    async fn artists(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<Artist>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .albums()
            .list_artists_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Artist::from).collect())
    }

    /// Get artists in a library with cursor-based pagination and filtering
    async fn artists_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<ArtistWhereInput>,
        order_by: Option<ArtistOrderByInput>,
    ) -> Result<ArtistConnection> {
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
            .unwrap_or(ArtistSortField::Name);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(OrderDirection::Asc);

        let (records, total) = db
            .albums()
            .list_artists_by_library_paginated(
                lib_id,
                offset,
                limit,
                name_filter.as_deref(),
                &artist_sort_field_to_column(sort_field),
                sort_dir == OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let artists: Vec<Artist> = records.into_iter().map(Artist::from).collect();
        let connection = Connection::from_items(artists, offset, limit, total);

        Ok(ArtistConnection::from_connection(connection))
    }

    /// Get an album with all its tracks and file status
    async fn album_with_tracks(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<AlbumWithTracks>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let album_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid album ID: {}", e)))?;

        let album_record = db
            .albums()
            .get_by_id(album_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let Some(album_record) = album_record else {
            return Ok(None);
        };

        // Fetch artist name
        let artist_name = db
            .albums()
            .get_artist_by_id(album_record.artist_id)
            .await
            .ok()
            .flatten()
            .map(|a| a.name);

        let tracks_with_status = db
            .tracks()
            .list_with_status(album_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let track_count = tracks_with_status.len() as i32;
        let tracks_with_files = tracks_with_status.iter().filter(|t| t.has_file).count() as i32;
        let missing_tracks = track_count - tracks_with_files;
        let completion_percent = if track_count > 0 {
            (tracks_with_files as f64 / track_count as f64) * 100.0
        } else {
            0.0
        };

        // Convert tracks and populate download_progress for "downloading" status
        let mut tracks: Vec<crate::graphql::types::TrackWithStatus> = tracks_with_status.into_iter().map(|t| t.into()).collect();
        let torrent_files_repo = db.torrent_files();
        for track_status in &mut tracks {
            if track_status.track.status == "downloading" {
                if let Ok(track_id) = Uuid::parse_str(&track_status.track.id) {
                    if let Ok(Some(progress)) = torrent_files_repo
                        .get_download_progress_for_track(track_id)
                        .await
                    {
                        track_status.track.download_progress = Some(progress);
                    }
                }
            }
        }

        Ok(Some(AlbumWithTracks {
            album: album_record.into(),
            artist_name,
            tracks,
            track_count,
            tracks_with_files,
            missing_tracks,
            completion_percent,
        }))
    }

    /// Get tracks for an album
    async fn tracks(&self, ctx: &Context<'_>, album_id: String) -> Result<Vec<Track>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let album_uuid = Uuid::parse_str(&album_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid album ID: {}", e)))?;

        let records = db
            .tracks()
            .list_by_album(album_uuid)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut tracks: Vec<Track> = records.into_iter().map(Track::from).collect();

        // Populate download_progress for tracks with status "downloading"
        let torrent_files_repo = db.torrent_files();
        for track in &mut tracks {
            if track.status == "downloading" {
                if let Ok(track_id) = Uuid::parse_str(&track.id) {
                    if let Ok(Some(progress)) = torrent_files_repo
                        .get_download_progress_for_track(track_id)
                        .await
                    {
                        track.download_progress = Some(progress);
                    }
                }
            }
        }

        Ok(tracks)
    }

    /// Get tracks in a library with cursor-based pagination and filtering
    async fn tracks_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<TrackWhereInput>,
        order_by: Option<TrackOrderByInput>,
    ) -> Result<TrackConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let (offset, limit) =
            parse_pagination_args(first, after).map_err(|e| async_graphql::Error::new(e))?;

        let title_filter = r#where
            .as_ref()
            .and_then(|w| w.title.as_ref().and_then(|f| f.contains.clone()));
        let has_file_filter = r#where
            .as_ref()
            .and_then(|w| w.has_file.as_ref().and_then(|f| f.eq));

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(TrackSortField::Title);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(OrderDirection::Asc);

        let (records, total) = db
            .tracks()
            .list_by_library_paginated(
                lib_id,
                offset,
                limit,
                title_filter.as_deref(),
                has_file_filter,
                &track_sort_field_to_column(sort_field),
                sort_dir == OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let tracks: Vec<Track> = records.into_iter().map(Track::from).collect();

        let connection = Connection::from_items(tracks, offset, limit, total);

        Ok(TrackConnection::from_connection(connection))
    }

    /// Search for albums on MusicBrainz
    ///
    /// By default, only searches for Albums. Use the include flags to also search for:
    /// - include_eps: Include EPs
    /// - include_singles: Include Singles  
    /// - include_compilations: Include Compilations
    /// - include_live: Include Live albums
    /// - include_soundtracks: Include Soundtracks
    async fn search_albums(
        &self,
        ctx: &Context<'_>,
        query: String,
        #[graphql(default = false)] include_eps: bool,
        #[graphql(default = false)] include_singles: bool,
        #[graphql(default = false)] include_compilations: bool,
        #[graphql(default = false)] include_live: bool,
        #[graphql(default = false)] include_soundtracks: bool,
    ) -> Result<Vec<AlbumSearchResult>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        // Build list of types to include
        let mut types = vec!["Album".to_string()];
        if include_eps {
            types.push("EP".to_string());
        }
        if include_singles {
            types.push("Single".to_string());
        }
        if include_compilations {
            types.push("Compilation".to_string());
        }
        if include_live {
            types.push("Live".to_string());
        }
        if include_soundtracks {
            types.push("Soundtrack".to_string());
        }

        let results = metadata
            .search_albums_with_types(&query, &types)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|a| AlbumSearchResult {
                provider: "musicbrainz".to_string(),
                provider_id: a.provider_id.to_string(),
                title: a.title,
                artist_name: a.artist_name,
                year: a.year,
                album_type: a.album_type,
                cover_url: a.cover_url,
                score: a.score,
            })
            .collect())
    }
}
