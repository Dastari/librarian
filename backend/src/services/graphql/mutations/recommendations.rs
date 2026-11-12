//! Actions on recommended releases (`jobs::recommend`): asking for a lookup when a page is opened,
//! and downloading a recommendation. The recommendations themselves are read through the generated
//! `releaseRecommendations` query; only the server writes them.

use async_graphql::{Context, InputObject, Object, SimpleObject};

use crate::db::Database;
use crate::jobs::recommend::{self, LookupRequest, Requested};
use crate::services::ServicesManager;
use crate::services::graphql::auth::AuthExt;
use crate::services::graphql::entities::{Album, Artist, Audiobook, Episode, Movie, Show, Track};

/// What to look a recommendation up for: a season (or an episode of it), a movie, an album (or a
/// track of it) or an audiobook.
#[derive(Debug, Clone, InputObject)]
#[graphql(name = "LookupRecommendationsInput")]
#[graphql(rename_fields = "camelCase")]
pub struct LookupRecommendationsInput {
    pub show_id: Option<String>,
    pub season: Option<i32>,
    pub episode_id: Option<String>,
    pub movie_id: Option<String>,
    pub album_id: Option<String>,
    pub track_id: Option<String>,
    pub audiobook_id: Option<String>,
    /// An artist: answered, like an album or a track of theirs, by the artist's coverage plan.
    pub artist_id: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "LookupRecommendationsPayload")]
#[graphql(rename_fields = "camelCase")]
pub struct LookupRecommendationsPayload {
    pub success: bool,
    pub error: Option<String>,
    /// `cached` (an answer is stored), `queued` or `waiting` (already queued).
    pub state: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "GrabRecommendationPayload")]
#[graphql(rename_fields = "camelCase")]
pub struct GrabRecommendationPayload {
    pub success: bool,
    pub error: Option<String>,
    pub torrent_id: Option<String>,
    pub torrent_name: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "GrabRecommendationPlanPayload")]
#[graphql(rename_fields = "camelCase")]
pub struct GrabRecommendationPlanPayload {
    /// True when every release of the plan started.
    pub success: bool,
    pub error: Option<String>,
    /// How many releases started.
    pub started: i32,
    /// `'release title': why` for each release that did not start.
    pub failed: Vec<String>,
    pub torrent_ids: Vec<String>,
}

fn denied() -> async_graphql::Error {
    async_graphql::Error::new("Target not found or access denied")
}

/// The owner, library, label and normalised request for what a person asked about, after checking
/// it is theirs (or they are an admin).
async fn authorize(
    db: &Database,
    user_id: &str,
    is_admin: bool,
    input: LookupRecommendationsInput,
) -> async_graphql::Result<(String, String, String, LookupRequest)> {
    let own = |owner: &str| is_admin || owner == user_id;
    if let Some(id) = &input.movie_id {
        let movie = Movie::get(db.pool(), id).await?.ok_or_else(denied)?;
        if !own(&movie.user_id) {
            return Err(denied());
        }
        let label = match movie.year {
            Some(year) => format!("{} ({year})", movie.title),
            None => movie.title.clone(),
        };
        return Ok((
            movie.user_id,
            movie.library_id,
            label,
            LookupRequest {
                movie_id: Some(movie.id),
                ..Default::default()
            },
        ));
    }
    if let Some(id) = &input.audiobook_id {
        let book = Audiobook::get(db.pool(), id).await?.ok_or_else(denied)?;
        if !own(&book.user_id) {
            return Err(denied());
        }
        return Ok((
            book.user_id,
            book.library_id,
            book.title.clone(),
            LookupRequest {
                audiobook_id: Some(book.id),
                ..Default::default()
            },
        ));
    }
    let mut album_id = input.album_id.clone();
    if let Some(track_id) = &input.track_id {
        let track = Track::get(db.pool(), track_id).await?.ok_or_else(denied)?;
        album_id = Some(track.album_id);
    }
    let mut artist_id = input.artist_id.clone();
    if let Some(id) = &album_id {
        let album = Album::get(db.pool(), id).await?.ok_or_else(denied)?;
        if !own(&album.user_id) {
            return Err(denied());
        }
        artist_id = Some(album.artist_id);
    }
    if let Some(id) = &artist_id {
        // An album or a track: all answered by the artist's coverage plan (Q110).
        let artist = Artist::get(db.pool(), id).await?.ok_or_else(denied)?;
        if !own(&artist.user_id) {
            return Err(denied());
        }
        return Ok((
            artist.user_id,
            artist.library_id,
            artist.name.clone(),
            LookupRequest {
                artist_id: Some(artist.id),
                ..Default::default()
            },
        ));
    }
    let (mut show_id, mut season) = (input.show_id.clone(), input.season);
    if let Some(episode_id) = &input.episode_id {
        let episode = Episode::get(db.pool(), episode_id)
            .await?
            .ok_or_else(denied)?;
        show_id = Some(episode.show_id);
        season = Some(episode.season);
    }
    let Some(show_id) = show_id else {
        return Err(async_graphql::Error::new(
            "Name a show, a season, an episode, a movie, an album, a track or an audiobook",
        ));
    };
    let show = Show::get(db.pool(), &show_id).await?.ok_or_else(denied)?;
    if !own(&show.user_id) {
        return Err(denied());
    }
    // A season, an episode or the whole show: all answered by the show's coverage plan.
    let label = show.name.clone();
    Ok((
        show.user_id,
        show.library_id,
        label,
        LookupRequest {
            show_id: Some(show.id),
            season,
            ..Default::default()
        },
    ))
}

#[derive(Default)]
pub struct RecommendationMutations;

#[Object]
impl RecommendationMutations {
    /// Queue a lookup of the best release for something missing, when its page or season is opened.
    /// An answer that has not expired is kept and nothing is searched; otherwise the worker looks
    /// within the hourly search budget and the page shows "looking" until it has. Owner or admin.
    #[graphql(name = "lookupRecommendations")]
    async fn lookup_recommendations(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: LookupRecommendationsInput,
    ) -> async_graphql::Result<LookupRecommendationsPayload> {
        let user = ctx.require_member()?;
        let db = ctx.data::<Database>()?;
        let (owner, library, label, request) =
            authorize(db, &user.user_id, user.is_admin(), input).await?;
        match recommend::request_lookup(db, &owner, &library, label, request).await {
            Ok(state) => Ok(LookupRecommendationsPayload {
                success: true,
                error: None,
                state: Some(
                    match state {
                        Requested::Cached => "cached",
                        Requested::Queued => "queued",
                        Requested::AlreadyWaiting => "waiting",
                    }
                    .to_string(),
                ),
            }),
            Err(error) => Ok(LookupRecommendationsPayload {
                success: false,
                error: Some(crate::services::redact::redact_text(&format!("{error:#}"))),
                state: None,
            }),
        }
    }

    /// Download every release of a show's coverage plan (a `plan` recommendation), in order, each
    /// through the same path as `grabRecommendation`. Releases that start are reported even when
    /// others do not. Owner or admin.
    #[graphql(name = "grabRecommendationPlan")]
    async fn grab_recommendation_plan(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: String,
    ) -> async_graphql::Result<GrabRecommendationPlanPayload> {
        let user = ctx.require_member()?;
        let manager = ctx.data::<std::sync::Arc<ServicesManager>>()?;
        match recommend::grab_plan(manager, &id, &user.user_id, user.is_admin()).await {
            Ok(outcome) => Ok(GrabRecommendationPlanPayload {
                success: outcome.failed.is_empty() && !outcome.started.is_empty(),
                error: (!outcome.failed.is_empty()).then(|| {
                    format!(
                        "{} of {} release(s) could not start",
                        outcome.failed.len(),
                        outcome.failed.len() + outcome.started.len()
                    )
                }),
                started: outcome.started.len() as i32,
                torrent_ids: outcome
                    .started
                    .iter()
                    .filter_map(|started| started.torrent_id.clone())
                    .collect(),
                failed: outcome.failed,
            }),
            Err(error) => Ok(GrabRecommendationPlanPayload {
                success: false,
                error: Some(crate::services::redact::redact_text(&format!("{error:#}"))),
                started: 0,
                failed: Vec::new(),
                torrent_ids: Vec::new(),
            }),
        }
    }

    /// Download a recommended release through the same path automatic downloading uses, linked to
    /// what it is for. With "Download only the missing files from packs" on, a pack downloads only
    /// the files that fill what was missing. Owner or admin.
    #[graphql(name = "grabRecommendation")]
    async fn grab_recommendation(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: String,
    ) -> async_graphql::Result<GrabRecommendationPayload> {
        let user = ctx.require_member()?;
        let manager = ctx.data::<std::sync::Arc<ServicesManager>>()?;
        match recommend::grab_recommendation(manager, &id, &user.user_id, user.is_admin()).await {
            Ok(outcome) => Ok(GrabRecommendationPayload {
                success: true,
                error: None,
                torrent_id: outcome.torrent_id,
                torrent_name: Some(outcome.torrent_name),
            }),
            Err(error) => Ok(GrabRecommendationPayload {
                success: false,
                error: Some(crate::services::redact::redact_text(&format!("{error:#}"))),
                torrent_id: None,
                torrent_name: None,
            }),
        }
    }
}
