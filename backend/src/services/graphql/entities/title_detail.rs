use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

/// Everything else a metadata provider says about one title, kept whether or not a page shows
/// it yet (`docs/design.md` Q130): studios, networks, ratings by country, keywords, alternative
/// titles, trailers and other videos, and the title's ids on other sites.
///
/// One row per title, named like a [`Credit`]'s target. Lists of plain names are JSON string
/// lists; structured lists are JSON text in a documented shape (see each field), read by the web
/// app's `lib/titleDetails.ts`. `fetchedAt` is when the provider was last read for this title,
/// which is also how the background fill knows a title is done.
#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    async_graphql::SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(complex)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "title_details",
    plural = "TitleDetails",
    default_sort = "fetched_at",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct TitleDetail {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    /// The owner of the title.
    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    pub user_id: String,

    /// `movie`, `show`, `season`, `episode`, `album`, `track` or `audiobook`.
    #[graphql(name = "targetKind")]
    #[filterable(type = "string")]
    pub target_kind: String,

    #[graphql(name = "movieId")]
    #[filterable(type = "string")]
    pub movie_id: Option<String>,

    #[graphql(name = "showId")]
    #[filterable(type = "string")]
    pub show_id: Option<String>,

    #[graphql(name = "seasonId")]
    #[filterable(type = "string")]
    pub season_id: Option<String>,

    #[graphql(name = "episodeId")]
    #[filterable(type = "string")]
    pub episode_id: Option<String>,

    #[graphql(name = "albumId")]
    #[filterable(type = "string")]
    pub album_id: Option<String>,

    #[graphql(name = "trackId")]
    #[filterable(type = "string")]
    pub track_id: Option<String>,

    #[graphql(name = "audiobookId")]
    #[filterable(type = "string")]
    pub audiobook_id: Option<String>,

    /// `tmdb`, `musicbrainz` or `audiobook`.
    #[graphql(name = "source")]
    #[filterable(type = "string")]
    pub source: String,

    /// The provider's id for the title (`movie/603`, `tv/1399`, a release MBID).
    #[graphql(name = "providerId")]
    #[filterable(type = "string")]
    pub provider_id: Option<String>,

    /// Production companies (TMDB) or labels (MusicBrainz).
    #[graphql(name = "studios")]
    #[json_field]
    pub studios: Vec<String>,

    /// Networks a show first aired on.
    #[graphql(name = "networks")]
    #[json_field]
    pub networks: Vec<String>,

    #[graphql(name = "keywords")]
    #[json_field]
    pub keywords: Vec<String>,

    /// Countries the title comes from (ISO 3166-1).
    #[graphql(name = "originCountries")]
    #[json_field]
    pub origin_countries: Vec<String>,

    /// JSON `[{ "country": "US", "rating": "PG-13", "kind": "theatrical" }]`: movie release
    /// certifications or show content ratings, one per country.
    #[graphql(name = "contentRatings")]
    pub content_ratings: Option<String>,

    /// JSON `[{ "title": "…", "country": "JP", "kind": "romaji" }]`.
    #[graphql(name = "alternativeTitles")]
    pub alternative_titles: Option<String>,

    /// JSON `[{ "site": "YouTube", "key": "…", "url": "https://…", "name": "…", "kind": "Trailer",
    /// "official": true, "language": "en", "publishedAt": "…" }]`: links only, nothing is fetched.
    #[graphql(name = "videos")]
    pub videos: Option<String>,

    /// JSON `{ "tmdb": "603", "imdb": "tt0133093", "tvdb": "…", "wikidata": "Q83495", … }`.
    #[graphql(name = "externalIds")]
    pub external_ids: Option<String>,

    #[graphql(name = "homepage")]
    pub homepage: Option<String>,

    #[graphql(name = "fetchedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub fetched_at: String,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "Movie", from = "movie_id", to = "id", on_delete = "cascade")]
    pub movie: Option<Box<Movie>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "Show", from = "show_id", to = "id", on_delete = "cascade")]
    pub show: Option<Box<Show>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(
        target = "Season",
        from = "season_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub season: Option<Box<Season>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(
        target = "Episode",
        from = "episode_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub episode: Option<Box<Episode>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "Album", from = "album_id", to = "id", on_delete = "cascade")]
    pub album: Option<Box<Album>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "Track", from = "track_id", to = "id", on_delete = "cascade")]
    pub track: Option<Box<Track>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(
        target = "Audiobook",
        from = "audiobook_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub audiobook: Option<Box<Audiobook>>,
}

#[derive(Default)]
pub struct TitleDetailCustomOperations;
