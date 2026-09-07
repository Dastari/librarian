# Security Patch Progress (2026-07-02)

Source: docs/audit-2026-07.md sections 1 and 3. Starting point: tree compiles, 56/56 tests pass.

## Checklist

- [x] Item 1 — Mandatory JWT secret (CRITICAL) — services/auth.rs:44, main.rs:147
- [x] Item 2 — CORS allowlist (CRITICAL) — app.rs:42-48, config/mod.rs
- [x] Item 3 — Auth on media endpoints (HIGH)
- [x] Item 4 — Honor config.host when binding (HIGH) — services/http_server.rs:86
- [x] Item 5 — Disable introspection + GraphiQL outside dev (MEDIUM) — graphql/schema.rs, graphql/service.rs
- [x] Item 6 — GraphQL depth/complexity limits (MEDIUM) — graphql/schema.rs build_schema

All 6 items done. See "Final verification" at the bottom of the log.

## Log

### Item 1 — DONE
- `AuthConfig::from_env()` (backend/src/services/auth.rs) now returns `anyhow::Result<Self>`. No fallback
  string. Requires `LIBRARIAN_JWT_SECRET` (or legacy `JWT_SECRET`) to be set and >= 32 bytes, else returns
  an error naming the env var and `openssl rand -base64 48` as the fix.
- Removed `impl Default for AuthConfig` (was just `from_env()`, incompatible with the new `Result` return).
  Added `#[cfg(test)] AuthConfig::for_tests()` with a fixed non-secret 32+ byte string for test call sites.
- Call sites updated: `main.rs:147` now `AuthConfig::from_env()?` (propagates to startup failure via
  `anyhow::Result` in `async_main`); `services/graphql/schema.rs` test and `services/library_scan.rs` test
  switched from `from_env()`/`default()` to `for_tests()`.
- No backend `.env` or `.env.example` exists in the repo (only `frontend/.env`), so per the "if a dev .env
  exists" instruction, none was created. `docker-compose.dist.yml` already required `JWT_SECRET` via
  `${JWT_SECRET:?JWT_SECRET is required}` and documents `openssl rand -hex 32` — consistent with this change,
  no edits needed there.
- `cargo check` clean.

### Item 3 — DONE
- Frontend investigation: `frontend/src/components/VideoPlayer.tsx` exports `getMediaStreamUrl()`, the
  single URL-builder used directly as `<video src>`/`<audio src>` in `PersistentPlayer.tsx` and
  `PersistentAudioPlayer.tsx` — confirmed media elements can't send Authorization headers, so a query-param
  fallback is required. `frontend/src/lib/api/authFetch.ts` (GraphQL/API calls) sends `Authorization: Bearer`
  from `getAccessToken()` (`frontend/src/lib/auth.ts`, token stored in a JS-readable cookie). No frontend
  caller of `/api/media/*/info` was found.
- New `backend/src/api/auth_guard.rs`: `require_authenticated_user(state, headers, query_token)` (header
  `Authorization: Bearer` OR `token` query param, via `AuthService::authenticate_access_token` — same
  validation GraphQL uses) and `require_authenticated_admin` (header-only, 403 if not admin). Reachable from
  `AppState.services.get_auth()` — no new state wiring needed.
- `backend/src/api/media.rs`: both `stream_media` and `media_info` now call `require_authenticated_user`
  first and return 401 on failure; `StreamParams`/new `MediaInfoParams` gained a `token: Option<String>`
  field for the query-param fallback.
- `backend/src/api/artwork.rs`: `storage_stats` (`/api/artwork/stats`) now requires an authenticated admin
  (401/403). `serve_artwork` (img tags) intentionally left open per instructions.
- `backend/src/api/health.rs`: `healthz` returns bare `status`/`version` (platform + network_paths now
  `Option`, `skip_serializing_if` none) unless the caller is authenticated (any valid token), in which case
  the previous full payload is returned.
- Chromecast caveat found and handled: `castMedia` (`services/graphql/entities/cast.rs`) builds a
  `/api/media/*/stream` URL that Chromecast devices fetch directly over LAN with no way to send
  Authorization headers — this would have silently broken casting. Fixed by threading the caller's own
  access token through: `services/graphql/auth.rs` adds `RawAccessToken(pub String)`; `graphql/service.rs`'s
  `graphql_handler` inserts it into request context data alongside `AuthUser` on successful auth; `cast_media`
  reads it via `ctx.data::<RawAccessToken>()` and appends `?token=...` (urlencoded) to the constructed
  stream URL. Token used is the requesting user's own short-lived access token, not a new credential.
- Logging caveat: verified `media.rs`'s own tracing calls never log the whole `StreamParams`/whole request
  URI (only `media_file_id`/`path` fields individually), so no token leak there. The broader risk was the
  app-wide `TraceLayer` (`app.rs`), whose default `DefaultMakeSpan` logs `request.uri()` including the query
  string. Changed `TraceLayer::new_for_http()` to `.make_span_with(...)` logging `request.uri().path()` only
  (no query string) — closes the token-in-logs risk for any query-string-bearing request, not just media.
- Frontend: `VideoPlayer.tsx`'s `getMediaStreamUrl()` now appends `?token=<access token>` (from
  `lib/auth.ts::getAccessToken()`, URL-encoded) when a token is present; both `<video>` and `<audio>` src
  call sites go through this one helper, so no other frontend files needed changes.
- `cargo check` clean; `pnpm exec tsc --noEmit` clean; `pnpm test` 19/19 passed; `cargo test` 56/56 passed
  (44 unit + 12 contract) — matches baseline, confirming items 1-3 introduced no regressions.

### Item 4 — DONE
- `backend/src/services/http_server.rs`: added `HttpServerService::bind_ip()` — parses `config.host` (trimmed)
  as `IpAddr` when set/non-empty; on parse failure logs a `tracing::warn!` and falls back to `0.0.0.0`
  (`Ipv4Addr::UNSPECIFIED`), same fallback used when host is unset/empty (preserves current behavior in that
  case). `start()` now binds `SocketAddr::new(bind_ip, config.port)` instead of the hardcoded `[0,0,0,0]`.
- `cargo check` clean.

### Item 2 — DONE
- Added `Config.cors_origins: Vec<String>` (backend/src/config/mod.rs), parsed from comma-separated
  `LIBRARIAN_CORS_ORIGINS`; defaults to `["http://localhost:3000", "http://127.0.0.1:3000"]` (Vite dev
  server port confirmed in frontend/vite.config.ts: `server.port = 3000`).
  Also added `pub fn dev_mode()` helper (`cfg!(debug_assertions) || LIBRARIAN_DEV=1`) here for item 5 reuse.
- backend/src/app.rs: replaced `AllowOrigin::mirror_request()` + mirrored methods/headers with a new
  `cors_layer(&Config)` that builds `AllowOrigin::list(...)` from parsed `HeaderValue`s (invalid entries are
  logged and skipped, not fatal), explicit methods GET/POST/PUT/DELETE/OPTIONS, explicit headers
  content-type/authorization, `allow_credentials(true)` retained.
- `cargo check` clean.

### Item 5 & 6 — DONE
- Discovered the just-landed graphql-orm 0.2.18 upgrade already changed `schema_builder(db)` (the generated
  helper `build_schema` calls) to apply default `SchemaLimits` (depth 16 / complexity 20_000) itself — see
  `graphql-orm/src/graphql/orm/query.rs` `SchemaLimits::default()`. It does **not** touch introspection.
- `backend/src/config/mod.rs`: added `pub fn dev_mode() -> bool` = `cfg!(debug_assertions) ||
  LIBRARIAN_DEV=1/true/yes/on` (added while doing item 2, reused here).
- `backend/src/services/graphql/schema.rs` (`build_schema`): after ORM defaults are applied, explicitly
  chains `.limit_depth(20).limit_complexity(10_000)` (overrides the ORM defaults — async-graphql's
  `limit_depth`/`limit_complexity` just set a field, last call wins) and calls `.disable_introspection()`
  unless `dev_mode()`. Checked depth against the largest frontend query documents
  (`frontend/src/lib/graphql/documents/*.graphql`): brace-nesting depth tops out around 7-8
  (`torrents.graphql`, `manual-match.graphql`, `libraries.graphql` at 7; `shows.graphql`/`collection-tabs.graphql`
  at 6) even before accounting for fragments being inlined slightly deeper — 20 leaves generous headroom.
  10,000 complexity is per the audit's suggested value and well above what these list/relation-shaped queries
  should hit; flagged as something to watch (not runnable-verifiable without executing real queries against
  a live server, which is out of scope per the "never run cargo run" rule) — raise `limit_complexity` if a
  legitimate query is ever rejected.
- `backend/src/services/graphql/service.rs` (`graphiql` handler): now serves the GraphiQL HTML page only
  when `crate::config::dev_mode()` is true; outside dev, an HTML-accepting GET returns `404 Not Found`
  (doesn't hint the playground exists) instead of the page, non-HTML GETs keep the existing 405 JSON error.
- `cargo check` clean; `cargo test` 56/56 passed (44 unit + 12 contract) — `schema_builds_without_recursing`
  and `core_services_start_with_storage_and_backup_registered` still pass; both run under `cargo test`
  (debug build, `cfg!(debug_assertions)` true) so `dev_mode()` is true and introspection/GraphiQL stay
  enabled in that context, unaffected by this change.
- `cargo clippy`: 15 warnings total (4 duplicates), all pre-existing in files untouched by this patch
  (`entities/chapter.rs`, `episode.rs`, `movie.rs`, `track.rs`, `torrent.rs`, and pre-existing
  `library_scan.rs` lines unrelated to the one line changed there for item 1's test helper) — zero new
  warnings in any file touched by this patch.

## Final verification (all 6 items complete)

- `cargo check` — clean, no warnings/errors.
- `cargo test` — 56/56 passed (44 unit + 12 contract), matches the starting baseline exactly.
- `cargo clippy` — no new warnings in any file this patch touched.
- `pnpm exec tsc --noEmit` (frontend) — clean.
- `pnpm test` (frontend) — 19/19 passed.
- Files touched (backend): `services/auth.rs`, `main.rs`, `services/graphql/schema.rs`,
  `services/library_scan.rs` (1 line, test helper only), `config/mod.rs`, `app.rs`, `api/media.rs`,
  `api/auth_guard.rs` (new), `api/mod.rs`, `api/artwork.rs`, `api/health.rs`, `services/graphql/auth.rs`,
  `services/graphql/mod.rs`, `services/graphql/service.rs`, `services/graphql/entities/cast.rs`,
  `services/http_server.rs`.
- Files touched (frontend): `components/VideoPlayer.tsx`.
- No git commits made. No files reformatted or reverted beyond the above.

