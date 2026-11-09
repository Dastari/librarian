//! Device and play-history actions with no generated equivalent (design.md Q133–Q134).
//!
//! Renaming and forgetting a device, clearing history and sharing statistics are the generated
//! `updateDevice`, `deleteDevice`, `deletePlayHistories` and `createStatsShare`/`deleteStatsShare`
//! with lifecycle hooks. These are the rest: a check-in that records what the server saw of
//! the request, signing a device out, and importing history from watch progress.

use async_graphql::{Context, ErrorExtensions, InputObject, Object, Result, SimpleObject};

use crate::db::Database;
use crate::graphql::auth::AuthExt;
use crate::graphql::entities::Device;
use crate::services::graphql::session_authority::SessionAuthority;
use crate::services::play_history::devices::{self, CheckIn, RequestClientInfo};
use crate::services::play_history::import_from_progress;

#[derive(InputObject)]
#[graphql(name = "CheckInDeviceInput", rename_fields = "camelCase")]
pub struct CheckInDeviceInput {
    /// The client's stable id, kept in its own storage.
    pub client_id: String,
    /// The client's suggested name ("Safari on iPad", "TV"); kept only until someone renames it.
    pub name: Option<String>,
    /// `tv`, `phone`, `tablet` or `desktop`.
    pub form_factor: Option<String>,
    pub platform: Option<String>,
    pub product: Option<String>,
    pub app_version: Option<String>,
}

#[derive(SimpleObject)]
#[graphql(name = "CheckInDevicePayload", rename_fields = "camelCase")]
pub struct CheckInDevicePayload {
    pub success: bool,
    pub error: Option<String>,
    pub device: Option<Device>,
}

#[derive(SimpleObject)]
#[graphql(name = "SignOutDevicePayload", rename_fields = "camelCase")]
pub struct SignOutDevicePayload {
    pub success: bool,
    pub error: Option<String>,
    /// Refresh tokens revoked (0 when the device had already signed out).
    pub revoked_sessions: i32,
}

#[derive(SimpleObject)]
#[graphql(name = "ImportPlayHistoryPayload", rename_fields = "camelCase")]
pub struct ImportPlayHistoryPayload {
    pub success: bool,
    pub error: Option<String>,
    pub imported: i32,
    pub skipped: i32,
}

#[derive(Default)]
pub struct DeviceActionMutations;

#[Object]
impl DeviceActionMutations {
    /// Add or refresh this client as one of your devices. Call it when the app starts, on a
    /// sign-in, and now and then while it runs (last seen).
    #[graphql(name = "checkInDevice")]
    async fn check_in_device(
        &self,
        ctx: &Context<'_>,
        input: CheckInDeviceInput,
    ) -> Result<CheckInDevicePayload> {
        let user = ctx.require_member()?;
        let db = ctx.data::<Database>()?;
        let seen = ctx
            .data_opt::<RequestClientInfo>()
            .cloned()
            .unwrap_or_default();
        let session_id = ctx
            .data_opt::<SessionAuthority>()
            .map(|authority| authority.0.session_id.to_string());
        let device = devices::check_in(
            db,
            user,
            session_id,
            &seen,
            CheckIn {
                client_id: input.client_id,
                name: input.name,
                form_factor: input.form_factor,
                platform: input.platform,
                product: input.product,
                app_version: input.app_version,
            },
        )
        .await?;
        Ok(CheckInDevicePayload {
            success: true,
            error: None,
            device: Some(device),
        })
    }

    /// Sign a device out: its login session is revoked and what it plays stops. Your own
    /// devices, or anyone's for an administrator.
    #[graphql(name = "signOutDevice")]
    async fn sign_out_device(&self, ctx: &Context<'_>, id: String) -> Result<SignOutDevicePayload> {
        let user = ctx.require_member()?;
        let db = ctx.data::<Database>()?;
        let device = Device::get(db.pool(), &id)
            .await?
            .filter(|device| user.is_admin() || device.user_id == user.user_id)
            .ok_or_else(|| {
                async_graphql::Error::new("Device not found")
                    .extend_with(|_, extensions| extensions.set("code", "NOT_FOUND"))
            })?;
        let revoked = devices::sign_out(db, &device).await?;
        tracing::info!(
            device_id = %device.id,
            device = %device.name,
            owner = %device.user_id,
            by = %user.user_id,
            revoked,
            "Signed out device '{}' ({revoked} refresh tokens revoked)",
            device.name
        );
        Ok(SignOutDevicePayload {
            success: true,
            error: None,
            revoked_sessions: revoked.clamp(0, i32::MAX as i64) as i32,
        })
    }

    /// Seed play history from watch progress: one completed play per watched item, with no
    /// time claimed, marked imported. Yours, or everyone's (`allUsers`, administrators).
    /// Each checkpoint is imported once, so running it again only adds what is new.
    #[graphql(name = "importPlayHistory")]
    async fn import_play_history(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "allUsers", default = false)] all_users: bool,
    ) -> Result<ImportPlayHistoryPayload> {
        let user = if all_users {
            ctx.require_admin()?
        } else {
            ctx.require_member()?
        };
        let db = ctx.data::<Database>()?;
        let outcome =
            import_from_progress(db, (!all_users).then_some(user.user_id.as_str())).await?;
        Ok(ImportPlayHistoryPayload {
            success: true,
            error: None,
            imported: outcome.imported.min(i32::MAX as u64) as i32,
            skipped: outcome.skipped.min(i32::MAX as u64) as i32,
        })
    }
}
