//! QualityProfile Entity
//!
//! A reusable, nameable set of quality rules (resolution/codec/HDR/source/
//! audio/release-group allow-lists) that can be assigned to a `Library`
//! (primary) or overridden per `Show`/`Movie`/`Album`/`Audiobook`. See
//! `docs/tier1-features-plan.md` §2 for the design rationale (a proper entity
//! rather than per-show override columns) and `docs/design.md`'s Quality
//! Profiles section / Q23-Q26.

use async_graphql::Enum;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use crate::graphql::entities::*;

/// Which kind of media a profile applies to. Video profiles evaluate
/// resolution/video codec/HDR/source; audio profiles evaluate audio format
/// only.
#[derive(Default, Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize, sqlx::Type)]
#[graphql(name = "MediaKind")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum MediaKind {
    #[graphql(name = "VIDEO")]
    #[default]
    Video,
    #[graphql(name = "AUDIO")]
    Audio,
}

impl std::fmt::Display for MediaKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Video => write!(f, "video"),
            Self::Audio => write!(f, "audio"),
        }
    }
}

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "quality_profiles",
    plural = "QualityProfiles",
    default_sort = "name",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct QualityProfile {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "mediaKind")]
    pub media_kind: MediaKind,

    /// Allowed resolutions (e.g. "2160p", "1080p"). Empty = any (Q26).
    #[graphql(name = "allowedResolutions")]
    #[json_field]
    pub allowed_resolutions: Vec<String>,

    /// Allowed video codecs (e.g. "hevc", "h264"). Empty = any.
    #[graphql(name = "allowedVideoCodecs")]
    #[json_field]
    pub allowed_video_codecs: Vec<String>,

    /// Allowed audio formats (e.g. "atmos", "flac"). Empty = any.
    #[graphql(name = "allowedAudioFormats")]
    #[json_field]
    pub allowed_audio_formats: Vec<String>,

    /// Allowed HDR types when `require_hdr` (or the source is HDR). Empty =
    /// any HDR type accepted.
    #[graphql(name = "allowedHdrTypes")]
    #[json_field]
    pub allowed_hdr_types: Vec<String>,

    /// Allowed source types (e.g. "bluray", "web-dl"). Empty = any.
    #[graphql(name = "allowedSources")]
    #[json_field]
    pub allowed_sources: Vec<String>,

    /// Release groups that are always rejected, regardless of other fields.
    #[graphql(name = "releaseGroupBlacklist")]
    #[json_field]
    pub release_group_blacklist: Vec<String>,

    /// If non-empty, only these release groups are accepted.
    #[graphql(name = "releaseGroupWhitelist")]
    #[json_field]
    pub release_group_whitelist: Vec<String>,

    /// Whether HDR is mandatory (Q23).
    #[graphql(name = "requireHdr")]
    #[filterable(type = "boolean")]
    pub require_hdr: bool,

    /// Resolution at/above which upgrade-seeking stops (only meaningful when
    /// `upgrade_until_cutoff = true`).
    #[graphql(name = "cutoffResolution")]
    pub cutoff_resolution: Option<String>,

    /// Whether the system should keep seeking upgrades until
    /// `cutoff_resolution` is reached (Q38 upgrade-notification gating).
    #[graphql(name = "upgradeUntilCutoff")]
    #[filterable(type = "boolean")]
    pub upgrade_until_cutoff: bool,

    // ========================================================================
    // Release-selection rules (auto-download hunt). These are evaluated
    // against a parsed release title plus the release's own metadata (size,
    // seeders, publish date) rather than against an imported file, so they
    // only apply on the acquisition side.
    // ========================================================================
    /// Preferred audio languages as ISO 639-1 codes, ordered (earlier = more
    /// preferred). Empty = any language. Soft preference unless
    /// `require_language_match` is set.
    #[graphql(name = "preferredLanguages")]
    #[json_field]
    #[graphql_orm(default = "'[]'")]
    pub preferred_languages: Vec<String>,

    /// When true a release must declare one of `preferred_languages` to be
    /// acceptable at all. A release with no detectable language tag counts as
    /// English (scene convention: only non-English releases are tagged).
    #[graphql(name = "requireLanguageMatch")]
    #[filterable(type = "boolean")]
    #[graphql_orm(default = "0")]
    pub require_language_match: bool,

    /// Minimum release size in MB. For TV season packs this is interpreted
    /// per episode (pack size / episode count). Null = no minimum.
    #[graphql(name = "minSizeMb")]
    pub min_size_mb: Option<i32>,

    /// Maximum release size in MB, per episode for TV season packs.
    /// Null = no maximum.
    #[graphql(name = "maxSizeMb")]
    pub max_size_mb: Option<i32>,

    /// Minimum seeder count for torrent releases. Releases from sources that
    /// do not report seeders are not filtered by this.
    #[graphql(name = "minSeeders")]
    #[filterable(type = "number")]
    #[graphql_orm(default = "1")]
    pub min_seeders: i32,

    /// Reject releases published more than this many days ago. Null = any age.
    #[graphql(name = "maxReleaseAgeDays")]
    pub max_release_age_days: Option<i32>,

    /// Ordered soft preference over release groups (earlier = better). The
    /// existing `release_group_whitelist`/`release_group_blacklist` remain the
    /// hard filters; this only affects ranking.
    #[graphql(name = "preferredReleaseGroups")]
    #[json_field]
    #[graphql_orm(default = "'[]'")]
    pub preferred_release_groups: Vec<String>,

    /// Whether whole-season packs may be grabbed for this profile.
    #[graphql(name = "allowSeasonPacks")]
    #[filterable(type = "boolean")]
    #[graphql_orm(default = "1")]
    pub allow_season_packs: bool,

    /// Whether PROPER/REPACK releases are preferred over plain ones at the
    /// same quality (and treated as an upgrade over an existing file).
    #[graphql(name = "preferProperRepack")]
    #[filterable(type = "boolean")]
    #[graphql_orm(default = "1")]
    pub prefer_proper_repack: bool,

    /// Ordered resolution preference (earlier = better), e.g.
    /// `["1080p", "2160p", "720p"]`. Empty = the highest allowed resolution
    /// wins. Entries must still be permitted by `allowed_resolutions`.
    #[graphql(name = "resolutionPreference")]
    #[json_field]
    #[graphql_orm(default = "'[]'")]
    pub resolution_preference: Vec<String>,

    /// The seeded default profile used when nothing else resolves
    /// (`profile::default_profile`). Exactly one profile should have this set
    /// at a time; not enforced at the DB level.
    #[graphql(name = "isDefault")]
    #[filterable(type = "boolean")]
    pub is_default: bool,

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
pub struct QualityProfileCustomOperations;
