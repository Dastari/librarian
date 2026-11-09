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
use crate::services::library_storage::SmbLocation;
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

/// Sign in to a network share to check it, and keep the sign-in for the share.
#[derive(InputObject)]
#[graphql(name = "ConnectShareInput")]
pub struct ConnectShareInput {
    /// Host name or IP address of the file server.
    #[graphql(name = "host")]
    pub host: String,
    /// Port, when it is not 445.
    #[graphql(name = "port")]
    pub port: Option<i32>,
    /// Share name, such as `media`.
    #[graphql(name = "share")]
    pub share: String,
    /// Blank for the saved sign-in, or a guest share when none is saved.
    #[graphql(name = "username")]
    pub username: Option<String>,
    /// Blank keeps the saved password when the user name is the saved one.
    #[graphql(name = "password")]
    pub password: Option<String>,
    /// Keep the sign-in, encrypted, if it works. On by default.
    #[graphql(name = "save")]
    pub save: Option<bool>,
}

/// Move a library to a folder its files were copied to.
#[derive(InputObject)]
#[graphql(name = "MoveLibraryInput", rename_fields = "camelCase")]
pub struct MoveLibraryInput {
    pub library_id: String,
    /// The new folder: an absolute path, or `smb://host/share/folder`.
    pub new_path: String,
    /// Move even though some files are not on the new folder yet. They are then reported
    /// missing by the next scan.
    pub allow_missing: Option<bool>,
}

#[derive(Clone, async_graphql::SimpleObject)]
#[graphql(name = "MoveLibraryResult", rename_fields = "camelCase")]
pub struct MoveLibraryResult {
    pub success: bool,
    pub error: Option<String>,
    /// File records rewritten to the new folder.
    pub moved_files: i32,
    /// Show, album and audiobook folders rewritten.
    pub moved_folders: i32,
    /// Open scan issues closed, because they named paths that no longer apply.
    pub closed_issues: i32,
}

#[derive(Clone)]
pub struct ConnectSharePayload {
    pub success: bool,
    pub error: Option<String>,
    pub share: Option<String>,
    pub folders: i32,
}

#[Object]
impl ConnectSharePayload {
    #[graphql(name = "success")]
    async fn success(&self) -> bool {
        self.success
    }

    #[graphql(name = "error")]
    async fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The share's normalised `smb://host[:port]/share` address.
    #[graphql(name = "share")]
    async fn share(&self) -> Option<&str> {
        self.share.as_deref()
    }

    /// Folders at the top of the share.
    #[graphql(name = "folders")]
    async fn folders(&self) -> i32 {
        self.folders
    }
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

    /// Sign in to a network share, check that it can be read, and keep the sign-in. The answer
    /// names what is wrong in words a person can act on (a name that does not resolve here, a
    /// refused password, a share that does not exist).
    #[graphql(name = "connectShare")]
    async fn connect_share(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: ConnectShareInput,
    ) -> Result<ConnectSharePayload> {
        ctx.require_admin()?;
        let db = ctx.data::<Database>()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let failed = |error: String| ConnectSharePayload {
            success: false,
            error: Some(error),
            share: None,
            folders: 0,
        };
        let host = input.host.trim();
        if host.is_empty() || input.share.trim().is_empty() {
            return Ok(failed("Enter the server and the share name".to_string()));
        }
        let port = match input.port {
            Some(port) if (1..=65535).contains(&port) => format!(":{port}"),
            Some(_) => return Ok(failed("The port must be between 1 and 65535".to_string())),
            None => String::new(),
        };
        let location = match SmbLocation::parse(&format!(
            "smb://{host}{port}/{}",
            input.share.trim().trim_matches('/')
        )) {
            Ok(location) => location,
            Err(error) => return Ok(failed(error.to_string())),
        };
        match filesystem_network::connect_share(
            db,
            services,
            &location,
            input.username.as_deref().unwrap_or(""),
            input.password.as_deref().unwrap_or(""),
            input.save.unwrap_or(true),
        )
        .await
        {
            Ok(connection) => Ok(ConnectSharePayload {
                success: true,
                error: None,
                share: Some(connection.share),
                folders: i32::try_from(connection.folders).unwrap_or(i32::MAX),
            }),
            Err(error) => Ok(failed(error.to_string())),
        }
    }

    /// Change a library's folder after its files were copied there, keeping every record: the
    /// files keep their ids, matches, progress and playlists. Run a scan afterwards to verify.
    #[graphql(name = "moveLibrary")]
    async fn move_library(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: MoveLibraryInput,
    ) -> Result<MoveLibraryResult> {
        ctx.require_admin()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let scan = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        let allow_missing = input.allow_missing.unwrap_or(false);
        // Detached from this request, so a closed browser tab cannot stop it half way.
        let task = tokio::spawn(async move {
            scan.apply_library_move(&input.library_id, &input.new_path, allow_missing)
                .await
        });
        let count = |value: usize| i32::try_from(value).unwrap_or(i32::MAX);
        Ok(match task.await {
            Ok(Ok(outcome)) => MoveLibraryResult {
                success: true,
                error: None,
                moved_files: count(outcome.moved_files),
                moved_folders: count(outcome.moved_folders),
                closed_issues: count(outcome.closed_issues),
            },
            Ok(Err(error)) => MoveLibraryResult {
                success: false,
                error: Some(format!("{error:#}")),
                moved_files: 0,
                moved_folders: 0,
                closed_issues: 0,
            },
            Err(error) => MoveLibraryResult {
                success: false,
                error: Some(format!("The move stopped unexpectedly: {error}")),
                moved_files: 0,
                moved_folders: 0,
                closed_issues: 0,
            },
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
    // A folder on a share is made through it, not as a local folder named `smb:`.
    let made = if crate::services::library_storage::is_smb_uri(input.path.trim()) {
        crate::services::library_storage::library_storage()
            .create_dir_all(input.path.trim())
            .await
            .map_err(|error| std::io::Error::other(error.to_string()))
    } else {
        fs::create_dir_all(&path).await
    };
    match made {
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
