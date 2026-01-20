use super::prelude::*;

#[derive(Default)]
pub struct RssFeedQueries;

#[Object]
impl RssFeedQueries {
    /// Get all RSS feeds for the current user
    async fn rss_feeds(
        &self,
        ctx: &Context<'_>,
        library_id: Option<String>,
    ) -> Result<Vec<RssFeed>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let records = if let Some(lib_id) = library_id {
            let lib_uuid = Uuid::parse_str(&lib_id)
                .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
            db.rss_feeds().list_by_library(lib_uuid).await
        } else {
            let user_id = Uuid::parse_str(&user.user_id)
                .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
            db.rss_feeds().list_by_user(user_id).await
        }
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| RssFeed {
                id: r.id.to_string(),
                library_id: r.library_id.map(|id| id.to_string()),
                name: r.name,
                url: r.url,
                enabled: r.enabled,
                poll_interval_minutes: r.poll_interval_minutes,
                last_polled_at: r.last_polled_at.map(|t| t.to_rfc3339()),
                last_successful_at: r.last_successful_at.map(|t| t.to_rfc3339()),
                last_error: r.last_error,
                consecutive_failures: r.consecutive_failures.unwrap_or(0),
            })
            .collect())
    }

    /// Parse a filename and identify the media
    async fn parse_and_identify_media(
        &self,
        ctx: &Context<'_>,
        title: String,
    ) -> Result<ParseAndIdentifyMediaResult> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let result = metadata
            .parse_and_identify(&title)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ParseAndIdentifyMediaResult {
            parsed: ParsedEpisodeInfo {
                original_title: result.parsed.original_title,
                show_name: result.parsed.show_name,
                season: result.parsed.season.map(|s| s as i32),
                episode: result.parsed.episode.map(|e| e as i32),
                year: result.parsed.year.map(|y| y as i32),
                date: result.parsed.date,
                resolution: result.parsed.resolution,
                source: result.parsed.source,
                codec: result.parsed.codec,
                hdr: result.parsed.hdr,
                audio: result.parsed.audio,
                release_group: result.parsed.release_group,
                is_proper: result.parsed.is_proper,
                is_repack: result.parsed.is_repack,
            },
            matches: result
                .matches
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
                .collect(),
        })
    }
}
