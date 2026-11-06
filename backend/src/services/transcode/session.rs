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
    child: Option<Child>,
    /// Segments are encoded on request, so this session has no fixed `ffmpeg`
    /// run: its encoder restarts wherever someone seeks to.
    on_demand: bool,
    /// The segment the current encoder started from (on-demand sessions).
    encoder_start: u32,
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
            child: Some(child),
            on_demand: false,
            encoder_start: 0,
            last_accessed: Utc::now(),
        }
    }

    /// A session with no encoder yet. It starts when the first segment is asked for.
    pub fn on_demand(
        media_file_id: String,
        session_id: String,
        output_dir: PathBuf,
        kind: TranscodeKind,
        audio_stream: Option<i32>,
    ) -> Self {
        Self {
            media_file_id,
            session_id,
            output_dir,
            kind,
            audio_stream,
            child: None,
            on_demand: true,
            encoder_start: 0,
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

/// How close ahead of the encoder a request can be and still be worth waiting for. Encoding
/// runs faster than playback, so stopping it for a segment this near would only lose time.
const ENCODER_WAIT_SEGMENTS: u32 = 4;

/// The outcome of asking for an on-demand segment; see [`SessionRegistry::plan_segment`].
#[derive(Debug, PartialEq, Eq)]
pub enum SegmentPlan {
    /// Already on disk.
    Ready(PathBuf),
    /// The running encoder will reach it shortly.
    Wait(PathBuf),
    /// Start an encoder at this segment; any earlier one has been stopped.
    Restart(PathBuf),
    /// The file ended before this segment.
    PastEnd,
}

/// Whether an encoder that started at `start` and has written up to `reached` will soon write `segment`.
pub fn segment_is_coming(start: u32, reached: u32, segment: u32) -> bool {
    segment >= start && segment <= reached.saturating_add(ENCODER_WAIT_SEGMENTS)
}

/// A segment's file, by the same naming `ffmpeg` is told to use.
pub fn segment_path(output_dir: &std::path::Path, segment: u32) -> PathBuf {
    output_dir.join(format!("seg_{segment:03}.ts"))
}

/// The number of a segment name asked for over HTTP: `seg_0042.ts` is 42.
pub fn segment_number(name: &str) -> Option<u32> {
    let digits = name.strip_prefix("seg_")?.strip_suffix(".ts")?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn next_missing_segment(output_dir: &std::path::Path, from: u32) -> u32 {
    let mut segment = from;
    while segment_path(output_dir, segment).exists() {
        segment += 1;
    }
    segment
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
            let reusable = session.on_demand
                || match session.child.as_mut().map(Child::try_wait) {
                    None | Some(Ok(None)) => true,
                    Some(Ok(Some(status))) => status.success(),
                    Some(Err(error)) => {
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

    /// What to do for a request of on-demand `segment`, given how far the encoder has got.
    /// `Restart` has already stopped the old encoder and returns the session's directory.
    pub async fn plan_segment(&self, key: &SessionKey, segment: u32) -> Option<SegmentPlan> {
        let mut stopped = None;
        let plan = {
            let mut guard = self.sessions.write().await;
            let session = guard.get_mut(key)?;
            session.last_accessed = Utc::now();
            let dir = session.output_dir.clone();
            if segment_path(&dir, segment).exists() {
                return Some(SegmentPlan::Ready(dir));
            }
            let status = session.child.as_mut().map(Child::try_wait);
            let start = session.encoder_start;
            let reached = next_missing_segment(&dir, start);
            match status {
                Some(Ok(None)) if segment_is_coming(start, reached, segment) => {
                    SegmentPlan::Wait(dir)
                }
                // Encoded through to the end of the file, and this wasn't in it.
                Some(Ok(Some(status))) if status.success() && segment >= start => {
                    SegmentPlan::PastEnd
                }
                _ => {
                    stopped = session.child.take();
                    session.encoder_start = segment;
                    SegmentPlan::Restart(dir)
                }
            }
        };
        if let Some(mut child) = stopped {
            let _ = child.start_kill();
            let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
        }
        Some(plan)
    }

    /// Hand a freshly spawned encoder to its session.
    pub async fn set_encoder(&self, key: &SessionKey, child: Child) {
        if let Some(session) = self.sessions.write().await.get_mut(key) {
            session.child = Some(child);
        }
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
    if let Some(child) = session.child.as_mut() {
        if let Err(e) = child.start_kill() {
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
        match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
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
    #[test]
    fn segment_numbers_come_only_from_segment_names() {
        assert_eq!(segment_number("seg_000.ts"), Some(0));
        assert_eq!(segment_number("seg_1351.ts"), Some(1351));
        assert_eq!(segment_number("seg_.ts"), None);
        assert_eq!(segment_number("seg_12a.ts"), None);
        assert_eq!(segment_number("seg_-1.ts"), None);
        assert_eq!(segment_number("../seg_1.ts"), None);
        assert_eq!(segment_number("seg_99999999999.ts"), None);
    }

    #[test]
    fn an_encoder_is_only_waited_for_when_it_is_nearly_there() {
        // Started at 100, has written up to 110.
        assert!(segment_is_coming(100, 110, 110));
        assert!(segment_is_coming(100, 110, 114));
        assert!(!segment_is_coming(100, 110, 115));
        // Behind where it started: it will never write this.
        assert!(!segment_is_coming(100, 110, 99));
    }

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
