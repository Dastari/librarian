//! GraphQL query modules (non-entity, domain-specific).

pub mod filesystem;
pub mod schema_migrations;

pub use filesystem::FilesystemQueries;
pub use schema_migrations::SchemaMigrationsQueries;
