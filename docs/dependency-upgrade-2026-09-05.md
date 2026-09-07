# Dependency upgrade — 2026-09-05

## Result and scope

Reviewed every declared frontend, backend, standalone macro, and vendored Cast dependency against
npm, crates.io, or its upstream Git repository. Updated the available releases, refreshed both
application lockfiles, and adapted incompatible APIs. Existing uncommitted project work was preserved.

The remaining direct-version holds are SQLx 0.8.6 and GraphQL.js 16.14.2, with concrete upstream
compatibility constraints below. Stable packages were not moved onto prereleases. The torrent
dependency moved from its existing prerelease to stable 9.0.1.

`pnpm outdated --format json` now reports only GraphQL.js. `cargo update --dry-run --verbose`
resolves zero further packages within the selected constraints. This does not mean every transitive
crate is on its latest major: upstream dependency ranges and exact pins still apply.

## Major changes

| Dependency | Before this review | Selected |
| --- | --- | --- |
| graphql-orm | 0.16.0 | 0.30.0 |
| graphql-orm-storage / backup | 0.6.0 / 0.7.0 | 0.6.2 / 0.7.2 |
| agql-auth | 0.12.0 | 0.19.0 |
| librqbit | 9.0.0-rc.0 | 9.0.1 stable |
| quick-xml | 0.41.0 | 0.42.0 |
| mdns-sd | 0.20.3 | 0.21.1 |
| data-table-pro | 4.1.0 | 5.3.0 |
| TanStack React Table | 8.21.3 | 9.2.4 |
| Framer Motion | 12.43.0 | 13.2.0 |
| Vitest | 4.1.10 | 5.0.0 |
| Vite | 8.1.5 | 8.2.2 |
| pnpm | 11.18.0 declared | 11.25.0 |
| hls.js | 1.6.16 | 1.7.2 |

The ORM packages use the same immutable revision
`39181034b02eca6a487eeadcf8d281aaf155a398`. Auth uses
`1d2e9fe2e1576105212a7b340a11abf8cad0382d`; the table library uses
`758c39dcd8c40a8b198c4d75c3242cf1cc8ed08d`.

Other backend upgrades: tower-http 0.7.1, futures 0.3.34, thiserror 2.0.20, async-trait 0.1.92,
uuid 1.26.0, time 0.3.55, ipnet 2.12.1, rust_decimal 1.43.0, aes-gcm 0.11.1, and base64 0.23.1.
The macro crate uses syn 3.0.5 and convert_case 0.12.0. Vendored rust_cast remains on its current
upstream release, 0.21.0; log, thiserror, and its mDNS example dependency were refreshed.

Other frontend upgrades: Apollo 4.2.12; current TanStack Router/core/plugin/history/devtools;
HeroUI styles 3.2.4; React Hook Form 7.87.0 and resolvers 5.9.1; Zod 4.5.4; nuqs 2.10.1;
tailwind-variants 3.3.1; graphql-ws 6.2.1; GraphQL Codegen CLI 7.4.0 and operations 6.1.6;
React Vite plugin 6.1.1; Testing Library React 16.3.3; updated Node/React type packages;
and web-vitals 6.2.1. Already-current dependencies retain their versions.

pnpm 11.25.0 is aligned in package.json, CI, release workflows, and both Dockerfiles. Node 24
remains the application build runtime. The application compiles with the existing Rust 1.95 toolchain.

After the user specified a CPU ceiling, all active build process trees were moved to a shared Linux
cgroup with `cpu.max = 320000 100000`: 80% of this container's four available CPUs. Subsequent builds
join the same group, so parallel invocations share the quota. Interactive tools are outside it.
`AGENTS.md` now records this requirement for future work; `-j 1` is not treated as a CPU limit.

## Compatibility changes

- **Table sorting:** migrated column `sortingFn` to the v9 `sortFn` API. Initial sorting now uses
  `initialState` rather than an always-controlled value, so tables without a controlling parent can
  change sort order. Added a test rendering the actual installed table library, exercising a custom
  comparator and a header click; the existing adapter tests remain.
- **Torrent state:** adapted patterns to librqbit's structured `Initializing` state while retaining
  the application's queued/active initialization behavior.
- **XML:** quick-xml 0.42 exposes decoded string event/name data. Removed redundant byte decoding,
  retained XML entity unescaping and CDATA handling, and adapted the RSS parser test helper.
- **Schema plans:** compound foreign-key models expose ordered column pairs rather than single
  source/target column fields. Plan descriptions now include all members.
- **Generated GraphQL:** exported SDL from the updated Rust schema and regenerated the frontend
  snapshot and types. `DateRangeInput.start` and `.end` are now required; other generated changes
  are descriptions. A second generation from the snapshot produced identical files.
- **Development route splitting:** subsequent server startup exposed an unbounded pre-existing
  `@babel/core >=7.29.1` override forcing Babel 8 into consumers that declare Babel 7. TanStack's
  development route splitter still uses `Scope.references`, removed in
  [Babel 8](https://babeljs.io/docs/v8-migration-api/). The override now applies only to Babel 7
  consumers and selects `^7.29.7`, preserving their supported API. All 37 checked development
  modules load, the public landing page renders, and frontend checks/build/audit pass again.
- **Backup values:** added exact typed decimal serialization/deserialization. A regression test
  covers an 18-digit value, retained precision/scale, and rejection of floating-point/string or
  precision-losing restore input. Existing values keep their prior representation.
- **Security patch:** removed the old Git-pinned librqbit-upnp override; stable librqbit now carries
  the fixed quick-xml dependency. Added the narrow async-graphql patch described below and included it
  in the Docker dependency build stage.

No application entity columns or environment variables were added. Upstream ORM releases changed
schema fingerprints and date-range validation; do not equate that with verified compatibility of old
backup manifests. Rehearse old-version backup/upgrade/restore on a disposable copy before deployment.
New agql-auth access tokens use the standard `scope` claim; its default validator still accepts legacy
scope arrays. This application issues and validates tokens in the same backend.

## Security findings and resolution

The first updated frontend install retained vulnerable transitive versions. Refreshing the complete
lockfile removed four high-severity advisory reports affecting brace-expansion, nanoid, and
browserslist. Both full and production-only audits now report zero known vulnerabilities.

The published async-graphql 7.2.1 DataLoader feature requires LRU 0.16, which triggered the existing
`cargo deny` unsoundness gate for
[RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253.html). A copy of the checksum-verified
registry package is retained under `vendor/async-graphql`; **only its LRU dependency requirement**
changes to 0.18.4 in its two manifests. Its Rust source and licenses are unchanged. See
[`PATCH.md`](../vendor/async-graphql/PATCH.md) for provenance and removal criteria. No new advisory
exception or relaxed security gate was added.

`cargo audit` and `cargo deny` pass under the existing, documented HS256-only RSA exception
(RUSTSEC-2023-0071, expiry 2026-10-30). Three transitive maintenance notices remain:
atomic-polyfill, crypto-hash, and paste. Duplicate-version warnings remain visible. These are not
claims of an advisory-free Rust dependency graph.

## Intentional holds

| Package | Constraint and next step |
| --- | --- |
| SQLx 0.8.6 | Current graphql-orm 0.30.0 still uses SQLx 0.8 and exposes its types. Moving only Librarian to 0.9 creates incompatible pool/query types. Upgrade together after upstream support. |
| GraphQL.js 16.14.2 | graphql-config 5.1.6, still used by current Codegen, declares peers only through GraphQL 16. Apollo now supports 17, but that alone does not make the complete toolchain compatible. |
| async-graphql 7.2.1 | Current stable line; patched dependency requirement avoids adopting GraphQL 8 prereleases merely to address a transitive vulnerability. |
| rust_cast 0.21.0 | Current upstream release; retain the existing local socket-deadline patch. |

Cargo also reports crypto-common 0.1.6, matchit 0.8.4, and pdqselect 0.1.0 behind available patches
under upstream constraints. They are transitive, not hidden direct dependencies. The reviewed
resolution produces no applicable vulnerability/unsoundness failure under the existing policy.

Primary sources: [ORM migration guide](https://github.com/Dastari/graphql-orm/blob/39181034b02eca6a487eeadcf8d281aaf155a398/MIGRATION.md),
[auth migration guide](https://github.com/Dastari/agql-auth/blob/1d2e9fe2e1576105212a7b340a11abf8cad0382d/MIGRATION.md),
[table package](https://github.com/Dastari/data-table-pro/blob/758c39dcd8c40a8b198c4d75c3242cf1cc8ed08d/package.json),
[Vite registry](https://www.npmjs.com/package/vite),
[graphql-config peer declaration](https://registry.npmjs.org/graphql-config/5.1.6).

## Verification

| Check | Result |
| --- | --- |
| Baseline backend all-target check | Passed |
| Baseline frontend type-check/tests | Passed, 25 tests |
| Updated backend all-target check | Passed |
| Backend format | Passed |
| Backend unit tests | Passed: 193, with one separately exercised ignored scalability harness; see memory limitation below |
| Backend repository contract tests | Passed: 15; updated the contract's pinned ORM revision/version assertions |
| Strict backend Clippy, all features | Passed |
| Backend schema export and regenerated frontend types | Passed; snapshot-based regeneration is deterministic |
| Macro crate test/format/strict Clippy | Passed; no runnable unit tests, five ignored example doc-tests |
| Vendored Cast all-target/all-feature check | Passed |
| Frontend frozen install, type-check, production build | Passed; type-check, tests, and build repeated successfully after schema regeneration |
| Frontend tests | Passed, 26 tests across 11 files |
| pnpm peer check | Passed |
| Full and production npm audits | Zero known vulnerabilities |
| cargo audit | Passed with existing RSA exception; three maintenance notices |
| cargo deny | Advisories, licenses, sources, and bans passed |
| cargo machete | No unused backend direct dependencies |
| Isolated FFmpeg commands | Remux and transcode produced complete probeable HLS from a synthetic eight-second fixture |
| Synthetic 100,000-file scan discovery | Passed; 314 ms traversal on this temporary filesystem, excluding fixture creation and metadata/import work |

### Backend test memory limitation

The ordinary `cargo test --locked -j 1` compilation was killed by the OOM killer on this 8 GiB
machine before tests ran. Retrying with `CARGO_PROFILE_TEST_DEBUG=0`,
`CARGO_PROFILE_TEST_CODEGEN_UNITS=256`, and `CARGO_INCREMENTAL=0` successfully built the unit-test
executable. Cargo then tried to build a second, non-test application executable for integration
tests. That compilation was stopped during heavy memory pressure. A 7 GiB soft memory threshold
was also applied to the build cgroup to preserve system responsiveness.

All 193 unit tests were run successfully from the completed executable. The 15 repository contract
tests were compiled separately with Rust 2024 and the resolved `regex`/`serde_json` libraries, then
run successfully. The schema export and ignored 100,000-file harness used that same unit-test
executable. The full default `cargo test` command itself is therefore **not** reported as passing.
The environment-only memory settings were not made permanent Cargo profile changes. CI and the
production backend build still need qualification on their actual runners.

Production frontend bundling still warns about the large vendor chunk. Physical Cast, live provider
downloads, browser visual QA, Windows compilation/installation, and a production release build are
not implied by these local checks. No server was launched or deployed.

### Subsequent requested server startup

After the initial review, the user requested both servers online. The updated backend executable
built successfully with the 80% CPU quota and these environment-only memory settings:

```sh
MALLOC_CONF=narenas:1,dirty_decay_ms:0,muzzy_decay_ms:0 \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_CODEGEN_UNITS=256 CARGO_INCREMENTAL=0 \
cargo rustc --locked --bin librarian -j 1 -- -C codegen-units=1024 -C llvm-args=-threads=1
```

This command was run inside the build cgroup, not unthrottled. The compiler allocator settings
released unused memory sufficiently to finish the application build on this machine. A consistent
SQLite snapshot was saved before starting. Startup confirmed the existing live schema already
matched and required no migration. The backend and frontend development servers were then started
outside the build cgroup. Public frontend, health, readiness, and GraphQL HTTP checks pass at
`https://librarian.dastari.net`; the landing page renders in a browser. Runtime logs and PID files
are under `/tmp/librarian-runtime/`. This is a development startup, not production release qualification.

The feature-completion assessment and prioritized acceptance criteria are in
[`project-review-2026-09-05.md`](project-review-2026-09-05.md).
