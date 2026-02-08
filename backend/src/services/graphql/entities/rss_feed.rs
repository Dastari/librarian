use async_graphql::{Result, SimpleObject};
use macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
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
#[graphql(name = "RssFeed")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "rss_feeds", plural = "RssFeeds", default_sort = "name")]
pub struct RssFeed {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "LibraryId")]
    #[filterable(type = "string")]
    pub library_id: Option<String>,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "Url")]
    #[filterable(type = "string")]
    pub url: String,

    #[graphql(name = "Enabled")]
    #[filterable(type = "boolean")]
    pub enabled: bool,

    #[graphql(name = "PollIntervalMinutes")]
    #[filterable(type = "number")]
    pub poll_interval_minutes: i32,

    #[graphql(name = "PostDownloadAction")]
    #[filterable(type = "string")]
    pub post_download_action: Option<String>,

    #[graphql(name = "LastPolledAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_polled_at: Option<String>,

    #[graphql(name = "LastSuccessfulAt")]
    #[filterable(type = "date")]
    pub last_successful_at: Option<String>,

    #[graphql(name = "LastError")]
    pub last_error: Option<String>,

    #[graphql(name = "ConsecutiveFailures")]
    #[filterable(type = "number")]
    pub consecutive_failures: Option<i32>,

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
pub struct RssFeedCustomOperations;
