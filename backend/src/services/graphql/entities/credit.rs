use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

/// One person's part in one title, from a metadata provider (`docs/design.md` Q129).
///
/// A credit names its title through exactly one of `movieId`, `showId`, `seasonId`,
/// `episodeId`, `albumId`, `trackId` or `audiobookId`, with `targetKind` saying which. `kind` is
/// `cast` or `crew`; a guest star is a `cast` credit with `guest` set. Crew credits carry the
/// provider's department and job ("Directing", "Director"); cast credits carry the character.
///
/// Credits are owned by the title's owner (`userId`, copied from the title when written), and
/// are replaced per title and source on every metadata refresh
/// (`services::metadata::credits::replace_credits`), so a refresh never duplicates them.
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
    table = "credits",
    plural = "Credits",
    default_sort = "credit_order",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct Credit {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    /// The owner of the credited title.
    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "personId")]
    #[filterable(type = "string")]
    pub person_id: String,

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

    /// `cast` or `crew`.
    #[graphql(name = "kind")]
    #[filterable(type = "string")]
    #[sortable]
    pub kind: String,

    /// A guest star of one episode (a `cast` credit).
    #[graphql(name = "guest")]
    #[filterable(type = "boolean")]
    pub guest: bool,

    /// The provider's department: "Directing", "Writing", "Production", "Acting"…
    #[graphql(name = "department")]
    #[filterable(type = "string")]
    #[sortable]
    pub department: Option<String>,

    /// "Director", "Screenplay", "Executive Producer", "Composer", "Narrator"…
    #[graphql(name = "job")]
    #[filterable(type = "string")]
    pub job: Option<String>,

    #[graphql(name = "characterName")]
    #[filterable(type = "string")]
    pub character_name: Option<String>,

    /// The provider's billing order, lowest first.
    #[graphql(name = "creditOrder")]
    #[filterable(type = "number")]
    #[sortable]
    pub credit_order: Option<i32>,

    /// Episodes this person appears in or worked on, for a show or season credit.
    #[graphql(name = "episodeCount")]
    #[filterable(type = "number")]
    #[sortable]
    pub episode_count: Option<i32>,

    /// Where the credit came from: `tmdb`, `musicbrainz` or `audiobook` (the book's own record).
    #[graphql(name = "source")]
    #[filterable(type = "string")]
    pub source: String,

    /// The provider's id for this credit, where it has one (TMDB `credit_id`).
    #[graphql(name = "providerCreditId")]
    pub provider_credit_id: Option<String>,

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
    #[relation(
        target = "Person",
        from = "person_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub person: Option<Box<Person>>,
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
pub struct CreditCustomOperations;
