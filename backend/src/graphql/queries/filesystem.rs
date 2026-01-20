use super::prelude::*;

#[derive(Default)]
pub struct FilesystemQueries;

#[Object]
impl FilesystemQueries {
    /// Browse a directory on the server filesystem
    async fn browse_directory(
        &self,
        ctx: &Context<'_>,
        input: Option<BrowseDirectoryInput>,
    ) -> Result<BrowseDirectoryResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let input = input.unwrap_or_else(|| BrowseDirectoryInput {
            path: None,
            dirs_only: Some(true),
            show_hidden: Some(false),
        });

        let result = fs_service
            .browse(
                input.path.as_deref(),
                input.dirs_only.unwrap_or(true),
                input.show_hidden.unwrap_or(false),
                user_id,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(BrowseDirectoryResult {
            current_path: result.current_path,
            parent_path: result.parent_path,
            entries: result
                .entries
                .into_iter()
                .map(|e| FileEntry {
                    name: e.name,
                    path: e.path,
                    is_dir: e.is_dir,
                    size: e.size as i64,
                    size_formatted: format_bytes(e.size),
                    readable: e.readable,
                    writable: e.writable,
                    mime_type: e.mime_type,
                    modified_at: e.modified_at.map(|t| t.to_rfc3339()),
                })
                .collect(),
            quick_paths: result
                .quick_paths
                .into_iter()
                .map(|p| QuickPath {
                    name: p.name,
                    path: p.path,
                })
                .collect(),
            is_library_path: result.is_library_path,
            library_id: result.library_id.map(|id| id.to_string()),
        })
    }

    /// Get quick-access filesystem paths
    async fn quick_paths(&self, _ctx: &Context<'_>) -> Result<Vec<QuickPath>> {
        // Quick paths don't require auth - they're just common paths on the system
        let paths = crate::services::FilesystemService::get_quick_paths();
        Ok(paths
            .into_iter()
            .map(|p| QuickPath {
                name: p.name,
                path: p.path,
            })
            .collect())
    }

    /// Validate if a path is inside a library
    async fn validate_path(&self, ctx: &Context<'_>, path: String) -> Result<PathValidationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let validation = fs_service
            .validate_path(&path, user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(PathValidationResult {
            is_valid: validation.is_valid,
            is_library_path: validation.is_library_path,
            library_id: validation.library_id.map(|id| id.to_string()),
            library_name: validation.library_name,
            error: validation.error,
        })
    }
}
