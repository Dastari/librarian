use super::prelude::*;

use crate::services::AuthService;

#[derive(Default)]
pub struct SystemQueries;

#[Object]
impl SystemQueries {
    /// Health check (no auth required)
    async fn health(&self) -> Result<bool> {
        Ok(true)
    }

    /// Server version
    async fn version(&self) -> Result<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    /// Check if first-time setup is required (no admin exists)
    ///
    /// No authentication required. Returns true if the application needs
    /// initial setup (no admin user has been created yet).
    async fn needs_setup(&self, ctx: &Context<'_>) -> Result<bool> {
        let db = ctx.data_unchecked::<Database>();
        let auth_service = AuthService::with_env(db.clone());

        auth_service
            .needs_setup()
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to check setup status: {}", e)))
    }
}
