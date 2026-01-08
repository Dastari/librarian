//! GraphQL subscriptions for real-time updates
//!
//! Subscriptions allow clients to receive push updates over WebSocket.

use std::sync::Arc;

use async_graphql::{Context, Subscription};
use futures::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::services::{TorrentEvent, TorrentService};

use super::types::{
    TorrentAddedEvent, TorrentCompletedEvent, TorrentProgress, TorrentRemovedEvent, TorrentState,
};

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to all torrent events (progress, added, completed, removed)
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
}
