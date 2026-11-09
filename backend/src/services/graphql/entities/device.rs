//! A client that signs in and plays: a browser, an installed app or a TV. The client keeps a
//! stable `clientId` of its own and checks in with it (`checkInDevice`); the server records
//! what it saw of the request (address, user agent, login session). Members see their own
//! devices and administrators everyone's. Only the name can be changed through the generated
//! update; everything else is written by the check-in and sign-out actions (design.md Q133).

use crate::graphql::entities::*;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "devices",
    plural = "Devices",
    default_sort = "last_seen_at",
    index = "user_id",
    index = "session_id",
    unique_index = "user_id,client_id",
    read_policy = "member.read",
    write_policy = "member.write"
)]
pub struct Device {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    /// The client's own stable identifier, kept in its storage.
    #[graphql(name = "clientId")]
    #[filterable(type = "string")]
    pub client_id: String,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    /// True once someone renamed it; check-ins then keep the name.
    #[graphql(name = "nameIsCustom")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub name_is_custom: bool,

    /// `tv`, `phone`, `tablet`, `desktop` or `unknown`.
    #[graphql(name = "formFactor")]
    #[filterable(type = "string")]
    pub form_factor: String,

    /// Operating system or TV platform, as the client reported it ("iPadOS", "webOS").
    #[graphql(name = "platform")]
    #[filterable(type = "string")]
    pub platform: Option<String>,

    /// Browser or app ("Safari", "Librarian app").
    #[graphql(name = "product")]
    pub product: Option<String>,

    #[graphql(name = "appVersion")]
    pub app_version: Option<String>,

    #[graphql(name = "userAgent")]
    pub user_agent: Option<String>,

    #[graphql(name = "lastIp")]
    pub last_ip: Option<String>,

    #[graphql(name = "lastSeenAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_seen_at: String,

    /// The login session that last checked in from this device. Signing the device out
    /// revokes it; it is never a credential and never leaves the server.
    #[graphql_orm(private)]
    pub session_id: Option<String>,

    #[graphql(name = "signedOutAt")]
    #[filterable(type = "date")]
    pub signed_out_at: Option<String>,

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
pub struct DeviceCustomOperations;
