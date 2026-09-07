//! Shared integration-test harness.
//!
//! Boots the *real* service stack (`ServicesManager` + the real Axum router
//! from [`librarian::build_app`]) against a temp SQLite database and temp
//! directories, on an ephemeral loopback port. Nothing here re-implements
//! product behaviour: tests drive the app through GraphQL over HTTP exactly
//! like a browser does, cookies included.
//!
//! Every `TestApp` is fully isolated (own database, own downloads/library
//! directories, own torrent session), so test binaries can run in parallel.

#![allow(dead_code)]

pub mod mock_indexer;

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use librarian::app::{AppState, build_app};
use librarian::app_mode::RunMode;
use librarian::config::Config;
use librarian::db::Database;
use librarian::jobs::AutoDownloadServiceConfig;
use librarian::services::sources::encryption::CredentialEncryption;
use librarian::services::sources::service::SourcesServiceConfig;
use librarian::services::torrent::TorrentServiceConfig;
use librarian::services::{
    AuthConfig, BackupServiceConfig, DatabaseServiceConfig, GraphqlServiceConfig,
    ObjectStorageServiceConfig, ServicesManager, library_scan::LibraryScanServiceConfig,
};
use serde_json::{Value, json};
use tokio::sync::oneshot;

/// Fixed 48-byte signing secret. Long enough for `AuthConfig`'s 32-byte
/// floor; the value is meaningless outside tests.
pub const TEST_JWT_SECRET: &str = "librarian-integration-test-jwt-signing-secret-0001";

/// Install a test tracing subscriber once per test binary. Quiet unless
/// `LIBRARIAN_TEST_LOG` is set (`LIBRARIAN_TEST_LOG=librarian=debug`).
pub fn init_tracing() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Ok(filter) = std::env::var("LIBRARIAN_TEST_LOG") else {
            return;
        };
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    });
}

/// Reserve a loopback port by binding and immediately releasing it. Racy in
/// principle, unique enough in practice, and the only way to learn the port
/// before handing it to `Config` (which the CORS/origin checks read).
pub fn free_port() -> u16 {
    reserve_port("127.0.0.1:0")
}

/// Same, but reserved on all interfaces — the torrent session listens on
/// `0.0.0.0`, and a port free only on loopback is not good enough.
pub fn free_port_any() -> u16 {
    reserve_port("0.0.0.0:0")
}

fn reserve_port(bind: &str) -> u16 {
    let listener = std::net::TcpListener::bind(bind).expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    port
}

/// Knobs a test can change before the stack boots.
pub struct TestAppOptions {
    /// Extra allowed origins on top of the app's own `http://127.0.0.1:<port>`.
    pub extra_cors_origins: Vec<String>,
    pub secure_cookies: bool,
    /// Register the torrent service (librqbit session). Off by default: the
    /// session costs ~1s to start and most tests never touch it. Implied by
    /// `with_auto_download`, which depends on it.
    pub with_torrent: bool,
    /// Register the auto-download background service. Its timer loop is
    /// disabled by default (`auto_download.enabled`), so this only makes
    /// `manager.get_auto_download()` available for `searchMissing`.
    pub with_auto_download: bool,
    pub torrent_max_concurrent: usize,
    pub torrent_seed_ratio_limit: f64,
    pub torrent_seed_time_minutes: i64,
    pub torrent_remove_after_import: bool,
}

impl Default for TestAppOptions {
    fn default() -> Self {
        Self {
            extra_cors_origins: Vec::new(),
            secure_cookies: false,
            with_torrent: false,
            with_auto_download: false,
            torrent_max_concurrent: 5,
            torrent_seed_ratio_limit: 0.0,
            torrent_seed_time_minutes: 0,
            torrent_remove_after_import: false,
        }
    }
}

/// A running Librarian backend, isolated in a temp directory.
pub struct TestApp {
    pub addr: SocketAddr,
    /// The loopback port the app's librqbit session listens on, so a test can
    /// point a peer straight at it.
    pub torrent_listen_port: u16,
    pub base_url: String,
    pub origin: String,
    pub services: Arc<ServicesManager>,
    pub db: Database,
    pub config: Arc<Config>,
    pub root: PathBuf,
    _temp: tempfile::TempDir,
    shutdown: Option<oneshot::Sender<()>>,
    server: Option<tokio::task::JoinHandle<()>>,
}

impl TestApp {
    pub async fn start() -> Self {
        Self::start_with(TestAppOptions::default()).await
    }

    pub async fn start_with(options: TestAppOptions) -> Self {
        init_tracing();
        let temp = tempfile::tempdir().expect("test temp dir");
        let root = temp.path().to_path_buf();
        for sub in [
            "data",
            "media",
            "downloads",
            "session",
            "storage",
            "backups",
            "cache",
        ] {
            std::fs::create_dir_all(root.join(sub)).expect("create test dir");
        }

        let port = free_port();
        let origin = format!("http://127.0.0.1:{port}");
        let mut cors_origins = vec![origin.clone()];
        cors_origins.extend(options.extra_cors_origins.iter().cloned());

        let config = Arc::new(Config {
            host: Some("127.0.0.1".to_string()),
            port,
            database_url: format!("sqlite://{}", root.join("data/librarian.db").display()),
            jwt_secret: String::new(),
            tvdb_api_key: None,
            tmdb_api_key: None,
            media_path: path_string(&root.join("media")),
            downloads_path: path_string(&root.join("downloads")),
            cache_path: path_string(&root.join("cache")),
            ffmpeg_path: None,
            session_path: path_string(&root.join("session")),
            storage_backend: "local".to_string(),
            storage_path: path_string(&root.join("storage")),
            backup_path: path_string(&root.join("backups")),
            torrent_enable_dht: false,
            torrent_listen_port: 0,
            torrent_max_concurrent: options.torrent_max_concurrent,
            cast_auto_discovery: false,
            cast_discovery_interval_secs: 3600,
            cast_discovery_timeout_ms: 100,
            cast_discovery_retention_days: 30,
            advertised_media_url: None,
            run_mode: RunMode::Server,
            tray_autostart: false,
            cors_origins,
            secure_cookies: options.secure_cookies,
            trusted_proxies: Vec::new(),
        });

        let mut builder = ServicesManager::builder()
            .add_service(DatabaseServiceConfig {
                database_url: config.database_url.clone(),
                connect_timeout: Duration::from_secs(10),
            })
            .add_service(ObjectStorageServiceConfig {
                backend: "local".to_string(),
                local_path: root.join("storage"),
            })
            .add_service(AuthConfig {
                // All fields are public: no env mutation needed, and the
                // lifetimes stay production-shaped so the cookie Max-Age
                // assertions are meaningful.
                jwt_secret: TEST_JWT_SECRET.to_string(),
                access_token_ttl_seconds: 15 * 60,
                refresh_token_ttl_seconds: 30 * 24 * 60 * 60,
            })
            .add_service(GraphqlServiceConfig { server_port: port })
            .add_service(BackupServiceConfig {
                repository_path: root.join("backups"),
            })
            .add_service(SourcesServiceConfig::with_key(
                CredentialEncryption::generate_key(),
            ))
            .add_service(LibraryScanServiceConfig::default());

        // The auto-download service declares a hard dependency on the torrent
        // service, so asking for one implies the other.
        let wants_torrent = options.with_torrent || options.with_auto_download;
        let torrent_port = if wants_torrent {
            // A real port per test: librqbit falls back to its own fixed
            // default when no listener is configured, which collides the
            // moment two tests run at once.
            let port = free_port_any();
            override_torrent_settings(
                &config.database_url,
                port,
                &root.join("downloads"),
                &root.join("session"),
                &options,
            )
            .await;
            port
        } else {
            0
        };

        if wants_torrent {
            builder = builder.add_service(TorrentServiceConfig {
                download_dir: root.join("downloads"),
                session_dir: root.join("session"),
                enable_dht: false,
                listen_port: torrent_port,
                max_concurrent: options.torrent_max_concurrent,
                upload_limit: 0,
                download_limit: 0,
                seed_ratio_limit: options.torrent_seed_ratio_limit,
                seed_time_minutes: options.torrent_seed_time_minutes,
                remove_after_import: options.torrent_remove_after_import,
            });
        }
        if options.with_auto_download {
            builder = builder.add_service(AutoDownloadServiceConfig::default());
        }

        let services = builder
            .add_api_routes("artwork", |_| librarian::api::artwork::router())
            .add_api_routes("health", |_| librarian::api::health::router())
            .add_api_routes("media", |_| librarian::api::media::router())
            .start()
            .await
            .expect("test services should start");

        let db = services
            .get_database()
            .await
            .expect("database service")
            .pool()
            .clone();
        let schema = services
            .get_graphql()
            .await
            .expect("graphql service")
            .schema()
            .await
            .expect("graphql schema");

        let state = AppState {
            config: config.clone(),
            db: db.clone(),
            schema,
            services: services.clone(),
        };
        let app = build_app(state).await;
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .expect("bind test http listener");
        let addr = listener.local_addr().expect("listener addr");

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
        });

        Self {
            addr,
            torrent_listen_port: torrent_port,
            base_url: format!("http://{addr}"),
            origin,
            services,
            db,
            config,
            root,
            _temp: temp,
            shutdown: Some(shutdown_tx),
            server: Some(server),
        }
    }

    /// A fresh unauthenticated GraphQL client pointed at this app.
    pub fn client(&self) -> GqlClient {
        GqlClient::new(&self.base_url, &self.origin)
    }

    /// A GraphQL client authenticated as a freshly registered admin. The first
    /// registration on an empty database self-bootstraps as admin, which is
    /// exactly the product's setup flow.
    pub async fn admin_client(&self) -> GqlClient {
        let mut client = self.client();
        client.register_admin("admin", "admin@example.test").await;
        client
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub async fn shutdown(mut self) {
        self.stop().await;
    }

    async fn stop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.server.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }
        let _ = self.services.stop_all().await;
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

/// Point the torrent settings at this test's own port and directories before
/// the real stack boots.
///
/// `TorrentService::start` lets `app_settings` rows override the
/// `TorrentServiceConfig` it was constructed with, and `bootstrap_defaults`
/// seeds all four of these keys with production values. So a test that only
/// sets `TorrentServiceConfig` still gets `torrent.listen_port = 6881`,
/// `torrent.enable_dht = true`, and — the dangerous one —
/// `torrent.download_dir = "/data/downloads"` /
/// `torrent.session_dir = "/data/session"`, i.e. it downloads into, and shares
/// a librqbit session with, whatever real deployment lives on this machine.
///
/// This is worth knowing outside tests too: an `app_settings` row silently
/// wins over the process configuration for every one of these keys.
///
/// Seeding skips keys that already exist, but it runs inside the database
/// service's own `start`, so the only way to get in first is to boot a
/// throwaway database-only manager, rewrite the rows, and shut it down again.
async fn override_torrent_settings(
    database_url: &str,
    listen_port: u16,
    download_dir: &Path,
    session_dir: &Path,
    options: &TestAppOptions,
) {
    let manager = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: database_url.to_string(),
            connect_timeout: Duration::from_secs(10),
        })
        .start()
        .await
        .expect("bootstrap database service should start");

    {
        let db_service = manager.get_database().await.expect("database service");
        let pool = db_service.pool().pool().clone();
        // Values are stored as JSON text, so paths must be quoted.
        let json_path = |path: &Path| serde_json::Value::from(path.to_string_lossy()).to_string();
        for (key, value) in [
            ("torrent.listen_port", listen_port.to_string()),
            ("torrent.enable_dht", "false".to_string()),
            ("torrent.download_dir", json_path(download_dir)),
            ("torrent.session_dir", json_path(session_dir)),
            // The seeding knobs are overridden the same way, so that
            // `TestAppOptions` is what actually decides them rather than
            // being silently replaced by the seeded production values
            // (`seed_ratio_limit = 1.0`, `remove_after_import = false`).
            (
                "torrent.seed_ratio_limit",
                options.torrent_seed_ratio_limit.to_string(),
            ),
            (
                "torrent.seed_time_minutes",
                options.torrent_seed_time_minutes.to_string(),
            ),
            (
                "torrent.remove_after_import",
                options.torrent_remove_after_import.to_string(),
            ),
            (
                "torrent.max_concurrent",
                options.torrent_max_concurrent.to_string(),
            ),
        ] {
            sqlx::query("UPDATE app_settings SET value = ? WHERE key = ?")
                .bind(value)
                .bind(key)
                .execute(&pool)
                .await
                .unwrap_or_else(|error| panic!("override app_setting {key}: {error}"));
        }
    }

    let _ = manager.stop_all().await;
}

// ---------------------------------------------------------------------------
// GraphQL client
// ---------------------------------------------------------------------------

/// One `Set-Cookie` header, parsed into the attributes the auth contract cares
/// about.
#[derive(Debug, Clone)]
pub struct ParsedCookie {
    pub name: String,
    pub value: String,
    pub path: Option<String>,
    pub max_age: Option<i64>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

impl ParsedCookie {
    pub fn parse(header: &str) -> Option<Self> {
        let mut parts = header.split(';').map(str::trim);
        let (name, value) = parts.next()?.split_once('=')?;
        let mut cookie = Self {
            name: name.to_string(),
            value: value.to_string(),
            path: None,
            max_age: None,
            http_only: false,
            secure: false,
            same_site: None,
        };
        for attribute in parts {
            let (key, val) = match attribute.split_once('=') {
                Some((key, val)) => (key.trim(), Some(val.trim())),
                None => (attribute, None),
            };
            match key.to_ascii_lowercase().as_str() {
                "path" => cookie.path = val.map(ToString::to_string),
                "max-age" => cookie.max_age = val.and_then(|v| v.parse().ok()),
                "httponly" => cookie.http_only = true,
                "secure" => cookie.secure = true,
                "samesite" => cookie.same_site = val.map(ToString::to_string),
                _ => {}
            }
        }
        Some(cookie)
    }
}

/// The result of one GraphQL POST: status, response body and the cookies the
/// server set on it.
#[derive(Debug, Clone)]
pub struct GqlResponse {
    pub status: u16,
    pub body: Value,
    pub set_cookies: Vec<ParsedCookie>,
}

impl GqlResponse {
    pub fn errors(&self) -> Vec<String> {
        self.body["errors"]
            .as_array()
            .map(|errors| {
                errors
                    .iter()
                    .map(|error| error["message"].as_str().unwrap_or("?").to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `data`, asserting there were no GraphQL errors.
    pub fn data(&self) -> &Value {
        assert!(
            self.body.get("errors").is_none(),
            "unexpected GraphQL errors: {}",
            self.body
        );
        &self.body["data"]
    }

    pub fn cookie(&self, name: &str) -> Option<&ParsedCookie> {
        self.set_cookies.iter().find(|cookie| cookie.name == name)
    }
}

/// A cookie-carrying GraphQL client. Cookies are stored and echoed back
/// verbatim, so tests observe exactly what a browser would.
pub struct GqlClient {
    http: reqwest::Client,
    endpoint: String,
    pub origin: Option<String>,
    pub cookies: BTreeMap<String, String>,
    pub bearer: Option<String>,
    pub user_id: Option<String>,
}

pub const ACCESS_COOKIE: &str = "librarian_access_token";
pub const REFRESH_COOKIE: &str = "librarian_refresh_token";

impl GqlClient {
    pub fn new(base_url: &str, origin: &str) -> Self {
        Self {
            http: reqwest::Client::builder()
                // Deliberately longer than the server's own longest bound
                // (`ADD_TORRENT_TIMEOUT`, 120s, which `searchMissing` waits on
                // while librqbit resolves metadata). If the client gave up
                // first, a stalled resolve would surface as an opaque
                // transport timeout instead of the server's own error.
                .timeout(Duration::from_secs(180))
                .build()
                .expect("test http client"),
            endpoint: format!("{base_url}/graphql"),
            origin: Some(origin.to_string()),
            cookies: BTreeMap::new(),
            bearer: None,
            user_id: None,
        }
    }

    pub fn without_origin(mut self) -> Self {
        self.origin = None;
        self
    }

    pub fn with_origin(mut self, origin: &str) -> Self {
        self.origin = Some(origin.to_string());
        self
    }

    pub fn forget_cookies(&mut self) {
        self.cookies.clear();
    }

    /// POST a GraphQL document, applying and capturing cookies.
    pub async fn post(&mut self, query: &str, variables: Value) -> GqlResponse {
        let mut request = self
            .http
            .post(&self.endpoint)
            .json(&json!({ "query": query, "variables": variables }));
        if let Some(origin) = &self.origin {
            request = request.header("Origin", origin);
        }
        if let Some(token) = &self.bearer {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        if !self.cookies.is_empty() {
            let cookie_header = self
                .cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("; ");
            request = request.header("Cookie", cookie_header);
        }

        let response = request.send().await.expect("graphql request should send");
        let status = response.status().as_u16();
        let set_cookies: Vec<ParsedCookie> = response
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter_map(ParsedCookie::parse)
            .collect();
        let text = response.text().await.expect("graphql response body");
        let body: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));

        for cookie in &set_cookies {
            if cookie.max_age == Some(0) {
                self.cookies.remove(&cookie.name);
            } else {
                self.cookies
                    .insert(cookie.name.clone(), cookie.value.clone());
            }
        }

        GqlResponse {
            status,
            body,
            set_cookies,
        }
    }

    /// POST and assert the request succeeded with no GraphQL errors.
    pub async fn query(&mut self, query: &str, variables: Value) -> Value {
        let response = self.post(query, variables).await;
        assert_eq!(
            response.status, 200,
            "expected HTTP 200, got {} for {query}",
            response.status
        );
        response.data().clone()
    }

    /// Register a user. The first user on an empty database becomes admin.
    pub async fn register_admin(&mut self, name: &str, email: &str) -> Value {
        let data = self
            .query(
                REGISTER_MUTATION,
                json!({
                    "input": {
                        "email": email,
                        "name": name,
                        "password": DEFAULT_PASSWORD,
                    }
                }),
            )
            .await;
        assert_eq!(
            data["register"]["success"], true,
            "admin registration failed: {data}"
        );
        self.user_id = data["register"]["user"]["id"].as_str().map(str::to_string);
        data
    }

    /// Re-run `query` until `ready` accepts the `data` payload, or fail.
    ///
    /// The generic [`wait_for`] cannot be used with a client: its closure would
    /// have to hand out a `&mut self` borrow that outlives the call.
    pub async fn wait_until(
        &mut self,
        timeout: Duration,
        label: &str,
        query: &str,
        variables: Value,
        mut ready: impl FnMut(&Value) -> bool,
    ) -> Value {
        let started = Instant::now();
        loop {
            let data = self.query(query, variables.clone()).await;
            if ready(&data) {
                return data;
            }
            assert!(
                started.elapsed() < timeout,
                "timed out after {timeout:?} waiting for {label}; last response: {data}"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn login(&mut self, principal: &str, password: &str) -> GqlResponse {
        self.post(
            LOGIN_MUTATION,
            json!({ "input": { "usernameOrEmail": principal, "password": password } }),
        )
        .await
    }
}

pub const DEFAULT_PASSWORD: &str = "Test-Password-123!";

pub const REGISTER_MUTATION: &str = r#"
mutation Register($input: RegisterUserInput!) {
  register(input: $input) {
    success
    error
    user { id username email role }
    tokens { expiresIn tokenType }
  }
}"#;

pub const LOGIN_MUTATION: &str = r#"
mutation Login($input: LoginInput!) {
  login(input: $input) {
    success
    error
    user { id username role }
    tokens { expiresIn tokenType }
  }
}"#;

pub const REFRESH_MUTATION: &str = r#"
mutation Refresh { refreshToken { success error user { id } tokens { expiresIn } } }"#;

pub const LOGOUT_MUTATION: &str = "mutation Logout { logout { success error } }";

// ---------------------------------------------------------------------------
// Small polling helper
// ---------------------------------------------------------------------------

/// Poll `check` until it returns `Some`, or fail after `timeout`.
pub async fn wait_for<T, F, Fut>(timeout: Duration, label: &str, mut check: F) -> T
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<T>>,
{
    let started = Instant::now();
    loop {
        if let Some(value) = check().await {
            return value;
        }
        assert!(
            started.elapsed() < timeout,
            "timed out after {:?} waiting for {label}",
            timeout
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// ---------------------------------------------------------------------------
// graphql-transport-ws client
// ---------------------------------------------------------------------------

/// Minimal `graphql-transport-ws` client for the generated `*Changed`
/// subscriptions. Authenticates through `connection_init` (the same path the
/// browser client uses) so no cookies — and therefore no origin guard — are
/// involved.
pub struct WsClient {
    stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

impl WsClient {
    pub async fn connect(app: &TestApp, access_token: &str) -> Self {
        use futures::SinkExt;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        use tokio_tungstenite::tungstenite::http::HeaderValue;

        let mut request = format!("ws://{}/graphql/ws", app.addr)
            .into_client_request()
            .expect("ws request");
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_static("graphql-transport-ws"),
        );
        let (mut stream, _) = tokio_tungstenite::connect_async(request)
            .await
            .expect("graphql-ws connect");

        stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json!({
                    "type": "connection_init",
                    "payload": { "Authorization": format!("Bearer {access_token}") }
                })
                .to_string()
                .into(),
            ))
            .await
            .expect("send connection_init");

        let mut client = Self { stream };
        let ack = client.next_message().await.expect("connection_ack");
        assert_eq!(ack["type"], "connection_ack", "unexpected ack: {ack}");
        client
    }

    pub async fn subscribe(&mut self, id: &str, query: &str) {
        use futures::SinkExt;
        self.stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json!({ "id": id, "type": "subscribe", "payload": { "query": query } })
                    .to_string()
                    .into(),
            ))
            .await
            .expect("send subscribe");
    }

    /// Next decoded protocol message, or `None` when the socket closed.
    pub async fn next_message(&mut self) -> Option<Value> {
        use futures::StreamExt;
        loop {
            let message = tokio::time::timeout(Duration::from_secs(10), self.stream.next())
                .await
                .expect("graphql-ws message should arrive within 10s")?;
            match message.expect("graphql-ws frame") {
                tokio_tungstenite::tungstenite::Message::Text(text) => {
                    return Some(serde_json::from_str(&text).expect("graphql-ws json"));
                }
                tokio_tungstenite::tungstenite::Message::Close(_) => return None,
                _ => continue,
            }
        }
    }

    /// Next `next` (data) message for `id`, skipping keepalives.
    pub async fn next_payload(&mut self, id: &str) -> Value {
        loop {
            let message = self
                .next_message()
                .await
                .expect("subscription closed before delivering a payload");
            match message["type"].as_str() {
                Some("next") if message["id"] == id => return message["payload"].clone(),
                Some("error") => panic!("subscription error: {message}"),
                _ => continue,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Test runtime
// ---------------------------------------------------------------------------

/// Run an async test body on a runtime shaped like the production one.
///
/// `#[tokio::test]` uses the platform default 2 MiB thread stacks, which the
/// generated GraphQL schema (recursion limit 512, ~46 entities with relation
/// types) can blow through while building or resolving. `main.rs` runs with
/// 8 MiB worker stacks for exactly that reason, so tests use the same size —
/// otherwise a test failure is a stack overflow rather than an assertion.
/// Every app test owns a full service stack (and, for acquisition tests, two librqbit
/// sessions). libtest runs the tests of a binary concurrently, and under a CPU quota
/// that contention turned progress-based waits into timeouts, so app tests run one at
/// a time. The cheap API binaries lose nothing; the acquisition binary gains stability.
static APP_TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn run_app_test<F, Fut, T>(body: F) -> T
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = T>,
    T: Send + 'static,
{
    let _serial = APP_TEST_SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let result = std::thread::Builder::new()
        .name("librarian-test".to_string())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                // Two is enough for a test body plus the server it drives.
                // libtest runs the tests in a binary concurrently, and each
                // one of these owns a whole service stack (plus, in the
                // acquisition tests, two librqbit sessions), so a larger pool
                // just starves the machine.
                .worker_threads(2)
                .max_blocking_threads(4)
                .thread_stack_size(8 * 1024 * 1024)
                .enable_all()
                .build()
                .expect("test runtime should build")
                .block_on(body())
        })
        .expect("test thread should spawn")
        .join();

    match result {
        Ok(value) => value,
        // Re-raise so the harness reports the original assertion, not a
        // generic "thread panicked" wrapper.
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
