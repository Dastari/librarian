//! GraphQL filesystem mutations: `createDirectory` plus network-path configuration.
//! File delete, copy, move and rename are not exposed: they were never implemented and
//! library files change through the organizer and the media file actions instead.
//! A new filesystem mutation gets the FilesystemChangeBroker from ctx and calls
//! .send(FilesystemChangeEvent { ... }) after each success so the FilesystemChanged
//! subscription receives events.

use std::path::PathBuf;
use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result};
use tokio::fs;

use crate::services::graphql::auth::AuthExt;
use crate::services::graphql::filesystem_network;
use crate::{db::Database, services::manager::ServicesManager};

// ---------------------------------------------------------------------------
// Shared result type (no DB – used only for GraphQL payloads)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct FileOperationResult {
    pub success: bool,
    pub error: Option<String>,
    pub affected_count: i32,
    pub messages: Vec<String>,
    pub path: Option<String>,
}

impl FileOperationResult {
    pub fn ok(path: Option<String>, affected_count: i32, messages: Vec<String>) -> Self {
        Self {
            success: true,
            error: None,
            affected_count,
            messages,
            path,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(error.into()),
            affected_count: 0,
            messages: vec![],
            path: None,
        }
    }
}

/// Payload returned by all filesystem mutations.
#[derive(Clone)]
pub struct FileOperationPayload(FileOperationResult);

#[Object]
impl FileOperationPayload {
    #[graphql(name = "success")]
    async fn success(&self) -> bool {
        self.0.success
    }

    #[graphql(name = "error")]
    async fn error(&self) -> Option<&str> {
        self.0.error.as_deref()
    }

    #[graphql(name = "affectedCount")]
    async fn affected_count(&self) -> i32 {
        self.0.affected_count
    }

    #[graphql(name = "messages")]
    async fn messages(&self) -> &[String] {
        &self.0.messages
    }

    #[graphql(name = "path")]
    async fn path(&self) -> Option<&str> {
        self.0.path.as_deref()
    }
}

// ---------------------------------------------------------------------------
// Input types for the current filesystem schema surface.
// ---------------------------------------------------------------------------

#[derive(InputObject)]
#[graphql(name = "CreateDirectoryInput")]
pub struct CreateDirectoryInput {
    #[graphql(name = "path")]
    pub path: String,
}

#[derive(InputObject)]
#[graphql(name = "ConfigureNetworkPathInput")]
pub struct ConfigureNetworkPathInput {
    #[graphql(name = "path")]
    pub path: String,
    #[graphql(name = "username")]
    pub username: Option<String>,
    #[graphql(name = "password")]
    pub password: Option<String>,
    #[graphql(name = "mountPoint")]
    pub mount_point: Option<String>,
    #[graphql(name = "persist")]
    pub persist: Option<bool>,
    #[graphql(name = "attemptConnect")]
    pub attempt_connect: Option<bool>,
}

#[derive(Clone)]
pub struct NetworkPathConfigPayload {
    pub success: bool,
    pub error: Option<String>,
    pub resolved_path: String,
    pub connected: bool,
    pub stored: bool,
    pub message: Option<String>,
}

#[Object]
impl NetworkPathConfigPayload {
    #[graphql(name = "success")]
    async fn success(&self) -> bool {
        self.success
    }

    #[graphql(name = "error")]
    async fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    #[graphql(name = "resolvedPath")]
    async fn resolved_path(&self) -> &str {
        &self.resolved_path
    }

    #[graphql(name = "connected")]
    async fn connected(&self) -> bool {
        self.connected
    }

    #[graphql(name = "stored")]
    async fn stored(&self) -> bool {
        self.stored
    }

    #[graphql(name = "message")]
    async fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

// ---------------------------------------------------------------------------
// Mutation root extension
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FilesystemMutations;

#[Object]
impl FilesystemMutations {
    #[graphql(name = "createDirectory")]
    async fn create_directory(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: CreateDirectoryInput,
    ) -> Result<FileOperationPayload> {
        ctx.require_admin()?;
        run_create_directory(ctx, &input).await
    }

    #[graphql(name = "configureNetworkPath")]
    async fn configure_network_path(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: ConfigureNetworkPathInput,
    ) -> Result<NetworkPathConfigPayload> {
        ctx.require_admin()?;

        let db = ctx.data::<Database>()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;

        let result = filesystem_network::configure_network_path(
            db,
            services,
            filesystem_network::ConfigureNetworkPathInput {
                path: input.path,
                username: input.username,
                password: input.password,
                mount_point: input.mount_point,
                persist: input.persist.unwrap_or(true),
                attempt_connect: input.attempt_connect.unwrap_or(true),
            },
        )
        .await;

        Ok(NetworkPathConfigPayload {
            success: result.success,
            error: result.error,
            resolved_path: result.resolved_path,
            connected: result.connected,
            stored: result.stored,
            message: result.message,
        })
    }

    #[graphql(name = "reconnectLibraryPath")]
    async fn reconnect_library_path(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "path")] path: String,
    ) -> Result<NetworkPathConfigPayload> {
        ctx.require_admin()?;

        let db = ctx.data::<Database>()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;

        let reconnected = filesystem_network::reconnect_target_path(db, services, &path).await;
        let payload = match reconnected {
            Ok(true) => NetworkPathConfigPayload {
                success: true,
                error: None,
                resolved_path: path,
                connected: true,
                stored: true,
                message: Some("Reconnect attempted successfully".to_string()),
            },
            Ok(false) => NetworkPathConfigPayload {
                success: false,
                error: Some("No saved network config for this path".to_string()),
                resolved_path: path,
                connected: false,
                stored: false,
                message: None,
            },
            Err(e) => NetworkPathConfigPayload {
                success: false,
                error: Some(format!("Reconnect failed: {}", e)),
                resolved_path: path,
                connected: false,
                stored: false,
                message: None,
            },
        };

        Ok(payload)
    }
}

async fn run_create_directory(
    ctx: &Context<'_>,
    input: &CreateDirectoryInput,
) -> Result<FileOperationPayload> {
    let path = PathBuf::from(input.path.trim());
    if path.as_os_str().is_empty() {
        return Ok(FileOperationPayload(FileOperationResult::err(
            "Path must not be empty",
        )));
    }
    match fs::create_dir_all(&path).await {
        Ok(_) => {
            use crate::services::graphql::subscriptions::filesystem::{
                FilesystemChangeBroker, FilesystemChangeEvent,
            };
            if let Some(broker) = ctx.data_opt::<Arc<FilesystemChangeBroker>>() {
                broker.send(FilesystemChangeEvent {
                    path: path.to_string_lossy().into_owned(),
                    change_type: "created".into(),
                    name: path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned()),
                    new_name: None,
                    timestamp: time::OffsetDateTime::now_utc()
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default(),
                });
            }
            Ok(FileOperationPayload(FileOperationResult::ok(
                Some(path.to_string_lossy().into_owned()),
                1,
                vec![],
            )))
        }
        Err(e) => Ok(FileOperationPayload(FileOperationResult::err(format!(
            "Failed to create directory: {}",
            e
        )))),
    }
}
