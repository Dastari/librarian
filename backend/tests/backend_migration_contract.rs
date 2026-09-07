mod common;

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

fn starts_uppercase_ascii(name: &str) -> bool {
    name.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_uppercase())
}

#[test]
fn backend_uses_shared_graphql_orm_and_agql_auth_dependencies() {
    let cargo_toml = read(backend_root().join("Cargo.toml"));
    let monorepo_url = "https://github.com/Dastari/graphql-orm.git";
    let monorepo_revision = "39181034b02eca6a487eeadcf8d281aaf155a398";

    for (package, version) in [
        ("graphql-orm", "0.30.0"),
        ("graphql-orm-storage", "0.6.2"),
        ("graphql-orm-backup", "0.7.2"),
    ] {
        let dependency = cargo_toml
            .lines()
            .find(|line| line.starts_with(&format!("{package} = ")))
            .unwrap_or_else(|| panic!("missing direct {package} dependency"));
        assert!(dependency.contains(&format!("git = \"{monorepo_url}\"")));
        assert!(dependency.contains(&format!("rev = \"{monorepo_revision}\"")));
        assert!(dependency.contains(&format!("version = \"{version}\"")));
    }

    assert!(!cargo_toml.contains("github.com/Dastari/graphql-orm-storage"));
    assert!(!cargo_toml.contains("github.com/Dastari/graphql-orm-backup"));
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
fn artwork_fetch_uses_the_bounded_ssrf_safe_client_path() {
    let artwork = read(backend_root().join("src/services/artwork.rs"));
    let http_client = read(backend_root().join("src/services/http_client.rs"));

    assert!(!artwork.contains("reqwest::get"));
    assert!(!artwork.contains(".bytes().await"));
    assert!(artwork.contains("OutboundHttpProfile::Artwork"));
    assert!(http_client.contains("Self::Artwork | Self::LocalService => redirect::Policy::none()"));
    assert!(artwork.contains("resolve_artwork_destination"));
    assert!(artwork.contains("BODY_IDLE_TIMEOUT"));
    assert!(artwork.contains("MAX_ARTWORK_BYTES"));
    assert!(artwork.contains("response.chunk()"));
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
fn legacy_sql_migration_files_are_removed() {
    let migrations_dir = backend_root().join("migrations_sqlite");
    if !migrations_dir.exists() {
        return;
    }

    let sql_files = fs::read_dir(&migrations_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", migrations_dir.display()))
        .filter_map(|entry| {
            let path = entry.expect("directory entry should be readable").path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("sql")).then_some(path)
        })
        .collect::<Vec<_>>();

    assert!(
        sql_files.is_empty(),
        "legacy SQL migration files should be removed: {:?}",
        sql_files
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
fn browser_auth_contract_keeps_credentials_out_of_graphql_payloads() {
    let auth_service = read(backend_root().join("src/services/auth.rs"));
    let auth_mutations = read(backend_root().join("src/services/graphql/mutations/auth.rs"));
    let graphql_service = read(backend_root().join("src/services/graphql/service.rs"));
    let config = read(backend_root().join("src/config/mod.rs"));

    assert!(!auth_service.contains("AuthTokens, SimpleObject"));
    assert!(auth_mutations.contains("AuthSessionInfo"));
    assert!(auth_mutations.contains("HttpOnly; SameSite=Lax"));
    assert!(auth_mutations.contains("Path=/graphql"));
    assert!(!auth_mutations.contains("RefreshTokenInput"));
    assert!(!auth_mutations.contains("LogoutInput"));

    assert!(graphql_service.contains("cookie_request_origin_allowed"));
    assert!(graphql_service.contains("peer_is_trusted"));
    assert!(config.contains("LIBRARIAN_TRUSTED_PROXIES"));
}

#[test]
fn private_artwork_requires_auth_and_private_cache_headers() {
    let artwork_api = read(backend_root().join("src/api/artwork.rs"));
    assert!(artwork_api.contains("require_authenticated_user"));
    assert!(artwork_api.contains("can_access_artwork"));
    assert!(artwork_api.contains("private, max-age=86400"));
    assert!(artwork_api.contains("Cookie, Authorization"));
    assert!(!artwork_api.contains("public, max-age"));
}

#[test]
fn database_startup_uses_entity_metadata_for_schema_reset() {
    let database_service = read(backend_root().join("src/services/database.rs"));

    // graphql-orm's schema sync is explicit (validate -> plan -> apply) rather than automatic
    // staged migrations; the backend drives all three steps from the same entity metadata list.
    assert!(database_service.contains("validate_against_entities"));
    assert!(database_service.contains("plan_migration_to_entities"));
    assert!(database_service.contains("apply_migration"));
    assert!(database_service.contains("entity_metadata()"));
    assert!(database_service.contains("as Entity>::metadata"));
    assert!(database_service.contains("PaginationConfig::legacy()"));

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
                contents.contains("use graphql_orm::{"),
                "{} should import graphql-orm derives",
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
            if contents.contains("#[relation(") {
                assert!(
                    contents.contains("GraphQLRelations"),
                    "{} should derive GraphQLRelations when it declares relations",
                    path.display()
                );
            }
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

/// Introspection over the whole schema: every type with its field names,
/// field argument names and input-field names.
const INTROSPECTION: &str = r#"query {
  __schema { types { name fields(includeDeprecated: true) { name args { name } }
                     inputFields { name } } }
}"#;

#[test]
fn generated_graphql_schema_has_no_pascal_case_fields_args_or_input_fields() {
    // Introspected from a live server rather than a checked-in artifact: the
    // artifact used to live in the (now retired) `frontend/` tree, and a
    // snapshot of the schema can only ever be as current as its last export.
    let schema = common::run_app_test(|| async {
        let app = common::TestApp::start().await;
        let mut admin = app.admin_client().await;
        let data = admin.query(INTROSPECTION, serde_json::json!({})).await;
        app.shutdown().await;
        data
    });

    let types = schema
        .pointer("/__schema/types")
        .and_then(|value| value.as_array())
        .expect("introspection schema should contain types");

    let mut violations = Vec::new();
    for ty in types {
        let type_name = ty
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if type_name.starts_with("__") {
            continue;
        }

        if let Some(fields) = ty.get("fields").and_then(|value| value.as_array()) {
            for field in fields {
                let field_name = field
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                if starts_uppercase_ascii(field_name) {
                    violations.push(format!("{type_name}.{field_name}"));
                }

                if let Some(args) = field.get("args").and_then(|value| value.as_array()) {
                    for arg in args {
                        let arg_name = arg
                            .get("name")
                            .and_then(|value| value.as_str())
                            .unwrap_or("");
                        if starts_uppercase_ascii(arg_name) {
                            violations.push(format!("{type_name}.{field_name}({arg_name}:)"));
                        }
                    }
                }
            }
        }

        if let Some(input_fields) = ty.get("inputFields").and_then(|value| value.as_array()) {
            for input_field in input_fields {
                let field_name = input_field
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                if starts_uppercase_ascii(field_name) {
                    violations.push(format!("{type_name}.{field_name}"));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "GraphQL fields, args, and input fields must be lower camelCase:\n{}",
        violations.join("\n")
    );
}

#[test]
fn graphql_name_overrides_do_not_define_pascal_case_fields_or_methods() {
    let graphql_dir = backend_root().join("src/services/graphql");
    let name_attr = regex::Regex::new(r#"#\[graphql\([^\]]*name\s*=\s*"([A-Z][A-Za-z0-9_]*)""#)
        .expect("name attr regex should compile");
    let mut violations = Vec::new();

    for path in rust_files_under(graphql_dir) {
        let contents = read(&path);
        let lines = contents.lines().collect::<Vec<_>>();
        for (idx, line) in lines.iter().enumerate() {
            let Some(capture) = name_attr.captures(line) else {
                continue;
            };

            let mut next = "";
            for candidate in lines.iter().skip(idx + 1).map(|line| line.trim()) {
                if candidate.is_empty() || candidate.starts_with("#[") {
                    continue;
                }
                next = candidate;
                break;
            }

            let is_type_or_enum = next.starts_with("pub struct ")
                || next.starts_with("struct ")
                || next.starts_with("pub enum ")
                || next.starts_with("enum ");
            let is_enum_variant =
                next.ends_with(',') && !next.contains(':') && !next.contains("fn ");
            if !is_type_or_enum && !is_enum_variant {
                violations.push(format!(
                    "{}:{} uses PascalCase #[graphql(name = \"{}\")] before `{next}`",
                    path.display(),
                    idx + 1,
                    &capture[1]
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "PascalCase GraphQL name overrides are only allowed for type names and enum values:\n{}",
        violations.join("\n")
    );
}

#[test]
fn rbac_policy_hooks_and_sensitive_entity_annotations_are_registered() {
    let database_service = read(backend_root().join("src/services/database.rs"));
    assert!(database_service.contains("set_entity_policy(AppEntityPolicy)"));

    let admin_only_entities = [
        "user.rs",
        "refresh_token.rs",
        "invite_token.rs",
        "app_setting.rs",
        "app_log.rs",
        "source.rs",
        "source_priority_rule.rs",
        "usenet_server.rs",
        "metadata_cache.rs",
        "schedule_sync_state.rs",
    ];

    for file_name in admin_only_entities {
        let contents = read(
            backend_root()
                .join("src/services/graphql/entities")
                .join(file_name),
        );
        assert!(
            contents.contains(r#"read_policy = "admin.read""#),
            "{file_name} should require admin reads"
        );
        assert!(
            contents.contains(r#"write_policy = "admin.write""#),
            "{file_name} should require admin writes"
        );
    }

    let user = read(backend_root().join("src/services/graphql/entities/user.rs"));
    let refresh_token = read(backend_root().join("src/services/graphql/entities/refresh_token.rs"));
    let invite_token = read(backend_root().join("src/services/graphql/entities/invite_token.rs"));
    let usenet_server = read(backend_root().join("src/services/graphql/entities/usenet_server.rs"));

    assert!(user.contains("#[graphql_orm(private)]"));
    assert!(refresh_token.matches("#[graphql_orm(private)]").count() >= 3);
    assert!(invite_token.contains("#[graphql_orm(private)]"));
    assert!(usenet_server.matches("#[graphql_orm(private)]").count() >= 2);
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
