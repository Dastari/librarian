use super::prelude::*;

#[derive(Default)]
pub struct FilesystemMutations;

#[Object]
impl FilesystemMutations {
    /// Create a directory on the filesystem
    async fn create_directory(
        &self,
        ctx: &Context<'_>,
        input: CreateDirectoryInput,
    ) -> Result<FileOperationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match fs_service.create_directory(&input.path, user_id).await {
            Ok(path) => Ok(FileOperationResult {
                success: true,
                error: None,
                affected_count: 1,
                messages: vec![format!("Created directory: {}", path)],
                path: Some(path),
            }),
            Err(e) => Ok(FileOperationResult {
                success: false,
                error: Some(e.to_string()),
                affected_count: 0,
                messages: vec![],
                path: None,
            }),
        }
    }

    /// Delete files or directories (must be inside a library)
    async fn delete_files(
        &self,
        ctx: &Context<'_>,
        input: DeleteFilesInput,
    ) -> Result<FileOperationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match fs_service
            .delete_files(&input.paths, input.recursive.unwrap_or(false), user_id)
            .await
        {
            Ok((count, messages)) => Ok(FileOperationResult {
                success: count > 0,
                error: if count == 0 {
                    Some("No files were deleted".to_string())
                } else {
                    None
                },
                affected_count: count,
                messages,
                path: None,
            }),
            Err(e) => Ok(FileOperationResult {
                success: false,
                error: Some(e.to_string()),
                affected_count: 0,
                messages: vec![],
                path: None,
            }),
        }
    }

    /// Copy files or directories (source and destination must be inside libraries)
    async fn copy_files(
        &self,
        ctx: &Context<'_>,
        input: CopyFilesInput,
    ) -> Result<FileOperationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match fs_service
            .copy_files(
                &input.sources,
                &input.destination,
                input.overwrite.unwrap_or(false),
                user_id,
            )
            .await
        {
            Ok((count, messages)) => Ok(FileOperationResult {
                success: count > 0,
                error: if count == 0 {
                    Some("No files were copied".to_string())
                } else {
                    None
                },
                affected_count: count,
                messages,
                path: None,
            }),
            Err(e) => Ok(FileOperationResult {
                success: false,
                error: Some(e.to_string()),
                affected_count: 0,
                messages: vec![],
                path: None,
            }),
        }
    }

    /// Move files or directories (source and destination must be inside libraries)
    async fn move_files(
        &self,
        ctx: &Context<'_>,
        input: MoveFilesInput,
    ) -> Result<FileOperationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match fs_service
            .move_files(
                &input.sources,
                &input.destination,
                input.overwrite.unwrap_or(false),
                user_id,
            )
            .await
        {
            Ok((count, messages)) => Ok(FileOperationResult {
                success: count > 0,
                error: if count == 0 {
                    Some("No files were moved".to_string())
                } else {
                    None
                },
                affected_count: count,
                messages,
                path: None,
            }),
            Err(e) => Ok(FileOperationResult {
                success: false,
                error: Some(e.to_string()),
                affected_count: 0,
                messages: vec![],
                path: None,
            }),
        }
    }

    /// Rename a file or directory (must be inside a library)
    async fn rename_file(
        &self,
        ctx: &Context<'_>,
        input: RenameFileInput,
    ) -> Result<FileOperationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match fs_service
            .rename_file(&input.path, &input.new_name, user_id)
            .await
        {
            Ok(new_path) => Ok(FileOperationResult {
                success: true,
                error: None,
                affected_count: 1,
                messages: vec![format!("Renamed to: {}", new_path)],
                path: Some(new_path),
            }),
            Err(e) => Ok(FileOperationResult {
                success: false,
                error: Some(e.to_string()),
                affected_count: 0,
                messages: vec![],
                path: None,
            }),
        }
    }

    /// Manually trigger auto-hunt for a specific library
    ///
    /// This immediately searches indexers for missing content in the library.
    /// Returns the number of items searched, matched, and downloaded.
    async fn trigger_auto_hunt(
        &self,
        ctx: &Context<'_>,
        library_id: String,
    ) -> Result<AutoHuntResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();

        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        // Verify the library exists and belongs to this user
        let library = db
            .libraries()
            .get_by_id(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Database error: {}", e)))?
            .ok_or_else(|| async_graphql::Error::new("Library not found"))?;

        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        if library.user_id != user_id {
            return Err(async_graphql::Error::new("Library not found"));
        }

        // Get encryption key and create IndexerManager
        let encryption_key = db
            .settings()
            .get_or_create_indexer_encryption_key()
            .await
            .map_err(|e| {
                async_graphql::Error::new(format!("Failed to get encryption key: {}", e))
            })?;

        let indexer_manager =
            crate::indexer::manager::IndexerManager::new(db.clone(), &encryption_key)
                .await
                .map_err(|e| {
                    async_graphql::Error::new(format!("Failed to create IndexerManager: {}", e))
                })?;

        // Load user's indexers
        indexer_manager
            .load_user_indexers(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to load indexers: {}", e)))?;

        let indexer_manager = std::sync::Arc::new(indexer_manager);

        // Run auto-hunt for this library (case-insensitive)
        let result = match library.library_type.to_lowercase().as_str() {
            "movies" => {
                crate::jobs::auto_hunt::hunt_movies_for_library(
                    db,
                    &library,
                    torrent_service,
                    &indexer_manager,
                )
                .await
            }
            "tv" => {
                crate::jobs::auto_hunt::hunt_tv_for_library(
                    db,
                    &library,
                    torrent_service,
                    &indexer_manager,
                )
                .await
            }
            _ => {
                return Ok(AutoHuntResult {
                    success: false,
                    error: Some(format!(
                        "Auto-hunt not yet supported for {} libraries",
                        library.library_type
                    )),
                    searched: 0,
                    matched: 0,
                    downloaded: 0,
                    skipped: 0,
                    failed: 0,
                });
            }
        };

        match result {
            Ok(hunt_result) => {
                tracing::info!(
                    user_id = %user.user_id,
                    library_id = %library_id,
                    library_name = %library.name,
                    searched = hunt_result.searched,
                    matched = hunt_result.matched,
                    downloaded = hunt_result.downloaded,
                    "Manual auto-hunt completed"
                );

                Ok(AutoHuntResult {
                    success: true,
                    error: None,
                    searched: hunt_result.searched,
                    matched: hunt_result.matched,
                    downloaded: hunt_result.downloaded,
                    skipped: hunt_result.skipped,
                    failed: hunt_result.failed,
                })
            }
            Err(e) => Ok(AutoHuntResult {
                success: false,
                error: Some(e.to_string()),
                searched: 0,
                matched: 0,
                downloaded: 0,
                skipped: 0,
                failed: 0,
            }),
        }
    }
}
