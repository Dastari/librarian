//! GraphQL filesystem mutations (PascalCase): CreateDirectory, DeleteFiles, CopyFiles, MoveFiles, RenameFile.
//! CreateDirectory is implemented with tokio::fs; others return "Filesystem service not configured"
//! until Arc<FilesystemService> (or inline impl) is added.
//! When implementing real ops, get FilesystemChangeBroker from ctx and call .send(FilesystemChangeEvent { ... })
//! after each successful mutation so FilesystemChanged subscription receives events.

use std::path::PathBuf;
use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result};
use tokio::fs;

use crate::services::graphql::auth::AuthUser;
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

/// Payload returned by all filesystem mutations (PascalCase).
#[derive(Clone)]
pub struct FileOperationPayload(FileOperationResult);

#[Object]
impl FileOperationPayload {
    #[graphql(name = "Success")]
    async fn success(&self) -> bool {
        self.0.success
    }

    #[graphql(name = "Error")]
    async fn error(&self) -> Option<&str> {
        self.0.error.as_deref()
    }

    #[graphql(name = "AffectedCount")]
    async fn affected_count(&self) -> i32 {
        self.0.affected_count
    }

    #[graphql(name = "Messages")]
    async fn messages(&self) -> &[String] {
        &self.0.messages
    }

    #[graphql(name = "Path")]
    async fn path(&self) -> Option<&str> {
        self.0.path.as_deref()
    }
}

// ---------------------------------------------------------------------------
// Input types (PascalCase)
// ---------------------------------------------------------------------------

#[derive(InputObject)]
#[graphql(name = "CreateDirectoryInput")]
pub struct CreateDirectoryInput {
    #[graphql(name = "Path")]
    pub path: String,
}

#[derive(InputObject)]
#[graphql(name = "DeleteFilesInput")]
pub struct DeleteFilesInput {
    #[graphql(name = "Paths")]
    pub paths: Vec<String>,
    #[graphql(name = "Recursive")]
    pub recursive: Option<bool>,
}

#[derive(InputObject)]
#[graphql(name = "CopyFilesInput")]
pub struct CopyFilesInput {
    #[graphql(name = "Sources")]
    pub sources: Vec<String>,
    #[graphql(name = "Destination")]
    pub destination: String,
    #[graphql(name = "Overwrite")]
    pub overwrite: Option<bool>,
}

#[derive(InputObject)]
#[graphql(name = "MoveFilesInput")]
pub struct MoveFilesInput {
    #[graphql(name = "Sources")]
    pub sources: Vec<String>,
    #[graphql(name = "Destination")]
    pub destination: String,
    #[graphql(name = "Overwrite")]
    pub overwrite: Option<bool>,
}

#[derive(InputObject)]
#[graphql(name = "RenameFileInput")]
pub struct RenameFileInput {
    #[graphql(name = "Path")]
    pub path: String,
    #[graphql(name = "NewName")]
    pub new_name: String,
}

#[derive(InputObject)]
#[graphql(name = "ConfigureNetworkPathInput")]
pub struct ConfigureNetworkPathInput {
    #[graphql(name = "Path")]
    pub path: String,
    #[graphql(name = "Username")]
    pub username: Option<String>,
    #[graphql(name = "Password")]
    pub password: Option<String>,
    #[graphql(name = "MountPoint")]
    pub mount_point: Option<String>,
    #[graphql(name = "Persist")]
    pub persist: Option<bool>,
    #[graphql(name = "AttemptConnect")]
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
    #[graphql(name = "Success")]
    async fn success(&self) -> bool {
        self.success
    }

    #[graphql(name = "Error")]
    async fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    #[graphql(name = "ResolvedPath")]
    async fn resolved_path(&self) -> &str {
        &self.resolved_path
    }

    #[graphql(name = "Connected")]
    async fn connected(&self) -> bool {
        self.connected
    }

    #[graphql(name = "Stored")]
    async fn stored(&self) -> bool {
        self.stored
    }

    #[graphql(name = "Message")]
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
    #[graphql(name = "CreateDirectory")]
    async fn create_directory(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: CreateDirectoryInput,
    ) -> Result<FileOperationPayload> {
        let _user = ctx
            .data_opt::<AuthUser>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;
        run_create_directory(ctx, &input).await
    }

    #[graphql(name = "DeleteFiles")]
    async fn delete_files(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: DeleteFilesInput,
    ) -> Result<FileOperationPayload> {
        let _user = ctx
            .data_opt::<AuthUser>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;
        run_delete_files(ctx, &input).await
    }

    #[graphql(name = "CopyFiles")]
    async fn copy_files(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: CopyFilesInput,
    ) -> Result<FileOperationPayload> {
        let _user = ctx
            .data_opt::<AuthUser>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;
        run_copy_files(ctx, &input).await
    }

    #[graphql(name = "MoveFiles")]
    async fn move_files(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: MoveFilesInput,
    ) -> Result<FileOperationPayload> {
        let _user = ctx
            .data_opt::<AuthUser>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;
        run_move_files(ctx, &input).await
    }

    #[graphql(name = "RenameFile")]
    async fn rename_file(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: RenameFileInput,
    ) -> Result<FileOperationPayload> {
        let _user = ctx
            .data_opt::<AuthUser>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;
        run_rename_file(ctx, &input).await
    }

    #[graphql(name = "ConfigureNetworkPath")]
    async fn configure_network_path(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: ConfigureNetworkPathInput,
    ) -> Result<NetworkPathConfigPayload> {
        let _user = ctx
            .data_opt::<AuthUser>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

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

    #[graphql(name = "ReconnectLibraryPath")]
    async fn reconnect_library_path(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Path")] path: String,
    ) -> Result<NetworkPathConfigPayload> {
        let _user = ctx
            .data_opt::<AuthUser>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required"))?;

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
