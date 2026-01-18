//! GraphQL subscriptions for real-time updates
//!
//! Subscriptions allow clients to receive push updates over WebSocket.
//! All subscriptions require authentication via the `AuthGuard`.

use std::sync::Arc;

use async_graphql::{Context, Subscription};
use futures::Stream;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::services::{
    CastDevicesEvent, CastService, CastSessionEvent, DirectoryChangeEvent as ServiceDirectoryChangeEvent,
    FilesystemService, LogEvent, TorrentEvent, TorrentService,
};

use super::auth::AuthGuard;
use super::types::{
    CastDevice, CastPlayerState, CastSession, DirectoryChangeEvent, LibraryChangedEvent,
    LogEventSubscription, LogLevel, TorrentAddedEvent, TorrentCompletedEvent, TorrentProgress,
    TorrentRemovedEvent, TorrentState,
};

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to all torrent events (progress, added, completed, removed)
    #[graphql(guard = "AuthGuard")]
    async fn torrent_progress<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> impl Stream<Item = TorrentProgress> + 'ctx {
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();
        let receiver = torrent_service.subscribe();

        BroadcastStream::new(receiver).filter_map(|result| {
            result.ok().and_then(|event| match event {
                TorrentEvent::Progress {
                    id,
                    info_hash,
                    progress,
                    download_speed,
                    upload_speed,
                    peers,
                    state,
                } => Some(TorrentProgress {
                    id: id as i32,
                    info_hash,
                    progress,
                    download_speed: download_speed as i64,
                    upload_speed: upload_speed as i64,
                    peers: peers as i32,
                    state: match state {
                        crate::services::TorrentState::Queued => TorrentState::Queued,
                        crate::services::TorrentState::Checking => TorrentState::Checking,
                        crate::services::TorrentState::Downloading => TorrentState::Downloading,
                        crate::services::TorrentState::Seeding => TorrentState::Seeding,
                        crate::services::TorrentState::Paused => TorrentState::Paused,
                        crate::services::TorrentState::Error => TorrentState::Error,
                    },
                }),
                _ => None,
            })
        })
    }

    /// Subscribe to torrent added events
    #[graphql(guard = "AuthGuard")]
    async fn torrent_added<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> impl Stream<Item = TorrentAddedEvent> + 'ctx {
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();
        let receiver = torrent_service.subscribe();

        BroadcastStream::new(receiver).filter_map(|result| {
            result.ok().and_then(|event| match event {
                TorrentEvent::Added {
                    id,
                    name,
                    info_hash,
                } => Some(TorrentAddedEvent {
                    id: id as i32,
                    name,
                    info_hash,
                }),
                _ => None,
            })
        })
    }

    /// Subscribe to torrent completion events
    #[graphql(guard = "AuthGuard")]
    async fn torrent_completed<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> impl Stream<Item = TorrentCompletedEvent> + 'ctx {
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();
        let receiver = torrent_service.subscribe();

        BroadcastStream::new(receiver).filter_map(|result| {
            result.ok().and_then(|event| match event {
                TorrentEvent::Completed {
                    id,
                    info_hash,
                    name,
                } => Some(TorrentCompletedEvent {
                    id: id as i32,
                    name,
                    info_hash,
                }),
                _ => None,
            })
        })
    }

    /// Subscribe to torrent removal events
    #[graphql(guard = "AuthGuard")]
    async fn torrent_removed<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> impl Stream<Item = TorrentRemovedEvent> + 'ctx {
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();
        let receiver = torrent_service.subscribe();

        BroadcastStream::new(receiver).filter_map(|result| {
            result.ok().and_then(|event| match event {
                TorrentEvent::Removed { id, info_hash } => Some(TorrentRemovedEvent {
                    id: id as i32,
                    info_hash,
                }),
                _ => None,
            })
        })
    }

    /// Subscribe to progress updates for a specific torrent
    #[graphql(guard = "AuthGuard")]
    async fn torrent_progress_by_id<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        id: i32,
    ) -> impl Stream<Item = TorrentProgress> + 'ctx {
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();
        let receiver = torrent_service.subscribe();

        BroadcastStream::new(receiver).filter_map(move |result| {
            result.ok().and_then(|event| match event {
                TorrentEvent::Progress {
                    id: event_id,
                    info_hash,
                    progress,
                    download_speed,
                    upload_speed,
                    peers,
                    state,
                } if event_id as i32 == id => Some(TorrentProgress {
                    id: event_id as i32,
                    info_hash,
                    progress,
                    download_speed: download_speed as i64,
                    upload_speed: upload_speed as i64,
                    peers: peers as i32,
                    state: match state {
                        crate::services::TorrentState::Queued => TorrentState::Queued,
                        crate::services::TorrentState::Checking => TorrentState::Checking,
                        crate::services::TorrentState::Downloading => TorrentState::Downloading,
                        crate::services::TorrentState::Seeding => TorrentState::Seeding,
                        crate::services::TorrentState::Paused => TorrentState::Paused,
                        crate::services::TorrentState::Error => TorrentState::Error,
                    },
                }),
                _ => None,
            })
        })
    }

    /// Subscribe to all log events (for real-time log viewing)
    #[graphql(guard = "AuthGuard")]
    async fn log_events<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        #[graphql(desc = "Only receive logs of these levels (defaults to all)")] levels: Option<
            Vec<LogLevel>,
        >,
    ) -> impl Stream<Item = LogEventSubscription> + 'ctx {
        let receiver = ctx
            .data_unchecked::<broadcast::Sender<LogEvent>>()
            .subscribe();

        let level_filter: Option<Vec<String>> = levels.map(|ls| {
            ls.into_iter()
                .map(|l| match l {
                    LogLevel::Trace => "TRACE".to_string(),
                    LogLevel::Debug => "DEBUG".to_string(),
                    LogLevel::Info => "INFO".to_string(),
                    LogLevel::Warn => "WARN".to_string(),
                    LogLevel::Error => "ERROR".to_string(),
                })
                .collect()
        });

        BroadcastStream::new(receiver).filter_map(move |result| {
            result.ok().and_then(|event| {
                // Filter by level if specified
                if let Some(ref levels) = level_filter
                    && !levels.contains(&event.level)
                {
                    return None;
                }

                Some(LogEventSubscription {
                    timestamp: event.timestamp,
                    level: LogLevel::from(event.level.as_str()),
                    target: event.target,
                    message: event.message,
                    fields: event.fields,
                    span_name: event.span_name,
                })
            })
        })
    }

    /// Subscribe to error logs only (for toast notifications)
    #[graphql(guard = "AuthGuard")]
    async fn error_logs<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> impl Stream<Item = LogEventSubscription> + 'ctx {
        let receiver = ctx
            .data_unchecked::<broadcast::Sender<LogEvent>>()
            .subscribe();

        BroadcastStream::new(receiver).filter_map(|result| {
            result.ok().and_then(|event| {
                // Only return ERROR level logs
                if event.level != "ERROR" {
                    return None;
                }

                Some(LogEventSubscription {
                    timestamp: event.timestamp,
                    level: LogLevel::Error,
                    target: event.target,
                    message: event.message,
                    fields: event.fields,
                    span_name: event.span_name,
                })
            })
        })
    }

    // ------------------------------------------------------------------------
    // Cast Subscriptions
    // ------------------------------------------------------------------------

    /// Subscribe to cast session updates (playback state, position, volume)
    #[graphql(guard = "AuthGuard")]
    async fn cast_session_updated<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        #[graphql(desc = "Filter to a specific session ID")] session_id: Option<String>,
    ) -> impl Stream<Item = CastSession> + 'ctx {
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();
        let receiver = cast_service.subscribe_sessions();

        let filter_id = session_id.and_then(|id| uuid::Uuid::parse_str(&id).ok());

        BroadcastStream::new(receiver).filter_map(move |result| {
            result.ok().and_then(|event: CastSessionEvent| {
                // Filter by session_id if provided
                if let Some(filter) = filter_id {
                    if event.session_id != filter {
                        return None;
                    }
                }

                Some(CastSession {
                    id: event.session_id.to_string(),
                    device_id: Some(event.device_id.to_string()),
                    device_name: None, // Would need async lookup
                    media_file_id: None,
                    episode_id: None,
                    stream_url: String::new(),
                    player_state: match event.player_state {
                        crate::services::CastPlayerState::Idle => CastPlayerState::Idle,
                        crate::services::CastPlayerState::Buffering => CastPlayerState::Buffering,
                        crate::services::CastPlayerState::Playing => CastPlayerState::Playing,
                        crate::services::CastPlayerState::Paused => CastPlayerState::Paused,
                    },
                    current_position: event.current_position,
                    duration: event.duration,
                    volume: event.volume,
                    is_muted: event.is_muted,
                    started_at: String::new(),
                })
            })
        })
    }

    /// Subscribe to cast device availability changes
    #[graphql(guard = "AuthGuard")]
    async fn cast_devices_changed<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> impl Stream<Item = Vec<CastDevice>> + 'ctx {
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();
        let receiver = cast_service.subscribe_devices();

        BroadcastStream::new(receiver).filter_map(|result| {
            result.ok().map(|event: CastDevicesEvent| {
                event
                    .devices
                    .into_iter()
                    .map(|d| CastDevice::from_record(d, false))
                    .collect()
            })
        })
    }

    // ------------------------------------------------------------------------
    // Library Subscriptions
    // ------------------------------------------------------------------------

    /// Subscribe to library changes (created, updated, deleted)
    ///
    /// Receives events when libraries are created, modified, or deleted.
    #[graphql(guard = "AuthGuard")]
    async fn library_changed<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> impl Stream<Item = LibraryChangedEvent> + 'ctx {
        let receiver = ctx
            .data_unchecked::<broadcast::Sender<LibraryChangedEvent>>()
            .subscribe();

        BroadcastStream::new(receiver).filter_map(|result| result.ok())
    }

    // ------------------------------------------------------------------------
    // Filesystem Subscriptions
    // ------------------------------------------------------------------------

    /// Subscribe to directory content changes
    ///
    /// Receives events when files/directories are created, modified, deleted, or renamed.
    /// Optionally filter to changes in a specific directory path.
    #[graphql(guard = "AuthGuard")]
    async fn directory_contents_changed<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        #[graphql(desc = "Filter to changes in this directory path (optional)")] path: Option<
            String,
        >,
    ) -> impl Stream<Item = DirectoryChangeEvent> + 'ctx {
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let receiver = fs_service.subscribe();

        let filter_path = path;

        BroadcastStream::new(receiver).filter_map(move |result| {
            result.ok().and_then(|event: ServiceDirectoryChangeEvent| {
                // Filter by path if specified
                if let Some(ref filter) = filter_path {
                    if !event.path.starts_with(filter) {
                        return None;
                    }
                }

                Some(DirectoryChangeEvent {
                    path: event.path,
                    change_type: event.change_type,
                    name: event.name,
                    new_name: event.new_name,
                    timestamp: event.timestamp.to_rfc3339(),
                })
            })
        })
    }
}
