//! Runtime regressions for pinned ORM descriptor identity versus physical migration identity.
//! Parent owns all compilation and execution; these use disposable generated schemas only.
use super::*;
use crate::db::DbPool;
use crate::graphql::entities::{AppSetting, CreateAppSettingInput};
use crate::services::database::bootstrap_schema;
use graphql_orm::graphql::orm::{ApplyOptions, ColumnBackupPolicy, stable_schema_hash};

async fn database() -> Database {
    let mut db = Database::new(DbPool::connect("sqlite::memory:").await.unwrap());
    // Backup fixtures need declared policies without application lifecycle hooks.
    db.set_entity_policy(crate::services::graphql::policy::AppEntityPolicy);
    db.set_field_policy(crate::services::graphql::policy::AppFieldPolicy);
    db.set_row_policy(crate::services::graphql::row_policy::OwnershipRowPolicy);
    bootstrap_schema(&db).await.unwrap();
    db
}

fn source(db: Database, root: &Path) -> BackupService {
    BackupService::new(
        db,
        Arc::new(
            ObjectStorageService::new(ObjectStorageServiceConfig {
                backend: "local".into(),
                local_path: root.join("objects"),
            })
            .unwrap(),
        ),
        BackupServiceConfig {
            repository_path: root.join("backups"),
        },
    )
}

async fn setting(db: &Database) -> AppSetting {
    AppSetting::insert(
        db,
        CreateAppSettingInput {
            key: "schema-identity-regression".into(),
            value: "fixture value".into(),
            description: Some("preserve source description".into()),
            category: "test".into(),
        },
    )
    .await
    .unwrap()
}

#[test]
fn normalizes_only_known_sqlite_spellings_and_rejects_unknown_backends() {
    assert_eq!(canonical_backend("Sqlite").unwrap(), "sqlite");
    assert_eq!(canonical_backend("sqlite").unwrap(), "sqlite");
    for backend in ["", "postgres", "Postgres", "sqlite ", "sqlite://", "SQLITE"] {
        assert!(
            canonical_backend(backend).is_err(),
            "accepted backend {backend:?}"
        );
    }
}

#[test]
fn schema_identity_requires_an_explicit_version_and_two_complete_fingerprints() {
    let identity = SchemaIdentity::new("0123456789abcdef", "fedcba9876543210");
    let encoded = identity.encode();
    assert_eq!(SchemaIdentity::parse(&encoded).unwrap(), identity);
    for invalid in [
        "0123456789abcdef", // Legacy descriptor-only backups have no physical attestation.
        "librarian-schema-v2:0123456789abcdef:fedcba9876543210",
        "librarian-schema-v1:0123456789abcdef",
        "librarian-schema-v1::fedcba9876543210",
        "librarian-schema-v1:0123456789abcdeg:fedcba9876543210",
        "librarian-schema-v1:0123456789ABCDEF:fedcba9876543210",
        "librarian-schema-v1:0123456789abcdef:fedcba987654321",
        "librarian-schema-v1:0123456789abcdef:fedcba9876543210:trailing",
    ] {
        assert!(
            SchemaIdentity::parse(invalid).is_err(),
            "accepted identity {invalid:?}"
        );
    }
}

#[tokio::test]
async fn real_full_backup_matches_migration_source_hash_and_restores_all_setting_values() {
    let root = tempfile::tempdir().unwrap();
    let db = database().await;
    let row = setting(&db).await;
    let service = source(db.clone(), root.path());
    let snapshot = service.create_full_backup().await.unwrap().unwrap();
    service
        .verify_snapshot(&snapshot.snapshot_id)
        .await
        .unwrap();
    let manifest = load_manifest(
        &service.repository,
        Uuid::parse_str(&snapshot.snapshot_id).unwrap(),
    )
    .await
    .unwrap();
    let descriptors = db.schema_snapshot("test", &entity_metadata());
    assert_eq!(
        descriptors.backend, "Sqlite",
        "pinned ORM backend spelling changed"
    );
    assert_eq!(manifest.database_backend, "sqlite");
    let physical = DefaultBackend::introspect_schema(db.pool()).await.unwrap();
    let identity = SchemaIdentity::parse(&manifest.graphql_orm_schema_hash).unwrap();
    assert_eq!(identity.physical_schema_hash, physical.stable_hash());
    assert_eq!(identity.descriptor_schema_hash, descriptors.schema_hash);
    assert_ne!(
        identity.physical_schema_hash,
        identity.descriptor_schema_hash
    );

    // A real planner uses the live physical fingerprint, irrespective of the
    // candidate metadata. Prove that the snapshot authorizes this source, not
    // its unrelated descriptor hash or a different physical source.
    let mut renamed = physical.clone();
    renamed
        .tables
        .iter_mut()
        .find(|table| table.table_name == "app_settings")
        .unwrap()
        .columns
        .iter_mut()
        .find(|column| column.name == "description")
        .unwrap()
        .name = "candidate_description".into();
    let plan = db
        .schema()
        .plan_migration("candidate", "fixture rename", &physical, &renamed)
        .unwrap();
    assert!(!plan.steps.is_empty());
    let proof = verify_schema_migration_backup(
        root.path().join("backups"),
        &snapshot.snapshot_id,
        plan.source_schema_hash.as_deref().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(proof.snapshot_id(), snapshot.snapshot_id);
    assert!(
        verify_schema_migration_backup(
            root.path().join("backups"),
            &snapshot.snapshot_id,
            &descriptors.schema_hash
        )
        .await
        .is_err()
    );
    assert!(
        verify_schema_migration_backup(
            root.path().join("backups"),
            &snapshot.snapshot_id,
            &renamed.stable_hash()
        )
        .await
        .is_err()
    );
    verify_full_backup(root.path().join("backups"), &snapshot.snapshot_id)
        .await
        .unwrap();

    let target_db = database().await;
    let target = BackupService::new_for_offline_restore(
        target_db.clone(),
        ObjectStorageServiceConfig {
            backend: "local".into(),
            local_path: root.path().join("restored"),
        },
        BackupServiceConfig {
            repository_path: root.path().join("backups"),
        },
    )
    .unwrap();
    target
        .restore_full_backup(&snapshot.snapshot_id)
        .await
        .unwrap();
    let restored = AppSetting::get(target_db.pool(), &row.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(row).unwrap()
    );
}

#[tokio::test]
async fn restore_independently_rejects_physical_and_logical_descriptor_changes_before_writes() {
    let root = tempfile::tempdir().unwrap();
    let source_db = database().await;
    setting(&source_db).await;
    let source = source(source_db, root.path());
    let snapshot = source.create_full_backup().await.unwrap().unwrap();
    let original = load_manifest(
        &source.repository,
        Uuid::parse_str(&snapshot.snapshot_id).unwrap(),
    )
    .await
    .unwrap();
    let db = database().await;
    let adapter = LibrarianBackupDatabaseAdapter::new(db.clone(), "test".into());
    let target = adapter.schema_snapshot().await.unwrap();
    let target_identity = SchemaIdentity::parse(&target.schema_hash).unwrap();
    let mut descriptors = db.list_backup_entities(&entity_metadata());
    let setting = descriptors
        .iter_mut()
        .find(|entity| entity.table_name == "app_settings")
        .unwrap();
    setting
        .columns
        .iter_mut()
        .find(|column| column.column_name == "description")
        .unwrap()
        .backup_policy = ColumnBackupPolicy::Redact;
    let altered_descriptors = stable_schema_hash(&descriptors);
    assert_ne!(altered_descriptors, target_identity.descriptor_schema_hash);
    let mut altered_physical = DefaultBackend::introspect_schema(db.pool()).await.unwrap();
    altered_physical
        .tables
        .iter_mut()
        .find(|table| table.table_name == "app_settings")
        .unwrap()
        .columns
        .iter_mut()
        .find(|column| column.name == "description")
        .unwrap()
        .name = "different_source_column".into();
    let altered_physical = altered_physical.stable_hash();
    assert_ne!(altered_physical, target_identity.physical_schema_hash);

    for (case, incompatible) in [
        SchemaIdentity::new(&altered_physical, target_identity.descriptor_schema_hash).encode(),
        SchemaIdentity::new(target_identity.physical_schema_hash, &altered_descriptors).encode(),
        target_identity.descriptor_schema_hash.to_string(),
    ]
    .into_iter()
    .enumerate()
    {
        let error = adapter
            .restore_full(
                GraphqlOrmBackupSchema {
                    schema_hash: incompatible.clone(),
                    ..target.clone()
                },
                Vec::new(),
                RestoreContext::empty_database(),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(error, BackupError::RestoreSchemaMismatch { .. }),
            "{error}"
        );
        assert!(adapter.restore_target_is_empty().await.unwrap());

        // Also exercise the pinned orchestrator's dry-run gate: it never calls
        // adapter.restore_full, so the complete identity must survive there too.
        let mut altered = original.clone();
        altered.graphql_orm_schema_hash = incompatible;
        graphql_orm_backup::set_manifest_checksum(&mut altered).unwrap();
        source
            .repository
            .put_blob(
                &snapshot.manifest_key,
                Bytes::from(serde_json::to_vec(&altered).unwrap()),
            )
            .await
            .unwrap();
        let object_root = root.path().join(format!("rejected-{case}"));
        let recovery = BackupService::new_for_offline_restore(
            db.clone(),
            ObjectStorageServiceConfig {
                backend: "local".into(),
                local_path: object_root.clone(),
            },
            BackupServiceConfig {
                repository_path: root.path().join("backups"),
            },
        )
        .unwrap();
        let error = recovery
            .restore_full_backup(&snapshot.snapshot_id)
            .await
            .unwrap_err();
        assert!(
            format!("{error:#}").contains("restore schema is incompatible"),
            "{error:#}"
        );
        assert!(adapter.restore_target_is_empty().await.unwrap());
        assert!(
            !object_root.exists(),
            "incompatible recovery claimed or wrote its object target"
        );
    }
}

#[tokio::test]
async fn backup_refuses_unrepresented_live_columns_instead_of_attesting_to_omitted_data() {
    let root = tempfile::tempdir().unwrap();
    let db = database().await;
    setting(&db).await;
    let current = DefaultBackend::introspect_schema(db.pool()).await.unwrap();
    let mut drifted = current.clone();
    let table = drifted
        .tables
        .iter_mut()
        .find(|table| table.table_name == "app_settings")
        .unwrap();
    let mut column = table
        .columns
        .iter()
        .find(|column| column.name == "description")
        .unwrap()
        .clone();
    column.name = "unrepresented_source_data".into();
    table.columns.push(column);
    let plan = db
        .schema()
        .plan_migration(
            "fixture-extra-column",
            "fixture schema drift",
            &current,
            &drifted,
        )
        .unwrap();
    db.schema()
        .apply_migration(
            &plan,
            ApplyOptions {
                additive_only: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    // The descriptor digest has no knowledge of the added live column.
    let descriptors = db.schema_snapshot("test", &entity_metadata());
    assert_ne!(
        current.stable_hash(),
        DefaultBackend::introspect_schema(db.pool())
            .await
            .unwrap()
            .stable_hash()
    );
    let service = source(db.clone(), root.path());
    let error = service.create_full_backup().await.unwrap_err();
    assert!(
        format!("{error:#}").contains("does not match generated entity metadata"),
        "{error:#}"
    );
    assert!(service.list_snapshots().await.unwrap().is_empty());
    assert_eq!(
        descriptors.schema_hash,
        db.schema_snapshot("test", &entity_metadata()).schema_hash
    );
}

#[tokio::test]
async fn schema_identity_and_backend_remain_bound_to_manifest_checksum() {
    let root = tempfile::tempdir().unwrap();
    let service = source(database().await, root.path());
    let snapshot = service.create_full_backup().await.unwrap().unwrap();
    let original = load_manifest(
        &service.repository,
        Uuid::parse_str(&snapshot.snapshot_id).unwrap(),
    )
    .await
    .unwrap();
    let identity = SchemaIdentity::parse(&original.graphql_orm_schema_hash).unwrap();
    for mutate_backend in [false, true] {
        let mut tampered = original.clone();
        if mutate_backend {
            // Even a recognized backend alias may not bypass the original checksum.
            tampered.database_backend = "Sqlite".into();
        } else {
            tampered.graphql_orm_schema_hash =
                SchemaIdentity::new("0000000000000000", identity.descriptor_schema_hash).encode();
        }
        service
            .repository
            .put_blob(
                &snapshot.manifest_key,
                Bytes::from(serde_json::to_vec(&tampered).unwrap()),
            )
            .await
            .unwrap();
        assert!(
            verify_full_backup(root.path().join("backups"), &snapshot.snapshot_id)
                .await
                .is_err()
        );
        assert!(
            verify_schema_migration_backup(
                root.path().join("backups"),
                &snapshot.snapshot_id,
                identity.physical_schema_hash
            )
            .await
            .is_err()
        );
    }
}

#[tokio::test]
async fn legacy_descriptor_only_snapshot_cannot_authorize_a_physical_schema_migration() {
    let root = tempfile::tempdir().unwrap();
    let db = database().await;
    let service = source(db.clone(), root.path());
    let snapshot = service.create_full_backup().await.unwrap().unwrap();
    let mut legacy = load_manifest(
        &service.repository,
        Uuid::parse_str(&snapshot.snapshot_id).unwrap(),
    )
    .await
    .unwrap();
    legacy.database_backend = "Sqlite".into();
    legacy.graphql_orm_schema_hash = db.schema_snapshot("test", &entity_metadata()).schema_hash;
    // Model a genuine, checksum-valid old-format snapshot, not corrupt data.
    graphql_orm_backup::set_manifest_checksum(&mut legacy).unwrap();
    service
        .repository
        .put_blob(
            &snapshot.manifest_key,
            Bytes::from(serde_json::to_vec(&legacy).unwrap()),
        )
        .await
        .unwrap();
    verify_full_backup(root.path().join("backups"), &snapshot.snapshot_id)
        .await
        .unwrap();
    let error = verify_schema_migration_backup(
        root.path().join("backups"),
        &snapshot.snapshot_id,
        &DefaultBackend::introspect_schema(db.pool())
            .await
            .unwrap()
            .stable_hash(),
    )
    .await
    .expect_err("descriptor-only snapshot must not acquire a physical schema proof");
    assert!(
        error
            .to_string()
            .contains("no supported physical/descriptor schema identity"),
        "{error:#}"
    );
}
