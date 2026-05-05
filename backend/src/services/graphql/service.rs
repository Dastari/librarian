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
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use axum::response::IntoResponse;
use axum::routing::get;
use tokio::sync::RwLock;
use tracing::info;

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

fn request_uses_secure_transport(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}

fn extract_token(headers: &HeaderMap) -> Option<String> {
    let header_token = headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string());

    header_token.or_else(|| extract_cookie(headers, ACCESS_TOKEN_COOKIE))
}

async fn graphiql(headers: HeaderMap) -> impl IntoResponse {
    let accepts_html = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"))
        .unwrap_or(false);

    if accepts_html {
        axum::response::Html(
            GraphiQLSource::build()
                .endpoint("/graphql")
                .subscription_endpoint("/graphql/ws")
                .finish(),
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
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut request = req.into_inner();
    let refresh_cookie = extract_cookie(&headers, REFRESH_TOKEN_COOKIE);
    let secure_transport = request_uses_secure_transport(&headers);
    request = request.data(AuthCookieContext {
        refresh_token: refresh_cookie,
        secure: secure_transport,
    });

    if let Some(token) = extract_token(&headers) {
        if let Some(auth) = state.services.get_auth().await {
            match auth.authenticate_access_token(&token).await {
                Ok(user) => {
                    let user = AuthUser::from(user);
                    tracing::debug!("Auth successful for user: {}", user.user_id);
                    request = request.data(user);
                }
                Err(e) => {
                    tracing::debug!("Token verification failed: {}", e);
                }
            }
        }
    } else {
        tracing::debug!("No auth token in request headers");
    }
    state.schema.execute(request).await.into()
}

async fn graphql_ws_handler(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    protocol: async_graphql_axum::GraphQLProtocol,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
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
                        if let Some(auth) = auth {
                            if let Ok(user) = auth.authenticate_access_token(token).await {
                                let mut data = async_graphql::Data::default();
                                data.insert(AuthUser::from(user));
                                return Ok(data);
                            }
                        }
                    }
                    Ok(async_graphql::Data::default())
                }
            })
            .serve()
        })
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
