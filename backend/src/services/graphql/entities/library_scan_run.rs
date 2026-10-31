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
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
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
    #[sortable]
    pub started_at: Option<String>,

    #[graphql(name = "finishedAt")]
    #[filterable(type = "date")]
    pub finished_at: Option<String>,

    #[graphql(name = "discoveredCount")]
    #[sortable]
    pub discovered_count: i32,

    #[graphql(name = "existingCount")]
    pub existing_count: i32,

    #[graphql(name = "matchedCount")]
    #[sortable]
    pub matched_count: i32,

    #[graphql(name = "unmatchedCount")]
    #[sortable]
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
    #[sortable]
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

    // Query relations do not add physical FKs or change existing delete behavior.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "cascade",
        propagate_change = "up",
        emit_fk = false
    )]
    pub library: Option<Box<Library>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "LibraryScanIssue",
        from = "id",
        to = "scan_run_id",
        multiple,
        emit_fk = false
    )]
    pub issues: Vec<LibraryScanIssue>,
}

#[derive(Default)]
pub struct LibraryScanRunCustomOperations;
