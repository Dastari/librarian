use async_graphql::SimpleObject;
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(name = "PendingFileMatch")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "pending_file_matches",
    plural = "PendingFileMatches",
    default_sort = "created_at"
)]
pub struct PendingFileMatch {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "SourcePath")]
    #[filterable(type = "string")]
    pub source_path: String,

    #[graphql(name = "SourceType")]
    #[filterable(type = "string")]
    pub source_type: String,

    #[graphql(name = "SourceId")]
    #[filterable(type = "string")]
    pub source_id: Option<String>,

    #[graphql(name = "SourceFileIndex")]
    #[filterable(type = "number")]
    pub source_file_index: Option<i32>,

    #[graphql(name = "FileSize")]
    #[filterable(type = "number")]
    #[sortable]
    pub file_size: i64,

    #[graphql(name = "EpisodeId")]
    #[filterable(type = "string")]
    pub episode_id: Option<String>,

    #[graphql(name = "MovieId")]
    #[filterable(type = "string")]
    pub movie_id: Option<String>,

    #[graphql(name = "TrackId")]
    #[filterable(type = "string")]
    pub track_id: Option<String>,

    #[graphql(name = "ChapterId")]
    #[filterable(type = "string")]
    pub chapter_id: Option<String>,

    #[graphql(name = "UnmatchedReason")]
    pub unmatched_reason: Option<String>,

    #[graphql(name = "MatchType")]
    #[filterable(type = "string")]
    pub match_type: Option<String>,

    #[graphql(name = "MatchConfidence")]
    #[filterable(type = "number")]
    pub match_confidence: Option<f64>,

    #[graphql(name = "MatchAttempts")]
    #[filterable(type = "number")]
    pub match_attempts: i32,

    #[graphql(name = "VerificationStatus")]
    #[filterable(type = "string")]
    pub verification_status: Option<String>,

    #[graphql(name = "VerificationReason")]
    pub verification_reason: Option<String>,

    #[graphql(name = "ParsedResolution")]
    #[filterable(type = "string")]
    pub parsed_resolution: Option<String>,

    #[graphql(name = "ParsedCodec")]
    #[filterable(type = "string")]
    pub parsed_codec: Option<String>,

    #[graphql(name = "ParsedSource")]
    #[filterable(type = "string")]
    pub parsed_source: Option<String>,

    #[graphql(name = "ParsedAudio")]
    #[filterable(type = "string")]
    pub parsed_audio: Option<String>,

    #[graphql(name = "CopiedAt")]
    #[filterable(type = "date")]
    pub copied_at: Option<String>,

    #[graphql(name = "CopyError")]
    pub copy_error: Option<String>,

    #[graphql(name = "CopyAttempts")]
    #[filterable(type = "number")]
    pub copy_attempts: i32,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

#[derive(Default)]
pub struct PendingFileMatchCustomOperations;
