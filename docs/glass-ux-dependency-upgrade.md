# ORM/auth preparation for Glass UX adoption

2026-10-23. This change updates dependencies and their consumer contracts; it does
not deploy the backend or perform the live database migration.

## Selected versions

| Dependency | Previous | Selected |
| --- | --- | --- |
| graphql-orm | 0.30.0 | 0.57.0 |
| graphql-orm-storage | 0.6.2 | 0.7.5, local feature only |
| graphql-orm-backup | 0.7.2 | 0.10.0, local feature only |
| agql-auth | 0.19.0 | 0.22.1 |
| glass-ux | absent | 0.9.10 release archive |
| @tanstack/react-router | 1.170.32 | 1.170.43, aligned with Glass UX |
| zod | 4.5.4 | 4.6.7, one compatible copy for host/library |

All ORM companions use immutable workspace revision
`a90b5bd41f952b02416f23999c729b9ceffcbb8d` (workspace tag
`workspace-2026.10.22.1`). Auth uses immutable revision
`86fabec09eea1eec0f675bc150e90e3809734567` (`v0.22.1`). The backend lockfile
resolves one auth version and one coherent ORM family. SQLx remains 0.8.6;
the existing async-graphql security patch remains. Router is pinned to the library's
exact dependency version to avoid duplicate runtime contexts. Zod is deduplicated
within the library's supported range; form schemas remain host-owned.

Primary upstream references:
[ORM changelog](https://github.com/Dastari/graphql-orm/blob/a90b5bd41f952b02416f23999c729b9ceffcbb8d/CHANGELOG.md),
[ORM migration guide](https://github.com/Dastari/graphql-orm/blob/a90b5bd41f952b02416f23999c729b9ceffcbb8d/MIGRATION.md),
[auth migration guide](https://github.com/Dastari/agql-auth/blob/v0.22.1/MIGRATION.md),
[backup changelog](https://github.com/Dastari/graphql-orm/blob/a90b5bd41f952b02416f23999c729b9ceffcbb8d/crates/graphql-orm-backup/CHANGELOG.md).

## Required application adaptations

### Refresh authority and stored sessions

Auth 0.20+ persists grant provenance and monotonically narrowing role ceilings.
RefreshToken now has three **private nullable TEXT columns**: `grant_source`,
`roles`, and `authorization_roles`, serialized through the existing generated
entity layer. They are not public query/mutation fields. Storage reads/writes
preserve all three on issuance and rotation. Malformed non-null authority JSON
fails instead of being guessed.

Old records without provenance deserialize to `Legacy` and **must sign in again**.
There is no bulk backfill marking them as password grants. Existing short-lived
access cookies retain their normal expiry; old refresh credentials cannot renew.

Registration previously used host-verified issuance with an AuthMethod label.
That now creates Host provenance, which requires a host authority callback on
refresh. Registration instead logs in against the newly persisted password user,
so both setup/registration and ordinary password login issue StoredUser grants.
No new provider/MFA/challenge flow was added. Library and product authorization
remain host-owned.

The persisted-session regression exercises registration refresh, new grant JSON,
rotation/replay, expiry/logout, missing legacy provenance, deliberately narrowed persisted roles surviving
subsequent rotation, role-change revocation, and transient store failures. Existing API
tests cover cookie paths/flags, registration refresh, membership and authorization.

### Authorization-aware list bounds

ORM 0.46+ requires an explicit finite AuthorizedScanConfig when row visibility
uses residual callbacks. Librarian's existing OwnershipRowPolicy is such a policy.
DatabaseService installs a 256-row batch and **100,000 candidate-row ceiling**.
The key is random per process; restarting invalidates scan continuation tokens.
No new secret/environment variable is required.

Exact visible list totals/offsets now fail closed if the candidate set cannot be
exhausted within that ceiling; no partial total is presented as exact. A matching
page can be small while the underlying candidate set is larger. This is a real
behavior/scale limit, not an execution-time or RSS guarantee. The UI plan includes
explicit server-error handling. A later optimization can implement complete
typed SQL visibility predicates in the entity layer for direct/derived ownership;
do not bypass the policy with raw SQL or silently remove the bound.

### Subscription payloads and deletes

ORM 0.45+ authorizes every event release and rejects payload selections that
contain computed/object-valued fields it cannot safely release. Generated web
subscriptions now select only invalidation metadata. Authorized ordinary queries
remain responsible for full entity data. The scan hook refetches its library-scoped
query for a delivered scan invalidation. The generator, generated documents,
client types and a regression contract are updated together.

The new ORM defaults deleted-preimage release to deny. OwnershipRowPolicy opts
in only for an actual GraphQL subscription context and rechecks the application's
ownership decision against the trusted generated preimage. This SQLite app has
no separate database-session/RLS restriction; derived ownership still requires
the current parent to exist. ORM entity/field checks remain independent. Raw
host event injection is not promoted into public subscriptions. The API stream
contract now covers a generated delete in addition to create/update.

### Storage and backup

Only the local provider is enabled. Storage 0.7 hardens conditional create and
single-attempt deletion. Backup 0.8 changes repository lock behavior; no automatic
age-based takeover should be assumed. Backup 0.9 wraps restore failures with stage
progress; a partly applied stage is not a resumable/atomic rollback checkpoint.

Backup 0.10 defaults to finite limits: 64 MiB per compatibility buffered body,
256 MiB decoded database payload and 512 MiB aggregate decoded bytes per stage.
This app's buffered full-table backup adapter is still buffered; those limits do
not bound every allocation. Large legitimate backups need a reviewed finite
configuration/streaming follow-up rather than an implicit unbounded override.
Snapshot wire-format stability does not prove old manifests are restorable into
the new application schema hash.

## Database/deployment rehearsal

1. Keep the current live process/database untouched while reviewing this change.
2. Build the updated backend on a runner with enough RAM. Run the auth/entity/
   ownership/backup integration suites and the full unit suite.
3. Export the new schema using the existing `EXPORT_GRAPHQL_SCHEMA` test hook,
   compare SDL, regenerate the web snapshot/clients if changed, and validate all
   documents. Private columns should not become GraphQL fields. New entity
   features/descriptions may still change the exported SDL.
4. On a disposable consistent copy, inspect `schema plan`. Expect the three
   nullable private refresh-token columns. Review every other upstream schema
   step; do not assume a hash change is automatically benign.
5. Rehearse the project's guarded schema apply using a verified backup snapshot
   and the exact plan hash. Start the new services only after the required schema
   is applied. Do not point the old binary at a half-migrated auth store.
6. Verify new login/registration/refresh/replay/logout, legacy-session re-login,
   member ownership filters, generated create/update/delete subscriptions,
   artwork storage, backup creation and restore into a fresh staging target.
7. Deploy only after that rehearsal. Communicate the one-time re-login. Rollback
   includes a consistent database/objects/backup state and the previous binary;
   changing dependency pins alone is not a database rollback.

No live schema apply, restore, key rotation or production server restart was run.
No application-domain SQL was introduced. The existing source credential key and
JWT key configuration are unchanged.

## Local validation record

All build/test invocations use `/sys/fs/cgroup/librarian-builds` with aggregate
`cpu.max = 320000 100000` on four available CPUs. Live/application servers stay
outside that group.

- Release runtime/docs checksums: passed; all six installed skills match the
  release package; committed shared rule copies match it byte-for-byte.
- Frontend final full suite: **266 passed, 3 skipped** across 36 files, including
  the new generated-subscription invalidation contract.
- Frontend production build: passed, including generation, build typecheck,
  bundling and PWA service-worker generation (118 precache entries). No global
  Glass styles are imported yet; library bundle/performance qualification belongs
  to the foundation migration where its components enter the runtime.
- TypeScript: passes after generation of metadata-only subscriptions.
- Frontend checks were rerun after Router/Zod alignment with the same 266-pass,
  3-skipped result and a passing production/PWA build. Module resolution confirms
  the host and library share Router, React and Zod instances.
- Rust formatting and direct-SQL source scan: pass.
- Full Rust unit-test code generation exceeded this 8 GiB machine's RAM and was
  killed by the kernel. Retried without debug info/incremental compilation, with
  one Cargo job and up to 1024 codegen units/one LLVM thread; still memory-limited.
  `cargo check --tests` also exceeded memory while checking the library test target.
  These attempts are **not reported as passing tests**.
- `cargo check --locked --lib -j 1`: **passed**, with debug info/incremental
  compilation disabled for this machine. Runtime/schema export/upgrade/restore
  qualification remains a deployment gate if the Rust executables cannot be
  built locally. Focused API suite results are recorded in the final addendum.

### Runtime-test addendum

The focused `api_auth`, `api_entities` and `backend_migration_contract` attempt
also exhausted RAM compiling the non-test library before any test executed.
A final diagnostic attempt with a single frontend/compiler thread and serialized
LLVM backend (`RUSTC_BOOTSTRAP=1` for local scheduling diagnostics only) failed
the same way. The compiler reported 6236 MB RSS during MIR borrow checking, then
was killed before producing an executable. No unstable compiler settings were
added to the repository or recommended for deployment.

**Neither the Rust unit suite nor the focused API suites passed locally.** The
stable `cargo check --locked --lib` did pass. Run the normal test suites, export
the actual new SDL, and rehearse the guarded schema/backup upgrade on a larger
runner before deploying these backend changes. The existing debug binary was
not replaced or restarted against the live database.
