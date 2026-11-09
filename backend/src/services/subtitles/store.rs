//! `Subtitle` rows for sidecar and downloaded files, kept in step with what is on disk.
//!
//! Rows are written through the generated mutations as the system user. Embedded rows belong
//! to analysis (`replace_media_analysis`) and are never touched here. A downloaded subtitle is a
//! sidecar file too; it keeps its `downloaded` row (with its OpenSubtitles id) and is never also
//! listed as a sidecar.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use graphql_orm::graphql::filters::StringFilter;
use serde_json::json;

use crate::db::Database;
use crate::services::graphql::LibrarianSchema;
use crate::services::graphql::entities::{Subtitle, SubtitleWhereInput};
use crate::services::library_storage::{EntryKind, StoragePath, library_storage};
use crate::services::torrent::database::{execute_mutation, system_auth_user};

use super::sidecar::{self, SidecarFile, VideoIndex, is_sidecar_extension, is_subs_folder};

pub const SOURCE_SIDECAR: &str = "sidecar";
pub const SOURCE_DOWNLOADED: &str = "downloaded";

/// Video extensions, to tell a folder's videos apart when placing its subtitles.
const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "m4v", "mov", "wmv", "flv", "webm", "mpeg", "mpg", "ts", "m2ts",
];

pub fn is_video_extension(extension: &str) -> bool {
    VIDEO_EXTENSIONS
        .iter()
        .any(|known| extension.eq_ignore_ascii_case(known))
}

/// What a sync changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncCounts {
    pub created: usize,
    pub updated: usize,
    pub removed: usize,
}

impl SyncCounts {
    pub fn add(&mut self, other: SyncCounts) {
        self.created += other.created;
        self.updated += other.updated;
        self.removed += other.removed;
    }
}

/// The rows of a file that have a file of their own (sidecar and downloaded).
pub async fn file_rows(db: &Database, media_file_id: &str) -> Result<Vec<Subtitle>> {
    Ok(Subtitle::query(db.pool())
        .filter(SubtitleWhereInput {
            media_file_id: Some(StringFilter {
                eq: Some(media_file_id.to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .filter(|row| row.file_path.is_some() && row.source_type != "embedded")
        .collect())
}

/// The changes that make a file's rows match the sidecars found for it.
#[derive(Debug, Default, PartialEq)]
pub struct SyncPlan {
    pub create: Vec<SidecarFile>,
    /// Row id and the sidecar it should now describe.
    pub update: Vec<(String, SidecarFile)>,
    pub remove: Vec<String>,
}

/// Plan a sync. Paths compare without case, so a share that reports another case is the same.
pub fn plan_sync(rows: &[Subtitle], found: &[SidecarFile]) -> SyncPlan {
    let key = |path: &str| path.to_lowercase();
    let found_by_path: HashMap<String, &SidecarFile> =
        found.iter().map(|file| (key(&file.path), file)).collect();
    let mut plan = SyncPlan::default();
    let mut claimed = HashSet::new();
    for row in rows {
        let Some(path) = row.file_path.as_deref() else {
            continue;
        };
        let path_key = key(path);
        let Some(file) = found_by_path.get(&path_key) else {
            plan.remove.push(row.id.clone());
            continue;
        };
        if !claimed.insert(path_key) {
            // Two rows for one file: keep the first.
            plan.remove.push(row.id.clone());
            continue;
        }
        if row.source_type == SOURCE_SIDECAR
            && (row.language.as_deref() != file.facts.language
                || row.is_forced != file.facts.forced
                || row.is_hearing_impaired != file.facts.hearing_impaired
                || row.is_default != file.facts.default
                || row.codec.as_deref() != Some(file.codec)
                || row.title != file.facts.title)
        {
            plan.update.push((row.id.clone(), (*file).clone()));
        }
    }
    for file in found {
        if !claimed.contains(&key(&file.path)) {
            claimed.insert(key(&file.path));
            plan.create.push(file.clone());
        }
    }
    plan
}

fn sidecar_input(file: &SidecarFile) -> serde_json::Value {
    json!({
        "sourceType": SOURCE_SIDECAR,
        "filePath": file.path,
        "codec": file.codec,
        "codecLongName": if file.bitmap { Some("VobSub (DVD) subtitles") } else { None },
        "language": file.facts.language,
        "title": file.facts.title,
        "isDefault": file.facts.default,
        "isForced": file.facts.forced,
        "isHearingImpaired": file.facts.hearing_impaired,
    })
}

/// Make a file's sidecar rows match `found`.
pub async fn sync_rows(
    db: &Database,
    schema: &LibrarianSchema,
    media_file_id: &str,
    found: &[SidecarFile],
) -> Result<SyncCounts> {
    let rows = file_rows(db, media_file_id).await?;
    let plan = plan_sync(&rows, found);
    let auth = system_auth_user();
    let mut counts = SyncCounts::default();
    for file in &plan.create {
        let mut input = sidecar_input(file);
        input["mediaFileId"] = json!(media_file_id);
        execute_mutation(
            schema,
            &auth,
            r#"mutation CreateSidecarSubtitle($input: CreateSubtitleInput!) {
                CreateSubtitle: createSubtitle(input: $input) { success error }
            }"#,
            json!({ "input": input }),
        )
        .await?;
        counts.created += 1;
    }
    for (id, file) in &plan.update {
        execute_mutation(
            schema,
            &auth,
            r#"mutation UpdateSidecarSubtitle($id: String!, $input: UpdateSubtitleInput!) {
                UpdateSubtitle: updateSubtitle(id: $id, input: $input) { success error }
            }"#,
            json!({ "id": id, "input": sidecar_input(file) }),
        )
        .await?;
        counts.updated += 1;
    }
    for id in &plan.remove {
        execute_mutation(
            schema,
            &auth,
            r#"mutation DeleteSidecarSubtitle($id: String!) {
                DeleteSubtitle: deleteSubtitle(id: $id) { success error }
            }"#,
            json!({ "id": id }),
        )
        .await?;
        counts.removed += 1;
    }
    Ok(counts)
}

/// Point a row at a moved file.
pub async fn move_row(schema: &LibrarianSchema, id: &str, new_path: &str) -> Result<()> {
    execute_mutation(
        schema,
        &system_auth_user(),
        r#"mutation MoveSubtitle($id: String!, $input: UpdateSubtitleInput!) {
            UpdateSubtitle: updateSubtitle(id: $id, input: $input) { success error }
        }"#,
        json!({ "id": id, "input": { "filePath": new_path } }),
    )
    .await?;
    Ok(())
}

/// The subtitle files around one video and the videos beside it: its folder, `Subs`-style
/// folders in it, and folders inside those. Read through the storage layer, so it works on a
/// share as on a local disk.
pub async fn look_around(video_path: &str) -> Result<(Vec<String>, Vec<String>)> {
    let video = StoragePath::new(video_path);
    let Some(folder) = video.parent() else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut videos = Vec::new();
    let mut subtitles = Vec::new();
    let extension_of = |name: &str| {
        name.rsplit_once('.')
            .map(|(_, ext)| ext.to_owned())
            .unwrap_or_default()
    };
    for entry in library_storage().list_dir(&folder).await? {
        match entry.kind {
            EntryKind::File if is_video_extension(&extension_of(&entry.name)) => {
                videos.push(folder.join(&entry.name).into_string());
            }
            EntryKind::File if is_sidecar_extension(&extension_of(&entry.name)) => {
                subtitles.push(folder.join(&entry.name).into_string());
            }
            EntryKind::Directory if is_subs_folder(&entry.name) => {
                let subs = folder.join(&entry.name);
                for inner in library_storage().list_dir(&subs).await.unwrap_or_default() {
                    match inner.kind {
                        EntryKind::File if is_sidecar_extension(&extension_of(&inner.name)) => {
                            subtitles.push(subs.join(&inner.name).into_string());
                        }
                        EntryKind::Directory => {
                            let nested = subs.join(&inner.name);
                            for file in library_storage()
                                .list_dir(&nested)
                                .await
                                .unwrap_or_default()
                            {
                                if file.kind == EntryKind::File
                                    && is_sidecar_extension(&extension_of(&file.name))
                                {
                                    subtitles.push(nested.join(&file.name).into_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if !videos.iter().any(|path| path == video_path) {
        videos.push(video_path.to_owned());
    }
    Ok((videos, subtitles))
}

/// The sidecars of one video, found on disk now.
pub async fn discover(video_path: &str) -> Result<Vec<SidecarFile>> {
    let (videos, subtitles) = look_around(video_path).await?;
    let index = VideoIndex::new(&videos);
    Ok(sidecar::assign(&index, &subtitles)
        .into_iter()
        .filter(|(video, _)| video == video_path)
        .map(|(_, file)| file)
        .collect())
}

/// Find one video's sidecars on disk and store them. A folder that cannot be read leaves the
/// rows as they are.
pub async fn sync_video(
    db: &Database,
    schema: &LibrarianSchema,
    media_file_id: &str,
    video_path: &str,
) -> Result<SyncCounts> {
    let found = discover(video_path).await?;
    sync_rows(db, schema, media_file_id, &found).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::subtitles::sidecar::NameFacts;

    fn row(id: &str, source: &str, path: &str, language: Option<&str>) -> Subtitle {
        Subtitle {
            id: id.into(),
            media_file_id: "f".into(),
            source_type: source.into(),
            stream_index: None,
            file_path: Some(path.into()),
            codec: Some("subrip".into()),
            codec_long_name: None,
            language: language.map(str::to_owned),
            title: None,
            is_default: false,
            is_forced: false,
            is_hearing_impaired: false,
            opensubtitles_id: None,
            downloaded_at: None,
            metadata: None,
            created_at: String::new(),
            updated_at: String::new(),
            media_file: None,
        }
    }

    fn file(path: &str, language: Option<&'static str>, forced: bool) -> SidecarFile {
        SidecarFile {
            path: path.into(),
            codec: "subrip",
            bitmap: false,
            facts: NameFacts {
                language,
                forced,
                ..Default::default()
            },
        }
    }

    #[test]
    fn sync_creates_updates_removes_and_keeps_downloads() {
        let rows = vec![
            row("keep", "sidecar", "/m/F.en.srt", Some("eng")),
            row("changed", "sidecar", "/m/F.de.srt", None),
            row("gone", "sidecar", "/m/F.fr.srt", Some("fre")),
            row("download", "downloaded", "/m/F.es.srt", Some("spa")),
            row("download-gone", "downloaded", "/m/F.it.srt", Some("ita")),
        ];
        let found = vec![
            file("/m/F.en.srt", Some("eng"), false),
            file("/m/F.DE.srt", Some("ger"), true),
            file("/m/F.es.srt", Some("spa"), false),
            file("/m/F.nl.srt", Some("dut"), false),
        ];
        let plan = plan_sync(&rows, &found);
        assert_eq!(
            plan.create
                .iter()
                .map(|f| f.path.as_str())
                .collect::<Vec<_>>(),
            vec!["/m/F.nl.srt"]
        );
        assert_eq!(plan.update.len(), 1);
        assert_eq!(plan.update[0].0, "changed");
        assert!(plan.update[0].1.facts.forced);
        assert_eq!(
            plan.remove,
            vec!["gone".to_owned(), "download-gone".to_owned()]
        );
    }

    /// Write empty files at these paths under `root`.
    fn layout(root: &std::path::Path, files: &[&str]) {
        for file in files {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"x").unwrap();
        }
    }

    fn found_names(
        found: &[SidecarFile],
        root: &std::path::Path,
    ) -> Vec<(String, Option<&'static str>, bool, bool)> {
        let mut names = found
            .iter()
            .map(|file| {
                (
                    file.path
                        .strip_prefix(&*root.to_string_lossy())
                        .unwrap()
                        .trim_start_matches('/')
                        .to_owned(),
                    file.facts.language,
                    file.facts.forced,
                    file.facts.hearing_impaired,
                )
            })
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[tokio::test]
    async fn torrent_layouts_find_each_videos_subtitles() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // A movie with subtitles beside it, `.ass` and a VobSub pair among them.
        layout(
            root,
            &[
                "Movie.2020.1080p/Movie.2020.1080p.mkv",
                "Movie.2020.1080p/Movie.2020.1080p.en.srt",
                "Movie.2020.1080p/Movie.2020.1080p.en.forced.srt",
                "Movie.2020.1080p/Movie.2020.1080p.ja.ass",
                "Movie.2020.1080p/Movie.2020.1080p.idx",
                "Movie.2020.1080p/Movie.2020.1080p.sub",
                "Movie.2020.1080p/Sample/sample.mkv",
            ],
        );
        let video = root
            .join("Movie.2020.1080p/Movie.2020.1080p.mkv")
            .to_string_lossy()
            .into_owned();
        let found = discover(&video).await.unwrap();
        assert_eq!(
            found_names(&found, root),
            vec![
                (
                    "Movie.2020.1080p/Movie.2020.1080p.en.forced.srt".to_owned(),
                    Some("eng"),
                    true,
                    false
                ),
                (
                    "Movie.2020.1080p/Movie.2020.1080p.en.srt".to_owned(),
                    Some("eng"),
                    false,
                    false
                ),
                (
                    "Movie.2020.1080p/Movie.2020.1080p.idx".to_owned(),
                    None,
                    false,
                    false
                ),
                (
                    "Movie.2020.1080p/Movie.2020.1080p.ja.ass".to_owned(),
                    Some("jpn"),
                    false,
                    false
                ),
            ]
        );
        assert!(
            found
                .iter()
                .any(|file| file.bitmap && file.path.ends_with(".idx"))
        );

        // A single-video torrent with a `Subs` folder named by language.
        layout(
            root,
            &[
                "Film/Film.mkv",
                "Film/Subs/English.srt",
                "Film/Subs/2_English.srt",
                "Film/Subs/3_Spanish.srt",
                "Film/Subs/4_English_SDH.srt",
            ],
        );
        let film = root.join("Film/Film.mkv").to_string_lossy().into_owned();
        assert_eq!(
            found_names(&discover(&film).await.unwrap(), root),
            vec![
                (
                    "Film/Subs/2_English.srt".to_owned(),
                    Some("eng"),
                    false,
                    false
                ),
                (
                    "Film/Subs/3_Spanish.srt".to_owned(),
                    Some("spa"),
                    false,
                    false
                ),
                (
                    "Film/Subs/4_English_SDH.srt".to_owned(),
                    Some("eng"),
                    false,
                    true
                ),
                (
                    "Film/Subs/English.srt".to_owned(),
                    Some("eng"),
                    false,
                    false
                ),
            ]
        );

        // A season pack: `Subs/<episode>/` per episode, and nothing shared between episodes.
        layout(
            root,
            &[
                "Show.S01/Show.S01E01.mkv",
                "Show.S01/Show.S01E02.mkv",
                "Show.S01/Subs/Show.S01E01/2_English.srt",
                "Show.S01/Subs/Show.S01E02/2_English.srt",
                "Show.S01/Subs/Show.S01E02/3_French.srt",
            ],
        );
        let second = root
            .join("Show.S01/Show.S01E02.mkv")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            found_names(&discover(&second).await.unwrap(), root),
            vec![
                (
                    "Show.S01/Subs/Show.S01E02/2_English.srt".to_owned(),
                    Some("eng"),
                    false,
                    false
                ),
                (
                    "Show.S01/Subs/Show.S01E02/3_French.srt".to_owned(),
                    Some("fre"),
                    false,
                    false
                ),
            ]
        );
        let first = root
            .join("Show.S01/Show.S01E01.mkv")
            .to_string_lossy()
            .into_owned();
        assert_eq!(discover(&first).await.unwrap().len(), 1);
    }
}
