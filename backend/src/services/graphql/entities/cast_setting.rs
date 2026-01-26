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
#[graphql(name = "CastSetting")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "cast_settings",
    plural = "CastSettings",
    default_sort = "created_at"
)]
pub struct CastSetting {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "AutoDiscoveryEnabled")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub auto_discovery_enabled: bool,

    #[graphql(name = "DiscoveryIntervalSeconds")]
    #[filterable(type = "number")]
    pub discovery_interval_seconds: i32,

    #[graphql(name = "DefaultVolume")]
    #[filterable(type = "number")]
    pub default_volume: f64,

    #[graphql(name = "TranscodeIncompatible")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub transcode_incompatible: bool,

    #[graphql(name = "PreferredQuality")]
    #[filterable(type = "string")]
    pub preferred_quality: Option<String>,

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
pub struct CastSettingCustomOperations;
