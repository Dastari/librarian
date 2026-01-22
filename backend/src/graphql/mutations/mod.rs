pub mod auth;
pub mod audiobooks;
pub mod filesystem;
pub mod indexers;
pub mod libraries;
pub mod logs;
pub mod media_files;
pub mod movies;
pub mod music;
pub mod notifications;
pub mod playback;
pub mod priority_rules;
pub mod rss_feeds;
pub mod settings;
pub mod torrents;
pub mod tv_shows;
pub mod usenet;
pub mod user;

pub use auth::AuthMutations;
pub use audiobooks::AudiobookMutations;
pub use filesystem::FilesystemMutations;
pub use indexers::IndexerMutations;
pub use libraries::LibraryMutations;
pub use logs::LogMutations;
pub use media_files::MediaFileMutations;
pub use movies::MovieMutations;
pub use music::MusicMutations;
pub use notifications::NotificationMutations;
pub use playback::PlaybackMutations;
pub use priority_rules::PriorityRuleMutations;
pub use rss_feeds::RssFeedMutations;
pub use settings::SettingsMutations;
pub use torrents::TorrentMutations;
pub use tv_shows::TvShowMutations;
pub use usenet::UsenetMutations;
pub use user::UserMutations;

pub(crate) mod prelude {
    pub(crate) use std::sync::Arc;

    pub(crate) use async_graphql::{Context, Object, Result};
    pub(crate) use uuid::Uuid;

    pub(crate) use crate::db::*;
    pub(crate) use crate::graphql::auth::AuthExt;
    pub(crate) use crate::graphql::helpers::*;
    pub(crate) use crate::graphql::types::*;
    pub(crate) use crate::services::{
        CastService, FilesystemService, MetadataService, NotificationService, ScannerService,
        TorrentService,
    };
}
