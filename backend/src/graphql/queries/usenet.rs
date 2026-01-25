use super::prelude::*;

#[derive(Default)]
pub struct UsenetQueries;

#[Object]
impl UsenetQueries {
    /// Get all usenet servers for the current user
    async fn usenet_servers(&self, ctx: &Context<'_>) -> Result<Vec<UsenetServer>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .usenet_servers()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(UsenetServer::from).collect())
    }

    /// Get a specific usenet server by ID
    async fn usenet_server(&self, ctx: &Context<'_>, id: String) -> Result<Option<UsenetServer>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let server_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid server ID: {}", e)))?;

        let record = db
            .usenet_servers()
            .get(server_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Verify ownership
        if let Some(ref r) = record {
            let user_id = Uuid::parse_str(&user.user_id)?;
            if r.user_id != user_id {
                return Ok(None);
            }
        }

        Ok(record.map(UsenetServer::from))
    }

    /// Get all usenet downloads for the current user
    async fn usenet_downloads(&self, ctx: &Context<'_>) -> Result<Vec<UsenetDownload>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .usenet_downloads()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(UsenetDownload::from).collect())
    }

    /// Get a specific usenet download by ID
    async fn usenet_download(&self, ctx: &Context<'_>, id: String) -> Result<Option<UsenetDownload>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let download_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid download ID: {}", e)))?;

        let record = db
            .usenet_downloads()
            .get(download_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Verify ownership
        if let Some(ref r) = record {
            let user_id = Uuid::parse_str(&user.user_id)?;
            if r.user_id != user_id {
                return Ok(None);
            }
        }

        Ok(record.map(UsenetDownload::from))
    }
}
