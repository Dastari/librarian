//! In-memory bookkeeping for live HLS transcode/remux sessions.
//!
//! Sessions are intentionally **not** a DB entity (`docs/tier1-features-plan.md`
//! §4: "Transcode sessions are intentionally ephemeral/in-memory") — they're
//! runtime-only state tied to a live `ffmpeg` `Child` process, keyed by
//! file, packaging kind and selected absolute audio stream index.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::process::Child;
use tokio::sync::RwLock;
use tracing::warn;

/// Which `ffmpeg` invocation produced a session's segments — kept so logs and
/// health/debug views can distinguish the fast (remux) path from the
/// expensive (transcode) path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranscodeKind {
    /// Stream-copy repackage (`-c copy`) — incompatible container, compatible codecs.
    Remux,
    /// Full re-encode (`-c:v libx264 -c:a aac`) — incompatible codec (HEVC/DV).
    Transcode,
}

/// Independent output namespace for each file/kind/audio selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionKey {
    pub media_file_id: String,
    pub kind: TranscodeKind,
    pub audio_stream: Option<i32>,
}

impl SessionKey {
    pub fn new(media_file_id: &str, kind: TranscodeKind, audio_stream: Option<i32>) -> Self {
        Self {
            media_file_id: media_file_id.to_owned(),
            kind,
            audio_stream,
        }
    }
}

/// A live HLS session: one `ffmpeg` child process writing segments/playlist
/// into `output_dir`, tracked so repeated segment/playlist requests reuse it
/// instead of spawning a second `ffmpeg` for the same file.
pub struct TranscodeSession {
    pub media_file_id: String,
    pub session_id: String,
    pub output_dir: PathBuf,
    pub kind: TranscodeKind,
    pub audio_stream: Option<i32>,
    child: Child,
    last_accessed: DateTime<Utc>,
}

impl TranscodeSession {
    pub fn new(
        media_file_id: String,
        session_id: String,
        output_dir: PathBuf,
        kind: TranscodeKind,
        child: Child,
    ) -> Self {
        Self {
            media_file_id,
            session_id,
            output_dir,
            kind,
            audio_stream: None,
            child,
            last_accessed: Utc::now(),
        }
    }

    fn key(&self) -> SessionKey {
        SessionKey::new(&self.media_file_id, self.kind, self.audio_stream)
    }
}

/// Whether a session last accessed at `last_accessed` should be reaped at
/// `now`, given an idle cutoff in minutes. Pulled out as a pure function (no
/// process/filesystem access) so the cutoff logic is unit-testable without a
/// real `ffmpeg` child or DB.
pub fn session_is_idle(
    last_accessed: DateTime<Utc>,
    now: DateTime<Utc>,
    idle_minutes: i64,
) -> bool {
    now.signed_duration_since(last_accessed) >= chrono::Duration::minutes(idle_minutes)
}

/// Validate a segment filename requested over HTTP before it's joined onto a
/// session's output directory. Rejects anything that isn't a bare
/// `seg_XXX.ts`-shaped name: no path separators, no `..`, must end in `.ts`.
/// This is the only thing standing between `GET /media/{id}/hls/{segment}`
/// and an arbitrary-file-read/path-traversal bug, since `segment` is
/// attacker-controlled input joined onto a filesystem path.
pub fn sanitize_segment_name(name: &str) -> Option<&str> {
    if name.is_empty() {
        return None;
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return None;
    }
    if !name.ends_with(".ts") {
        return None;
    }
    Some(name)
}

/// In-memory registry of independent file/kind/audio variants.
#[derive(Default)]
pub struct SessionRegistry {
    sessions: RwLock<HashMap<SessionKey, TranscodeSession>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// If a session already exists for `media_file_id`, bump its
    /// `last_accessed` and return its output directory. Used both to decide
    /// whether a new `ffmpeg` needs to be spawned and to keep an in-progress
    /// session alive across repeated playlist/segment polls.
    pub async fn touch(&self, media_file_id: &str) -> Option<PathBuf> {
        // Legacy callers have no kind parameter. Prefer the default remux;
        // never return another client's explicit audio selection.
        if let Some(dir) = self
            .touch_for_kind(media_file_id, TranscodeKind::Remux)
            .await
        {
            return Some(dir);
        }
        self.touch_for_kind(media_file_id, TranscodeKind::Transcode)
            .await
    }

    /// Reuse a session only when its remux/transcode mode matches the caller's
    /// capability decision.
    pub async fn touch_for_kind(
        &self,
        media_file_id: &str,
        kind: TranscodeKind,
    ) -> Option<PathBuf> {
        self.touch_variant(&SessionKey::new(media_file_id, kind, None))
            .await
    }

    /// Failed children are removed so a later playlist request can retry.
    /// Successful EOF remains reusable: its complete VOD output is valid.
    pub async fn touch_variant(&self, key: &SessionKey) -> Option<PathBuf> {
        let failed = {
            let mut guard = self.sessions.write().await;
            let session = guard.get_mut(key)?;
            let reusable = match session.child.try_wait() {
                Ok(None) => true,
                Ok(Some(status)) => status.success(),
                Err(error) => {
                    warn!(media_file_id = %session.media_file_id, session_id = %session.session_id,
                        %error, "Transcode: failed to inspect HLS child");
                    false
                }
            };
            if reusable {
                session.last_accessed = Utc::now();
                return Some(session.output_dir.clone());
            }
            guard.remove(key)
        };
        if let Some(mut session) = failed {
            kill_and_cleanup(&mut session).await;
        }
        None
    }

    /// Stop and remove one active session, if present.
    pub async fn remove(&self, media_file_id: &str) {
        let sessions: Vec<_> = {
            let mut guard = self.sessions.write().await;
            let keys: Vec<_> = guard
                .keys()
                .filter(|key| key.media_file_id == media_file_id)
                .cloned()
                .collect();
            keys.into_iter()
                .filter_map(|key| guard.remove(&key))
                .collect()
        };
        for mut session in sessions {
            kill_and_cleanup(&mut session).await;
        }
    }

    /// Insert a newly spawned session, replacing (and killing) any prior
    /// session for the same file — this should not normally happen since
    /// callers check [`touch`](Self::touch) first, but guards against a race.
    pub async fn insert(&self, session: TranscodeSession) {
        let mut guard = self.sessions.write().await;
        if let Some(mut previous) = guard.insert(session.key(), session) {
            warn!(
                media_file_id = %previous.media_file_id,
                previous_session_id = %previous.session_id,
                "Transcode: replacing an existing HLS session for the same variant"
            );
            kill_and_cleanup(&mut previous).await;
        }
    }

    /// Kill the ffmpeg child and delete the segment directory for every
    /// session idle longer than `idle_minutes`. Returns the reaped
    /// `media_file_id`s (for logging by the caller).
    pub async fn reap_idle(&self, idle_minutes: i64) -> Vec<String> {
        let now = Utc::now();
        let idle_ids: Vec<SessionKey> = {
            let guard = self.sessions.read().await;
            guard
                .iter()
                .filter(|(_, s)| session_is_idle(s.last_accessed, now, idle_minutes))
                .map(|(id, _)| id.clone())
                .collect()
        };

        let mut removed = Vec::with_capacity(idle_ids.len());
        for id in idle_ids {
            let removed_session = {
                let mut guard = self.sessions.write().await;
                // A playlist/segment may have touched it since the snapshot.
                if guard.get(&id).is_some_and(|session| {
                    session_is_idle(session.last_accessed, now, idle_minutes)
                }) {
                    guard.remove(&id)
                } else {
                    None
                }
            };
            if let Some(mut session) = removed_session {
                kill_and_cleanup(&mut session).await;
                removed.push(id.media_file_id);
            }
        }
        removed
    }

    /// Kill every live session's ffmpeg child and delete its segment
    /// directory. Called from `TranscodeService::stop()` so graceful shutdown
    /// never leaves orphaned ffmpeg processes behind.
    pub async fn kill_all(&self) {
        let sessions: Vec<TranscodeSession> = {
            let mut guard = self.sessions.write().await;
            guard.drain().map(|(_, s)| s).collect()
        };
        for mut session in sessions {
            kill_and_cleanup(&mut session).await;
        }
    }

    /// Number of live sessions (for health/debug reporting).
    pub async fn len(&self) -> usize {
        self.sessions.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.sessions.read().await.is_empty()
    }
}

async fn kill_and_cleanup(session: &mut TranscodeSession) {
    if let Err(e) = session.child.start_kill() {
        warn!(
            media_file_id = %session.media_file_id,
            session_id = %session.session_id,
            kind = ?session.kind,
            error = %e,
            "Transcode: failed to signal ffmpeg child to stop"
        );
    }
    // Bounded wait for the process to actually exit after the kill signal;
    // never block shutdown/reap indefinitely on a wedged child.
    match tokio::time::timeout(Duration::from_secs(5), session.child.wait()).await {
        Ok(Ok(_status)) => {}
        Ok(Err(e)) => warn!(
            media_file_id = %session.media_file_id,
            session_id = %session.session_id,
            error = %e,
            "Transcode: error waiting for killed ffmpeg child to exit"
        ),
        Err(_) => warn!(
            media_file_id = %session.media_file_id,
            session_id = %session.session_id,
            "Transcode: timed out waiting for ffmpeg child to exit after kill"
        ),
    }

    if let Err(e) = tokio::fs::remove_dir_all(&session.output_dir).await
        && e.kind() != std::io::ErrorKind::NotFound
    {
        warn!(
            media_file_id = %session.media_file_id,
            session_id = %session.session_id,
            output_dir = %session.output_dir.display(),
            error = %e,
            "Transcode: failed to delete HLS segment directory during cleanup"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_is_idle_false_before_cutoff() {
        let now = Utc::now();
        let last_accessed = now - chrono::Duration::minutes(10);
        assert!(!session_is_idle(last_accessed, now, 30));
    }

    #[test]
    fn session_is_idle_true_after_cutoff() {
        let now = Utc::now();
        let last_accessed = now - chrono::Duration::minutes(45);
        assert!(session_is_idle(last_accessed, now, 30));
    }

    #[test]
    fn session_is_idle_true_at_exact_cutoff() {
        let now = Utc::now();
        let last_accessed = now - chrono::Duration::minutes(30);
        assert!(session_is_idle(last_accessed, now, 30));
    }

    #[test]
    fn sanitize_segment_name_accepts_expected_shape() {
        assert_eq!(sanitize_segment_name("seg_000.ts"), Some("seg_000.ts"));
        assert_eq!(sanitize_segment_name("seg_123.ts"), Some("seg_123.ts"));
    }

    #[test]
    fn sanitize_segment_name_rejects_path_traversal() {
        assert_eq!(sanitize_segment_name("../../etc/passwd"), None);
        assert_eq!(sanitize_segment_name("../secrets.ts"), None);
        assert_eq!(sanitize_segment_name("sub/seg_000.ts"), None);
        assert_eq!(sanitize_segment_name("sub\\seg_000.ts"), None);
    }

    #[test]
    fn sanitize_segment_name_rejects_wrong_extension_and_empty() {
        assert_eq!(sanitize_segment_name(""), None);
        assert_eq!(sanitize_segment_name("playlist.m3u8"), None);
        assert_eq!(sanitize_segment_name("seg_000.mp4"), None);
    }
}
