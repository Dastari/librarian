use super::prelude::*;

#[derive(Default)]
pub struct MusicMutations;

#[Object]
impl MusicMutations {
    /// Add an album to a library from MusicBrainz
    async fn add_album(&self, ctx: &Context<'_>, input: AddAlbumInput) -> Result<AlbumResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let lib_id = Uuid::parse_str(&input.library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
        let mbid = Uuid::parse_str(&input.musicbrainz_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid MusicBrainz ID: {}", e)))?;

        // Verify library exists and belongs to user
        let library = db
            .libraries()
            .get_by_id(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Library not found"))?;

        if library.user_id != user_id {
            return Err(async_graphql::Error::new(
                "Not authorized to access this library",
            ));
        }

        // Add album from MusicBrainz
        use crate::services::metadata::AddAlbumOptions;
        match metadata
            .add_album_from_provider(AddAlbumOptions {
                musicbrainz_id: mbid,
                library_id: lib_id,
                user_id,
                monitored: true,
            })
            .await
        {
            Ok(record) => {
                tracing::info!(
                    user_id = %user.user_id,
                    album_name = %record.name,
                    album_id = %record.id,
                    library_id = %lib_id,
                    "User added album: {}",
                    record.name
                );

                Ok(AlbumResult {
                    success: true,
                    album: Some(Album::from(record)),
                    error: None,
                })
            }
            Err(e) => Ok(AlbumResult {
                success: false,
                album: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete an album from a library
    async fn delete_album(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let album_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid album ID: {}", e)))?;

        // Verify album exists
        let album = db
            .albums()
            .get_by_id(album_id)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to get album: {}", e)))?;

        if album.is_none() {
            return Ok(MutationResult {
                success: false,
                error: Some("Album not found".to_string()),
            });
        }

        // Delete the album and all associated data
        match db.albums().delete(album_id).await {
            Ok(deleted) => {
                if deleted {
                    tracing::info!("Deleted album {}", album_id);
                    Ok(MutationResult {
                        success: true,
                        error: None,
                    })
                } else {
                    Ok(MutationResult {
                        success: false,
                        error: Some("Album not found".to_string()),
                    })
                }
            }
            Err(e) => {
                tracing::error!(album_id = %album_id, error = %e, "Failed to delete album");
                Ok(MutationResult {
                    success: false,
                    error: Some(format!("Failed to delete album: {}", e)),
                })
            }
        }
    }
}
