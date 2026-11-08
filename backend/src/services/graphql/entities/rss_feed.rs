use crate::graphql::entities::*;
use async_graphql::Result;
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
    table = "rss_feeds",
    plural = "RssFeeds",
    default_sort = "name",
    write_policy = "admin.write"
)]
pub struct RssFeed {
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
    pub library_id: Option<String>,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    /// The feed address, which carries the tracker passkey. Stored as an encrypted envelope (the
    /// write transform encrypts it like `Source.credentials`); never readable through GraphQL.
    /// Read it with `jobs::rss_poll::reveal_feed_url`.
    #[graphql(skip)]
    #[input_only]
    #[graphql_orm(sensitive)]
    pub url: String,

    /// What to show of the address: host, path and parameter names with every value hidden. Set
    /// by the write transform whenever the address is written.
    #[graphql(name = "urlDisplay")]
    pub url_display: Option<String>,

    #[graphql(name = "enabled")]
    #[filterable(type = "boolean")]
    pub enabled: bool,

    #[graphql(name = "pollIntervalMinutes")]
    #[filterable(type = "number")]
    pub poll_interval_minutes: i32,

    #[graphql(name = "postDownloadAction")]
    #[filterable(type = "string")]
    pub post_download_action: Option<String>,

    #[graphql(name = "lastPolledAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_polled_at: Option<String>,

    #[graphql(name = "lastSuccessfulAt")]
    #[filterable(type = "date")]
    pub last_successful_at: Option<String>,

    #[graphql(name = "lastError")]
    pub last_error: Option<String>,

    #[graphql(name = "consecutiveFailures")]
    #[filterable(type = "number")]
    pub consecutive_failures: Option<i32>,

    /// `ok`, `failing` or `auth_expired`, from the last poll or health check.
    #[graphql(name = "healthStatus")]
    #[filterable(type = "string")]
    pub health_status: Option<String>,

    /// Why it is failing: `auth`, `unreachable`, `rate_limited` or `other`.
    #[graphql(name = "healthKind")]
    pub health_kind: Option<String>,

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
        target = "RssFeedItem",
        from = "id",
        to = "feed_id",
        multiple,
        emit_fk = false
    )]
    pub items: Vec<RssFeedItem>,
}

#[derive(Default)]
pub struct RssFeedCustomOperations;
