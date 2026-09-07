//! GraphQL filesystem mutations currently exposed through legacy root fields:
//! CreateDirectory, DeleteFiles, CopyFiles, MoveFiles, RenameFile.
//! CreateDirectory is implemented with tokio::fs; others return "Filesystem service not configured"
//! until Arc<FilesystemService> (or inline impl) is added.
//! When implementing real ops, get FilesystemChangeBroker from ctx and call .send(FilesystemChangeEvent { ... })
//! after each successful mutation so FilesystemChanged subscription receives events.

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
#[graphql(name = "DeleteFilesInput")]
pub struct DeleteFilesInput {
    #[graphql(name = "paths")]
    pub paths: Vec<String>,
    #[graphql(name = "recursive")]
    pub recursive: Option<bool>,
}

#[derive(InputObject)]
#[graphql(name = "CopyFilesInput")]
pub struct CopyFilesInput {
    #[graphql(name = "sources")]
    pub sources: Vec<String>,
    #[graphql(name = "destination")]
    pub destination: String,
    #[graphql(name = "overwrite")]
    pub overwrite: Option<bool>,
}

#[derive(InputObject)]
#[graphql(name = "MoveFilesInput")]
pub struct MoveFilesInput {
    #[graphql(name = "sources")]
    pub sources: Vec<String>,
    #[graphql(name = "destination")]
    pub destination: String,
    #[graphql(name = "overwrite")]
    pub overwrite: Option<bool>,
}

#[derive(InputObject)]
#[graphql(name = "RenameFileInput")]
pub struct RenameFileInput {
    #[graphql(name = "path")]
    pub path: String,
    #[graphql(name = "newName")]
    pub new_name: String,
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

    #[graphql(name = "deleteFiles")]
    async fn delete_files(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: DeleteFilesInput,
    ) -> Result<FileOperationPayload> {
        ctx.require_admin()?;
        run_delete_files(ctx, &input).await
    }

    #[graphql(name = "copyFiles")]
    async fn copy_files(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: CopyFilesInput,
    ) -> Result<FileOperationPayload> {
        ctx.require_admin()?;
        run_copy_files(ctx, &input).await
    }

    #[graphql(name = "moveFiles")]
    async fn move_files(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: MoveFilesInput,
    ) -> Result<FileOperationPayload> {
        ctx.require_admin()?;
        run_move_files(ctx, &input).await
    }

    #[graphql(name = "renameFile")]
    async fn rename_file(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: RenameFileInput,
    ) -> Result<FileOperationPayload> {
        ctx.require_admin()?;
        run_rename_file(ctx, &input).await
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

fn not_configured() -> Result<FileOperationPayload> {
    Ok(FileOperationPayload(FileOperationResult::err(
        "Filesystem service not configured",
    )))
}

async fn run_create_directory(
    _ctx: &Context<'_>,
    input: &CreateDirectoryInput,
) -> Result<FileOperationPayload> {
    let path = PathBuf::from(input.path.trim());
    if path.as_os_str().is_empty() {
        return Ok(FileOperationPayload(FileOperationResult::err(
            "Path must not be empty",
        )));
    }
    match fs::create_dir_all(&path).await {
        Ok(_) => Ok(FileOperationPayload(FileOperationResult::ok(
            Some(path.to_string_lossy().into_owned()),
            1,
            vec![],
        ))),
        Err(e) => Ok(FileOperationPayload(FileOperationResult::err(format!(
            "Failed to create directory: {}",
            e
        )))),
    }
}

async fn run_delete_files(
    _ctx: &Context<'_>,
    input: &DeleteFilesInput,
) -> Result<FileOperationPayload> {
    let _ = input;
    not_configured()
}

async fn run_copy_files(
    _ctx: &Context<'_>,
    input: &CopyFilesInput,
) -> Result<FileOperationPayload> {
    let _ = input;
    not_configured()
}

async fn run_move_files(
    _ctx: &Context<'_>,
    input: &MoveFilesInput,
) -> Result<FileOperationPayload> {
    let _ = input;
    not_configured()
}

async fn run_rename_file(
    _ctx: &Context<'_>,
    input: &RenameFileInput,
) -> Result<FileOperationPayload> {
    let _ = input;
    not_configured()
}
