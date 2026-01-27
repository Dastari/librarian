//! GraphQL filesystem mutations (PascalCase): CreateDirectory, DeleteFiles, CopyFiles, MoveFiles, RenameFile.
//! CreateDirectory is implemented with tokio::fs; others return "Filesystem service not configured"
//! until Arc<FilesystemService> (or inline impl) is added.
//! When implementing real ops, get FilesystemChangeBroker from ctx and call .send(FilesystemChangeEvent { ... })
//! after each successful mutation so FilesystemChanged subscription receives events.

use std::path::PathBuf;

use async_graphql::{Context, InputObject, Object, Result};
use tokio::fs;

use crate::services::graphql::auth::AuthUser;

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

async fn run_delete_files(_ctx: &Context<'_>, input: &DeleteFilesInput) -> Result<FileOperationPayload> {
    let _ = input;
    not_configured()
}

async fn run_copy_files(_ctx: &Context<'_>, input: &CopyFilesInput) -> Result<FileOperationPayload> {
    let _ = input;
    not_configured()
}

async fn run_move_files(_ctx: &Context<'_>, input: &MoveFilesInput) -> Result<FileOperationPayload> {
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
