//! Torrent client subscriptions: live torrent events (progress/add/remove/complete).

use std::pin::Pin;
use std::sync::Arc;

use async_graphql::{Context, SimpleObject, Subscription};
use futures::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::services::manager::ServicesManager;
use crate::services::torrent::TorrentEvent;

use super::super::auth::AuthUser;

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "TorrentProgress")]
pub struct TorrentProgress {
    #[graphql(name = "id")]
    pub id: i32,
    #[graphql(name = "infoHash")]
    pub info_hash: String,
    #[graphql(name = "progress")]
    pub progress: f64,
    #[graphql(name = "downloadSpeed")]
    pub download_speed: i64,
    #[graphql(name = "uploadSpeed")]
    pub upload_speed: i64,
    #[graphql(name = "peers")]
    pub peers: i32,
    #[graphql(name = "state")]
    pub state: String,
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "TorrentAddedEvent")]
pub struct TorrentAddedEvent {
    #[graphql(name = "id")]
    pub id: i32,
    #[graphql(name = "name")]
    pub name: String,
    #[graphql(name = "infoHash")]
    pub info_hash: String,
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "TorrentCompletedEvent")]
pub struct TorrentCompletedEvent {
    #[graphql(name = "id")]
    pub id: i32,
    #[graphql(name = "name")]
    pub name: String,
    #[graphql(name = "infoHash")]
    pub info_hash: String,
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "TorrentRemovedEvent")]
pub struct TorrentRemovedEvent {
    #[graphql(name = "id")]
    pub id: i32,
    #[graphql(name = "infoHash")]
    pub info_hash: String,
}

#[derive(Default)]
pub struct TorrentSubscriptions;

#[Subscription]
impl TorrentSubscriptions {
    #[graphql(name = "torrentProgress")]
    async fn torrent_progress(
        &self,
        ctx: &Context<'_>,
    ) -> Pin<Box<dyn Stream<Item = TorrentProgress> + Send>> {
        if ctx.data_opt::<AuthUser>().is_none() {
            return Box::pin(Box::new(futures::stream::empty::<TorrentProgress>())
                as Box<dyn Stream<Item = TorrentProgress> + Send + Unpin>);
        }

        let manager = match ctx.data::<Arc<ServicesManager>>() {
            Ok(m) => m.clone(),
            Err(_) => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentProgress>())
                    as Box<dyn Stream<Item = TorrentProgress> + Send + Unpin>);
            }
        };

        let service = match manager.get_torrent().await {
            Some(svc) => svc,
            None => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentProgress>())
                    as Box<dyn Stream<Item = TorrentProgress> + Send + Unpin>);
            }
        };

        let rx = match service.subscribe() {
            Some(rx) => rx,
            None => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentProgress>())
                    as Box<dyn Stream<Item = TorrentProgress> + Send + Unpin>);
            }
        };

        let stream = BroadcastStream::new(rx).filter_map(|event| {
            match event.ok()? {
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
                    state: state.to_string(),
                }),
                _ => None,
            }
        });

        Box::pin(Box::new(stream)
            as Box<dyn Stream<Item = TorrentProgress> + Send + Unpin>)
    }

    #[graphql(name = "torrentAdded")]
    async fn torrent_added(
        &self,
        ctx: &Context<'_>,
    ) -> Pin<Box<dyn Stream<Item = TorrentAddedEvent> + Send>> {
        if ctx.data_opt::<AuthUser>().is_none() {
            return Box::pin(Box::new(futures::stream::empty::<TorrentAddedEvent>())
                as Box<dyn Stream<Item = TorrentAddedEvent> + Send + Unpin>);
        }

        let manager = match ctx.data::<Arc<ServicesManager>>() {
            Ok(m) => m.clone(),
            Err(_) => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentAddedEvent>())
                    as Box<dyn Stream<Item = TorrentAddedEvent> + Send + Unpin>);
            }
        };

        let service = match manager.get_torrent().await {
            Some(svc) => svc,
            None => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentAddedEvent>())
                    as Box<dyn Stream<Item = TorrentAddedEvent> + Send + Unpin>);
            }
        };

        let rx = match service.subscribe() {
            Some(rx) => rx,
            None => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentAddedEvent>())
                    as Box<dyn Stream<Item = TorrentAddedEvent> + Send + Unpin>);
            }
        };

        let stream = BroadcastStream::new(rx).filter_map(|event| {
            match event.ok()? {
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
            }
        });

        Box::pin(Box::new(stream)
            as Box<dyn Stream<Item = TorrentAddedEvent> + Send + Unpin>)
    }

    #[graphql(name = "torrentCompleted")]
    async fn torrent_completed(
        &self,
        ctx: &Context<'_>,
    ) -> Pin<Box<dyn Stream<Item = TorrentCompletedEvent> + Send>> {
        if ctx.data_opt::<AuthUser>().is_none() {
            return Box::pin(Box::new(futures::stream::empty::<TorrentCompletedEvent>())
                as Box<dyn Stream<Item = TorrentCompletedEvent> + Send + Unpin>);
        }

        let manager = match ctx.data::<Arc<ServicesManager>>() {
            Ok(m) => m.clone(),
            Err(_) => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentCompletedEvent>())
                    as Box<dyn Stream<Item = TorrentCompletedEvent> + Send + Unpin>);
            }
        };

        let service = match manager.get_torrent().await {
            Some(svc) => svc,
            None => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentCompletedEvent>())
                    as Box<dyn Stream<Item = TorrentCompletedEvent> + Send + Unpin>);
            }
        };

        let rx = match service.subscribe() {
            Some(rx) => rx,
            None => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentCompletedEvent>())
                    as Box<dyn Stream<Item = TorrentCompletedEvent> + Send + Unpin>);
            }
        };

        let stream = BroadcastStream::new(rx).filter_map(|event| {
            match event.ok()? {
                TorrentEvent::Completed {
                    id,
                    name,
                    info_hash,
                } => Some(TorrentCompletedEvent {
                    id: id as i32,
                    name,
                    info_hash,
                }),
                _ => None,
            }
        });

        Box::pin(Box::new(stream)
            as Box<dyn Stream<Item = TorrentCompletedEvent> + Send + Unpin>)
    }

    #[graphql(name = "torrentRemoved")]
    async fn torrent_removed(
        &self,
        ctx: &Context<'_>,
    ) -> Pin<Box<dyn Stream<Item = TorrentRemovedEvent> + Send>> {
        if ctx.data_opt::<AuthUser>().is_none() {
            return Box::pin(Box::new(futures::stream::empty::<TorrentRemovedEvent>())
                as Box<dyn Stream<Item = TorrentRemovedEvent> + Send + Unpin>);
        }

        let manager = match ctx.data::<Arc<ServicesManager>>() {
            Ok(m) => m.clone(),
            Err(_) => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentRemovedEvent>())
                    as Box<dyn Stream<Item = TorrentRemovedEvent> + Send + Unpin>);
            }
        };

        let service = match manager.get_torrent().await {
            Some(svc) => svc,
            None => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentRemovedEvent>())
                    as Box<dyn Stream<Item = TorrentRemovedEvent> + Send + Unpin>);
            }
        };

        let rx = match service.subscribe() {
            Some(rx) => rx,
            None => {
                return Box::pin(Box::new(futures::stream::empty::<TorrentRemovedEvent>())
                    as Box<dyn Stream<Item = TorrentRemovedEvent> + Send + Unpin>);
            }
        };

        let stream = BroadcastStream::new(rx).filter_map(|event| {
            match event.ok()? {
                TorrentEvent::Removed { id, info_hash } => Some(TorrentRemovedEvent {
                    id: id as i32,
                    info_hash,
                }),
                _ => None,
            }
        });

        Box::pin(Box::new(stream)
            as Box<dyn Stream<Item = TorrentRemovedEvent> + Send + Unpin>)
    }
}
