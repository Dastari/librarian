//! The preview queue: one job at a time, lowest priority, for every video file (design.md Q124).
//!
//! A job runs crop detection, then the storyboard (drawn from the picture inside any bars), and
//! writes both to the file's record through the generated `updateMediaFile` mutation, with
//! `previewsCheckedAt` so the file is not done again until it changes. Jobs come from three
//! places, in this order of urgency:
//!
//! 1. **Playback**: the first play of a file without previews queues it at the front.
//! 2. **Analysis**: a freshly analysed file is queued at the back.
//! 3. **Backfill**: when the queue is empty, files never done are picked up a few at a time, with
//!    a pause after each, unless the `playback.video_preview_backfill` setting is off.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use chrono::Utc;
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::db::Database;
use crate::graphql::AuthUser;
use crate::graphql::entities::{
    MediaFile, MediaFileWhereInput, PictureCrop, VideoStoryboard, VideoStream,
    VideoStreamWhereInput,
};
use crate::services::library_storage::tool_argument;
use crate::services::manager::ServicesManager;

use super::cropdetect::CropDecision;
use super::extract::{Tools, detect_crop, render_storyboard};
use super::storyboard::{self, LAYOUT_VERSION};

/// `false` stops the background backfill; playback and analysis still queue files.
pub const SETTING_BACKFILL: &str = "playback.video_preview_backfill";
/// How long the queue waits, empty, before looking for files to backfill.
const BACKFILL_IDLE: Duration = Duration::from_secs(120);
/// The pause after each backfilled file, so a large library is worked through slowly.
const BACKFILL_PAUSE: Duration = Duration::from_secs(20);
/// Files fetched per backfill look; a few are queued from them.
const BACKFILL_PAGE: i64 = 200;
const BACKFILL_BATCH: usize = 5;
/// A file whose job failed (share offline, ffmpeg missing) is not retried for this long.
const RETRY_AFTER: Duration = Duration::from_secs(6 * 3600);
/// A file played again within this time is not looked up again.
const PLAYBACK_RECHECK: Duration = Duration::from_secs(10 * 60);
/// Codecs of cover art in music files, which ffprobe also calls video.
const STILL_CODECS: &[&str] = &["mjpeg", "png", "bmp", "gif", "webp", "tiff"];

#[derive(Debug, Clone)]
pub struct VideoPreviewConfig {
    /// The server's cache folder; sheets go in `storyboards/` under it.
    pub cache_path: PathBuf,
    pub ffmpeg_path: String,
}

#[derive(Default)]
struct Queue {
    jobs: VecDeque<String>,
    queued: HashSet<String>,
    failed: HashMap<String, Instant>,
    played: HashMap<String, Instant>,
    backfill: HashSet<String>,
    backfill_offset: i64,
}

pub struct VideoPreviewService {
    manager: Arc<ServicesManager>,
    config: VideoPreviewConfig,
    queue: Mutex<Queue>,
    wake: Notify,
    cancel: CancellationToken,
}

static SERVICE: OnceLock<Arc<VideoPreviewService>> = OnceLock::new();

/// The running service, once [`install`] has started it.
pub fn video_previews() -> Option<Arc<VideoPreviewService>> {
    SERVICE.get().cloned()
}

/// Start the service and its single worker. A second call returns the first service.
pub fn install(
    manager: Arc<ServicesManager>,
    config: VideoPreviewConfig,
) -> Arc<VideoPreviewService> {
    SERVICE
        .get_or_init(|| {
            let service = Arc::new(VideoPreviewService {
                manager,
                config,
                queue: Mutex::new(Queue::default()),
                wake: Notify::new(),
                cancel: CancellationToken::new(),
            });
            let worker = service.clone();
            tokio::spawn(async move { worker.run().await });
            info!(
                cache = %service.config.cache_path.display(),
                "Video preview queue started (storyboards and black-bar detection, one file at a time)"
            );
            service
        })
        .clone()
}

/// Whether a storyboard key from a URL has the shape this service makes.
pub fn valid_storyboard_key(key: &str) -> bool {
    key.len() == 16 && key.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Where a sheet is cached.
pub fn sheet_path(cache_path: &Path, media_file_id: &str, key: &str, sheet: u32) -> PathBuf {
    storyboards_dir(cache_path)
        .join(media_file_id)
        .join(key)
        .join(format!("{sheet}.jpg"))
}

fn storyboards_dir(cache_path: &Path) -> PathBuf {
    cache_path.join("storyboards")
}

/// Whether a record is a video worth previewing.
fn is_video(file: &MediaFile) -> bool {
    file.content_type.as_deref() == Some("video")
        && file
            .video_codec
            .as_deref()
            .is_some_and(|codec| !STILL_CODECS.contains(&codec.to_ascii_lowercase().as_str()))
        && file.duration.is_some_and(|duration| duration > 0)
        && file.width.is_some_and(|width| width > 0)
        && file.height.is_some_and(|height| height > 0)
}

/// Whether a record still wants its previews made.
fn needs_previews(file: &MediaFile) -> bool {
    file.analyzed_at.is_some() && file.previews_checked_at.is_none() && is_video(file)
}

fn content_key(
    file: &MediaFile,
    layout: &storyboard::StoryboardLayout,
    crop: Option<&PictureCrop>,
) -> String {
    let mut hash = Sha256::new();
    hash.update(file.id.as_bytes());
    hash.update(file.size.to_le_bytes());
    hash.update(file.file_modified_at.as_deref().unwrap_or("").as_bytes());
    hash.update(format!(
        "v{LAYOUT_VERSION}:{}x{}:{}:{}",
        layout.tile_width, layout.tile_height, layout.interval, layout.frame_count
    ));
    if let Some(crop) = crop {
        hash.update(format!(
            "{}:{}:{}:{}",
            crop.left, crop.top, crop.width, crop.height
        ));
    }
    hash.finalize()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl VideoPreviewService {
    pub fn stop(&self) {
        self.cancel.cancel();
    }

    pub fn cache_path(&self) -> &Path {
        &self.config.cache_path
    }

    async fn database(&self) -> Result<Database> {
        Ok(self
            .manager
            .get_database()
            .await
            .context("The database is not available")?
            .pool()
            .clone())
    }

    /// Queue a file after its analysis: behind anything already waiting.
    pub async fn queue_after_analysis(&self, media_file_id: &str) {
        let mut queue = self.queue.lock().await;
        if queue.queued.insert(media_file_id.to_string()) {
            queue.jobs.push_back(media_file_id.to_string());
            queue.backfill.remove(media_file_id);
            drop(queue);
            self.wake.notify_one();
        }
    }

    /// A file is being played: if it has no previews yet, make them next.
    pub async fn queue_for_playback(self: &Arc<Self>, media_file_id: &str) {
        {
            let mut queue = self.queue.lock().await;
            let now = Instant::now();
            queue
                .played
                .retain(|_, at| now.duration_since(*at) < PLAYBACK_RECHECK);
            if queue.played.contains_key(media_file_id) {
                return;
            }
            queue.played.insert(media_file_id.to_string(), now);
        }
        let service = self.clone();
        let id = media_file_id.to_string();
        tokio::spawn(async move {
            let file = match service.database().await {
                Ok(db) => MediaFile::get(db.pool(), &id).await.ok().flatten(),
                Err(_) => None,
            };
            let Some(file) = file.filter(needs_previews) else {
                return;
            };
            let mut queue = service.queue.lock().await;
            queue.failed.remove(&id);
            queue.backfill.remove(&id);
            if queue.queued.insert(id.clone()) {
                queue.jobs.push_front(id.clone());
            } else if let Some(position) = queue.jobs.iter().position(|job| job == &id) {
                // Already waiting: move it to the front.
                queue.jobs.remove(position);
                queue.jobs.push_front(id.clone());
            }
            drop(queue);
            info!(
                media_file_id = %id,
                media_file_path = %file.path,
                "Video previews queued first because the file is being played: '{}'",
                file.path
            );
            service.wake.notify_one();
        });
    }

    async fn backfill_enabled(&self, db: &Database) -> bool {
        match crate::services::torrent::get_setting::<serde_json::Value>(db, SETTING_BACKFILL).await
        {
            Ok(Some(serde_json::Value::Bool(value))) => value,
            Ok(Some(serde_json::Value::String(text))) => !matches!(
                text.trim().to_ascii_lowercase().as_str(),
                "false" | "0" | "off" | "no"
            ),
            Ok(_) => true,
            Err(error) => {
                warn!(error = %error, "Could not read the video preview backfill setting; backfill waits");
                false
            }
        }
    }

    /// Queue a few files that were never done. Returns how many were queued.
    async fn plan_backfill(&self) -> Result<usize> {
        let db = self.database().await?;
        if !self.backfill_enabled(&db).await {
            return Ok(0);
        }
        let offset = self.queue.lock().await.backfill_offset;
        let page = MediaFile::query(db.pool())
            .filter(MediaFileWhereInput {
                content_type: Some(StringFilter {
                    eq: Some("video".to_string()),
                    ..Default::default()
                }),
                previews_checked_at: Some(DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                analyzed_at: Some(DateFilter {
                    is_null: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .limit(BACKFILL_PAGE)
            .offset(offset)
            .fetch_all()
            .await
            .context("Could not list video files for the preview backfill")?;
        let full_page = page.len() as i64 == BACKFILL_PAGE;
        let mut queue = self.queue.lock().await;
        let now = Instant::now();
        queue
            .failed
            .retain(|_, at| now.duration_since(*at) < RETRY_AFTER);
        let mut added = 0;
        for file in page.iter().filter(|file| needs_previews(file)) {
            if added == BACKFILL_BATCH {
                break;
            }
            if queue.failed.contains_key(&file.id) || queue.queued.contains(&file.id) {
                continue;
            }
            queue.queued.insert(file.id.clone());
            queue.backfill.insert(file.id.clone());
            queue.jobs.push_back(file.id.clone());
            added += 1;
        }
        // Nothing usable on a full page (all failed recently, or not really video): look further
        // next time. An empty or short page starts again from the top.
        queue.backfill_offset = if added == 0 && full_page {
            offset + BACKFILL_PAGE
        } else if !full_page {
            0
        } else {
            offset
        };
        if added > 0 {
            info!(
                files = added,
                "Video preview backfill queued {} files that have no previews yet", added
            );
        }
        Ok(added)
    }

    async fn run(self: Arc<Self>) {
        let mut last_backfill_look: Option<Instant> = None;
        loop {
            if self.cancel.is_cancelled() {
                break;
            }
            let next = {
                let mut queue = self.queue.lock().await;
                queue.jobs.pop_front().map(|id| {
                    let backfill = queue.backfill.remove(&id);
                    (id, backfill)
                })
            };
            let Some((id, backfill)) = next else {
                let due = last_backfill_look.is_none_or(|at| at.elapsed() >= BACKFILL_IDLE);
                if due {
                    last_backfill_look = Some(Instant::now());
                    match self.plan_backfill().await {
                        Ok(added) if added > 0 => continue,
                        Ok(_) => {}
                        Err(error) => {
                            debug!(error = %error, "Video preview backfill could not look for files")
                        }
                    }
                }
                tokio::select! {
                    _ = self.cancel.cancelled() => break,
                    _ = self.wake.notified() => {}
                    _ = tokio::time::sleep(BACKFILL_IDLE) => {}
                }
                continue;
            };
            let outcome = self.make_previews(&id).await;
            {
                let mut queue = self.queue.lock().await;
                queue.queued.remove(&id);
                if outcome.is_err() {
                    queue.failed.insert(id.clone(), Instant::now());
                }
            }
            if let Err(error) = outcome {
                warn!(
                    media_file_id = %id,
                    error = %error,
                    "Video previews failed; the file is tried again later: media_file_id={}, error={:#}",
                    id,
                    error
                );
            }
            if backfill {
                tokio::select! {
                    _ = self.cancel.cancelled() => break,
                    _ = tokio::time::sleep(BACKFILL_PAUSE) => {}
                }
            }
        }
    }

    /// Detect black bars and draw the storyboard for one file, and record both.
    async fn make_previews(&self, media_file_id: &str) -> Result<()> {
        let db = self.database().await?;
        let Some(file) = MediaFile::get(db.pool(), &media_file_id.to_string())
            .await
            .context("Could not read the media file")?
        else {
            return Ok(());
        };
        if !needs_previews(&file) {
            return Ok(());
        }
        let started = Instant::now();
        let (width, height) = (file.width.unwrap_or(0), file.height.unwrap_or(0));
        let duration = f64::from(file.duration.unwrap_or(0));
        let display_aspect = VideoStream::query(db.pool())
            .filter(VideoStreamWhereInput {
                media_file_id: Some(StringFilter {
                    eq: Some(file.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
            .ok()
            .and_then(|streams| {
                streams
                    .into_iter()
                    .min_by_key(|stream| stream.stream_index)
                    .and_then(|stream| stream.aspect_ratio)
            });
        let tools = Tools {
            ffmpeg: self.config.ffmpeg_path.clone(),
        };
        // One loopback URL for the whole job when the file is on a share (design.md Q87).
        let input = tool_argument(file.path.as_str())
            .await
            .with_context(|| format!("Cannot give ffmpeg access to '{}'", file.path))?;

        let decision = detect_crop(&tools, &input, duration, width, height).await?;
        let crop = decision.crop();
        let aspect = storyboard::picture_aspect(
            width,
            height,
            display_aspect.as_deref(),
            crop.as_ref().map(|crop| (crop.width, crop.height)),
        );
        let storyboard = match storyboard::plan(duration, aspect) {
            Some(layout) => {
                match render_storyboard(
                    &tools,
                    &input,
                    duration,
                    &layout,
                    crop.as_ref(),
                    file.is_hdr,
                )
                .await
                {
                    Ok(sheets) => {
                        let key = content_key(&file, &layout, crop.as_ref());
                        self.save_sheets(&file.id, &key, &sheets).await?;
                        Some(VideoStoryboard {
                            key,
                            interval: layout.interval,
                            tile_width: layout.tile_width as i32,
                            tile_height: layout.tile_height as i32,
                            columns: layout.columns as i32,
                            rows: layout.rows as i32,
                            frame_count: layout.frame_count as i32,
                            sheet_count: layout.sheet_count as i32,
                        })
                    }
                    Err(error) => {
                        warn!(
                            media_file_id = %file.id,
                            media_file_path = %file.path,
                            error = %error,
                            "No storyboard for '{}': {:#}",
                            file.path,
                            error
                        );
                        None
                    }
                }
            }
            None => None,
        };

        // The file may have changed while this ran; then its new version gets its own job.
        let current = MediaFile::get(db.pool(), &file.id)
            .await
            .context("Could not read the media file again")?;
        if current.as_ref().is_none_or(|current| {
            current.size != file.size || current.file_modified_at != file.file_modified_at
        }) {
            info!(
                media_file_id = %file.id,
                media_file_path = %file.path,
                "Video previews discarded because '{}' changed while they were made",
                file.path
            );
            return Ok(());
        }
        self.record(&file.id, crop.as_ref(), storyboard.as_ref())
            .await?;
        info!(
            media_file_id = %file.id,
            media_file_path = %file.path,
            picture = %decision.describe(),
            storyboard_frames = storyboard.as_ref().map_or(0, |board| board.frame_count),
            storyboard_sheets = storyboard.as_ref().map_or(0, |board| board.sheet_count),
            elapsed_secs = started.elapsed().as_secs(),
            "Made video previews for '{}': {}, {} storyboard frames in {}s",
            file.path,
            decision.describe(),
            storyboard.as_ref().map_or(0, |board| board.frame_count),
            started.elapsed().as_secs()
        );
        if matches!(decision, CropDecision::Inconclusive) {
            debug!(media_file_path = %file.path, "Black-bar readings disagreed; no crop kept");
        }
        Ok(())
    }

    /// Write the sheets to `storyboards/<file>/<key>/`, replacing any older set for the file.
    async fn save_sheets(
        &self,
        media_file_id: &str,
        key: &str,
        sheets: &[super::extract::Sheet],
    ) -> Result<()> {
        let file_dir = storyboards_dir(&self.config.cache_path).join(media_file_id);
        let partial = file_dir.join(format!("{key}.partial"));
        let finished = file_dir.join(key);
        let _ = tokio::fs::remove_dir_all(&partial).await;
        tokio::fs::create_dir_all(&partial)
            .await
            .with_context(|| format!("Could not create '{}'", partial.display()))?;
        for sheet in sheets {
            tokio::fs::write(partial.join(format!("{}.jpg", sheet.index)), &sheet.jpeg)
                .await
                .context("Could not write a storyboard sheet")?;
        }
        let _ = tokio::fs::remove_dir_all(&finished).await;
        tokio::fs::rename(&partial, &finished)
            .await
            .context("Could not put the storyboard sheets in place")?;
        if let Ok(mut entries) = tokio::fs::read_dir(&file_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.file_name().to_string_lossy() != key {
                    let _ = tokio::fs::remove_dir_all(entry.path()).await;
                }
            }
        }
        Ok(())
    }

    async fn record(
        &self,
        media_file_id: &str,
        crop: Option<&PictureCrop>,
        storyboard: Option<&VideoStoryboard>,
    ) -> Result<()> {
        let schema = self
            .manager
            .get_graphql()
            .await
            .context("GraphQL is not available")?
            .schema()
            .await
            .context("The GraphQL schema is not available")?;
        crate::services::torrent::database::execute_mutation(
            &schema,
            &AuthUser::system_admin(),
            r#"mutation RecordVideoPreviews($id: String!, $input: UpdateMediaFileInput!) {
                updateMediaFile(id: $id, input: $input) { success error }
            }"#,
            serde_json::json!({
                "id": media_file_id,
                "input": {
                    "pictureCrop": crop,
                    "storyboard": storyboard,
                    "previewsCheckedAt": Utc::now().to_rfc3339(),
                }
            }),
        )
        .await
        .context("Could not record the video previews")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storyboard_keys_and_paths_are_checked() {
        assert!(valid_storyboard_key("0123456789abcdef"));
        assert!(!valid_storyboard_key("../../etc/passwd"));
        assert!(!valid_storyboard_key("0123456789abcdeg"));
        assert_eq!(
            sheet_path(Path::new("/cache"), "f", "0123456789abcdef", 2),
            PathBuf::from("/cache/storyboards/f/0123456789abcdef/2.jpg")
        );
    }
}
