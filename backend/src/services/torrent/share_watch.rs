//! Torrents on a share that stops answering wait for it instead of failing (Q100).
//!
//! The share storage reports every call that could not reach the share. The watcher then pauses
//! the torrents stored on that share, so nothing keeps trying (and failing) against it, says why
//! in their status, and checks the share every [`PROBE_EVERY`]. When it answers again the
//! torrents it paused are resumed, and any that librqbit had stopped with an error are started
//! again; librqbit checks what is already on the share, so nothing is downloaded twice and
//! nothing is deleted. Which torrents the watcher paused is kept in an app setting, so a restart
//! during an outage still resumes them, and a torrent a person paused is never resumed by it.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use librqbit::{ManagedTorrent, Session, TorrentStatsState};

type ManagedTorrentHandle = Arc<ManagedTorrent>;
use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::db::Database;
use crate::services::library_storage::{StorageKind, StorageLocation, library_storage};

/// How often a share that is down is tried again.
const PROBE_EVERY: Duration = Duration::from_secs(20);
/// The app setting that lists the torrents the watcher paused.
pub const PAUSED_SETTING: &str = "torrent.share_paused";

#[derive(Clone, Debug)]
struct Outage {
    since: DateTime<Utc>,
    message: String,
}

#[derive(Default)]
struct State {
    /// By share root (`smb://host/share`).
    outages: HashMap<String, Outage>,
    /// Info hashes the watcher paused.
    paused: HashSet<String>,
    loaded: bool,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(State::default()))
}

/// The share root a torrent folder is on, if it is on a share.
pub fn share_of(folder: &str) -> Option<String> {
    let location = StorageLocation::parse(folder).ok()?;
    if location.kind() != StorageKind::Smb {
        return None;
    }
    location
        .root_path()
        .as_smb()
        .map(|smb| smb.share_root().to_uri())
}

/// A call on `folder`'s share could not reach it.
pub fn report_outage(folder: &str, message: &str) {
    let Some(share) = share_of(folder) else {
        return;
    };
    let mut state = state().lock();
    if let std::collections::hash_map::Entry::Vacant(entry) = state.outages.entry(share) {
        warn!(
            share = %entry.key(),
            error = %message,
            "The download share stopped answering; its torrents will wait for it"
        );
        entry.insert(Outage {
            since: Utc::now(),
            message: message.to_string(),
        });
    }
}

/// What to show for a torrent stored in `folder` while its share is down.
pub fn waiting_message(info_hash: &str, folder: &str) -> Option<String> {
    let share = share_of(folder)?;
    let state = state().lock();
    let outage = state.outages.get(&share)?;
    let paused = state.paused.contains(info_hash);
    Some(format!(
        "Waiting for the download share {share}, unreachable since {}{}: {}",
        outage.since.format("%Y-%m-%d %H:%M UTC"),
        if paused {
            ". It resumes by itself when the share answers"
        } else {
            ""
        },
        outage.message
    ))
}

/// What adds a torrent back from its own metadata: its torrent bytes, its folder at `folder`
/// (the same one, or a new one after a move), its wanted files and the given paused state.
pub fn readd_options(
    handle: &ManagedTorrentHandle,
    folder: &std::path::Path,
    paused: bool,
) -> anyhow::Result<(librqbit::AddTorrent<'static>, librqbit::AddTorrentOptions)> {
    use anyhow::Context;
    let metadata = handle
        .metadata
        .load_full()
        .context("the torrent has no metadata yet")?;
    Ok((
        librqbit::AddTorrent::TorrentFileBytes(metadata.torrent_bytes.clone()),
        librqbit::AddTorrentOptions {
            overwrite: true,
            paused,
            only_files: handle.only_files(),
            output_folder: Some(folder.to_string_lossy().into_owned()),
            ..Default::default()
        },
    ))
}

/// Take a torrent out of the session keeping its files, and add it again from its own metadata
/// (`readd_options`). The client then checks every piece on disk (or on the share) again:
/// nothing that is there is downloaded twice. After an outage this clears pieces whose check
/// could not read the share, which the client would otherwise never ask for again.
pub async fn readd_from_metadata(
    session: &Arc<Session>,
    handle: &ManagedTorrentHandle,
    folder: &std::path::Path,
    paused: bool,
) -> anyhow::Result<()> {
    use anyhow::Context;
    let info_hash = hash_of(handle);
    let (add, options) = readd_options(handle, folder, paused)?;
    session
        .delete(librqbit::api::TorrentIdOrHash::Id(handle.id()), false)
        .await
        .with_context(|| format!("could not take {info_hash} out of the client"))?;
    super::share_storage::register_folder(&info_hash, folder);
    session
        .add_torrent(add, Some(options))
        .await
        .with_context(|| format!("could not add {info_hash} back from {}", folder.display()))?;
    Ok(())
}

fn hash_of(handle: &ManagedTorrentHandle) -> String {
    handle
        .info_hash()
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn save_paused(db: &Database, paused: &HashSet<String>) {
    let mut list: Vec<&String> = paused.iter().collect();
    list.sort();
    let value = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
    if let Err(error) = super::database::put_setting_value(
        db,
        PAUSED_SETTING,
        &value,
        "torrent",
        "Torrents paused while their download share was unreachable",
    )
    .await
    {
        warn!(error = %error, "Could not record which torrents wait for their share");
    }
}

async fn load_paused(db: &Database) {
    if state().lock().loaded {
        return;
    }
    let saved = super::database::get_setting::<Vec<String>>(db, PAUSED_SETTING)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut state = state().lock();
    state.paused.extend(saved);
    state.loaded = true;
}

/// Whether the share answers now. Its remembered failure is forgotten first, so this is a real
/// attempt.
async fn share_answers(share: &str) -> bool {
    let Ok(location) = crate::services::library_storage::SmbLocation::parse(share) else {
        return false;
    };
    library_storage().forget_share(&location);
    library_storage()
        .stat(location.storage_path())
        .await
        .is_ok()
}

fn is_share_error(error: &str) -> bool {
    error.contains("download share is unavailable")
}

/// One pass: pause torrents on shares that are down, resume them on shares that came back.
async fn pass(session: &Arc<Session>, db: &Database) {
    load_paused(db).await;
    let handles: Vec<ManagedTorrentHandle> =
        session.with_torrents(|torrents| torrents.map(|(_, handle)| handle.clone()).collect());
    let mut by_share: HashMap<String, Vec<ManagedTorrentHandle>> = HashMap::new();
    for handle in handles {
        if let Some(share) = share_of(&handle.output_folder().to_string_lossy()) {
            // A torrent librqbit stopped because the share failed marks that share as down too.
            let stats = handle.stats();
            if matches!(stats.state, TorrentStatsState::Error)
                && stats.error.as_deref().is_some_and(is_share_error)
            {
                report_outage(
                    &handle.output_folder().to_string_lossy(),
                    stats.error.as_deref().unwrap_or_default(),
                );
            }
            by_share.entry(share).or_default().push(handle);
        }
    }
    let outages: Vec<String> = state().lock().outages.keys().cloned().collect();
    let mut changed = false;
    for share in outages {
        let torrents = by_share.remove(&share).unwrap_or_default();
        if share_answers(&share).await {
            info!(share = %share, "The download share answers again; resuming its torrents");
            let resume: Vec<ManagedTorrentHandle> = {
                let mut state = state().lock();
                state.outages.remove(&share);
                torrents
                    .into_iter()
                    .filter(|handle| {
                        let hash = hash_of(handle);
                        let ours = state.paused.remove(&hash);
                        changed |= ours;
                        ours || matches!(handle.stats().state, TorrentStatsState::Error)
                    })
                    .collect()
            };
            for handle in resume {
                // A finished torrent only seeds, so it carries on. One that was downloading is
                // checked again from the share: a piece whose check failed during the outage would
                // otherwise never be asked for again.
                let finished = handle.stats().finished;
                let resumed = if finished {
                    session.unpause(&handle).await
                } else {
                    let folder = handle.output_folder().to_path_buf();
                    readd_from_metadata(session, &handle, &folder, false).await
                };
                if let Err(error) = resumed {
                    warn!(
                        torrent = %handle.name().unwrap_or_default(),
                        error = %error,
                        "Could not resume a torrent after its share came back"
                    );
                }
            }
            continue;
        }
        // Still down: pause what is running there, once.
        for handle in torrents {
            let stats = handle.stats();
            if matches!(stats.state, TorrentStatsState::Live) && !handle.is_paused() {
                let hash = hash_of(&handle);
                if let Err(error) = session.pause(&handle).await {
                    warn!(
                        torrent = %handle.name().unwrap_or_default(),
                        error = %error,
                        "Could not pause a torrent whose share is down"
                    );
                    continue;
                }
                state().lock().paused.insert(hash);
                changed = true;
            }
        }
    }
    // Torrents the watcher paused before a restart, on shares that answer now.
    let leftover: Vec<String> = state().lock().paused.iter().cloned().collect();
    for hash in leftover {
        let Some(handle) = super::database::session_handle(session, &hash) else {
            continue;
        };
        let folder = handle.output_folder().to_string_lossy().into_owned();
        let Some(share) = share_of(&folder) else {
            continue;
        };
        if state().lock().outages.contains_key(&share) {
            continue;
        }
        if share_answers(&share).await {
            state().lock().paused.remove(&hash);
            changed = true;
            if handle.is_paused()
                && let Err(error) = session.unpause(&handle).await
            {
                warn!(error = %error, "Could not resume a torrent after its share came back");
            }
        } else {
            report_outage(&folder, "the share did not answer when Librarian started");
        }
    }
    if changed {
        let paused = state().lock().paused.clone();
        save_paused(db, &paused).await;
    }
}

/// Watch the shares torrents are stored on until `cancel`.
pub async fn watch_loop(session: Arc<Session>, db: Database, cancel: CancellationToken) {
    loop {
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = tokio::time::sleep(PROBE_EVERY) => {}
        }
        pass(&session, &db).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_torrent_folder_names_its_share() {
        assert_eq!(
            share_of("smb://10.0.1.190/Media/Downloads/Some.Show.S01").as_deref(),
            Some("smb://10.0.1.190/media")
        );
        assert_eq!(share_of("/data/downloads/Some.Show.S01"), None);
    }

    #[test]
    fn an_outage_is_shown_on_its_torrents_only() {
        report_outage("smb://nas-test-outage/dl/A", "connection refused");
        let shown = waiting_message("abc", "smb://nas-test-outage/dl/B").unwrap();
        assert!(shown.contains("smb://nas-test-outage/dl"));
        assert!(shown.contains("connection refused"));
        assert!(waiting_message("abc", "smb://other-test-host/dl/B").is_none());
        state().lock().outages.remove("smb://nas-test-outage/dl");
    }
}
