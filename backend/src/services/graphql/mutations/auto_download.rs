//! Manual "search now" entrypoints for the auto-download hunt
//! (`jobs::auto_download`/`jobs::download_monitor`):
//!
//! - `triggerAutoDownload(libraryId)` runs a whole-library pass.
//! - `searchMissing(input)` runs the hunt for one show/season/movie/album/
//!   audiobook right now, ignoring both the enabled gate and the per-target
//!   search backoff.
//!
//! This always runs one auto-download pass immediately, regardless of whether
//! the background timer loop is enabled. It is the safe way for an admin to
//! exercise candidate discovery + grab on a server where the automatic loop
//! is left at its default-disabled setting (see
//! `docs/tier1-features-plan.md` §1 and `jobs::download_monitor`'s module docs
//! for the safety rationale).

use async_graphql::{Context, InputObject, Object, SimpleObject};

use crate::jobs::auto_download::HuntScope;
use crate::services::ServicesManager;
use crate::services::graphql::auth::AuthExt;

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "TriggerAutoDownloadResult")]
#[graphql(rename_fields = "camelCase")]
pub struct TriggerAutoDownloadResult {
    pub success: bool,
    pub candidates_considered: i32,
    pub searched: i32,
    pub grabbed: i32,
    pub errors: Vec<String>,
    pub error: Option<String>,
}

/// Scope for `searchMissing`. At least one field must be set; a `season`
/// without a `showId` is ignored.
#[derive(Debug, Clone, InputObject)]
#[graphql(name = "SearchMissingInput")]
#[graphql(rename_fields = "camelCase")]
pub struct SearchMissingInput {
    pub library_id: Option<String>,
    pub show_id: Option<String>,
    pub season: Option<i32>,
    pub movie_id: Option<String>,
    pub album_id: Option<String>,
    pub audiobook_id: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "SearchMissingPayload")]
#[graphql(rename_fields = "camelCase")]
pub struct SearchMissingPayload {
    pub success: bool,
    pub error: Option<String>,
    /// Releases grabbed (queued with the download client) by this search.
    pub queued: i32,
    /// Targets actually searched for.
    pub searched: i32,
}

#[derive(Default)]
pub struct AutoDownloadMutations;

#[Object]
impl AutoDownloadMutations {
    /// Run one auto-download pass (candidate discovery + search + grab) right
    /// now, optionally scoped to a single library. Admin-only.
    #[graphql(name = "triggerAutoDownload")]
    async fn trigger_auto_download(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "libraryId")] library_id: Option<String>,
    ) -> async_graphql::Result<TriggerAutoDownloadResult> {
        ctx.require_admin()?;
        let manager = ctx.data_unchecked::<std::sync::Arc<ServicesManager>>();
        let service = manager
            .get_auto_download()
            .await
            .ok_or_else(|| async_graphql::Error::new("Auto-download service not available"))?;

        match service.trigger_now(library_id).await {
            Ok(summary) => Ok(TriggerAutoDownloadResult {
                success: true,
                candidates_considered: summary.candidates_considered as i32,
                searched: summary.searched as i32,
                grabbed: summary.grabbed as i32,
                errors: summary.errors,
                error: None,
            }),
            Err(e) => Ok(TriggerAutoDownloadResult {
                success: false,
                candidates_considered: 0,
                searched: 0,
                grabbed: 0,
                errors: vec![],
                error: Some(e.to_string()),
            }),
        }
    }

    /// Hunt for the missing children of one target right now. Unlike
    /// `triggerAutoDownload` this ignores the per-target search backoff, so an
    /// admin pressing "search" always gets a real search. Admin-only.
    #[graphql(name = "searchMissing")]
    async fn search_missing(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: SearchMissingInput,
    ) -> async_graphql::Result<SearchMissingPayload> {
        ctx.require_admin()?;

        let scope = HuntScope {
            library_id: input.library_id,
            show_id: input.show_id,
            season: input.season,
            movie_id: input.movie_id,
            album_id: input.album_id,
            audiobook_id: input.audiobook_id,
            ignore_backoff: true,
        };
        if !scope.has_target() {
            return Ok(SearchMissingPayload {
                success: false,
                error: Some(
                    "At least one of libraryId, showId, movieId, albumId or audiobookId is required"
                        .to_string(),
                ),
                queued: 0,
                searched: 0,
            });
        }

        let manager = ctx.data_unchecked::<std::sync::Arc<ServicesManager>>();
        let service = manager
            .get_auto_download()
            .await
            .ok_or_else(|| async_graphql::Error::new("Auto-download service not available"))?;

        match service.search_missing(scope).await {
            Ok(summary) => Ok(SearchMissingPayload {
                success: true,
                error: None,
                queued: summary.grabbed as i32,
                searched: summary.searched as i32,
            }),
            Err(e) => Ok(SearchMissingPayload {
                success: false,
                error: Some(e.to_string()),
                queued: 0,
                searched: 0,
            }),
        }
    }
}
