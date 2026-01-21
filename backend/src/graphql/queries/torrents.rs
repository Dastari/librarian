use super::prelude::*;

#[derive(Default)]
pub struct TorrentQueries;

#[Object]
impl TorrentQueries {
    /// Get all torrents
    async fn torrents(&self, ctx: &Context<'_>) -> Result<Vec<Torrent>> {
        let user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();
        let db = ctx.data_unchecked::<Database>();

        // Get live torrents from the service
        let torrents = service.list_torrents().await;
        let mut result: Vec<Torrent> = torrents.into_iter().map(|t| t.into()).collect();

        // Try to get added_at timestamps from database
        let user_uuid = Uuid::parse_str(&user.user_id).unwrap_or_default();
        if let Ok(records) = db.torrents().list_by_user(user_uuid).await {
            // Create a map of info_hash -> added_at
            let added_at_map: HashMap<String, String> = records
                .into_iter()
                .filter_map(|r| {
                    r.added_at
                        .format(&time::format_description::well_known::Rfc3339)
                        .ok()
                        .map(|ts| (r.info_hash, ts))
                })
                .collect();

            // Merge added_at into the result
            for torrent in &mut result {
                if let Some(added_at) = added_at_map.get(&torrent.info_hash) {
                    torrent.added_at = Some(added_at.clone());
                }
            }
        }

        Ok(result)
    }

    /// Get a specific torrent by ID
    async fn torrent(&self, ctx: &Context<'_>, id: i32) -> Result<Option<Torrent>> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();
        match service.get_torrent_info(id as usize).await {
            Ok(info) => Ok(Some(info.into())),
            Err(_) => Ok(None),
        }
    }

    /// Get detailed information about a torrent (for info modal)
    async fn torrent_details(&self, ctx: &Context<'_>, id: i32) -> Result<Option<TorrentDetails>> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();
        match service.get_torrent_details(id as usize).await {
            Ok(details) => Ok(Some(details.into())),
            Err(_) => Ok(None),
        }
    }

    /// Get file matches for a torrent
    ///
    /// Returns the list of files in the torrent and what library items they match to.
    /// Accepts either a database UUID or an info_hash.
    async fn torrent_file_matches(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Torrent ID (UUID) or info_hash")] id: String,
    ) -> Result<Vec<TorrentFileMatch>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        // Try to parse as UUID first, then fall back to info_hash lookup
        let torrent_id = if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
            uuid
        } else {
            // Look up torrent by info_hash
            let torrent = db
                .torrents()
                .get_by_info_hash(&id)
                .await?
                .ok_or_else(|| async_graphql::Error::new("Torrent not found"))?;
            torrent.id
        };

        let records = db
            .torrent_file_matches()
            .list_by_torrent(torrent_id)
            .await?;

        Ok(records
            .into_iter()
            .map(TorrentFileMatch::from_record)
            .collect())
    }

    /// Get the count of active downloads
    ///
    /// Returns the number of torrents in QUEUED, CHECKING, or DOWNLOADING state.
    /// Use this to initialize the navbar badge before subscribing to updates.
    async fn active_download_count(&self, ctx: &Context<'_>) -> Result<i32> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();

        let torrents = service.list_torrents().await;
        let count = torrents
            .iter()
            .filter(|t| {
                matches!(
                    t.state,
                    crate::services::TorrentState::Queued
                        | crate::services::TorrentState::Checking
                        | crate::services::TorrentState::Downloading
                )
            })
            .count();

        Ok(count as i32)
    }
}
