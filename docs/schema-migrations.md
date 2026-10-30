# Schema Migration Safety

Librarian treats Rust entity metadata as the target schema, but application startup is not permission to
transform or discard existing data.

## Startup policy

On every ordinary start, Librarian:

1. validates the live database against entity metadata;
2. builds the complete migration plan before executing a statement;
3. classifies every step;
4. automatically applies the plan only when every step is additive.

Additive changes include new tables, nullable/defaulted columns, and new indexes. Required columns without
defaults, constraint changes, index removal, type conversions, column removal, and table removal do not run
at startup. Any SQL type conversion is treated as destructive until migration-specific transformation code
proves how values are preserved. A field rename without `db_column` compatibility metadata plans as
add-plus-drop and is therefore destructive.

When startup blocks a plan, its error contains a sanitized step list, source and target schema hashes, the
stable plan hash, and the explicit command shape. Migration SQL is not printed.

`LIBRARIAN_ALLOW_DESTRUCTIVE_MIGRATIONS` is intentionally unsupported. There is no environment-variable
bypass.

## Explicit non-additive workflow

Run these steps with the current/old application version still available:

1. Create a full logical backup from **Settings → Backup** or the admin backup mutation, or offline
   with the current binary while Librarian is stopped (`librarian backup create` takes and verifies
   one in `BACKUP_PATH`). The backup must come from a build whose entities match the live schema;
   the new binary refuses to back up a schema it does not describe.
2. Verify that snapshot and record its snapshot ID.
3. Stop Librarian so no request or worker can mutate the database during migration.
4. Install the new binary and print its proposed plan:

   ```text
   librarian schema plan
   ```

5. Review the classification and every listed table/column operation. For a rename or conversion, add and
   test migration-specific transformation support instead of approving an unintended drop/add.
6. Apply exactly the reviewed plan:

   ```text
   librarian schema apply \
     --plan-hash <PLAN_HASH> \
     --backup-snapshot <SNAPSHOT_ID>
   ```

The apply command fails closed unless:

- the plan hash still matches the current live-to-target plan;
- the snapshot is a full or synthetic-full Librarian snapshot;
- its manifest and every referenced table/object payload pass checksum verification;
- its database backend is SQLite;
- its schema hash exactly matches the plan's live source schema.

The ORM also rechecks the source schema hash immediately before application and records a successful
migration in its migration history. Re-run `librarian schema plan` after application; it must report no
pending plan before ordinary startup.

`schema plan` and `schema apply` always diff the full introspected schema. Startup validation does not
report every constraint difference (a missing `UNIQUE`, for one) and lets such a database start, so a
constraint-only change is visible and applicable only through these commands.

### Offline maintenance commands

Both commands connect directly to the configured database; stop the server first. Each one first applies a
pending plan if it is purely additive, exactly as startup would.

- `librarian backup create` takes a full backup and verifies its checksums, printing the snapshot ID. With a
  non-additive plan pending it backs up the schema as it is, which is what `schema apply` needs.
- `librarian repair duplicate-torrents [--apply]` merges torrent rows that share an info hash, which a
  `UNIQUE` info-hash constraint cannot be added over. It keeps the row furthest along (finished, then
  progress, then most files, then oldest), copies post-processing results, library/title links, file
  matches and unlisted files onto it, and deletes the rest through the entity layer so their delete hooks
  run. Without `--apply` it only reports. It refuses while a non-additive plan is pending, and skips hashes
  shared by different users.

The 2026-10-30 live upgrade (seasons, episode stills, personal tables and the torrent info-hash constraint)
used these steps: the build without the constraint applied its additive plan, repaired seven phantom
duplicate torrents and took snapshot `63395272-3e0b-44e9-b265-b82715e5c2e8`; the final build then applied the
reviewed two-step info-hash plan with that snapshot.

## Recovery

If application fails, do not start the new binary repeatedly. Preserve its error and:

1. use a qualified recovery build matching the snapshot's schema;
2. restore database rows and managed object bytes together into new offline targets:

   ```text
   librarian backup restore --snapshot <SNAPSHOT_ID> --database <NEW_DATABASE_FILE> --objects <NEW_OBJECT_DIRECTORY>
   ```

3. verify the restored snapshot and application health;
4. investigate or correct the migration-specific transformation before retrying.

Restore is deliberately empty-database-only. Never overlay a logical snapshot onto a partially migrated
database.

The recovery command refuses existing targets and verifies restored object bytes
through application storage reads before success. It starts no application
services. A failed attempt leaves staging in place for inspection; discard those
targets before retrying. Publish the database and object root together only
after successful verification. The live GraphQL service advertises restore as
unavailable and rejects attempts to replace its active database.

Schema compatibility is enforced; this command is not a cross-version schema
converter. If the previous release has no qualified offline recovery tool, build
and rehearse one against that exact schema before approving a migration. An
export-only build cannot perform or qualify recovery.

## Infrastructure exception

Schema planning/application and logical backup internals use the typed primitives supplied by
`graphql-orm` and `graphql-orm-backup`. This is infrastructure plumbing, not application-domain CRUD.
Services, resolvers, and jobs must continue to use generated entity queries and mutations and must not add
ad hoc SQL.

## Isolated runtime recovery rehearsal

The parent-owned serial compile lane can run the synthetic rehearsal without starting a server:

```bash
scripts/rehearsal-recovery.sh \
  /sys/fs/cgroup/<existing-build-group> \
  /tmp/librarian-recovery-<new-run-id>
```

The second argument must be an absolute, nonexistent directory. The build group must already exist,
be writable, and cap the aggregate compiler/linker CPU usage at 80% of the available CPUs or less.
For the four-CPU qualification VM, the parent provisions `cpu.max=320000 100000`.
The original 8 GiB VM could not complete compilation with a 6–7 GiB build limit.
After expansion to 16 GiB, the normal rehearsal passed with a 14 GiB `memory.max`,
leaving roughly 2 GiB for the host and orchestrator. The script records the configured limits.
Run no concurrent builds. Dependency artifacts default to `backend/target`; set
`REHEARSAL_TARGET_DIR` to an absolute cache path to choose another parent-owned target directory.
Both variants build with `--locked --offline --no-default-features --features sqlite -j 1`, using
the copied lockfile and its pinned ORM/backup/storage revision. An absent cached dependency is a
prerequisite failure, not permission to update the lockfile.

The script snapshots the current dirty source and local vendor patches, records input SHA-256s,
and adds two harness bridges **only in that isolated source copy**. It never copies `.env`, live
data, downloads or torrent state, never deletes a target, and never edits the working tree's
`lib.rs`, `database.rs` or `backup.rs`. Generated typed inserts create the fixture, including
legacy summary drift; production hooks and the actual reconciliation function perform the repair.
The real offline `run_backup_restore` and `run_schema_command` implementations perform recovery and
reviewed migration. No merged GraphQL schema, HTTP listener, service startup or worker is needed.
Public GraphQL authentication/dispatch remains a separate qualification gate.

`scripts/rehearsal-rustc.py` applies `-Zhint-mostly-unused=yes` only to the `librarian` library and
serializes its codegen, using `RUSTC_BOOTSTRAP=1`. This defers unused library codegen without replacing
runtime bodies. `schema-export-only` is disabled and explicitly rejected. The existing SDL exporter
and its executable cannot qualify this rehearsal. The normal-runtime strategy passed the
16 GiB VM rehearsal recorded below; each subsequent run must still report an OOM or compiler
error with its retained build log rather than claiming qualification.

The command performs these checks, each in a new process:

1. Create a full fixture schema from real entity metadata. The synthetic source build maps
   `AppSetting.description` to `rehearsal_description_v1`, allowing an intentional destructive
   drop/add rehearsal without inventing domain tables or using application SQL.
2. Inspect stale Artist/Album/Audiobook summaries with the real dry-run reconciler; compare all
   fixture rows to prove no writes. Repair, check file-byte deduplication and explicit Artist
   membership/duration, preserve provider counts, then prove repeated repair is a no-op.
3. Create and checksum-verify a real full backup containing a PNG linked through `ArtworkCache`,
   a binary managed object (including non-text bytes), and an empty managed object.
4. Qualify the **source-schema executable first**: restore through the actual offline CLI
   implementation into new database/object paths, compare every fixture row, and compare all
   three objects byte-for-byte through both application storage reads and filesystem reads.
5. Inject a deterministic failure on the second object write, delegating the other operations
   to the real pinned local provider. Require actual imported rows, exactly one written object,
   and the persistent in-progress marker. A new process must refuse that retained target through
   both CLI path claims and the recovery service; retries must leave its rows and bytes unchanged.
6. Build the candidate from the same source snapshot with only the synthetic mapping removed.
   Record the real sanitized schema plan and reviewed hash, require destructive classification,
   and prove ordinary bootstrap rejects it without changing the schema.
7. Invoke the real reviewed apply implementation with the recorded plan hash and full snapshot.
   Never fabricate a verified-backup proof or rewrite a manifest/hash to pass the gate. On success,
   require no pending plan, demonstrate the fixture description was lost, and write a candidate
   sentinel through a generated mutation.
8. Require candidate recovery of the source-schema snapshot to fail before importing rows or
   objects. Restore again to new rollback paths with the qualified source-schema executable;
   compare all original rows, the pre-candidate description/sentinel and every managed/artwork byte.

Evidence remains under the run directory: `input-sha256.json`, `staging-contract.json`,
`binary-sha256.txt`, `resources.txt`, phase/build logs, `state/migration-plan.txt`,
`state/reviewed-plan.json`, `state/snapshot.json`, summary reports, retained failed targets, qualified
and rollback targets, and `result.json` when all recovery phases finish. The script exits nonzero
when reviewed migration is blocked, while still attempting the independent incompatible-schema
and fresh rollback checks. It does not label a blocked apply as a passed destructive migration.
An earlier build or phase failure stops immediately and preserves the evidence collected so far.

This is a synthetic schema-transition rehearsal, not proof that an actual historical release can
recover a snapshot. Before a real non-additive upgrade, retain and independently qualify a recovery
build against that release's exact schema and data. The new binary must not be assumed to accept
older backup schemas. A successful rehearsal also does not qualify the full backend test suite,
merged-schema dispatch, large-library paging or production application health.

### Backup schema identity and pinned-library compatibility

Early qualification attempts on 2026-10-26 were **blocked by build memory**. The normal
rehearsal first exposed missing personal-entity batch-loader registration and an incorrect
consuming subscription-error call; both source defects were corrected. A second normal build
and the single guarded dispatch-omitted attempt were killed at the 6 GiB build limit before
fixture seeding or any restore/migration phase. No live database was migrated or reset.
Evidence is retained in `/tmp/librarian-recovery-20261026-{a,b,c}` and the corresponding
top-level `.log` files. The small transform tests (7), staging/tamper tests (11), and normal
vendored macro tests (17) pass; they do not qualify backup bytes, summary repair or rollback.
These attempts were superseded by the successful normal-runtime rehearsal below.

A further normal-runtime retry (`/tmp/librarian-recovery-20261026-d`) stopped both the
Librarian backend and managed frontend, freed about 800 MiB, and raised the build memory
limit to 7 GiB while retaining the 80% aggregate CPU cap. Rustc was again killed under
memory pressure, at approximately 6.9 GiB charged to the build cgroup; its OOM-kill counter
increased from 18 to 19. No fixture or recovery phase ran. `qualification.json` records
this result. Services remain stopped, and the build limit was restored to 6 GiB.

After the user expanded the VM to 16 GiB, the normal build completed. Attempt `e`
then exposed a fixture setup defect: strict declared policies require the application
policy providers even for trusted repository inserts. The rehearsal now installs the real
entity, field and row providers without lifecycle hooks during legacy fixture creation.
No permissive policy mode or replacement runtime bodies are used.

The fresh normal-runtime rehearsal `/tmp/librarian-recovery-20261026-f` **passed** all
phases with the 14 GiB build cap and unchanged 80% aggregate CPU quota:

- Summary dry-run, repair and idempotence for Artist, Album and Audiobook.
- Full backup and offline restore of 16 fixture rows and three managed objects,
  including artwork and an empty object, with row and byte comparisons.
- Injected partial object-write failure and persistent restart refusal.
- Reviewed destructive migration of the synthetic physical column change using the
  verified source backup, rejection of incompatible-schema recovery, and rollback to
  fresh source-schema targets with matching rows and object bytes.

Evidence is in `result.json`, `resources.txt`, `binary-sha256.txt`, `state/` and `logs/`
under that run directory. Generated public dispatch was present, and both normal library
variants built; it was not exercised by this typed offline rehearsal. This does not
qualify a historical release upgrade, public GraphQL dispatch or the complete application.
The live library was not migrated or reset, and both application services remain stopped.


ORM revision `a90b5bd41f952b02416f23999c729b9ceffcbb8d` emits `Sqlite` in its descriptor snapshot.
Librarian's adapter emits the documented canonical backend identifier `sqlite`; full-backup
verification accepts only these two known SQLite spellings and still verifies the original manifest
checksum. Unknown backends are rejected. Normalization never rewrites a stored manifest.

The pinned ORM has two distinct fingerprints. Its backup-descriptor hash covers entity/column
mapping, logical types (including decimal definitions), backup inclusion/redaction policy and
restore dependencies. Its physical `SchemaModel` hash covers the live tables, columns, defaults,
indexes and constraints used by migration planning. These digests are not interchangeable, even
when both are sixteen-digit FNV-1a64 strings.

New Librarian snapshots store this explicit, versioned identity in the manifest's opaque
`graphql_orm_schema_hash` field:

```text
librarian-schema-v1:<LIVE_PHYSICAL_SCHEMA_HASH>:<BACKUP_DESCRIPTOR_SCHEMA_HASH>
```

Both components use the pinned library's actual hash functions. The physical component is computed
from `DefaultBackend::introspect_schema`, exactly as the migration planner computes its source
fingerprint. The adapter first requires a strict, empty migration plan from that live schema to
its generated entity metadata. This rejects unrepresented extra tables/columns and other drift,
preventing a logical exporter from attesting to a source it cannot faithfully recover. It executes
no migration and never repairs schema during backup.

The existing SHA-256 manifest checksum binds the entire versioned identity, backend, and payload
references. The pinned restore preflight and adapter compare the **whole identity**, so changing
either physical schema nor logical backup interpretation can pass, including in dry-run mode.
Reviewed migration verifies all manifest/payload checksums, parses the supported identity version,
and compares only its explicitly identified physical component to the current plan's physical
source hash. The descriptor fingerprint is retained for restore compatibility; it is never treated
as evidence of physical-schema equality. The reviewed-plan hash and ORM source recheck remain in
force.

Descriptor-only snapshots created before this adapter fix have no physical-schema attestation.
Their checksums can still be verified as full backups, but they cannot authorize a reviewed schema
migration or satisfy the new exact-identity recovery check. Use a separately qualified historical
recovery build to recover one, then create a new full backup with the qualified source-schema build
before migration. Do not rewrite a legacy manifest or infer a physical hash from its descriptor
hash. Unknown identity versions and malformed components fail closed.

The rehearsal records the complete identity, its two components and the independent live-schema
hash in `state/snapshot.json`. The normal-runtime result above qualifies these checks on the
synthetic disposable fixture; historical release/data compatibility remains separate.

### Optional staging-only omission of generated public dispatch

The normal feature-off rehearsal failed to compile on the qualification VM: rustc received SIGKILL
under its 6 GiB memory cap, and the configured virtual swap device did not provide usable compiler
swap. Repeating that unchanged build is not a qualification strategy. One changed, bounded attempt
can use the opt-in flag below; the default command remains the normal-runtime rehearsal:

```bash
scripts/rehearsal-recovery.sh \
  /sys/fs/cgroup/<existing-build-group> \
  /tmp/librarian-recovery-20261026-c \
  --omit-public-dispatch
```

This option replaces `schema_export.rs` **only inside the copied macro package**, using
`scripts/rehearsal-dispatch.rs`. The repository's normal macro package, Cargo features, schema roots,
entity list and typed runtime implementations remain unchanged. `schema-export-only` stays disabled.
The staged transform accepts only direct async methods in inherent impls with exactly one of these
attribute paths: `graphql_orm::async_graphql::{Object,Subscription,ComplexObject}` or
`async_graphql::{Object,Subscription,ComplexObject}`. Rust visibility and method names do not select
methods. Synchronous methods, skipped helpers, trait impls carrying those attributes, and unknown
items inside selected impls cause a compile error instead of broadening erasure. Return signatures
remain intact; omitted dispatch panics if called. Opaque stream/future returns receive unreachable
concrete type witnesses, following the existing exporter helper.

Only the existing operations/relations expansion boundaries are processed. All unmarked inherent
methods, typed repository APIs, mutation-context traits, hooks, policies, relation loaders, metadata,
filters and event generation remain normal. Handwritten application resolver bodies and
`GraphQLEntity` field getters are not rewritten. After **every expansion**, the transform masks only
the selected resolver bodies in the original and emitted ASTs, then requires equality of every
remaining normalized token. This checks signatures, attributes and declarations as well as every
unselected item/body. Selected impl/method names are recorded after the invariant succeeds.

The staged library and example require an explicit rehearsal cfg marker, supplied only by the copied
workspace wrapper when `LIBRARIAN_REHEARSAL_DISPATCH_MARKER=typed-runtime-only-v1` is present. Server
binary builds and backend tests/doctests are forbidden. The original `lib::run` body is preserved
but excluded by an added cfg attribute; its staged replacement refuses service startup. Offline
recovery/migration CLI function bodies are unchanged. Before and after each successful build,
`scripts/rehearsal-dispatch-stage.py` checks all copied input SHA-256s after reversing only the known
library/backup bridges, run/server guards, wrapper marker guard and synthetic source column change.
This proves the selected recovery/reconciliation production bodies retain their original bytes.

The small syntax/invariant fixture suite has its own package and pinned lockfile; it compiles no
backend or generated application runtime. The parent runs it in the serial, capped build lane:

```bash
cargo test --locked --offline \
  --manifest-path scripts/rehearsal-dispatch-tests/Cargo.toml -j 1
```

That fixture suite checks classification, unselected-body and transaction-trait preservation,
signature preservation, stream/future witnesses, and fail-closed rejection of unexpected shapes.
Its success alone does not qualify any database behavior. The runtime rehearsal must still complete
all original backup, byte-restore, summary, failure/restart and reviewed-migration/rollback phases.

Additional evidence includes `dispatch-staging-contract.json`,
`source-invariant-{source,candidate}-schema.json`, and
`dispatch-audit-{source,candidate}-schema.json`, plus the raw impl/method records in each build log.
A build lacking dispatch audit records cannot qualify an executable. Result reporting explicitly
marks generated public dispatch as omitted and normal backend compilation/public GraphQL behavior
as unqualified. If this changed attempt also exceeds the VM limit, retain its evidence and report
the resource block; do not retry unchanged or extend omission into repository, hook or recovery code.

The staged-source check also requires an exact regular-file inventory: the original input manifest
plus only the generated `backend/examples/rehearse-recovery.rs` and saved
`vendor/graphql-orm-macros/src/schema_export.rs.production-original`. Unexpected or missing files,
symlinks, special files and unreadable inventory walks are rejected. Adding a `backend/build.rs`,
an extra module or Cargo configuration cannot be silently adopted. The entire generated example
must equal a fixed byte sequence, including its real `recovery_rehearsal::run().await` call and marker
guard; retaining only the guard suffix is insufficient. Its SHA-256 is recorded in the source
invariant report. Static preparation regressions, requiring no Cargo or compiler, run with:

```bash
python3 scripts/rehearsal-dispatch-stage-tests.py
```
