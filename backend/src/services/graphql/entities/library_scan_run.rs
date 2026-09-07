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
    table = "library_scan_runs",
    plural = "LibraryScanRuns",
    default_sort = "created_at",
    read_policy = "member.read",
    write_policy = "admin.write",
    index = "user_id",
    index = "library_id",
    index = "status"
)]
pub struct LibraryScanRun {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    pub library_id: String,

    #[graphql(name = "status")]
    #[filterable(type = "string")]
    #[sortable]
    pub status: String,

    #[graphql(name = "currentStage")]
    #[filterable(type = "string")]
    pub current_stage: String,

    #[graphql(name = "startedAt")]
    #[filterable(type = "date")]
    pub started_at: Option<String>,

    #[graphql(name = "finishedAt")]
    #[filterable(type = "date")]
    pub finished_at: Option<String>,

    #[graphql(name = "discoveredCount")]
    pub discovered_count: i32,

    #[graphql(name = "existingCount")]
    pub existing_count: i32,

    #[graphql(name = "matchedCount")]
    pub matched_count: i32,

    #[graphql(name = "unmatchedCount")]
    pub unmatched_count: i32,

    #[graphql(name = "providerBlockedCount")]
    pub provider_blocked_count: i32,

    #[graphql(name = "analysisQueuedCount")]
    pub analysis_queued_count: i32,

    #[graphql(name = "analysisSucceededCount")]
    pub analysis_succeeded_count: i32,

    #[graphql(name = "analysisFailedCount")]
    pub analysis_failed_count: i32,

    #[graphql(name = "organizationSucceededCount")]
    pub organization_succeeded_count: i32,

    #[graphql(name = "organizationFailedCount")]
    pub organization_failed_count: i32,

    #[graphql(name = "missingCount")]
    pub missing_count: i32,

    #[graphql(name = "reconciledCount")]
    pub reconciled_count: i32,

    #[graphql(name = "summary")]
    pub summary: Option<String>,

    #[graphql(name = "errorCode")]
    #[filterable(type = "string")]
    pub error_code: Option<String>,

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
pub struct LibraryScanRunCustomOperations;
