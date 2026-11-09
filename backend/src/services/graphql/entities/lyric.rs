//! Lyric Entity: the lyrics found for a track's file, cached.
//!
//! One row per media file, written only by the lyrics service (`services::lyrics`) through the
//! generated mutations. `source` says where they came from: `sidecar` (an `.lrc` beside the
//! file), `embedded` (USLT/SYLT, `LYRICS`, `©lyr`), `lrclib`, or `none` when nothing was found
//! (kept so a miss is not asked again for a while). See `docs/design.md` Q121.

use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    async_graphql::SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(complex)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "lyrics",
    plural = "Lyrics",
    default_sort = "created_at",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct Lyric {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub media_file_id: String,

    /// The track the file plays, when it is linked to one.
    #[graphql(name = "trackId")]
    #[filterable(type = "string")]
    pub track_id: Option<String>,

    /// `sidecar`, `embedded`, `lrclib` or `none`.
    #[graphql(name = "source")]
    #[filterable(type = "string")]
    pub source: String,

    /// Whether `syncedLyrics` holds timed lines.
    #[graphql(name = "synced")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub synced: bool,

    /// Timed lines, normalised to `[mm:ss.cc]text`, one per line, sorted.
    #[graphql(name = "syncedLyrics")]
    pub synced_lyrics: Option<String>,

    /// The words without times.
    #[graphql(name = "plainLyrics")]
    pub plain_lyrics: Option<String>,

    /// The language, when the source says.
    #[graphql(name = "language")]
    pub language: Option<String>,

    /// The source says the track has no words.
    #[graphql(name = "instrumental")]
    #[boolean_field]
    pub instrumental: bool,

    /// The provider's id for what was found (LRCLIB's record id).
    #[graphql(name = "providerId")]
    pub provider_id: Option<String>,

    #[graphql(name = "fetchedAt")]
    #[filterable(type = "date")]
    pub fetched_at: String,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    // Deleted with its file by `cleanup_media_file_dependents`.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "MediaFile",
        from = "media_file_id",
        to = "id",
        on_delete = "cascade",
        emit_fk = false
    )]
    pub media_file: Option<Box<MediaFile>>,
}

#[derive(Default)]
pub struct LyricCustomOperations;
