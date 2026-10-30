# Local macro patches: subscription lag and schema export tooling

Source: graphql-orm 0.57.0, revision
`a90b5bd41f952b02416f23999c729b9ceffcbb8d`, `crates/graphql-orm-macros`.
The MIT license is included. Runtime/storage/backup crates retain the same pin.

The generated entity subscription previously discarded broadcast receive errors.
A browser could remain connected with an incorrect snapshot indefinitely after
a burst exceeded channel capacity. The host cannot infer this lost receive from
the remaining metadata payloads or transport health.

This patch changes generated stream items to GraphQL Results and reports a
`LIVE_RESYNC_REQUIRED` error on lag in both ordinary single-primary-key and
append-only subscription generators. The previously patched branch was the
append-only generator, not a composite-key subscription generator. Successful
payloads, schema fields, caller authorization and generated witness checks
remain unchanged. The frontend must
reconcile its authorized snapshot and reattach the affected listener. Events
remain bounded invalidations, not a durable replay log.

Both branches pass an owned `async_graphql::Error` to the pinned
`ErrorExtensions::extend_with(self, ...)` API. Passing a reference selects the
blanket reference implementation, which requires `Display` and does not compile
for this error type. The feature-off AST expectations use the same owned call.

The only normal-runtime behavior change is the subscription generator in
`src/operations.rs`. The optional tooling changes below are disabled by default.
The crate manifest replaces inherited workspace rust_decimal metadata with the
matching dependency range so this narrow vendored crate builds independently.
Remove the patch when a qualified upstream release provides equivalent recovery.

Patched `src/operations.rs` SHA-256:
`bbca22ce5ad4877797d06fe63e3eee987bbe5ba9ad0e50238365b21a35cda978`.

`src/operations_lag_tests.rs` adds two AST expansion checks, one for each
subscription branch, compiled only with `schema-export-only` disabled. They
inspect the original generator output, verify the export postprocessor is an
identity, and assert identical Result item types, resync error mapping,
successful payload wrapping, and authorization-error filtering. They also
check that provenance, row/field policy and authority rechecks remain present.
All 17 feature-off macro tests, including these two, passed on 2026-10-25.
`backend/src/services/graphql/subscriptions/lag_tests.rs` additionally drives a
real generated Library subscription through a two-event channel, committed
writes, a lag error, an authorized snapshot and reattachment. This runtime test
has been written but not run on the constrained VM; AST checks do not qualify
its runtime behavior.


## Optional schema export mode

`schema-export-only` is a proc-macro feature, **off by default**. The backend
forwards its same-named feature through an optional direct path dependency on
this exact patched macro package, unifying that host feature with the ORM's
transitive proc-macro dependency. Runtime/storage/backup revision pins remain
unchanged; no second ORM or hand-maintained SDL is introduced.

`src/schema_export.rs` postprocesses the generated `GraphQLOperations` and
`GraphQLRelations` syntax trees, including the operations part of
`RepositoryEntity`. It replaces only async functions and functions returning
Future/Stream types. Every function signature, visibility, generic, attribute,
field/default/input type, policy descriptor, metadata function, synchronous
constructor and schema/DataLoader registration stays in place. GraphQLEntity
and schema_roots expansions are not transformed. With the feature disabled,
the original token stream is returned directly without parsing or reprinting;
the independent lag-delivery patch remains unchanged.

Generated runtime bodies panic in this mode. Opaque Stream/Future returns retain
an unreachable concrete witness with their original Item/Output type, including
`Result<impl Stream<Item = Result<ChangedEvent>>>`. Unsupported opaque wrappers
fail expansion with a diagnostic rather than inventing a public type. This
trades generated runtime body compilation for schema export; it does **not**
verify that those original bodies compile, authorize correctly or work at runtime.

The backend binary emits compile_error under this feature. Backend unit/doctest
builds emit compile_error, and the library's run() returns an error before
starting Tokio/services. The export example still constructs the actual merged
build_schema result and calls schema.sdl(); it does not execute queries or start
application services. It labels tooling-only exports on stderr. Never deploy a
library/binary built with this feature or use it to claim runtime qualification.

When the parent's validation slot is free, inside the repository's aggregate
CPU/memory build limits, build only the export example:

```sh
cargo build --manifest-path backend/Cargo.toml --example export-schema --features schema-export-only
backend/target/debug/examples/export-schema web/schema.graphql
```

On a memory-limited Linux host, `scripts/export-schema-low-memory.sh
<existing-build-cgroup> <sdl-output>` also applies guarded compiler hints to the
tooling library and exporter only. It refuses a cgroup without an aggregate CPU
quota at or below 80%, uses one compiler lane and bounded timeouts, and starts no
services. Configure an appropriate cgroup memory limit before invoking it. Its
wrapper leaves normal runtime invocations unchanged. These unstable compiler
hints require the installed compiler to support them; they are not production
build settings. Normal-versus-tooling SDL parity is enforced separately in CI.

Use a separate tooling target directory when convenient to distinguish artifacts.
Do not use --all-targets in this mode: the runtime/test guards intentionally
reject it. Codegen may consume a successful actual merged SDL export, but normal
library/binary compilation, runtime suites, policy/event tests and migration
qualification remain separate mandatory gates. On a sufficiently capable runner,
compare this export with the same example built **without** the feature before
claiming SDL parity for the qualified revision.

Five small source-expansion tests cover feature gating, unchanged declarations
and metadata/constructors, the typed subscription witness, actual operation and
relation expansions, rejection of unsupported opaque shapes, and independent
stub-block round trips without inner attributes. Stub AST construction uses
fallible parsing so syntax errors become expansion diagnostics rather than
proc-macro panics. These are
macro transformation tests, not backend runtime tests. The parent reports all
five passed in `/tmp/librarian-core-macro-tests.log`; the new feature-off lag
tests above were added afterward and remain unrun. Export success and SDL parity
remain separate gates for the parent to perform.
