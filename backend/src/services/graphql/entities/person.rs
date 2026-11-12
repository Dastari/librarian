use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

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
    table = "people",
    plural = "People",
    default_sort = "name",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct Person {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    /// The person's TMDB id. A person known only from another source (a MusicBrainz artist, a
    /// book's author or narrator) has no TMDB id; the column predates that and cannot become
    /// nullable additively, so such a person holds a negative stand-in derived from their own key
    /// (`services::metadata::credits::stand_in_tmdb_id`). Zero and below mean "none".
    #[graphql(name = "tmdbPersonId")]
    #[filterable(type = "number")]
    #[unique]
    pub tmdb_person_id: i32,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "profileUrl")]
    pub profile_url: Option<String>,

    #[graphql(name = "imdbId")]
    #[filterable(type = "string")]
    pub imdb_id: Option<String>,

    #[graphql(name = "musicbrainzArtistId")]
    #[filterable(type = "string")]
    pub musicbrainz_artist_id: Option<String>,

    #[graphql(name = "wikidataId")]
    #[filterable(type = "string")]
    pub wikidata_id: Option<String>,

    /// The name lower-cased with punctuation and spacing folded, for people known by name only.
    #[graphql(name = "nameKey")]
    #[filterable(type = "string")]
    pub name_key: Option<String>,

    #[graphql(name = "biography")]
    pub biography: Option<String>,

    /// `YYYY-MM-DD`.
    #[graphql(name = "birthday")]
    pub birthday: Option<String>,

    /// `YYYY-MM-DD`.
    #[graphql(name = "deathday")]
    pub deathday: Option<String>,

    #[graphql(name = "placeOfBirth")]
    pub place_of_birth: Option<String>,

    /// "Acting", "Directing", "Writing", "Sound"…
    #[graphql(name = "knownForDepartment")]
    #[filterable(type = "string")]
    pub known_for_department: Option<String>,

    #[graphql(name = "alsoKnownAs")]
    #[json_field]
    pub also_known_as: Option<Vec<String>>,

    /// Every profile picture the provider lists, best first (remote URLs; the first is
    /// `profileUrl`, which the artwork cache serves).
    #[graphql(name = "imageUrls")]
    #[json_field]
    pub image_urls: Option<Vec<String>>,

    /// When the person's own record (biography, dates, ids, pictures) was last read.
    #[graphql(name = "detailsFetchedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub details_fetched_at: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    /// Movie credits for this person. Deleting the person deletes its credits.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "MovieCastCredit",
        from = "id",
        to = "person_id",
        multiple,
        emit_fk = false
    )]
    pub cast_credits: Vec<MovieCastCredit>,

    /// Every credit of this person, on any title (`Credit`).
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Credit",
        from = "id",
        to = "person_id",
        multiple,
        emit_fk = false
    )]
    pub credits: Vec<Credit>,
}

#[derive(Default)]
pub struct PersonCustomOperations;

#[derive(SimpleObject)]
#[graphql(name = "PersonDetailsResult", rename_fields = "camelCase")]
pub struct PersonDetailsResult {
    pub success: bool,
    pub person: Option<Person>,
    pub error: Option<String>,
}

/// Reading a person's own record from TMDB is an action (a provider import), not CRUD.
#[derive(Default)]
pub struct PersonMetadataMutations;

#[async_graphql::Object]
impl PersonMetadataMutations {
    /// Read a person's biography, dates, place of birth, ids and pictures from TMDB again.
    #[graphql(name = "refreshPersonDetails")]
    async fn refresh_person_details(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(name = "id")] id: String,
    ) -> async_graphql::Result<PersonDetailsResult> {
        use crate::graphql::auth::AuthExt;
        use crate::services::metadata::providers::MetadataService;

        let user = ctx.librarian_auth_user()?;
        if !user.is_admin() {
            return Err(async_graphql::Error::new(
                "Refreshing a person's details requires administrator authority",
            ));
        }
        let metadata = ctx.data_unchecked::<std::sync::Arc<MetadataService>>();
        Ok(match metadata.refresh_person_details(&id).await {
            Ok(person) => PersonDetailsResult {
                success: true,
                person: Some(person),
                error: None,
            },
            Err(error) => PersonDetailsResult {
                success: false,
                person: None,
                error: Some(error.to_string()),
            },
        })
    }
}
