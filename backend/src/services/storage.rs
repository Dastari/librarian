//! Object storage service used for file bytes that should not live in the database.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use graphql_orm_storage::{
    LocalStorageBackend, StorageBackend, StorageNamespace, StorageService, StoredObject,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::graphql::entities::StorageObject;
use crate::services::manager::{Service, ServiceHealth};

#[derive(Debug, Clone)]
pub struct ObjectStorageServiceConfig {
    pub backend: String,
    pub local_path: PathBuf,
}

#[derive(Clone)]
pub struct ObjectStorageService {
    storage: StorageService,
}

impl ObjectStorageService {
    pub fn new(config: ObjectStorageServiceConfig) -> Result<Self> {
        let backend = StorageBackend::from_str(&config.backend).with_context(|| {
            format!(
                "Unsupported object storage backend '{}'. Supported backend: local",
                config.backend
            )
        })?;

        let storage = match backend {
            StorageBackend::Local => {
                StorageService::new(Arc::new(LocalStorageBackend::new(config.local_path)))
            }
            StorageBackend::S3 | StorageBackend::AzureBlob | _ => {
                anyhow::bail!(
                    "Object storage backend '{}' is not supported by Librarian yet",
                    backend.as_str()
                );
            }
        };

        Ok(Self { storage })
    }

    pub fn storage(&self) -> &StorageService {
        &self.storage
    }

    pub fn stored_object_from_entity(&self, row: &StorageObject) -> Result<StoredObject> {
        Ok(StoredObject {
            object_id: Uuid::parse_str(&row.object_id)
                .with_context(|| format!("Invalid storage object_id '{}'", row.object_id))?,
            namespace: parse_namespace(&row.namespace)?,
            backend: StorageBackend::from_str(&row.backend)
                .with_context(|| format!("Invalid storage backend '{}'", row.backend))?,
            storage_key: row.storage_key.clone(),
            original_file_name: row.original_file_name.clone(),
            mime_type: row.mime_type.clone(),
            size_bytes: u64::try_from(row.size_bytes)
                .with_context(|| format!("Invalid storage size_bytes '{}'", row.size_bytes))?,
            sha256_hex: row.sha256_hex.clone(),
            created_at: parse_storage_timestamp(&row.created_at)?,
        })
    }
}

fn parse_storage_timestamp(value: &str) -> Result<OffsetDateTime> {
    // Entity date fields can contain Unix seconds from the ORM or RFC3339 values
    // from storage imports. Both describe the same metadata, without rewriting rows.
    let parsed = match value.parse::<i64>() {
        Ok(seconds) => OffsetDateTime::from_unix_timestamp(seconds).map_err(anyhow::Error::from),
        Err(_) => OffsetDateTime::parse(value, &Rfc3339).map_err(anyhow::Error::from),
    };
    parsed.with_context(|| format!("Invalid storage created_at '{value}'"))
}

fn parse_namespace(value: &str) -> Result<StorageNamespace> {
    match value.trim().to_ascii_lowercase().as_str() {
        "originals" => Ok(StorageNamespace::Originals),
        "recycle_bin" | "recycle-bin" => Ok(StorageNamespace::RecycleBin),
        "thumbnails" => Ok(StorageNamespace::Thumbnails),
        "derivatives" => Ok(StorageNamespace::Derivatives),
        "exports" => Ok(StorageNamespace::Exports),
        "temp" => Ok(StorageNamespace::Temp),
        other => anyhow::bail!("Unsupported storage namespace '{}'", other),
    }
}

#[async_trait]
impl Service for ObjectStorageService {
    fn name(&self) -> &str {
        "storage"
    }

    async fn start(&self) -> Result<()> {
        tracing::info!(
            service = "storage",
            backend = %self.storage.backend().as_str(),
            "Object storage service started"
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!(service = "storage", "Object storage service stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        Ok(ServiceHealth::healthy())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_storage_timestamp;

    #[test]
    fn reads_existing_unix_timestamp_artwork_metadata() {
        let created_at = parse_storage_timestamp("1785390864").unwrap();
        assert_eq!(created_at.unix_timestamp(), 1_785_390_864);
    }

    #[test]
    fn preserves_rfc3339_storage_timestamps() {
        let created_at = parse_storage_timestamp("2026-07-30T12:00:00+02:00").unwrap();
        assert_eq!(created_at.unix_timestamp(), 1_785_405_600);
    }

    #[test]
    fn rejects_corrupt_or_out_of_range_storage_timestamps() {
        for value in ["", "not-a-date", "9223372036854775807"] {
            assert!(parse_storage_timestamp(value).is_err(), "accepted {value}");
        }
    }
}
