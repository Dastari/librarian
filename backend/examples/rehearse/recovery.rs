//! Offline runtime rehearsal, linked into an isolated copy of the real library.
//! See scripts/rehearsal-recovery.sh. Never compile with schema-export-only.
#[cfg(feature = "schema-export-only")]
compile_error!("Recovery rehearsal requires real generated runtime bodies");

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use base64::Engine;
use graphql_orm::graphql::orm::{DefaultBackend, IntrospectionBackend};
use graphql_orm_backup::{LocalBackupRepository, load_manifest};
use graphql_orm_storage::{StorageNamespace, StoragePutRequest};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::cli::{BackupRestoreCommand, SchemaCommand};
use crate::config::Config;
use crate::db::Database;
use crate::graphql::entities::*;
use crate::services::backup::{BackupService, BackupServiceConfig};
use crate::services::database::{self, DatabaseService};
use crate::services::storage::{ObjectStorageService, ObjectStorageServiceConfig};

#[path = "fixtures.rs"]
mod fixtures;

const MARKER: &str = ".librarian-restore-in-progress";
const SENTINEL: &str = "source-schema description must survive rollback";

fn write_json(path: impl AsRef<Path>, value: &Value) -> Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn read_json(path: impl AsRef<Path>) -> Result<Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn snapshot_id(root: &Path) -> Result<String> {
    Ok(read_json(root.join("snapshot.json"))?["snapshotId"]
        .as_str()
        .context("missing snapshot id")?
        .to_string())
}

fn storage_config(root: PathBuf) -> ObjectStorageServiceConfig {
    ObjectStorageServiceConfig {
        backend: "local".into(),
        local_path: root,
    }
}

fn repository_config(root: &Path) -> BackupServiceConfig {
    BackupServiceConfig {
        repository_path: root.join("backups"),
    }
}

async fn open_database(path: &Path, fresh: bool) -> Result<Database> {
    if fresh {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
    }
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(path)
                .foreign_keys(true)
                .create_if_missing(false),
        )
        .await?;
    let mut db = Database::new(pool);
    // Strict declared policies also apply to trusted repository operations.
    // Match the application's providers while deliberately omitting lifecycle
    // hooks so fixture creation can model persisted pre-upgrade summary drift.
    db.set_entity_policy(crate::services::graphql::policy::AppEntityPolicy);
    db.set_field_policy(crate::services::graphql::policy::AppFieldPolicy);
    db.set_row_policy(crate::services::graphql::row_policy::OwnershipRowPolicy);
    if fresh {
        database::bootstrap_schema(&db).await?;
    }
    Ok(db)
}

fn backup(db: Database, root: &Path) -> Result<BackupService> {
    Ok(BackupService::new(
        db,
        Arc::new(ObjectStorageService::new(storage_config(
            root.join("objects"),
        ))?),
        repository_config(root),
    ))
}

fn offline(db: Database, root: &Path, target: &str) -> Result<BackupService> {
    BackupService::new_for_offline_restore(
        db,
        storage_config(root.join(format!("{target}-objects"))),
        repository_config(root),
    )
}

fn config(root: &Path) -> Result<Config> {
    // The script runs with a clean environment; only these two offline paths are used.
    let mut config = Config::from_env()?;
    config.database_url = format!("sqlite://{}", root.join("source.db").display());
    config.backup_path = root.join("backups").to_string_lossy().into_owned();
    Ok(config)
}

async fn restore_cli(root: &Path, target: &str) -> Result<()> {
    crate::run_backup_restore(
        &config(root)?,
        BackupRestoreCommand {
            snapshot_id: snapshot_id(root)?,
            database_path: root
                .join(format!("{target}.db"))
                .to_string_lossy()
                .into_owned(),
            object_root: root
                .join(format!("{target}-objects"))
                .to_string_lossy()
                .into_owned(),
        },
    )
    .await
}

async fn live_hash(db: &Database) -> Result<String> {
    Ok(DefaultBackend::introspect_schema(db.pool())
        .await?
        .stable_hash())
}

/// Compare every seeded row, including IDs, dates, private fields and object/artwork links.
/// The other application tables are empty in this deliberately small fixture.
async fn rows(db: &Database) -> Result<Value> {
    let mut output = serde_json::Map::new();
    macro_rules! capture {
        ($($entity:ident),+ $(,)?) => { $(
            let mut values = $entity::query(db.pool()).fetch_all().await?
                .into_iter().map(serde_json::to_value).collect::<serde_json::Result<Vec<_>>>()?;
            values.sort_by_key(|value| value["Id"].as_str().unwrap_or_default().to_string());
            output.insert(stringify!($entity).into(), Value::Array(values));
        )+ };
    }
    capture!(
        User,
        Library,
        Artist,
        Album,
        Track,
        Audiobook,
        Chapter,
        MediaFile,
        AppSetting,
        StorageObject,
        ArtworkCache
    );
    Ok(Value::Object(output))
}

async fn seed(root: &Path) -> Result<()> {
    let db = open_database(&root.join("source.db"), true).await?;
    // Generated typed inserts with no lifecycle hooks simulate persisted pre-upgrade drift.
    // No alternate schema, SQL, resolver stubs, services or workers are used.
    let owner = fixtures::user(&db).await;
    let library = fixtures::library(&db, &owner).await;
    let artist = fixtures::artist(&db, &library).await;
    let album = fixtures::album(&db, &library, &artist).await;
    let book = fixtures::audiobook(&db, &library).await;
    let audio = fixtures::media_file(&db, &library, 17, None, None).await;
    let narration = fixtures::media_file(&db, &library, 23, None, None).await;
    for _ in 0..2 {
        let song = fixtures::track(&db, &album, Some(&audio.id)).await;
        Track::update_by_id(
            &db,
            &song.id,
            UpdateTrackInput {
                artist_id: Some(Some(artist.id.clone())),
                duration_secs: Some(Some(13)),
                ..Default::default()
            },
        )
        .await?;
        fixtures::chapter(&db, &book, Some(&narration.id)).await;
    }
    AppSetting::insert(
        &db,
        CreateAppSettingInput {
            key: "rehearsal-sentinel".into(),
            value: "before-candidate".into(),
            description: Some(SENTINEL.into()),
            category: "rehearsal".into(),
        },
    )
    .await?;

    let storage = ObjectStorageService::new(storage_config(root.join("objects")))?;
    let image = base64::engine::general_purpose::STANDARD.decode(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/lXcAAAAASUVORK5CYII=")?;
    let mut objects = Vec::new();
    for (namespace, name, mime, bytes) in [
        (
            StorageNamespace::Thumbnails,
            "poster.png",
            "image/png",
            image,
        ),
        (
            StorageNamespace::Originals,
            "managed.bin",
            "application/octet-stream",
            b"managed\0payload\xff\x01".to_vec(),
        ),
        (
            StorageNamespace::Exports,
            "empty.bin",
            "application/octet-stream",
            Vec::new(),
        ),
    ] {
        let stored = storage
            .storage()
            .put_object(StoragePutRequest {
                namespace,
                file_name: Some(name.into()),
                mime_type: Some(mime.into()),
                bytes: bytes.clone(),
            })
            .await?;
        let row = StorageObject::insert(
            &db,
            CreateStorageObjectInput {
                object_id: stored.object_id.to_string(),
                namespace: stored.namespace.as_str().into(),
                backend: stored.backend.as_str().into(),
                storage_key: stored.storage_key.clone(),
                original_file_name: stored.original_file_name,
                mime_type: stored.mime_type,
                size_bytes: i64::try_from(stored.size_bytes)?,
                sha256_hex: stored.sha256_hex,
            },
        )
        .await?;
        if namespace == StorageNamespace::Thumbnails {
            ArtworkCache::insert(
                &db,
                CreateArtworkCacheInput {
                    entity_type: "album".into(),
                    entity_id: album.id.clone(),
                    artwork_type: "cover".into(),
                    storage_object_id: row.id.clone(),
                    source_url: None,
                    width: Some(1),
                    height: Some(1),
                },
            )
            .await?;
        }
        objects.push(json!({"id": row.id, "key": row.storage_key, "hex": hex::encode(bytes)}));
    }
    write_json(root.join("expected-objects.json"), &json!(objects))?;
    write_json(root.join("before-summary.json"), &rows(&db).await?)?;
    db.pool().close().await;
    Ok(())
}

async fn summaries(root: &Path) -> Result<()> {
    let raw = open_database(&root.join("source.db"), false).await?;
    let db = DatabaseService::new(raw).pool().clone();
    let before = rows(&db).await?;
    let preview = database::reconcile_stored_media_summaries(&db, true).await?;
    ensure!(
        preview.dry_run
            && preview.artists_changed == 1
            && preview.albums_changed == 1
            && preview.audiobooks_changed == 1,
        "dry run must identify all three stale parents: {preview:?}"
    );
    ensure!(
        rows(&db).await? == before,
        "summary dry run changed persisted rows"
    );
    let applied = database::reconcile_stored_media_summaries(&db, false).await?;
    ensure!(
        !applied.dry_run
            && applied.artists_changed == 1
            && applied.albums_changed == 1
            && applied.audiobooks_changed == 1,
        "repair report disagrees with dry run: {applied:?}"
    );
    let artist = Artist::query(db.pool()).fetch_all().await?.remove(0);
    ensure!(
        (
            artist.album_count,
            artist.track_count,
            artist.total_duration_secs
        ) == (Some(1), Some(2), Some(26)),
        "incorrect explicit Artist membership/duration summary"
    );
    let album = Album::query(db.pool()).fetch_all().await?.remove(0);
    let book = Audiobook::query(db.pool()).fetch_all().await?.remove(0);
    ensure!(
        album.has_files && album.size_bytes == Some(17) && album.track_count == Some(99),
        "album must deduplicate file bytes and preserve provider track count"
    );
    ensure!(
        book.has_files && book.size_bytes == Some(23) && book.chapter_count == Some(99),
        "audiobook must deduplicate file bytes and preserve provider chapter count"
    );
    let repaired = rows(&db).await?;
    let repeated = database::reconcile_stored_media_summaries(&db, false).await?;
    let final_preview = database::reconcile_stored_media_summaries(&db, true).await?;
    for report in [&repeated, &final_preview] {
        ensure!(
            report.artists_changed == 0
                && report.albums_changed == 0
                && report.audiobooks_changed == 0,
            "reconciliation was not idempotent: {report:?}"
        );
    }
    ensure!(
        rows(&db).await? == repaired,
        "idempotent repair changed persisted rows"
    );
    write_json(root.join("baseline.json"), &repaired)?;
    std::fs::write(
        root.join("summary-report.txt"),
        format!(
            "dryRun={preview:?}\nrepair={applied:?}\nrepeat={repeated:?}\nfinalDryRun={final_preview:?}\n"
        ),
    )?;
    db.pool().close().await;
    Ok(())
}

async fn create_snapshot(root: &Path) -> Result<()> {
    let db = open_database(&root.join("source.db"), false).await?;
    let service = backup(db.clone(), root)?;
    ensure!(
        !service.capabilities().restore_available,
        "live backup advertised restore"
    );
    let snapshot = service
        .create_full_backup()
        .await?
        .context("full backup unavailable")?;
    ensure!(
        snapshot.object_count == 3,
        "full backup omitted managed/artwork/empty bytes"
    );
    service.verify_snapshot(&snapshot.snapshot_id).await?;
    let manifest = load_manifest(
        &LocalBackupRepository::new(root.join("backups")),
        uuid::Uuid::parse_str(&snapshot.snapshot_id)?,
    )
    .await?;
    // Reporting only: the real verifier/restore implementation owns identity validation.
    let identity = manifest
        .graphql_orm_schema_hash
        .split(':')
        .collect::<Vec<_>>();
    ensure!(
        identity.len() == 3 && identity[0] == "librarian-schema-v1",
        "backup did not emit a versioned physical/descriptor schema identity"
    );
    let physical_hash = live_hash(&db).await?;
    ensure!(
        identity[1] == physical_hash,
        "backup physical component differs from the live schema"
    );
    write_json(
        root.join("snapshot.json"),
        &json!({
            "snapshotId": snapshot.snapshot_id, "manifestKey": snapshot.manifest_key,
            "manifestSchemaHash": manifest.graphql_orm_schema_hash,
            "physicalSchemaHash": identity[1], "descriptorSchemaHash": identity[2],
            "manifestBackend": manifest.database_backend, "liveSchemaHash": physical_hash,
            "objectCount": manifest.objects.len(), "rowCount": manifest.database.row_count,
        }),
    )?;
    db.pool().close().await;
    Ok(())
}

async fn verify(root: &Path, target: &str) -> Result<()> {
    let db = open_database(&root.join(format!("{target}.db")), false).await?;
    ensure!(
        database::plan_schema(&db).await?.is_none(),
        "restored schema has pending migrations"
    );
    ensure!(
        rows(&db).await? == read_json(root.join("baseline.json"))?,
        "restored rows differ from pre-migration baseline"
    );
    let object_root = root.join(format!("{target}-objects"));
    ensure!(
        !object_root.join(MARKER).exists(),
        "restore did not finalize persistent marker"
    );
    let storage = ObjectStorageService::new(storage_config(object_root.clone()))?;
    let expected = read_json(root.join("expected-objects.json"))?;
    for item in expected.as_array().context("expected object array")? {
        let row = StorageObject::get(
            db.pool(),
            &item["id"].as_str().context("object id")?.to_string(),
        )
        .await?
        .context("missing object metadata")?;
        ensure!(
            row.storage_key == item["key"].as_str().context("object key")?,
            "object identity changed"
        );
        let expected = hex::decode(item["hex"].as_str().context("object bytes")?)?;
        let stored = storage.stored_object_from_entity(&row)?;
        ensure!(
            storage.storage().get_object(&stored).await?.bytes == expected,
            "application storage read differs byte-for-byte for {}",
            row.storage_key
        );
        ensure!(
            std::fs::read(object_root.join(&row.storage_key))? == expected,
            "filesystem bytes differ for {}",
            row.storage_key
        );
        ensure!(
            row.sha256_hex == hex::encode(Sha256::digest(&expected)),
            "object checksum differs"
        );
    }
    let artwork = ArtworkCache::query(db.pool()).fetch_all().await?;
    ensure!(artwork.len() == 1, "artwork reference missing");
    let poster = StorageObject::get(db.pool(), &artwork[0].storage_object_id)
        .await?
        .context("artwork storage link broken")?;
    let stored = storage.stored_object_from_entity(&poster)?;
    ensure!(
        storage
            .storage()
            .get_object(&stored)
            .await?
            .bytes
            .starts_with(b"\x89PNG\r\n\x1a\n"),
        "artwork consumer did not resolve the restored PNG"
    );
    db.pool().close().await;
    Ok(())
}

async fn partial_failure(root: &Path) -> Result<()> {
    let db = open_database(&root.join("partial.db"), true).await?;
    let mut service = offline(db.clone(), root, "partial")?;
    crate::services::backup::inject_second_object_failure(&mut service)?;
    let error = service
        .restore_full_backup(&snapshot_id(root)?)
        .await
        .expect_err("second object write must fail");
    ensure!(
        format!("{error:#}").contains("rehearsal injected second-object write failure"),
        "restore failed before reaching injected partial write: {error:#}"
    );
    ensure!(
        root.join("partial-objects").join(MARKER).exists(),
        "partial restore lost persistent marker"
    );
    ensure!(
        StorageObject::query(db.pool()).fetch_all().await?.len() == 3,
        "fault did not occur after real row restore"
    );
    let expected = read_json(root.join("expected-objects.json"))?;
    let count = expected
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| {
            root.join("partial-objects")
                .join(item["key"].as_str().unwrap())
                .is_file()
        })
        .count();
    ensure!(
        count == 1,
        "expected one real object before injected failure, got {count}"
    );
    write_json(root.join("partial-baseline.json"), &rows(&db).await?)?;
    std::fs::write(root.join("partial-error.txt"), format!("{error:#}\n"))?;
    ensure!(
        service
            .restore_full_backup(&snapshot_id(root)?)
            .await
            .unwrap_err()
            .to_string()
            .contains("already used"),
        "in-process target reuse was accepted"
    );
    db.pool().close().await;
    Ok(())
}

async fn restart_rejection(root: &Path) -> Result<()> {
    let error = restore_cli(root, "partial")
        .await
        .expect_err("CLI must refuse existing failed targets");
    ensure!(
        format!("{error:#}").contains("existing targets are refused"),
        "unexpected CLI rejection: {error:#}"
    );
    let db = open_database(&root.join("partial.db"), false).await?;
    let before_objects = object_digest(root.join("partial-objects"))?;
    let service = offline(db.clone(), root, "partial")?;
    let error = service
        .restore_full_backup(&snapshot_id(root)?)
        .await
        .expect_err("new process must refuse persistent partial staging");
    ensure!(
        format!("{error:#}").contains("Object restore target is not empty"),
        "unexpected rejection: {error:#}"
    );
    ensure!(
        rows(&db).await? == read_json(root.join("partial-baseline.json"))?,
        "restart retry changed rows"
    );
    ensure!(
        object_digest(root.join("partial-objects"))? == before_objects,
        "restart retry changed bytes"
    );
    db.pool().close().await;
    Ok(())
}

fn object_digest(root: PathBuf) -> Result<Value> {
    let mut values = serde_json::Map::new();
    for entry in walkdir::WalkDir::new(&root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            values.insert(
                entry
                    .path()
                    .strip_prefix(&root)?
                    .to_string_lossy()
                    .into_owned(),
                json!(hex::encode(Sha256::digest(std::fs::read(entry.path())?))),
            );
        }
    }
    Ok(Value::Object(values))
}

async fn plan(root: &Path) -> Result<()> {
    let db = open_database(&root.join("source.db"), false).await?;
    let before = live_hash(&db).await?;
    let plan = database::plan_schema(&db)
        .await?
        .context("synthetic rename must produce a plan")?;
    ensure!(
        database::schema_plan_class(&plan) == database::SchemaChangeClass::Destructive,
        "rename-as-drop/add was not classified destructive"
    );
    std::fs::write(
        root.join("migration-plan.txt"),
        database::format_schema_plan(&plan),
    )?;
    write_json(
        root.join("reviewed-plan.json"),
        &json!({"planHash": plan.plan_hash,
        "sourceSchemaHash": plan.source_schema_hash, "targetSchemaHash": plan.target_schema_hash}),
    )?;
    let error = database::bootstrap_schema(&db)
        .await
        .expect_err("ordinary startup must block this plan");
    ensure!(
        format!("{error:#}").contains("ordinary startup only applies additive"),
        "unexpected gate: {error:#}"
    );
    ensure!(
        live_hash(&db).await? == before,
        "blocked startup changed live schema"
    );
    std::fs::write(root.join("startup-rejection.txt"), format!("{error:#}\n"))?;
    db.pool().close().await;
    Ok(())
}

async fn migrate(root: &Path) -> Result<()> {
    let reviewed = read_json(root.join("reviewed-plan.json"))?;
    // Exact CLI administration implementation: verified backup, replan/hash check and apply.
    // Never fabricate VerifiedSchemaBackup or patch manifest/hash checks to make this pass.
    crate::run_schema_command(
        &config(root)?,
        SchemaCommand::Apply {
            plan_hash: reviewed["planHash"]
                .as_str()
                .context("reviewed plan hash")?
                .into(),
            backup_snapshot_id: snapshot_id(root)?,
        },
    )
    .await?;
    let db = open_database(&root.join("source.db"), false).await?;
    ensure!(
        database::plan_schema(&db).await?.is_none(),
        "candidate migration left pending drift"
    );
    let row = AppSetting::query(db.pool()).fetch_all().await?.remove(0);
    ensure!(
        row.description.is_none(),
        "fixture-only destructive rename did not demonstrate the lost value"
    );
    AppSetting::update_by_id(
        &db,
        &row.id,
        UpdateAppSettingInput {
            value: Some("candidate wrote after migration".into()),
            ..Default::default()
        },
    )
    .await?;
    db.pool().close().await;
    Ok(())
}

async fn incompatible_recovery(root: &Path) -> Result<()> {
    let error = restore_cli(root, "incompatible")
        .await
        .expect_err("candidate must not silently convert source-schema snapshots");
    ensure!(
        format!("{error:#}").to_lowercase().contains("schema"),
        "unexpected failure: {error:#}"
    );
    let db = open_database(&root.join("incompatible.db"), false).await?;
    ensure!(
        rows(&db)
            .await?
            .as_object()
            .unwrap()
            .values()
            .all(|v| v.as_array().unwrap().is_empty()),
        "incompatible restore imported domain rows"
    );
    ensure!(
        std::fs::read_dir(root.join("incompatible-objects"))?
            .next()
            .is_none(),
        "incompatible restore wrote object bytes"
    );
    std::fs::write(
        root.join("incompatible-recovery.txt"),
        format!("{error:#}\n"),
    )?;
    db.pool().close().await;
    Ok(())
}

/// Run exactly one phase per process; callers must retain the fresh staging directory.
pub async fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let action = args.next().context("phase required")?;
    let root = PathBuf::from(
        args.next()
            .context("absolute rehearsal state root required")?,
    );
    ensure!(
        root.is_absolute() && root.is_dir(),
        "state root must be an existing absolute directory"
    );
    let target = args.next();
    match action.as_str() {
        "seed" => seed(&root).await,
        "summaries" => summaries(&root).await,
        "backup" => create_snapshot(&root).await,
        "restore" => {
            restore_cli(
                &root,
                target.as_deref().context("fresh target name required")?,
            )
            .await
        }
        "verify" => verify(&root, target.as_deref().context("target name required")?).await,
        "partial-failure" => partial_failure(&root).await,
        "restart-rejection" => restart_rejection(&root).await,
        "plan" => plan(&root).await,
        "migrate" => migrate(&root).await,
        "incompatible-recovery" => incompatible_recovery(&root).await,
        _ => anyhow::bail!("unknown rehearsal phase {action}"),
    }
    .with_context(|| format!("rehearsal phase {action} under {}", root.display()))
}
