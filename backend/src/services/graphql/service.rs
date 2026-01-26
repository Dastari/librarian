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

use super::{AuthUser, LibrarianSchema, build_schema, verify_token};

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

fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string())
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
    if let Some(token) = extract_token(&headers) {
        let secret = match state.services.get_auth().await {
            Some(auth) => auth.get_jwt_secret().await.ok(),
            None => None,
        };
        if let Some(secret) = secret {
            match verify_token(&token, &secret) {
                Ok(user) => {
                    tracing::debug!("Auth successful for user: {}", user.user_id);
                    request = request.data(user);
                }
                Err(e) => {
                    tracing::debug!(
                        "Token verification failed: {} (token prefix: {}...)",
                        e.message,
                        &token[..token.len().min(20)]
                    );
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
    let secret = match state.services.get_auth().await {
        Some(auth) => auth.get_jwt_secret().await.ok(),
        None => None,
    };
    let auth_user = extract_token(&headers).and_then(|token| {
        secret
            .as_ref()
            .and_then(|s| verify_token(&token, s).ok())
    });
    let secret_for_init = secret.clone();

    ws.protocols(["graphql-transport-ws", "graphql-ws"])
        .on_upgrade(move |socket| {
            let mut ws =
                async_graphql_axum::GraphQLWebSocket::new(socket, state.schema.clone(), protocol);
            if let Some(user) = auth_user {
                let mut data = async_graphql::Data::default();
                data.insert(user);
                ws = ws.with_data(data);
            }
            let secret = secret_for_init;
            ws.on_connection_init(move |params| {
                let secret = secret.clone();
                async move {
                    if let Some(token) = params
                        .get("Authorization")
                        .or_else(|| params.get("authorization"))
                        .and_then(|v| v.as_str())
                    {
                        let token = token.strip_prefix("Bearer ").unwrap_or(token);
                        if let Some(ref s) = secret {
                            if let Ok(user) = verify_token(token, s) {
                                let mut data = async_graphql::Data::default();
                                data.insert(user);
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
        info!(service = "graphql", "GraphQL service starting");
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
        let schema = build_schema(db, auth);
        *self.schema.write().await = Some(schema);
        info!(service = "graphql", "GraphQL service started");
        info!(
            service = "graphql",
            "GraphQL playground: http://localhost:{}/graphql",
            self.server_port
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.schema.write().await = None;
        info!(service = "graphql", "Stopped");
        Ok(())
    }

    fn provides_routes(&self) -> bool {
        true
    }
}
