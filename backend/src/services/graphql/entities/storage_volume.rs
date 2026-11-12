use crate::graphql::entities::*;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

/// One disk or share that library folders, the download folder or Librarian's own data live on,
/// with its latest free-space reading (`docs/design.md` Q146).
///
/// Several libraries usually share a volume (every library on `smb://nas/media` does), so the
/// reading is kept once here and each `Library.storageVolumeId` points at it. Written only by
/// the capacity job (`jobs::storage_capacity`), and only when the reading moves meaningfully,
/// so subscriptions stay quiet.
#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "storage_volumes",
    plural = "StorageVolumes",
    default_sort = "label",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct StorageVolume {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    /// What identifies the volume: `smb://host/share` for a share, `local:<device id>` for a disk
    /// on this machine.
    #[graphql(name = "volumeKey")]
    #[filterable(type = "string")]
    pub volume_key: String,

    /// `local` or `smb`.
    #[graphql(name = "kind")]
    #[filterable(type = "string")]
    pub kind: String,

    /// What a person calls it: "media on 10.0.1.190", or the folder measured on this machine.
    #[graphql(name = "label")]
    #[filterable(type = "string")]
    #[sortable]
    pub label: String,

    /// `ok`, or `unavailable` when the share or folder could not be read; the byte counts then
    /// keep the last good reading.
    #[graphql(name = "status")]
    #[filterable(type = "string")]
    pub status: String,

    /// `normal`, `warning` or `critical`, with hysteresis: a level is left only once the space is
    /// clearly back above its threshold.
    #[graphql(name = "level")]
    #[filterable(type = "string")]
    #[sortable]
    pub level: String,

    #[graphql(name = "totalBytes")]
    pub total_bytes: Option<i64>,

    /// What Librarian may still write, after quotas and reserved blocks.
    #[graphql(name = "availableBytes")]
    #[sortable]
    pub available_bytes: Option<i64>,

    /// Free on the volume as a whole; more than `availableBytes` under a quota.
    #[graphql(name = "freeBytes")]
    pub free_bytes: Option<i64>,

    /// Why the last reading failed, in words a person can act on.
    #[graphql(name = "detail")]
    pub detail: Option<String>,

    /// Whether the torrent download folder is on this volume.
    #[graphql(name = "holdsDownloads")]
    #[filterable(type = "boolean")]
    pub holds_downloads: bool,

    /// Whether Librarian's own data and cache folder is on this volume.
    #[graphql(name = "holdsAppData")]
    #[filterable(type = "boolean")]
    pub holds_app_data: bool,

    /// The names of the libraries on it, as a JSON list, for alerts and tooltips.
    #[graphql(name = "libraryNames")]
    pub library_names: Option<String>,

    /// When the stored reading was taken.
    #[graphql(name = "checkedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub checked_at: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

#[derive(Default)]
pub struct StorageVolumeCustomOperations;
