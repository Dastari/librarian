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
    table = "cast_devices",
    plural = "CastDevices",
    default_sort = "name",
    unique_index = "address,port",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct CastDevice {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "address")]
    #[filterable(type = "string")]
    pub address: String,

    #[graphql(name = "port")]
    #[filterable(type = "number")]
    pub port: i32,

    #[graphql(name = "model")]
    #[filterable(type = "string")]
    pub model: Option<String>,

    #[graphql(name = "deviceType")]
    #[filterable(type = "string")]
    pub device_type: String,

    #[graphql(name = "isFavorite")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_favorite: bool,

    #[graphql(name = "isManual")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_manual: bool,

    #[graphql(name = "enabled")]
    #[filterable(type = "boolean")]
    pub enabled: Option<bool>,

    #[graphql(name = "discoveryOrigin")]
    #[filterable(type = "string")]
    pub discovery_origin: Option<String>,

    #[graphql(name = "playbackSupported")]
    #[filterable(type = "boolean")]
    pub playback_supported: Option<bool>,

    #[graphql(name = "firstSeenAt")]
    #[filterable(type = "date")]
    pub first_seen_at: Option<String>,

    #[graphql(name = "lastSeenAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_seen_at: Option<String>,

    #[graphql(name = "lastProbeAt")]
    #[filterable(type = "date")]
    pub last_probe_at: Option<String>,

    #[graphql(name = "lastProbeError")]
    pub last_probe_error: Option<String>,

    /// Who made it, when the receiver says (DLNA descriptions do; Google Cast does not).
    #[graphql(name = "manufacturer")]
    pub manufacturer: Option<String>,

    /// A stable identifier that survives an address change: the Google Cast `id`
    /// or the UPnP UDN. Discovery finds a receiver by it when its address moved.
    #[graphql(name = "receiverId")]
    #[filterable(type = "string")]
    pub receiver_id: Option<String>,

    /// True once an administrator renamed it; discovery then keeps that name.
    /// Null on rows from before the column existed, read as false.
    #[graphql(name = "nameIsCustom")]
    #[filterable(type = "boolean")]
    pub name_is_custom: Option<bool>,

    /// What discovery last learned, as JSON (`services::cast::describe::DiscoveryDetails`):
    /// the protocol, the name as reported, the TXT record or UPnP description and so on.
    #[graphql(name = "discoveryDetails")]
    pub discovery_details: Option<String>,

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
pub struct CastDeviceCustomOperations;
