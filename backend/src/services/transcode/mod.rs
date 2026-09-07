//! HLS/FFmpeg on-demand transcoding (`docs/tier1-features-plan.md` §4).
//!
//! - [`ffmpeg`]: `ffmpeg` availability check + remux/transcode process
//!   spawning, mirroring `library_scan.rs`'s `ffprobe` invocation pattern.
//! - [`session`]: in-memory (not a DB entity) bookkeeping for live HLS
//!   sessions, keyed by `media_file_id`.
//! - [`service`]: `TranscodeService: Service`, the idle-session reaper loop,
//!   and startup cleanup of orphaned segment directories.
//!
//! HTTP routes (`GET /media/{file_id}/hls/playlist.m3u8` and
//! `GET /media/{file_id}/hls/{segment}`) live in `api/media.rs`, alongside
//! the `needs_transcode` decision function and the existing direct-play
//! `stream_media` route.

pub mod ffmpeg;
pub mod service;
pub mod session;

pub use service::{TranscodeService, TranscodeServiceConfig};
pub use session::TranscodeKind;
