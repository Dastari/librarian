//! Common Enums for Media Entities
//!
//! This module contains shared enums used across multiple media entity types
//! (Show, Movie, Album, Audiobook).

use anyhow::{Context, Result};
use async_graphql::Enum;
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::db::Database;
use crate::graphql::entities::{
    LibraryScanIssue, LibraryScanIssueWhereInput, MediaFile, PendingFileMatch,
    PendingFileMatchWhereInput, PlaybackSession, PlaybackSessionWhereInput,
};

/// Content status for playable media items (episodes, movies, tracks, chapters)
#[derive(Enum, Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
#[graphql(name = "ContentStatus")]
pub enum ContentStatus {
    /// Content is currently being played
    #[graphql(name = "PLAYING")]
    Playing,
    /// Content playback is paused
    #[graphql(name = "PAUSED")]
    Paused,
    /// The file is being analyzed or otherwise prepared
    #[graphql(name = "PROCESSING")]
    Processing,
    /// Content file is available (has media file)
    #[graphql(name = "AVAILABLE")]
    Available,
    /// Content is available, but its quality profile marks it suboptimal
    #[graphql(name = "UPGRADABLE")]
    Upgradable,
    /// Content is currently downloading
    #[graphql(name = "DOWNLOADING")]
    Downloading,
    /// The latest processing attempt failed and needs intervention
    #[graphql(name = "FAILED")]
    Failed,
    /// Content is explicitly ignored
    #[graphql(name = "IGNORED")]
    Ignored,
    /// Content is wanted, but its release date is in the future
    #[graphql(name = "UPCOMING")]
    Upcoming,
    /// Content is wanted but not yet downloaded
    #[graphql(name = "WANTED")]
    Wanted,
    /// Content is missing (no file, not wanted)
    #[graphql(name = "MISSING")]
    Missing,
}

/// Content type for status calculation.
#[derive(Enum, Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
#[graphql(name = "ContentStatusType")]
pub enum ContentType {
    #[graphql(name = "EPISODE")]
    Episode,
    #[graphql(name = "MOVIE")]
    Movie,
    #[graphql(name = "TRACK")]
    Track,
    #[graphql(name = "CHAPTER")]
    Chapter,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct ContentStatusInputs {
    pub playing: bool,
    pub paused: bool,
    pub processing: bool,
    pub pending_download: bool,
    pub has_file: bool,
    pub quality_suboptimal: bool,
    pub failed: bool,
    pub ignored: bool,
    pub release_in_future: bool,
    pub wanted: bool,
}

/// Reduce independent runtime facts to one authoritative display status.
///
/// Precedence reflects observable reality: active playback and work take
/// priority over stored intent; an available file remains visible even when it
/// is not wanted; ignore/upcoming/wanted apply only once no file/work exists.
pub fn reduce_content_status(input: ContentStatusInputs) -> ContentStatus {
    if input.playing {
        ContentStatus::Playing
    } else if input.paused {
        ContentStatus::Paused
    } else if input.pending_download {
        ContentStatus::Downloading
    } else if input.failed {
        ContentStatus::Failed
    } else if input.processing {
        ContentStatus::Processing
    } else if input.has_file && input.quality_suboptimal {
        ContentStatus::Upgradable
    } else if input.has_file {
        ContentStatus::Available
    } else if input.ignored {
        ContentStatus::Ignored
    } else if input.wanted && input.release_in_future {
        ContentStatus::Upcoming
    } else if input.wanted {
        ContentStatus::Wanted
    } else {
        ContentStatus::Missing
    }
}

/// Calculate status from entity-layer facts without persisting a competing
/// status column.
pub async fn calculate_content_status(
    db: &Database,
    content_type: ContentType,
    content_id: &str,
    user_id: &str,
    media_file_id: Option<&str>,
    wanted: bool,
    release_at: Option<&str>,
) -> Result<ContentStatus> {
    let key = (content_type, content_id.to_string());
    calculate_content_status_batch(
        db,
        user_id,
        &[ContentStatusSubject {
            content_type,
            content_id: content_id.to_string(),
            media_file_id: media_file_id.map(ToOwned::to_owned),
            wanted,
            ignored: false,
            release_at: release_at.map(ToOwned::to_owned),
        }],
    )
    .await?
    .remove(&key)
    .context("Content status reducer omitted the requested item")
}

#[derive(Debug, Clone)]
pub struct ContentStatusSubject {
    pub content_type: ContentType,
    pub content_id: String,
    pub media_file_id: Option<String>,
    pub wanted: bool,
    pub ignored: bool,
    pub release_at: Option<String>,
}

/// Resolve a page of statuses with bounded entity queries instead of issuing
/// three to four round trips per row.
pub async fn calculate_content_status_batch(
    db: &Database,
    user_id: &str,
    subjects: &[ContentStatusSubject],
) -> Result<HashMap<(ContentType, String), ContentStatus>> {
    let pool = db.pool();
    let media_ids: Vec<String> = subjects
        .iter()
        .filter_map(|subject| subject.media_file_id.clone())
        .collect();
    let media_files = if media_ids.is_empty() {
        Vec::new()
    } else {
        MediaFile::query(pool)
            .filter(crate::graphql::entities::MediaFileWhereInput {
                id: Some(string_in(&media_ids)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("Failed to batch-read media analysis state for content status")?
    };
    let media_by_id: HashMap<String, MediaFile> = media_files
        .into_iter()
        .map(|media| (media.id.clone(), media))
        .collect();

    let playback_or: Vec<PlaybackSessionWhereInput> = subjects
        .iter()
        .filter_map(|subject| {
            if let Some(media_file_id) = subject.media_file_id.as_deref() {
                return Some(PlaybackSessionWhereInput {
                    media_file_id: Some(string_eq(media_file_id)),
                    ..Default::default()
                });
            }
            let mut filter = PlaybackSessionWhereInput::default();
            match subject.content_type {
                ContentType::Movie => filter.movie_id = Some(string_eq(&subject.content_id)),
                ContentType::Episode => filter.episode_id = Some(string_eq(&subject.content_id)),
                ContentType::Track => filter.track_id = Some(string_eq(&subject.content_id)),
                ContentType::Chapter => return None,
            }
            Some(filter)
        })
        .collect();
    let playback_sessions = if playback_or.is_empty() {
        Vec::new()
    } else {
        PlaybackSession::query(pool)
            .filter(PlaybackSessionWhereInput {
                user_id: Some(string_eq(user_id)),
                completed_at: Some(DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                or: Some(playback_or),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("Failed to batch-read playback state for content status")?
    };

    let pending_or: Vec<PendingFileMatchWhereInput> = subjects
        .iter()
        .map(|subject| {
            let mut filter = PendingFileMatchWhereInput::default();
            match subject.content_type {
                ContentType::Movie => filter.movie_id = Some(string_eq(&subject.content_id)),
                ContentType::Episode => filter.episode_id = Some(string_eq(&subject.content_id)),
                ContentType::Track => filter.track_id = Some(string_eq(&subject.content_id)),
                ContentType::Chapter => filter.chapter_id = Some(string_eq(&subject.content_id)),
            }
            filter
        })
        .collect();
    let pending_downloads = if pending_or.is_empty() {
        Vec::new()
    } else {
        PendingFileMatch::query(pool)
            .filter(PendingFileMatchWhereInput {
                user_id: Some(string_eq(user_id)),
                copied_at: Some(DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                or: Some(pending_or),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("Failed to batch-read download state for content status")?
    };

    let failed_media_ids: HashSet<String> = if media_ids.is_empty() {
        HashSet::new()
    } else {
        LibraryScanIssue::query(pool)
            .filter(LibraryScanIssueWhereInput {
                user_id: Some(string_eq(user_id)),
                media_file_id: Some(string_in(&media_ids)),
                stage: Some(string_eq("ANALYSIS")),
                resolved_at: Some(DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("Failed to batch-read processing failures for content status")?
            .into_iter()
            .filter_map(|issue| issue.media_file_id)
            .collect()
    };

    let pending_keys: HashSet<(ContentType, String)> = pending_downloads
        .iter()
        .filter_map(|pending| {
            pending
                .movie_id
                .as_ref()
                .map(|id| (ContentType::Movie, id.clone()))
                .or_else(|| {
                    pending
                        .episode_id
                        .as_ref()
                        .map(|id| (ContentType::Episode, id.clone()))
                })
                .or_else(|| {
                    pending
                        .track_id
                        .as_ref()
                        .map(|id| (ContentType::Track, id.clone()))
                })
                .or_else(|| {
                    pending
                        .chapter_id
                        .as_ref()
                        .map(|id| (ContentType::Chapter, id.clone()))
                })
        })
        .collect();

    let mut result = HashMap::with_capacity(subjects.len());
    for subject in subjects {
        let media_file = subject
            .media_file_id
            .as_ref()
            .and_then(|id| media_by_id.get(id));
        let playback = playback_sessions.iter().find(|session| {
            subject
                .media_file_id
                .as_deref()
                .is_some_and(|id| session.media_file_id.as_deref() == Some(id))
                || match subject.content_type {
                    ContentType::Movie => {
                        session.movie_id.as_deref() == Some(subject.content_id.as_str())
                    }
                    ContentType::Episode => {
                        session.episode_id.as_deref() == Some(subject.content_id.as_str())
                    }
                    ContentType::Track => {
                        session.track_id.as_deref() == Some(subject.content_id.as_str())
                    }
                    ContentType::Chapter => false,
                }
        });
        let media_file_id = subject.media_file_id.as_deref();
        let status = reduce_content_status(ContentStatusInputs {
            playing: playback.is_some_and(|session| session.is_playing),
            paused: playback.is_some_and(|session| !session.is_playing),
            processing: media_file.is_some_and(|file| file.analyzed_at.is_none()),
            pending_download: pending_keys
                .contains(&(subject.content_type, subject.content_id.clone())),
            has_file: media_file.is_some(),
            quality_suboptimal: media_file
                .and_then(|file| file.quality_status.as_deref())
                .is_some_and(|status| status.eq_ignore_ascii_case("suboptimal")),
            failed: media_file_id.is_some_and(|id| failed_media_ids.contains(id)),
            ignored: subject.ignored,
            release_in_future: release_is_in_future(subject.release_at.as_deref()),
            wanted: subject.wanted,
        });
        result.insert((subject.content_type, subject.content_id.clone()), status);
    }
    Ok(result)
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

fn release_is_in_future(value: Option<&str>) -> bool {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return false;
    };
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&chrono::Utc) > chrono::Utc::now())
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map(|date| date > chrono::Utc::now().date_naive())
        })
        .unwrap_or(false)
}

/// Auto-download mode for media items
#[derive(Default, Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize, sqlx::Type)]
#[graphql(name = "AutoDownloadMode")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum AutoDownloadMode {
    /// Do not auto-download
    #[graphql(name = "NONE")]
    #[default]
    None,
    /// Auto-download all items
    #[graphql(name = "ALL")]
    All,
    /// Auto-download only wanted items
    #[graphql(name = "WANTED")]
    Wanted,
}

impl std::fmt::Display for AutoDownloadMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::All => write!(f, "all"),
            Self::Wanted => write!(f, "wanted"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_status_reducer_reaches_every_state() {
        let cases = [
            (ContentStatusInputs::default(), ContentStatus::Missing),
            (
                ContentStatusInputs {
                    wanted: true,
                    ..Default::default()
                },
                ContentStatus::Wanted,
            ),
            (
                ContentStatusInputs {
                    wanted: true,
                    release_in_future: true,
                    ..Default::default()
                },
                ContentStatus::Upcoming,
            ),
            (
                ContentStatusInputs {
                    ignored: true,
                    ..Default::default()
                },
                ContentStatus::Ignored,
            ),
            (
                ContentStatusInputs {
                    has_file: true,
                    ..Default::default()
                },
                ContentStatus::Available,
            ),
            (
                ContentStatusInputs {
                    has_file: true,
                    quality_suboptimal: true,
                    ..Default::default()
                },
                ContentStatus::Upgradable,
            ),
            (
                ContentStatusInputs {
                    processing: true,
                    ..Default::default()
                },
                ContentStatus::Processing,
            ),
            (
                ContentStatusInputs {
                    failed: true,
                    ..Default::default()
                },
                ContentStatus::Failed,
            ),
            (
                ContentStatusInputs {
                    pending_download: true,
                    ..Default::default()
                },
                ContentStatus::Downloading,
            ),
            (
                ContentStatusInputs {
                    paused: true,
                    ..Default::default()
                },
                ContentStatus::Paused,
            ),
            (
                ContentStatusInputs {
                    playing: true,
                    ..Default::default()
                },
                ContentStatus::Playing,
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(reduce_content_status(input), expected);
        }
    }

    #[test]
    fn content_status_precedence_reflects_active_reality() {
        let every_fact = ContentStatusInputs {
            playing: true,
            paused: true,
            processing: true,
            pending_download: true,
            has_file: true,
            quality_suboptimal: true,
            failed: true,
            ignored: true,
            release_in_future: true,
            wanted: true,
        };
        assert_eq!(reduce_content_status(every_fact), ContentStatus::Playing);
        assert_eq!(
            reduce_content_status(ContentStatusInputs {
                pending_download: true,
                has_file: true,
                quality_suboptimal: true,
                ..Default::default()
            }),
            ContentStatus::Downloading
        );
        assert_eq!(
            reduce_content_status(ContentStatusInputs {
                has_file: true,
                ignored: true,
                ..Default::default()
            }),
            ContentStatus::Available
        );
    }
}
