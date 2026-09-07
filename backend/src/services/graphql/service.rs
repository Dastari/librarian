//! GraphQL service: owns schema building and exposes HTTP routes for /graphql and /graphql/ws.
//!
//! Depends on the database and auth services; builds the schema in [start](Service::start) after
//! both are available. Main wires routes by merging [Self::router] into the app and uses
//! [Self::schema] to build [AppState].

use std::sync::Arc;

use anyhow::Result;
use async_graphql::http::GraphiQLSource;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use async_trait::async_trait;
use axum::Router;
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::http::header::{AUTHORIZATION, FORWARDED, HOST, ORIGIN};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use std::net::{IpAddr, SocketAddr};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::services::manager::{Service, ServiceHealth};

use super::{AuthUser, LibrarianSchema, build_schema};
use crate::services::graphql::mutations::auth::AuthCookieContext;

const ACCESS_TOKEN_COOKIE: &str = "librarian_access_token";
const REFRESH_TOKEN_COOKIE: &str = "librarian_refresh_token";

/// Configuration for the GraphQL service (server port for playground URL logging).
#[derive(Debug, Clone)]
pub struct GraphqlServiceConfig {
    pub server_port: u16,
}

/// GraphQL service: builds and holds the schema, provides routes for the playground and API.
pub struct GraphqlService {
    manager: Arc<crate::services::ServicesManager>,
    server_port: u16,
    schema: RwLock<Option<LibrarianSchema>>,
}

impl GraphqlService {
    /// Create the service. Register with [register_graphql](crate::services::ServicesManager::register_graphql)
    /// before [start_all](crate::services::ServicesManager::start_all); [start](Service::start) will
    /// obtain the database and auth service from the manager and build the schema.
    /// `server_port` is used to log the GraphQL playground URL on start.
    pub fn new(manager: Arc<crate::services::ServicesManager>, server_port: u16) -> Self {
        Self {
            manager,
            server_port,
            schema: RwLock::new(None),
        }
    }

    /// Return the built schema, if the service has been started. Main uses this to build [AppState].
    pub async fn schema(&self) -> Option<LibrarianSchema> {
        self.schema.read().await.clone()
    }

    /// Return a router with /graphql and /graphql/ws. Merge this into the app and call
    /// `.with_state(state)` on the combined router so handlers receive [AppState].
    pub fn router() -> Router<crate::AppState> {
        Router::new()
            .route("/graphql", get(graphiql).post(graphql_handler))
            .route("/graphql/ws", get(graphql_ws_handler))
    }
}

fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    cookie_header.split(';').find_map(|segment| {
        let mut parts = segment.trim().splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        if key == name && !value.is_empty() {
            Some(value.to_string())
        } else {
            None
        }
    })
}

fn has_auth_cookie(headers: &HeaderMap) -> bool {
    extract_cookie(headers, ACCESS_TOKEN_COOKIE).is_some()
        || extract_cookie(headers, REFRESH_TOKEN_COOKIE).is_some()
}

fn origin_is_cleartext_http(headers: &HeaderMap) -> bool {
    headers
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .is_some_and(|origin| origin.starts_with("http://"))
}

fn cookie_request_origin_allowed(
    headers: &HeaderMap,
    cors_origins: &[String],
    request_is_secure: bool,
) -> bool {
    if !has_auth_cookie(headers) {
        return true;
    }
    let Some(origin) = headers
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
    else {
        return false;
    };
    let normalized_origin = origin.trim_end_matches('/');
    if cors_origins
        .iter()
        .any(|allowed| allowed.trim_end_matches('/') == normalized_origin)
    {
        return true;
    }
    let Some(host) = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|host| !host.is_empty())
    else {
        return false;
    };
    // HTTPS origins may use the Host fallback only when cookie transport is
    // actually secure. HTTP origins (local Vite on :3000/:3002, LAN IPs) are
    // still same-origin even if the public deployment sets Secure cookies.
    if normalized_origin == format!("https://{host}") {
        return request_is_secure;
    }
    normalized_origin == format!("http://{host}")
}

fn forwarded_proto(headers: &HeaderMap) -> Option<&str> {
    if let Some(proto) = headers
        .get(FORWARDED)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .and_then(|element| {
            element.split(';').find_map(|parameter| {
                let (name, value) = parameter.trim().split_once('=')?;
                name.eq_ignore_ascii_case("proto")
                    .then(|| value.trim_matches('"'))
            })
        })
    {
        return Some(proto);
    }
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
}

fn request_uses_secure_cookies(
    headers: &HeaderMap,
    peer_ip: Option<IpAddr>,
    config: &crate::config::Config,
) -> bool {
    // A browser on http://localhost:3002 (or any other HTTP origin) will drop
    // `Secure` cookies, except for the localhost secure-context exception.
    // Never force Secure just because the public HTTPS UI enabled the flag.
    if origin_is_cleartext_http(headers) {
        return false;
    }
    secure_cookie_transport(
        headers,
        peer_ip,
        config.secure_cookies,
        &config.trusted_proxies,
    )
}

fn secure_cookie_transport(
    headers: &HeaderMap,
    peer_ip: Option<IpAddr>,
    secure_override: bool,
    trusted_proxies: &[ipnet::IpNet],
) -> bool {
    if secure_override {
        return true;
    }
    let peer_is_trusted =
        peer_ip.is_some_and(|ip| trusted_proxies.iter().any(|network| network.contains(&ip)));
    peer_is_trusted
        && forwarded_proto(headers).is_some_and(|proto| proto.eq_ignore_ascii_case("https"))
}

fn extract_token(headers: &HeaderMap) -> Option<String> {
    let header_token = headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    header_token.or_else(|| extract_cookie(headers, ACCESS_TOKEN_COOKIE))
}

fn sanitize_execution_errors(mut response: async_graphql::Response) -> async_graphql::Response {
    for error in &mut response.errors {
        let has_public_code = error
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.get("code"))
            .is_some();
        // Parser/schema-validation errors have no resolver path and are useful to
        // clients. Every resolver failure must opt into a stable public code;
        // uncoded failures are treated as internal at this final boundary.
        if has_public_code || error.path.is_empty() {
            continue;
        }
        let correlation_id = Uuid::new_v4().to_string();
        tracing::error!(
            correlation_id,
            path = ?error.path,
            internal_message = %error.message,
            "Sanitized uncoded GraphQL resolver error"
        );
        error.message = "The operation could not be completed".to_string();
        let extensions = error.extensions.get_or_insert_default();
        extensions.set("code", crate::services::graphql::error::INTERNAL_ERROR_CODE);
        extensions.set("correlationId", correlation_id);
    }
    response
}

async fn graphiql(headers: HeaderMap) -> impl IntoResponse {
    let accepts_html = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"))
        .unwrap_or(false);

    if accepts_html && crate::config::dev_mode() {
        axum::response::Html(
            GraphiQLSource::build()
                .endpoint("/graphql")
                .subscription_endpoint("/graphql/ws")
                .finish(),
        )
        .into_response()
    } else if accepts_html {
        // GraphiQL is dev-only (see crate::config::dev_mode); don't serve the playground
        // (or hint at its existence) outside development.
        (
            axum::http::StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "Not found"
            })),
        )
            .into_response()
    } else {
        (
            axum::http::StatusCode::METHOD_NOT_ALLOWED,
            axum::Json(serde_json::json!({
                "error": "GET requests are not supported for GraphQL queries. Use POST with Content-Type: application/json"
            })),
        )
            .into_response()
    }
}

async fn graphql_handler(
    State(state): State<crate::AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> Response {
    let request_is_secure = request_uses_secure_cookies(&headers, Some(peer.ip()), &state.config);
    if !cookie_request_origin_allowed(&headers, &state.config.cors_origins, request_is_secure) {
        tracing::warn!(
            origin = headers
                .get(ORIGIN)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("missing"),
            host = headers
                .get(HOST)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("missing"),
            "Rejected cookie-authenticated GraphQL request with an untrusted origin"
        );
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut request = req.into_inner();
    let refresh_cookie = extract_cookie(&headers, REFRESH_TOKEN_COOKIE);
    request = request.data(AuthCookieContext {
        refresh_token: refresh_cookie,
        secure: request_is_secure,
    });

    if let Some(token) = extract_token(&headers) {
        if let Some(auth) = state.services.get_auth().await {
            match auth.authenticate_access_token(&token).await {
                Ok(user) => {
                    let user = AuthUser::from(user);
                    let user_id = user.user_id.clone();
                    tracing::debug!("Auth successful for user: {}", user.user_id);
                    request = request.data(user).data(user_id);
                }
                Err(e) => {
                    tracing::debug!("Token verification failed: {}", e);
                }
            }
        }
    } else {
        tracing::debug!("No auth token in request headers");
    }
    GraphQLResponse::from(sanitize_execution_errors(
        state.schema.execute(request).await,
    ))
    .into_response()
}

async fn graphql_ws_handler(
    State(state): State<crate::AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    protocol: async_graphql_axum::GraphQLProtocol,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let request_is_secure = request_uses_secure_cookies(&headers, Some(peer.ip()), &state.config);
    if !cookie_request_origin_allowed(&headers, &state.config.cors_origins, request_is_secure) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let auth = state.services.get_auth().await;
    let auth_user = match (extract_token(&headers), auth.as_ref()) {
        (Some(token), Some(auth)) => auth
            .authenticate_access_token(&token)
            .await
            .ok()
            .map(AuthUser::from),
        _ => None,
    };
    let auth_for_init = auth.clone();

    ws.protocols(["graphql-transport-ws", "graphql-ws"])
        .on_upgrade(move |socket| {
            let mut ws =
                async_graphql_axum::GraphQLWebSocket::new(socket, state.schema.clone(), protocol);
            if let Some(user) = auth_user {
                let mut data = async_graphql::Data::default();
                data.insert(user.user_id.clone());
                data.insert(user);
                ws = ws.with_data(data);
            }
            ws.on_connection_init(move |params| {
                let auth = auth_for_init.clone();
                async move {
                    if let Some(token) = params
                        .get("Authorization")
                        .or_else(|| params.get("authorization"))
                        .and_then(|v| v.as_str())
                    {
                        let token = token.strip_prefix("Bearer ").unwrap_or(token);
                        if let Some(auth) = auth
                            && let Ok(user) = auth.authenticate_access_token(token).await
                        {
                            let mut data = async_graphql::Data::default();
                            let user = AuthUser::from(user);
                            data.insert(user.user_id.clone());
                            data.insert(user);
                            return Ok(data);
                        }
                    }
                    Ok(async_graphql::Data::default())
                }
            })
            .serve()
        })
        .into_response()
}

#[async_trait]
impl Service for GraphqlService {
    fn name(&self) -> &str {
        "graphql"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string(), "auth".to_string()]
    }

    async fn start(&self) -> Result<()> {
        info!(
            service = "graphql",
            server_port = self.server_port,
            "GraphQL service starting: building schema and routes on port {}",
            self.server_port
        );
        let db = self
            .manager
            .get_database()
            .await
            .map(|svc| svc.pool().clone())
            .ok_or_else(|| anyhow::anyhow!("database service not available"))?;
        let auth = self
            .manager
            .get_auth()
            .await
            .ok_or_else(|| anyhow::anyhow!("auth service not available"))?;
        let schema = build_schema(db, auth, self.manager.clone());
        *self.schema.write().await = Some(schema);
        info!(
            service = "graphql",
            server_port = self.server_port,
            "GraphQL service started: schema initialized and router ready on port {}",
            self.server_port
        );
        info!(
            service = "graphql",
            "GraphQL playground: http://localhost:{}/graphql", self.server_port
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.schema.write().await = None;
        info!(
            service = "graphql",
            "GraphQL service stopped: in-memory schema cleared"
        );
        Ok(())
    }

    fn provides_routes(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn cookie_headers(origin: Option<&str>, host: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            HeaderValue::from_static("librarian_access_token=secret"),
        );
        headers.insert(HOST, HeaderValue::from_str(host).unwrap());
        if let Some(origin) = origin {
            headers.insert(ORIGIN, HeaderValue::from_str(origin).unwrap());
        }
        headers
    }

    #[test]
    fn cookie_auth_requires_an_allowed_origin() {
        let allowed = vec!["http://localhost:3000".to_string()];
        assert!(!cookie_request_origin_allowed(
            &cookie_headers(None, "localhost:3001"),
            &allowed,
            false,
        ));
        assert!(cookie_request_origin_allowed(
            &cookie_headers(Some("http://localhost:3000"), "localhost:3001"),
            &allowed,
            false,
        ));
        assert!(!cookie_request_origin_allowed(
            &cookie_headers(Some("https://attacker.example"), "localhost:3001"),
            &allowed,
            false,
        ));
    }

    #[test]
    fn same_origin_scheme_comes_only_from_explicit_cookie_config() {
        let headers = cookie_headers(Some("https://library.example"), "library.example");
        assert!(cookie_request_origin_allowed(&headers, &[], true));
        assert!(!cookie_request_origin_allowed(&headers, &[], false));

        let mut spoofed = cookie_headers(Some("https://library.example"), "library.example");
        spoofed.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        assert!(!cookie_request_origin_allowed(&spoofed, &[], false));
    }

    #[test]
    fn http_same_origin_is_allowed_when_secure_cookies_are_enabled() {
        let headers = cookie_headers(Some("http://localhost:3002"), "localhost:3002");
        assert!(cookie_request_origin_allowed(&headers, &[], true));
        assert!(cookie_request_origin_allowed(&headers, &[], false));
        assert!(cookie_request_origin_allowed(
            &cookie_headers(Some("http://127.0.0.1:3002"), "127.0.0.1:3002"),
            &[],
            true,
        ));
    }

    #[test]
    fn cleartext_http_origin_does_not_use_secure_cookies() {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, HeaderValue::from_static("http://localhost:3002"));
        assert!(origin_is_cleartext_http(&headers));

        headers.insert(
            ORIGIN,
            HeaderValue::from_static("https://librarian.dastari.net"),
        );
        assert!(!origin_is_cleartext_http(&headers));
        assert!(!origin_is_cleartext_http(&HeaderMap::new()));
    }

    #[test]
    fn only_trusted_proxy_peers_can_forward_the_secure_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        let trusted = vec!["10.0.0.0/8".parse().unwrap()];

        assert!(secure_cookie_transport(
            &headers,
            Some("10.1.2.3".parse().unwrap()),
            false,
            &trusted,
        ));
        assert!(!secure_cookie_transport(
            &headers,
            Some("192.0.2.10".parse().unwrap()),
            false,
            &trusted,
        ));
        assert!(!secure_cookie_transport(&headers, None, false, &trusted));
    }

    #[test]
    fn requests_without_auth_cookies_do_not_need_csrf_origin_checks() {
        assert!(cookie_request_origin_allowed(&HeaderMap::new(), &[], true));
    }
}
