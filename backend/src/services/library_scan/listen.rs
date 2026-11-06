//! Checking a book's chapters by listening for them being announced. Where a narrator says "Chapter
//! Ten", that's where Chapter Ten starts, whatever the listing's time suggests. Names and order stay
//! the listing's, and "Use the listed chapters" puts its times back.

use super::*;
use crate::graphql::entities::{MediaFile, MediaFileWhereInput};
use crate::services::identify_rules::{self as rules, ChapterCue};
use crate::services::whisper::{self, Whisper};

/// Either side of where a chapter is expected. Most are close, so this is short and quick.
const NEAR_SECS: f64 = 60.0;
/// A second look further out, for extra audio earlier in the book that shifted everything.
const FAR_SECS: f64 = 180.0;
/// With nothing heard after this many tries, this narrator doesn't announce chapters.
const GIVE_UP_AFTER: usize = 6;
/// How far back to look for the pause before an announcement. Further can reach a paragraph break.
const PAUSE_SEARCH_SECS: f64 = 3.0;
/// How far either side of where a chapter nobody heard should be to look for a pause.
const UNHEARD_REACH_SECS: f64 = 6.0;
/// With no pause found, start this far before the word rather than clip it.
const LEAD_SECS: f64 = 1.0;
/// Just inside the pause, so the first sound isn't lost.
const INTO_PAUSE_SECS: f64 = 0.15;

impl LibraryScanService {
    /// Check a book's chapters in the background, and say how it went on the book.
    pub fn start_chapter_check(self: &Arc<Self>, audiobook_id: &str) {
        let this = self.clone();
        let id = audiobook_id.to_string();
        tokio::spawn(async move {
            let write = |state: &'static str, text: Option<String>| {
                let this = this.clone();
                let id = id.clone();
                async move {
                    if let Err(error) = this
                        .update_audiobook_fields(
                            &AuthUser::system_admin(),
                            &id,
                            serde_json::json!({ "chapterCheckState": state, "chapterCheck": text }),
                        )
                        .await
                    {
                        warn!(audiobook_id = %id, error = %error, "Could not record a chapter check");
                    }
                }
            };
            write("listening", None).await;
            let Ok(_turn) = this.listen_slot.clone().acquire_owned().await else {
                return;
            };
            let outcome = async {
                let whisper = Whisper::from_env();
                whisper
                    .check()
                    .await
                    .map_err(|reason| anyhow::anyhow!(reason))?;
                this.check_book_chapters(&id, &whisper).await
            }
            .await;
            match outcome {
                Ok(report) => write("done", Some(report)).await,
                Err(error) => write("failed", Some(error.to_string())).await,
            }
        });
    }

    /// A check that was running when the server stopped won't finish on its own.
    pub(super) async fn clear_interrupted_chapter_checks(&self) {
        use crate::graphql::entities::{Audiobook, AudiobookWhereInput};
        let Ok(database) = self.database().await else {
            return;
        };
        let books = Audiobook::query(database.pool().pool())
            .filter(AudiobookWhereInput::default())
            .fetch_all()
            .await
            .unwrap_or_default();
        for book in books {
            if book.chapter_check_state.as_deref() == Some("listening")
                && let Err(error) = self
                    .update_audiobook_fields(
                        &AuthUser::system_admin(),
                        &book.id,
                        serde_json::json!({
                            "chapterCheckState": "failed",
                            "chapterCheck": "The server restarted before the check finished.",
                        }),
                    )
                    .await
            {
                warn!(audiobook_id = %book.id, error = %error, "Could not clear a chapter check");
            }
        }
    }

    /// A book's files in playing order, with their real lengths.
    pub(super) async fn book_files_in_order(
        &self,
        chapters: &[crate::graphql::entities::Chapter],
    ) -> Result<Vec<MediaFile>> {
        let database = self.database().await?;
        let ids: Vec<String> = chapters.iter().map(|chapter| chapter.id.clone()).collect();
        let mut files = MediaFile::query(database.pool().pool())
            .filter(MediaFileWhereInput {
                chapter_id: Some(StringFilter {
                    in_list: Some(ids),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        // A file holding many chapters names only one of them, and a chapter can point at it anyway.
        let known: HashSet<String> = files.iter().map(|file| file.id.clone()).collect();
        let others: HashSet<&String> = chapters
            .iter()
            .filter_map(|chapter| chapter.media_file_id.as_ref())
            .filter(|id| !known.contains(*id))
            .collect();
        for id in others {
            if let Some(file) = MediaFile::get(database.pool().pool(), id).await? {
                files.push(file);
            }
        }
        files.sort_by(|a, b| Self::natural_sort_key(&a.path).cmp(&Self::natural_sort_key(&b.path)));
        Ok(files)
    }

    /// Point each file at the chapter it begins in, so a file stays part of the book when chapters
    /// are renumbered or removed. Runs before chapters are placed, which then point back at files.
    pub(super) async fn relink_book_files(
        &self,
        files: &[MediaFile],
        layout: &release_match::BookLayout,
        chapter_ids: &[String],
    ) -> Result<()> {
        let holders = release_match::chapter_of_each_file(layout, files.len());
        for (file, holder) in files.iter().zip(holders) {
            let Some(chapter_id) = holder.and_then(|holder| chapter_ids.get(holder)) else {
                continue;
            };
            if file.chapter_id.as_deref() == Some(chapter_id.as_str()) {
                continue;
            }
            let data = self
                .execute_mutation(
                    &AuthUser::system_admin(),
                    r#"mutation RelinkBookFile($id: String!, $input: UpdateMediaFileInput!) {
                        Updated: updateMediaFile(id: $id, input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "id": file.id, "input": { "chapterId": chapter_id } }),
                )
                .await?;
            anyhow::ensure!(
                data.pointer("/Updated/Success")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true),
                "Could not move a file to its chapter"
            );
        }
        Ok(())
    }

    /// Listen at each chapter's start, and move the chapters if enough were heard.
    pub(super) async fn check_book_chapters(
        &self,
        audiobook_id: &str,
        whisper: &Whisper,
    ) -> Result<String> {
        let owner = AuthUser::system_admin();
        let (book, chapters, facts, reference) = self.book_reference(audiobook_id).await?;
        anyhow::ensure!(
            !reference.is_empty(),
            "This book has no listed chapters to check"
        );
        if let Some(source) = book.chapter_source.as_deref() {
            anyhow::ensure!(
                matches!(source, "provider" | "listened"),
                "These chapters come from the files themselves"
            );
        }
        let files = self.book_files_in_order(&chapters).await?;
        anyhow::ensure!(!files.is_empty(), "This book has no files to listen to");
        let mut timeline = Vec::with_capacity(files.len());
        for file in &files {
            let probed = ffprobe_analyze(&file.path)
                .await
                .ok()
                .and_then(|found| found.duration_secs);
            let seconds = probed
                .or(file.duration.map(f64::from))
                .ok_or_else(|| anyhow::anyhow!("Can't tell how long {} is", file.path))?;
            timeline.push((file.path.clone(), seconds));
        }
        let total: f64 = timeline.iter().map(|(_, seconds)| seconds).sum();
        let listed = facts
            .seconds
            .filter(|seconds| *seconds > 0.0)
            .unwrap_or_else(|| {
                reference
                    .iter()
                    .filter_map(|chapter| chapter.seconds.map(|seconds| chapter.start + seconds))
                    .fold(0.0, f64::max)
            });
        let scale = if listed > 0.0 { total / listed } else { 1.0 };

        let cues: Vec<Option<ChapterCue>> = reference
            .iter()
            .map(|chapter| chapter.title.as_deref().and_then(rules::chapter_cue))
            .collect();
        let cue_count = cues.iter().flatten().count();
        let mut heard: Vec<Option<f64>> = vec![None; reference.len()];
        let mut shift = 0.0;
        let mut tried = 0;
        let mut found = 0;
        let clip = std::env::temp_dir().join(format!("librarian-listen-{}.wav", Uuid::new_v4()));
        let outcome: Result<()> = async {
            for (index, cue) in cues.iter().enumerate() {
                let Some(cue) = cue else {
                    continue;
                };
                if found == 0 && tried >= GIVE_UP_AFTER {
                    break;
                }
                tried += 1;
                let expected = reference[index].start * scale + shift;
                for reach in [NEAR_SECS, FAR_SECS] {
                    let from = (expected - reach).max(0.0);
                    let length = (reach * 2.0).min((total - from).max(0.0));
                    if length <= 0.0 {
                        break;
                    }
                    whisper::cut_clip(&timeline, from, length, &clip).await?;
                    let words: Vec<rules::HeardWord> = whisper
                        .words(&clip)
                        .await?
                        .into_iter()
                        .map(|word| rules::HeardWord {
                            text: word.text,
                            start: word.start + from,
                        })
                        .collect();
                    if let Some(word_at) = rules::find_cue(&words, cue, expected) {
                        let search_from = (word_at - PAUSE_SEARCH_SECS).max(0.0);
                        whisper::cut_clip(
                            &timeline,
                            search_from,
                            word_at - search_from + 1.0,
                            &clip,
                        )
                        .await?;
                        let pauses = whisper::pauses(&clip).await.unwrap_or_default();
                        let word_in_clip = word_at - search_from;
                        let at = match rules::start_of_announcement(word_in_clip, &pauses) {
                            Some(end) => search_from + end - INTO_PAUSE_SECS,
                            None => word_at - LEAD_SECS,
                        }
                        .max(0.0);
                        heard[index] = Some(at);
                        shift = at - reference[index].start * scale;
                        found += 1;
                        break;
                    }
                }
            }
            Ok(())
        }
        .await;
        outcome?;

        if !rules::enough_heard(cue_count, found) {
            let _ = tokio::fs::remove_file(&clip).await;
            return Ok(if cue_count == 0 {
                "These chapter names aren't things a narrator would say, so their times are unchanged.".to_string()
            } else {
                format!(
                    "Heard {found} of {cue_count} chapters being announced, which isn't enough to go on, so their times are unchanged."
                )
            });
        }

        let mut starts = rules::settle_chapter_starts(&reference, scale, &heard, total);
        // Chapters not heard, or that nobody announces, still follow a pause.
        for (index, start) in starts.iter_mut().enumerate() {
            if heard[index].is_some() || *start <= 0.0 {
                continue;
            }
            let from = (*start - UNHEARD_REACH_SECS).max(0.0);
            let snapped = async {
                whisper::cut_clip(&timeline, from, UNHEARD_REACH_SECS * 2.0, &clip).await?;
                whisper::pauses(&clip).await
            }
            .await;
            if let Ok(pauses) = snapped
                && let Some(end) = rules::break_near(*start - from, UNHEARD_REACH_SECS, &pauses)
            {
                *start = from + end - INTO_PAUSE_SECS;
            }
        }
        let _ = tokio::fs::remove_file(&clip).await;
        let durations: Vec<f64> = timeline.iter().map(|(_, seconds)| *seconds).collect();
        let layout = release_match::listened_layout(&durations, &reference, &starts);
        let largest = chapters
            .iter()
            .zip(&starts)
            .map(|(chapter, start)| (chapter.start_time_secs - start).abs())
            .fold(0.0, f64::max);
        let moved = chapters
            .iter()
            .zip(&starts)
            .filter(|(chapter, start)| (chapter.start_time_secs - **start).abs() >= 1.0)
            .count();
        let ids = self
            .rewrite_book_chapters(&owner, &book, &chapters, &reference, &layout, true)
            .await?;
        self.relink_book_files(&files, &layout, &ids).await?;
        let placements = layout
            .chapters
            .iter()
            .enumerate()
            .filter_map(|(index, chapter)| {
                Some((
                    ids.get(index)?.clone(),
                    files.get(chapter.file?)?.id.clone(),
                    chapter.offset,
                ))
            })
            .collect();
        let imported: HashMap<String, String> = files
            .iter()
            .map(|file| (file.id.clone(), file.id.clone()))
            .collect();
        self.write_book_chapters(
            &owner,
            &BookWrites {
                audiobook_id: book.id.clone(),
                placements,
                source: layout.source.as_str(),
                shape: layout.shape.as_str(),
            },
            &imported,
        )
        .await?;
        Ok(format!(
            "Heard {found} of {cue_count} chapters being announced. Moved {moved}, by up to {} seconds.",
            largest.round()
        ))
    }
}
