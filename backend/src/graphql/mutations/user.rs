use super::prelude::*;

#[derive(Default)]
pub struct UserMutations;

#[Object]
impl UserMutations {
    /// Update user preferences
    async fn update_preferences(
        &self,
        ctx: &Context<'_>,
        input: UpdatePreferencesInput,
    ) -> Result<UserPreferences> {
        let _user = ctx.auth_user()?;
        // TODO: Update in database
        Ok(UserPreferences {
            theme: input.theme.unwrap_or_else(|| "system".to_string()),
            notifications_enabled: input.notifications_enabled.unwrap_or(true),
        })
    }
}
