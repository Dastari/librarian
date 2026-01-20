use super::prelude::*;

#[derive(Default)]
pub struct UserQueries;

#[Object]
impl UserQueries {
    /// Get the current authenticated user
    async fn me(&self, ctx: &Context<'_>) -> Result<User> {
        let user = ctx.auth_user()?;
        Ok(User {
            id: user.user_id.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
        })
    }

    /// Get user preferences
    async fn my_preferences(&self, ctx: &Context<'_>) -> Result<UserPreferences> {
        let _user = ctx.auth_user()?;
        // TODO: Fetch from database
        Ok(UserPreferences {
            theme: "system".to_string(),
            notifications_enabled: true,
        })
    }
}
