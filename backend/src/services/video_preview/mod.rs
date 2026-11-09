//! Previews made from a video after analysis: seek-bar storyboards (design.md Q122) and the
//! picture inside baked-in black bars (design.md Q123), sharing one slow background queue
//! (design.md Q124).

pub mod cropdetect;
pub mod extract;
mod service;
pub mod storyboard;

pub use service::{
    SETTING_BACKFILL, VideoPreviewConfig, VideoPreviewService, install, sheet_path,
    valid_storyboard_key, video_previews,
};
