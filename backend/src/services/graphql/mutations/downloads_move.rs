//! Moving the download folder (Q105): a read-only plan, then the move. See
//! `services::torrent::move_downloads`.

use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::services::graphql::auth::AuthExt;
use crate::services::manager::ServicesManager;

#[derive(Clone, SimpleObject)]
#[graphql(name = "DownloadsMovePlan", rename_fields = "camelCase")]
pub struct DownloadsMovePlanResult {
    /// The download folder now.
    pub old_path: String,
    /// The new folder, as it would be saved.
    pub new_path: String,
    /// Torrents stored under the download folder.
    pub torrent_count: i32,
    /// Their files that are on disk now.
    pub file_count: i32,
    /// Found at the new folder with the same size.
    pub matched: i32,
    pub missing: i32,
    /// The first of them, as paths below the download folder.
    pub missing_files: Vec<String>,
    /// At the new folder at another size (a copy that did not finish).
    pub changed: i32,
    pub changed_files: Vec<String>,
    /// Why the move cannot be made at all.
    pub blocked: Option<String>,
    pub can_apply: bool,
}

#[derive(InputObject)]
#[graphql(name = "MoveDownloadsInput", rename_fields = "camelCase")]
pub struct MoveDownloadsInput {
    /// The new folder: an absolute path, or `smb://host/share/folder` on a saved server.
    pub new_path: String,
    /// Point the torrents in the download folder at their copy there (default). Off: only new
    /// downloads go to the new folder, and the torrents stay where they are.
    pub move_torrents: Option<bool>,
    /// Move even though some files are not at the new folder yet. The client downloads them again.
    pub allow_missing: Option<bool>,
}

#[derive(Clone, SimpleObject)]
#[graphql(name = "MoveDownloadsResult", rename_fields = "camelCase")]
pub struct MoveDownloadsResult {
    pub success: bool,
    pub error: Option<String>,
    pub moved_torrents: i32,
    pub moved_files: i32,
}

fn count(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[derive(Default)]
pub struct DownloadsMoveQueries;

#[Object]
impl DownloadsMoveQueries {
    /// Compare the torrents in the download folder with a folder they were copied to, before
    /// moving the download folder there. Read only.
    #[graphql(name = "downloadsMovePlan")]
    async fn downloads_move_plan(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "newPath")] new_path: String,
    ) -> Result<DownloadsMovePlanResult> {
        ctx.require_admin()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let torrent = services
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("The torrent client is not running"))?;
        let plan = torrent
            .downloads_move_plan(&new_path)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        Ok(DownloadsMovePlanResult {
            can_apply: plan.can_apply(),
            old_path: plan.old_path,
            new_path: plan.new_path,
            torrent_count: count(plan.torrent_count),
            file_count: count(plan.file_count),
            matched: count(plan.matched),
            missing: count(plan.missing),
            missing_files: plan.missing_files,
            changed: count(plan.changed),
            changed_files: plan.changed_files,
            blocked: plan.blocked,
        })
    }
}

#[derive(Default)]
pub struct DownloadsMoveMutations;

#[Object]
impl DownloadsMoveMutations {
    /// Move the download folder after its files were copied there: every torrent keeps seeding
    /// from its copy, and nothing is downloaded again. The old files are not deleted. The client
    /// restarts so the new folder is where new downloads go.
    #[graphql(name = "moveDownloads")]
    async fn move_downloads(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: MoveDownloadsInput,
    ) -> Result<MoveDownloadsResult> {
        ctx.require_admin()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let torrent = services
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("The torrent client is not running"))?;
        match torrent
            .move_downloads(
                &input.new_path,
                input.move_torrents.unwrap_or(true),
                input.allow_missing.unwrap_or(false),
            )
            .await
        {
            Ok(done) => Ok(MoveDownloadsResult {
                success: true,
                error: None,
                moved_torrents: count(done.torrents),
                moved_files: count(done.files),
            }),
            Err(error) => Ok(MoveDownloadsResult {
                success: false,
                error: Some(format!("{error:#}")),
                moved_torrents: 0,
                moved_files: 0,
            }),
        }
    }
}
