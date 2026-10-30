//! Offline data repairs run from the CLI while the server is stopped.
//!
//! Every read and write goes through the typed entity layer, so hooks (file cleanup,
//! summaries) fire exactly as they would for an API mutation.

use std::collections::BTreeMap;

use anyhow::Result;
use graphql_orm::graphql::filters::StringFilter;

use crate::db::Database;
use crate::graphql::entities::*;

/// What `repair duplicate-torrents` found or changed.
#[derive(Debug, Default, PartialEq)]
pub struct DuplicateTorrentReport {
    /// Info hashes held by more than one torrent row.
    pub duplicate_hashes: usize,
    /// Rows merged into a keeper and deleted (or that would be, on a dry run).
    pub removed_rows: usize,
    /// Torrent files moved to, or file matches copied onto, the keeper.
    pub preserved_files: usize,
    /// Hashes shared by different users, which are left for an administrator.
    pub skipped_hashes: Vec<String>,
}

/// Merge torrent rows that share an info hash into one row per hash.
///
/// The keeper is the row furthest along (completed/seeding, then progress, then the one with
/// the most files, then the oldest). Anything the keeper lacks — post-processing results,
/// library and title links, file matches, files it never listed — is copied from the other
/// rows before they are deleted. With `apply == false` nothing is written.
pub async fn repair_duplicate_torrents(
    db: &Database,
    apply: bool,
) -> Result<DuplicateTorrentReport> {
    let mut by_hash: BTreeMap<String, Vec<Torrent>> = BTreeMap::new();
    for torrent in Torrent::query(db.pool()).fetch_all().await? {
        by_hash
            .entry(torrent.info_hash.to_lowercase())
            .or_default()
            .push(torrent);
    }

    let mut report = DuplicateTorrentReport::default();
    for (hash, rows) in by_hash.into_iter().filter(|(_, rows)| rows.len() > 1) {
        report.duplicate_hashes += 1;
        if rows.iter().any(|row| row.user_id != rows[0].user_id) {
            report.skipped_hashes.push(hash);
            continue;
        }

        let mut ranked = Vec::with_capacity(rows.len());
        for row in rows {
            let files = files_of(db, &row.id).await?;
            ranked.push((row, files));
        }
        ranked.sort_by(|(a, a_files), (b, b_files)| {
            rank(b)
                .partial_cmp(&rank(a))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b_files.len().cmp(&a_files.len()))
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        let mut ranked = ranked.into_iter();
        let (mut keeper, mut keeper_files) = ranked.next().expect("group has rows");

        for (other, other_files) in ranked {
            let input = fill_missing(&keeper, &other);
            for file in other_files {
                match keeper_files
                    .iter_mut()
                    .find(|kept| kept.file_index == file.file_index)
                {
                    Some(kept) if kept.media_file_id.is_none() && file.media_file_id.is_some() => {
                        report.preserved_files += 1;
                        if apply {
                            TorrentFile::update_by_id(
                                db,
                                &kept.id,
                                UpdateTorrentFileInput {
                                    media_file_id: Some(file.media_file_id.clone()),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        }
                        kept.media_file_id = file.media_file_id.clone();
                    }
                    Some(_) => {}
                    None => {
                        report.preserved_files += 1;
                        if apply {
                            TorrentFile::update_by_id(
                                db,
                                &file.id,
                                UpdateTorrentFileInput {
                                    torrent_id: Some(keeper.id.clone()),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        }
                        keeper_files.push(file);
                    }
                }
            }

            if apply {
                if let Some(input) = input
                    && let Some(updated) = Torrent::update_by_id(db, &keeper.id, input).await?
                {
                    keeper = updated;
                }
                // The delete hook removes whatever files the row still owns.
                Torrent::delete_by_id(db, &other.id).await?;
            } else if let Some(input) = input {
                apply_locally(&mut keeper, input);
            }
            report.removed_rows += 1;
            tracing::info!(
                info_hash = %hash,
                kept = %keeper.id,
                removed = %other.id,
                apply,
                "Merged duplicate torrent row"
            );
        }
    }
    Ok(report)
}

async fn files_of(db: &Database, torrent_id: &str) -> Result<Vec<TorrentFile>> {
    Ok(TorrentFile::query(db.pool())
        .filter(TorrentFileWhereInput {
            torrent_id: Some(StringFilter {
                eq: Some(torrent_id.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?)
}

fn rank(torrent: &Torrent) -> (bool, f64, bool) {
    let finished = matches!(torrent.state.as_str(), "seeding" | "completed");
    (finished, torrent.progress, torrent.completed_at.is_some())
}

/// The keeper's empty fields that `other` can fill, or `None` when it has nothing to add.
fn fill_missing(keeper: &Torrent, other: &Torrent) -> Option<UpdateTorrentInput> {
    fn take<T: Clone>(
        kept: &Option<T>,
        other: &Option<T>,
        changed: &mut bool,
    ) -> Option<Option<T>> {
        if kept.is_none() && other.is_some() {
            *changed = true;
            Some(other.clone())
        } else {
            None
        }
    }
    let mut changed = false;
    let input = UpdateTorrentInput {
        magnet_uri: take(&keeper.magnet_uri, &other.magnet_uri, &mut changed),
        download_path: take(&keeper.download_path, &other.download_path, &mut changed),
        source_url: take(&keeper.source_url, &other.source_url, &mut changed),
        source_feed_id: take(&keeper.source_feed_id, &other.source_feed_id, &mut changed),
        source_indexer_id: take(
            &keeper.source_indexer_id,
            &other.source_indexer_id,
            &mut changed,
        ),
        library_id: take(&keeper.library_id, &other.library_id, &mut changed),
        post_process_status: take(
            &keeper.post_process_status,
            &other.post_process_status,
            &mut changed,
        ),
        post_process_error: take(
            &keeper.post_process_error,
            &other.post_process_error,
            &mut changed,
        ),
        processed_at: take(&keeper.processed_at, &other.processed_at, &mut changed),
        episode_id: take(&keeper.episode_id, &other.episode_id, &mut changed),
        movie_id: take(&keeper.movie_id, &other.movie_id, &mut changed),
        track_id: take(&keeper.track_id, &other.track_id, &mut changed),
        chapter_id: take(&keeper.chapter_id, &other.chapter_id, &mut changed),
        show_id: take(&keeper.show_id, &other.show_id, &mut changed),
        album_id: take(&keeper.album_id, &other.album_id, &mut changed),
        audiobook_id: take(&keeper.audiobook_id, &other.audiobook_id, &mut changed),
        season: take(&keeper.season, &other.season, &mut changed),
        ..Default::default()
    };
    changed.then_some(input)
}

/// Mirror an update on a dry run so later rows in the group compare against it.
fn apply_locally(keeper: &mut Torrent, input: UpdateTorrentInput) {
    macro_rules! set {
        ($($field:ident),*) => {
            $(if let Some(value) = input.$field { keeper.$field = value; })*
        };
    }
    set!(
        magnet_uri,
        download_path,
        source_url,
        source_feed_id,
        source_indexer_id,
        library_id,
        post_process_status,
        post_process_error,
        processed_at,
        episode_id,
        movie_id,
        track_id,
        chapter_id,
        show_id,
        album_id,
        audiobook_id,
        season
    );
}
