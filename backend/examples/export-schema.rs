//! Export the actual merged schema without starting any application services.
//! Kept separate from the full unit-test harness for memory-limited machines.
//! `schema-export-only` stubs generated runtime bodies, never schema declarations.
//! Export success does not qualify normal compilation, authorization or runtime.
use std::sync::Arc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "schema-export-only")]
    eprintln!(
        "Schema-export-only build: exporting actual merged SDL; backend runtime qualification remains pending."
    );
    let db = librarian::db::Database::connect_sqlite("sqlite::memory:").await?;
    let schema = librarian::graphql::build_schema(
        db,
        (),
        Arc::new(librarian::services::ServicesManager::new()),
    );
    let sdl = schema.sdl();
    let entities: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/authorization-matrix.json"))?;
    for entity in entities.as_array().expect("entity registry") {
        let name = entity["entity"].as_str().expect("entity name");
        let first = name.chars().next().expect("nonempty entity");
        let changed = format!(
            "{}{}Changed",
            first.to_lowercase(),
            &name[first.len_utf8()..]
        );
        anyhow::ensure!(
            sdl.contains(&format!("{changed}(")) || sdl.contains(&format!("{changed}:")),
            "Missing registered change root {changed}"
        );
    }
    if let Some(path) = std::env::args().nth(1) {
        std::fs::write(path, sdl)?;
    } else {
        println!("{sdl}");
    }
    Ok(())
}
