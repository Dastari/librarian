# Local macro patch: schema export tooling

Source: graphql-orm 0.89.0, workspace tag `workspace-2026.11.01.1`, revision
`c9465e5134ce3311c0fdb3dd656f24e731874eb5`, `crates/graphql-orm-macros`.
The MIT license is included. Runtime/storage/backup crates use the same tag.

The earlier local patch that surfaced subscription lag as `LIVE_RESYNC_REQUIRED` is
gone: graphql-orm 0.88.0 ends a lagging generated `Changed` stream with one
`SUBSCRIPTION_CONTINUITY_LOST` error, which the backend's
`subscriptions/lag_tests.rs` and the frontend's live layer now rely on.

Normal builds compile the upstream macros unchanged: `src/lib.rs` only passes their
output through `schema_export::finish`, an identity without the feature below. The
manifest replaces inherited workspace rust_decimal metadata with the matching
dependency range so this vendored crate builds independently.


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
proc-macro panics. These are macro transformation tests, not backend runtime tests.
