use std::fs;
use std::path::{Path, PathBuf};

fn backend_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn rust_files_under(path: impl AsRef<Path>) -> Vec<PathBuf> {
    fn visit(path: &Path, files: &mut Vec<PathBuf>) {
        if !path.exists() {
            return;
        }

        for entry in fs::read_dir(path)
            .unwrap_or_else(|err| panic!("failed to read directory {}: {err}", path.display()))
        {
            let entry = entry.expect("directory entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                visit(&path, files);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    visit(path.as_ref(), &mut files);
    files.sort();
    files
}

fn assert_not_contains(path: &Path, contents: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            !contents.contains(needle),
            "{} contains forbidden pattern `{}`",
            path.display(),
            needle
        );
    }
}

#[test]
fn backend_uses_shared_graphql_orm_and_agql_auth_dependencies() {
    let cargo_toml = read(backend_root().join("Cargo.toml"));

    assert!(
        cargo_toml.contains("graphql-orm = { git = \"https://github.com/Dastari/graphql-orm\"")
    );
    assert!(cargo_toml.contains("agql-auth = { git = \"https://github.com/Dastari/agql-auth\""));
    assert!(cargo_toml.contains("sqlite = [\"sqlx/sqlite\", \"graphql-orm/sqlite\"]"));

    assert!(!cargo_toml.contains("bcrypt"));
    assert!(!cargo_toml.contains("jsonwebtoken"));
    assert!(!cargo_toml.contains("macros ="));
}

#[test]
fn backend_source_does_not_contain_direct_sql_access() {
    let forbidden = [
        "sqlx::query",
        "sqlx::query_as",
        "query_scalar",
        "SELECT ",
        "INSERT ",
        "UPDATE ",
        "DELETE ",
        "CREATE TABLE",
        "ALTER TABLE",
        "DROP TABLE",
        "PRAGMA ",
    ];

    let mut checked = 0usize;
    for path in rust_files_under(backend_root().join("src")) {
        let contents = read(&path);
        assert_not_contains(&path, &contents, &forbidden);
        checked += 1;
    }

    assert!(checked > 0, "expected backend source files to be checked");
}

#[test]
fn legacy_sql_repository_and_loader_modules_are_removed() {
    let removed_paths = [
        "src/db/artwork.rs",
        "src/db/users.rs",
        "src/db/seed.rs",
        "src/db/schema_sync.rs",
        "src/db/sqlite_helpers.rs",
        "src/services/graphql/loaders.rs",
        "src/services/graphql/queries/schema_migrations.rs",
    ];

    for relative_path in removed_paths {
        let path = backend_root().join(relative_path);
        assert!(
            !path.exists(),
            "legacy SQL/data-access path should be removed: {}",
            path.display()
        );
    }

    assert!(
        rust_files_under(backend_root().join("src/db/operations")).is_empty(),
        "legacy db operations directory should not contain Rust source files"
    );
    assert!(
        rust_files_under(backend_root().join("src/services/graphql/orm")).is_empty(),
        "legacy GraphQL ORM directory should not contain Rust source files"
    );
}

#[test]
fn graphql_schema_is_composed_with_graphql_orm_schema_roots() {
    let schema = read(backend_root().join("src/services/graphql/schema.rs"));

    assert!(schema.contains("graphql_orm::schema_roots"));
    assert!(schema.contains("schema_roots!"));
    assert!(schema.contains("entities: ["));
    assert!(schema.contains("extra_query_types:"));
    assert!(schema.contains("extra_mutation_types:"));
    assert!(schema.contains("extra_subscription_types:"));
    assert!(schema.contains("schema_builder(db)"));

    assert!(!schema.contains("DataLoader"));
    assert!(!schema.contains("SchemaMigrationsQueries"));
    assert!(!schema.contains("crate::services::graphql::loaders"));
}

#[test]
fn auth_service_uses_agql_auth_without_legacy_password_or_token_libraries() {
    let auth = read(backend_root().join("src/services/auth.rs"));

    assert!(auth.contains("agql_auth::"));
    assert!(auth.contains("impl UserStore for LibrarianAuthStore"));
    assert!(auth.contains("impl RefreshTokenStore for LibrarianAuthStore"));
    assert!(auth.contains("User::query"));
    assert!(auth.contains("RefreshToken::query"));

    assert!(!auth.contains("bcrypt"));
    assert!(!auth.contains("jsonwebtoken"));
    assert!(!auth.contains("sqlx::query"));
}

#[test]
fn database_startup_uses_entity_metadata_for_schema_reset() {
    let database_service = read(backend_root().join("src/services/database.rs"));

    assert!(database_service.contains("SchemaStageRunner"));
    assert!(database_service.contains("SchemaStage::from_entities"));
    assert!(database_service.contains("entity_schema_stage()"));
    assert!(database_service.contains("apply_schema_stages"));
    assert!(database_service.contains("as Entity>::metadata"));

    assert!(!database_service.contains("migrations_sqlite"));
    assert!(!database_service.contains("schema_sync"));
    assert!(!database_service.contains("crate::db::seed"));
    assert!(!database_service.contains("seed::"));
}

#[test]
fn entity_files_use_graphql_orm_derives_and_camel_case_generation() {
    let entities_dir = backend_root().join("src/services/graphql/entities");
    let mut entity_files = Vec::new();

    for path in rust_files_under(&entities_dir) {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if matches!(
            file_name,
            "batch_load.rs" | "cast.rs" | "common.rs" | "mod.rs"
        ) {
            continue;
        }

        let contents = read(&path);
        if contents.contains("#[graphql_entity(") {
            entity_files.push(path.clone());
            assert!(
                contents.contains(
                    "use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations}"
                ) || contents.contains(
                    "use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};"
                ) || contents.contains(
                    "use graphql_orm::{GraphQLEntity, GraphQLRelations, GraphQLOperations}"
                ) || contents.contains(
                    "use graphql_orm::{GraphQLEntity, GraphQLRelations, GraphQLOperations};"
                ),
                "{} should import shared graphql-orm derives",
                path.display()
            );
            assert!(
                contents.contains("GraphQLEntity"),
                "{} should derive GraphQLEntity",
                path.display()
            );
            assert!(
                contents.contains("GraphQLOperations"),
                "{} should derive GraphQLOperations",
                path.display()
            );
            assert!(
                contents.contains("GraphQLRelations"),
                "{} should derive GraphQLRelations",
                path.display()
            );
            assert!(
                contents.contains("#[graphql(rename_fields = \"camelCase\")]"),
                "{} should align generated ORM fields to camelCase",
                path.display()
            );
            assert!(
                !contents.contains("notify ="),
                "{} should not use legacy local macro notify attributes",
                path.display()
            );
        }
    }

    assert!(
        entity_files.len() >= 40,
        "expected broad entity coverage, found {} entity files",
        entity_files.len()
    );
}

#[test]
fn batch_loader_supports_every_graphql_orm_entity_export() {
    let entities_mod = read(backend_root().join("src/services/graphql/entities/mod.rs"));
    let batch_load = read(backend_root().join("src/services/graphql/entities/batch_load.rs"));

    let skipped_exports = ["CastMutations"];
    let mut checked = 0usize;

    for line in entities_mod.lines() {
        let line = line.trim();
        if !line.starts_with("pub use ") || !line.ends_with("::*;") {
            continue;
        }

        let module = line
            .trim_start_matches("pub use ")
            .trim_end_matches("::*;")
            .trim();
        let module_file = backend_root()
            .join("src/services/graphql/entities")
            .join(format!("{module}.rs"));
        let module_contents = read(&module_file);
        if !module_contents.contains("#[graphql_entity(") {
            continue;
        }

        let type_name = module
            .split('_')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<String>();

        if skipped_exports.contains(&type_name.as_str()) {
            continue;
        }

        assert!(
            batch_load.contains(&format!("super::{type_name}")),
            "batch_load.rs should implement BatchLoadEntity for {type_name}"
        );
        checked += 1;
    }

    assert!(
        checked >= 40,
        "expected many batch-loaded entities, checked {checked}"
    );
}
