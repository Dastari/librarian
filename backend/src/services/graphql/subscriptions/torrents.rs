//! Torrent client subscriptions: live torrent events (progress/add/remove/complete).

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_graphql::{Context, SimpleObject, Subscription};
use futures::{Stream, StreamExt};
use graphql_orm::graphql::orm::{ChangeAction, ChangeKind};
use graphql_orm::graphql::subscriptions::SubscriptionEvent;
use tokio::sync::broadcast;

use crate::db::Database;
use crate::graphql::entities::{Torrent, TorrentChangedEvent};
use crate::services::graphql::entities::torrent::{
    current_torrent_user, owns_runtime_torrent, persisted_runtime_torrent,
};

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

// Reuse the generated, post-commit witness channel. Public/raw changed events
// are deliberately insufficient evidence of a removed row's previous owner.
type TorrentWitness = SubscriptionEvent<TorrentChangedEvent>;
type RuntimeStream = Pin<Box<dyn Stream<Item = TorrentEvent> + Send>>;

fn trusted_deleted_torrent(witness: &TorrentWitness) -> Option<&Torrent> {
    let event = witness.event.as_ref();
    let row = event.entity.as_ref()?;
    (witness.trusted_delete
        && event.action == ChangeAction::Deleted
        && event.change_kind == ChangeKind::Direct
        && event.source_entity.is_none()
        && event.source_id.is_none()
        && event.path.is_empty()
        && event.id == row.id
        && witness.keys == Torrent::__gom_key_values(&row.id))
    .then_some(row)
}

#[derive(Default)]
struct RemovalProvenance {
    // Briefly retain only identities from trusted committed delete preimages.
    // Runtime removal follows the generated delete in TorrentService. Retention
    // handles independently buffered runtime/entity channels, not row authority.
    deleted: HashMap<String, (Torrent, Instant)>,
}

impl RemovalProvenance {
    fn observe(&mut self, witness: &TorrentWitness) {
        self.deleted
            .retain(|_, (_, at)| at.elapsed() < Duration::from_secs(60));
        if let Some(row) = trusted_deleted_torrent(witness) {
            self.deleted
                .insert(row.info_hash.clone(), (row.clone(), Instant::now()));
        } else {
            // Reparent/update invalidates an earlier identity for the same row.
            self.deleted
                .retain(|_, (row, _)| row.id != witness.event.id);
        }
    }

    fn take(&mut self, hash: &str) -> Option<Torrent> {
        self.deleted
            .remove(hash)
            .filter(|(_, at)| at.elapsed() < Duration::from_secs(60))
            .map(|(row, _)| row)
    }
}

fn event_hash(event: &TorrentEvent) -> &str {
    match event {
        TorrentEvent::Added { info_hash, .. }
        | TorrentEvent::Progress { info_hash, .. }
        | TorrentEvent::Completed { info_hash, .. }
        | TorrentEvent::Removed { info_hash, .. }
        | TorrentEvent::Error { info_hash, .. } => info_hash,
    }
}

/// Fresh runtime payloads are released only after a fresh persisted-owner and
/// account check. No global snapshot, ownership cache, or custom bus is used.
fn authorized_runtime_events(
    db: Database,
    authenticated: AuthUser,
    runtime: broadcast::Receiver<TorrentEvent>,
    changes: broadcast::Receiver<TorrentWitness>,
) -> RuntimeStream {
    let state = (
        db,
        authenticated,
        runtime,
        changes,
        RemovalProvenance::default(),
    );
    Box::pin(futures::stream::unfold(
        state,
        |(db, authenticated, mut runtime, mut changes, mut provenance)| async move {
            loop {
                // A lagged channel has lost identity ordering. End this stream so a
                // reconnect starts with a freshly authorized query snapshot.
                let event = runtime.recv().await.ok()?;
                loop {
                    match changes.try_recv() {
                        Ok(witness) => provenance.observe(&witness),
                        Err(broadcast::error::TryRecvError::Empty) => break,
                        Err(_) => return None,
                    }
                }
                let hash = event_hash(&event);
                if matches!(&event, TorrentEvent::Added { .. }) {
                    provenance.deleted.remove(hash);
                }
                let removed = matches!(&event, TorrentEvent::Removed { .. });
                let deleted = if removed { provenance.take(hash) } else { None };
                let Ok(user) = current_torrent_user(&db, &authenticated).await else {
                    return None;
                };
                let Ok(persisted) = persisted_runtime_torrent(&db, hash).await else {
                    return None;
                };
                // Current mapping takes precedence over a delete preimage. A member
                // must never inherit a former owner's access after hash reuse.
                let allowed = if let Some(row) = persisted {
                    owns_runtime_torrent(&user, &row)
                } else if removed {
                    user.is_admin()
                        || deleted
                            .as_ref()
                            .is_some_and(|row| owns_runtime_torrent(&user, row))
                } else {
                    user.is_admin()
                };
                if !allowed {
                    continue;
                }
                // Account/role checks must also follow the awaited mapping lookup.
                let Ok(current) = current_torrent_user(&db, &authenticated).await else {
                    return None;
                };
                if current.parsed_role() != user.parsed_role() {
                    continue;
                }
                return Some((event, (db, authenticated, runtime, changes, provenance)));
            }
        },
    ))
}

async fn runtime_events(ctx: &Context<'_>) -> RuntimeStream {
    let Some(user) = ctx.data_opt::<AuthUser>().cloned() else {
        return Box::pin(futures::stream::empty());
    };
    let Some(db) = ctx.data_opt::<Database>().cloned() else {
        return Box::pin(futures::stream::empty());
    };
    if current_torrent_user(&db, &user).await.is_err() {
        return Box::pin(futures::stream::empty());
    }
    let Some(manager) = ctx.data_opt::<Arc<ServicesManager>>() else {
        return Box::pin(futures::stream::empty());
    };
    let Some(service) = manager.get_torrent().await else {
        return Box::pin(futures::stream::empty());
    };
    // Register the generated witness receiver first: removal commits its delete
    // before publishing the runtime Removed event.
    let changes = db.ensure_event_sender::<TorrentWitness>().subscribe();
    let Some(runtime) = service.subscribe().await else {
        return Box::pin(futures::stream::empty());
    };
    authorized_runtime_events(db, user, runtime, changes)
}

#[derive(Default)]
pub struct TorrentSubscriptions;

#[Subscription(name = "TorrentRuntimeSubscriptions")]
impl TorrentSubscriptions {
    #[graphql(name = "torrentProgress")]
    async fn torrent_progress(
        &self,
        ctx: &Context<'_>,
    ) -> Pin<Box<dyn Stream<Item = TorrentProgress> + Send>> {
        Box::pin(runtime_events(ctx).await.filter_map(|event| async move {
            match event {
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
        }))
    }

    #[graphql(name = "torrentAdded")]
    async fn torrent_added(
        &self,
        ctx: &Context<'_>,
    ) -> Pin<Box<dyn Stream<Item = TorrentAddedEvent> + Send>> {
        Box::pin(runtime_events(ctx).await.filter_map(|event| async move {
            match event {
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
        }))
    }

    #[graphql(name = "torrentCompleted")]
    async fn torrent_completed(
        &self,
        ctx: &Context<'_>,
    ) -> Pin<Box<dyn Stream<Item = TorrentCompletedEvent> + Send>> {
        Box::pin(runtime_events(ctx).await.filter_map(|event| async move {
            match event {
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
        }))
    }

    #[graphql(name = "torrentRemoved")]
    async fn torrent_removed(
        &self,
        ctx: &Context<'_>,
    ) -> Pin<Box<dyn Stream<Item = TorrentRemovedEvent> + Send>> {
        Box::pin(runtime_events(ctx).await.filter_map(|event| async move {
            match event {
                TorrentEvent::Removed { id, info_hash } => Some(TorrentRemovedEvent {
                    id: id as i32,
                    info_hash,
                }),
                _ => None,
            }
        }))
    }
}

#[cfg(test)]
#[path = "torrent_authorization_tests.rs"]
mod authorization_tests;
