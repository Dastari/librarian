use super::prelude::*;

#[derive(Default)]
pub struct UpcomingQueries;

#[Object]
impl UpcomingQueries {
    /// Get upcoming TV episodes from TVMaze for the next N days
    ///
    /// This fetches the global TV schedule from TVMaze, showing what's airing
    /// on broadcast/cable networks. Use country filter to narrow results.
    async fn upcoming_episodes(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 7, desc = "Number of days to look ahead")] days: i32,
        #[graphql(desc = "Country code filter (e.g., 'US', 'GB')")] country: Option<String>,
    ) -> Result<Vec<UpcomingEpisode>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let schedule = metadata
            .get_upcoming_schedule(days as u32, country.as_deref())
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(schedule
            .into_iter()
            .filter_map(|entry| {
                // Skip entries without season/episode numbers (specials)
                let season = entry.season?;
                let episode = entry.number?;

                Some(UpcomingEpisode {
                    tvmaze_id: entry.id as i32,
                    name: entry.name,
                    season: season as i32,
                    episode: episode as i32,
                    air_date: entry.airdate.unwrap_or_default(),
                    air_time: entry.airtime,
                    air_stamp: entry.air_stamp,
                    runtime: entry.runtime.map(|r| r as i32),
                    summary: entry.summary.map(|s| {
                        // Strip HTML tags from summary
                        let re = regex::Regex::new(r"<[^>]+>").unwrap();
                        re.replace_all(&s, "").trim().to_string()
                    }),
                    episode_image_url: entry.image.as_ref().and_then(|i| i.medium.clone()),
                    show: UpcomingEpisodeShow {
                        tvmaze_id: entry.show.id as i32,
                        name: entry.show.name,
                        network: entry
                            .show
                            .network
                            .as_ref()
                            .map(|n| n.name.clone())
                            .or_else(|| entry.show.web_channel.as_ref().map(|w| w.name.clone())),
                        poster_url: entry.show.image.as_ref().and_then(|i| i.medium.clone()),
                        genres: entry.show.genres,
                    },
                })
            })
            .collect())
    }

    /// Get upcoming episodes from the user's libraries
    ///
    /// Returns episodes from shows in the user's TV libraries that are
    /// airing in the next N days. Only includes monitored shows.
    async fn library_upcoming_episodes(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 7, desc = "Number of days to look ahead")] days: i32,
    ) -> Result<Vec<LibraryUpcomingEpisode>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .episodes()
            .list_upcoming_by_user(user_id, days)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| LibraryUpcomingEpisode {
                id: r.id.to_string(),
                tvmaze_id: r.episode_tvmaze_id,
                name: r.episode_title,
                season: r.season,
                episode: r.episode,
                air_date: r.air_date.map(|d| d.to_string()).unwrap_or_default(),
                status: match r.status.as_str() {
                    "missing" => EpisodeStatus::Missing,
                    "wanted" => EpisodeStatus::Wanted,
                    "available" => EpisodeStatus::Available,
                    "downloading" => EpisodeStatus::Downloading,
                    "downloaded" => EpisodeStatus::Downloaded,
                    "ignored" => EpisodeStatus::Ignored,
                    _ => EpisodeStatus::Missing,
                },
                show: LibraryUpcomingShow {
                    id: r.show_id.to_string(),
                    name: r.show_name,
                    year: r.show_year,
                    network: r.show_network,
                    poster_url: r.show_poster_url,
                    library_id: r.library_id.to_string(),
                },
            })
            .collect())
    }
}
