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
#[graphql(name = "CastDevice")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "cast_devices", plural = "CastDevices", default_sort = "name")]
pub struct CastDevice {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "Address")]
    #[filterable(type = "string")]
    pub address: String,

    #[graphql(name = "Port")]
    #[filterable(type = "number")]
    pub port: i32,

    #[graphql(name = "Model")]
    #[filterable(type = "string")]
    pub model: Option<String>,

    #[graphql(name = "DeviceType")]
    #[filterable(type = "string")]
    pub device_type: String,

    #[graphql(name = "IsFavorite")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_favorite: bool,

    #[graphql(name = "IsManual")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_manual: bool,

    #[graphql(name = "LastSeenAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_seen_at: Option<String>,

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
pub struct CastDeviceCustomOperations;
