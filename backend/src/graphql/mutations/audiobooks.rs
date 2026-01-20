use super::prelude::*;

#[derive(Default)]
pub struct AudiobookMutations;

#[Object]
impl AudiobookMutations {
    /// Add an audiobook to a library from OpenLibrary
    async fn add_audiobook(
        &self,
        ctx: &Context<'_>,
        input: AddAudiobookInput,
    ) -> Result<AudiobookResult> {
        let user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();
        let library_id = Uuid::parse_str(&input.library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
        use crate::services::metadata::AddAudiobookOptions;
        match metadata
            .add_audiobook_from_provider(AddAudiobookOptions {
                openlibrary_id: input.openlibrary_id,
                library_id,
                user_id,
                monitored: true,
            })
            .await
        {
            Ok(record) => Ok(AudiobookResult {
                success: true,
                audiobook: Some(Audiobook::from(record)),
                error: None,
            }),
            Err(e) => Ok(AudiobookResult {
                success: false,
                audiobook: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete an audiobook from a library
    async fn delete_audiobook(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let audiobook_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        // Verify audiobook exists
        let audiobook =
            db.audiobooks().get_by_id(audiobook_id).await.map_err(|e| {
                async_graphql::Error::new(format!("Failed to get audiobook: {}", e))
            })?;

        if audiobook.is_none() {
            return Ok(MutationResult {
                success: false,
                error: Some("Audiobook not found".to_string()),
            });
        }

        // Delete the audiobook and all associated data
        match db.audiobooks().delete(audiobook_id).await {
            Ok(deleted) => {
                if deleted {
                    tracing::info!(audiobook_id = %audiobook_id, "Deleted audiobook");
                    Ok(MutationResult {
                        success: true,
                        error: None,
                    })
                } else {
                    Ok(MutationResult {
                        success: false,
                        error: Some("Audiobook not found".to_string()),
                    })
                }
            }
            Err(e) => {
                tracing::error!(audiobook_id = %audiobook_id, error = %e, "Failed to delete audiobook");
                Ok(MutationResult {
                    success: false,
                    error: Some(format!("Failed to delete audiobook: {}", e)),
                })
            }
        }
    }
}
