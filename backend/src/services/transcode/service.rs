//! `TranscodeService`: HLS/ffmpeg transcode session lifecycle.
//!
//! Depends only on `database` (per `docs/tier1-features-plan.md` §4); the
//! session bookkeeping itself is pure in-memory state. Background loops
//! (torrent-service cancellation-token pattern, see
//! `services/torrent/service.rs`'s `reconciliation_sweep_loop` as the
//! template):
//! 1. **Reaper** — kills `ffmpeg` and deletes segment directories for
//!    sessions idle longer than `idle_timeout_minutes`. Required for
//!    correctness, not polish: without it, HLS segment output under
//!    `cache_path` grows unbounded (same class of bug as the app_log
//!    retention issue).
//! 2. **Startup cleanup** — since sessions are ephemeral/in-memory, every
//!    directory under `{cache_path}/hls` at process start is by definition
//!    orphaned (no `Child` process from a previous run can still be alive),
//!    so `start()` wipes and recreates that directory tree.
//!
//! `stop()` kills every live `ffmpeg` child via
//! [`SessionRegistry::kill_all`] so graceful shutdown never leaves an
//! orphaned transcode process running after the server exits.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::services::manager::{Service, ServiceHealth, ServicesManager};
use crate::services::transcode::ffmpeg;
use crate::services::transcode::session::{
    SessionKey, SessionRegistry, TranscodeKind, TranscodeSession,
};

/// Config for [`TranscodeService`]. `ffmpeg_path` mirrors `Config::cache_path`'s
/// shape: read once at startup (see `config/mod.rs`'s `FFMPEG_PATH` env var),
/// defaulting to the bare `"ffmpeg"` on `$PATH` when unset, so a
/// Windows-bundled-binary distribution can point at a shipped `ffmpeg.exe`
/// without a code change.
#[derive(Debug, Clone)]
pub struct TranscodeServiceConfig {
    pub cache_path: PathBuf,
    pub ffmpeg_path: String,
    pub idle_timeout_minutes: i64,
    pub reaper_interval_minutes: u64,
}

impl Default for TranscodeServiceConfig {
    fn default() -> Self {
        Self {
            cache_path: PathBuf::from("./data/cache"),
            ffmpeg_path: "ffmpeg".to_string(),
            idle_timeout_minutes: 30,
            reaper_interval_minutes: 5,
        }
    }
}

struct TranscodeRuntime {
    cancel_token: CancellationToken,
    reaper_handle: tokio::task::JoinHandle<()>,
}

/// HLS/ffmpeg transcode service: owns the in-memory session registry, spawns
/// remux/transcode `ffmpeg` children on demand (called from
/// `api/media.rs`'s HLS routes), and runs the idle-session reaper loop.
pub struct TranscodeService {
    config: TranscodeServiceConfig,
    sessions: Arc<SessionRegistry>,
    starts: tokio::sync::Mutex<HashMap<SessionKey, Weak<tokio::sync::Mutex<()>>>>,
    subtitle_slots: tokio::sync::Semaphore,
    ffmpeg_available: tokio::sync::RwLock<bool>,
    inner: tokio::sync::RwLock<Option<TranscodeRuntime>>,
}

impl TranscodeService {
    pub fn new(_manager: Arc<ServicesManager>, config: TranscodeServiceConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(SessionRegistry::new()),
            starts: tokio::sync::Mutex::new(HashMap::new()),
            subtitle_slots: tokio::sync::Semaphore::new(2),
            ffmpeg_available: tokio::sync::RwLock::new(true),
            inner: tokio::sync::RwLock::new(None),
        }
    }

    /// Directory HLS output lives under: `{cache_path}/hls`.
    fn hls_root(&self) -> PathBuf {
        self.config.cache_path.join("hls")
    }

    /// Return the output directory for an existing session (bumping its
    /// `last_accessed`), or start a new `ffmpeg` remux/transcode session and
    /// return its (freshly created) output directory. Called from the
    /// `/hls/playlist.m3u8` route.
    pub async fn get_or_start_session(
        &self,
        media_file_id: &str,
        input_path: &Path,
        kind: TranscodeKind,
    ) -> Result<PathBuf> {
        self.get_or_start_session_with_audio(media_file_id, input_path, kind, None)
            .await
    }

    /// Start/reuse an independent variant. `audio_stream` is an absolute
    /// ffprobe stream index already authorized against the file by the caller.
    pub async fn get_or_start_session_with_audio(
        &self,
        media_file_id: &str,
        input_path: &Path,
        kind: TranscodeKind,
        audio_stream: Option<i32>,
    ) -> Result<PathBuf> {
        anyhow::ensure!(
            audio_stream.is_none_or(|index| index >= 0),
            "negative audio stream index"
        );
        let key = SessionKey::new(media_file_id, kind, audio_stream);
        let start_lock = {
            let mut starts = self.starts.lock().await;
            starts.retain(|_, lock| lock.strong_count() > 0);
            if let Some(lock) = starts.get(&key).and_then(Weak::upgrade) {
                lock
            } else {
                let lock = Arc::new(tokio::sync::Mutex::new(()));
                starts.insert(key.clone(), Arc::downgrade(&lock));
                lock
            }
        };
        let _start = start_lock.lock().await;
        if let Some(dir) = self.sessions.touch_variant(&key).await {
            return Ok(dir);
        }

        let session_id = Uuid::new_v4().to_string();
        let output_dir = self.hls_root().join(&session_id);
        tokio::fs::create_dir_all(&output_dir)
            .await
            .with_context(|| format!("failed to create HLS output dir {}", output_dir.display()))?;

        let child = match ffmpeg::spawn_hls_with_audio(
            &self.config.ffmpeg_path,
            input_path,
            &output_dir,
            &session_id,
            kind,
            audio_stream,
        ) {
            Ok(child) => child,
            Err(error) => {
                let _ = tokio::fs::remove_dir_all(&output_dir).await;
                return Err(error).with_context(|| {
                    format!("failed to spawn ffmpeg for media file {media_file_id}")
                });
            }
        };

        info!(
            media_file_id = %media_file_id,
            session_id = %session_id,
            kind = ?kind,
            audio_stream = ?audio_stream,
            output_dir = %output_dir.display(),
            "Transcode: started HLS session"
        );

        let mut session = TranscodeSession::new(
            media_file_id.to_string(),
            session_id,
            output_dir.clone(),
            kind,
            child,
        );
        session.audio_stream = audio_stream;
        self.sessions.insert(session).await;
        Ok(output_dir)
    }

    /// Bump `last_accessed` for an existing session and return its output
    /// directory, or `None` if no session is active for this file. Called
    /// from the `/hls/{segment}` route — segment requests never start a new
    /// session on their own (the playlist route must be hit first).
    pub async fn touch_session(&self, media_file_id: &str) -> Option<PathBuf> {
        self.sessions.touch(media_file_id).await
    }

    /// Segments must resolve to precisely the playlist's kind/audio variant.
    pub async fn touch_session_with_audio(
        &self,
        media_file_id: &str,
        kind: TranscodeKind,
        audio_stream: Option<i32>,
    ) -> Option<PathBuf> {
        self.sessions
            .touch_variant(&SessionKey::new(media_file_id, kind, audio_stream))
            .await
    }

    /// Convert a previously authorized, canonical subtitle source to WebVTT.
    /// Bound queueing as well as process time and output allocation.
    pub async fn subtitle_webvtt(
        &self,
        input: &Path,
        stream_index: Option<i32>,
        input_format: Option<&str>,
    ) -> std::result::Result<Vec<u8>, ffmpeg::SubtitleError> {
        let _slot = tokio::time::timeout(Duration::from_secs(2), self.subtitle_slots.acquire())
            .await
            .map_err(|_| ffmpeg::SubtitleError::Busy)?
            .map_err(|_| ffmpeg::SubtitleError::Busy)?;
        ffmpeg::subtitle_webvtt(&self.config.ffmpeg_path, input, stream_index, input_format).await
    }

    /// Number of live HLS sessions (for health/debug reporting).
    pub async fn active_session_count(&self) -> usize {
        self.sessions.len().await
    }
}

#[async_trait]
impl Service for TranscodeService {
    fn name(&self) -> &str {
        "transcode"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string()]
    }

    async fn start(&self) -> Result<()> {
        if self.config.idle_timeout_minutes <= 0 {
            anyhow::bail!("Transcode idle timeout must be greater than zero");
        }
        if self.config.reaper_interval_minutes == 0 {
            anyhow::bail!("Transcode reaper interval must be greater than zero");
        }
        let previous = self.inner.write().await.take();
        if let Some(runtime) = previous {
            runtime.cancel_token.cancel();
            let _ = runtime.reaper_handle.await;
        }

        let ffmpeg_ok = ffmpeg::ffmpeg_available(&self.config.ffmpeg_path).await;
        *self.ffmpeg_available.write().await = ffmpeg_ok;
        if !ffmpeg_ok {
            warn!(
                ffmpeg_path = %self.config.ffmpeg_path,
                "ffmpeg is unavailable; HLS transcode/remux requests will fail until it is installed or FFMPEG_PATH is corrected"
            );
        }

        // Ephemeral/in-memory sessions: anything left on disk under
        // {cache_path}/hls from a previous process is by definition orphaned.
        if let Err(e) = cleanup_orphaned_dirs(&self.hls_root()).await {
            warn!(error = %e, "Transcode: failed to clean up orphaned HLS cache directory at startup");
        }

        let cancel_token = CancellationToken::new();
        let token = cancel_token.clone();
        let sessions = self.sessions.clone();
        let idle_timeout_minutes = self.config.idle_timeout_minutes;
        let reaper_interval_minutes = self.config.reaper_interval_minutes;
        let reaper_handle = tokio::spawn(async move {
            reaper_loop(
                sessions,
                idle_timeout_minutes,
                reaper_interval_minutes,
                token,
            )
            .await;
        });

        *self.inner.write().await = Some(TranscodeRuntime {
            cancel_token,
            reaper_handle,
        });

        info!(
            service = "transcode",
            cache_path = %self.config.cache_path.display(),
            "Transcode service started"
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let previous = self.inner.write().await.take();
        if let Some(runtime) = previous {
            runtime.cancel_token.cancel();
            let _ = runtime.reaper_handle.await;
        }
        // Kill every live ffmpeg child + delete its segment dir so graceful
        // shutdown never leaves an orphaned transcode process running.
        self.sessions.kill_all().await;
        info!(service = "transcode", "Transcode service stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        let runtime = self.inner.read().await;
        let Some(runtime) = runtime.as_ref() else {
            return Ok(ServiceHealth::degraded("transcode service not started"));
        };
        if runtime.reaper_handle.is_finished() {
            return Ok(ServiceHealth::degraded(
                "transcode reaper worker exited unexpectedly",
            ));
        }
        if !*self.ffmpeg_available.read().await {
            return Ok(ServiceHealth::degraded(
                "ffmpeg is unavailable; HLS transcode/remux requests will fail",
            ));
        }
        Ok(ServiceHealth::healthy())
    }
}

/// Delete and recreate `{cache_path}/hls`. Safe because sessions are
/// in-memory only: at process start there is no live `Child` for anything
/// found on disk, so every subdirectory is leftover from a prior run (a
/// crash, or a container restart) and can't be reclaimed by any other means.
async fn cleanup_orphaned_dirs(hls_root: &Path) -> Result<()> {
    if hls_root.exists() {
        tokio::fs::remove_dir_all(hls_root).await.with_context(|| {
            format!(
                "failed to remove orphaned HLS cache dir {}",
                hls_root.display()
            )
        })?;
    }
    tokio::fs::create_dir_all(hls_root)
        .await
        .with_context(|| format!("failed to create HLS cache dir {}", hls_root.display()))?;
    Ok(())
}

async fn reaper_loop(
    sessions: Arc<SessionRegistry>,
    idle_timeout_minutes: i64,
    reaper_interval_minutes: u64,
    cancel: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(Duration::from_secs(reaper_interval_minutes.max(1) * 60)) => {
                let reaped = sessions.reap_idle(idle_timeout_minutes).await;
                if !reaped.is_empty() {
                    info!(
                        count = reaped.len(),
                        media_file_ids = ?reaped,
                        "Transcode: reaped idle HLS sessions"
                    );
                }
            }
        }
    }
}
