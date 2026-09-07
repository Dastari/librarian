//! ReleaseBlocklist Entity
//!
//! Releases the auto-download hunt must never grab again. The retry sweep
//! (`jobs::download_monitor`) adds an entry whenever a grabbed torrent ends up
//! `failed`, or stalls at zero progress for more than a day, and then re-hunts
//! the wanted item so a different release can be picked. `jobs::auto_download`
//! drops any candidate whose `infoHash` or `guid` is blocklisted.
//!
//! Entries are keyed by `infoHash` when the source reported one and by `guid`
//! otherwise (usenet/RSS releases have no info hash). `expiresAt` allows a
//! temporary block; a null value blocks forever.

use crate::graphql::entities::*;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "release_blocklists",
    plural = "ReleaseBlocklists",
    default_sort = "created_at",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct ReleaseBlocklist {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    /// Torrent info hash of the blocked release, when the source reported one.
    #[graphql(name = "infoHash")]
    #[filterable(type = "string")]
    pub info_hash: Option<String>,

    /// Source-specific release identifier (usually the details URL). Used when
    /// there is no info hash, and as a second matching key when there is.
    #[graphql(name = "guid")]
    #[filterable(type = "string")]
    pub guid: Option<String>,

    /// Release title, kept for the UI and for log lines.
    #[graphql(name = "title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    /// The `Source` the release came from, when known.
    #[graphql(name = "sourceId")]
    #[filterable(type = "string")]
    pub source_id: Option<String>,

    /// Why the release was blocked, e.g. `import_failed` or `stalled`.
    #[graphql(name = "reason")]
    #[filterable(type = "string")]
    pub reason: String,

    /// Wanted-target the blocked release was grabbed for, so the UI can show
    /// "blocked releases" per item and the sweep knows what to re-hunt.
    #[graphql(name = "showId")]
    #[filterable(type = "string")]
    pub show_id: Option<String>,

    #[graphql(name = "movieId")]
    #[filterable(type = "string")]
    pub movie_id: Option<String>,

    #[graphql(name = "albumId")]
    #[filterable(type = "string")]
    pub album_id: Option<String>,

    #[graphql(name = "audiobookId")]
    #[filterable(type = "string")]
    pub audiobook_id: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    /// When the block lapses. Null blocks the release permanently.
    #[graphql(name = "expiresAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub expires_at: Option<String>,
}

#[derive(Default)]
pub struct ReleaseBlocklistCustomOperations;
