//! TorrentContents Entity
//!
//! What a torrent holds, read *before* it is grabbed: its file list (names and sizes), total size
//! and piece layout, with what each file means (which episode, which track, a sample, an extra).
//! `services::torrent::inspect` writes these; the hunt and the recommender read them to judge a
//! release by its files rather than its name (Q99).
//!
//! A torrent's metadata is immutable (the info hash is a hash of it), so a row is cached forever,
//! keyed by info hash. `infoBytes` keeps the bencoded info dictionary of a public torrent so a
//! later grab of a magnet does not have to ask peers for it again; it is never kept for a private
//! torrent, whose `.torrent` is fetched from its tracker at grab time anyway. The info dictionary
//! never holds a tracker address, so no passkey is stored here; `trackers` keeps a public torrent's
//! announce addresses, never a private one's.

use crate::graphql::entities::*;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "torrent_contents",
    plural = "TorrentContentsList",
    default_sort = "fetched_at",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct TorrentContents {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    /// Lowercase hex v1 info hash.
    #[graphql(name = "infoHash")]
    #[unique]
    #[filterable(type = "string")]
    pub info_hash: String,

    /// A hash of the release's guid or link at the source it was read from, so a `.torrent` link
    /// with no info hash in its listing is not fetched twice. A hash, because a feed's link holds
    /// the account's passkey.
    #[graphql(name = "linkKey")]
    #[filterable(type = "string")]
    pub link_key: Option<String>,

    /// The torrent's own name (its top folder, or its only file).
    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "totalSize")]
    #[filterable(type = "number")]
    #[sortable]
    pub total_size: i64,

    #[graphql(name = "fileCount")]
    #[filterable(type = "number")]
    pub file_count: i32,

    #[graphql(name = "pieceLength")]
    pub piece_length: i32,

    #[graphql(name = "pieceCount")]
    pub piece_count: i32,

    /// The torrent is marked private: its peers come only from its tracker.
    #[graphql(name = "isPrivate")]
    #[filterable(type = "boolean")]
    pub is_private: bool,

    /// How it was read: `torrent_file` (a `.torrent` parsed locally, no peers asked) or `magnet`
    /// (metadata asked of peers in list-only mode, nothing downloaded).
    #[graphql(name = "via")]
    #[filterable(type = "string")]
    pub via: String,

    /// The files, in torrent order, as a JSON array of `{index, path, size, kind, season,
    /// episodes, track, disc}` (`services::torrent::inspect::ContentFile`). Padding files are left
    /// out; `index` is the file's index in the torrent, which file selection uses.
    #[graphql(name = "files")]
    pub files: String,

    /// Base64 of the bencoded info dictionary, public torrents only.
    #[graphql(name = "infoBytes")]
    pub info_bytes: Option<String>,

    /// A public torrent's tracker addresses, as a JSON array, so a grab from `infoBytes` can be
    /// given its trackers and its `Torrent` row a magnet to restore from (Q111). Never kept for a
    /// private torrent, whose announce address holds the account's passkey.
    #[graphql(name = "trackers")]
    pub trackers: Option<String>,

    #[graphql(name = "fetchedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub fetched_at: String,

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
pub struct TorrentContentsCustomOperations;
