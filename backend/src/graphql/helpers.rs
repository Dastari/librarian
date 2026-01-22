// Helper functions shared across GraphQL query/mutation modules.

use crate::db::MovieRecord;
use crate::graphql::types::{
    AlbumSortField, ArtistSortField, AudiobookAuthorSortField, AudiobookChapterSortField,
    AudiobookSortField, DownloadStatus, Movie, MovieSortField, MovieStatus, TvShowSortField,
};

/// Convert a MovieRecord from the database to a GraphQL Movie type
pub(crate) fn movie_record_to_graphql(r: MovieRecord) -> Movie {
    // Compute download_status from media_file_id presence
    let download_status = if r.media_file_id.is_some() {
        DownloadStatus::Downloaded
    } else if r.monitored {
        DownloadStatus::Wanted
    } else {
        DownloadStatus::Missing
    };

    Movie {
        id: r.id.to_string(),
        library_id: r.library_id.to_string(),
        title: r.title,
        sort_title: r.sort_title,
        original_title: r.original_title,
        year: r.year,
        tmdb_id: r.tmdb_id,
        imdb_id: r.imdb_id,
        status: r
            .status
            .as_deref()
            .map(MovieStatus::from)
            .unwrap_or_default(),
        overview: r.overview,
        tagline: r.tagline,
        runtime: r.runtime,
        genres: r.genres,
        director: r.director,
        cast_names: r.cast_names,
        poster_url: r.poster_url,
        backdrop_url: r.backdrop_url,
        monitored: r.monitored,
        media_file_id: r.media_file_id.map(|id| id.to_string()),
        download_status,
        collection_id: r.collection_id,
        collection_name: r.collection_name,
        collection_poster_url: r.collection_poster_url,
        tmdb_rating: r
            .tmdb_rating
            .and_then(|d| d.to_string().parse::<f64>().ok()),
        tmdb_vote_count: r.tmdb_vote_count,
        certification: r.certification,
        release_date: r.release_date.map(|d| d.to_string()),
        download_progress: None, // Populated by resolvers when download_status is 'downloading'
    }
}

/// Convert MovieSortField enum to database column name
pub(crate) fn sort_field_to_column(field: MovieSortField) -> String {
    match field {
        MovieSortField::Title => "title".to_string(),
        MovieSortField::SortTitle => "sort_title".to_string(),
        MovieSortField::Year => "year".to_string(),
        MovieSortField::CreatedAt => "created_at".to_string(),
        MovieSortField::ReleaseDate => "release_date".to_string(),
    }
}

/// Convert TvShowSortField enum to database column name
pub(crate) fn tv_sort_field_to_column(field: TvShowSortField) -> String {
    match field {
        TvShowSortField::Name => "name".to_string(),
        TvShowSortField::SortName => "sort_name".to_string(),
        TvShowSortField::Year => "year".to_string(),
        TvShowSortField::CreatedAt => "created_at".to_string(),
    }
}

/// Convert AlbumSortField enum to database column name
pub(crate) fn album_sort_field_to_column(field: AlbumSortField) -> String {
    match field {
        AlbumSortField::Name => "name".to_string(),
        AlbumSortField::SortName => "sort_name".to_string(),
        AlbumSortField::Year => "year".to_string(),
        AlbumSortField::CreatedAt => "created_at".to_string(),
        AlbumSortField::Artist => "artist_id".to_string(),
    }
}

/// Convert ArtistSortField enum to database column name
pub(crate) fn artist_sort_field_to_column(field: ArtistSortField) -> String {
    match field {
        ArtistSortField::Name => "name".to_string(),
        ArtistSortField::SortName => "sort_name".to_string(),
    }
}

/// Convert AudiobookSortField enum to database column name
pub(crate) fn audiobook_sort_field_to_column(field: AudiobookSortField) -> String {
    match field {
        AudiobookSortField::Title => "title".to_string(),
        AudiobookSortField::SortTitle => "sort_title".to_string(),
        AudiobookSortField::CreatedAt => "created_at".to_string(),
    }
}

/// Convert AudiobookAuthorSortField enum to database column name
pub(crate) fn audiobook_author_sort_field_to_column(field: AudiobookAuthorSortField) -> String {
    match field {
        AudiobookAuthorSortField::Name => "name".to_string(),
        AudiobookAuthorSortField::SortName => "sort_name".to_string(),
    }
}

/// Convert AudiobookChapterSortField enum to database column name
pub(crate) fn audiobook_chapter_sort_field_to_column(field: AudiobookChapterSortField) -> String {
    match field {
        AudiobookChapterSortField::ChapterNumber => "chapter_number".to_string(),
        AudiobookChapterSortField::Title => "title".to_string(),
        AudiobookChapterSortField::CreatedAt => "created_at".to_string(),
    }
}

/// Download a .torrent file from a private tracker with authentication
pub(crate) async fn download_torrent_file_authenticated(
    url: &str,
    indexer_type: &str,
    credentials: &std::collections::HashMap<String, String>,
) -> anyhow::Result<Vec<u8>> {
    use anyhow::Context;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let mut request = client.get(url);

    // Add authentication based on indexer type
    match indexer_type {
        "iptorrents" => {
            // IPTorrents uses cookie-based authentication
            if let Some(cookie) = credentials.get("cookie") {
                request = request.header("Cookie", cookie);
            }
            if let Some(user_agent) = credentials.get("user_agent") {
                request = request.header("User-Agent", user_agent);
            } else {
                // Default user agent if not provided
                request = request.header(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                );
            }
        }
        _ => {
            // Generic: try cookie auth if available
            if let Some(cookie) = credentials.get("cookie") {
                request = request.header("Cookie", cookie);
            }
            if let Some(api_key) = credentials.get("api_key") {
                // Some indexers use API key as query param
                request = request.query(&[("apikey", api_key)]);
            }
        }
    }

    let response = request.send().await.context("Failed to send request")?;

    let status = response.status();
    if !status.is_success() {
        // Try to get more details from the response body
        let body = response.bytes().await.unwrap_or_default();
        let preview = String::from_utf8_lossy(&body[..std::cmp::min(200, body.len())]);
        anyhow::bail!(
            "Failed to download .torrent file: HTTP {} - {}",
            status,
            preview.chars().take(100).collect::<String>()
        );
    }

    // Check content-type header for clues (must be done before consuming response)
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let bytes = response
        .bytes()
        .await
        .context("Failed to read response body")?;

    tracing::debug!(
        url = %url,
        content_type = %content_type,
        size = bytes.len(),
        "Downloaded torrent file"
    );

    // Verify it's actually a torrent file (starts with "d" for bencoded dict)
    if bytes.is_empty() {
        anyhow::bail!("Downloaded empty file from tracker");
    }

    if bytes[0] != b'd' {
        // Check if it might be an HTML error page
        let preview = String::from_utf8_lossy(&bytes[..std::cmp::min(500, bytes.len())]);
        if preview.contains("<!DOCTYPE") || preview.contains("<html") {
            // Try to extract a meaningful error from the HTML
            if preview.to_lowercase().contains("login") || preview.to_lowercase().contains("sign in") {
                anyhow::bail!("Tracker returned login page - session cookie may have expired. Please update your indexer credentials.");
            }
            if preview.to_lowercase().contains("error") {
                anyhow::bail!("Tracker returned an error page. The download link may have expired or be invalid.");
            }
            anyhow::bail!("Received HTML instead of torrent file (content-type: {}) - authentication may have failed", content_type);
        }
        anyhow::bail!(
            "Downloaded file does not appear to be a valid torrent (first byte: 0x{:02x}, content-type: {}, size: {} bytes)",
            bytes[0],
            content_type,
            bytes.len()
        );
    }

    Ok(bytes.to_vec())
}
