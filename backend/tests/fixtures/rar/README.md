# RAR test fixtures

No Rust crate can *create* RAR archives (the `unrar` crate is decompress-only,
and there is no pure-Rust RAR compressor), and this environment has no `rar`
binary. The archives here are therefore copied verbatim from the test data of
the [`unrar`](https://crates.io/crates/unrar) crate (`unrar-0.5.8/data/`),
which is dual-licensed MIT / Apache-2.0 — see `LICENSE-MIT`.

- `version.rar` — single-volume archive with one small text file.
- `crypted.rar` — password-protected archive, used to assert that extraction
  fails with a durable "password protected" error instead of a silent skip.
- `archive.part1.rar` — first volume of a multi-volume set (the remaining
  volumes are intentionally absent, so this also exercises the
  "missing continuation volume" failure path).

`backend/src/services/extract.rs` skips the corresponding tests with a printed
reason if these files are missing.
