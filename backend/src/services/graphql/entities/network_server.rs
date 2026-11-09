use crate::graphql::entities::*;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

/// A file server Librarian signs in to by itself (SMB, no operating-system mount), saved once
/// with its sign-in so libraries and download folders on its shares never ask for it again
/// (`docs/design.md` Q98). A location on it stays a plain `smb://host[:port]/share/...` string;
/// the server is found from the host and port.
///
/// The password is written through the generated mutations and encrypted by the write
/// transform (the same envelope as `Source.credentials`); it is never readable through GraphQL.
/// `hasPassword` says whether one is stored. The lifecycle hooks normalise the host, refuse a
/// second server for the same host and port, and refuse removing a server, or moving it to
/// another host, while a library or the download folder still uses it.
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
    table = "network_servers",
    plural = "NetworkServers",
    default_sort = "name",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct NetworkServer {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    /// What the server is called in Librarian ("TrueNAS").
    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    /// A name or an IP address, lower case.
    #[graphql(name = "host")]
    #[filterable(type = "string")]
    #[sortable]
    pub host: String,

    /// `None` is the standard SMB port, 445.
    #[graphql(name = "port")]
    #[filterable(type = "number")]
    pub port: Option<i32>,

    /// Empty for a guest sign-in.
    #[graphql(name = "username")]
    pub username: Option<String>,

    /// The password, as an encrypted envelope. Input only; see the type's notes.
    #[graphql(skip)]
    #[input_only]
    #[graphql_orm(sensitive)]
    pub password: String,

    /// Whether a password is stored. Set by the write transform from `password`.
    #[graphql(name = "hasPassword")]
    #[filterable(type = "boolean")]
    pub has_password: bool,

    /// Share names known on this server: those it listed, and any typed in by hand where it would
    /// not list them. Offered when choosing a location, so a share is never typed twice.
    #[graphql(name = "knownShares")]
    #[json_field]
    pub known_shares: Vec<String>,

    /// `ok` or `failing`, from the last test or use.
    #[graphql(name = "status")]
    #[filterable(type = "string")]
    pub status: Option<String>,

    #[graphql(name = "lastError")]
    pub last_error: Option<String>,

    #[graphql(name = "lastCheckedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_checked_at: Option<String>,

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
pub struct NetworkServerCustomOperations;

#[async_graphql::ComplexObject]
impl NetworkServer {
    /// The server's address as locations on it begin: `smb://host` or `smb://host:port`.
    #[graphql(name = "address")]
    async fn address(&self) -> String {
        match self.port {
            Some(port) => format!("smb://{}:{port}", self.host),
            None => format!("smb://{}", self.host),
        }
    }
}
