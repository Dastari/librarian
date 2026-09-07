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
    table = "rss_feed_items",
    plural = "RssFeedItems",
    default_sort = "seen_at",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct RssFeedItem {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "feedId")]
    #[filterable(type = "string")]
    pub feed_id: String,

    #[graphql(name = "guid")]
    #[filterable(type = "string")]
    pub guid: Option<String>,

    #[graphql(name = "linkHash")]
    #[filterable(type = "string")]
    pub link_hash: String,

    #[graphql(name = "titleHash")]
    #[filterable(type = "string")]
    pub title_hash: String,

    #[graphql(name = "title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    #[graphql(name = "link")]
    pub link: String,

    #[graphql(name = "pubDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub pub_date: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,

    #[graphql(name = "parsedShowName")]
    #[filterable(type = "string")]
    pub parsed_show_name: Option<String>,

    #[graphql(name = "parsedSeason")]
    #[filterable(type = "number")]
    pub parsed_season: Option<i32>,

    #[graphql(name = "parsedEpisode")]
    #[filterable(type = "number")]
    pub parsed_episode: Option<i32>,

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

    #[graphql(name = "parsedHdr")]
    #[filterable(type = "string")]
    pub parsed_hdr: Option<String>,

    #[graphql(name = "processed")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub processed: bool,

    #[graphql(name = "torrentId")]
    #[filterable(type = "string")]
    pub torrent_id: Option<String>,

    #[graphql(name = "skippedReason")]
    pub skipped_reason: Option<String>,

    #[graphql(name = "seenAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub seen_at: String,
}

#[derive(Default)]
pub struct RssFeedItemCustomOperations;
