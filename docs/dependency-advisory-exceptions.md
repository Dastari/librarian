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
- **Expiry:** 2026-10-30.

The 2026-09-05 dependency refresh moves the torrent client to stable `librqbit` 9.0.1 and removes
the temporary Git-pinned UPnP override; the published release carries the fixed `quick-xml` line.
`async-graphql` 7.2.1 is vendored with only its LRU dependency requirement advanced to 0.18.4,
removing RUSTSEC-2026-0253 without an exception. See
[`vendor/async-graphql/PATCH.md`](../vendor/async-graphql/PATCH.md) for provenance and removal criteria.
Both the full frontend graph and production-only graph report no known vulnerabilities in this
review. Rust maintenance notices for `atomic-polyfill`, `crypto-hash`, and `paste` remain visible.
