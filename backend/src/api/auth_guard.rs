//! Shared access-token validation for REST (`/api/*`) endpoints.
//!
//! GraphQL validates tokens via `AuthService::authenticate_access_token` (see
//! `services/graphql/service.rs`); these REST handlers reuse the same service so REST and
//! GraphQL enforce identical auth semantics. Browser clients authenticate with the
//! server-managed HttpOnly access cookie. The query-token fallback remains temporarily
//! for legacy non-browser media clients and must not be used by the frontend.

use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};

use crate::AppState;
use crate::graphql::AuthUser;

const ACCESS_TOKEN_COOKIE: &str = "librarian_access_token";

fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|segment| {
            let (key, value) = segment.trim().split_once('=')?;
            (key.trim() == name && !value.trim().is_empty()).then(|| value.trim().to_string())
        })
}

/// Extract a bearer token from the `Authorization` header, then the HttpOnly access
/// cookie, with a final compatibility fallback to `query_token`.
pub fn extract_bearer_token(headers: &HeaderMap, query_token: Option<&str>) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .or_else(|| extract_cookie(headers, ACCESS_TOKEN_COOKIE))
        .or_else(|| {
            query_token
                .map(str::trim)
                .filter(|token| !token.is_empty())
                .map(str::to_string)
        })
}

/// Require a valid access token from a supported transport.
/// Returns `401 Unauthorized` when no token is present or it fails verification.
pub async fn require_authenticated_user(
    state: &AppState,
    headers: &HeaderMap,
    query_token: Option<&str>,
) -> Result<AuthUser, StatusCode> {
    let token = extract_bearer_token(headers, query_token).ok_or(StatusCode::UNAUTHORIZED)?;
    let auth = state
        .services
        .get_auth()
        .await
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    auth.authenticate_access_token(&token)
        .await
        .map(AuthUser::from)
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

/// Require a valid access token belonging to an admin user. Header-only (no query-param
/// fallback) since admin-gated REST endpoints aren't loaded as media/img `src` URLs.
/// Returns `401` when unauthenticated, `403` when authenticated but not an admin.
pub async fn require_authenticated_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthUser, StatusCode> {
    let user = require_authenticated_user(state, headers, None).await?;
    if user.is_admin() {
        Ok(user)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}
