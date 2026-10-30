//! Librarian backend binary.
//!
//! All behaviour lives in the `librarian` library crate (`src/lib.rs`) so the
//! integration tests under `backend/tests/` can boot the same service stack.

#[cfg(feature = "schema-export-only")]
compile_error!(
    "schema-export-only is tooling only; build the export-schema example, never the runtime binary"
);

fn main() -> anyhow::Result<()> {
    librarian::run()
}
