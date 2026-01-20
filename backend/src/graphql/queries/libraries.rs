use super::prelude::*;

#[derive(Default)]
pub struct LibraryQueries;

#[Object]
impl LibraryQueries {
    /// Get all libraries for the current user
    async fn libraries(&self, ctx: &Context<'_>) -> Result<Vec<LibraryFull>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .libraries()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut libraries = Vec::new();
        for r in records {
            let stats = match db.libraries().get_stats(r.id).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(library_id = %r.id, error = %e, "Failed to get library stats");
                    LibraryStats::default()
                }
            };

            libraries.push(LibraryFull::from_record_with_stats(r, stats));
        }

        Ok(libraries)
    }

    /// Get a specific library by ID
    async fn library(&self, ctx: &Context<'_>, id: String) -> Result<Option<LibraryFull>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let record = db
            .libraries()
            .get_by_id_and_user(lib_id, user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(r) = record {
            let stats = db.libraries().get_stats(r.id).await.unwrap_or_default();
            Ok(Some(LibraryFull::from_record_with_stats(r, stats)))
        } else {
            Ok(None)
        }
    }
}
