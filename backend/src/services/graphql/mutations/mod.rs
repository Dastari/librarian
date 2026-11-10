//! GraphQL mutation modules (auth, filesystem, etc.).

pub mod artwork;
pub mod auth;
pub mod auto_download;
pub mod backup;
pub mod devices;
pub mod downloads_move;
pub mod feeds;
pub mod filesystem;
pub mod library_scan;
pub mod network_servers;
pub mod notifications;
pub mod quality;
pub mod recommendations;
pub mod server_update;
pub mod subtitles;

pub use artwork::ArtworkMutations;
pub use auth::AuthMutations;
pub use auto_download::AutoDownloadMutations;
pub use backup::BackupMutations;
pub use devices::DeviceActionMutations;
pub use downloads_move::{DownloadsMoveMutations, DownloadsMoveQueries};
pub use feeds::FeedMutations;
pub use filesystem::FilesystemMutations;
pub use library_scan::LibraryScanMutations;
pub use network_servers::{ServerConnectionMutations, ServerConnectionQueries};
pub use notifications::NotificationFeedMutations;
pub use quality::QualityMutations;
pub use recommendations::RecommendationMutations;
pub use server_update::{ServerUpdateMutations, ServerUpdateQueries};
pub use subtitles::SubtitleActionMutations;

#[cfg(test)]
mod auth_session_tests;

pub mod maintenance;
#[cfg(test)]
mod media_file_authorization_tests;
#[cfg(test)]
mod notification_tests;
pub use maintenance::MaintenanceMutations;

/// The media file, if the caller may reach it through the generated row policy. Hand-written
/// mutations on a file run with system authority below this point, so each must call this first.
pub(crate) async fn authorized_media_file(
    ctx: &async_graphql::Context<'_>,
    media_file_id: &str,
) -> async_graphql::Result<crate::services::graphql::entities::MediaFile> {
    use crate::services::graphql::entities::MediaFile;
    use graphql_orm::graphql::orm::{Entity, EntityAccessKind, EntityAccessSurface};

    let denied = || async_graphql::Error::new("Media file not found or access denied");
    let db = ctx.data::<crate::db::Database>()?;
    let policy_key = <MediaFile as Entity>::metadata().read_policy;
    db.ensure_entity_access(
        Some(ctx),
        "MediaFile",
        policy_key,
        EntityAccessKind::Read,
        EntityAccessSurface::GraphqlMutation,
    )
    .await?;
    let file = MediaFile::get(db.pool(), &media_file_id.to_string())
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(denied)?;
    let policy = db
        .row_policy()
        .ok_or_else(|| async_graphql::Error::new("Media file access policy unavailable"))?;
    if !policy
        .can_read_row(
            Some(ctx),
            db,
            "MediaFile",
            policy_key,
            EntityAccessSurface::GraphqlMutation,
            &file,
        )
        .await?
    {
        return Err(denied());
    }
    Ok(file)
}
