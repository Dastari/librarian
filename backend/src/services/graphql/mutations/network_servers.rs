//! Actions on saved file servers that are not CRUD (Q98): signing in to test a server, and
//! listing the shares it offers. Adding, editing and removing a server are the generated
//! `NetworkServer` mutations, whose lifecycle hooks do the checks.

use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::db::Database;
use crate::graphql::entities::NetworkServer;
use crate::services::graphql::auth::AuthExt;
use crate::services::library_storage::{
    EntryKind, ShareListing, SmbCredentials, SmbLocation, SmbStorage, Storage, list_server_shares,
};
use crate::services::manager::ServicesManager;
use crate::services::network_servers as servers;

/// What to sign in with: a saved server, or what was typed before saving it (or both: typed
/// values win, and a blank password with the saved user name keeps the saved password).
#[derive(InputObject)]
#[graphql(name = "TestNetworkServerInput", rename_fields = "camelCase")]
pub struct TestNetworkServerInput {
    /// A saved server. Its result is recorded on it.
    pub id: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Also open this share and count its top-level folders.
    pub share: Option<String>,
}

#[derive(Clone, SimpleObject)]
#[graphql(name = "NetworkShare", rename_fields = "camelCase")]
pub struct NetworkShare {
    pub name: String,
    pub comment: Option<String>,
    /// Listed by the server just now (otherwise only remembered).
    pub listed: bool,
}

#[derive(Clone, SimpleObject)]
#[graphql(name = "TestNetworkServerResult", rename_fields = "camelCase")]
pub struct TestNetworkServerResult {
    /// Signed in (and, with `share`, opened it).
    pub success: bool,
    pub error: Option<String>,
    pub shares: Vec<NetworkShare>,
    /// Why the shares could not be listed, when signing in worked.
    pub shares_error: Option<String>,
    /// Folders at the top of `share`, when one was given.
    pub folders: Option<i32>,
}

#[derive(Clone, SimpleObject)]
#[graphql(name = "NetworkServerSharesResult", rename_fields = "camelCase")]
pub struct NetworkServerSharesResult {
    pub success: bool,
    pub error: Option<String>,
    pub shares: Vec<NetworkShare>,
}

fn merge(listed: &[ShareListing], known: &[String]) -> Vec<NetworkShare> {
    let mut out: Vec<NetworkShare> = listed
        .iter()
        .map(|share| NetworkShare {
            name: share.name.clone(),
            comment: Some(share.comment.clone()).filter(|comment| !comment.trim().is_empty()),
            listed: true,
        })
        .collect();
    for name in known {
        if !out
            .iter()
            .any(|share| share.name.eq_ignore_ascii_case(name))
        {
            out.push(NetworkShare {
                name: name.clone(),
                comment: None,
                listed: false,
            });
        }
    }
    out.sort_by_key(|share| share.name.to_lowercase());
    out
}

async fn saved(db: &Database, id: &str) -> Result<NetworkServer> {
    NetworkServer::get(db.pool(), &id.to_string())
        .await
        .map_err(|error| async_graphql::Error::new(error.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("That server is not saved"))
}

async fn schema(
    services: &Arc<ServicesManager>,
) -> Option<crate::services::graphql::LibrarianSchema> {
    services.get_graphql().await?.schema().await
}

#[derive(Default)]
pub struct ServerConnectionMutations;

#[Object]
impl ServerConnectionMutations {
    /// Sign in to a server and list its shares, with a saved server's sign-in or with what was
    /// typed. The answer names what is wrong in words a person can act on. A saved server's
    /// status records the outcome, and the shares it lists are remembered on it.
    #[graphql(name = "testNetworkServer")]
    async fn test_network_server(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: TestNetworkServerInput,
    ) -> Result<TestNetworkServerResult> {
        ctx.require_admin()?;
        let db = ctx.data::<Database>()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let failed = |error: String| TestNetworkServerResult {
            success: false,
            error: Some(error),
            shares: Vec::new(),
            shares_error: None,
            folders: None,
        };
        let server = match input.id.as_deref() {
            Some(id) => Some(saved(db, id).await?),
            None => None,
        };
        let host = match input
            .host
            .as_deref()
            .or(server.as_ref().map(|server| server.host.as_str()))
        {
            Some(host) => match servers::normalize_host(host) {
                Ok(host) => host,
                Err(error) => return Ok(failed(error)),
            },
            None => return Ok(failed("Enter the server's name or IP address".to_string())),
        };
        let port = match servers::normalize_port(
            input
                .port
                .or(server.as_ref().and_then(|server| server.port)),
        ) {
            Ok(port) => port.and_then(|port| u16::try_from(port).ok()),
            Err(error) => return Ok(failed(error)),
        };
        let saved_credentials = match server.as_ref() {
            Some(server) => match servers::credentials_of(services, server).await {
                Ok(credentials) => Some(credentials),
                Err(error) => return Ok(failed(error.to_string())),
            },
            None => None,
        };
        let credentials = match (input.username.as_deref().map(str::trim), saved_credentials) {
            (Some(username), saved) => {
                let typed = input.password.clone().unwrap_or_default();
                let password = match saved {
                    Some(saved)
                        if typed.is_empty() && saved.username.eq_ignore_ascii_case(username) =>
                    {
                        saved.password
                    }
                    _ => typed,
                };
                SmbCredentials {
                    username: username.to_string(),
                    password,
                }
            }
            (None, Some(saved)) => saved,
            (None, None) => SmbCredentials {
                username: String::new(),
                password: input.password.clone().unwrap_or_default(),
            },
        };

        let listing = list_server_shares(&host, port, &credentials).await;
        let schema = schema(services).await;
        let (shares, shares_error) = match listing {
            Err(error) => {
                if let (Some(server), Some(schema)) = (server.as_ref(), schema.as_ref()) {
                    servers::record_check(schema, &server.id, Some(&error.to_string())).await;
                }
                return Ok(failed(error.to_string()));
            }
            Ok(Ok(listed)) => (listed, None),
            Ok(Err(error)) => (Vec::new(), Some(error.to_string())),
        };

        let mut folders = None;
        if let Some(share) = input
            .share
            .as_deref()
            .map(str::trim)
            .filter(|share| !share.is_empty())
        {
            let address = match port {
                Some(port) => format!("smb://{host}:{port}/{share}"),
                None => format!("smb://{host}/{share}"),
            };
            let location = match SmbLocation::parse(&address) {
                Ok(location) => location,
                Err(error) => return Ok(failed(error.to_string())),
            };
            let storage = SmbStorage::new(location.clone(), credentials.clone());
            let opened = match storage.check().await {
                Ok(()) => storage.list_dir(&location.storage_path()).await,
                Err(error) => Err(error),
            };
            match opened {
                Ok(entries) => {
                    folders = Some(
                        entries
                            .iter()
                            .filter(|entry| entry.kind == EntryKind::Directory)
                            .count() as i32,
                    );
                }
                Err(error) => {
                    if let (Some(server), Some(schema)) = (server.as_ref(), schema.as_ref()) {
                        servers::record_check(schema, &server.id, Some(&error.to_string())).await;
                    }
                    return Ok(failed(error.to_string()));
                }
            }
        }

        if let (Some(server), Some(schema)) = (server.as_ref(), schema.as_ref()) {
            servers::record_check(schema, &server.id, None).await;
            let mut known: Vec<String> = shares.iter().map(|share| share.name.clone()).collect();
            known.extend(input.share.clone());
            for share in known {
                servers::remember_share(schema, server, &share).await;
            }
        }
        let known = server.map(|server| server.known_shares).unwrap_or_default();
        Ok(TestNetworkServerResult {
            success: true,
            error: None,
            shares: merge(&shares, &known),
            shares_error,
            folders,
        })
    }
}

#[derive(Default)]
pub struct ServerConnectionQueries;

#[Object]
impl ServerConnectionQueries {
    /// The shares a saved server offers: what it lists now, with the shares remembered on it.
    /// When it cannot be reached the remembered shares are returned with the reason.
    #[graphql(name = "networkServerShares")]
    async fn network_server_shares(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: String,
    ) -> Result<NetworkServerSharesResult> {
        ctx.require_admin()?;
        let db = ctx.data::<Database>()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let server = saved(db, &id).await?;
        let known = server.known_shares.clone();
        let credentials = match servers::credentials_of(services, &server).await {
            Ok(credentials) => credentials,
            Err(error) => {
                return Ok(NetworkServerSharesResult {
                    success: false,
                    error: Some(error.to_string()),
                    shares: merge(&[], &known),
                });
            }
        };
        let port = server.port.and_then(|port| u16::try_from(port).ok());
        match list_server_shares(&server.host, port, &credentials).await {
            Ok(Ok(listed)) => Ok(NetworkServerSharesResult {
                success: true,
                error: None,
                shares: merge(&listed, &known),
            }),
            Ok(Err(error)) | Err(error) => Ok(NetworkServerSharesResult {
                success: false,
                error: Some(error.to_string()),
                shares: merge(&[], &known),
            }),
        }
    }
}
