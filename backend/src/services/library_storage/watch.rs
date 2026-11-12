//! Being told when something under a folder changes, the same way for every storage
//! (`docs/design.md` Q115): inotify (through the `notify` crate) for a folder on this machine,
//! SMB2 CHANGE_NOTIFY with WATCH_TREE for a folder on a share.
//!
//! A watch hands out [`WatchSignal`]s. It says what changed, by path below the watched folder; it
//! never says what to do about it: the library watch in the scanner decides that.

use tokio::sync::mpsc;

use super::StorageError;

/// What happened to a path, as the storage reported it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WatchChangeKind {
    Added,
    Removed,
    Modified,
    /// The old name of something renamed.
    RenamedFrom,
    /// The new name of something renamed.
    RenamedTo,
}

/// One changed path, relative to the watched folder, with `/` between names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchChange {
    pub path: String,
    pub kind: WatchChangeKind,
}

#[derive(Debug)]
pub enum WatchSignal {
    Changed(Vec<WatchChange>),
    /// More changed than the storage could tell one by one (the SMB server's buffer filled, or the
    /// kernel's queue overflowed): anything under the folder may have changed.
    Overflow,
    /// The watch stopped (the share stopped answering, or the folder went away). Nothing more
    /// comes from this watch; open a new one.
    Lost(StorageError),
}

/// A folder being watched. Dropping it stops the watch.
pub struct FolderWatch {
    signals: mpsc::Receiver<WatchSignal>,
    _keep: Box<dyn Send + Sync>,
}

impl FolderWatch {
    pub(super) fn new(signals: mpsc::Receiver<WatchSignal>, keep: Box<dyn Send + Sync>) -> Self {
        Self {
            signals,
            _keep: keep,
        }
    }

    /// The next signal. `None` means the watch has ended without saying why, which is treated
    /// like [`WatchSignal::Lost`].
    pub async fn next(&mut self) -> Option<WatchSignal> {
        self.signals.recv().await
    }
}

/// Stops a spawned task when dropped.
pub(super) struct AbortOnDrop(pub tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Signals queued before a slow consumer is told it missed some ([`WatchSignal::Overflow`]).
pub(super) const SIGNAL_QUEUE: usize = 256;
