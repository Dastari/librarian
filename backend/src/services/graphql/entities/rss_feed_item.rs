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
#[graphql(name = "RssFeedItem")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "rss_feed_items",
    plural = "RssFeedItems",
    default_sort = "seen_at"
)]
pub struct RssFeedItem {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "FeedId")]
    #[filterable(type = "string")]
    pub feed_id: String,

    #[graphql(name = "Guid")]
    #[filterable(type = "string")]
    pub guid: Option<String>,

    #[graphql(name = "LinkHash")]
    #[filterable(type = "string")]
    pub link_hash: String,

    #[graphql(name = "TitleHash")]
    #[filterable(type = "string")]
    pub title_hash: String,

    #[graphql(name = "Title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    #[graphql(name = "Link")]
    pub link: String,

    #[graphql(name = "PubDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub pub_date: Option<String>,

    #[graphql(name = "Description")]
    pub description: Option<String>,

    #[graphql(name = "ParsedShowName")]
    #[filterable(type = "string")]
    pub parsed_show_name: Option<String>,

    #[graphql(name = "ParsedSeason")]
    #[filterable(type = "number")]
    pub parsed_season: Option<i32>,

    #[graphql(name = "ParsedEpisode")]
    #[filterable(type = "number")]
    pub parsed_episode: Option<i32>,

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

    #[graphql(name = "ParsedHdr")]
    #[filterable(type = "string")]
    pub parsed_hdr: Option<String>,

    #[graphql(name = "Processed")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub processed: bool,

    #[graphql(name = "TorrentId")]
    #[filterable(type = "string")]
    pub torrent_id: Option<String>,

    #[graphql(name = "SkippedReason")]
    pub skipped_reason: Option<String>,

    #[graphql(name = "SeenAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub seen_at: String,
}

#[derive(Default)]
pub struct RssFeedItemCustomOperations;
