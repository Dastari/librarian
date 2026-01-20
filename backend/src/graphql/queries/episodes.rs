use super::prelude::*;

#[derive(Default)]
pub struct EpisodeQueries;

#[Object]
impl EpisodeQueries {
    /// Get all episodes for a TV show
    async fn episodes(&self, ctx: &Context<'_>, tv_show_id: String) -> Result<Vec<Episode>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let show_id = Uuid::parse_str(&tv_show_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid show ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .episodes()
            .list_by_show(show_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Batch fetch watch progress for all episodes
        let episode_ids: Vec<Uuid> = records.iter().map(|r| r.id).collect();
        let watch_progress_list = db
            .watch_progress()
            .get_episode_progress_batch(user_id, &episode_ids)
            .await
            .unwrap_or_default();

        // Create a map for quick lookup
        let progress_map: std::collections::HashMap<Uuid, crate::db::WatchProgressRecord> =
            watch_progress_list
                .into_iter()
                .filter_map(|wp| wp.episode_id.map(|eid| (eid, wp)))
                .collect();

        // For downloaded episodes, look up the media file with its metadata
        let mut episodes = Vec::with_capacity(records.len());
        for r in records {
            let episode_id = r.id;
            let media_file = if r.status == "downloaded" {
                // Try to get the media file for this episode (includes metadata from FFmpeg analysis)
                db.media_files()
                    .get_by_episode_id(r.id)
                    .await
                    .ok()
                    .flatten()
            } else {
                None
            };

            // Get watch progress for this episode
            let watch_progress = progress_map.get(&episode_id).cloned();

            episodes.push(Episode::from_record_with_progress(
                r,
                media_file,
                watch_progress,
            ));
        }

        Ok(episodes)
    }

    /// Get wanted (missing) episodes
    async fn wanted_episodes(
        &self,
        ctx: &Context<'_>,
        library_id: Option<String>,
    ) -> Result<Vec<Episode>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let records = if let Some(lib_id) = library_id {
            let lib_uuid = Uuid::parse_str(&lib_id)
                .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
            db.episodes().list_wanted_by_library(lib_uuid).await
        } else {
            let user_id = Uuid::parse_str(&user.user_id)
                .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
            db.episodes().list_wanted_by_user(user_id).await
        }
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| Episode::from_record(r, None)) // Wanted episodes don't have media files yet
            .collect())
    }
}
