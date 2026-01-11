//! GraphQL subscriptions for real-time updates
//!
//! Subscriptions allow clients to receive push updates over WebSocket.
//! All subscriptions require authentication via the `AuthGuard`.

use std::sync::Arc;

use async_graphql::{Context, Subscription};
use futures::Stream;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::services::{LogEvent, TorrentEvent, TorrentService};

use super::auth::AuthGuard;
use super::types::{
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
                TorrentEvent::Added { id, name, info_hash } => Some(TorrentAddedEvent {
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
                TorrentEvent::Completed { id, info_hash, name } => Some(TorrentCompletedEvent {
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
        #[graphql(desc = "Only receive logs of these levels (defaults to all)")] levels: Option<Vec<LogLevel>>,
    ) -> impl Stream<Item = LogEventSubscription> + 'ctx {
        let receiver = ctx.data_unchecked::<broadcast::Sender<LogEvent>>().subscribe();

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
                    && !levels.contains(&event.level) {
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
        let receiver = ctx.data_unchecked::<broadcast::Sender<LogEvent>>().subscribe();

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
}
