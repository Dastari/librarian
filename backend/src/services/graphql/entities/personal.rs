//! Private viewing intent and presentation preferences. None of these entities
//! is an acquisition instruction. Ordinary access is generated, including nested
//! connections and commit-scoped subscriptions.

use crate::graphql::entities::*;
use async_graphql::Enum;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum PersonalListKind {
    Manual,
    Smart,
    WatchLater,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum PersonalContentType {
    Movie,
    Show,
    Album,
    Audiobook,
    Track,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum PersonalAvailability {
    Any,
    Available,
    Missing,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum PersonalSort {
    Title,
    CreatedAt,
    UpdatedAt,
}

macro_rules! display_enum {
    ($ty:ty, $($variant:ident => $value:literal),+ $(,)?) => {
        impl std::fmt::Display for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(match self { $(Self::$variant => $value),+ })
            }
        }
    };
}
display_enum!(PersonalListKind, Manual => "manual", Smart => "smart", WatchLater => "watch_later");
display_enum!(PersonalContentType, Movie => "movie", Show => "show", Album => "album", Audiobook => "audiobook", Track => "track");
display_enum!(PersonalAvailability, Any => "any", Available => "available", Missing => "missing");
display_enum!(PersonalSort, Title => "title", CreatedAt => "created_at", UpdatedAt => "updated_at");

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
    table = "personal_lists",
    plural = "PersonalLists",
    default_sort = "name",
    index = "user_id",
    unique_index = "user_id,name",
    read_policy = "member.read",
    write_policy = "member.write"
)]
pub struct PersonalList {
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,
    pub description: Option<String>,
    pub kind: PersonalListKind,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub saved_view_id: Option<String>,
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "SavedView",
        from = "saved_view_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub saved_view: Option<Box<SavedView>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "ListEntry",
        from = "id",
        to = "list_id",
        multiple,
        emit_fk = false
    )]
    pub entries: Vec<ListEntry>,
}
#[derive(Default)]
pub struct PersonalListCustomOperations;

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
    table = "list_entries",
    plural = "ListEntries",
    default_sort = "position",
    keyset = "position asc, id asc",
    index = "user_id",
    index = "list_id,position,id",
    unique_index = "list_id,movie_id",
    unique_index = "list_id,show_id",
    unique_index = "list_id,album_id",
    unique_index = "list_id,audiobook_id",
    unique_index = "list_id,track_id",
    read_policy = "member.read",
    write_policy = "member.write"
)]
pub struct ListEntry {
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub list_id: String,
    #[filterable(type = "number")]
    #[sortable]
    pub position: i32,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub movie_id: Option<String>,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub show_id: Option<String>,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub album_id: Option<String>,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub audiobook_id: Option<String>,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub track_id: Option<String>,
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
    // Keep the ID tie-breaker after position in generated composite ordering.
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "PersonalList",
        from = "list_id",
        to = "id",
        on_delete = "cascade",
        propagate_change = "up"
    )]
    pub list: Option<Box<PersonalList>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "Movie", from = "movie_id", to = "id", on_delete = "cascade")]
    pub movie: Option<Box<Movie>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "Show", from = "show_id", to = "id", on_delete = "cascade")]
    pub show: Option<Box<Show>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "Album", from = "album_id", to = "id", on_delete = "cascade")]
    pub album: Option<Box<Album>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Audiobook",
        from = "audiobook_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub audiobook: Option<Box<Audiobook>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "Track", from = "track_id", to = "id", on_delete = "cascade")]
    pub track: Option<Box<Track>>,
}
#[derive(Default)]
pub struct ListEntryCustomOperations;

/// Version 1 is a conjunction of a title substring, optional library and stored
/// availability predicate. Null library means all libraries owned by the caller.
/// Shows support ANY availability only: there is no stored show availability
/// field and generated nested existential filtering is not assumed.
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
    table = "saved_views",
    plural = "SavedViews",
    default_sort = "name",
    index = "user_id",
    read_policy = "member.read",
    write_policy = "member.write"
)]
pub struct SavedView {
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,
    pub version: i32,
    pub content_type: PersonalContentType,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: Option<String>,
    pub title_contains: Option<String>,
    pub availability: PersonalAvailability,
    pub sort: PersonalSort,
    #[boolean_field]
    pub descending: bool,
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub library: Option<Box<Library>>,
}
#[derive(Default)]
pub struct SavedViewCustomOperations;

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
    table = "home_shelves",
    plural = "HomeShelves",
    default_sort = "position",
    keyset = "position asc, id asc",
    unique_index = "user_id,shelf_key",
    read_policy = "member.read",
    write_policy = "member.write"
)]
pub struct HomeShelf {
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,
    #[filterable(type = "string")]
    pub shelf_key: String,
    #[filterable(type = "number")]
    #[sortable]
    pub position: i32,
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub visible: bool,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub list_id: Option<String>,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: Option<String>,
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
    // Keep the ID tie-breaker after position in generated composite ordering.
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "PersonalList",
        from = "list_id",
        to = "id",
        on_delete = "set_null"
    )]
    pub list: Option<Box<PersonalList>>,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "set_null"
    )]
    pub library: Option<Box<Library>>,
}
#[derive(Default)]
pub struct HomeShelfCustomOperations;

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
    table = "library_preferences",
    plural = "LibraryPreferences",
    default_sort = "position",
    keyset = "position asc, id asc",
    unique_index = "user_id,library_id",
    read_policy = "member.read",
    write_policy = "member.write"
)]
pub struct LibraryPreference {
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: String,
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub favourite: bool,
    #[filterable(type = "number")]
    #[sortable]
    pub position: i32,
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
    // Keep the ID tie-breaker after position in generated composite ordering.
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub library: Option<Box<Library>>,
}
#[derive(Default)]
pub struct LibraryPreferenceCustomOperations;
