# Backend Code Review

Date: 2026-02-12  
Scope: `backend/` (Rust Axum + GraphQL + services + macros)  
Focus: security, performance, reusability, fragmentation, consistency

## Executive Summary

The backend is feature-rich and compiles cleanly, but it currently has several high-risk authorization and operational security issues, plus substantial architectural inconsistency in data access and module boundaries.

Top concerns:
1. Authenticated users can access broad generated CRUD operations without role-level authorization.
2. Unauthenticated REST endpoints expose media/artwork/health data and filesystem-relevant state.
3. Any authenticated user can trigger privileged OS mount operations.
4. Sensitive values (token prefix, command-line passwords) are exposed in logs/process arguments.
5. Codebase consistency is degraded by duplicated implementations and very large multi-responsibility files.

---

## Findings (Ordered by Severity)

### 1. Missing RBAC on generated GraphQL CRUD allows broad privilege escalation
Severity: Critical  
Files:
- `macros/src/lib.rs:2307`
- `macros/src/lib.rs:2369`
- `macros/src/lib.rs:2443`
- `macros/src/lib.rs:2501`
- `backend/src/services/graphql/entities/user.rs:10`
- `backend/src/services/graphql/entities/app_setting.rs:8`
- `backend/src/services/graphql/entities/refresh_token.rs:5`
- `backend/src/services/graphql/entities/invite_token.rs:5`

Issue:
- Generated CRUD requires authentication (`ctx.auth_user()?`) but does not enforce role/ownership constraints.
- Sensitive tables (`users`, `app_settings`, `refresh_tokens`, `invite_tokens`, etc.) are exposed via `GraphQLOperations` derives.
- Any authenticated account appears able to perform global list/read/update/delete operations unless separately constrained.

Impact:
- Horizontal and vertical privilege escalation.
- Unauthorized changes to global config and user lifecycle data.

Recommendation:
- Introduce policy hooks in macro-generated operations (entity-level authorization policy trait).
- Apply `RoleGuard` (admin-only) or ownership checks per entity operation.
- Explicitly disable generated mutations for sensitive entities and expose controlled custom mutations instead.

---

### 2. Unauthenticated REST endpoints expose protected media/data surfaces
Severity: High  
Files:
- `backend/src/app.rs:40`
- `backend/src/api/media.rs:28`
- `backend/src/api/media.rs:57`
- `backend/src/api/artwork.rs:127`
- `backend/src/api/health.rs:31`
- `backend/src/api/health.rs:35`

Issue:
- `/api/media/{file_id}/stream`, `/api/media/{file_id}/info`, `/api/artwork/...`, `/api/artwork/stats`, `/api/healthz`, and `/api/readyz` are mounted without auth middleware.
- `healthz` also enumerates saved network path state.

Impact:
- Unauthorized media/artwork access.
- Information leakage of storage/network topology.

Recommendation:
- Add auth middleware for `/api` by default, then explicitly allow-list only truly public endpoints.
- Restrict health detail levels (public liveness vs authenticated diagnostics).

---

### 3. Privileged mount/reconnect operations available to any authenticated user
Severity: High  
Files:
- `backend/src/services/graphql/mutations/filesystem.rs:270`
- `backend/src/services/graphql/mutations/filesystem.rs:307`
- `backend/src/services/graphql/filesystem_network.rs:385`
- `backend/src/services/graphql/filesystem_network.rs:405`
- `backend/src/services/graphql/filesystem_network.rs:442`

Issue:
- `ConfigureNetworkPath` and `ReconnectLibraryPath` only require authentication, not admin role.
- Operations execute `net use`/`mount` for user-provided paths/credentials.

Impact:
- Non-admin users can trigger host-level network mounts and alter runtime filesystem behavior.

Recommendation:
- Make these mutations admin-only.
- Add strict allow-listing of mount targets and mount-point base paths.
- Add audit logs for who initiated mounts and what target was requested.

---

### 4. Sensitive credentials may leak via process arguments and logs
Severity: High  
Files:
- `backend/src/services/graphql/filesystem_network.rs:438`
- `backend/src/services/graphql/filesystem_network.rs:352`
- `backend/src/services/graphql/service.rs:120`

Issue:
- Linux CIFS mount passes `password=...` in command args (`mount -o ...password=...`), which can be visible in process lists.
- GraphQL auth failure logs include a token prefix.

Impact:
- Credential/token partial disclosure in runtime observability surfaces.

Recommendation:
- Use credential files or stdin-based secret passing for mounts (no password in argv).
- Remove token content from logs completely.

---

### 5. Open CORS policy for entire app increases cross-origin attack surface
Severity: Medium  
Files:
- `backend/src/app.rs:43`
- `backend/src/app.rs:44`

Issue:
- `allow_origin(Any)`, `allow_methods(Any)`, `allow_headers(Any)` globally.

Impact:
- Broadens browser-based exploitation possibilities and weakens defense-in-depth.

Recommendation:
- Restrict origins by environment config.
- Limit methods/headers to required set.

---

### 6. Password hashing/verification is blocking in async request flow
Severity: Medium  
Files:
- `backend/src/services/auth.rs:539`
- `backend/src/services/auth.rs:545`
- `backend/src/services/auth.rs:359`

Issue:
- `bcrypt::hash` and `bcrypt::verify` execute on async runtime threads.

Impact:
- Reduced throughput and potential latency spikes under auth load.

Recommendation:
- Move hash/verify to `tokio::task::spawn_blocking` and bound concurrency.

---

### 7. Login path performs duplicate lookups
Severity: Medium  
Files:
- `backend/src/services/auth.rs:362`
- `backend/src/services/auth.rs:369`

Issue:
- Login does two separate queries (`username` then `email`).

Impact:
- Extra DB round trip and unnecessary load on hot path.

Recommendation:
- Use one query with OR condition or normalized login identifier strategy.

---

### 8. Health endpoint can be expensive and leak infrastructure details
Severity: Medium  
Files:
- `backend/src/api/health.rs:35`
- `backend/src/api/health.rs:37`

Issue:
- On each `/healthz`, it loads saved network configs and checks path availability.

Impact:
- Latency and I/O overhead; possible instability under probes.

Recommendation:
- Keep health lightweight (liveness/readiness only).
- Move deep diagnostics to authenticated admin endpoint.

---

### 9. Architecture/data-access consistency violations (direct SQL spread)
Severity: Medium  
Files (examples):
- `backend/src/services/graphql/filesystem_network.rs:242`
- `backend/src/services/graphql/entities/source.rs:538`
- `backend/src/api/media.rs:230`
- `backend/src/api/artwork.rs:37`

Issue:
- Direct SQL is widely used in domain paths despite repository guidance to route through generated GraphQL entity layer.

Impact:
- Inconsistent code patterns, weaker abstraction boundaries, harder review/testing.

Recommendation:
- Define a strict boundary: entity/repository layer for domain reads/writes.
- Track exceptions explicitly and reduce them incrementally.

---

### 10. Duplicate filesystem browsing implementations create divergence risk
Severity: Medium  
Files:
- `backend/src/api/filesystem.rs:60`
- `backend/src/api/filesystem.rs:120`
- `backend/src/services/graphql/queries/filesystem.rs:175`
- `backend/src/services/graphql/queries/filesystem.rs:286`

Issue:
- Same logic exists in REST and GraphQL variants with slight behavioral differences.

Impact:
- Bug fixes and security hardening can drift between copies.

Recommendation:
- Extract shared filesystem browsing service and reuse from both API surfaces.

---

### 11. Large multi-responsibility files increase fragmentation and review cost
Severity: Medium  
Files:
- `backend/src/services/library_scan.rs` (6641 lines)
- `backend/src/services/auth.rs` (749 lines)
- `backend/src/services/graphql/filesystem_network.rs` (704 lines)
- `backend/src/services/graphql/entities/source.rs` (706 lines)

Issue:
- Several files combine orchestration, domain logic, infra, and adapters.

Impact:
- High cognitive load, fragile changes, inconsistent style over time.

Recommendation:
- Split by bounded contexts/use-cases:
  - service orchestration
  - persistence adapter
  - parser/transform logic
  - API resolver layer

---

### 12. Consistency debt: stale/typo artifacts and commented route history
Severity: Low  
Files:
- `backend/src/services/metadata/provders.rs:1`
- `backend/src/api/mod.rs:1`

Issue:
- Empty typo file (`provders.rs`) and commented legacy route notes reduce code clarity.

Impact:
- Signals uneven quality controls and confuses contributors.

Recommendation:
- Remove stale files/comments; keep module and route declarations canonical.

---

## Performance and Scalability Notes

1. Authentication path should be optimized first (`spawn_blocking` + single user lookup).
2. Health probes must remain O(1) and non-blocking.
3. Review DataLoader query generation and entity sorting defaults for heavy list operations; add indexes aligned to common filters/sorts.
4. Consider pagination caps and query complexity limits on GraphQL to reduce abusive query patterns.

## Consistency and Reusability Recommendations

1. Enforce one authorization model:
- Central policy layer (role + ownership + library scope) used by generated and custom resolvers.

2. Enforce one data access model:
- Domain operations via entity/repository abstractions, not scattered SQL in resolvers and services.

3. Enforce one filesystem/network model:
- Shared module for path normalization, permission checks, mount policy, and audit logging.

4. Enforce one error model:
- Standardized API error codes and user-safe messages; keep sensitive internals out of client-facing text.

5. Enforce one logging model:
- No tokens, no passwords, no secret prefixes in logs.

## Prioritized Remediation Plan

### Immediate (P0)
1. Lock down GraphQL CRUD by role/policy for sensitive entities.
2. Add auth/authorization middleware for `/api/*` and restrict public surface.
3. Make network mount/reconnect mutations admin-only with target/mount allow-lists.
4. Remove token prefix logging and password-in-argv behavior.

### Near Term (P1)
1. Move bcrypt work to blocking pool and optimize login query path.
2. Refactor health endpoint into lightweight probe + admin diagnostics.
3. Consolidate duplicated filesystem browsing logic.

### Follow-up (P2)
1. Break up large files into focused modules.
2. Eliminate stale artifacts (`provders.rs`, commented dead declarations).
3. Gradually migrate direct SQL call sites to approved abstractions.

## Validation Performed

- Static review across backend modules with targeted grep for auth, SQL, filesystem, process execution, and token handling.
- Build validation: `cargo check` in `backend/` succeeds (with warnings).

