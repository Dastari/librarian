//! What Librarian itself is changing in a library right now, so a folder watch does not take its
//! own imports, organize renames, trash moves and folder tidying for someone else's changes and
//! scan them in a loop (`docs/design.md` Q116).
//!
//! Every change made through [`LibraryStorage`](super::LibraryStorage) is noted here, as are the
//! local-only placements in the scanner. A path is "own" while the change is in flight and for
//! [`SETTLE`] after it ends; a change notification for that path, or for a folder above it (a
//! folder's own time changes when something in it does), is then ignored.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::StoragePath;

/// How long after one of Librarian's own changes ends its notifications are still expected. A
/// share reports changes a moment late, and a debounced watch acts on them later still.
pub const SETTLE: Duration = Duration::from_secs(30);

#[derive(Default)]
struct Entry {
    in_flight: usize,
    ended: Option<Instant>,
}

fn registry() -> &'static Mutex<HashMap<String, Entry>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Entry>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn key(path: &StoragePath) -> String {
    path.as_str().trim_end_matches('/').to_string()
}

/// Held while Librarian changes some paths; dropping it starts their settling time.
#[must_use = "the paths count as Librarian's own only while the guard is held, and a while after"]
pub struct OwnWrite {
    keys: Vec<String>,
}

/// Note that Librarian is about to change these paths (the source and target of a move, a file
/// it writes or removes, a folder it makes or removes).
pub fn own_write<'a>(paths: impl IntoIterator<Item = &'a StoragePath>) -> OwnWrite {
    let keys: Vec<String> = paths.into_iter().map(key).collect();
    if let Ok(mut entries) = registry().lock() {
        let now = Instant::now();
        // Forget what has long settled, so the map stays small.
        entries.retain(|_, entry| {
            entry.in_flight > 0 || entry.ended.is_some_and(|ended| now - ended < SETTLE * 4)
        });
        for key in &keys {
            entries.entry(key.clone()).or_default().in_flight += 1;
        }
    }
    OwnWrite { keys }
}

impl Drop for OwnWrite {
    fn drop(&mut self) {
        if let Ok(mut entries) = registry().lock() {
            let now = Instant::now();
            for key in &self.keys {
                if let Some(entry) = entries.get_mut(key) {
                    entry.in_flight = entry.in_flight.saturating_sub(1);
                    entry.ended = Some(now);
                }
            }
        }
    }
}

/// Whether a change at `path` is one of Librarian's own: the path itself, or a folder holding
/// one, is being changed by Librarian or was within [`SETTLE`].
pub fn is_own_write(path: &StoragePath) -> bool {
    let wanted = key(path);
    let Ok(entries) = registry().lock() else {
        return false;
    };
    let now = Instant::now();
    entries.iter().any(|(own, entry)| {
        let current = entry.in_flight > 0 || entry.ended.is_some_and(|ended| now - ended < SETTLE);
        current
            && (own == &wanted
                || own
                    .strip_prefix(wanted.as_str())
                    .is_some_and(|rest| rest.starts_with('/')))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_and_the_folders_above_it_are_own_while_written() {
        let file = StoragePath::new("smb://h/s/own-writes-test/Show/Season 1/e.mkv");
        let folder = StoragePath::new("smb://h/s/own-writes-test/Show");
        let sibling = StoragePath::new("smb://h/s/own-writes-test/Show 2");
        let below = StoragePath::new("smb://h/s/own-writes-test/Show/Season 1/e.mkv/x");
        assert!(!is_own_write(&file));
        let guard = own_write([&file]);
        assert!(is_own_write(&file));
        assert!(is_own_write(&folder));
        assert!(
            !is_own_write(&sibling),
            "a name that only starts the same is not above it"
        );
        assert!(!is_own_write(&below));
        drop(guard);
        assert!(is_own_write(&file), "still settling after the write");
    }
}
