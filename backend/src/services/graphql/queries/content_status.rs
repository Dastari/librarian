use async_graphql::{Context, InputObject, Object, SimpleObject};
use graphql_orm::graphql::filters::StringFilter;
use std::collections::{HashMap, HashSet};

use crate::db::Database;
use crate::graphql::auth::AuthExt;
use crate::graphql::entities::{
    Audiobook, AudiobookWhereInput, Chapter, ChapterWhereInput, Episode, EpisodeWhereInput,
    Library, LibraryWhereInput, Movie, MovieWhereInput, Show, ShowWhereInput, Torrent, TorrentFile,
    TorrentFileWhereInput, TorrentWhereInput, Track, TrackWhereInput,
    common::{ContentStatus, ContentStatusSubject, ContentType, calculate_content_status_batch},
};

#[derive(Debug, Clone, InputObject)]
#[graphql(name = "ContentStatusRequestInput", rename_fields = "camelCase")]
pub struct ContentStatusRequestInput {
    pub content_type: ContentType,
    pub id: String,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "ContentStatusResult", rename_fields = "camelCase")]
pub struct ContentStatusResult {
    pub content_type: ContentType,
    pub id: String,
    pub status: ContentStatus,
    /// How far along its download is, from 0 to 1, while one is running.
    pub download_progress: Option<f64>,
}

#[derive(Default)]
pub struct ContentStatusQueries;

#[Object]
impl ContentStatusQueries {
    /// Resolve authoritative statuses in one bounded request. Missing or
    /// unauthorized IDs are omitted instead of exposing their existence.
    #[graphql(name = "contentStatuses")]
    async fn content_statuses(
        &self,
        ctx: &Context<'_>,
        inputs: Vec<ContentStatusRequestInput>,
    ) -> async_graphql::Result<Vec<ContentStatusResult>> {
        let user = ctx.librarian_auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        if inputs.len() > 500 {
            return Err(async_graphql::Error::new(
                "At most 500 content statuses can be requested at once",
            ));
        }

        let mut facts_by_key =
            authorized_content_facts_batch(db, &user.user_id, user.is_admin(), &inputs).await?;
        let mut authorized = Vec::with_capacity(inputs.len());
        for input in inputs {
            let Some(facts) = facts_by_key.remove(&(input.content_type, input.id.clone())) else {
                continue;
            };
            authorized.push((input, facts));
        }
        let subjects: Vec<ContentStatusSubject> = authorized
            .iter()
            .map(|(input, facts)| ContentStatusSubject {
                content_type: input.content_type,
                content_id: input.id.clone(),
                media_file_id: facts.media_file_id.clone(),
                wanted: facts.wanted,
                ignored: facts.ignored,
                release_at: facts.release_at.clone(),
            })
            .collect();
        let resolved = calculate_content_status_batch(db, &user.user_id, &subjects)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        let downloads = active_downloads(db, &user.user_id, user.is_admin())
            .await
            .map_err(internal_query_error)?;
        let mut statuses = Vec::with_capacity(authorized.len());
        for (input, facts) in authorized {
            let Some(status) = resolved
                .get(&(input.content_type, input.id.clone()))
                .copied()
            else {
                continue;
            };
            // A title with a file stays as it is: a pack that includes it doesn't change that.
            let download_progress = if facts.media_file_id.is_none() {
                download_progress_for(&input.id, &facts.download_target, &downloads)
            } else {
                None
            };
            statuses.push(ContentStatusResult {
                content_type: input.content_type,
                id: input.id,
                status: if download_progress.is_some() {
                    ContentStatus::Downloading
                } else {
                    status
                },
                download_progress,
            });
        }
        Ok(statuses)
    }
}

struct ContentFacts {
    media_file_id: Option<String>,
    wanted: bool,
    ignored: bool,
    release_at: Option<String>,
    download_target: DownloadTarget,
}

/// What a torrent can name to be downloading this title, directly or through what it belongs to.
#[derive(Debug, Clone, PartialEq)]
enum DownloadTarget {
    Movie,
    Episode {
        show_id: String,
        season: i32,
        episode: i32,
    },
    Track {
        album_id: String,
    },
    Chapter {
        audiobook_id: String,
    },
}

/// A torrent still downloading, and its files. A stored row keeps up with the client on each sync.
#[derive(Debug, Clone, Default)]
struct ActiveDownload {
    progress: f64,
    movie_id: Option<String>,
    episode_id: Option<String>,
    show_id: Option<String>,
    season: Option<i32>,
    track_id: Option<String>,
    album_id: Option<String>,
    chapter_id: Option<String>,
    audiobook_id: Option<String>,
    files: Vec<(String, f64)>,
}

const DOWNLOADING_STATES: [&str; 4] = ["downloading", "checking", "initializing", "queued"];

async fn active_downloads(
    db: &Database,
    user_id: &str,
    is_admin: bool,
) -> anyhow::Result<Vec<ActiveDownload>> {
    let states: Vec<String> = DOWNLOADING_STATES.map(str::to_string).to_vec();
    let torrents = Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            state: Some(string_in(&states)),
            user_id: (!is_admin).then(|| StringFilter {
                eq: Some(user_id.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    let ids: Vec<String> = torrents.iter().map(|torrent| torrent.id.clone()).collect();
    let mut files: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    if !ids.is_empty() {
        for file in TorrentFile::query(db.pool())
            .filter(TorrentFileWhereInput {
                torrent_id: Some(string_in(&ids)),
                ..Default::default()
            })
            .fetch_all()
            .await?
        {
            files
                .entry(file.torrent_id)
                .or_default()
                .push((file.relative_path, file.progress));
        }
    }
    Ok(torrents
        .into_iter()
        .map(|torrent| ActiveDownload {
            progress: torrent.progress,
            files: files.remove(&torrent.id).unwrap_or_default(),
            movie_id: torrent.movie_id,
            episode_id: torrent.episode_id,
            show_id: torrent.show_id,
            season: torrent.season,
            track_id: torrent.track_id,
            album_id: torrent.album_id,
            chapter_id: torrent.chapter_id,
            audiobook_id: torrent.audiobook_id,
        })
        .collect())
}

/// `S01E03` in a file name, or the range in `S01E01-E02`.
fn path_has_episode(path: &str, season: i32, episode: i32) -> bool {
    static PATTERN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)\bs(\d{1,2})e(\d{1,3})(?:-?e(\d{1,3}))?").expect("episode pattern")
    });
    PATTERN.captures_iter(path).any(|found| {
        let number = |index: usize| {
            found
                .get(index)
                .and_then(|part| part.as_str().parse::<i32>().ok())
        };
        let first = number(2).unwrap_or(-1);
        number(1) == Some(season) && (first..=number(3).unwrap_or(first)).contains(&episode)
    })
}

/// How far along the download of this title is, if one is running. Within a pack an episode goes
/// by its own file; one the pack doesn't contain isn't downloading at all.
fn download_progress_for(
    id: &str,
    target: &DownloadTarget,
    downloads: &[ActiveDownload],
) -> Option<f64> {
    let names = |candidate: &Option<String>, wanted: &str| candidate.as_deref() == Some(wanted);
    downloads.iter().find_map(|download| {
        let progress = match target {
            DownloadTarget::Movie => names(&download.movie_id, id).then_some(download.progress),
            DownloadTarget::Track { album_id } => (names(&download.track_id, id)
                || names(&download.album_id, album_id))
            .then_some(download.progress),
            DownloadTarget::Chapter { audiobook_id } => (names(&download.chapter_id, id)
                || names(&download.audiobook_id, audiobook_id))
            .then_some(download.progress),
            DownloadTarget::Episode {
                show_id,
                season,
                episode,
            } => {
                if names(&download.episode_id, id) {
                    Some(download.progress)
                } else if names(&download.show_id, show_id)
                    && download.season.is_none_or(|pack| pack == *season)
                {
                    if download.files.is_empty() {
                        // Until the file list arrives, a torrent grabbed for one episode claims only that one.
                        download
                            .episode_id
                            .is_none()
                            .then_some(download.season)
                            .flatten()
                            .map(|_| download.progress)
                    } else {
                        download
                            .files
                            .iter()
                            .find(|(path, _)| path_has_episode(path, *season, *episode))
                            .map(|(_, progress)| *progress)
                    }
                } else {
                    None
                }
            }
        }?;
        Some(if progress.is_finite() {
            progress.clamp(0.0, 1.0)
        } else {
            0.0
        })
    })
}

async fn authorized_content_facts_batch(
    db: &Database,
    user_id: &str,
    is_admin: bool,
    inputs: &[ContentStatusRequestInput],
) -> async_graphql::Result<HashMap<(ContentType, String), ContentFacts>> {
    let owns = |owner_id: &str| is_admin || owner_id == user_id;
    let ids_for = |content_type| {
        inputs
            .iter()
            .filter(|input| input.content_type == content_type)
            .map(|input| input.id.clone())
            .collect::<Vec<_>>()
    };
    let movie_ids = ids_for(ContentType::Movie);
    let episode_ids = ids_for(ContentType::Episode);
    let track_ids = ids_for(ContentType::Track);
    let chapter_ids = ids_for(ContentType::Chapter);
    let mut result = HashMap::with_capacity(inputs.len());

    if !movie_ids.is_empty() {
        let movies = Movie::query(db.pool())
            .filter(MovieWhereInput {
                id: Some(string_in(&movie_ids)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .map_err(internal_query_error)?;
        for movie in movies.into_iter().filter(|movie| owns(&movie.user_id)) {
            result.insert(
                (ContentType::Movie, movie.id.clone()),
                ContentFacts {
                    media_file_id: movie.media_file_id,
                    wanted: movie.wanted,
                    ignored: movie.ignored.unwrap_or(false),
                    release_at: movie.release_date,
                    download_target: DownloadTarget::Movie,
                },
            );
        }
    }

    if !episode_ids.is_empty() {
        let episodes = Episode::query(db.pool())
            .filter(EpisodeWhereInput {
                id: Some(string_in(&episode_ids)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .map_err(internal_query_error)?;
        let show_ids: Vec<String> = episodes
            .iter()
            .map(|episode| episode.show_id.clone())
            .collect();
        let authorized_shows: HashSet<String> = if show_ids.is_empty() {
            HashSet::new()
        } else {
            Show::query(db.pool())
                .filter(ShowWhereInput {
                    id: Some(string_in(&show_ids)),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .map_err(internal_query_error)?
                .into_iter()
                .filter(|show| owns(&show.user_id))
                .map(|show| show.id)
                .collect()
        };
        for episode in episodes
            .into_iter()
            .filter(|episode| authorized_shows.contains(&episode.show_id))
        {
            result.insert(
                (ContentType::Episode, episode.id.clone()),
                ContentFacts {
                    download_target: DownloadTarget::Episode {
                        show_id: episode.show_id,
                        season: episode.season,
                        episode: episode.episode,
                    },
                    media_file_id: episode.media_file_id,
                    wanted: episode.wanted,
                    ignored: episode.ignored.unwrap_or(false),
                    release_at: episode.air_date,
                },
            );
        }
    }

    if !track_ids.is_empty() {
        let tracks = Track::query(db.pool())
            .filter(TrackWhereInput {
                id: Some(string_in(&track_ids)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .map_err(internal_query_error)?;
        let library_ids: Vec<String> = tracks
            .iter()
            .map(|track| track.library_id.clone())
            .collect();
        let authorized_libraries: HashSet<String> = if library_ids.is_empty() {
            HashSet::new()
        } else {
            Library::query(db.pool())
                .filter(LibraryWhereInput {
                    id: Some(string_in(&library_ids)),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .map_err(internal_query_error)?
                .into_iter()
                .filter(|library| owns(&library.user_id))
                .map(|library| library.id)
                .collect()
        };
        for track in tracks
            .into_iter()
            .filter(|track| authorized_libraries.contains(&track.library_id))
        {
            result.insert(
                (ContentType::Track, track.id.clone()),
                ContentFacts {
                    download_target: DownloadTarget::Track {
                        album_id: track.album_id,
                    },
                    media_file_id: track.media_file_id,
                    wanted: track.wanted,
                    ignored: track.ignored.unwrap_or(false),
                    release_at: None,
                },
            );
        }
    }

    if !chapter_ids.is_empty() {
        let chapters = Chapter::query(db.pool())
            .filter(ChapterWhereInput {
                id: Some(string_in(&chapter_ids)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .map_err(internal_query_error)?;
        let audiobook_ids: Vec<String> = chapters
            .iter()
            .map(|chapter| chapter.audiobook_id.clone())
            .collect();
        let authorized_books: HashSet<String> = if audiobook_ids.is_empty() {
            HashSet::new()
        } else {
            Audiobook::query(db.pool())
                .filter(AudiobookWhereInput {
                    id: Some(string_in(&audiobook_ids)),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .map_err(internal_query_error)?
                .into_iter()
                .filter(|book| owns(&book.user_id))
                .map(|book| book.id)
                .collect()
        };
        for chapter in chapters
            .into_iter()
            .filter(|chapter| authorized_books.contains(&chapter.audiobook_id))
        {
            result.insert(
                (ContentType::Chapter, chapter.id.clone()),
                ContentFacts {
                    download_target: DownloadTarget::Chapter {
                        audiobook_id: chapter.audiobook_id,
                    },
                    media_file_id: chapter.media_file_id,
                    wanted: chapter.wanted,
                    ignored: chapter.ignored.unwrap_or(false),
                    release_at: None,
                },
            );
        }
    }
    Ok(result)
}

fn string_in(values: &[String]) -> StringFilter {
    StringFilter {
        in_list: Some(values.to_vec()),
        ..Default::default()
    }
}

fn internal_query_error(error: impl std::fmt::Display) -> async_graphql::Error {
    tracing::error!(error = %error, "Content status entity query failed");
    async_graphql::Error::new("Content status could not be resolved")
}

#[cfg(test)]
mod download_progress_tests {
    use super::*;

    fn pack(files: &[(&str, f64)]) -> ActiveDownload {
        ActiveDownload {
            progress: 0.4,
            show_id: Some("show".to_string()),
            season: Some(1),
            files: files
                .iter()
                .map(|(path, progress)| (path.to_string(), *progress))
                .collect(),
            ..Default::default()
        }
    }

    fn episode(season: i32, episode: i32) -> DownloadTarget {
        DownloadTarget::Episode {
            show_id: "show".to_string(),
            season,
            episode,
        }
    }

    #[test]
    fn an_episode_in_a_pack_goes_by_its_own_file() {
        let downloads = [pack(&[("S01E01-E02.mkv", 1.0), ("Show.S01E03.mkv", 0.25)])];
        assert_eq!(
            download_progress_for("e", &episode(1, 3), &downloads),
            Some(0.25)
        );
        assert_eq!(
            download_progress_for("e", &episode(1, 2), &downloads),
            Some(1.0)
        );
        // In the show, but not in this download.
        assert_eq!(download_progress_for("e", &episode(1, 9), &downloads), None);
        assert_eq!(download_progress_for("e", &episode(2, 3), &downloads), None);
    }

    #[test]
    fn a_pack_with_no_file_list_yet_covers_its_season() {
        let downloads = [pack(&[])];
        assert_eq!(
            download_progress_for("e", &episode(1, 3), &downloads),
            Some(0.4)
        );
        assert_eq!(download_progress_for("e", &episode(2, 3), &downloads), None);
    }

    #[test]
    fn a_single_episode_grab_with_no_file_list_yet_claims_only_that_episode() {
        let downloads = [ActiveDownload {
            episode_id: Some("wanted".to_string()),
            ..pack(&[])
        }];
        assert_eq!(
            download_progress_for("wanted", &episode(1, 3), &downloads),
            Some(0.4)
        );
        assert_eq!(
            download_progress_for("other", &episode(1, 4), &downloads),
            None
        );
    }

    #[test]
    fn a_torrent_names_a_title_or_what_it_belongs_to() {
        let book = ActiveDownload {
            progress: 0.36,
            audiobook_id: Some("book".to_string()),
            ..Default::default()
        };
        let chapter = DownloadTarget::Chapter {
            audiobook_id: "book".to_string(),
        };
        assert_eq!(
            download_progress_for("c", &chapter, std::slice::from_ref(&book)),
            Some(0.36)
        );
        let other = DownloadTarget::Chapter {
            audiobook_id: "other".to_string(),
        };
        assert_eq!(download_progress_for("c", &other, &[book]), None);
        let movie = ActiveDownload {
            progress: 2.0,
            movie_id: Some("m".to_string()),
            ..Default::default()
        };
        assert_eq!(
            download_progress_for("m", &DownloadTarget::Movie, &[movie]),
            Some(1.0)
        );
    }
}
