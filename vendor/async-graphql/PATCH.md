# Local dependency patch

Source: the crates.io `async-graphql` 7.2.1 package, downloaded and checksum-verified
by Cargo. Upstream source and licenses are retained unchanged.

The only upstream-file changes are the `lru` version requirements in `Cargo.toml`
and `Cargo.toml.orig`: `0.16.2` becomes `0.18.4`. The published GraphQL release's
DataLoader feature otherwise pulls `lru` 0.16.4, affected by
[RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253.html).
The patched LRU retains the cache API used here. No advisory suppression is added.

Remove this directory and the application's `[patch.crates-io]` entry when a
compatible async-graphql release requires LRU 0.18.2 or newer. Re-run the backend
authorization, schema, relation, and backup tests and the Rust security checks.

The crate's original `Cargo.lock` and Cargo's local installation markers are
omitted; `backend/Cargo.lock` owns application resolution.
