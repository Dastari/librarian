use crate::graphql::entities::*;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

/// One step of working out what a download is: what was tried, what it found and how long it took.
/// Steps are grouped by run, and a download keeps only its last few runs.
#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "identification_events",
    plural = "IdentificationEvents",
    default_sort = "sequence",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct IdentificationEvent {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "admin.write")]
    pub user_id: String,

    #[graphql(name = "infoHash")]
    #[filterable(type = "string")]
    pub info_hash: String,

    /// Which run this step belongs to. Sorting by it keeps a run's steps together.
    #[graphql(name = "runId")]
    #[filterable(type = "string")]
    #[sortable]
    pub run_id: String,

    /// What started the run, shown on its first step.
    #[graphql(name = "trigger")]
    pub trigger: String,

    #[graphql(name = "sequence")]
    #[filterable(type = "number")]
    #[sortable]
    pub sequence: i32,

    /// `running`, `done`, `found`, `warning`, `skipped` or `failed`.
    #[graphql(name = "status")]
    pub status: String,

    #[graphql(name = "message")]
    pub message: String,

    #[graphql(name = "detail")]
    pub detail: Option<String>,

    #[graphql(name = "durationMs")]
    pub duration_ms: Option<i64>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

/// Something found in a download: a book, an album, a season, a movie. It says what it most likely is,
/// what else it could be, and whether it has reached the library.
#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "identified_items",
    plural = "IdentifiedItems",
    default_sort = "title",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct IdentifiedItem {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "admin.write")]
    pub user_id: String,

    #[graphql(name = "infoHash")]
    #[filterable(type = "string")]
    pub info_hash: String,

    /// Identifies it within the download across runs, such as a book's group of files.
    #[graphql(name = "itemKey")]
    #[filterable(type = "string")]
    pub item_key: String,

    /// `audiobook`, `music`, `tv`, `movie`, `ebook` or `unknown`.
    #[graphql(name = "kind")]
    #[filterable(type = "string")]
    pub kind: String,

    /// `imported`, `wanted`, `in_library`, `identified`, `needs_choice` or `unknown`.
    #[graphql(name = "status")]
    #[filterable(type = "string")]
    pub status: String,

    /// What its files say it is.
    #[graphql(name = "label")]
    pub label: String,

    /// What it was identified as, once it has been.
    #[graphql(name = "title")]
    #[sortable]
    pub title: Option<String>,

    /// The author, artist or show.
    #[graphql(name = "creator")]
    pub creator: Option<String>,

    #[graphql(name = "year")]
    pub year: Option<i32>,

    #[graphql(name = "coverUrl")]
    pub cover_url: Option<String>,

    #[graphql(name = "fileCount")]
    pub file_count: i32,

    /// How many of its files have finished downloading.
    #[graphql(name = "finishedFiles")]
    pub finished_files: i32,

    #[graphql(name = "seconds")]
    pub seconds: Option<f64>,

    /// Zero to one.
    #[graphql(name = "confidence")]
    pub confidence: f64,

    /// One line on what it is or what's needed.
    #[graphql(name = "summary")]
    pub summary: Option<String>,

    /// Why it's thought to be this. JSON list of strings.
    #[graphql(name = "evidence")]
    pub evidence: String,

    /// What else it could be, best first. JSON list of candidates.
    #[graphql(name = "candidates")]
    pub candidates: String,

    /// Its files' paths. JSON list of strings.
    #[graphql(name = "filePaths")]
    pub file_paths: String,

    /// What its files say about themselves, to compare listings with. JSON.
    #[graphql(name = "identity")]
    pub identity: String,

    /// The provider listing it matched.
    #[graphql(name = "providerId")]
    pub provider_id: Option<String>,

    /// `Audiobook`, `Album`, `Show` or `Movie`: what it matched in the library.
    #[graphql(name = "libraryItemType")]
    pub library_item_type: Option<String>,

    #[graphql(name = "libraryItemId")]
    pub library_item_id: Option<String>,

    #[graphql(name = "libraryId")]
    pub library_id: Option<String>,

    /// Season number for a TV item.
    #[graphql(name = "season")]
    pub season: Option<i32>,

    /// A person can import it despite what the plan says.
    #[graphql(name = "canAccept")]
    #[boolean_field]
    pub can_accept: bool,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

#[derive(Default)]
pub struct IdentificationEventCustomOperations;

#[derive(Default)]
pub struct IdentifiedItemCustomOperations;
