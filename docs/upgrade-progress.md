# Internal dependency upgrade — progress tracker

> Historical record: this document describes the earlier standalone-repository
> upgrade. Current active manifests use the consolidated `graphql-orm`
> monorepo baseline documented in `docs/features/object-storage-and-backup.md`.
> Old repository URLs below are retained only as historical evidence.

Task: bump 4 internal git deps (graphql-orm, agql-auth, graphql-orm-storage,
graphql-orm-backup) in /root/librarian/backend/Cargo.toml to new revs and fix
all resulting compile errors. See task instructions for full detail; this file
is the resumable checklist. If you are a successor agent: read this whole file
before touching anything, then resume at the first unchecked item.

## Rules recap (do not violate)
- Never `cargo run` / `pnpm dev`. Only cargo check / test / clippy.
- Never git commit. Never touch unrelated files (268 other uncommitted files
  in the tree belong to the owner's other work — leave them alone).
- No new direct SQL anywhere (must go through graphql-orm entity layer).
- Minimal diffs — adapt call sites, don't refactor/rename/improve.
- Reference clones at NEW versions: /tmp/orm-check/{graphql-orm,agql-auth,graphql-orm-storage,graphql-orm-backup}

## Checklist

- [x] Create this progress file (step 0)
- [x] STEP 1a: bump graphql-orm rev to de5aa66fa5b80fc0b7fb6a3158b2d7d1c12a35e5
- [x] STEP 1b: bump agql-auth rev to d9bdd501b0e2f47f94b724eeccf3a18b3ca7c2e2
- [x] STEP 1c: bump graphql-orm-storage rev to d60a449e4145659b126ca6a0a3a0a2640004b6df
- [x] STEP 1d: bump graphql-orm-backup rev to 02708fb49637f4d38b92828866bfcc26ed004cdf
- [x] STEP 1e: initial `cargo check`, record error count below

### IMPORTANT dependency-resolution fix (do not remove)
New `graphql-orm-backup` (rev 02708fb4) Cargo.toml depends on
`graphql-orm-storage = { version = "0.3.0", path = "../graphql-orm-storage" }`.
That relative path escapes the graphql-orm-backup git checkout (it's not a
workspace member of that repo), so plain cargo git-dependency resolution fails
with "no matching package named `graphql-orm-storage` found ... location
searched: Git repository .../graphql-orm-backup?rev=...". Fixed by adding to
backend/Cargo.toml:
```
[patch."https://github.com/Dastari/graphql-orm-backup"]
graphql-orm-storage = { git = "https://github.com/Dastari/graphql-orm-storage", rev = "d60a449e4145659b126ca6a0a3a0a2640004b6df" }
```
This redirects the package graphql-orm-backup expects to find at that broken
path to our actual graphql-orm-storage git dependency (same rev as the
backend's own direct dependency), so only one copy of graphql-orm-storage is
built. Verified this resolves cleanly and unifies with the direct dependency.

### Initial cargo check result (after rev bump + patch): 6 errors, all in backend code (none in the 4 crates themselves):
1. src/services/auth.rs:205 — `AuthUserStore`/RefreshTokenStore impl missing `rotate_refresh_token` (new required trait method in agql-auth 0.6)
2. src/services/graphql/mutations/backup.rs:34 — FnOnce not general enough / E0308 lifetime mismatch (likely a closure signature change in graphql-orm-backup or async-graphql context)
3. src/services/backup.rs:145 — non-exhaustive match (new enum variant in graphql-orm-backup)
4. src/services/storage.rs:38 — non-exhaustive match (new enum variant in graphql-orm-storage)

Notably: src/services/database.rs (the huge owner-WIP file, +1604 lines vs HEAD)
compiles as-is against new graphql-orm — `apply_schema_stages`/`SchemaStage::from_entities`
still exist for compat. BUT the task requires reworking its schema bootstrap to
the new validate/plan/apply flow with destructive-migration gating regardless
of whether it compiles today (audit remediation requirement) — doing this as
its own step below.
- [x] STEP 2: fix src/services/database.rs (schema bootstrap rework — details further below)
- [x] STEP 3: fix src/services/auth.rs (agql-auth 0.6 API) — implemented `rotate_refresh_token`
- [x] STEP 4: fix src/services/storage.rs (graphql-orm-storage API) — `#[non_exhaustive]` match arm
- [x] STEP 5: fix src/services/backup.rs (graphql-orm-backup 0.2 API) — `#[non_exhaustive]` match arm
      + verify_snapshot Send/HRTB workaround (see below)
- [x] STEP 6: fix any other call sites cargo check reveals — `cargo check` is CLEAN now
      (only remaining work is the mandatory database.rs schema-bootstrap rework, which
      compiles today via legacy `apply_schema_stages` but must be replaced per task spec)

### Fixes applied so far (details)

1. **src/services/auth.rs** — `RefreshTokenStore` trait gained a required
   `rotate_refresh_token` method in agql-auth 0.6. Implemented it on
   `LibrarianAuthStore`: look up the current token via `RefreshToken::get`,
   return `Ok(false)` if missing/already revoked, otherwise insert the
   replacement then update the current token's `revoked_at` /
   `replaced_by_token_id` / `revocation_reason` (`Rotation`) /
   `last_used_at` / `ip_address` / `user_agent` — mirrors the doc's SQL-shape
   example as closely as possible using only entity-layer calls (no raw SQL;
   not a real DB transaction since the ORM doesn't expose one to app code
   outside mutation hooks, but matches existing code's precedent of using
   entity CRUD without wrapping transactions elsewhere in this file).
2. **src/services/storage.rs** — `graphql_orm_storage::StorageBackend` is now
   `#[non_exhaustive]`; added a wildcard arm to the existing `S3 | AzureBlob`
   match arm (still bails with "not supported yet" — Local behavior
   unchanged).
3. **src/services/backup.rs** — `graphql_orm_backup::BackupKind` is now
   `#[non_exhaustive]`; added `_ => "Unknown"` to the `summary_from_manifest`
   match.
4. **src/services/backup.rs** `verify_snapshot` — `verify_manifest_and_objects`
   (new in 0.2) internally uses `Stream::map(|item| async_fn(borrowed_repo,
   item))` combinators whose opaque `Future` type cannot be proven `Send` in
   a higher-ranked-lifetime context. Calling `.await` on it directly, or via
   `tokio::spawn` (which also needs `Send + 'static`), fails with "implementation
   of `FnOnce` is not general enough" — this is a known rustc limitation with
   async fns invoked from iterator/stream combinators, NOT something fixable
   from our call site by boxing/spawning (tried both, still fails; confirmed
   the graphql-orm-backup crate itself checks clean standalone with no Send
   requirement in play, so this is purely a caller-context Send-proof issue).
   Workaround: run it to completion via `Handle::block_on` inside
   `tokio::task::spawn_blocking` — `block_on` has no `Send` bound on its
   future, so the problematic opaque type never needs the HRTB Send proof.
   Behavior is unchanged (still synchronously verifies and returns the same
   Result), just executed on a blocking-pool thread instead of inline.

`cargo check` is clean (no errors) as of this point — see full log at
/tmp/cargo_check_1.log (pre-fix, 6 errors) for reference; a clean re-run
finished in ~1m30s.
- [x] STEP 2: database.rs schema bootstrap rework — DONE, see below
- [x] STEP 7: `cargo check` clean (confirmed after database.rs rework too)
- [x] STEP 8: `cargo test` all green — 56/56 passed, 0 failed
      (44 unit tests in src/main.rs, including the lifecycle_tests that
      exercise the new bootstrap_schema against in-memory SQLite, + 12 tests
      in tests/backend_migration_contract.rs). Contract test
      `database_startup_uses_entity_metadata_for_schema_reset` was updated to
      assert the new validate/plan/apply symbols instead of the removed
      SchemaStage/apply_schema_stages ones (same intent preserved: startup
      schema management driven from entity metadata, no SQL files, no local
      macros crate).
- [x] STEP 9: `cargo clippy` — 15 warnings, ALL pre-existing and all in files
      not touched by this upgrade (graphql/entities/{chapter,episode,movie,track,torrent}.rs,
      library_scan.rs). Zero clippy warnings in auth.rs, storage.rs, backup.rs,
      database.rs, or the contract test. Log: /tmp/clippy.log
- [x] STEP 10: final report written

## TASK COMPLETE — nothing left to resume.

### STEP 2 details — database.rs schema bootstrap rework

Replaced `entity_schema_stage()`/`entity_schema_stages()` (built `SchemaStage`
0001/0002/0003 objects) and the `pool().apply_schema_stages(...)` call in
`DatabaseService::start` with a new `bootstrap_schema(&Database) -> Result<()>`
function (added just above the old stage functions, same place in the file),
implementing exactly the required flow:

1. `schema.validate_against_entities(&entity_metadata())` — reuses the
   existing `entity_metadata()` list (unchanged, still ~44 entities).
2. If `!validation.has_errors()`: log and return — nothing to do (this is the
   common case on every normal startup once the schema is in sync, so no
   migration history growth in steady state).
3. Otherwise log each diagnostic, then
   `schema.plan_migration_to_entities(version, "sync GraphQL ORM entity schema", &entities)`.
   `version` is `"auto-<rfc3339-now>"` (only generated when actually
   planning/applying, so it's fine that it's not stable across restarts).
4. Partition `plan.steps` by `step.risk == MigrationRisk::Destructive`.
5. If no destructive steps: `schema.apply_migration(&plan, ApplyOptions::default())` —
   applied automatically, preserving prior UX for additive/compatible/risky-but-
   non-destructive changes.
6. If destructive steps exist and `LIBRARIAN_ALLOW_DESTRUCTIVE_MIGRATIONS` env
   var is not exactly `"1"`: log each destructive step (`step.reason` +
   `step.step` debug-formatted) at `warn`, then fail startup via `anyhow::bail!`
   with an explanatory message naming the env var.
7. If destructive steps exist and the env var IS `"1"`: log each at `warn`
   (noting the override was used), then apply with
   `ApplyOptions { allow_destructive: true, ..Default::default() }`.

Only `MigrationRisk::Destructive` is gated — this matches
`graphql-orm`'s own `reject_disallowed_risks` (verified in
crates/graphql-orm/src/graphql/orm/schema_manager.rs): `ApplyOptions::allow_destructive`
only ever gates `Destructive`-risk steps; `Risky` steps are always applied
(that's upstream's classification, not something we're loosening).

`DatabaseService::start()` now calls `bootstrap_schema(self.pool()).await`
instead of `apply_schema_stages`.

Test `lifecycle_tests::entity_schema_declares_provider_unique_indexes` used to
inspect `SchemaStage::from_entities(...).target_schema`; rewrote it to build
`graphql_orm::graphql::orm::SchemaModel::from_entities(&entity_metadata())`
directly (pure in-memory, no DB) and assert the same unique-index expectations
on `.tables`/`.indexes` — same assertions, just against the model type
directly instead of via a stage wrapper. Other lifecycle tests are unchanged
functionally; `fresh_database()` still calls `DatabaseService::new` +
`.start()`, which now runs through `bootstrap_schema` against a fresh
in-memory SQLite DB (validate → drift found since DB is empty → plan → no
destructive steps for a from-scratch DB → auto-applied), so behavior for
tests is equivalent to before.

No changes needed to `src/db/mod.rs` (`Database::new(pool)` already defaults
to `SchemaPolicy::Managed` for SQLite per `crates/graphql-orm/src/db.rs`
`default_schema_policy()`), and no changes needed for pagination config
(backend never called `.unbounded()`; grepped confirmed).

Imports changed: added `ApplyOptions, MigrationRisk` to the
`graphql_orm::graphql::orm::{...}` import list in database.rs; removed
`SchemaStage, SchemaStageRunner` (no longer referenced anywhere in the
backend — confirmed via grep before removing).

## Notes / findings (append as you go)

- backend crate feature `sqlite = ["sqlx/sqlite", "graphql-orm/sqlite"]` — confirmed
  new graphql-orm crate (crates/graphql-orm/Cargo.toml) still has a `sqlite` feature
  (default) that pulls in graphql-orm-macros/sqlite + geo deps. No change needed to
  backend [features] section wiring.
- File sizes before edits: database.rs 1655 lines, auth.rs 532, storage.rs 111,
  backup.rs 305.

## Next step

None — all steps complete. Final verification: cargo check clean, cargo test
56/56 pass, cargo clippy has no new warnings. Do not git commit (task rule).

## Deferred / TODOs for the owner

- graphql-orm-backup 0.2's real full-backup path (`create_full_backup` /
  `create_incremental_backup` / `restore_snapshot`) requires implementing
  database export/import adapter traits. Per task scope these were NOT
  implemented; `BackupService::capabilities()` still reports the conservative
  staged-mode response and `BackupService::create_full_backup` still bails
  with MISSING_BACKUP_RUNTIME_REASON. Listing (`list_snapshots`) and
  verification (`verify_snapshot`) work against the new API.
- The `[patch]` section in backend/Cargo.toml (see above) exists only because
  graphql-orm-backup's published Cargo.toml uses a relative `path` dependency
  that escapes its own repo. If upstream fixes that (e.g. re-publishes with a
  git dependency or merges the crates into one workspace), the patch can be
  removed.
- `verify_snapshot`'s spawn_blocking/block_on workaround can be removed if
  upstream graphql-orm-backup rewrites its verify stream combinators so the
  returned future is provably `Send` (or exposes a Send-friendly entry point).
