use super::prelude::*;

#[derive(Default)]
pub struct LibraryMutations;

#[Object]
impl LibraryMutations {
    /// Create a new library
    async fn create_library(
        &self,
        ctx: &Context<'_>,
        input: CreateLibraryInput,
    ) -> Result<LibraryResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let library_type = match input.library_type {
            LibraryType::Movies => "movies",
            LibraryType::Tv => "tv",
            LibraryType::Music => "music",
            LibraryType::Audiobooks => "audiobooks",
            LibraryType::Other => "other",
        }
        .to_string();

        let post_download_action = match input.post_download_action {
            Some(PostDownloadAction::Move) => "move",
            Some(PostDownloadAction::Hardlink) => "hardlink",
            _ => "copy",
        }
        .to_string();

        let record = db
            .libraries()
            .create(CreateLibrary {
                user_id,
                name: input.name,
                path: input.path,
                library_type,
                icon: input.icon,
                color: input.color,
                auto_scan: input.auto_scan.unwrap_or(true),
                scan_interval_minutes: input.scan_interval_minutes.unwrap_or(60),
                watch_for_changes: input.watch_for_changes.unwrap_or(false),
                post_download_action,
                organize_files: input.organize_files.unwrap_or(true),
                rename_style: input.rename_style.unwrap_or_else(|| "none".to_string()),
                naming_pattern: input.naming_pattern,
                auto_add_discovered: input.auto_add_discovered.unwrap_or(true),
                auto_download: input.auto_download.unwrap_or(true),
                auto_hunt: input.auto_hunt.unwrap_or(false),
                // Inline quality settings (empty = any)
                allowed_resolutions: input.allowed_resolutions.unwrap_or_default(),
                allowed_video_codecs: input.allowed_video_codecs.unwrap_or_default(),
                allowed_audio_formats: input.allowed_audio_formats.unwrap_or_default(),
                require_hdr: input.require_hdr.unwrap_or(false),
                allowed_hdr_types: input.allowed_hdr_types.unwrap_or_default(),
                allowed_sources: input.allowed_sources.unwrap_or_default(),
                release_group_blacklist: input.release_group_blacklist.unwrap_or_default(),
                release_group_whitelist: input.release_group_whitelist.unwrap_or_default(),
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::info!(
            user_id = %user.user_id,
            library_id = %record.id,
            library_name = %record.name,
            library_type = %record.library_type,
            "User created library: {}",
            record.name
        );

        // Spawn initial scan in background (same pattern as scan_library mutation)
        let library_id = record.id;
        let library_name = record.name.clone();
        let scanner = ctx.data_unchecked::<Arc<ScannerService>>().clone();
        let db_clone = db.clone();
        tokio::spawn(async move {
            tracing::info!("Starting initial scan for library '{}'", library_name);
            if let Err(e) = scanner.scan_library(library_id).await {
                tracing::error!("Initial scan failed for '{}': {}", library_name, e);
                if let Err(reset_err) = db_clone.libraries().set_scanning(library_id, false).await {
                    tracing::error!(library_id = %library_id, error = %reset_err, "Failed to reset scanning state");
                }
            }
        });

        let library = Library {
            id: record.id.to_string(),
            name: record.name,
            path: record.path,
            library_type: input.library_type,
            icon: record.icon.unwrap_or_else(|| "folder".to_string()),
            color: record.color.unwrap_or_else(|| "slate".to_string()),
            auto_scan: record.auto_scan,
            scan_interval_minutes: record.scan_interval_minutes,
            item_count: 0,
            total_size_bytes: 0,
            last_scanned_at: None,
            scanning: record.scanning,
        };

        // Emit library created event
        if let Ok(library_tx) = ctx.data::<tokio::sync::broadcast::Sender<LibraryChangedEvent>>() {
            let _ = library_tx.send(LibraryChangedEvent {
                change_type: LibraryChangeType::Created,
                library_id: library.id.clone(),
                library_name: Some(library.name.clone()),
                library: Some(library.clone()),
            });
        }

        // Check if this is a movie library and TMDB isn't configured
        if input.library_type == LibraryType::Movies {
            let settings = db.settings();
            let tmdb_configured = match settings.get_value::<String>("metadata.tmdb_api_key").await {
                Ok(Some(key)) if !key.is_empty() => true,
                _ => std::env::var("TMDB_API_KEY").map(|k| !k.is_empty()).unwrap_or(false),
            };

            if !tmdb_configured {
                // Create a notification warning about missing TMDB config
                if let Ok(notification_service) = ctx.data::<Arc<crate::services::NotificationService>>() {
                    let library_name = library.name.clone();
                    let notification_svc = notification_service.clone();
                    let notify_user_id = user_id;
                    tokio::spawn(async move {
                        if let Err(e) = notification_svc
                            .create_warning(
                                notify_user_id,
                                "Movie metadata provider not configured".to_string(),
                                format!(
                                    "The movie library '{}' was created, but TMDB API is not configured. \
                                    Movie metadata, posters, and search will be limited. \
                                    Add your TMDB API key in Settings → Metadata to enable full functionality.",
                                    library_name
                                ),
                                crate::db::NotificationCategory::Configuration,
                            )
                            .await
                        {
                            tracing::warn!("Failed to create TMDB warning notification: {}", e);
                        }
                    });
                }
            }
        }

        Ok(LibraryResult {
            success: true,
            library: Some(library),
            error: None,
        })
    }

    /// Update an existing library
    async fn update_library(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateLibraryInput,
    ) -> Result<LibraryResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let post_download_action = input.post_download_action.map(|a| {
            match a {
                PostDownloadAction::Move => "move",
                PostDownloadAction::Hardlink => "hardlink",
                PostDownloadAction::Copy => "copy",
            }
            .to_string()
        });

        let result = db
            .libraries()
            .update(
                lib_id,
                UpdateLibrary {
                    name: input.name,
                    path: input.path,
                    icon: input.icon,
                    color: input.color,
                    auto_scan: input.auto_scan,
                    scan_interval_minutes: input.scan_interval_minutes,
                    watch_for_changes: input.watch_for_changes,
                    post_download_action,
                    organize_files: input.organize_files,
                    rename_style: input.rename_style,
                    naming_pattern: input.naming_pattern,
                    auto_add_discovered: input.auto_add_discovered,
                    auto_download: input.auto_download,
                    auto_hunt: input.auto_hunt,
                    // Inline quality settings
                    allowed_resolutions: input.allowed_resolutions,
                    allowed_video_codecs: input.allowed_video_codecs,
                    allowed_audio_formats: input.allowed_audio_formats,
                    require_hdr: input.require_hdr,
                    allowed_hdr_types: input.allowed_hdr_types,
                    allowed_sources: input.allowed_sources,
                    release_group_blacklist: input.release_group_blacklist,
                    release_group_whitelist: input.release_group_whitelist,
                },
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(record) = result {
            let library = Library {
                id: record.id.to_string(),
                name: record.name,
                path: record.path,
                library_type: match record.library_type.as_str() {
                    "movies" => LibraryType::Movies,
                    "tv" => LibraryType::Tv,
                    "music" => LibraryType::Music,
                    "audiobooks" => LibraryType::Audiobooks,
                    _ => LibraryType::Other,
                },
                icon: record.icon.unwrap_or_else(|| "folder".to_string()),
                color: record.color.unwrap_or_else(|| "slate".to_string()),
                auto_scan: record.auto_scan,
                scan_interval_minutes: record.scan_interval_minutes,
                item_count: 0,
                total_size_bytes: 0,
                last_scanned_at: record.last_scanned_at.map(|t| t.to_rfc3339()),
                scanning: record.scanning,
            };

            // Emit library updated event
            if let Ok(library_tx) =
                ctx.data::<tokio::sync::broadcast::Sender<LibraryChangedEvent>>()
            {
                let _ = library_tx.send(LibraryChangedEvent {
                    change_type: LibraryChangeType::Updated,
                    library_id: library.id.clone(),
                    library_name: Some(library.name.clone()),
                    library: Some(library.clone()),
                });
            }

            Ok(LibraryResult {
                success: true,
                library: Some(library),
                error: None,
            })
        } else {
            Ok(LibraryResult {
                success: false,
                library: None,
                error: Some("Library not found".to_string()),
            })
        }
    }

    /// Delete a library
    async fn delete_library(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        // Get library name before deleting for the event
        let library_name = db
            .libraries()
            .get_by_id(lib_id)
            .await
            .ok()
            .flatten()
            .map(|lib| lib.name);

        let deleted = db
            .libraries()
            .delete(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Emit library deleted event
        if deleted {
            if let Ok(library_tx) =
                ctx.data::<tokio::sync::broadcast::Sender<LibraryChangedEvent>>()
            {
                let _ = library_tx.send(LibraryChangedEvent {
                    change_type: LibraryChangeType::Deleted,
                    library_id: id.clone(),
                    library_name,
                    library: None,
                });
            }
        }

        Ok(MutationResult {
            success: deleted,
            error: if deleted {
                None
            } else {
                Some("Library not found".to_string())
            },
        })
    }

    /// Trigger a library scan
    async fn scan_library(&self, ctx: &Context<'_>, id: String) -> Result<ScanStatus> {
        let _user = ctx.auth_user()?;
        let scanner = ctx.data_unchecked::<Arc<ScannerService>>();
        let db = ctx.data_unchecked::<Database>().clone();

        let library_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        tracing::info!(library_id = %id, "Scan requested for library");

        // Spawn the scan in the background so the mutation returns immediately
        let scanner = scanner.clone();
        tokio::spawn(async move {
            tracing::debug!(library_id = %library_id, "Scan task started");
            match scanner.scan_library(library_id).await {
                Ok(progress) => {
                    tracing::info!(
                        library_id = %library_id,
                        total_files = progress.total_files,
                        new_files = progress.new_files,
                        "Library scan completed successfully"
                    );
                }
                Err(e) => {
                    tracing::error!(library_id = %library_id, error = %e, "Library scan failed");
                    // Ensure scanning state is reset on error
                    if let Err(reset_err) = db.libraries().set_scanning(library_id, false).await {
                        tracing::error!(library_id = %library_id, error = %reset_err, "Failed to reset scanning state");
                    }
                }
            }
        });

        Ok(ScanStatus {
            library_id: id,
            status: "started".to_string(),
            message: Some("Scan has been started".to_string()),
        })
    }

    /// Consolidate library folders - merge duplicate show folders, update paths
    /// This is useful after changing naming conventions to clean up old folder structures
    async fn consolidate_library(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<ConsolidateLibraryResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let library_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        tracing::debug!("Consolidation requested for library {}", id);

        let organizer = crate::services::OrganizerService::new(db.clone());

        match organizer.consolidate_library(library_id).await {
            Ok(result) => {
                tracing::info!(
                    library_id = %id,
                    folders_removed = result.folders_removed,
                    files_moved = result.files_moved,
                    "Library consolidation complete"
                );
                Ok(ConsolidateLibraryResult {
                    success: result.success,
                    folders_removed: result.folders_removed,
                    files_moved: result.files_moved,
                    messages: result.messages,
                })
            }
            Err(e) => {
                tracing::error!(library_id = %id, error = %e, "Library consolidation failed");
                Ok(ConsolidateLibraryResult {
                    success: false,
                    folders_removed: 0,
                    files_moved: 0,
                    messages: vec![format!("Consolidation failed: {}", e)],
                })
            }
        }
    }
}
