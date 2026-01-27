//! FilesystemChanged subscription: emitted when CreateDirectory, DeleteFiles, CopyFiles, MoveFiles, or RenameFile runs.
//! When a FilesystemChangeBroker is in schema data (e.g. from FilesystemService), mutations broadcast here.

use std::pin::Pin;
use std::sync::Arc;

use async_graphql::{Context, Object, Subscription};
use futures::Stream;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use super::super::auth::AuthUser;

/// Event emitted when a filesystem mutation completes (PascalCase for GraphQL).
#[derive(Clone, async_graphql::SimpleObject)]
#[graphql(name = "FilesystemChangeEvent")]
pub struct FilesystemChangeEvent {
    #[graphql(name = "Path")]
    pub path: String,
    #[graphql(name = "ChangeType")]
    pub change_type: String,
    #[graphql(name = "Name")]
    pub name: Option<String>,
    #[graphql(name = "NewName")]
    pub new_name: Option<String>,
    #[graphql(name = "Timestamp")]
    pub timestamp: String,
}

/// Optional broker in context; when present, mutations can send and subscription receives.
/// Add to schema via .data(FilesystemChangeBroker::new()) and have mutations call send() after success.
pub struct FilesystemChangeBroker {
    sender: broadcast::Sender<FilesystemChangeEvent>,
}

impl FilesystemChangeBroker {
    pub fn new(cap: usize) -> Arc<Self> {
        let (sender, _) = broadcast::channel(cap);
        Arc::new(Self { sender })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<FilesystemChangeEvent> {
        self.sender.subscribe()
    }

    pub fn send(&self, event: FilesystemChangeEvent) {
        let _ = self.sender.send(event);
    }
}

#[derive(Default)]
pub struct FilesystemSubscriptions;

#[Subscription]
impl FilesystemSubscriptions {
    /// Subscribe to filesystem change events (create/delete/copy/move/rename).
    /// Fires when any filesystem mutation completes. Optional path filter.
    #[graphql(name = "FilesystemChanged")]
    async fn filesystem_changed(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Path", desc = "Filter to changes under this path (optional)")] path_filter: Option<String>,
    ) -> Pin<Box<dyn Stream<Item = FilesystemChangeEvent> + Send>> {
        let filter = path_filter;

        if ctx.data_opt::<AuthUser>().is_some() {
            if let Ok(broker) = ctx.data::<Arc<FilesystemChangeBroker>>() {
                let rx = broker.subscribe();
                let stream = BroadcastStream::new(rx)
                    .filter_map(|r| r.ok())
                    .filter_map(move |event| {
                        let pass = match filter.as_ref() {
                            Some(p) => event.path.starts_with(p),
                            None => true,
                        };
                        if pass { Some(event) } else { None }
                    });
                return Box::pin(Box::new(stream) as Box<dyn Stream<Item = FilesystemChangeEvent> + Send + Unpin>);
            }
        }

        Box::pin(Box::new(futures::stream::empty::<FilesystemChangeEvent>()) as Box<dyn Stream<Item = FilesystemChangeEvent> + Send + Unpin>)
    }
}
