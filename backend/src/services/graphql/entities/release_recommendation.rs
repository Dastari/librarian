//! ReleaseRecommendation Entity
//!
//! The best release for something that is missing, worked out ahead of time and shown as
//! "Recommended" with a one-press Download. `jobs::recommend` writes these; clients only read them
//! (and grab one through `grabRecommendation`).
//!
//! A row is one answer for one target at one scope: a movie, an episode, a season pack, a
//! complete-series pack, an album, a single track or a whole book. A show's answer is a coverage
//! plan (scope `plan`): one row for the plan, and one per release in it, linked by `planId`. `status` is `ready` (a release
//! was chosen), `searching` (a lookup is queued or running) or `none` (looked, nothing usable, so
//! the lookup is not repeated before `expiresAt`). Rows expire after about a day, vanish when the
//! release is blocklisted or gone from its feed, and when the target is no longer missing.
//!
//! The chosen release is the one automatic downloading would take (`jobs::auto_download::
//! pick_for_candidate`), preferring the smallest at the profile's best quality. Its download link can
//! carry a tracker passkey, so it is stored encrypted (the write transform) and never readable
//! through GraphQL: a feed release is re-resolved from its catalogue item, an indexer release uses the
//! stored link.

use crate::graphql::entities::*;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "release_recommendations",
    plural = "ReleaseRecommendations",
    default_sort = "found_at",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct ReleaseRecommendation {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    /// The library's owner: recommendations belong to them.
    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    pub library_id: String,

    /// `episode`, `season_pack`, `series_pack`, `movie`, `album`, `track` or `book`.
    #[graphql(name = "scope")]
    #[filterable(type = "string")]
    pub scope: String,

    /// `ready`, `searching` or `none`.
    #[graphql(name = "status")]
    #[filterable(type = "string")]
    pub status: String,

    /// What the lookup is for, as a person would say it: `Fallout S01`, `Heat (1995)`.
    #[graphql(name = "targetLabel")]
    #[filterable(type = "string")]
    pub target_label: String,

    #[graphql(name = "showId")]
    #[filterable(type = "string")]
    pub show_id: Option<String>,

    /// The season a season-pack or episode recommendation is for.
    #[graphql(name = "season")]
    #[filterable(type = "number")]
    pub season: Option<i32>,

    #[graphql(name = "episodeId")]
    #[filterable(type = "string")]
    pub episode_id: Option<String>,

    #[graphql(name = "movieId")]
    #[filterable(type = "string")]
    pub movie_id: Option<String>,

    #[graphql(name = "albumId")]
    #[filterable(type = "string")]
    pub album_id: Option<String>,

    #[graphql(name = "trackId")]
    #[filterable(type = "string")]
    pub track_id: Option<String>,

    #[graphql(name = "audiobookId")]
    #[filterable(type = "string")]
    pub audiobook_id: Option<String>,

    /// The release's title. Empty unless `status` is `ready`.
    #[graphql(name = "title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: Option<String>,

    /// `indexer` or `feed`.
    #[graphql(name = "sourceKind")]
    #[filterable(type = "string")]
    pub source_kind: Option<String>,

    #[graphql(name = "sourceId")]
    #[filterable(type = "string")]
    pub source_id: Option<String>,

    #[graphql(name = "sourceName")]
    pub source_name: Option<String>,

    /// For a release from a feed: its catalogue item.
    #[graphql(name = "feedItemId")]
    #[filterable(type = "string")]
    pub feed_item_id: Option<String>,

    /// The release's download link or magnet. Encrypted by the write transform; never readable.
    #[graphql(skip)]
    #[input_only]
    #[graphql_orm(sensitive)]
    pub release_link: String,

    #[graphql(name = "infoHash")]
    #[filterable(type = "string")]
    pub info_hash: Option<String>,

    #[graphql(name = "guid")]
    #[filterable(type = "string")]
    pub guid: Option<String>,

    #[graphql(name = "size")]
    #[filterable(type = "number")]
    pub size: Option<i64>,

    #[graphql(name = "seeders")]
    #[filterable(type = "number")]
    pub seeders: Option<i32>,

    /// The parsed quality, such as `1080p WEB-DL h264`.
    #[graphql(name = "quality")]
    pub quality: Option<String>,

    /// How many of the missing items this release would fill.
    #[graphql(name = "coversCount")]
    #[filterable(type = "number")]
    pub covers_count: Option<i32>,

    /// How many are missing in the scope that was looked at.
    #[graphql(name = "missingCount")]
    #[filterable(type = "number")]
    pub missing_count: Option<i32>,

    /// Ids of the episodes or tracks it covers, as a JSON array. Used to download only those files
    /// from a pack.
    #[graphql(name = "coversIds")]
    pub covers_ids: Option<String>,

    /// Why it won, short: `Best allowed quality`, `Smallest at that quality`, `From a feed`.
    #[graphql(name = "reasons")]
    #[json_field]
    pub reasons: Vec<String>,

    /// For `none`: why nothing was usable.
    #[graphql(name = "note")]
    pub note: Option<String>,

    /// For a release that is part of a coverage plan (Q101): the id of the plan's own row, whose
    /// scope is `plan`. A plan's row says how many of the missing items its releases cover and what
    /// stays uncovered (`note`); each release's row says what it adds.
    #[graphql(name = "planId")]
    #[filterable(type = "string")]
    pub plan_id: Option<String>,

    /// Where a release comes in its plan, from 0.
    #[graphql(name = "planPosition")]
    #[sortable]
    pub plan_position: Option<i32>,

    /// The seasons of the missing episodes this release covers, as a JSON array (shows only).
    #[graphql(name = "coversSeasons")]
    pub covers_seasons: Option<String>,

    /// The torrent's file indexes to download for exactly what it covers, as a JSON array, when its
    /// files were looked into before it was recommended (Q100). Used with "only the missing files".
    #[graphql(name = "fileIndices")]
    pub file_indices: Option<String>,

    /// The version of the rules that chose this answer (`jobs::recommend::RULES_VERSION`). An answer
    /// from older rules is not trusted: it is removed and looked up again (Q98).
    #[graphql(name = "rulesVersion")]
    pub rules_version: Option<i32>,

    #[graphql(name = "foundAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub found_at: String,

    #[graphql(name = "expiresAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub expires_at: String,

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
pub struct ReleaseRecommendationCustomOperations;
