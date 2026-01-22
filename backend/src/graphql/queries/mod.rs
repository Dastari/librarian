pub mod audiobooks;
pub mod episodes;
pub mod filesystem;
pub mod indexers;
pub mod libraries;
pub mod logs;
pub mod media;
pub mod media_files;
pub mod movies;
pub mod music;
pub mod notifications;
pub mod playback;
pub mod priority_rules;
pub mod rss_feeds;
pub mod settings;
pub mod system;
pub mod torrents;
pub mod tv_shows;
pub mod upcoming;
pub mod usenet;
pub mod user;

pub use audiobooks::AudiobookQueries;
pub use episodes::EpisodeQueries;
pub use filesystem::FilesystemQueries;
pub use indexers::IndexerQueries;
pub use libraries::LibraryQueries;
pub use logs::LogQueries;
pub use media::MediaQueries;
pub use media_files::MediaFileQueries;
pub use movies::MovieQueries;
pub use music::MusicQueries;
pub use notifications::NotificationQueries;
pub use playback::PlaybackQueries;
pub use priority_rules::PriorityRuleQueries;
pub use rss_feeds::RssFeedQueries;
pub use settings::SettingsQueries;
pub use system::SystemQueries;
pub use torrents::TorrentQueries;
pub use tv_shows::TvShowQueries;
pub use upcoming::UpcomingQueries;
pub use usenet::UsenetQueries;
pub use user::UserQueries;

pub(crate) mod prelude {
    pub(crate) use std::collections::HashMap;
    pub(crate) use std::sync::Arc;

    pub(crate) use async_graphql::{Context, Object, Result};
    pub(crate) use uuid::Uuid;

    pub(crate) use crate::db::*;
    pub(crate) use crate::graphql::auth::AuthExt;
    pub(crate) use crate::graphql::filters::OrderDirection;
    pub(crate) use crate::graphql::helpers::*;
    pub(crate) use crate::graphql::pagination::{Connection, parse_pagination_args};
    pub(crate) use crate::graphql::types::*;
    pub(crate) use crate::services::{
        CastService, FilesystemService, MetadataService, NotificationService, TorrentService,
    };
}
