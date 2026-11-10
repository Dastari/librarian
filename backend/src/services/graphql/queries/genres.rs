//! Genre shelves for a library's Recommended view.
//!
//! Genres are stored as a JSON list on movies and shows, which the generated filters cannot
//! match, so this query groups one library's titles by genre after an explicit ownership
//! check. Reads go through the typed entity repositories, never raw SQL.

use std::collections::HashMap;

use async_graphql::{Context, ErrorExtensions, Object, Result, SimpleObject};
use graphql_orm::graphql::filters::StringFilter;

use crate::db::Database;
use crate::graphql::auth::AuthExt;
use crate::graphql::entities::*;

const MAX_GENRES: i32 = 12;
const MAX_PER_GENRE: i32 = 40;

/// One genre row: the most recently added titles of that genre and how many there are.
#[derive(SimpleObject)]
pub struct GenreShelf {
    pub genre: String,
    /// Titles of this genre in the library, including those not returned.
    pub total: i32,
    /// Newest first; empty for TV libraries.
    pub movies: Vec<Movie>,
    /// Newest first; empty for movie libraries.
    pub shows: Vec<Show>,
}

/// A genre someone can pick as an interest, with how many titles of it they can see.
#[derive(SimpleObject)]
pub struct GenreChoice {
    pub genre: String,
    pub movies: i32,
    pub shows: i32,
}

#[derive(Default)]
pub struct GenreQueries;

/// Movies and shows the caller can see: their own, or everything for an administrator.
async fn visible_titles(
    db: &Database,
    user: &crate::graphql::AuthUser,
) -> Result<(Vec<Movie>, Vec<Show>)> {
    let owner = (!user.is_admin()).then(|| StringFilter {
        eq: Some(user.user_id.clone()),
        ..Default::default()
    });
    let movies = Movie::query(db.pool())
        .filter(MovieWhereInput {
            user_id: owner.clone(),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    let shows = Show::query(db.pool())
        .filter(ShowWhereInput {
            user_id: owner,
            ..Default::default()
        })
        .fetch_all()
        .await?;
    Ok((movies, shows))
}

#[Object]
impl GenreQueries {
    /// The library's largest genres, each with its newest titles. Readable by the library's
    /// owner and administrators; anyone else gets "Library not found".
    #[graphql(name = "libraryGenreShelves")]
    async fn library_genre_shelves(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "libraryId")] library_id: String,
        #[graphql(name = "maxGenres", default = 6)] max_genres: i32,
        #[graphql(name = "perGenre", default = 20)] per_genre: i32,
    ) -> Result<Vec<GenreShelf>> {
        let user = ctx.require_member()?;
        if !(1..=MAX_GENRES).contains(&max_genres) || !(1..=MAX_PER_GENRE).contains(&per_genre) {
            return Err(async_graphql::Error::new(format!(
                "maxGenres must be 1-{MAX_GENRES} and perGenre 1-{MAX_PER_GENRE}"
            ))
            .extend_with(|_, extensions| extensions.set("code", "BAD_USER_INPUT")));
        }
        let db = ctx.data::<Database>()?;
        let library = Library::get(db.pool(), &library_id)
            .await?
            .filter(|library| user.is_admin() || library.user_id == user.user_id)
            .ok_or_else(|| {
                async_graphql::Error::new("Library not found")
                    .extend_with(|_, extensions| extensions.set("code", "NOT_FOUND"))
            })?;
        let in_library = StringFilter {
            eq: Some(library.id.clone()),
            ..Default::default()
        };

        let movies = Movie::query(db.pool())
            .filter(MovieWhereInput {
                library_id: Some(in_library.clone()),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        let shows = Show::query(db.pool())
            .filter(ShowWhereInput {
                library_id: Some(in_library),
                ..Default::default()
            })
            .fetch_all()
            .await?;

        let movie_groups = group_by_genre(movies, |movie| &movie.genres, |movie| &movie.created_at);
        let show_groups = group_by_genre(shows, |show| &show.genres, |show| &show.created_at);
        let mut shelves: Vec<GenreShelf> = movie_groups
            .into_iter()
            .map(|(genre, items)| GenreShelf {
                genre,
                total: items.len() as i32,
                movies: items,
                shows: Vec::new(),
            })
            .chain(show_groups.into_iter().map(|(genre, items)| GenreShelf {
                genre,
                total: items.len() as i32,
                movies: Vec::new(),
                shows: items,
            }))
            .collect();
        // Largest genres first; ties stay alphabetical so the order is stable.
        shelves.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.genre.cmp(&b.genre)));
        shelves.truncate(max_genres as usize);
        for shelf in &mut shelves {
            shelf.movies.truncate(per_genre as usize);
            shelf.shows.truncate(per_genre as usize);
        }
        Ok(shelves)
    }

    /// Every genre among the titles you can see, most titles first, for picking interests.
    #[graphql(name = "genreChoices")]
    async fn genre_choices(&self, ctx: &Context<'_>) -> Result<Vec<GenreChoice>> {
        let user = ctx.require_member()?;
        let db = ctx.data::<Database>()?;
        let (movies, shows) = visible_titles(db, user).await?;
        let mut choices: HashMap<String, GenreChoice> = HashMap::new();
        for (genre, items) in
            group_by_genre(movies, |movie| &movie.genres, |movie| &movie.created_at)
        {
            choices
                .entry(genre.to_lowercase())
                .or_insert_with(|| GenreChoice {
                    genre,
                    movies: 0,
                    shows: 0,
                })
                .movies = items.len() as i32;
        }
        for (genre, items) in group_by_genre(shows, |show| &show.genres, |show| &show.created_at) {
            choices
                .entry(genre.to_lowercase())
                .or_insert_with(|| GenreChoice {
                    genre,
                    movies: 0,
                    shows: 0,
                })
                .shows = items.len() as i32;
        }
        let mut choices: Vec<GenreChoice> = choices.into_values().collect();
        choices.sort_by(|a, b| {
            (b.movies + b.shows)
                .cmp(&(a.movies + a.shows))
                .then_with(|| a.genre.cmp(&b.genre))
        });
        Ok(choices)
    }

    /// One shelf per genre you picked as an interest, in your order, each with the newest
    /// movies and shows of it you can see. Genres with nothing to show are left out
    /// (design.md Q149).
    #[graphql(name = "genreInterestShelves")]
    async fn genre_interest_shelves(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "perGenre", default = 20)] per_genre: i32,
    ) -> Result<Vec<GenreShelf>> {
        let user = ctx.require_member()?;
        if !(1..=MAX_PER_GENRE).contains(&per_genre) {
            return Err(
                async_graphql::Error::new(format!("perGenre must be 1-{MAX_PER_GENRE}"))
                    .extend_with(|_, extensions| extensions.set("code", "BAD_USER_INPUT")),
            );
        }
        let db = ctx.data::<Database>()?;
        let interests = User::get(db.pool(), &user.user_id)
            .await?
            .and_then(|account| account.genre_interests)
            .unwrap_or_default();
        if interests.is_empty() {
            return Ok(Vec::new());
        }
        let (movies, shows) = visible_titles(db, user).await?;
        let mut movie_groups: HashMap<String, Vec<Movie>> =
            group_by_genre(movies, |movie| &movie.genres, |movie| &movie.created_at)
                .into_iter()
                .map(|(genre, items)| (genre.to_lowercase(), items))
                .collect();
        let mut show_groups: HashMap<String, Vec<Show>> =
            group_by_genre(shows, |show| &show.genres, |show| &show.created_at)
                .into_iter()
                .map(|(genre, items)| (genre.to_lowercase(), items))
                .collect();
        Ok(interests
            .iter()
            .filter_map(|genre| {
                let key = genre.trim().to_lowercase();
                let mut movies = movie_groups.remove(&key).unwrap_or_default();
                let mut shows = show_groups.remove(&key).unwrap_or_default();
                let total = (movies.len() + shows.len()) as i32;
                if total == 0 {
                    return None;
                }
                movies.truncate(per_genre as usize);
                shows.truncate(per_genre as usize);
                Some(GenreShelf {
                    genre: genre.trim().to_string(),
                    total,
                    movies,
                    shows,
                })
            })
            .collect())
    }
}

/// Group titles under each of their genres (case-insensitively), newest first. A genre is
/// labelled with its most common spelling; ties take the alphabetically first, which
/// prefers the capitalised form ("Thriller" over "thriller").
fn group_by_genre<T: Clone>(
    items: Vec<T>,
    genres: impl Fn(&T) -> &Vec<String>,
    created_at: impl Fn(&T) -> &String,
) -> Vec<(String, Vec<T>)> {
    let mut groups: HashMap<String, (HashMap<String, usize>, Vec<T>)> = HashMap::new();
    for item in &items {
        for genre in genres(item) {
            let name = genre.trim();
            if name.is_empty() {
                continue;
            }
            let (spellings, titles) = groups.entry(name.to_lowercase()).or_default();
            *spellings.entry(name.to_string()).or_default() += 1;
            titles.push(item.clone());
        }
    }
    groups
        .into_values()
        .map(|(spellings, mut titles)| {
            let label = spellings
                .into_iter()
                .max_by(|(a, count_a), (b, count_b)| count_a.cmp(count_b).then_with(|| b.cmp(a)))
                .map(|(name, _)| name)
                .unwrap_or_default();
            titles.sort_by(|a, b| created_at(b).cmp(created_at(a)));
            (label, titles)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::group_by_genre;

    #[test]
    fn titles_group_under_each_genre_newest_first_ignoring_case() {
        let items = vec![
            (
                "a",
                vec!["Drama".to_string(), "Thriller".to_string()],
                "2026-01-01".to_string(),
            ),
            ("b", vec!["drama".to_string()], "2026-03-01".to_string()),
            ("d", vec!["Drama".to_string()], "2025-12-01".to_string()),
            ("c", vec![" ".to_string()], "2026-02-01".to_string()),
        ];
        let mut groups = group_by_genre(items, |item| &item.1, |item| &item.2);
        groups.sort_by(|a, b| a.0.cmp(&b.0));
        let names: Vec<_> = groups
            .iter()
            .map(|(genre, titles)| {
                (
                    genre.as_str(),
                    titles.iter().map(|t| t.0).collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(
            names,
            vec![("Drama", vec!["b", "a", "d"]), ("Thriller", vec!["a"])]
        );
    }
}
