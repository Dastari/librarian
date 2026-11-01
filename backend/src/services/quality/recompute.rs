//! Keeping `MediaFile.quality_status` current when something it depends on changes.
//!
//! The status is computed from the file's analysed properties against its resolved
//! profile (title override, then library, then the default). The `AppMutationHook`
//! calls into here after commit when a profile is edited or deleted, when a library
//! or title is assigned a different profile, and when a file is (re)analysed or
//! re-linked. Only analysed files are evaluated: an unanalysed file has no verified
//! properties to judge, so its status stays unknown until analysis runs.
//!
//! The scope helpers read files through the typed entity queries; the single write is
//! `profile::recompute_and_persist`, which skips an unchanged status so a bulk pass
//! over settled files writes (and emits) nothing.

use anyhow::{Context, Result};
use graphql_orm::graphql::filters::{DateFilter, StringFilter};

use super::profile;
use crate::db::Database;
use crate::graphql::entities::{
    Chapter, ChapterWhereInput, Episode, EpisodeWhereInput, MediaFile, MediaFileWhereInput, Track,
    TrackWhereInput,
};

/// Ids per `IN (...)` filter, well under SQLite's bound-variable limit.
const ID_CHUNK: usize = 200;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RecomputeSummary {
    pub evaluated: usize,
    pub failed: usize,
}

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

fn string_in(values: &[String]) -> StringFilter {
    StringFilter {
        in_list: Some(values.to_vec()),
        ..Default::default()
    }
}

fn analysed() -> DateFilter {
    DateFilter {
        is_null: Some(false),
        ..Default::default()
    }
}

/// Re-evaluate the given files. A file that fails (deleted meanwhile, no profile)
/// is logged and skipped; the rest still run.
pub async fn recompute_files(db: &Database, file_ids: &[String]) -> RecomputeSummary {
    let mut summary = RecomputeSummary::default();
    for id in file_ids {
        match profile::recompute_and_persist(db, id).await {
            Ok(_) => summary.evaluated += 1,
            Err(error) => {
                summary.failed += 1;
                tracing::warn!(
                    media_file_id = %id,
                    error = %error,
                    "Quality status recompute failed for media file"
                );
            }
        }
    }
    summary
}

async fn analysed_file_ids(db: &Database, filter: MediaFileWhereInput) -> Result<Vec<String>> {
    let files = MediaFile::query(db.pool())
        .filter(MediaFileWhereInput {
            analyzed_at: Some(analysed()),
            ..filter
        })
        .fetch_all()
        .await
        .context("failed to query media files for quality recompute")?;
    Ok(files.into_iter().map(|file| file.id).collect())
}

/// Every analysed file in the libraries, one library at a time.
pub async fn recompute_libraries(db: &Database, library_ids: &[String]) -> RecomputeSummary {
    let mut total = RecomputeSummary::default();
    for library_id in library_ids {
        let ids = match analysed_file_ids(
            db,
            MediaFileWhereInput {
                library_id: Some(string_eq(library_id)),
                ..Default::default()
            },
        )
        .await
        {
            Ok(ids) => ids,
            Err(error) => {
                tracing::warn!(library_id = %library_id, error = %error, "Quality recompute could not list library files");
                continue;
            }
        };
        let summary = recompute_files(db, &ids).await;
        total.evaluated += summary.evaluated;
        total.failed += summary.failed;
    }
    total
}

/// The kinds of title that can carry their own profile override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleKind {
    Movie,
    Show,
    Album,
    Audiobook,
}

/// Analysed files belonging to one title: its own file for a movie, its episodes'
/// files for a show, and likewise tracks (album) and chapters (audiobook).
pub async fn title_file_ids(db: &Database, kind: TitleKind, id: &str) -> Result<Vec<String>> {
    match kind {
        TitleKind::Movie => {
            analysed_file_ids(
                db,
                MediaFileWhereInput {
                    movie_id: Some(string_eq(id)),
                    ..Default::default()
                },
            )
            .await
        }
        TitleKind::Show => {
            let children: Vec<String> = Episode::query(db.pool())
                .filter(EpisodeWhereInput {
                    show_id: Some(string_eq(id)),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .context("failed to query episodes")?
                .into_iter()
                .map(|row| row.id)
                .collect();
            files_linked_to(db, &children, |ids| MediaFileWhereInput {
                episode_id: Some(string_in(ids)),
                ..Default::default()
            })
            .await
        }
        TitleKind::Album => {
            let children: Vec<String> = Track::query(db.pool())
                .filter(TrackWhereInput {
                    album_id: Some(string_eq(id)),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .context("failed to query tracks")?
                .into_iter()
                .map(|row| row.id)
                .collect();
            files_linked_to(db, &children, |ids| MediaFileWhereInput {
                track_id: Some(string_in(ids)),
                ..Default::default()
            })
            .await
        }
        TitleKind::Audiobook => {
            let children: Vec<String> = Chapter::query(db.pool())
                .filter(ChapterWhereInput {
                    audiobook_id: Some(string_eq(id)),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .context("failed to query chapters")?
                .into_iter()
                .map(|row| row.id)
                .collect();
            files_linked_to(db, &children, |ids| MediaFileWhereInput {
                chapter_id: Some(string_in(ids)),
                ..Default::default()
            })
            .await
        }
    }
}

async fn files_linked_to(
    db: &Database,
    children: &[String],
    filter: impl Fn(&[String]) -> MediaFileWhereInput,
) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    for chunk in children.chunks(ID_CHUNK) {
        ids.extend(analysed_file_ids(db, filter(chunk)).await?);
    }
    Ok(ids)
}

pub async fn recompute_title(db: &Database, kind: TitleKind, id: &str) -> RecomputeSummary {
    match title_file_ids(db, kind, id).await {
        Ok(ids) => recompute_files(db, &ids).await,
        Err(error) => {
            tracing::warn!(title_id = %id, error = %error, "Quality recompute could not list title files");
            RecomputeSummary::default()
        }
    }
}
