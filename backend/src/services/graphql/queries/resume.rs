//! Bounded personal resume projection, intentionally a computed-read exception.
//!
//! Ordinary roots/grouped aggregates cannot express latest-per-logical-title
//! through MediaFile -> Episode/Track/Chapter -> Show/Album/Audiobook joins.
//! Keep existing generated entities and relations; do not invent HAVING/window
//! SQL or persist a duplicate read model. Typed entity reads share one SQLite
//! deferred read snapshot, with caller entity/SQL-row/field policy checks.

use std::collections::{HashMap, HashSet};

use async_graphql::{Context, Object, Result, SimpleObject};
use graphql_orm::DefaultBackend;
use graphql_orm::graphql::filters::StringFilter;
use graphql_orm::graphql::orm::{
    DatabaseFilter, DbAuthContext, Entity, EntityAccessKind, EntityAccessSurface, EntityQuery,
    FromSqlRow, MutationContext, ReadVisibility, TransactionMode,
};
use graphql_orm::graphql::pagination::KeysetPageInput;

use crate::db::Database;
use crate::graphql::auth::AuthExt;
use crate::graphql::entities::*;

const MAX_LIMIT: i32 = 100;
// Below the pinned transaction read's legacy 100-row maximum. Every typed
// ID batch is bounded by this size; no raw-pool explicit-limit truncation.
const BATCH_SIZE: i64 = 64;
const MAX_SCANNED: usize = 10_000;

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "ResumeShelf", rename_fields = "camelCase")]
pub struct ResumeShelf {
    pub items: Vec<PlaybackProgress>,
    /// True only after finding another eligible logical title; false only
    /// after exhausting the relevant history; null when proof is incomplete.
    pub has_more: Option<bool>,
    /// The bounded top-N window is proven, not a claim that all history is sent.
    pub complete: bool,
    pub scanned_count: i32,
    pub error: Option<String>,
}

impl ResumeShelf {
    fn incomplete(scanned: usize) -> Self {
        Self { items: vec![], has_more: None, complete: false, scanned_count: scanned as i32,
            error: Some("Resume history scan limit reached; the requested shelf could not be proven complete".into()) }
    }
}

#[derive(Default)]
pub struct ResumeQueries;

#[Object]
impl ResumeQueries {
    /// Latest personal checkpoint per readable movie, show, album or book.
    /// Watched/completed newest checkpoints suppress older stale checkpoints.
    #[graphql(name = "resumeShelf")]
    async fn resume_shelf(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: i32,
        #[graphql(name = "libraryId")] library_id: Option<String>,
    ) -> Result<ResumeShelf> {
        let user = ctx.require_member()?;
        if !(1..=MAX_LIMIT).contains(&limit) {
            return Err(async_graphql::Error::new(
                "Resume limit must be between 1 and 100",
            ));
        }
        let db = ctx.data::<Database>()?;
        // The pin calls this a MutationContext; only read APIs are used. A
        // deferred SQLite BEGIN establishes a stable snapshot at the first
        // read. Always roll it back: no writes, hooks, events or hidden repair.
        let mut snapshot = MutationContext::begin_with_auth(
            db,
            TransactionMode::Default,
            ctx.data_opt::<DbAuthContext>(),
        )
        .await
        .map_err(query_error)?;
        let result = collect_shelf(
            ctx,
            db,
            &mut snapshot,
            &user.user_id,
            library_id.as_deref(),
            limit as usize,
            MAX_SCANNED,
        )
        .await;
        snapshot.rollback().await.map_err(query_error)?;
        result
    }
}

fn query_error(error: impl std::fmt::Display) -> async_graphql::Error {
    async_graphql::Error::new(format!("Resume query failed: {error}"))
}
fn eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.into()),
        ..Default::default()
    }
}
fn ids(values: impl IntoIterator<Item = String>) -> StringFilter {
    let values: HashSet<_> = values.into_iter().collect();
    StringFilter {
        in_list: Some(values.into_iter().collect()),
        ..Default::default()
    }
}

async fn visibility<T: Entity + 'static>(
    ctx: &Context<'_>,
    db: &Database,
) -> Result<ReadVisibility> {
    let metadata = T::metadata();
    db.ensure_entity_access(
        Some(ctx),
        metadata.entity_name,
        metadata.read_policy,
        EntityAccessKind::Read,
        EntityAccessSurface::GraphqlQuery,
    )
    .await?;
    if db.row_policy().is_none() {
        return Err(async_graphql::Error::new(
            "Resume query requires a row policy",
        ));
    }
    let visibility = db
        .read_visibility::<T>(Some(ctx), EntityAccessSurface::GraphqlQuery)
        .await?;
    if visibility.requires_residual_checks() {
        return Err(async_graphql::Error::new(
            "Resume query requires complete database read visibility",
        ));
    }
    Ok(visibility)
}

async fn read<T, F>(
    ctx: &Context<'_>,
    db: &Database,
    snapshot: &mut MutationContext<'_>,
    filter: F,
) -> Result<Vec<T>>
where
    T: Entity + FromSqlRow<DefaultBackend> + Clone + Send + Sync + 'static,
    F: DatabaseFilter,
{
    let policy = visibility::<T>(ctx, db).await?;
    let rows = EntityQuery::<T>::new()
        .filter(&filter)
        .with_read_visibility(&policy)?
        .fetch_all_in_transaction(snapshot)
        .await
        .map_err(query_error)?;
    for row in &rows {
        // Complete SQL visibility is the row authorization boundary, just as
        // for generated keyset roots. Field access remains independent.
        for field in T::repository_field_policies() {
            db.ensure_readable_field(
                ctx,
                T::entity_name(),
                field.api_name,
                field.read_policy,
                Some(row),
            )
            .await?;
        }
    }
    Ok(rows)
}

struct Facts {
    files: HashMap<String, MediaFile>,
    libraries: HashMap<String, Library>,
    movies: HashMap<String, Movie>,
    episodes: HashMap<String, Episode>,
    shows: HashMap<String, Show>,
    tracks: HashMap<String, Track>,
    albums: HashMap<String, Album>,
    chapters: HashMap<String, Chapter>,
    books: HashMap<String, Audiobook>,
}

impl Facts {
    async fn load(
        ctx: &Context<'_>,
        db: &Database,
        snapshot: &mut MutationContext<'_>,
        rows: &[PlaybackProgress],
        owner: &str,
    ) -> Result<Self> {
        macro_rules! load {
            ($ty:ty, $where:ident, $values:expr) => {{
                let values: Vec<String> = $values;
                if values.is_empty() {
                    HashMap::new()
                } else {
                    read::<$ty, _>(
                        ctx,
                        db,
                        snapshot,
                        $where {
                            id: Some(ids(values)),
                            ..Default::default()
                        },
                    )
                    .await?
                    .into_iter()
                    .map(|row| (row.id.clone(), row))
                    .collect::<HashMap<_, _>>()
                }
            }};
        }
        let files = load!(
            MediaFile,
            MediaFileWhereInput,
            rows.iter()
                .filter_map(|row| row.media_file_id.clone())
                .collect()
        );
        let library_ids: Vec<_> = files
            .values()
            .filter_map(|file| file.library_id.clone())
            .collect();
        let libraries = if library_ids.is_empty() {
            HashMap::new()
        } else {
            read::<Library, _>(
                ctx,
                db,
                snapshot,
                LibraryWhereInput {
                    id: Some(ids(library_ids)),
                    user_id: Some(eq(owner)),
                    ..Default::default()
                },
            )
            .await?
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect()
        };
        let movies = load!(
            Movie,
            MovieWhereInput,
            files
                .values()
                .filter_map(|file| file.movie_id.clone())
                .collect()
        );
        let episodes = load!(
            Episode,
            EpisodeWhereInput,
            files
                .values()
                .filter_map(|file| file.episode_id.clone())
                .collect()
        );
        let tracks = load!(
            Track,
            TrackWhereInput,
            files
                .values()
                .filter_map(|file| file.track_id.clone())
                .collect()
        );
        let chapters = load!(
            Chapter,
            ChapterWhereInput,
            files
                .values()
                .filter_map(|file| file.chapter_id.clone())
                .collect()
        );
        let shows = load!(
            Show,
            ShowWhereInput,
            episodes
                .values()
                .map(|episode| episode.show_id.clone())
                .collect()
        );
        let albums = load!(
            Album,
            AlbumWhereInput,
            tracks
                .values()
                .map(|track| track.album_id.clone())
                .collect()
        );
        let books = load!(
            Audiobook,
            AudiobookWhereInput,
            chapters
                .values()
                .map(|chapter| chapter.audiobook_id.clone())
                .collect()
        );
        Ok(Self {
            files,
            libraries,
            movies,
            episodes,
            shows,
            tracks,
            albums,
            chapters,
            books,
        })
    }

    /// A progress row does not confer authority on the file or content it names.
    /// Require one unambiguous target, existing owned library, and a readable
    /// same-library logical parent. Admin feeds remain personal too.
    fn logical_title(
        &self,
        row: &PlaybackProgress,
        owner: &str,
        library_scope: Option<&str>,
    ) -> Option<(&'static str, String)> {
        if row.user_id != owner {
            return None;
        }
        let file = self.files.get(row.media_file_id.as_ref()?)?;
        let library_id = file.library_id.as_deref()?;
        let library = self.libraries.get(library_id)?;
        if library.user_id != owner || library_scope.is_some_and(|scope| scope != library_id) {
            return None;
        }
        if [
            &file.movie_id,
            &file.episode_id,
            &file.track_id,
            &file.chapter_id,
        ]
        .into_iter()
        .filter(|id| id.is_some())
        .count()
            != 1
        {
            return None;
        }
        if let Some(id) = &file.movie_id {
            let movie = self.movies.get(id)?;
            return (movie.user_id == owner && movie.library_id == library_id)
                .then(|| ("movie", movie.id.clone()));
        }
        if let Some(id) = &file.episode_id {
            let episode = self.episodes.get(id)?;
            let show = self.shows.get(&episode.show_id)?;
            return (show.user_id == owner && show.library_id == library_id)
                .then(|| ("show", show.id.clone()));
        }
        if let Some(id) = &file.track_id {
            let track = self.tracks.get(id)?;
            let album = self.albums.get(&track.album_id)?;
            return (track.library_id == library_id
                && album.user_id == owner
                && album.library_id == library_id)
                .then(|| ("album", album.id.clone()));
        }
        let chapter = self.chapters.get(file.chapter_id.as_ref()?)?;
        let book = self.books.get(&chapter.audiobook_id)?;
        (book.user_id == owner && book.library_id == library_id).then(|| ("book", book.id.clone()))
    }
}

fn eligible(row: &PlaybackProgress, file: &MediaFile) -> bool {
    let duration = row.duration.or(file.duration.map(f64::from));
    !row.is_watched
        && row.progress_percent.is_finite()
        && row.progress_percent > 1.0
        && row.progress_percent < 100.0
        && row.current_position.is_finite()
        && row.current_position > 0.0
        && duration.is_none_or(|duration| duration.is_finite() && duration > row.current_position)
}

async fn collect_shelf(
    ctx: &Context<'_>,
    db: &Database,
    snapshot: &mut MutationContext<'_>,
    owner: &str,
    library_scope: Option<&str>,
    limit: usize,
    budget: usize,
) -> Result<ResumeShelf> {
    visibility::<PlaybackProgress>(ctx, db).await?;
    if let Some(library) = library_scope {
        let library = read::<Library, _>(
            ctx,
            db,
            snapshot,
            LibraryWhereInput {
                id: Some(eq(library)),
                user_id: Some(eq(owner)),
                ..Default::default()
            },
        )
        .await?;
        if library.is_empty() {
            return Err(async_graphql::Error::new(
                "Library not found or access denied",
            ));
        }
    }
    let mut after = None;
    let mut scanned = 0;
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    loop {
        if scanned >= budget {
            return Ok(ResumeShelf::incomplete(scanned));
        }
        // Generated transaction keyset provides the exact persisted timestamp
        // + ID ordering, avoiding date-filter coercion or offset drift. The
        // explicit personal filter prevents admin access widening the history.
        let page = snapshot
            .keyset_page::<PlaybackProgress>(
                PlaybackProgressWhereInput {
                    user_id: Some(eq(owner)),
                    media_file_id: Some(StringFilter {
                        is_null: Some(false),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                KeysetPageInput {
                    after,
                    limit: Some(BATCH_SIZE.min((budget - scanned) as i64)),
                    include_total_count: false,
                },
            )
            .await
            .map_err(query_error)?;
        let last = page.page_info.end_cursor.clone();
        let exhausted = !page.page_info.has_next_page;
        let candidates: Vec<_> = page.edges.into_iter().map(|edge| edge.node).collect();
        if candidates.is_empty() {
            return Ok(ResumeShelf {
                items,
                has_more: Some(false),
                complete: true,
                scanned_count: scanned as i32,
                error: None,
            });
        }
        // The generated transaction repository helper has no request context.
        // Re-read its bounded IDs with the actual caller's complete visibility
        // and field checks on this same snapshot before consuming any facts.
        let visible = read::<PlaybackProgress, _>(
            ctx,
            db,
            snapshot,
            PlaybackProgressWhereInput {
                id: Some(ids(candidates.iter().map(|row| row.id.clone()))),
                user_id: Some(eq(owner)),
                ..Default::default()
            },
        )
        .await?
        .into_iter()
        .map(|row| (row.id.clone(), row))
        .collect::<HashMap<_, _>>();
        let rows: Vec<_> = candidates
            .iter()
            .filter_map(|row| visible.get(&row.id).cloned())
            .collect();
        let facts = Facts::load(ctx, db, snapshot, &rows, owner).await?;
        for candidate in candidates {
            scanned += 1;
            let Some(row) = visible.get(&candidate.id) else {
                continue;
            };
            let Some(key) = facts.logical_title(row, owner, library_scope) else {
                continue;
            };
            if !seen.insert(key) {
                continue;
            }
            let file = facts
                .files
                .get(row.media_file_id.as_ref().expect("validated file"))
                .expect("validated file");
            if !eligible(row, file) {
                continue;
            }
            if items.len() == limit {
                return Ok(ResumeShelf {
                    items,
                    has_more: Some(true),
                    complete: true,
                    scanned_count: scanned as i32,
                    error: None,
                });
            }
            items.push(row.clone());
        }
        if exhausted {
            return Ok(ResumeShelf {
                items,
                has_more: Some(false),
                complete: true,
                scanned_count: scanned as i32,
                error: None,
            });
        }
        if last.is_none() {
            return Err(async_graphql::Error::new(
                "Resume history cursor did not advance",
            ));
        }
        after = last;
    }
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod tests;
