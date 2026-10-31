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
    table = "library_scan_issues",
    plural = "LibraryScanIssues",
    default_sort = "created_at",
    read_policy = "member.read",
    write_policy = "admin.write",
    index = "scan_run_id",
    index = "library_id",
    index = "media_file_id",
    index = "issue_code"
)]
pub struct LibraryScanIssue {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "scanRunId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub scan_run_id: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: String,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub media_file_id: Option<String>,

    #[graphql(name = "stage")]
    #[filterable(type = "string")]
    #[sortable]
    pub stage: String,

    #[graphql(name = "issueCode")]
    #[filterable(type = "string")]
    #[sortable]
    pub issue_code: String,

    #[graphql(name = "severity")]
    #[filterable(type = "string")]
    #[sortable]
    pub severity: String,

    #[graphql(name = "message")]
    #[filterable(type = "string")]
    pub message: String,

    #[graphql(name = "remediation")]
    pub remediation: Option<String>,

    /// Bounded machine-readable context for remediation actions. This must
    /// never contain credentials; paths and provider-safe diagnostics only.
    #[graphql(name = "detailsJson")]
    pub details_json: Option<String>,

    #[graphql(name = "occurrenceCount")]
    #[sortable]
    pub occurrence_count: i32,

    /// Acknowledgement is independent of remediation; reading never resolves an issue.
    #[graphql(name = "readAt")]
    #[filterable(type = "date")]
    pub read_at: Option<String>,

    #[graphql(name = "resolvedAt")]
    #[filterable(type = "date")]
    pub resolved_at: Option<String>,

    #[graphql(name = "resolution")]
    pub resolution: Option<String>,

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
        target = "LibraryScanRun",
        from = "scan_run_id",
        to = "id",
        on_delete = "cascade",
        propagate_change = "up",
        emit_fk = false
    )]
    pub scan_run: Option<Box<LibraryScanRun>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "cascade",
        emit_fk = false
    )]
    pub library: Option<Box<Library>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "MediaFile",
        from = "media_file_id",
        to = "id",
        on_delete = "set_null",
        emit_fk = false
    )]
    pub media_file: Option<Box<MediaFile>>,
}

#[derive(Default)]
pub struct LibraryScanIssueCustomOperations;
