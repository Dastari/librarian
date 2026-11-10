//! Account security actions (design.md Q147–Q148): the second sign-in step, a person's own
//! password and two-factor sign-in, and an administrator's account recovery. These are actions
//! with no generated equivalent; the profile fields themselves are edited with `updateUser`.

use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use super::auth::AuthPayload;
use crate::graphql::auth::AuthExt;
use crate::services::auth::{AuthService, SignIn};
use crate::services::graphql::error::{internal_message, service_unavailable};

fn auth<'a>(ctx: &'a Context<'_>) -> Result<&'a Arc<AuthService>> {
    ctx.data::<Arc<AuthService>>()
        .map_err(|_| service_unavailable("Authentication"))
}

/// Messages written for the person; anything else becomes an internal reference.
const PUBLIC_MESSAGES: &[&str] = &[
    crate::services::login_rate_limit::RATE_LIMITED_MESSAGE,
    "This sign-in has expired. Sign in again.",
    "That code didn't work",
    "The current password is wrong",
    "Use at least 10 characters",
    "That password is too long",
    "Choose a password other than the temporary one",
    "The new password is the same as the current one",
    "Start setting up two-factor sign-in first",
    "Two-factor sign-in is off",
    "This needs an administrator",
    "Account not found",
    "The credential key is not loaded, so two-factor sign-in is unavailable",
];

fn public_message(operation: &'static str, error: &anyhow::Error) -> String {
    let detail = error.to_string();
    if crate::services::login_rate_limit::is_rate_limit_message(&detail) {
        return crate::services::login_rate_limit::RATE_LIMITED_MESSAGE.to_string();
    }
    if let Some(message) = PUBLIC_MESSAGES.iter().find(|message| **message == detail) {
        return (*message).to_string();
    }
    if detail.contains("password reset token") {
        return "This reset link doesn't work: it was used, replaced or has expired. Ask for a new one."
            .to_string();
    }
    internal_message(operation, error)
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "AccountActionResult")]
pub struct AccountActionResult {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

impl AccountActionResult {
    fn from(operation: &'static str, result: anyhow::Result<()>) -> Self {
        match result {
            Ok(()) => Self {
                success: true,
                error: None,
            },
            Err(error) => Self {
                success: false,
                error: Some(public_message(operation, &error)),
            },
        }
    }
}

/// A new secret to add to an authenticator app: scan `qrSvg` or type `secret`.
#[derive(Clone, SimpleObject)]
#[graphql(name = "TwoFactorSetupResult")]
pub struct TwoFactorSetupResult {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "error")]
    pub error: Option<String>,
    /// Base32, for typing in.
    #[graphql(name = "secret")]
    pub secret: Option<String>,
    /// The `otpauth://` address the QR code holds.
    #[graphql(name = "uri")]
    pub uri: Option<String>,
    /// The QR code as an SVG document.
    #[graphql(name = "qrSvg")]
    pub qr_svg: Option<String>,
}

/// Recovery codes, shown this once.
#[derive(Clone, SimpleObject)]
#[graphql(name = "RecoveryCodesResult")]
pub struct RecoveryCodesResult {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "error")]
    pub error: Option<String>,
    #[graphql(name = "recoveryCodes")]
    pub recovery_codes: Vec<String>,
}

impl RecoveryCodesResult {
    fn from(operation: &'static str, result: anyhow::Result<Vec<String>>) -> Self {
        match result {
            Ok(codes) => Self {
                success: true,
                error: None,
                recovery_codes: codes,
            },
            Err(error) => Self {
                success: false,
                error: Some(public_message(operation, &error)),
                recovery_codes: Vec::new(),
            },
        }
    }
}

/// A reset link for an administrator to pass on.
#[derive(Clone, SimpleObject)]
#[graphql(name = "PasswordResetLinkResult")]
pub struct PasswordResetLinkResult {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "error")]
    pub error: Option<String>,
    /// The page to open, relative to the web app: `/reset-password?token=…`.
    #[graphql(name = "path")]
    pub path: Option<String>,
    #[graphql(name = "expiresAt")]
    pub expires_at: Option<String>,
}

#[derive(InputObject)]
#[graphql(name = "CompleteTwoFactorSignInInput")]
pub struct CompleteTwoFactorSignInInput {
    #[graphql(name = "token")]
    pub token: String,
    /// Six digits from the app, or a recovery code.
    #[graphql(name = "code")]
    pub code: String,
}

#[derive(InputObject)]
#[graphql(name = "CompletePasswordChangeInput")]
pub struct CompletePasswordChangeInput {
    #[graphql(name = "token")]
    pub token: String,
    #[graphql(name = "newPassword")]
    pub new_password: String,
}

#[derive(InputObject)]
#[graphql(name = "ResetPasswordWithLinkInput")]
pub struct ResetPasswordWithLinkInput {
    #[graphql(name = "token")]
    pub token: String,
    #[graphql(name = "newPassword")]
    pub new_password: String,
}

#[derive(InputObject)]
#[graphql(name = "ChangePasswordInput")]
pub struct ChangePasswordInput {
    #[graphql(name = "currentPassword")]
    pub current_password: String,
    #[graphql(name = "newPassword")]
    pub new_password: String,
}

#[derive(InputObject)]
#[graphql(name = "PasswordConfirmationInput")]
pub struct PasswordConfirmationInput {
    #[graphql(name = "password")]
    pub password: String,
}

#[derive(InputObject)]
#[graphql(name = "TwoFactorCodeInput")]
pub struct TwoFactorCodeInput {
    #[graphql(name = "code")]
    pub code: String,
}

#[derive(InputObject)]
#[graphql(name = "SetTemporaryPasswordInput")]
pub struct SetTemporaryPasswordInput {
    #[graphql(name = "userId")]
    pub user_id: String,
    #[graphql(name = "password")]
    pub password: String,
}

#[derive(Default)]
pub struct AccountMutations;

#[Object]
impl AccountMutations {
    /// The second sign-in step: a code from the authenticator app or a recovery code, for the
    /// challenge `login` returned. Signs in, or returns the next challenge (a new password).
    #[graphql(name = "completeTwoFactorSignIn")]
    async fn complete_two_factor_sign_in(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: CompleteTwoFactorSignInInput,
    ) -> Result<AuthPayload> {
        let auth = auth(ctx)?;
        Ok(
            match auth.complete_two_factor(&input.token, &input.code).await {
                Ok(SignIn::Session(result)) => AuthPayload::signed_in(ctx, auth, result),
                Ok(SignIn::Challenge(challenge)) => AuthPayload::challenged(challenge),
                Err(error) => {
                    tracing::debug!(reason = %error, "Two-factor sign-in step rejected");
                    AuthPayload::failed(public_message("auth.twoFactor", &error))
                }
            },
        )
    }

    /// After a temporary password: choose a new one and sign in.
    #[graphql(name = "completePasswordChange")]
    async fn complete_password_change(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: CompletePasswordChangeInput,
    ) -> Result<AuthPayload> {
        let auth = auth(ctx)?;
        Ok(
            match auth
                .complete_password_change(&input.token, &input.new_password)
                .await
            {
                Ok(result) => AuthPayload::signed_in(ctx, auth, result),
                Err(error) => AuthPayload::failed(public_message("auth.passwordChange", &error)),
            },
        )
    }

    /// Set a new password with a reset link an administrator passed on. Signs nobody in.
    #[graphql(name = "resetPasswordWithLink")]
    async fn reset_password_with_link(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: ResetPasswordWithLinkInput,
    ) -> Result<AccountActionResult> {
        let result = auth(ctx)?
            .reset_password_with_link(&input.token, &input.new_password)
            .await;
        Ok(AccountActionResult::from("auth.resetLink", result))
    }

    /// Change your password. Every session of yours ends; this device gets a new one.
    #[graphql(name = "changePassword")]
    async fn change_password(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: ChangePasswordInput,
    ) -> Result<AuthPayload> {
        let user = ctx.require_member()?;
        let auth = auth(ctx)?;
        Ok(
            match auth
                .change_password(&user.user_id, &input.current_password, &input.new_password)
                .await
            {
                Ok(result) => AuthPayload::signed_in(ctx, auth, result),
                Err(error) => AuthPayload::failed(public_message("auth.changePassword", &error)),
            },
        )
    }

    /// Start turning on two-factor sign-in, with your password.
    #[graphql(name = "beginTwoFactorSetup")]
    async fn begin_two_factor_setup(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: PasswordConfirmationInput,
    ) -> Result<TwoFactorSetupResult> {
        let user = ctx.require_member()?;
        Ok(
            match auth(ctx)?
                .begin_two_factor(&user.user_id, &input.password)
                .await
            {
                Ok(enrollment) => TwoFactorSetupResult {
                    success: true,
                    error: None,
                    secret: Some(enrollment.secret),
                    uri: Some(enrollment.uri),
                    qr_svg: Some(enrollment.qr_svg),
                },
                Err(error) => TwoFactorSetupResult {
                    success: false,
                    error: Some(public_message("auth.twoFactorSetup", &error)),
                    secret: None,
                    uri: None,
                    qr_svg: None,
                },
            },
        )
    }

    /// Finish turning it on with a code from the app. Returns the recovery codes.
    #[graphql(name = "confirmTwoFactorSetup")]
    async fn confirm_two_factor_setup(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: TwoFactorCodeInput,
    ) -> Result<RecoveryCodesResult> {
        let user = ctx.require_member()?;
        let result = auth(ctx)?
            .confirm_two_factor(&user.user_id, &input.code)
            .await;
        Ok(RecoveryCodesResult::from("auth.twoFactorConfirm", result))
    }

    /// New recovery codes; the old ones stop working.
    #[graphql(name = "regenerateRecoveryCodes")]
    async fn regenerate_recovery_codes(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: PasswordConfirmationInput,
    ) -> Result<RecoveryCodesResult> {
        let user = ctx.require_member()?;
        let result = auth(ctx)?
            .regenerate_recovery_codes(&user.user_id, &input.password)
            .await;
        Ok(RecoveryCodesResult::from("auth.recoveryCodes", result))
    }

    /// Turn your two-factor sign-in off, with your password.
    #[graphql(name = "disableTwoFactor")]
    async fn disable_two_factor(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: PasswordConfirmationInput,
    ) -> Result<AccountActionResult> {
        let user = ctx.require_member()?;
        let result = auth(ctx)?
            .disable_two_factor(&user.user_id, &input.password)
            .await;
        Ok(AccountActionResult::from("auth.twoFactorDisable", result))
    }

    /// Account recovery (administrators): turn someone's two-factor sign-in off. Logged.
    #[graphql(name = "resetUserTwoFactor")]
    async fn reset_user_two_factor(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "userId")] user_id: String,
    ) -> Result<AccountActionResult> {
        let admin = ctx.require_admin()?;
        let result = auth(ctx)?
            .admin_reset_two_factor(&admin.user_id, &user_id)
            .await;
        Ok(AccountActionResult::from("auth.resetUserTwoFactor", result))
    }

    /// Administrators: give someone a temporary password, which they must change when they next
    /// sign in. Their sessions end.
    #[graphql(name = "setTemporaryPassword")]
    async fn set_temporary_password(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: SetTemporaryPasswordInput,
    ) -> Result<AccountActionResult> {
        let admin = ctx.require_admin()?;
        let result = auth(ctx)?
            .admin_set_temporary_password(&admin.user_id, &input.user_id, &input.password)
            .await;
        Ok(AccountActionResult::from("auth.temporaryPassword", result))
    }

    /// Administrators: a one-time reset link for someone, valid for a day, to pass on yourself.
    /// It replaces any earlier link.
    #[graphql(name = "createPasswordResetLink")]
    async fn create_password_reset_link(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "userId")] user_id: String,
    ) -> Result<PasswordResetLinkResult> {
        let admin = ctx.require_admin()?;
        Ok(
            match auth(ctx)?
                .admin_create_reset_link(&admin.user_id, &user_id)
                .await
            {
                Ok((token, expires_at)) => PasswordResetLinkResult {
                    success: true,
                    error: None,
                    path: Some(format!(
                        "/reset-password?token={}",
                        urlencoding::encode(&token)
                    )),
                    expires_at: Some(
                        expires_at
                            .format(&time::format_description::well_known::Rfc3339)
                            .unwrap_or_default(),
                    ),
                },
                Err(error) => PasswordResetLinkResult {
                    success: false,
                    error: Some(public_message("auth.resetLink", &error)),
                    path: None,
                    expires_at: None,
                },
            },
        )
    }
}
