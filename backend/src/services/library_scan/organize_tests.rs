//! Where matched files end up: one folder per artist, files a scan skipped as unchanged still
//! organized, and the folders organizing empties removed. See `docs/design.md` (Q27, Q63-Q65).

use super::*;
use crate::graphql::entities::{
    Album, Artist, CreateAlbumInput, CreateArtistInput, CreateLibraryInput, CreateMediaFileInput,
    CreateMovieInput, CreateTrackInput, CreateUserInput, Library, LibraryScanRun, MediaFile,
    MediaFileWhereInput, Movie, Track, User,
};
use graphql_orm::graphql::filters::StringFilter;

const MUSIC_PATTERN: &str = "{artist}/{album} ({year})/{track:02} - {title}.{ext}";
const MOVIE_PATTERN: &str = "{title} ({year})/{title} ({year}).{ext}";

// ---- Pure rules ----------------------------------------------------------------------------

fn track_facts(artist: Option<&str>) -> TargetFacts {
    TargetFacts::Track {
        title: "Welcome to the Jungle".to_string(),
        album_name: "Appetite for Destruction".to_string(),
        album_year: Some(1987),
        track_number: 1,
        disc_number: Some(1),
        discs: 1,
        artist_name: artist.map(str::to_string),
    }
}

#[test]
fn an_artist_has_one_folder_whichever_apostrophe_a_source_spells_it_with() {
    let plain = track_facts(Some("Guns N' Roses"))
        .relative_path(MUSIC_PATTERN, "01.mp3", "mp3")
        .unwrap();
    let curly = track_facts(Some("Guns N\u{2019} Roses"))
        .relative_path(MUSIC_PATTERN, "01.mp3", "mp3")
        .unwrap();
    let opening = track_facts(Some("Guns N\u{2018} Roses"))
        .relative_path(MUSIC_PATTERN, "01.mp3", "mp3")
        .unwrap();
    assert_eq!(
        plain,
        PathBuf::from(
            "Guns N' Roses/Appetite for Destruction (1987)/01 - Welcome to the Jungle.mp3"
        )
    );
    assert_eq!(plain, curly);
    assert_eq!(plain, opening);
    // The same sanitising covers every name, so the title and album agree too.
    assert_eq!(
        sanitize_for_filename("Don\u{2019}t Cry: Live?"),
        "Don't Cry_ Live_"
    );
}

#[test]
fn a_placeholder_artist_is_never_a_folder() {
    for artist in [
        None,
        Some(""),
        Some("  "),
        Some("Unknown Artist"),
        Some("unknown artist"),
    ] {
        let error = track_facts(artist)
            .relative_path(MUSIC_PATTERN, "01.mp3", "mp3")
            .expect_err("no folder for an unknown artist");
        assert_eq!(error.to_string(), ARTIST_UNKNOWN_REASON, "{artist:?}");
    }
    // A real name that merely contains the words is fine.
    assert!(
        track_facts(Some("Unknown Artist Collective"))
            .relative_path(MUSIC_PATTERN, "01.mp3", "mp3")
            .is_ok()
    );
}

#[test]
fn only_the_albums_artist_record_names_the_folder() {
    // `TargetFacts::Track` carries no per-track artist at all: a guest credit on one track can
    // not put that track in a folder of its own.
    let facts = track_facts(Some("Guns N' Roses"));
    let TargetFacts::Track { artist_name, .. } = &facts else {
        unreachable!()
    };
    assert_eq!(artist_name.as_deref(), Some("Guns N' Roses"));
}

// ---- Empty folders -------------------------------------------------------------------------

#[tokio::test]
async fn cleanup_removes_whole_empty_trees_and_never_the_root() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    std::fs::create_dir_all(root.join("Old Artist/Old Album/Disc 1"))?;
    std::fs::create_dir_all(root.join("Old Artist/Another Album"))?;
    std::fs::create_dir_all(root.join("Kept Artist/Kept Album"))?;
    std::fs::write(root.join("Kept Artist/Kept Album/01.mp3"), b"audio")?;

    LibraryScanService::cleanup_empty_unprotected_folders(root, &HashSet::new()).await?;

    assert!(
        !root.join("Old Artist").exists(),
        "the whole empty tree goes"
    );
    assert!(root.join("Kept Artist/Kept Album/01.mp3").exists());
    assert!(root.exists(), "never the library root");
    Ok(())
}

#[tokio::test]
async fn cleanup_keeps_a_folder_with_any_file_hidden_ones_included() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    std::fs::create_dir_all(root.join("Artist/Album"))?;
    std::fs::write(root.join("Artist/Album/.DS_Store"), b"finder")?;
    std::fs::create_dir_all(root.join("Other/Nested"))?;
    std::fs::write(root.join("Other/cover.jpg"), b"art")?;

    LibraryScanService::cleanup_empty_unprotected_folders(root, &HashSet::new()).await?;

    assert!(root.join("Artist/Album/.DS_Store").exists());
    assert!(root.join("Other/cover.jpg").exists());
    assert!(
        !root.join("Other/Nested").exists(),
        "its empty child still goes"
    );
    Ok(())
}

#[tokio::test]
async fn cleanup_never_touches_the_trash_or_other_hidden_folders() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    // Trash with a file, and a nested empty folder inside it.
    std::fs::create_dir_all(root.join(LIBRARY_TRASH_DIR).join("emptied"))?;
    std::fs::write(
        root.join(LIBRARY_TRASH_DIR).join("1234-Track.mp3"),
        b"recoverable",
    )?;
    // An empty hidden folder, as a sync tool leaves, and a folder holding only one.
    std::fs::create_dir_all(root.join(".stfolder"))?;
    std::fs::create_dir_all(root.join("Synced/.stfolder"))?;
    // An empty trash is still left alone.
    let empty_library = tempfile::tempdir()?;
    std::fs::create_dir_all(empty_library.path().join(LIBRARY_TRASH_DIR))?;

    LibraryScanService::cleanup_empty_unprotected_folders(root, &HashSet::new()).await?;
    LibraryScanService::cleanup_empty_unprotected_folders(empty_library.path(), &HashSet::new())
        .await?;

    assert!(root.join(LIBRARY_TRASH_DIR).join("1234-Track.mp3").exists());
    assert!(root.join(LIBRARY_TRASH_DIR).join("emptied").exists());
    assert!(root.join(".stfolder").exists());
    assert!(root.join("Synced/.stfolder").exists());
    assert!(empty_library.path().join(LIBRARY_TRASH_DIR).exists());
    Ok(())
}

#[test]
fn removing_upward_stops_at_a_folder_in_use_the_root_and_expected_folders() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    std::fs::create_dir_all(root.join("Artist/Album/Disc 1"))?;
    std::fs::create_dir_all(root.join("Artist/Other Album"))?;
    std::fs::write(root.join("Artist/Other Album/01.mp3"), b"audio")?;

    // The empty chain goes as far as the first folder with something in it.
    let removed = remove_empty_folders_upward(
        root,
        &root.join("Artist/Album/Disc 1"),
        &HashSet::from([root.to_path_buf()]),
    );
    assert_eq!(
        removed,
        vec![root.join("Artist/Album/Disc 1"), root.join("Artist/Album")]
    );
    assert!(root.join("Artist/Other Album/01.mp3").exists());

    // Everything empty goes, but not the root.
    std::fs::remove_file(root.join("Artist/Other Album/01.mp3"))?;
    let removed = remove_empty_folders_upward(
        root,
        &root.join("Artist/Other Album"),
        &HashSet::from([root.to_path_buf()]),
    );
    assert_eq!(removed.len(), 2);
    assert!(root.exists());
    assert!(!root.join("Artist").exists());

    // A folder a library item expects stays, and so does everything above it.
    std::fs::create_dir_all(root.join("Show/Season 01"))?;
    let removed = remove_empty_folders_upward(
        root,
        &root.join("Show/Season 01"),
        &HashSet::from([root.join("Show/Season 01")]),
    );
    assert!(removed.is_empty());
    assert!(root.join("Show/Season 01").exists());
    Ok(())
}

// ---- Scans and organize, on a real database ----------------------------------------------

/// The entity futures are deeper than a debug test thread's default stack; production runs
/// them on 8 MiB (see `main.rs`), so these tests do too.
fn on_big_stack<F>(name: &str, test: fn() -> F) -> anyhow::Result<()>
where
    F: std::future::Future<Output = anyhow::Result<()>> + 'static,
{
    std::thread::Builder::new()
        .name(name.to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(test())
        })?
        .join()
        .map_err(|_| anyhow::anyhow!("{name} panicked"))?
}

struct Harness {
    _temp: tempfile::TempDir,
    services: Arc<crate::services::ServicesManager>,
    database: Arc<crate::services::DatabaseService>,
    scanner: Arc<LibraryScanService>,
    library: Library,
    root: PathBuf,
}

impl Harness {
    async fn new(
        library_type: &str,
        naming_pattern: &str,
        auto_organize: bool,
    ) -> anyhow::Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("library");
        std::fs::create_dir_all(&root)?;
        let services = crate::services::ServicesManager::builder()
            .add_service(crate::services::DatabaseServiceConfig {
                database_url: format!("sqlite://{}", temp.path().join("librarian.db").display()),
                connect_timeout: Duration::from_secs(5),
            })
            .add_service(crate::services::AuthConfig::for_tests())
            .add_service(crate::services::GraphqlServiceConfig { server_port: 0 })
            .add_service(LibraryScanServiceConfig {
                autoscan_poll_interval: Duration::from_secs(3600),
                analyze_workers: 1,
            })
            .start()
            .await?;
        let database = services
            .get_database()
            .await
            .context("database service missing")?;
        let user = User::insert(
            database.pool(),
            CreateUserInput {
                username: "organize-owner".to_string(),
                email: Some("organize-owner@example.test".to_string()),
                password_hash: "test-only".to_string(),
                role: "Member".to_string(),
                display_name: None,
                avatar_url: None,
                is_active: true,
                last_login_at: None,
            },
        )
        .await?;
        let library = Library::insert(
            database.pool(),
            CreateLibraryInput {
                user_id: user.id,
                name: format!("{library_type} organize test"),
                path: root.to_string_lossy().to_string(),
                library_type: library_type.to_string(),
                icon: None,
                color: None,
                auto_scan: false,
                auto_organize,
                auto_resolve_duplicates: None,
                naming_pattern: naming_pattern.to_string(),
                scan_interval_minutes: 60,
                watch_for_changes: false,
                scanning: false,
                last_scanned_at: None,
                quality_profile_id: None,
            },
        )
        .await?;
        let scanner = services
            .get_library_scan()
            .await
            .context("library scan service missing")?;
        Ok(Self {
            _temp: temp,
            services,
            database,
            scanner,
            library,
            root,
        })
    }

    fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        path
    }

    async fn scan(&self) -> anyhow::Result<LibraryScanRun> {
        let run_id = self
            .scanner
            .queue_scan(&self.library.id)
            .await?
            .scan_run_id
            .context("scan did not return a run id")?;
        for _ in 0..1200 {
            if let Some(run) = LibraryScanRun::get(self.database.pool().pool(), &run_id).await?
                && matches!(
                    run.status.as_str(),
                    "COMPLETED" | "COMPLETED_WITH_ISSUES" | "FAILED" | "CANCELLED"
                )
            {
                return Ok(run);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        anyhow::bail!("scan did not finish in time")
    }

    async fn media_files(&self) -> anyhow::Result<Vec<MediaFile>> {
        Ok(MediaFile::query(self.database.pool().pool())
            .filter(MediaFileWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(self.library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?)
    }

    async fn media_file_at(&self, path: &Path) -> anyhow::Result<MediaFile> {
        self.media_files()
            .await?
            .into_iter()
            .find(|file| Path::new(&file.path) == path)
            .with_context(|| format!("no media file at {}", path.display()))
    }

    async fn movie(&self, title: &str, year: i32) -> anyhow::Result<Movie> {
        Ok(Movie::insert(
            self.database.pool(),
            CreateMovieInput {
                locked_fields: None,
                library_id: self.library.id.clone(),
                user_id: self.library.user_id.clone(),
                title: title.to_string(),
                sort_title: None,
                original_title: None,
                year: Some(year),
                tmdb_id: None,
                imdb_id: None,
                overview: None,
                tagline: None,
                runtime: None,
                genres: Vec::new(),
                director: None,
                cast_names: Vec::new(),
                production_countries: Vec::new(),
                spoken_languages: Vec::new(),
                tmdb_rating: None,
                tmdb_vote_count: None,
                poster_url: None,
                backdrop_url: None,
                collection_id: None,
                collection_name: None,
                collection_poster_url: None,
                release_date: None,
                certification: None,
                monitored: true,
                tmdb_status: None,
                wanted: false,
                ignored: None,
                download_status: None,
                has_file: false,
                media_file_id: None,
                quality_profile_id: None,
            },
        )
        .await?)
    }

    async fn artist(&self, name: &str) -> anyhow::Result<Artist> {
        Ok(Artist::insert(
            self.database.pool(),
            CreateArtistInput {
                library_id: self.library.id.clone(),
                user_id: self.library.user_id.clone(),
                name: name.to_string(),
                sort_name: None,
                musicbrainz_id: None,
                bio: None,
                disambiguation: None,
                image_url: None,
                album_count: None,
                track_count: None,
                total_duration_secs: None,
            },
        )
        .await?)
    }

    async fn album(&self, artist: &Artist, name: &str, year: i32) -> anyhow::Result<Album> {
        Ok(Album::insert(
            self.database.pool(),
            CreateAlbumInput {
                artist_id: artist.id.clone(),
                library_id: self.library.id.clone(),
                user_id: self.library.user_id.clone(),
                name: name.to_string(),
                sort_name: None,
                year: Some(year),
                musicbrainz_id: None,
                album_type: None,
                genres: Vec::new(),
                label: None,
                country: None,
                release_date: None,
                cover_url: None,
                track_count: None,
                disc_count: None,
                total_duration_secs: None,
                auto_download: false,
                auto_download_mode: AutoDownloadMode::None,
                quality_profile_id: None,
                has_files: false,
                size_bytes: None,
                path: None,
            },
        )
        .await?)
    }

    /// A track whose own artist credit is whatever the source said, which has no say in where
    /// its file goes.
    async fn track(
        &self,
        album: &Album,
        number: i32,
        title: &str,
        credited_as: Option<&str>,
    ) -> anyhow::Result<Track> {
        Ok(Track::insert(
            self.database.pool(),
            CreateTrackInput {
                album_id: album.id.clone(),
                library_id: self.library.id.clone(),
                title: title.to_string(),
                track_number: number,
                disc_number: Some(1),
                musicbrainz_id: None,
                isrc: None,
                duration_secs: None,
                explicit: false,
                artist_name: credited_as.map(str::to_string),
                artist_id: None,
                wanted: false,
                ignored: None,
                media_file_id: None,
            },
        )
        .await?)
    }

    /// A file already known to the library and unchanged since the last scan: its size and
    /// modification time are what the scan will see, so the scan skips it.
    async fn known_file(
        &self,
        path: &Path,
        movie_id: Option<&str>,
        track_id: Option<&str>,
    ) -> anyhow::Result<MediaFile> {
        let metadata = std::fs::metadata(path)?;
        let modified_at =
            DateTime::<Utc>::from(metadata.modified()?).to_rfc3339_opts(SecondsFormat::Nanos, true);
        Ok(MediaFile::insert(
            self.database.pool(),
            CreateMediaFileInput {
                library_id: Some(self.library.id.clone()),
                episode_id: None,
                movie_id: movie_id.map(str::to_string),
                track_id: track_id.map(str::to_string),
                chapter_id: None,
                path: path.to_string_lossy().to_string(),
                relative_path: path
                    .strip_prefix(&self.root)
                    .ok()
                    .map(|relative| relative.to_string_lossy().to_string()),
                original_name: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_string),
                size: i64::try_from(metadata.len())?,
                container: None,
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                resolution: None,
                is_hdr: false,
                hdr_type: None,
                closed_captions: None,
                audio_channels: None,
                metadata: None,
                content_type: None,
                added_at: "2026-11-01T00:00:00Z".to_string(),
                analyzed_at: Some("2026-11-01T00:00:00Z".to_string()),
                file_modified_at: Some(modified_at),
                match_type: Some("auto".to_string()),
                matched_by_user_id: None,
                match_confirmed_at: None,
                quality_status: Some("optimal".to_string()),
            },
        )
        .await?)
    }

    async fn stop(self) -> anyhow::Result<()> {
        self.services.stop_all().await?;
        Ok(())
    }
}

#[test]
fn a_scan_organizes_an_unchanged_matched_movie_that_is_loose_in_the_library_root()
-> anyhow::Result<()> {
    on_big_stack("organize-loose-movie", loose_movie_case)
}

async fn loose_movie_case() -> anyhow::Result<()> {
    let h = Harness::new("movies", MOVIE_PATTERN, true).await?;
    let patriot = h.movie("Patriot Games", 1992).await?;
    let clear = h.movie("Clear and Present Danger", 1994).await?;
    let loose = h.write("Patriot Games (1992).mp4", b"patriot games");
    let foldered = h.write(
        "Clear and Present Danger (1994)/Clear and Present Danger (1994).mp4",
        b"clear and present danger",
    );
    let patriot_file = h.known_file(&loose, Some(&patriot.id), None).await?;
    h.known_file(&foldered, Some(&clear.id), None).await?;

    let run = h.scan().await?;

    let moved = h.root.join("Patriot Games (1992)/Patriot Games (1992).mp4");
    assert!(moved.exists(), "the loose file is in its own folder");
    assert!(!loose.exists());
    assert!(foldered.exists(), "a file already in place stays");
    let row = MediaFile::get(h.database.pool().pool(), &patriot_file.id)
        .await?
        .context("media file vanished")?;
    assert_eq!(Path::new(&row.path), moved);
    assert_eq!(
        row.relative_path.as_deref(),
        Some("Patriot Games (1992)/Patriot Games (1992).mp4")
    );
    assert_eq!(row.movie_id.as_deref(), Some(patriot.id.as_str()));
    assert_eq!(h.media_files().await?.len(), 2, "nothing new was recorded");
    // Both files were skipped as unchanged: nothing was matched again.
    assert_eq!(run.matched_count, 2);
    assert_eq!(run.discovered_count, 2);

    // A later scan has nothing left to do and leaves it where it is.
    h.scan().await?;
    assert!(moved.exists());
    assert_eq!(
        h.media_file_at(&moved).await?.id,
        patriot_file.id,
        "still the same record"
    );
    h.stop().await
}

#[test]
fn a_scan_with_organize_off_leaves_an_unchanged_loose_movie_alone() -> anyhow::Result<()> {
    on_big_stack("organize-off-loose-movie", organize_off_case)
}

async fn organize_off_case() -> anyhow::Result<()> {
    let h = Harness::new("movies", MOVIE_PATTERN, false).await?;
    let patriot = h.movie("Patriot Games", 1992).await?;
    let loose = h.write("Patriot Games (1992).mp4", b"patriot games");
    h.known_file(&loose, Some(&patriot.id), None).await?;
    h.write("Stale/Tree/.keep-me", b"x");

    h.scan().await?;

    assert!(
        loose.exists(),
        "without auto-organize files are never moved"
    );
    assert!(h.root.join("Stale/Tree/.keep-me").exists());
    h.stop().await
}

#[test]
fn a_scan_gathers_one_artist_into_one_folder_and_removes_what_it_empties() -> anyhow::Result<()> {
    on_big_stack("organize-one-artist", one_artist_case)
}

async fn one_artist_case() -> anyhow::Result<()> {
    let h = Harness::new("music", MUSIC_PATTERN, true).await?;
    // One Artist record, as the live library has, under the curly spelling a source credited.
    let artist = h.artist("Guns N\u{2019} Roses").await?;
    let appetite = h.album(&artist, "Appetite for Destruction", 1987).await?;
    let illusion = h.album(&artist, "Use Your Illusion II", 1991).await?;
    // Tracks credited differently, or to nobody: the per-track credit decides nothing.
    let jungle = h.track(&appetite, 1, "Welcome to the Jungle", None).await?;
    let easy = h
        .track(&appetite, 2, "It's So Easy", Some("Guns N\u{2019} Roses"))
        .await?;
    let nightrain = h
        .track(&appetite, 3, "Nightrain", Some("Guns N' Roses"))
        .await?;
    let civil = h.track(&illusion, 1, "Civil War", None).await?;

    let unknown_1 = h.write(
        "Unknown Artist/Appetite for Destruction (1987)/01 - Welcome to the Jungle.mp3",
        b"jungle",
    );
    let curly = h.write(
        "Guns N\u{2019} Roses/Appetite for Destruction (1987)/02 - It's So Easy.mp3",
        b"easy",
    );
    let right = h.write(
        "Guns N' Roses/Appetite for Destruction (1987)/03 - Nightrain.mp3",
        b"nightrain",
    );
    let unknown_2 = h.write(
        "Unknown Artist/Use Your Illusion II (1991)/01 - Civil War.mp3",
        b"civil war",
    );
    for (path, track) in [
        (&unknown_1, &jungle),
        (&curly, &easy),
        (&right, &nightrain),
        (&unknown_2, &civil),
    ] {
        h.known_file(path, None, Some(&track.id)).await?;
    }
    // Folders left over from an earlier layout, a folder in use, and the trash.
    std::fs::create_dir_all(h.root.join("Old Artist/Old Album/Disc 1"))?;
    h.write("Cover Art/.hidden", b"x");
    h.write(
        &format!("{LIBRARY_TRASH_DIR}/0000-Duplicate.mp3"),
        b"trashed",
    );
    std::fs::create_dir_all(h.root.join(LIBRARY_TRASH_DIR).join("empty"))?;

    h.scan().await?;

    let folder = h.root.join("Guns N' Roses");
    let expected = [
        (
            &jungle,
            folder.join("Appetite for Destruction (1987)/01 - Welcome to the Jungle.mp3"),
        ),
        (
            &easy,
            folder.join("Appetite for Destruction (1987)/02 - It's So Easy.mp3"),
        ),
        (
            &nightrain,
            folder.join("Appetite for Destruction (1987)/03 - Nightrain.mp3"),
        ),
        (
            &civil,
            folder.join("Use Your Illusion II (1991)/01 - Civil War.mp3"),
        ),
    ];
    for (track, path) in &expected {
        assert!(
            path.exists(),
            "{} is in the one artist folder",
            path.display()
        );
        let file = h.media_file_at(path).await?;
        assert_eq!(file.track_id.as_deref(), Some(track.id.as_str()));
    }
    assert!(
        !h.root.join("Unknown Artist").exists(),
        "no placeholder folder"
    );
    assert!(
        !h.root.join("Guns N\u{2019} Roses").exists(),
        "no second spelling of the artist"
    );
    let artist_folders: Vec<String> = std::fs::read_dir(&h.root)?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name.starts_with("Guns"))
        .collect();
    assert_eq!(artist_folders, vec!["Guns N' Roses".to_string()]);
    assert_eq!(h.media_files().await?.len(), 4, "no record was duplicated");

    // Empty trees go, but not a folder with a file, and not the trash.
    assert!(!h.root.join("Old Artist").exists());
    assert!(h.root.join("Cover Art/.hidden").exists());
    assert!(
        h.root
            .join(LIBRARY_TRASH_DIR)
            .join("0000-Duplicate.mp3")
            .exists()
    );
    assert!(h.root.join(LIBRARY_TRASH_DIR).join("empty").exists());
    assert!(h.root.exists());
    // The trash is not part of the library: nothing in it was recorded.
    assert!(
        h.media_files()
            .await?
            .iter()
            .all(|file| !file.path.contains(LIBRARY_TRASH_DIR))
    );
    h.stop().await
}

#[test]
fn organizing_never_overwrites_a_file_already_at_the_target() -> anyhow::Result<()> {
    on_big_stack("organize-no-clobber", no_clobber_case)
}

async fn no_clobber_case() -> anyhow::Result<()> {
    let h = Harness::new("music", MUSIC_PATTERN, true).await?;
    let artist = h.artist("Guns N' Roses").await?;
    let album = h.album(&artist, "Appetite for Destruction", 1987).await?;
    let track = h.track(&album, 1, "Welcome to the Jungle", None).await?;
    let stray = h.write(
        "Unknown Artist/Appetite for Destruction (1987)/01 - Welcome to the Jungle.mp3",
        b"the stray copy",
    );
    let occupant = h.write(
        "Guns N' Roses/Appetite for Destruction (1987)/01 - Welcome to the Jungle.mp3",
        b"a different file already here",
    );
    h.known_file(&stray, None, Some(&track.id)).await?;
    // The occupant belongs to a track nothing can plan a path for, so it stays where it is and
    // nothing frees the stray's target.
    let unnamed = h.artist("Unknown Artist").await?;
    let other_album = h.album(&unnamed, "Mystery Tape", 1999).await?;
    let other_track = h.track(&other_album, 1, "Side A", None).await?;
    h.known_file(&occupant, None, Some(&other_track.id)).await?;

    h.scan().await?;

    assert_eq!(std::fs::read(&occupant)?, b"a different file already here");
    assert_eq!(
        std::fs::read(&stray)?,
        b"the stray copy",
        "the stray stays put"
    );
    assert!(
        h.root.join("Unknown Artist").exists(),
        "a folder that still has a file stays"
    );
    h.stop().await
}

#[test]
fn a_track_whose_artist_is_unknown_is_left_where_it_is() -> anyhow::Result<()> {
    on_big_stack("organize-unknown-artist", unknown_artist_case)
}

async fn unknown_artist_case() -> anyhow::Result<()> {
    let h = Harness::new("music", MUSIC_PATTERN, true).await?;
    let artist = h.artist("Unknown Artist").await?;
    let album = h.album(&artist, "Mystery Tape", 1999).await?;
    let track = h.track(&album, 1, "Side A", Some("Somebody")).await?;
    let file = h.write("Mystery Tape (1999)/01 - Side A.mp3", b"side a");
    h.known_file(&file, None, Some(&track.id)).await?;

    h.scan().await?;

    assert!(file.exists(), "nothing is filed under a placeholder artist");
    assert!(!h.root.join("Unknown Artist").exists());
    h.stop().await
}

#[test]
fn organizing_one_file_removes_the_folders_it_empties() -> anyhow::Result<()> {
    on_big_stack("organize-prunes", prune_after_organize_case)
}

async fn prune_after_organize_case() -> anyhow::Result<()> {
    let h = Harness::new("movies", MOVIE_PATTERN, true).await?;
    let patriot = h.movie("Patriot Games", 1992).await?;
    let clear = h.movie("Clear and Present Danger", 1994).await?;
    let alone = h.write(
        "Downloads/Tom Clancy/Patriot/Patriot.Games.1992.mp4",
        b"one",
    );
    let shared = h.write("Pile/Nested/Clear.and.Present.Danger.1994.mp4", b"two");
    h.write("Pile/readme.txt", b"stays");
    let alone_file = h.known_file(&alone, Some(&patriot.id), None).await?;
    let shared_file = h.known_file(&shared, Some(&clear.id), None).await?;

    let result = h.scanner.organize_media_file(&alone_file.id).await?;
    assert!(result.success, "{:?}", result.reason);
    assert!(
        h.root
            .join("Patriot Games (1992)/Patriot Games (1992).mp4")
            .exists()
    );
    assert!(
        !h.root.join("Downloads").exists(),
        "the whole emptied tree is gone"
    );

    let result = h.scanner.organize_media_file(&shared_file.id).await?;
    assert!(result.success, "{:?}", result.reason);
    assert!(!h.root.join("Pile/Nested").exists());
    assert!(
        h.root.join("Pile/readme.txt").exists(),
        "a folder with a file in it stays"
    );
    assert!(h.root.exists());
    h.stop().await
}
