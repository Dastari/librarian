use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "pending_file_matches",
    plural = "PendingFileMatches",
    default_sort = "created_at"
)]
pub struct PendingFileMatch {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "sourcePath")]
    #[filterable(type = "string")]
    pub source_path: String,

    #[graphql(name = "sourceType")]
    #[filterable(type = "string")]
    pub source_type: String,

    #[graphql(name = "sourceId")]
    #[filterable(type = "string")]
    pub source_id: Option<String>,

    #[graphql(name = "sourceFileIndex")]
    #[filterable(type = "number")]
    pub source_file_index: Option<i32>,

    #[graphql(name = "fileSize")]
    #[filterable(type = "number")]
    #[sortable]
    pub file_size: i64,

    #[graphql(name = "episodeId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub episode_id: Option<String>,

    #[graphql(name = "movieId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub movie_id: Option<String>,

    #[graphql(name = "trackId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub track_id: Option<String>,

    #[graphql(name = "chapterId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub chapter_id: Option<String>,

    #[graphql(name = "unmatchedReason")]
    pub unmatched_reason: Option<String>,

    #[graphql(name = "matchType")]
    #[filterable(type = "string")]
    pub match_type: Option<String>,

    #[graphql(name = "matchConfidence")]
    #[filterable(type = "number")]
    pub match_confidence: Option<f64>,

    #[graphql(name = "matchAttempts")]
    #[filterable(type = "number")]
    pub match_attempts: i32,

    #[graphql(name = "verificationStatus")]
    #[filterable(type = "string")]
    pub verification_status: Option<String>,

    #[graphql(name = "verificationReason")]
    pub verification_reason: Option<String>,

    #[graphql(name = "parsedResolution")]
    #[filterable(type = "string")]
    pub parsed_resolution: Option<String>,

    #[graphql(name = "parsedCodec")]
    #[filterable(type = "string")]
    pub parsed_codec: Option<String>,

    #[graphql(name = "parsedSource")]
    #[filterable(type = "string")]
    pub parsed_source: Option<String>,

    #[graphql(name = "parsedAudio")]
    #[filterable(type = "string")]
    pub parsed_audio: Option<String>,

    #[graphql(name = "copiedAt")]
    #[filterable(type = "date")]
    pub copied_at: Option<String>,

    #[graphql(name = "copyError")]
    pub copy_error: Option<String>,

    #[graphql(name = "copyAttempts")]
    #[filterable(type = "number")]
    pub copy_attempts: i32,

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
pub struct PendingFileMatchCustomOperations;
