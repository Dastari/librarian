use async_graphql::{Context, InputObject, Object, SimpleObject};
use graphql_orm::graphql::filters::StringFilter;
use std::collections::{HashMap, HashSet};

use crate::db::Database;
use crate::graphql::auth::AuthExt;
use crate::graphql::entities::{
    Audiobook, AudiobookWhereInput, Chapter, ChapterWhereInput, Episode, EpisodeWhereInput,
    Library, LibraryWhereInput, Movie, MovieWhereInput, Show, ShowWhereInput, Track,
    TrackWhereInput,
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
        let mut statuses = Vec::with_capacity(authorized.len());
        for (input, _) in authorized {
            let Some(status) = resolved
                .get(&(input.content_type, input.id.clone()))
                .copied()
            else {
                continue;
            };
            statuses.push(ContentStatusResult {
                content_type: input.content_type,
                id: input.id,
                status,
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
