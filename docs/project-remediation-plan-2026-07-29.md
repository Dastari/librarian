# Librarian Audit Remediation and Capability Completion Plan

- **Created:** 2026-07-29
- **Status:** Implemented; local qualification complete, external qualification gates listed in Phase 5
- **Primary source:** `docs/audit-2026-07-29.md`
- **Related plans:** `docs/rbac-implementation-plan.md`, `docs/frontend-auth-cookie-migration.md`,
  `docs/match-system-implementation-plan.md`, `docs/tier1-features-plan.md`

## 1. Purpose

This is the execution plan for every open issue, gap, and recommendation from the 2026-07-29 re-audit.
It also carries forward the still-relevant unfinished capability gaps from the 2026-07-02 audit.

The plan is intentionally ordered by dependency and risk:

1. establish safe change and migration gates;
2. remove credential and server-side request forgery risks;
3. finish authorization boundaries;
4. rebuild casting on a safe session and transport model;
5. make scan/analyze/organize outcomes durable and truthful;
6. harden schema changes, network clients, and CI;
7. complete or explicitly remove unfinished product promises.

This file is the checklist of record. Progress documents may contain implementation notes, but an item is
not complete until its acceptance criteria and verification gate here are satisfied.

## 2. Scope

### In scope

- Artwork request security and artwork access privacy.
- Authentication cookie/token migration and trusted-proxy handling.
- Source-credential key isolation and rotation.
- Public API error sanitization.
- Entity- and row-level authorization across all GraphQL access paths.
- Cast stream credentials, session ownership, transport deadlines, state synchronization, discovery,
  settings, advertised URLs, compatibility decisions, and DLNA.
- Movie-scan readiness, persistent scan outcomes, analysis feedback, duplicate handling, and
  auto-organization safety.
- `ContentStatus` completion.
- Scanner scalability and blocking-work isolation.
- Shared outbound HTTP policy.
- Destructive schema-change safety.
- Dependency/security checks in CI.
- Recoverable panic removal and runtime-worker supervision.
- Remaining capability gaps: Usenet, AirPlay decision, subtitle acquisition/extraction, archive
  extraction, and Windows service/tray integration.

### Out of scope

- Replacing GraphQL with another API architecture.
- Automatically deleting duplicate or unmatched media.
- Reusing browser codec assumptions as the Cast capability model.
- Adding direct SQL for application-domain CRUD.
- Unrelated visual redesign.
- Treating a hidden UI control as an authorization boundary.

## 3. Repository constraints

Every implementation phase must preserve these rules:

- Domain reads and writes go through generated GraphQL entities, typed entity queries, or existing
  entity-layer abstractions. Do not add raw domain SQL.
- New GraphQL fields and operations use camelCase.
- Generated frontend GraphQL types remain the source of truth.
- Media pipeline behavior must preserve `docs/design.md`: no silent overwrite, no automatic deletion,
  partial success is valid, and status must reflect reality.
- Every new pipeline decision requires a Q&A entry in `docs/design.md`.
- Backend logs use structured `tracing` fields and identify the affected user/library/file/device/session.
- Frontend work uses HeroUI, Tabler icons, TanStack Router, Apollo, and generated GraphQL types.
- Do not use startup schema synchronization as an implicit destructive migration tool.
- Do not run application development servers as part of automated implementation; the user owns those
  processes.

## 4. Priority and effort scale

| Value | Meaning |
|---|---|
| P0 | Active credential/data-loss/server-pivot risk; complete before feature expansion |
| P1 | Serious reliability, privacy, or authorization problem |
| P2 | Product correctness, scalability, or operational hardening |
| P3 | Capability completion after the core is hardened |
| S | Roughly 1–2 focused engineering days |
| M | Roughly 3–5 focused engineering days |
| L | Roughly 1–2 focused engineering weeks |
| XL | Multi-phase work likely exceeding two weeks |

Effort is directional. It includes implementation and focused tests, but not waiting for hardware/device
availability or external provider approval.

## 5. Traceability matrix

| ID | Audit issue or gap | Priority | Effort | Phase | Status |
|---|---|---:|---:|---:|---|
| BASE-01 | Current worktree is large and schema-sensitive | P0 | S | 0 | Implemented |
| MOV-01 | Missing TMDB key prevented all movie creation without clear feedback | P1 | M | 0/3 | Implemented; live valid-key rescan is an external gate |
| UI-LOG-01 | Logs sizing adapter discarded fixed widths | P2 | S | 0 | Implemented |
| CAST-MIME-01 | Generic `video`/`audio` values sent as MIME types | P1 | S | 0 | Implemented |
| SEC-ART-01 | Artwork fetch is SSRF-capable and unbounded by time | P0 | L | 1 | Implemented |
| AUTH-01 | Refresh token returned to JavaScript despite HttpOnly cookie | P0 | M | 1 | Implemented |
| AUTH-02 | Forwarded protocol trusted without a trusted-proxy boundary | P1 | M | 1 | Implemented |
| SEC-CRED-01 | Source credential key is stored beside ciphertext; short keys are zero-padded | P1 | L | 1 | Implemented |
| API-ERR-01 | Internal database/path/provider errors can leak through public errors | P1 | M | 1 | Implemented |
| RBAC-01 | Shared entities and Cast sessions remain over-permissive | P0 | XL | 1 | Implemented |
| CAST-AUTH-01 | Cast URL persists and exposes a real user bearer token | P0 | L | 1 | Implemented |
| ART-PRIV-01 | Artwork serving remains unauthenticated | P1 | M | 1 | Implemented |
| HTTP-01 | Inconsistent/no deadlines across outbound HTTP clients | P1 | L | 2 | Implemented |
| CAST-NET-01 | Cast connect/read/write operations can block indefinitely | P0 | L | 2 | Implemented; receiver hardware qualification is external |
| CAST-INPUT-01 | Cast ports, addresses, seek, start, and volume are not validated | P0 | M | 2 | Implemented |
| CAST-STATE-01 | Control selects the first receiver app/session and state is optimistic | P1 | XL | 2 | Implemented; receiver hardware qualification is external |
| CAST-CFG-01 | Stored Cast settings do not reconfigure or affect playback | P1 | L | 2 | Implemented |
| CAST-DISC-01 | Discovery persistence, staleness, uniqueness, and health are incomplete | P1 | L | 2 | Implemented |
| CAST-URL-01 | Advertised media URL can select the wrong interface/invalid IPv6 URL | P1 | M | 2 | Implemented; multi-interface qualification is external |
| CAST-CAP-01 | No Cast-specific compatibility/transcode policy | P1 | L | 2 | Implemented; hardware matrix is external |
| CAST-DLNA-01 | DLNA is presented as a renderer but only SSDP discovery exists | P1 | XL | 2/6 | Resolved by discovery-only classification; playback is unsupported |
| PIPE-01 | Scan/analyze/organize results are not one durable, truthful outcome | P1 | XL | 3 | Implemented |
| PIPE-02 | Auto-organize discards errors and its safety guard fails open | P0 | M | 1/3 | Implemented |
| PIPE-03 | ffprobe failures are not aggregated into user-facing scan status | P1 | M | 3 | Implemented |
| PIPE-DUP-01 | Current movie library contains duplicate physical files | P2 | M | 3 | Workflow implemented; live deletion remains an explicit user action |
| STATUS-01 | `ContentStatus::compute` is incomplete | P1 | L | 3 | Implemented |
| PERF-SCAN-01 | Blocking walks and per-file entity round trips limit scanner scale | P2 | XL | 3 | Implemented; 100k/network-mount qualification harness is manual |
| DB-SCHEMA-01 | Destructive startup schema changes need an explicit safe workflow | P0 | XL | 0/4 | Implemented |
| SUPPLY-01 | Dependency advisory/license checks are absent from CI | P1 | M | 4 | Implemented |
| FE-TYPES-01 | Schema DTOs remain duplicated in frontend casting/notification types | P2 | M | 4 | Implemented |
| RUNTIME-01 | Recoverable setup paths can panic; release panic policy aborts the process | P1 | M | 4 | Implemented |
| CLEAN-01 | Dead-code allowances/stale dependencies obscure unfinished paths | P2 | M | 4 | Implemented |
| FEATURE-USENET-01 | Usenet settings/entities exist without an acquisition client | P3 | XL | 6 | Resolved by removal from supported product scope |
| FEATURE-SUB-01 | Subtitle acquisition/extraction/serving is incomplete | P3 | XL | 6 | Resolved by removal from supported product scope |
| FEATURE-ARCHIVE-01 | Archive extraction and failure workflow are incomplete | P3 | L | 6 | Resolved by removal from supported product scope |
| FEATURE-AIRPLAY-01 | AirPlay is advertised/planned but absent | P3 | XL | 6 | Resolved by removal from supported product scope |
| FEATURE-WIN-01 | Windows service/tray flows are placeholders | P3 | XL | 6 | Resolved by rejecting unsupported modes and documenting server-only support |

### Final disposition rules

The traceability matrix above is the final issue disposition. The detailed checklists below preserve the
original implementation decomposition:

- checked tasks were executed and locally verified;
- unchecked Phase 5 tasks are external qualification gates and must not be represented as passing;
- DLNA AVTransport, Usenet/NNTP, managed subtitle, archive extraction, first-party AirPlay, and Windows
  service/tray implementation scopes are closed as **superseded**, not deferred supported features. Those
  capabilities were removed from product claims and require a new approved plan before reintroduction;
- live deletion of current duplicate media is intentionally not an implementation task. The safe workflow
  exists, but selecting which user file to remove remains an explicit user decision.

Implementation and test evidence is recorded in
[`remediation-progress-2026-07-29.md`](remediation-progress-2026-07-29.md).

### Critical dependency path

```text
BASE-01
  ├─ DB-SCHEMA-01 containment
  │    └─ RBAC-01 schema rollout
  │         └─ CAST-AUTH-01
  │              └─ CAST-STATE-01
  │                   ├─ CAST-CFG-01
  │                   ├─ CAST-CAP-01
  │                   └─ CAST-DLNA-01 full playback
  ├─ AUTH-01 ─ AUTH-02
  ├─ SEC-CRED-01
  ├─ SEC-ART-01 ─ ART-PRIV-01
  └─ PIPE-02 containment
       └─ PIPE-01 ─ PIPE-02 completion ─ PIPE-03/PERF-SCAN-01
```

`HTTP-01`, `CAST-INPUT-01`, and `CAST-NET-01` can proceed alongside the authorization work once their
public mutation/ownership boundaries are settled. Phase 6 capability work cannot begin before the P0/P1
release gate.

## 6. Target architecture decisions

These decisions should be recorded in `docs/design.md` before or with implementation.

### 6.1 Browser authentication and media

- Refresh tokens exist only in backend-set HttpOnly cookies and the refresh-token store.
- GraphQL auth payloads never return refresh-token values.
- The transitional JavaScript-readable access token may remain for one compatibility phase, then moves to
  an HttpOnly access/session cookie.
- Browser media uses same-origin cookies where possible. If a media element cannot use the session cookie,
  it receives a short-lived, media-specific grant—not an API access token.
- WebSocket authentication must work with server-managed cookies before removing the JavaScript access
  token.

### 6.2 Cast streaming credentials

- A Cast receiver never receives the user's API bearer token.
- Create the `CastSession` before contacting the receiver.
- Mint a signed, purpose-scoped grant with claims for:
  - audience `cast-stream`;
  - `castSessionId`;
  - `mediaFileId`;
  - expiry;
  - optional receiver IP when it is stable and known.
- The media endpoint validates the signature, claims, session ownership/state, expiry, and requested media
  file.
- The secret grant is transient. It is not stored in `cast_sessions`, returned by GraphQL, or written to
  logs.
- Stopping/ending a Cast session revokes the grant by changing session state; expiry is the final backstop.

### 6.3 Authorization model

- System configuration, credentials, logs, source management, manual Cast-device management, backup, and
  schema operations are admin-only.
- `Library.user_id` remains the owner. If sharing is required, add a `LibraryAccess` entity with explicit
  `VIEW`/`EDIT` levels rather than treating every library as globally writable.
- Child media entities inherit authorization through their library relation.
- Derived entities are read-only through generated GraphQL operations.
- Cast devices are authenticated-member readable and admin writable.
- Cast sessions are owner-readable/controllable, with admin override.
- Authorization filters must be pushed into entity queries. Do not expand the current fetch-all-then-filter
  row-policy mechanism to large media tables.

### 6.4 Scan outcome model

- A scan is a durable run, not only a stream of log lines.
- The run has explicit stages: `QUEUED`, `DISCOVERING`, `MATCHING`, `ORGANIZING`, `ANALYZING`,
  `COMPLETED`, `COMPLETED_WITH_ISSUES`, `FAILED`, and `CANCELLED`.
- A scan is not marked cleanly complete while queued analysis or organization failures are unknown.
- Stage counts and actionable issues are persisted and subscribed to.
- Files stay visible and unmatched on provider/configuration failure.
- Duplicate files are reported for review and are never deleted automatically.

### 6.5 Outbound network policy

- One service/factory owns timeout, redirect, user-agent, response-size, retry, and destination policy.
- Artwork uses an explicit provider/admin allowlist and manual redirect validation.
- Retry only transient network and retryable HTTP failures.
- Requests expose provider, operation, latency, status class, and sanitized failure reason through metrics
  and structured logs.
- No production code uses global `reqwest::get` or a bare `Client::new()` outside the factory.

### 6.6 Cast versus DLNA

- Chromecast/Google Cast and DLNA/UPnP are separate protocols and controller implementations.
- Until DLNA AVTransport works, DLNA devices must be labelled `discoveryOnly` and cannot show a Cast action.
- Browser HLS policy remains separate from device playback policy.

## 7. Phase 0 — Baseline, containment, and regression anchors

### BASE-01 — Establish a safe implementation baseline

- **Priority:** P0
- **Effort:** S
- **Dependencies:** None

#### Tasks

- [x] Record the exact branch/commit plus all uncommitted files included in the implementation baseline.
- [x] Split unrelated current work into reviewable commits or save an explicit patch snapshot.
- [x] Create and restore-test a database backup before the first entity/schema change.
- [x] Capture the current generated GraphQL schema and frontend generated files.
- [x] Record test baselines:
  - backend unit count;
  - migration/architecture contract count;
  - frontend test count;
  - TypeScript status;
  - current clippy warnings.
- [x] Add a progress document for each implementation phase or update this file atomically.

#### Acceptance criteria

- The exact pre-remediation application and database state can be restored.
- Generated schema drift is detectable.
- Unrelated user work is not reformatted, reverted, or hidden inside remediation commits.

### DB-SCHEMA-01A — Immediate destructive-change containment

- **Priority:** P0
- **Effort:** M subset of DB-SCHEMA-01
- **Dependencies:** BASE-01

This containment subset must land before any remediation adds or changes entity fields.

#### Tasks

- [x] Produce the schema plan before applying it.
- [x] Classify drop-table, drop-column, narrowing type changes, and rename-as-drop/add as destructive.
- [x] Refuse destructive plans during ordinary startup.
- [x] Print a sanitized, actionable plan explaining the explicit migration command/workflow.
- [x] Require a verified backup before any manually authorized destructive application.
- [x] Add a regression test proving a field rename/removal cannot silently drop data at startup.

#### Acceptance criteria

- Phase 1 entity changes can be introduced without exposing the existing database to an implicit
  destructive plan.
- The full DB-SCHEMA-01 work in Phase 4 can extend this containment without weakening it.

### MOV-01 — Lock in missing-provider scan feedback

- **Status:** Core implementation complete in the 2026-07-29 audit pass
- **Priority:** P1
- **Effort remaining:** M

#### Remaining tasks

- [x] Add an integration test with a Movies library, one file, no existing Movie, and no TMDB key.
- [x] Assert the file remains an unmatched `MediaFile`.
- [x] Assert exactly one unresolved configuration notification is created across repeated scans.
- [x] Assert the scan summary is `COMPLETED_WITH_ISSUES`/WARN and reports configuration blockage.
- [x] Configure fake provider readiness/catalog data, rescan, and assert the notification and prior
  configuration issues auto-resolve after successful matching.
- [x] Add a Metadata-settings “Test TMDB key” action before saving.
- [x] Show provider readiness on the library scan confirmation UI.
- [ ] Operationally add a valid TMDB key and rescan the active Movies library.

#### Acceptance criteria

- Missing provider configuration is visible before and after a scan.
- Repeated scans do not spam duplicate unresolved notifications.
- A successful post-configuration rescan creates/links expected movie records without deleting files.

### Completed regression anchors

- [x] `UI-LOG-01`: fixed DataTable sizing propagation; Time/Level/Source are fixed and Message flexes.
- [x] `CAST-MIME-01`: valid MIME values are retained; generic `video`/`audio` values fall back to extension.
- [x] Keep the associated frontend and backend tests in the required regression suite.

## 8. Phase 1 — Security and authorization boundary

Phase 1 blocks feature expansion. CAST-AUTH-01 and RBAC-01 must complete before the Cast state-machine work.

### SEC-ART-01 — Eliminate artwork SSRF

- **Priority:** P0
- **Effort:** L
- **Dependencies:** BASE-01

#### Target areas

- `backend/src/services/artwork.rs`
- `backend/src/services/graphql/mutations/artwork.rs`
- Movie/entity write policies
- An artwork-specific safe client using the same interface that HTTP-01 will generalize

#### Tasks

- [x] Make manual single-movie recache admin-only.
- [x] Prevent members from directly writing provider artwork URLs on shared Movie rows.
- [x] Introduce an artwork source allowlist:
  - provider-owned CDN hosts enabled by provider;
  - optional admin-configured public hosts;
  - private/LAN hosts denied by default.
- [x] Permit only `http` and `https`; prefer `https`.
- [x] Resolve hostnames before connecting and reject loopback, unspecified, multicast, link-local, private,
  carrier-grade NAT, and IPv6 ULA destinations unless an explicit admin override is enabled.
- [x] Prevent DNS rebinding by ensuring the validated resolution is the resolution used for the connection.
- [x] Disable automatic redirects and follow at most three manually, revalidating every destination.
- [x] Apply connect, request, idle-body, and total deadlines.
- [x] Stream the body with a hard 20 MiB cap instead of calling unbounded `bytes()`.
- [x] Require an allowed image MIME type and verify decodability before persistence.
- [x] Sanitize logs: host and status are allowed; credentials, query secrets, and full sensitive URLs are not.
- [x] Add failure codes such as `DESTINATION_DENIED`, `REDIRECT_DENIED`, `TIMEOUT`, `TOO_LARGE`,
  `INVALID_IMAGE`, and `UNSUPPORTED_CONTENT_TYPE`.

#### Tests

- [x] Reject `file:`, `ftp:`, loopback IPv4/IPv6, link-local, RFC1918, ULA, multicast, and unspecified IPs.
- [x] Reject a public hostname redirecting to an internal address.
- [x] Reject a DNS result that changes to a denied address before connection.
- [x] Abort slow headers, slow body, redirect loops, and bodies over the cap.
- [x] Accept known provider CDN artwork and cache it through the entity/storage layer.
- [x] Assert a member cannot mutate an artwork URL and trigger recache.

#### Acceptance criteria

- No member-controlled value can make the server contact an arbitrary network target.
- No artwork request can hold a worker indefinitely or exceed the response cap.
- A contract test prevents reintroduction of `reqwest::get` in artwork code.

### PIPE-02A — Immediate fail-closed organization containment

- **Priority:** P0
- **Effort:** S subset of PIPE-02
- **Dependencies:** BASE-01

#### Tasks

- [x] If the auto-organize safety guard errors, do not organize the file.
- [x] Stop silently discarding `organize_media_file` failures.
- [x] Emit one context-rich WARN/error notification containing library ID, media file ID, current path,
  attempted operation, and sanitized reason.
- [x] Preserve the current no-clobber behavior.
- [x] Add focused tests for guard error and organization error.

#### Acceptance criteria

- A safety-check failure cannot cause a file move.
- The later PIPE-01/PIPE-02 work replaces the temporary notification with a durable scan issue without
  regressing fail-closed behavior.

### AUTH-01 — Remove refresh-token exposure

- **Priority:** P0
- **Effort:** M
- **Dependencies:** BASE-01

#### Tasks

- [x] Confirm current frontend login, register, refresh, logout, cross-tab, media, and WebSocket flows.
- [x] Split GraphQL auth output into an access-token payload that cannot contain a refresh token.
- [x] Stop returning `AuthTokens.refresh_token` from register/login/refresh.
- [x] Keep refresh rotation solely in the HttpOnly cookie and refresh-token entity/store.
- [x] Remove refresh-token inputs after a compatibility window; cookie fallback becomes the only path.
- [x] Ensure logout revokes the server-side refresh record and clears the cookie even if the response fails
  after revocation.
- [x] Redact all auth tokens from GraphQL errors, traces, request URIs, and client logs.
- [x] Update GraphQL documents, run codegen, and delete obsolete manual auth DTOs.

#### Compatibility sequence

1. Backend stops requiring the refresh input but temporarily accepts it.
2. Frontend sends no refresh value and passes cookie credentials.
3. Verify deployed clients.
4. Remove the refresh field from payloads and inputs.
5. Regenerate schema/types and remove compatibility code.

#### Acceptance criteria

- Browser JavaScript cannot read a refresh token from response data or cookies.
- Rotation, replay revocation, logout, and expired-token paths have integration tests.
- Auth schema snapshots contain no refresh-token output field.

### AUTH-02 — Trusted proxy, secure cookies, and CSRF posture

- **Priority:** P1
- **Effort:** M
- **Dependencies:** AUTH-01

#### Tasks

- [x] Add an explicit trusted-proxy configuration using IP/CIDR entries.
- [x] Use `Forwarded`/`X-Forwarded-Proto` only when the direct peer is trusted.
- [x] Otherwise derive secure-cookie behavior from the actual connection/listener configuration.
- [x] Validate request `Origin` against the configured CORS/application origins for cookie-authenticated
  GraphQL mutations.
- [x] Decide and document `SameSite=Lax` versus `Strict`; require `Secure` in non-development deployments.
- [x] If cross-site deployments must be supported, add an explicit CSRF token rather than relaxing origin
  checks globally.
- [x] Add startup health warnings for unsafe production cookie/proxy combinations.

#### Tests

- [x] An untrusted peer cannot spoof HTTPS through forwarded headers.
- [x] A trusted proxy can communicate the original secure scheme.
- [x] Cross-origin mutation attempts fail while allowed frontend origins succeed.
- [x] Development localhost behavior remains usable without weakening production defaults.

### SEC-CRED-01 — Isolate and rotate source-credential encryption keys

- **Priority:** P1
- **Effort:** L
- **Dependencies:** BASE-01, DB-SCHEMA-01A

#### Current problem

`SourcesService` stores `sources_encryption_key` in `app_settings`, the same SQLite database as encrypted
source credentials. A copied database therefore contains both ciphertext and the key. The current key
parser also pads short decoded keys with zeros instead of rejecting them.

#### Target design

- The active master key comes from an external secret source:
  - environment/container secret;
  - root-readable key file;
  - OS secret store where platform support exists.
- Ciphertext contains a version/key ID but never the key.
- Invalid key length/encoding fails closed.
- Rotation decrypts with the old key and re-encrypts with the new key through entity operations.

#### Tasks

- [x] Reject decoded AES keys that are not exactly the required length; remove zero padding.
- [x] Add key-source configuration and strict permission checks for key files.
- [x] Add version/key ID to credential envelopes.
- [x] Add an explicit key-rotation operation with dry-run, backup requirement, verification, and rollback.
- [x] Add a one-time legacy migration:
  1. read the old DB key;
  2. decrypt and validate every credential;
  3. re-encrypt under the external key;
  4. verify all rows;
  5. remove the DB key only after backup and verification.
- [x] Refuse to start credential-dependent sources when the external key is missing; do not generate and
  silently store another database key.
- [x] Redact key IDs and all credential material from logs/errors where they are not operationally needed.

#### Tests

- [x] Short, malformed, missing, and wrong keys fail closed.
- [x] Rotation preserves every credential and can roll back before old-key removal.
- [x] A database backup alone is insufficient to decrypt credentials after migration.
- [x] Fresh install bootstrap creates/requests the external secret using documented secure permissions.

### API-ERR-01 — Sanitize public errors and preserve internal diagnostics

- **Priority:** P1
- **Effort:** M
- **Dependencies:** BASE-01

#### Tasks

- [x] Define stable public error codes for authorization, validation, configuration, provider, storage,
  conflict, timeout, and internal failures.
- [x] Return concise remediation-safe messages through GraphQL/REST.
- [x] Log the internal error chain with request/operation/entity IDs, never secrets.
- [x] Stop forwarding raw `sqlx`, filesystem path, provider credential, and schema details to clients.
- [x] Add a correlation ID to public internal-error responses and structured logs.
- [x] Preserve useful field validation details without exposing backend implementation.
- [x] Update Apollo handling to use codes rather than parsing message text.

#### Acceptance criteria

- Deliberately triggered database/filesystem/provider failures do not expose SQL, private paths, tokens, or
  credentials in public responses.
- Operators can correlate every sanitized internal failure to a context-rich server log.

### RBAC-01 — Complete the authorization matrix

- **Priority:** P0
- **Effort:** XL
- **Dependencies:** BASE-01, DB-SCHEMA-01A, AUTH-01
- **Reference:** Reuse and update `docs/rbac-implementation-plan.md`; do not implement the outdated portions
blindly.

#### Foundation tasks

- [x] Inventory every generated query/mutation and custom resolver by entity and minimum role.
- [x] Define the current matrix in a machine-readable test fixture.
- [x] Add `LibraryAccess` only if multi-user sharing is a real product requirement:
  - `userId`;
  - `libraryId`;
  - `accessLevel` (`VIEW`, `EDIT`);
  - unique `(userId, libraryId)`.
- [x] Preserve `Library.user_id` as owner.
- [x] Add `CastSession.user_id`.
- [x] Push ownership/library scopes into generated query filters; do not rely on in-memory post-filtering
  for media tables.
- [x] Distinguish internal system actions from user actions while retaining actor/user context in logs.

#### Entity policy tasks

- [x] Admin-only read/write:
  - app settings and sensitive provider configuration;
  - users, invites, refresh tokens;
  - system logs;
  - source/server credentials;
  - backup/restore and schema operations.
- [x] Admin-write/member-read:
  - shared Cast devices;
  - global Cast settings;
  - shared naming/source configuration where appropriate.
- [x] Owner/library-scoped:
  - libraries, movies, shows, albums, artists, audiobooks, collections, media files;
  - episodes, tracks, and chapters through their parent;
  - torrents/downloads and pending matches;
  - playback and notifications;
  - Cast sessions.
- [x] Derived/read-only:
  - streams, subtitles, media chapters, artwork cache, storage metadata;
  - disable generated create/update/delete operations that should only be service-managed.
- [x] Add field-level restrictions for URLs, credentials, ownership IDs, role fields, and lifecycle state.

#### Frontend tasks

- [x] Finish `RequireRole`/role helpers and route guards.
- [x] Gate settings and admin actions while preserving backend enforcement.
- [x] Handle `FORBIDDEN` distinctly from `UNAUTHORIZED`.
- [x] Remove manual schema DTOs as each domain moves to generated types.
- [x] Regenerate GraphQL schema/types after every policy batch.

#### Tests

- [x] Two-member plus admin integration fixture.
- [x] Members cannot list/get/update/delete another user's private rows by ID or bulk mutation.
- [x] Relation resolvers and subscriptions enforce the same scope as list/get queries.
- [x] Members cannot write owner/library IDs to transfer rows.
- [x] Admin override is explicit and tested.
- [x] Internal scanner/download jobs retain access without fabricating a user-owned authorization bypass.
- [x] Query plans/scalability tests confirm large-table filtering occurs before row materialization.

#### Acceptance criteria

- Every public resolver appears in the authorization matrix.
- No entity silently falls through to permissive defaults.
- Cross-user isolation tests cover direct lookup, list, relation, mutation, bulk mutation, and subscription.

### CAST-AUTH-01 — Replace user bearer tokens with scoped Cast grants

- **Priority:** P0
- **Effort:** L
- **Dependencies:** DB-SCHEMA-01A, AUTH-01, RBAC-01

#### Data/schema changes

- [x] Add `userId` to `CastSession`.
- [x] Add `receiverAddress`, `grantExpiresAt`, and explicit lifecycle state.
- [x] Add receiver identifiers needed by CAST-STATE-01, or reserve nullable fields in the same migration.
- [x] Remove `streamUrl` from public GraphQL output.
- [x] Stop persisting a query-secret-bearing URL. Store a non-secret media path or media file ID only.
- [x] Backfill existing sessions to ended/revoked state; do not attempt to preserve old tokenized URLs.

#### Service/API tasks

- [x] Create session in `STARTING` state before invoking the receiver.
- [x] Mint a signed grant with the claims from section 6.2.
- [x] Add a separate media authorization path for `castGrant`.
- [x] Validate session ownership/state and exact media ID on every request, including HTTP Range requests.
- [x] Set a bounded grant TTL; do not use the user's JWT lifetime.
- [x] Revoke by ending/failing/stopping the session.
- [x] Return a redacted Cast session payload.
- [x] Ensure request tracing logs only URI paths.

#### Tests

- [x] Valid grant streams only its bound media file.
- [x] Wrong media/session/audience/signature, expired, ended, and tampered grants fail.
- [x] A member cannot query or control another member's Cast session.
- [x] No token or grant appears in CastSession GraphQL responses, database fields, app logs, or snapshots.
- [x] Range and reconnect requests work until expiry/session end.

#### Rollout

1. End all active legacy sessions during migration.
2. Add the grant verification path.
3. Switch `castMedia` to scoped grants.
4. Remove access-token query fallback from Cast.
5. Verify on real Chromecast hardware before deleting compatibility code.

### ART-PRIV-01 — Protect artwork delivery

- **Priority:** P1
- **Effort:** M
- **Dependencies:** AUTH-01, SEC-ART-01

#### Tasks

- [x] Decide policy: authenticated library artwork by default; optional public artwork only through an
  explicit library/public-sharing setting.
- [x] Authenticate `/api/artwork/*` with the same server-managed cookie/session used by the frontend.
- [x] If browser origin constraints require it, mint a short-lived artwork grant rather than making the
  route public.
- [x] Apply ownership/library access before serving the object.
- [x] Keep cache validators/range behavior without caching private responses in shared proxies.
- [x] Add `Cache-Control: private` and correct content type/nosniff headers.

#### Acceptance criteria

- An unauthenticated network client cannot enumerate or retrieve private library artwork.
- Authorized image components continue loading without JavaScript-readable bearer tokens.

## 9. Phase 2 — Network policy and complete casting

### HTTP-01 — Shared outbound HTTP client policy

- **Priority:** P1
- **Effort:** L
- **Dependencies:** BASE-01

#### Tasks

- [x] Add a shared outbound-client factory/service with named profiles:
  - metadata;
  - indexer;
  - artwork;
  - LAN device descriptor/control.
- [x] Configure connect, first-byte, idle-body, and total operation deadlines.
- [x] Configure bounded redirects and a project user-agent.
- [x] Define retry classification and jittered backoff.
- [x] Add per-provider concurrency/rate limits.
- [x] Expose sanitized structured telemetry.
- [x] Migrate artwork, OpenLibrary, fingerprint/external metadata, and remaining bare clients.
- [x] Add a source contract test that rejects `reqwest::get` and bare `Client::new()` outside allowed factory
  or test files.

#### Acceptance criteria

- Every external request has an explicit deadline and response-size strategy.
- One slow provider cannot indefinitely hold a GraphQL request or global search.

### CAST-INPUT-01 — Validate and constrain receiver targets

- **Priority:** P0
- **Effort:** M
- **Dependencies:** RBAC-01

#### Tasks

- [x] Validate port as integer `1..=65535` before any `u16` conversion.
- [x] Require finite, non-negative start/seek positions.
- [x] Require finite volume in `0.0..=1.0`; clamp only after validation and persist the applied value.
- [x] Restrict manual device creation/update to admin.
- [x] Allow members to cast only to persisted, enabled devices.
- [x] Validate LAN destination policy for IPv4 private/link-local and IPv6 ULA/link-local.
- [x] Reject loopback, multicast, unspecified, and public destinations by default.
- [x] Resolve approved hostnames and protect against address changes.
- [x] Record discovery origin and admin override explicitly.

#### Tests

- [x] Negative, overflowing, NaN, infinite, and out-of-range values are rejected.
- [x] `-1`/`65536` never wrap into a valid port.
- [x] Members cannot turn stored devices into arbitrary network targets.

### CAST-NET-01 — Real transport deadlines and concurrency control

- **Priority:** P0
- **Effort:** L
- **Dependencies:** CAST-INPUT-01

#### Design spike

- [x] Confirm whether the pinned `rust_cast` version can accept a preconnected/configured socket.
- [x] If not, choose one:
  - contribute/use an upstream timeout-capable API;
  - maintain a minimal pinned fork;
  - replace the transport layer.
- [x] Do not accept `tokio::time::timeout(spawn_blocking(...))` as the final fix; it does not terminate the
  blocked socket operation.

#### Implementation

- [x] Apply OS-level connect/read/write timeouts.
- [x] Add global and per-device semaphores.
- [x] Serialize commands per active receiver session.
- [x] Bound retries and use backoff for transient errors only.
- [x] Mark device/session degraded after repeated failures and surface last error/time.
- [x] Ensure shutdown joins or terminates controller workers.

#### Acceptance criteria

- An unreachable address returns within the configured deadline.
- Repeated unreachable-device requests cannot exhaust the blocking pool.
- Load tests prove normal GraphQL/scan work remains responsive during Cast failures.

### CAST-STATE-01 — Receiver-aware session controller

- **Priority:** P1
- **Effort:** XL
- **Dependencies:** CAST-AUTH-01, CAST-NET-01

#### Data model

- [x] Add to `CastSession`:
  - `userId`;
  - `receiverAppId`;
  - `receiverSessionId`;
  - `transportId`;
  - `mediaSessionId`;
  - `state`;
  - `lastHeartbeatAt`;
  - `lastError`;
  - `endedAt`;
  - `updatedAt`.
- [x] Index active sessions by device and owner.

#### Controller tasks

- [x] Create one controller/actor per active device session.
- [x] Retain exact receiver identifiers returned by launch/load.
- [x] Send play/pause/seek/stop/volume/mute to those identifiers, never “first application”.
- [x] Poll receiver status while active with bounded backoff.
- [x] Reconcile remote controls, disconnects, playback completion, position, duration, volume, and mute.
- [x] End stale sessions and revoke stream grants.
- [x] Publish CastSession subscription updates.
- [x] Make commands idempotent where practical.
- [x] Define conflict behavior when another user starts playback on an occupied device.

#### Frontend tasks

- [x] Consume generated session/subscription types in `useCast`.
- [x] Display connecting, playing, paused, buffering, ended, disconnected, and failed states.
- [x] Show last receiver error with retry/stop actions.
- [x] Stop optimistic local drift when remote controls change state.

#### Acceptance criteria

- Controls cannot target the wrong application/media session.
- Frontend state converges after remote-control actions and disconnect/reconnect.
- A session cannot remain “playing” indefinitely after receiver loss.

### CAST-CFG-01 — Make Cast settings effective

- **Priority:** P1
- **Effort:** L
- **Dependencies:** CAST-STATE-01

#### Tasks

- [x] Load persisted Cast settings before starting discovery.
- [x] Apply settings changes to the running service through an explicit `applySettings` service method.
- [x] Restart/reschedule discovery safely when enabled/interval changes.
- [x] Send default volume and mute state to the receiver after load.
- [x] Honor `transcodeIncompatible` only through CAST-CAP-01.
- [x] Define supported values for `preferredQuality` and validate them.
- [x] Make settings single-row or explicitly per-user; enforce uniqueness.
- [x] Report effective runtime settings, not only persisted values.

#### Acceptance criteria

- Saving each visible setting causes the documented runtime behavior without restarting the backend.
- Invalid settings are rejected and do not partially apply.

### CAST-DISC-01 — Discovery persistence, staleness, uniqueness, and health

- **Priority:** P1
- **Effort:** L
- **Dependencies:** CAST-INPUT-01, CAST-CFG-01

#### Tasks

- [x] Persist automatic and manual discovery through the same entity mutation path.
- [x] Add a unique identity strategy:
  - Google Cast stable device ID when available;
  - protocol + stable USN/UDN for DLNA;
  - address/port only as a fallback.
- [x] Prevent query-then-create races with a schema-level unique constraint.
- [x] Track `firstSeenAt`, `lastSeenAt`, `lastProbeAt`, `lastProbeError`, and effective connectivity.
- [x] Mark stale devices offline; do not immediately delete favorites/manual devices.
- [x] Evict stale non-favorite transient devices after a configurable retention period.
- [x] Make service health reflect last successful scan and scan errors, not only map length.
- [x] Ensure mDNS/SSDP interfaces and IPv6 addresses are represented correctly.

#### Acceptance criteria

- Automatic discovery survives restart.
- Duplicate concurrent discoveries produce one device row.
- `isConnected` and service health are evidence-based.

### CAST-URL-01 — Explicit advertised media URL

- **Priority:** P1
- **Effort:** M
- **Dependencies:** CAST-INPUT-01

#### Tasks

- [x] Add `LIBRARIAN_ADVERTISED_MEDIA_URL` and equivalent persisted/admin-visible configuration.
- [x] Parse with `url::Url`; support bracketed IPv6 and explicit HTTPS.
- [x] Prefer explicit configuration over local-IP guessing.
- [x] If auto-detection remains, choose the route/interface appropriate to the receiver address.
- [x] Add a health diagnostic showing the effective URL and whether it is locally bound/reachable.
- [x] Never log grant query parameters.

#### Acceptance criteria

- Multi-interface and IPv6 deployments generate valid receiver-reachable URLs.
- Misconfiguration fails before launching playback with a clear remediation message.

### CAST-CAP-01 — Device-specific playback compatibility

- **Priority:** P1
- **Effort:** L
- **Dependencies:** CAST-STATE-01, CAST-CFG-01, CAST-URL-01

#### Tasks

- [x] Define a `CastPlaybackDecision`: `DIRECT`, `REMUX`, `TRANSCODE`, or `UNSUPPORTED`.
- [x] Inputs include device/protocol capability profile plus analyzed container/video/audio/HDR data.
- [x] Keep this decision separate from browser Q57.
- [x] Maintain known-model defaults plus conservative unknown-device behavior.
- [x] Route remux/transcode through a Cast-owned HLS/session lifecycle.
- [x] Honor preferred quality and transcode settings.
- [x] Expose the decision/reason in logs and CastSession diagnostics.
- [x] Evaluate capability caching: keep the deterministic, local decision uncached so current persisted
  metadata is always used and no invalidation/staleness path exists (design Q66).

#### Tests

- [x] Table-driven container/codec/device capability matrix.
- [x] Direct play, remux, transcode, and unsupported receiver paths.
- [x] Unknown analysis/device data uses the documented conservative behavior.

### CAST-DLNA-01 — Honest UI first, then UPnP AVTransport

- **Priority:** P1 for truthful UI; P3 for complete protocol
- **Effort:** S containment + XL implementation
- **Dependencies:** CAST-DISC-01, CAST-STATE-01, CAST-CAP-01

#### Immediate containment

- [x] Add `playbackSupported`/`discoveryOnly` capability.
- [x] Stop routing `DLNA_RENDERER` rows into Google Cast V2.
- [x] Disable the Cast action for discovery-only devices with a clear explanation.

#### DLNA playback disposition

- [x] Parse SSDP `LOCATION`, not the UDP source port.
- [x] Do not fetch descriptors or expose SOAP controls while DLNA is discovery-only.
- [x] Remove DLNA playback from supported product scope instead of shipping an incomplete AVTransport
  adapter.
- [x] Preserve the full descriptor/control URL, SOAP action, normalized-error, session-integration, and
  two-renderer qualification requirements as prerequisites for any future DLNA playback proposal.

#### Acceptance criteria

- No DLNA device is falsely shown as playable before support exists.
- Implemented DLNA playback never opens control URLs unrelated to the discovered LAN device.

## 10. Phase 3 — Media pipeline truth, safety, and scale

### PIPE-01 — Durable scan runs and actionable stage outcomes

- **Priority:** P1
- **Effort:** XL
- **Dependencies:** RBAC-01, BASE-01

#### Proposed entities

`LibraryScanRun`:

- `id`, `userId`, `libraryId`;
- `status`, `currentStage`;
- `startedAt`, `finishedAt`;
- discovered, existing, matched, unmatched, provider-blocked, analysis-queued/succeeded/failed,
  organization-succeeded/failed, missing/reconciled counts;
- sanitized summary/error;
- timestamps.

`LibraryScanIssue`:

- `id`, `scanRunId`, `libraryId`, optional `mediaFileId`;
- stage, stable issue code, severity;
- message and remediation;
- resolved state/timestamps.

Both must use entity operations, library/user scoping, indexes, retention, and subscription events.

#### Tasks

- [x] Create a run when the scan is queued and return its ID.
- [x] Thread run ID through match, provider fallback, analysis queue, organization, and reconciliation.
- [x] Replace ad hoc counters with typed stage outcome structs.
- [x] Define clean success, partial success, configuration block, and fatal failure precisely.
- [x] Persist bounded issue details while aggregating repeated identical failures.
- [x] Finalize only after all jobs associated with the run settle or reach a documented timeout.
- [x] Add scan-run subscription and scan-history query.
- [x] Update scan UI with stage/progress/counts and direct issue remediation.
- [x] Retain logs as diagnostics, not the sole user-visible state.
- [x] Add retention/compaction for old runs and resolved per-file issues.

#### Failure behavior

- A file-level failure increments counts and continues when safe.
- A missing/unreadable library root fails closed before missing-file reconciliation.
- Provider configuration failure preserves unmatched files.
- Fatal database/schema failures mark the run failed and clear the library scanning flag.
- Worker panic/cancellation finalizes the run as failed/cancelled through a guard.

#### Acceptance criteria

- Every scan has one terminal durable state.
- “Completed” always matches persisted stage counts.
- The user can identify which files failed, at which stage, and what action is available.

### PIPE-02 — Fail-safe auto-organization

- **Priority:** P0
- **Effort:** M
- **Dependencies:** PIPE-02A; PIPE-01 for durable issue integration

The fail-closed behavior lands in Phase 1 through PIPE-02A. This Phase 3 work completes recovery,
compensation, and durable outcome integration.

#### Tasks

- [x] Change safety-guard errors from fail-open to fail-closed.
- [x] Stop discarding `organize_media_file` results.
- [x] Record organize failure in scan run/issue and keep the file at its safe current location.
- [x] Preserve no-clobber and cross-device guarantees.
- [x] Define transaction/compensation ordering for filesystem and entity update:
  - reserve/check target;
  - perform no-clobber placement;
  - update entity path;
  - compensate or create a high-severity issue if the entity update fails.
- [x] Ensure `relativePath` and original-name fields remain consistent.
- [x] Add recovery for a file moved on disk but not updated in the entity.

#### Tests

- [x] Safety-query failure does not organize.
- [x] Target exists never overwrites.
- [x] Entity update failure is recoverable and reported.
- [x] Cross-filesystem copy/move and seeding-source preservation remain correct.
- [x] Repeated scan is idempotent after partial failure.

### PIPE-03 — Analysis failure visibility and recovery

- **Priority:** P1
- **Effort:** M
- **Dependencies:** PIPE-01

#### Tasks

- [x] Persist ffprobe exit code, bounded stderr, stage, and attempt count as an issue.
- [x] Distinguish missing executable, timeout, malformed JSON, unsupported/corrupt media, and permission error.
- [x] Add per-analysis deadline and process kill/reap behavior.
- [x] Provide “Retry analysis” for a file and failed subset of a scan.
- [x] Aggregate failures in the scan run and notifications.
- [x] Investigate and document the two active Matroska JSON failures.
- [x] Never block filename/provider matching solely because analysis failed.

#### Acceptance criteria

- The current Matroska failures become visible with useful bounded diagnostics.
- Retry succeeds or produces a stable actionable failure without log spam.

### PIPE-DUP-01 — Duplicate physical file workflow

- **Priority:** P2
- **Effort:** M
- **Dependencies:** PIPE-01

#### Tasks

- [x] Detect duplicates using strong file identity when available:
  - same filesystem inode/device;
  - verified content hash;
  - size plus fingerprint only as a candidate, not proof.
- [x] Group duplicate paths under the linked media item.
- [x] Create a review issue/notification; never auto-delete.
- [x] Show path, size, quality, analysis state, and whether a file is a hardlink/copy.
- [x] Offer explicit keep/remove actions with confirmation and no-clobber/trash semantics.
- [x] Reconcile entity links after user-approved removal.
- [ ] Apply the workflow to the four duplicate groups in the active Movies library after TMDB matching.

#### Acceptance criteria

- Duplicate detection cannot mistake different editions/cuts for byte-identical duplicates.
- No duplicate media is deleted without explicit user action.

### STATUS-01 — Complete `ContentStatus`

- **Priority:** P1
- **Effort:** L
- **Dependencies:** PIPE-01

#### Tasks

- [x] Inventory every status consumer and currently unreachable state.
- [x] Define one typed pure status reducer with explicit precedence.
- [x] Inputs include wanted/release timing, pending download, file link, quality status, playback/session
  state, processing/failure state, and explicit ignore state.
- [x] Ensure computed status is not persisted as a competing source of truth.
- [x] Add table-driven tests for every state and precedence collision.
- [x] Update GraphQL computed fields and frontend generated types.
- [x] Remove frontend status reimplementations.
- [x] Add/adjust the corresponding `docs/design.md` Q&A.

#### Acceptance criteria

- Every exposed status is reachable by a documented input combination.
- Backend and frontend cannot disagree on status for the same entity.

### PERF-SCAN-01 — Scanner scalability

- **Priority:** P2
- **Effort:** XL
- **Dependencies:** PIPE-01

#### Tasks

- [x] Move `WalkDir` and blocking metadata calls to a blocking producer with a bounded async channel.
- [x] Add cancellation checks during traversal.
- [x] Page all entity queries; remove fixed 10,000-row truncation assumptions.
- [x] Resolve the system/user authorization context once per scan.
- [x] Batch path lookups and entity reads through supported typed/entity abstractions.
- [x] Use bounded stage concurrency and backpressure.
- [x] Avoid re-running expensive provider/analysis work for unchanged files using size/mtime plus verified
  analysis state.
- [x] Clear/invalidate match caches at mutation boundaries, not only scan end.
- [x] Add metrics for files/sec, GraphQL entity operations/file, queue depth, stage latency, and provider
  latency.
- [x] Add an ignored synthetic 10k/100k-file qualification harness; retain a representative
  slow-network-mount run as an external qualification gate.

#### Acceptance criteria

- A slow filesystem walk does not starve HTTP/GraphQL.
- Libraries above 10,000 files scan without silent omission.
- Cancellation and shutdown leave no stuck scanning flag or orphan run.

## 11. Phase 4 — Schema, supply chain, privacy, and type hygiene

### DB-SCHEMA-01 — Safe schema-change workflow

- **Priority:** P0
- **Effort:** XL
- **Dependencies:** BASE-01

#### Tasks

- [x] Split schema planning from schema application.
- [x] Classify changes as:
  - additive safe;
  - data-transforming;
  - destructive.
- [x] Auto-apply only additive safe changes by default.
- [x] Refuse destructive changes at normal startup with a human-readable plan.
- [x] Require an explicit admin/CLI operation for destructive application.
- [x] Create and verify a backup before destructive application.
- [x] Require migration-specific transformation code for rename/type conversion instead of drop/add.
- [x] Store applied schema version and migration result.
- [x] Make failed migration startup behavior deterministic and recoverable.
- [x] Add restore rehearsal and old-version-to-current fixtures.
- [x] Document the allowed infrastructure exception if schema tooling needs direct database primitives.

#### Acceptance criteria

- Renaming/removing a field cannot silently drop production data on boot.
- Every destructive plan has an identified, restorable backup and explicit authorization.

### SUPPLY-01 — Dependency and CI security gates

- **Priority:** P1
- **Effort:** M
- **Dependencies:** BASE-01

#### Tasks

- [x] Add `cargo audit` to CI with a documented advisory exception process.
- [x] Add `cargo deny` for advisories, licenses, duplicate/high-risk sources, and banned dependencies.
- [x] Add `cargo machete` or equivalent unused-dependency check.
- [x] Add frontend audit policy using the package manager lockfile.
- [x] Pin Git dependencies to reviewed revisions and document ownership/update cadence.
- [x] Track the backend application lockfile and enforce `--locked` in CI/release/build paths.
- [x] Generate an SBOM for release artifacts.
- [x] Add secret scanning and generated-schema drift checks.
- [x] Cache tools in CI without silently skipping missing scanners.

#### Acceptance criteria

- CI fails on unreviewed known vulnerabilities, prohibited licenses, secret leakage, and stale generated
  schema/types.
- Exceptions have owner, reason, expiry, and compensating control.

### FE-TYPES-01 — Finish generated frontend type migration

- **Priority:** P2
- **Effort:** M
- **Dependencies:** RBAC-01, CAST-STATE-01
- **Reference:** `docs/frontend-types-audit.md`

#### Tasks

- [x] Migrate notifications to generated operation result types.
- [x] Migrate casting device/session/settings/result types.
- [x] Migrate remaining schema-mirror media DTOs.
- [x] Keep only genuine UI view models in `frontend/src/lib/graphql/types.ts`.
- [x] Remove dead aliases and re-exports after each domain.
- [x] Add a lint/contract check against hand-written schema DTO duplication where feasible.

#### Acceptance criteria

- Cast and notification UI compile solely against generated schema types plus clearly named view models.
- Running codegen twice produces no diff.

### RUNTIME-01 — Remove recoverable process-abort paths and supervise workers

- **Priority:** P1
- **Effort:** M
- **Dependencies:** BASE-01

#### Tasks

- [x] Inventory production `unwrap`, `expect`, explicit panic, and panic-prone dynamic indexing/parsing.
- [x] Replace HTTP-client construction and zero-duration interval `expect` paths with startup validation and
  `Result`.
- [x] Validate all non-zero interval/config invariants before services start.
- [x] Decide whether release `panic = "abort"` remains acceptable after the inventory; prefer unwind where
  worker supervision/recovery is expected.
- [x] Supervise scan, analysis, logging, discovery, download, and transcode worker exits.
- [x] On worker panic/unexpected exit:
  - log worker/service and current job IDs;
  - clear/finalize durable in-progress state;
  - restart with bounded backoff where safe;
  - degrade service health.
- [x] Never use panic recovery as a substitute for input validation or error handling.

#### Tests

- [x] Invalid rate-limit/client/interval configuration returns a startup error rather than aborting.
- [x] Injected worker panic finalizes its job, degrades health, and does not kill unrelated services.
- [x] Repeated panics stop restarting after the configured bound.

### CLEAN-01 — Remove dead-code and dependency camouflage

- **Priority:** P2
- **Effort:** M
- **Dependencies:** SUPPLY-01, RUNTIME-01

#### Tasks

- [x] Remove crate-wide or broad dead-code allowances; keep narrowly documented exceptions only.
- [x] Delete stale placeholder files/modules after confirming no supported path depends on them.
- [x] Remove unused dependencies identified by the supply-chain checks.
- [x] Replace stale documentation paths and commands with current module locations.
- [x] Classify every remaining placeholder as:
  - implemented in Phase 6;
  - explicitly unsupported and removed from UI/docs;
  - retained behind a feature flag with owner/rationale.
- [x] Keep cleanup commits separate from behavior changes.

#### Acceptance criteria

- Dead-code and unused-dependency checks pass with a small, reviewed exception list.
- Product UI/docs do not promise code paths that only exist as placeholders.

## 12. Phase 5 — Release qualification

No new platform capability should begin until the P0/P1 exit gate below passes.

### Required automated suite

- [x] `cargo fmt --check`.
- [x] `cargo clippy --all-targets` with `-D warnings`.
- [x] `cargo check --all-targets`.
- [x] `cargo test -j 1` in memory-constrained environments.
- [x] Migration tests for every supported transition class (additive, rename/removal, and type change).
- [x] `pnpm exec tsc --noEmit`.
- [x] `pnpm test`.
- [x] Frontend production build.
- [x] GraphQL codegen and no-diff verification.
- [x] No-direct-domain-SQL architecture contract.
- [x] Authorization matrix integration suite.
- [x] Security tests for SSRF, scoped grants, token redaction, and CSRF.
- [x] Credential-key isolation/rotation and public-error redaction tests.
- [x] Worker panic/supervision tests.

### Hardware/manual matrix

- [ ] Chromecast/Google TV direct play.
- [ ] Chromecast remux and transcode.
- [ ] Receiver unreachable/slow/disconnected.
- [ ] Receiver remote-control state changes.
- [ ] Multi-interface server.
- [ ] IPv4 and IPv6 advertised URL.
- [ ] At least two DLNA renderers before declaring DLNA supported.
- [ ] Browser media/artwork with server-managed cookies.
- [x] Two-user isolation.
- [x] Backup, destructive migration rehearsal, and restore.
- [x] Movies scan with missing configuration and fake configured catalog/match data.
- [ ] Active Movies scan with a live valid TMDB key.

### P0/P1 exit gate

- No real user bearer/refresh token appears in Cast URLs, GraphQL payloads, logs, or JavaScript-readable
  storage outside the documented temporary access-token compatibility phase.
- Artwork cannot contact denied destinations or run without deadlines/size caps.
- Cross-user CRUD and subscription isolation passes.
- Cast calls return within transport deadlines and cannot exhaust worker capacity.
- Auto-organize fails closed and never overwrites.
- Destructive schema changes cannot auto-apply.
- Scan outcomes and user-visible status reflect actual stage failures.
- A copied database does not contain the active source-credential decryption key.
- Recoverable configuration/worker failures cannot abort the whole process without a durable failure record.

## 13. Phase 6 — Capability disposition

These substantial product features were not safe to imply through decorative settings or partial
implementations. The remediation closes them by removing them from supported product scope. Reintroducing
one requires a separately approved capability plan and all of the prerequisites preserved below.

### FEATURE-USENET-01 — Real Usenet acquisition

- **Priority:** P3
- **Effort:** XL

#### Closure

- [x] Remove Usenet/NNTP acquisition from navigation and supported-capability claims.
- [x] Retain no decorative server settings as evidence of working acquisition.
- [x] Require any future proposal to cover Newznab → NZB → bounded TLS NNTP → yEnc/checksum/resume →
  PAR2/unpack → the existing authorized import pipeline, with encrypted credentials, durable errors,
  fixtures, and provider health UI.

### FEATURE-SUB-01 — Subtitle discovery, extraction, acquisition, and playback

- **Priority:** P3
- **Effort:** XL

#### Closure

- [x] Remove managed subtitle acquisition/extraction from supported-capability claims.
- [x] Require any future proposal to cover sidecar/embedded/provider sources, authenticated serving,
  WebVTT conversion, bitmap-format honesty, language/accessibility metadata, player selection, bounded
  extraction, diagnostics, and deterministic tests.

### FEATURE-ARCHIVE-01 — Safe archive extraction

- **Priority:** P3
- **Effort:** L

#### Closure

- [x] Remove automated archive extraction from supported-capability claims.
- [x] Require any future proposal to cover explicit tool health, per-job staging, path/symlink/device
  rejection, file-count/byte/time/bomb limits, durable Q41/Q42 failures, authorized import, and
  confirmation-aware cleanup.

### FEATURE-AIRPLAY-01 — Decide and implement an honest AirPlay scope

- **Priority:** P3
- **Effort:** XL

#### Decision gate

- [x] Decide whether the supported target is audio-only RAOP, video AirPlay, or neither.
- [x] Validate available maintained Rust protocol support and licensing/compatibility.
- [x] If reliable support is not feasible, remove AirPlay from product promises and document the decision.

#### Closure

- [x] Record that Safari's media-element control is browser behavior, not a Librarian AirPlay adapter.
- [x] Require a separate protocol identity, scoped grants, target validation, common state, and
  generation-specific device tests before any future first-party AirPlay claim.

### FEATURE-WIN-01 — Windows service and tray integration

- **Priority:** P3
- **Effort:** XL

#### Closure

- [x] Release only a portable Windows server archive and label service/tray/MSI modes unsupported.
- [x] Remove installer and unpinned bundled-FFmpeg behavior from release automation.
- [x] Add Windows compile coverage without representing it as VM/runtime qualification.
- [x] Require service lifecycle, platform directories, graceful stop, firewall, authenticated tray,
  installer upgrade/rollback, and VM tests before any future support claim.

### CAST-DLNA-01 completion

- [x] Keep DLNA `discoveryOnly`; require the full Phase 2 AVTransport scope before moving it to playable.

## 14. Observability requirements

Every workstream must add enough evidence to diagnose failure without exposing secrets.

### Stable fields

- Authentication: user ID, operation, outcome code; never token/cookie values.
- Artwork: entity ID/type, approved host, status class, bytes, duration, failure code.
- Cast: device/session/user/media IDs, protocol, command, state transition, elapsed time, failure code;
  never grant URL/query.
- Scan: run/library/media IDs, stage, counts, elapsed time, issue code.
- Schema: plan ID, from/to version, classification, backup ID, result.

### Metrics/health

- Outbound request latency/error/timeout by provider and operation.
- Active/queued/timed-out Cast operations and controller workers.
- Cast discovery last success/error and online/stale device counts.
- Scan queue depth, stage latency, files/sec, and issue counts.
- Analysis timeout/failure codes.
- Schema version and pending destructive-plan status.

### Retention

- Keep detailed per-file scan issues for a bounded period.
- Aggregate old successful scan runs.
- Never retain bearer tokens, scoped grants, cookies, credential-bearing URLs, or provider secrets in logs.

## 15. Rollout and rollback strategy

### Commit boundaries

Prefer one independently reviewable commit per ID or tightly coupled schema/API pair. Do not combine the
Cast credential migration, state-controller rewrite, and DLNA implementation into one commit.

### Schema changes

1. Backup and verify.
2. Add nullable fields/indexes.
3. Deploy code capable of old and new rows where a compatibility phase is required.
4. Backfill through entity operations.
5. Switch reads/writes.
6. Remove legacy fields only through DB-SCHEMA-01's explicit destructive workflow.

### Security compatibility

- Token migration: accept legacy inputs briefly, but stop producing secrets first.
- Cast migration: end legacy sessions; do not translate/preserve their bearer-token URLs.
- Artwork privacy: deploy cookie/grant-capable frontend before requiring auth on image delivery.

### Rollback rules

- A rollback must not re-enable bearer-token persistence or arbitrary artwork fetching.
- If the new Cast controller fails, disable casting with a health message rather than restoring legacy token
  URLs.
- If an entity migration fails, restore the verified backup and previous binary/schema together.
- Never roll back by deleting newly discovered/unmatched media files.

## 16. Completion definition

The remediation program is complete when:

- Every traceability row is checked complete, explicitly removed from scope by a documented product
  decision, or moved to a dated follow-up with owner and rationale.
- All P0/P1 exit gates pass.
- `docs/design.md` reflects every new media/Cast/auth decision.
- GraphQL schema, generated frontend types, implementation, and authorization tests agree.
- The active Movies library can be rescanned with clear provider readiness, durable outcomes, analysis
  issues, and duplicate review.
- Casting uses scoped grants, owned sessions, bounded transports, receiver-specific state, live settings,
  and honest protocol capabilities.
- Backup/restore and destructive migration are rehearsed.
- The full automated and hardware/manual qualification matrices are recorded with evidence.
