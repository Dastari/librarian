// Base entities (no relations to other entities)
pub mod audio_stream;
pub mod media_chapter;
pub mod media_file;
pub mod subtitle;
pub mod torrent_file;
pub mod video_stream;

// User and auth entities
pub mod invite_token;
pub mod refresh_token;
pub mod user;

// Settings and logging
pub mod app_log;
pub mod app_setting;

// Content entities
pub mod chapter;
pub mod episode;
pub mod movie;
pub mod rss_feed;
pub mod rss_feed_item;
pub mod show;
pub mod track;

// Indexer entities
pub mod indexer_config;
pub mod indexer_search_cache;
pub mod indexer_setting;

// Playback and cast entities
pub mod cast_device;
pub mod cast_session;
pub mod cast_setting;
pub mod playback_progress;
pub mod playback_session;

// Download entities
pub mod pending_file_match;
pub mod usenet_download;
pub mod usenet_server;

// Schedule and automation
pub mod naming_pattern;
pub mod schedule_cache;
pub mod schedule_sync_state;
pub mod source_priority_rule;

// Other entities
pub mod artwork_cache;
pub mod notification;
pub mod torznab_category;

// Higher-level entities with multiple relations
pub mod album;
pub mod artist;
pub mod audiobook;
pub mod library;
pub mod torrent;

// Re-export all entity types
pub use album::*;
pub use app_log::*;
pub use app_setting::*;
pub use artist::*;
pub use artwork_cache::*;
pub use audio_stream::*;
pub use audiobook::*;
pub use cast_device::*;
pub use cast_session::*;
pub use cast_setting::*;
pub use chapter::*;
pub use episode::*;
pub use indexer_config::*;
pub use indexer_search_cache::*;
pub use indexer_setting::*;
pub use invite_token::*;
pub use library::*;
pub use media_chapter::*;
pub use media_file::*;
pub use movie::*;
pub use naming_pattern::*;
pub use notification::*;
pub use pending_file_match::*;
pub use playback_progress::*;
pub use playback_session::*;
pub use refresh_token::*;
pub use rss_feed::*;
pub use rss_feed_item::*;
pub use schedule_cache::*;
pub use schedule_sync_state::*;
pub use show::*;
pub use source_priority_rule::*;
pub use subtitle::*;
pub use torrent::*;
pub use torrent_file::*;
pub use torznab_category::*;
pub use track::*;
pub use usenet_download::*;
pub use usenet_server::*;
pub use user::*;
pub use video_stream::*;
