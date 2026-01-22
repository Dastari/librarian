//! GraphQL schema definition with queries, mutations, and subscriptions
//!
//! This is the single API surface for the Librarian backend.
//! All operations require authentication unless explicitly noted.

use std::sync::Arc;

use async_graphql::{MergedObject, Schema};

use crate::db::Database;
use crate::graphql::mutations;
use crate::graphql::queries;
use crate::graphql::types::{LibraryChangedEvent, MediaFileUpdatedEvent};
use crate::services::{
    AuthService, CastService, FilesystemService, LogEvent, MetadataService, NotificationService,
    ScannerService, TorrentService,
};

use super::subscriptions::SubscriptionRoot;

/// The GraphQL schema type
pub type LibrarianSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Build the GraphQL schema with all resolvers
pub fn build_schema(
    torrent_service: Arc<TorrentService>,
    metadata_service: Arc<MetadataService>,
    scanner_service: Arc<ScannerService>,
    cast_service: Arc<CastService>,
    filesystem_service: Arc<FilesystemService>,
    notification_service: Arc<NotificationService>,
    auth_service: Arc<AuthService>,
    db: Database,
    analysis_queue: Arc<crate::services::MediaAnalysisQueue>,
    log_broadcast: Option<tokio::sync::broadcast::Sender<LogEvent>>,
    library_broadcast: Option<tokio::sync::broadcast::Sender<LibraryChangedEvent>>,
    media_file_broadcast: Option<tokio::sync::broadcast::Sender<MediaFileUpdatedEvent>>,
) -> LibrarianSchema {
    // Create library events broadcast channel (use provided or create new)
    let library_tx = library_broadcast
        .unwrap_or_else(|| tokio::sync::broadcast::channel::<LibraryChangedEvent>(100).0);

    // Create media file updated broadcast channel (use provided or create new)
    let media_file_tx = media_file_broadcast
        .unwrap_or_else(|| tokio::sync::broadcast::channel::<MediaFileUpdatedEvent>(100).0);

    let mut schema = Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        SubscriptionRoot,
    )
    .data(torrent_service)
    .data(metadata_service)
    .data(scanner_service)
    .data(cast_service)
    .data(filesystem_service)
    .data(notification_service)
    .data(auth_service)
    .data(db)
    .data(analysis_queue)
    .data(library_tx)
    .data(media_file_tx);

    // Add log broadcast sender if provided
    if let Some(sender) = log_broadcast {
        schema = schema.data(sender);
    }

    schema.finish()
}

#[derive(MergedObject, Default)]
pub struct QueryRoot(
    queries::UserQueries,
    queries::LibraryQueries,
    queries::TvShowQueries,
    queries::MediaFileQueries,
    queries::MovieQueries,
    queries::MusicQueries,
    queries::AudiobookQueries,
    queries::EpisodeQueries,
    queries::RssFeedQueries,
    queries::MediaQueries,
    queries::PlaybackQueries,
    queries::TorrentQueries,
    queries::SettingsQueries,
    queries::LogQueries,
    queries::UpcomingQueries,
    queries::IndexerQueries,
    queries::FilesystemQueries,
    queries::SystemQueries,
    queries::PriorityRuleQueries,
    queries::UsenetQueries,
    queries::NotificationQueries,
);

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    mutations::AuthMutations,
    mutations::UserMutations,
    mutations::LibraryMutations,
    mutations::MediaFileMutations,
    mutations::TvShowMutations,
    mutations::MovieMutations,
    mutations::MusicMutations,
    mutations::AudiobookMutations,
    mutations::RssFeedMutations,
    mutations::PlaybackMutations,
    mutations::TorrentMutations,
    mutations::SettingsMutations,
    mutations::LogMutations,
    mutations::IndexerMutations,
    mutations::FilesystemMutations,
    mutations::PriorityRuleMutations,
    mutations::UsenetMutations,
    mutations::NotificationMutations,
);
