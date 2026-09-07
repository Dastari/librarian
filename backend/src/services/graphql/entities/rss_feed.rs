use crate::graphql::entities::*;
use async_graphql::Result;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "rss_feeds", plural = "RssFeeds", default_sort = "name")]
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

    #[graphql(name = "url")]
    #[filterable(type = "string")]
    pub url: String,

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
pub struct RssFeedCustomOperations;
