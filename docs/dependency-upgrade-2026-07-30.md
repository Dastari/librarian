# Dependency upgrade report — 2026-07-30

## Outcome

Backend, proc-macro, vendored Cast, and frontend manifests were compared with
their authoritative registries or upstream repositories. Every direct
dependency is now either:

1. on the current stable release (or the current published prerelease where
   Librarian already depends on that line), or
2. explicitly held at the newest compatible release with the reason recorded
   below.

Both application lockfiles were regenerated. `cargo update --dry-run --verbose`
now resolves zero additional packages within the selected constraints.
`pnpm outdated` reports only the intentionally held GraphQL major.

## Backend

### Major and compatibility-significant upgrades

| Dependency | Selected version | Notes |
| --- | --- | --- |
| `agql-auth` | `0.12.0`, revision `3f3b0c5365adfbe436514a681d977b600991b797` | Updated to reviewed upstream HEAD; adapted the stored refresh-token contract. |
| `graphql-orm` monorepo | revision `55d0bd255ce0ed913be86f510773a8ba1baa6eed` | `graphql-orm`, storage, and backup remain on one source identity. |
| `reqwest` | `0.13.4` | Migrated the Rustls and query feature names. |
| `tower-http` | `0.7.0` | Updated the application HTTP middleware line. |
| `governor` | `0.10.4` | Updated rate limiting from `0.6`. |
| `sysinfo` | `0.39.6` | Updated system metrics from `0.32`. |
| `scraper` | `0.27.0` | Updated HTML parsing from `0.22`. |
| `mdns-sd` | `0.20.3` | Updated Cast discovery from `0.11`. |
| `flume` | `0.12.0` | Aligned the direct channel dependency with current mDNS APIs. |
| `aes-gcm` | `0.11.0` | Migrated nonce construction away from deprecated slice APIs. |
| `base64` | `0.23.0` | Current direct encoding API. |
| `rand` | `0.10.2` | Migrated from `0.8` through the current `Rng` trait API. |
| `sha2` / `hmac` | `0.11.0` / `0.13.0` | Updated digest formatting and HMAC key initialization. |
| `crossterm` | `0.29.0` | Updated the TUI terminal backend. |
| `windows-service` | `0.8.1` | Updated the Windows target dependency. |

All other registry-backed direct dependencies were advanced to their current
selected releases in `backend/Cargo.toml`; the lockfile also received the
compatible transitive refresh.

### Current intentional holds

- `sqlx` remains at `0.8.6`. `graphql-orm` `0.16.0` currently exposes and
  compiles against SQLx `0.8`; moving only the application to SQLx `0.9` would
  create incompatible pool/query type identities. Upgrade both together when
  the ORM monorepo publishes SQLx `0.9` support.
- `librqbit` remains at the current published maximum, `9.0.0-rc.0`. The
  security-reviewed `librqbit-upnp` patch remains pinned to upstream revision
  `4e5f94cbcf1d57ec500885c77cf1e24d70232d89` until a release contains the
  fixed `quick-xml` line.
- Vendored `rust_cast` remains at the current upstream release, `0.21.0`. Its
  manifest and original manifest now select current compatible dependency
  releases.

### Proc-macro crate

The standalone `macros` crate now uses `syn 3.0.3`, `convert_case 0.11.0`,
`proc-macro2 1.0.107`, and `quote 1.0.47`. The Syn 3 migration also corrected
two stale metadata bindings and removed one dead helper.

## Frontend

### Updated runtime families

- Apollo Client `4.2.8`, GraphQL WS `6.2.0`
- TanStack Router `1.170.18`, router core `1.171.15`, router plugin `1.168.23`,
  and React devtools `0.10.9`
- Tailwind CSS and both integrations `4.3.3`
- React Hook Form `7.83.0` and resolvers `5.5.7`
- Framer Motion `12.43.0`, Tabler Icons `3.46.0`, and Tailwind Variants `3.3.0`
- `data-table-pro 4.1.0` at reviewed upstream revision
  `68439e73891519439f2777778680d76fdfa29db4`

### Updated build and test families

- pnpm `11.18.0`
- Vite `8.1.5`, React plugin `6.0.4`, Vitest `4.1.10`
- TypeScript `7.0.2`, Node types `26.1.2`, React types `19.2.17`
- GraphQL Code Generator CLI `7.2.0` and current plugin releases
- jsdom `30.0.1`, Prettier `3.9.6`, Web Vitals `6.0.1`

TypeScript 7 removed `baseUrl`; the existing `@/*` path mapping now uses its
config-relative path directly.

Unused direct dependencies `lucide-react` and `wonka` were removed. Icons
continue to use the repository-standard Tabler package.

### Current intentional hold

- `graphql` remains at `16.14.2`. The latest GraphQL Code Generator dependency
  graph still contains `graphql-config 5.1.6`, whose peer range ends at GraphQL
  16. GraphQL 17 produced a real peer-contract failure. Move to 17 after that
  upstream toolchain publishes compatible peer metadata.

The Rolldown WASM binding used by Vite `8.1.5` declares `@emnapi/* 1.x` while a
floating `@napi-rs/wasm-runtime` minor moved its peer contract to `2.x`.
The override scopes that nested runtime to the compatible `1.1.6` release;
`pnpm peers check` is clean.

## Verification evidence

### Passed

- `cargo fmt --all --check`
- `cargo check --locked --all-targets -j 1`
- `cargo clippy --locked --all-targets --all-features -j 1 -- -D warnings`
- Backend unit harness: 192 passed, 1 ignored manual scale qualification
- Backend migration/architecture integration suite: 15 passed
- `cargo audit` under the documented advisory-exception policy
- `cargo deny check`
- `cargo machete`
- Standalone macro crate formatting and strict clippy
- `pnpm peers check`
- `pnpm lint`
- `pnpm build`
- `pnpm test`: 25 passed
- `pnpm codegen`: passed twice with byte-identical generated output
- CI and container frontend toolchains aligned on Node 24 and pnpm 11.18.0
- Full and production-only `pnpm audit`: no known vulnerabilities

The backend unit and integration suites completed on the first upgraded
lockfile. After the final `rand 0.10.2` semver correction, the all-target check,
strict clippy, audit, deny, and machete gates all passed. Re-linking the
monolithic generated GraphQL unit harness again was killed by this runner's
8 GiB memory limit during rustc code generation, before any test executed or
diagnostic was emitted.

The monolithic backend release binary initially reached the same runner limit
during final code generation. After the container memory allocation was raised
to 16 GiB, the normal full-LTO
`cargo build --locked --release --features embed-frontend -j 1` completed and
refreshed `target/release/librarian`.

### Codegen note

Production GraphQL introspection remains disabled by design. Codegen now uses
the checked-in schema snapshot by default, while CI exports SDL directly from
the Rust schema and rejects generated drift. Set `CODEGEN_SCHEMA_FILE=0` only
when intentionally generating against an introspection-enabled development
backend.
