# Librarian Remediation Progress Ledger

- **Started:** 2026-07-29
- **Plan of record:** [`project-remediation-plan-2026-07-29.md`](project-remediation-plan-2026-07-29.md)
- **Audit source:** [`audit-2026-07-29.md`](audit-2026-07-29.md)
- **Baseline commit:** `31fa9913b757cf7a77fd005c28af7f9dc6ad924e`
- **Baseline branch:** `main`
- **Execution status:** Local implementation and qualification complete; external gates remain unqualified

This ledger records implementation evidence and exceptions while the plan remains the acceptance checklist
of record. Existing changes in the working tree are user-owned and must not be reset or overwritten.

## Recoverable baseline

At the beginning of remediation, the repository contained 315 modified or untracked status entries. No
baseline commit was created because that would mix and implicitly claim ownership of pre-existing work.

The live SQLite database was copied through SQLite's online backup API so the write-ahead log was included
transactionally:

- Backup:
  `backend/data/backups/librarian.db.remediation-baseline.20260729T143952Z.bak`
- `PRAGMA integrity_check`: `ok`
- `PRAGMA quick_check`: `ok`
- Restored table count: `46`
- Backup size: `34,885,632` bytes

The backup is local recovery material and must not be committed.

The source tree (including all tracked and untracked source files, but excluding `.git`, runtime data,
build outputs, dependencies, and `.env` files) was archived at:

- Archive:
  `backend/data/backups/librarian-source.remediation-baseline.20260729T143952Z.tar.gz`
- SHA-256: `acd9bb2bc0dda942aaa64ac3980e544db0e9c0c9947f6dab607cdc2df7479b64`
- Archive read test: passed
- Archive size: `19,490,228` bytes

Generated GraphQL baseline hashes:

- `frontend/src/lib/graphql/generated/graphql.ts`:
  `64bed8dd3435deb83dc11066a174b77312c90c85e3cf673b155bffbf14fed768`
- `frontend/src/lib/graphql/generated/schema.json`:
  `be04795909c36c2fe757d8b7e10ab151114c073cafe87acd14bc8202a17f96f7`
- `frontend/src/lib/graphql/generated/types.ts`:
  `ed76e666f77234899a38475a373dc35d5d75dbfa466123a3c206fa6e8c4522c9`

## Baseline verification

The audited tree passed these checks before the new remediation phases began:

- `cargo check`
- `cargo test -j1`: 147 backend unit tests and 12 backend integration tests
- `pnpm test`: 19 frontend tests
- `pnpm exec tsc --noEmit`
- Production `cargo clippy -D warnings` baseline: 22 warnings promoted to errors. Five originate in
  `GraphQLRelations` generated code; the remainder are local style/complexity warnings tracked by
  `CLEAN-01`.

## Phase status

| Phase | Scope | Status |
|---|---|---|
| 0 | Baseline, movie readiness, log layout, MIME handling, schema containment | Complete locally |
| 1 | Security boundaries, authorization, credential isolation, organization containment | Complete locally |
| 2 | Casting transport, input, state, discovery, settings, URLs, compatibility, DLNA | Complete locally; hardware unqualified; DLNA remains discovery-only |
| 3 | Durable pipeline outcomes, analysis feedback, duplicates, status, scale | Complete locally; live-library cleanup remains user-controlled |
| 4 | Schema workflow, dependency policy, generated types, runtime cleanup | Complete locally |
| 5 | Qualification and release gates | Locally executable gates complete; CI-only and external matrix gates identified |
| 6 | Usenet, subtitles, archives, AirPlay decision, Windows integration | Closed by explicit removal from supported scope |

## Implementation log

### 2026-07-29

- Re-audited the current repository and documented the findings.
- Added explicit TMDB readiness feedback and scan-summary behavior for movie libraries.
- Corrected system-log column sizing so metadata columns remain compact and the message column consumes
  the remaining width.
- Corrected Cast MIME normalization so generic media kinds are not emitted as invalid MIME values.
- Captured and restore-checked the live database baseline.
- Added an explicit schema workflow:
  - ordinary startup applies additive plans only;
  - type conversions and rename-as-drop/add are destructive;
  - reviewed application requires the stable plan hash and a checksum-verified full backup matching the
    source schema;
  - the environment-variable destructive bypass was removed;
  - rename/removal and type-conversion regression tests were added.
- Made scan-time auto-organization fail closed:
  - safety-guard errors cannot trigger a move;
  - unsuccessful/error outcomes are no longer discarded;
  - one unresolved scoped notification is emitted for each failure kind;
  - the existing atomic no-clobber behavior remains intact.
- Added design decisions Q60 and Q61 and the operator guide `docs/schema-migrations.md`.
- Verification after containment:
  - `cargo check`: passed;
  - `cargo check --tests -j1`: passed;
  - targeted test execution is deferred to the next serialized full-suite run because linking the current
    backend test binary used approximately 6.5 GiB in this environment.

## Verification exceptions

- External Cast hardware qualification, provider-backed downloads, and Windows service/tray qualification
  require their corresponding environments. Windows service/tray, DLNA playback, Usenet/NNTP, subtitle
  acquisition, archive extraction, and first-party AirPlay were removed from the supported product scope,
  so they are not represented as partially working capabilities.
- `cargo audit`, `cargo deny`, and `cargo machete` were installed into an isolated temporary tool root
  and executed locally. Gitleaks, Windows compilation, and release SBOM generation remain CI/runner
  gates and are not claimed as locally executed.

## Completed implementation

### Scan incident and media pipeline

- A missing TMDB key now produces one durable `TMDB_NOT_CONFIGURED` scan issue, one deduplicated
  `ACTION_REQUIRED` notification, `COMPLETED_WITH_ISSUES`, and a direct Metadata-settings remediation
  path. Files remain visible and unmatched.
- Metadata settings can test a TMDB key before it is saved, and scan confirmation displays provider
  readiness.
- Scan runs, stages, counters, bounded issue details, subscriptions/history, retry-analysis,
  resolve-issue, and retention/compaction are persisted through the entity layer.
- ffprobe has a bounded deadline, kill/reap handling, stable failure categories, bounded stderr, attempt
  counts, retry controls, and scan aggregation.
- Auto-organization fails closed, never overwrites, compensates a failed entity update by moving the file
  back when possible, and records actionable failures.
- Byte-identical duplicates are reported without deletion. The UI shows both paths and offers an explicit,
  re-hashed, confirmed move into recoverable `.librarian-trash`. Review details include exact size,
  analysis/quality state, and a cross-platform copy-versus-hardlink identity check.
- Scanner traversal runs in a blocking producer behind a bounded channel with cancellation. Top-level and
  child entity reads page to exhaustion; stage concurrency and queues are bounded; scan queue depth,
  analysis queue depth, elapsed time, files/second, GraphQL entity operations/file, stage latency, and
  provider latency are structured telemetry.
- Size plus persisted filesystem modification time now form the scan version marker. Unchanged,
  already-linked files reuse matching and successful analysis; changed files clear stale ffprobe/quality
  fields and are requeued, while unmatched files remain retryable after provider/catalog changes.
- An ignored 10k/100k synthetic traversal qualification harness is available as
  `synthetic_large_tree_discovery`.
- `ContentStatus` is now a pure, precedence-defined reducer covering every advertised state; frontend
  consumers use the generated GraphQL result.
- Search-to-library torrent creation now retains the newly created movie/show target ID.

### Security and authorization

- Access and refresh tokens are backend-set HttpOnly cookies. Refresh values were removed from GraphQL
  payloads and refresh inputs; logout revokes server state and clears both cookies.
- Cookie-authenticated GraphQL mutations require an allowed origin. Forwarded scheme data is accepted only
  from configured trusted IP/CIDR proxies.
- Artwork fetching is admin-controlled, authenticated on read, destination restricted, redirect
  revalidated, deadline/body capped, MIME checked, and decode verified.
- Source credentials use an external exact 32-byte key and versioned envelope. CLI plan/rotation requires a
  verified full backup and provides verification and rollback; database-resident legacy material is only a
  migration source.
- Public errors are code-based and sanitized while internal structured logs retain correlation context.
- The authorization matrix is machine-readable. Generated and custom operations enforce admin, member,
  owner/library, derived-read-only, field, relation, subscription, and Cast-session boundaries.
- The behavioral two-member/admin fixture proves row isolation, ownership-transfer denial, and explicit
  admin override.
- Cast receivers receive a short-lived session/media/receiver-bound grant, never a user bearer token. The
  secret URL is neither persisted nor returned.

### Casting and outbound networking

- All outbound backend response bodies use bounded readers. Shared client profiles set user agent,
  redirects, connect/request deadlines, response limits, transient-only retries, jitter, rate limits, and
  sanitized target telemetry.
- The pinned `rust_cast` 0.21 source is vendored with a narrow transport patch because upstream did not
  expose socket deadlines. Every Cast connection now has OS connect/read/write timeouts, an overall command
  deadline, global concurrency bounds, and per-device serialization.
- Cast inputs reject unsafe ports, public/loopback/multicast/unspecified targets, non-finite positions, and
  invalid volume.
- Exact receiver transport/session/media identifiers are retained and used for control; a bounded monitor
  reconciles remote status and revokes stale sessions.
- Persisted discovery settings reconfigure the live loop. Automatic discovery uses the same entity upsert
  path as manual discovery, `(address, port)` is unique, staleness is reflected in connectivity, and health
  includes last success/error. Stale transient network discoveries are pruned after the validated
  `CAST_DISCOVERY_RETENTION_DAYS`; manual and favorite devices are preserved.
- The advertised media origin is explicit and validated. Direct/remux/transcode/unsupported decisions are
  Cast-specific and diagnostic.
- SSDP uses the validated `LOCATION` endpoint rather than the response source port. DLNA rows are
  `playbackSupported=false`; no Google Cast command is sent to them.

### Schema, runtime, cleanup, and delivery

- Startup plans schema changes and auto-applies additive changes only. Destructive changes require an
  explicit plan hash and checksum-verified full backup; rename/removal and type-conversion regression tests
  prove startup cannot silently destroy data.
- Background scan, analysis, logging, Cast discovery, torrent, auto-download, and transcode workers expose
  unexpected exits. The library workers catch panics and restart three times; the manager restarts other
  failed background services with bounded backoff and a three-attempt budget.
- Release panic policy is `unwind`; recoverable configuration paths validate before spawning workers.
- Unused TVMaze schedule APIs, fake Match actions, placeholder media query hints, broad dead-code
  suppressions, unused TUI setters, and stale defaults were removed.
- Frontend casting/notification/media consumers use generated GraphQL operation types. Schema export and
  codegen are deterministic.
- CI now runs format, check, strict Clippy, tests, TypeScript, frontend tests/build, schema-drift,
  authorization/security contracts, Rust advisory/license/unused-dependency tools, production pnpm audit,
  secret scanning, Windows compile, and SBOM generation.
- Actions and Git dependencies are immutable-revision pinned; Dependabot tracks Actions, Cargo, and npm.
- The frontend lockfile constrains vulnerable transitive dependencies to patched versions. Both the full
  and production-only pnpm audits report no known vulnerabilities.
- Fixable Rust advisories were removed by upgrading `crossbeam-epoch`, `quinn-proto`, `quick-xml`,
  `anyhow`, `memmap2`, `ratatui`, and `lru`. `librqbit` moved to 9.0.0-rc.0 and its UPnP package is
  pinned to the upstream commit containing the `quick-xml` fix. The direct unmaintained `backoff`
  dependency and five unused direct dependencies were removed.
- The backend application lockfile is now tracked. CI, release, Docker, distro, and Makefile build/test
  paths use `--locked`; the Docker build copies the vendored Cast dependency, no longer references
  deleted SQL migrations, and uses a Rust image compatible with the current crate MSRV.
- The exception policy is documented in `docs/dependency-advisory-exceptions.md`. Its sole active
  exception is the no-fix RSA timing advisory linked by `agql-auth` for RS256/OIDC paths; Librarian
  configures HS256 only, so the affected private-key operation is unreachable. The exception expires
  2026-10-30.
- Release automation publishes Docker, Linux, and a portable Windows **server** archive. Unsupported
  Windows service/tray/MSI claims and the unpinned FFmpeg bundle download were removed.

## Final local qualification — 2026-07-30

- `cargo fmt --all -- --check`: passed.
- `cargo check --locked --all-targets -j1`: passed against the tracked backend lockfile.
- `cargo clippy --all-targets -j1 -- -D warnings`: passed.
- `cargo test -j1`: 192 unit tests passed, one ignored 10k/100k manual qualification harness, and all
  15 integration/architecture contracts passed.
- `cargo audit`: passed after upgrading every fixable vulnerability; the single no-fix RSA advisory is
  governed by the scoped, expiring exception above. Seven informational/yanked transitive warnings
  remain visible.
- `cargo deny check`: passed advisories, bans, licenses, and sources. Duplicate-version and one yanked
  transitive-crate finding remain warnings rather than hidden exceptions.
- `cargo machete`: passed with no unused direct dependencies after removing five findings.
- GraphQL schema export: passed; a second codegen pass produced identical hashes:
  - `graphql.ts`: `538ff4aa8783d86b6665832400fce379c143845ae71bef2fbe4ee326a3d500f6`;
  - `types.ts`: `1a960ff64568f7e0e6f9bb54c4321ba502b8e14db823ef61b6a34286dcb9d5b3`;
  - `schema.json`: `c7516ae195c53079d647a2f8fcbf2dad20d69ee694be171bc6cecb8a28ede601`.
- Frontend production build and both TypeScript passes: passed; the removed Usenet settings route is
  absent from the route tree and production chunks.
- Vitest: 8 files and 21 tests passed.
- Full and production-only pnpm audits: no known vulnerabilities.
- CI/release/Dependabot YAML parsed successfully, workflow actions are immutable-revision pinned, and
  `git diff --check` passed.
- `docker build --check .`: passed with no Dockerfile warnings.

## External qualification ledger

These are deliberately **not** claimed as complete:

| Gate | State | Required evidence |
|---|---|---|
| Active Movies library with valid TMDB key | Unqualified | Add/test a real key, rescan, confirm catalog creation, then review duplicate groups |
| Real provider/CDN artwork | Unqualified | Cache allowed artwork with live provider credentials and verify private browser delivery |
| Chromecast/Google TV | Unqualified | Direct, remux, transcode, remote-control, disconnect, and slow/unreachable receiver matrix |
| Multi-interface/IPv6 advertised URL | Unqualified | Receiver-reachable IPv4 and bracketed IPv6 environments |
| Slow network mount and 100k scan | Harness available | Run the documented synthetic fixture and a representative remote filesystem |
| Current duplicate physical files | Awaiting user action | Review each group and explicitly choose whether to move a copy to trash |
| Windows portable server | Compile gate only | VM launch, config/data paths, firewall, graceful shutdown, and upgrade rehearsal |
| Live logs-table visual smoke | Build/adapter tests passed | Attach the user-owned dev server and verify fixed Time/Level/Source columns plus flexible Message at representative widths |
| Secret/SBOM CI tools | Configured, not local | Run gitleaks and release SBOM generation on CI; Rust supply-chain tools passed locally |

DLNA playback, Usenet/NNTP, managed subtitles, archive extraction, first-party AirPlay, and Windows
service/tray installation are unsupported. They require a new approved capability plan rather than being
silently reclassified as qualified remediation.
