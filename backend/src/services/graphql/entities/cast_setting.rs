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
    table = "cast_settings",
    plural = "CastSettings",
    default_sort = "created_at",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct CastSetting {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "autoDiscoveryEnabled")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub auto_discovery_enabled: bool,

    #[graphql(name = "discoveryIntervalSeconds")]
    #[filterable(type = "number")]
    pub discovery_interval_seconds: i32,

    #[graphql(name = "defaultVolume")]
    #[filterable(type = "number")]
    pub default_volume: f64,

    #[graphql(name = "transcodeIncompatible")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub transcode_incompatible: bool,

    #[graphql(name = "preferredQuality")]
    #[filterable(type = "string")]
    pub preferred_quality: Option<String>,

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
pub struct CastSettingCustomOperations;
