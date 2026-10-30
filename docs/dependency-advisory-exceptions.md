# Dependency Advisory Policy and Exceptions

## Policy

- Production dependencies fail CI on any high or critical npm advisory.
- Rust dependencies are checked in CI with `cargo audit`, `cargo deny`, and `cargo machete`.
- GitHub Actions and Git dependencies use immutable commit revisions.
- Dependabot tracks Cargo, npm, and GitHub Actions updates.
- A temporary exception must identify the dependency, affected scope, reason an immediate upgrade is
  unsafe, compensating controls, review trigger, and expiry. It may not weaken the production gate.

## Active exceptions

### RUSTSEC-2023-0071 — `rsa` 0.9.10

- **Owner:** Librarian backend/security maintainers.
- **Affected scope:** `rsa` is linked by `agql-auth`/`jsonwebtoken` to support their RS256 and OIDC
  modes.
- **Librarian exposure:** none of those modes are configured. Librarian constructs
  `AgqlAuthConfig::new` with a minimum-32-byte secret and uses HS256 for access tokens. It neither
  loads nor performs operations with an RSA private key.
- **Why not upgraded:** RustSec reports no patched `rsa` release. Removing the crate requires an
  upstream `agql-auth` feature boundary that does not yet exist.
- **Compensating controls:** HS256 is the only configured signing mode; startup rejects absent or
  short secrets; the advisory remains visible in both `cargo audit` and `cargo deny` configuration
  and is reviewed by Dependabot/security CI.
- **Review trigger:** remove this exception as soon as `rsa` publishes a fixed release or
  `agql-auth` can compile without its RS256 dependency.
- **Expiry:** 2027-01-31. Re-reviewed 2026-10-30 for the 0.3.0 release: still no patched `rsa`
  release, and Librarian still configures HS256 only.

### RUSTSEC-2026-0387 — `crypto-bigint` 0.5.5

- **Owner:** Librarian backend/security maintainers.
- **Affected scope:** linked through `elliptic-curve`/`ecdsa`/`p256` by `jsonwebtoken` 10's
  `rust_crypto` feature, which `agql-auth` enables for its ES256 support.
- **Librarian exposure:** none. The timing variability affects elliptic-curve operations on secret
  scalars; Librarian signs and verifies HS256 only (see RUSTSEC-2023-0071) and never loads an EC key.
- **Why not upgraded:** the fix is `crypto-bigint` 0.7, which needs the RustCrypto 0.14 curve stack
  that `jsonwebtoken` 10 does not use. Dropping it needs an `agql-auth` feature boundary.
- **Compensating controls:** as for RUSTSEC-2023-0071; the advisory stays listed in both configs.
- **Review trigger:** a `jsonwebtoken` release on the patched curve stack, or an `agql-auth` build
  without EC algorithms.
- **Expiry:** 2027-01-31.

### RUSTSEC-2026-0293 — `ringbuf` 0.4.8

- **Owner:** Librarian backend maintainers.
- **Affected scope:** `librqbit-utp` 0.7.0 (via `librqbit` 9.0.1) uses `ringbuf` for uTP send buffers.
- **Librarian exposure:** none. The double free needs an element whose `Drop` panics during
  `Consumer::skip`/`clear`; `librqbit-utp`'s buffers are `SharedRb<Heap<u8>>`, and `u8` has no `Drop`.
- **Why not upgraded:** the fix is `ringbuf` 0.5, a breaking change only `librqbit-utp` can adopt; no
  newer `librqbit-utp` 0.7.x exists.
- **Review trigger:** a `librqbit`/`librqbit-utp` release on `ringbuf` 0.5.
- **Expiry:** 2027-01-31.

### RUSTSEC-2026-0391, RUSTSEC-2026-0403, RUSTSEC-2026-0398 — `zstd` 0.13.3 / `zstd-safe` 7.3.0

- **Owner:** Librarian backend maintainers (with `graphql-orm`).
- **Affected scope:** `graphql-orm-backup` (pinned `graphql-orm` revision) compresses and decompresses
  logical backup payloads with `zstd` 0.13. The archive extractor already uses the fixed `zstd` 0.14
  through `zip` 9.
- **Librarian exposure:**
  - 0391 needs a compression/decompression context reused after an abandoned stream. Each backup
    payload uses its own `encode_all` call or `Encoder`/`Decoder`, so no context is reused.
  - 0398 is in the `Seekable` API, which is never called.
  - 0403 exposes uninitialised memory when streaming decompression of a *legacy* (pre-v0.8) frame
    fails. Librarian only decompresses payloads it wrote itself, as current-format frames, in a
    local backup repository that only the server account and administrators can write. Anyone who
    can plant a crafted frame there already controls restore contents.
  - `cargo deny` (which fails on unsound advisories) also lists ten `zstd-safe` 7 unsound-API notices:
    RUSTSEC-2026-0395/0396/0397/0399/0400/0401/0402/0404/0405/0406. They cover context cloning
    (`try_clone`), dictionaries (`init_using_cdict`/`ddict`, `train_from_buffer`), multithreaded
    contexts, `CParameter::StableInBuffer`, `Seekable`, and caller-supplied out-of-range
    `InBuffer`/`OutBuffer` positions. `graphql-orm-backup` calls none of these; it uses only the `zstd`
    crate's single-threaded stream wrappers, which keep buffer positions in range. `cargo audit`
    reports them as allowed warnings.
- **Why not upgraded:** the fixes are `zstd` 0.14 / `zstd-safe` 8, which must be adopted in
  `graphql-orm-backup`; Librarian pins that crate by commit.
- **Review trigger:** the next `graphql-orm` revision bump, which should move `graphql-orm-backup`
  to `zstd` 0.14.
- **Expiry:** 2027-01-31.

The 0.3.0 release also updated `rustls` 0.23.46, `rustls-webpki` 0.103.16, `webpki-roots`/
`webpki-root-certs` 1.0.11, `quinn-proto` 0.11.19, `ctutils` 0.4.7, `zstd-sys` 2.1.2 and `tokio`
1.54.1, and moved archive extraction to `zip` 9 (RUSTSEC-2026-0367/0368/0369). Librarian's extractor
already rejected symlinks and special modes itself, so those `zip` advisories were not reachable.

The 2026-09-05 dependency refresh moves the torrent client to stable `librqbit` 9.0.1 and removes
the temporary Git-pinned UPnP override; the published release carries the fixed `quick-xml` line.
`async-graphql` 7.2.1 is vendored with only its LRU dependency requirement advanced to 0.18.4,
removing RUSTSEC-2026-0253 without an exception. See
[`vendor/async-graphql/PATCH.md`](../vendor/async-graphql/PATCH.md) for provenance and removal criteria.
Both the full frontend graph and production-only graph report no known vulnerabilities in this
review. Rust maintenance notices for `atomic-polyfill`, `crypto-hash`, and `paste` remain visible.
