# Services

This document describes how to implement, register, and unregister backend **services** — long-running or background components managed by the global [ServicesManager](backend/src/services/manager.rs).

## When something is a service

Implement the [Service](backend/src/services/manager.rs) trait when the component:

- Runs **background tasks** (e.g. torrent progress monitor, cast mDNS discovery)
- Holds a **long-lived session or connection** (e.g. database pool, librqbit session)
- Runs on **timers or event loops** (e.g. job scheduler, completion handler)

Do **not** implement `Service` for stateless or on-demand helpers (e.g. metadata API client, ffmpeg wrapper, auth). Those live in `backend/src/services/` as utilities and are constructed where needed.

## Requirements

Every service must:

1. Implement **start**, **stop**, **restart** (or use the default), and **health** (or use the default).
2. Declare **dependencies** via [dependencies](backend/src/services/manager.rs) if it requires other services to be started first.
3. Use the **[tracing](https://docs.rs/tracing)** crate for **all** lifecycle and operational logging.

### Logging

- Log at appropriate levels: `info` for start/stop/restart, `debug` for periodic work, `warn`/`error` for failures.
- Include the service name or a consistent target so logs are filterable, e.g.:
  - `tracing::info!(service = %self.name(), "Started");`
  - `tracing::error!(service = "torrent", error = %e, "Progress loop failed");`
- The manager itself logs at `info`/`warn` when registering, starting, stopping, or restarting services.

## Dependency order

- Each service can return a list of **dependency** names: services that must be **started before** this one.
- The manager **starts** services in **dependency order** (dependencies first). It uses a topological sort; unknown dependencies or cycles are an error.
- The manager **stops** services in **reverse dependency order** (dependents first).
- **Example:** Register `database` first (no dependencies), then `logging` with `dependencies() = vec!["database".to_string()]`. When you call `start_all()`, the manager starts `database`, then `logging`.

## Health checks

- Every service can report [ServiceHealth](backend/src/services/manager.rs): **Healthy**, **Degraded**, or **Unhealthy**, with an optional message.
- Default [health](backend/src/services/manager.rs) returns `Healthy`. Override to run a real check (e.g. ping database, check worker task).
- The manager exposes [health_one](backend/src/services/manager.rs) and [health_all](backend/src/services/manager.rs) for monitoring or `/healthz`-style endpoints.

## Implementing the Service trait

```rust
use async_trait::async_trait;
use anyhow::Result;
use crate::services::manager::{Service, ServiceHealth, HealthStatus};

#[async_trait]
impl Service for MyService {
    fn name(&self) -> &str {
        "my_service"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string()]   // start after database
    }

    async fn start(&self) -> Result<()> {
        tracing::info!(service = "my_service", "Starting");
        // spawn background tasks, open connections, etc.
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!(service = "my_service", "Stopping");
        // signal tasks to exit, abort handles, close connections
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        // default: Ok(ServiceHealth::healthy())
        if something_wrong() {
            Ok(ServiceHealth::degraded("high latency"))
        } else {
            Ok(ServiceHealth::healthy())
        }
    }

    fn provides_routes(&self) -> bool {
        true   // if this service contributes HTTP routes (see below)
    }
}
```

## Database and logging as services

- **Database** is the single owner of the connection pool. Use [DatabaseService](backend/src/services/database.rs) and register it with [register_database](backend/src/services/manager.rs). No other code creates or owns a pool; everyone gets it via [get_database](backend/src/services/manager.rs) (or [get_db_pool](backend/src/main.rs) in the backend crate). When the database service is stopped or unavailable, `get_database()` returns `None`, so other services can detect that and avoid using the pool (and not crash).
- **Database service startup:** When the database service [start](backend/src/services/database.rs)s, it (1) runs **entity schema sync** ([sync_all_entity_schemas](backend/src/db/schema_sync.rs): creates missing tables, adds missing columns) and (2) runs **JWT secret initialization** (uses `JWT_SECRET` from environment or generates). Main does **not** need to run schema sync or JWT init; the database service does both before reporting started.
- **Logging** (e.g. the DB-backed logging layer) should depend on the database: `dependencies() = vec!["database".to_string()]`. It should only start after the database service is started.

## How other code gets the database

Only the **database service** creates and owns the pool. All other code must get the pool from the services manager:

1. **Register the database service with [register_database](backend/src/services/manager.rs)**  
   Do not use `register()` for the database service. Use `services.register_database(Arc::new(DatabaseService::new(pool))).await` so that [get_database](backend/src/services/manager.rs) works.

2. **Get the pool from the manager**  
   - **Started only:** `services.get_database().await` returns `Some(Arc<DatabaseService>)` only when the database service is **started**. When it is stopped or unregistered, it returns `None`, so callers can skip DB work or return 503.
   - **Pool clone:** Use `.map(|svc| svc.pool().clone())` or the backend helper [get_db_pool](backend/src/main.rs): `get_db_pool(services).await` → `Option<Database>`.

3. **In request handlers**  
   [AppState](backend/src/main.rs) has `state.db` (a pool clone set at startup). Handlers can use it for simplicity. If the database service is stopped later, that clone is the same underlying pool (now closed), so the next query will fail. For handlers that should react to “DB unavailable”, use `get_db_pool(&state.services).await` and return 503 if `None`.

4. **In other services (e.g. logging, torrent)**  
   Take `Arc<ServicesManager>` (or get it from app state). In `start()` or whenever you need the DB, call `services.get_database().await`. If `None`, the database is not started—skip DB-dependent work, log, and/or mark yourself degraded. When `Some(svc)`, use `svc.pool().clone()` for the current request or store the clone for background work (accept that after the database service stops, the pool is closed and the next use will error unless you check `get_database()` again).

## Configuration and builder (Bevy-style)

Services take **configuration** (e.g. [DatabaseServiceConfig](backend/src/services/database.rs) with `database_url`, `connect_timeout`). The recommended way to create and start the manager is the **builder**:

- **ServicesManager::builder()** returns a [ServicesManagerBuilder](backend/src/services/manager.rs).
- **.add_service(...)** adds a service. Accepts:
  - **Configs:** [DatabaseServiceConfig](backend/src/services/database.rs), [LoggingServiceConfig](backend/src/services/logging.rs). The manager instantiates the service when you call `start()` or `build()`.
  - **Pre-built:** `Arc<dyn Service>` (e.g. one you constructed with its own config).
- Add **database before logging** so the logging service (which depends on the database) can be started in the correct order.
- **.build()** creates the manager and registers all services; does not start them.
- **.start()** builds, registers, and **starts** all services in dependency order; returns `Arc<ServicesManager>`.

Example (in `main`):

```rust
use std::time::Duration;
use crate::services::{DatabaseServiceConfig, LoggingServiceConfig, ServicesManager};

let services = ServicesManager::builder()
    .add_service(DatabaseServiceConfig {
        database_url: config.database_url.clone(),
        connect_timeout: Duration::from_secs(30),
    })
    .add_service(LoggingServiceConfig::default())  // depends on database
    .add_service(Arc::new(MyService::new(my_config)))
    .start()
    .await?;

let db = get_db_pool(services.as_ref()).await.expect("database just started");
let db_layer = services.get_logging().await.unwrap().tracing_layer().unwrap();
tracing_subscriber::registry().with(env_filter).with((*db_layer).clone()).init();
```

The **logging service** depends on the database and exposes a tracing layer: after `start()`, call [get_logging](backend/src/services/manager.rs) and [LoggingService::tracing_layer](backend/src/services/logging.rs) to wire the layer into `tracing_subscriber`. Each service type can define its own config (e.g. `LoggingServiceConfig`); pass the config to `.add_service(config)` or add a pre-built `Arc<dyn Service>`.

## Registering a service manually

If you don’t use the builder, you can still register and start manually:

1. **Database service:** Use [register_database](backend/src/services/manager.rs) with a service created via [DatabaseService::from_config](backend/src/services/database.rs) or `DatabaseService::new(pool)`.
2. Other services: construct (with their config), wrap in `Arc`, then `services.register(arc_service).await`.
3. Register in **dependency order**: database first, then services that depend on it.
4. Call `services.start_all().await`.

Registration does **not** start the service. Starting is separate (`start_one` / `start_all`).

## Unregistering a service

1. **Stop** the service first (if it is running):
   - `services.stop_one("my_service").await`
2. **Unregister** it:
   - `services.unregister("my_service").await`

This returns `Option<Arc<dyn Service>>` (the previous instance). The manager no longer holds the service; if you don’t keep the `Arc` elsewhere, it will be dropped.

Unregistering does **not** call `stop`. You must stop before unregistering if the service is running.

## Manager API summary

| Method | Description |
|--------|-------------|
| `register(Arc<dyn Service>)` | Add a service (does not start it). Re-registering the same name overwrites. |
| `register_database(Arc<DatabaseService>)` | Register the database service so [get_database](backend/src/services/manager.rs) works. Use this instead of `register` for the database. |
| `get_database()` | Returns `Some(Arc<DatabaseService>)` only when the database service is **started**; `None` when stopped or unregistered. Use this to get the pool so callers can react to DB unavailability. |
| `get_database_unchecked()` | Returns the database service if registered, regardless of started state. Prefer `get_database()` for normal use. |
| `unregister(name)` | Remove a service by name. Does not stop it; stop first if needed. Clears the typed database handle when name is `"database"`. |
| `start_all()` | Start all in **dependency order** (dependencies first). Errors on unknown dep or cycle. |
| `stop_all()` | Stop all in **reverse dependency order** (dependents first). |
| `restart_all()` | Stop all, then start all (in dependency order). |
| `start_one(name)` | Start one service. **Requires its dependencies to already be started.** |
| `stop_one(name)` | Stop one service. Returns `bool` (found and stopped). |
| `restart_one(name)` | Restart one service. Errors if not found or restart fails. |
| `health_one(name)` | Run health check for one service. |
| `health_all()` | Run health check for all; returns `HashMap<String, ServiceHealth>`. |
| `is_started(name)` | Whether the service is currently started (tracked by manager). |
| `get(name)` | Get `Option<Arc<dyn Service>>` by name (e.g. for downcast or app state). |
| `names()` | List all registered service names. |
| `services_with_routes()` | List names of services that return `true` from `provides_routes()`. |

## Registering API routes with the HTTP server

Services (or `main`) can register **`/api/*`** route builders so the HTTP server includes their endpoints without the HTTP service knowing about each module. Use the builder’s **[add_api_routes](backend/src/services/manager.rs)**:

- **.add_api_routes(name, builder)** — `name` is for logging; `builder` is a closure `Fn(AppState) -> Router<AppState>` that returns a router to merge under `/api`.
- The [HttpServerService](backend/src/services/http_server.rs) builds the app via [build_app](backend/src/app.rs), which calls [api_router](backend/src/app.rs). `api_router` calls [build_api_router](backend/src/services/manager.rs) on the manager, which merges all registered route builders in order.

**Example (in `main` or when setting up the builder):**

```rust
let services = ServicesManager::builder()
    .add_api_routes("health", |state| crate::api::health::router())   // e.g. /api/healthz, /api/readyz
    .add_api_routes("artwork", |state| crate::api::artwork::router(state))
    .add_service(DatabaseServiceConfig { ... })
    // ...
    .start()
    .await?;
```

Each route builder receives [AppState](backend/src/app.rs) (config, db, schema, services) and returns a `Router<AppState>`. All returned routers are merged into the single `/api` router. GraphQL stays at `/graphql` and is not registered this way.

Services that only expose HTTP routes (no background tasks) can still register via `add_api_routes` without implementing the `Service` trait. Services that are also long-running should implement `Service` and can register their routes in `main` with `add_api_routes` when building the manager.

## Graceful shutdown

On shutdown, call `services.stop_all().await` so all services stop in **reverse dependency order** (dependents first, then dependencies). You can hook this into a signal handler or a shutdown channel used by the server.

## Example lifecycle (full app)

1. Create manager: `let services = Arc::new(ServicesManager::new());`
2. Connect database (e.g. `connect_with_retry`), then create and register `DatabaseService`, then register any service that depends on it (e.g. logging).
3. Register remaining services in dependency order.
4. Call `services.start_all().await` (starts in dependency order).
5. Build router (including `api_router(state)` for service routes), then run the server.
6. On shutdown: `services.stop_all().await`, then exit.
