//! Librarian backend binary.
//!
//! All behaviour lives in the `librarian` library crate (`src/lib.rs`) so the
//! integration tests under `backend/tests/` can boot the same service stack.

fn main() -> anyhow::Result<()> {
    librarian::run()
}
