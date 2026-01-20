use super::prelude::*;

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
}
