//! Isolated transaction tests for canonical local-file summaries and lifecycle cleanup.
//! No servers, migrations outside disposable schema bootstrap, or application SQL.
use super::*;
use graphql_orm::DbPool;
use graphql_orm::graphql::errors::{OrmErrorCode, OrmPublicError};
use graphql_orm::graphql::subscriptions::SubscriptionEvent;

async fn database(hooks: bool) -> Database {
    let mut db = Database::new(DbPool::connect("sqlite::memory:").await.unwrap());
    bootstrap_schema(&db).await.unwrap();
    if hooks {
        DatabaseService::new(db).pool().clone()
    } else {
        // Keep declared policies while omitting lifecycle hooks for legacy fixtures.
        db.set_entity_policy(crate::services::graphql::policy::AppEntityPolicy);
        db.set_field_policy(crate::services::graphql::policy::AppFieldPolicy);
        db.set_row_policy(crate::services::graphql::row_policy::OwnershipRowPolicy);
        db
    }
}

async fn user(db: &Database) -> User {
    User::insert(
        db,
        CreateUserInput {
            username: uuid::Uuid::new_v4().to_string(),
            email: None,
            password_hash: "fixture password_hash".to_string(),
            role: "member".to_string(),
            display_name: None,
            avatar_url: None,
            is_active: true,
            last_login_at: None,
        },
    )
    .await
    .unwrap()
}

async fn library(db: &Database, user: &User) -> Library {
    Library::insert(
        db,
        CreateLibraryInput {
            user_id: user.id.clone(),
            name: "fixture library".to_string(),
            path: format!("/fixture/{}", uuid::Uuid::new_v4()),
            library_type: "music".to_string(),
            icon: None,
            color: None,
            auto_scan: false,
            auto_organize: false,
            naming_pattern: "fixture naming_pattern".to_string(),
            scan_interval_minutes: 0,
            watch_for_changes: false,
            scanning: false,
            last_scanned_at: None,
            quality_profile_id: None,
        },
    )
    .await
    .unwrap()
}

async fn artist(db: &Database, library: &Library) -> Artist {
    Artist::insert(
        db,
        CreateArtistInput {
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            name: "fixture artist".to_string(),
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
    .await
    .unwrap()
}

async fn album(db: &Database, library: &Library, artist: &Artist) -> Album {
    Album::insert(
        db,
        CreateAlbumInput {
            artist_id: artist.id.clone(),
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            name: "fixture album".to_string(),
            sort_name: None,
            year: None,
            musicbrainz_id: None,
            album_type: None,
            genres: Vec::new(),
            label: None,
            country: None,
            release_date: None,
            cover_url: None,
            track_count: Some(99),
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
    .await
    .unwrap()
}

async fn audiobook(db: &Database, library: &Library) -> Audiobook {
    Audiobook::insert(
        db,
        CreateAudiobookInput {
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            title: "fixture audiobook".to_string(),
            sort_title: None,
            author_name: None,
            narrator_name: None,
            narrators: Vec::new(),
            description: None,
            publisher: None,
            published_date: None,
            language: None,
            isbn: None,
            asin: None,
            audible_id: None,
            goodreads_id: None,
            total_duration_secs: None,
            chapter_count: Some(99),
            cover_url: None,
            auto_download: false,
            auto_download_mode: AutoDownloadMode::None,
            quality_profile_id: None,
            has_files: false,
            size_bytes: None,
            path: None,
        },
    )
    .await
    .unwrap()
}

async fn track(db: &Database, album: &Album, file_id: Option<&str>) -> Track {
    Track::insert(
        db,
        CreateTrackInput {
            album_id: album.id.clone(),
            library_id: album.library_id.clone(),
            title: "fixture track".to_string(),
            track_number: 0,
            disc_number: None,
            musicbrainz_id: None,
            isrc: None,
            duration_secs: None,
            explicit: false,
            artist_name: None,
            artist_id: None,
            wanted: file_id.is_none(),
            ignored: None,
            media_file_id: file_id.map(str::to_string),
        },
    )
    .await
    .unwrap()
}

async fn chapter(db: &Database, book: &Audiobook, file_id: Option<&str>) -> Chapter {
    Chapter::insert(
        db,
        CreateChapterInput {
            audiobook_id: book.id.clone(),
            chapter_number: 0,
            title: None,
            start_time_secs: 0.0,
            end_time_secs: None,
            duration_secs: None,
            wanted: file_id.is_none(),
            ignored: None,
            media_file_id: file_id.map(str::to_string),
        },
    )
    .await
    .unwrap()
}

async fn show(db: &Database, library: &Library) -> Show {
    Show::insert(
        db,
        CreateShowInput {
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            name: "fixture show".to_string(),
            sort_name: None,
            year: None,
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            imdb_id: None,
            overview: None,
            network: None,
            runtime: None,
            genres: Vec::new(),
            poster_url: None,
            backdrop_url: None,
            content_rating: None,
            auto_download: false,
            auto_download_mode: AutoDownloadMode::None,
            quality_profile_id: None,
            path: None,
        },
    )
    .await
    .unwrap()
}

async fn episode(db: &Database, show: &Show, file_id: Option<&str>) -> Episode {
    Episode::insert(
        db,
        CreateEpisodeInput {
            show_id: show.id.clone(),
            season: 0,
            episode: 0,
            absolute_number: None,
            title: None,
            overview: None,
            air_date: None,
            air_stamp: None,
            runtime: None,
            still_url: None,
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            wanted: file_id.is_none(),
            ignored: None,
            media_file_id: file_id.map(str::to_string),
        },
    )
    .await
    .unwrap()
}

async fn movie(db: &Database, library: &Library) -> Movie {
    Movie::insert(
        db,
        CreateMovieInput {
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            title: "fixture movie".to_string(),
            sort_title: None,
            original_title: None,
            year: None,
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
            monitored: false,
            tmdb_status: None,
            wanted: true,
            ignored: None,
            download_status: None,
            has_file: false,
            media_file_id: None,
            quality_profile_id: None,
        },
    )
    .await
    .unwrap()
}

async fn media_file(
    db: &Database,
    library: &Library,
    size: i64,
    track_id: Option<&str>,
    chapter_id: Option<&str>,
) -> MediaFile {
    MediaFile::insert(
        db,
        CreateMediaFileInput {
            library_id: Some(library.id.clone()),
            episode_id: None,
            movie_id: None,
            track_id: track_id.map(str::to_string),
            chapter_id: chapter_id.map(str::to_string),
            path: format!("/fixture/{}.bin", uuid::Uuid::new_v4()),
            relative_path: None,
            original_name: None,
            size,
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
            audio_channels: None,
            metadata: None,
            content_type: None,
            added_at: AppMutationHook::now_rfc3339(),
            analyzed_at: None,
            file_modified_at: None,
            match_type: None,
            matched_by_user_id: None,
            match_confirmed_at: None,
            quality_status: None,
        },
    )
    .await
    .unwrap()
}

async fn person(db: &Database) -> Person {
    Person::insert(
        db,
        CreatePersonInput {
            tmdb_person_id: 0,
            name: "fixture person".to_string(),
            profile_url: None,
        },
    )
    .await
    .unwrap()
}

async fn movie_cast_credit(db: &Database, movie: &Movie, person: &Person) -> MovieCastCredit {
    MovieCastCredit::insert(
        db,
        CreateMovieCastCreditInput {
            movie_id: movie.id.clone(),
            person_id: person.id.clone(),
            character_name: None,
            cast_order: None,
        },
    )
    .await
    .unwrap()
}

async fn library_scan_run(db: &Database, library: &Library) -> LibraryScanRun {
    LibraryScanRun::insert(
        db,
        CreateLibraryScanRunInput {
            user_id: library.user_id.clone(),
            library_id: library.id.clone(),
            status: "completed".to_string(),
            current_stage: "fixture current_stage".to_string(),
            started_at: None,
            finished_at: None,
            discovered_count: 0,
            existing_count: 0,
            matched_count: 0,
            unmatched_count: 0,
            provider_blocked_count: 0,
            analysis_queued_count: 0,
            analysis_succeeded_count: 0,
            analysis_failed_count: 0,
            organization_succeeded_count: 0,
            organization_failed_count: 0,
            missing_count: 0,
            reconciled_count: 0,
            summary: None,
            error_code: None,
        },
    )
    .await
    .unwrap()
}

async fn library_scan_issue(
    db: &Database,
    library: &Library,
    run: &LibraryScanRun,
    file: &MediaFile,
) -> LibraryScanIssue {
    LibraryScanIssue::insert(
        db,
        CreateLibraryScanIssueInput {
            user_id: library.user_id.clone(),
            scan_run_id: run.id.clone(),
            library_id: library.id.clone(),
            media_file_id: Some(file.id.clone()),
            stage: "fixture stage".to_string(),
            issue_code: "fixture issue_code".to_string(),
            severity: "fixture severity".to_string(),
            message: "historical issue".to_string(),
            remediation: None,
            details_json: None,
            occurrence_count: 0,
            read_at: None,
            resolved_at: None,
            resolution: None,
        },
    )
    .await
    .unwrap()
}

async fn video_stream(db: &Database, file: &MediaFile) -> VideoStream {
    VideoStream::insert(
        db,
        CreateVideoStreamInput {
            media_file_id: file.id.clone(),
            stream_index: 0,
            codec: "fixture codec".to_string(),
            codec_long_name: None,
            width: 0,
            height: 0,
            aspect_ratio: None,
            frame_rate: None,
            avg_frame_rate: None,
            bitrate: None,
            pixel_format: None,
            color_space: None,
            color_transfer: None,
            color_primaries: None,
            hdr_type: None,
            bit_depth: None,
            language: None,
            title: None,
            is_default: false,
            metadata: None,
        },
    )
    .await
    .unwrap()
}

async fn audio_stream(db: &Database, file: &MediaFile) -> AudioStream {
    AudioStream::insert(
        db,
        CreateAudioStreamInput {
            media_file_id: file.id.clone(),
            stream_index: 0,
            codec: "fixture codec".to_string(),
            codec_long_name: None,
            channels: 0,
            channel_layout: None,
            sample_rate: None,
            bitrate: None,
            bit_depth: None,
            language: None,
            title: None,
            is_default: false,
            is_commentary: false,
            metadata: None,
        },
    )
    .await
    .unwrap()
}

async fn subtitle(db: &Database, file: &MediaFile) -> Subtitle {
    Subtitle::insert(
        db,
        CreateSubtitleInput {
            media_file_id: file.id.clone(),
            source_type: "fixture source_type".to_string(),
            stream_index: None,
            file_path: None,
            codec: None,
            codec_long_name: None,
            language: None,
            title: None,
            is_default: false,
            is_forced: false,
            is_hearing_impaired: false,
            opensubtitles_id: None,
            downloaded_at: None,
            metadata: None,
        },
    )
    .await
    .unwrap()
}

async fn media_chapter(db: &Database, file: &MediaFile) -> MediaChapter {
    MediaChapter::insert(
        db,
        CreateMediaChapterInput {
            media_file_id: file.id.clone(),
            chapter_index: 0,
            start_secs: 0.0,
            end_secs: 0.0,
            title: None,
        },
    )
    .await
    .unwrap()
}

async fn music_fixture(db: &Database) -> (User, Library, Artist, Album) {
    let owner = user(db).await;
    let lib = library(db, &owner).await;
    let performer = artist(db, &lib).await;
    let record = album(db, &lib, &performer).await;
    (owner, lib, performer, record)
}

async fn assert_album(db: &Database, id: &str, has_files: bool, size: i64) {
    let parent = Album::get(db.pool(), &id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (parent.has_files, parent.size_bytes),
        (has_files, Some(size))
    );
    assert_eq!(
        parent.track_count,
        Some(99),
        "provider totals must survive local-file repair"
    );
    assert_eq!(parent.name, "fixture album");
}

async fn assert_book(db: &Database, id: &str, has_files: bool, size: i64) {
    let parent = Audiobook::get(db.pool(), &id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (parent.has_files, parent.size_bytes),
        (has_files, Some(size))
    );
    assert_eq!(parent.chapter_count, Some(99));
    assert_eq!(parent.title, "fixture audiobook");
}

#[tokio::test]
async fn album_first_link_resize_rematch_reparent_and_last_unlink_are_persisted() {
    let db = database(true).await;
    let (_, lib, performer, first) = music_fixture(&db).await;
    let second = album(&db, &lib, &performer).await;
    let song = track(&db, &first, None).await;
    assert_album(&db, &first.id, false, 0).await;
    // The file-side Created hook must establish the inverse and the first availability.
    let file = media_file(&db, &lib, 120, Some(&song.id), None).await;
    let linked = Track::get(db.pool(), &song.id).await.unwrap().unwrap();
    assert_eq!(linked.media_file_id.as_deref(), Some(file.id.as_str()));
    assert!(!linked.wanted);
    assert_album(&db, &first.id, true, 120).await;
    MediaFile::update_by_id(
        &db,
        &file.id,
        UpdateMediaFileInput {
            size: Some(180),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_album(&db, &first.id, true, 180).await;

    let mut changes = db
        .ensure_event_sender::<SubscriptionEvent<AlbumChangedEvent>>()
        .subscribe();
    Track::update_by_id(
        &db,
        &song.id,
        UpdateTrackInput {
            album_id: Some(second.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_album(&db, &first.id, false, 0).await;
    assert_album(&db, &second.id, true, 180).await;
    let mut direct = BTreeSet::new();
    while let Ok(change) = changes.try_recv() {
        if change.event.source_entity.is_none() {
            direct.insert(change.event.id.clone());
        }
    }
    assert!(
        direct.contains(&first.id),
        "old parent needs its own provenance after reparenting"
    );
    assert!(direct.contains(&second.id));

    let other = track(&db, &first, None).await;
    MediaFile::update_by_id(
        &db,
        &file.id,
        UpdateMediaFileInput {
            track_id: Some(Some(other.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        Track::get(db.pool(), &song.id)
            .await
            .unwrap()
            .unwrap()
            .media_file_id,
        None
    );
    assert_eq!(
        MediaFile::get(db.pool(), &file.id)
            .await
            .unwrap()
            .unwrap()
            .track_id,
        Some(other.id.clone())
    );
    assert_album(&db, &first.id, true, 180).await;
    assert_album(&db, &second.id, false, 0).await;
    MediaFile::update_by_id(
        &db,
        &file.id,
        UpdateMediaFileInput {
            track_id: Some(None),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_album(&db, &first.id, false, 0).await;
    assert!(
        Track::get(db.pool(), &other.id)
            .await
            .unwrap()
            .unwrap()
            .wanted
    );
}

#[tokio::test]
async fn chapter_creation_reparent_delete_and_media_file_deletion_reconcile_books() {
    let db = database(true).await;
    let (_, lib, _, _) = music_fixture(&db).await;
    let first = audiobook(&db, &lib).await;
    let second = audiobook(&db, &lib).await;
    let file = media_file(&db, &lib, 300, None, None).await;
    let part = chapter(&db, &first, Some(&file.id)).await;
    assert_book(&db, &first.id, true, 300).await;
    assert_eq!(
        MediaFile::get(db.pool(), &file.id)
            .await
            .unwrap()
            .unwrap()
            .chapter_id,
        Some(part.id.clone())
    );
    Chapter::update_by_id(
        &db,
        &part.id,
        UpdateChapterInput {
            audiobook_id: Some(second.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_book(&db, &first.id, false, 0).await;
    assert_book(&db, &second.id, true, 300).await;
    MediaFile::delete_by_id(&db, &file.id).await.unwrap();
    assert_book(&db, &second.id, false, 0).await;
    let detached = Chapter::get(db.pool(), &part.id).await.unwrap().unwrap();
    assert!(detached.wanted);
    assert_eq!(detached.media_file_id, None);
    Chapter::delete_by_id(&db, &part.id).await.unwrap();
    assert_book(&db, &second.id, false, 0).await;
}

#[tokio::test]
async fn direct_and_bulk_parent_deletes_run_child_hooks_and_preserve_unlinked_files() {
    let db = database(true).await;
    let (_, lib, performer, record) = music_fixture(&db).await;
    let song_file = media_file(&db, &lib, 100, None, None).await;
    let song = track(&db, &record, Some(&song_file.id)).await;
    let series = show(&db, &lib).await;
    let episode_file = media_file(&db, &lib, 200, None, None).await;
    let ep = episode(&db, &series, Some(&episode_file.id)).await;
    let book = audiobook(&db, &lib).await;
    let chapter_file = media_file(&db, &lib, 300, None, None).await;
    let part = chapter(&db, &book, Some(&chapter_file.id)).await;
    let film = movie(&db, &lib).await;
    let cast = person(&db).await;
    let credit = movie_cast_credit(&db, &film, &cast).await;
    let movie_file = media_file(&db, &lib, 400, None, None).await;
    Movie::update_by_id(
        &db,
        &film.id,
        UpdateMovieInput {
            media_file_id: Some(Some(movie_file.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mut episode_events = db
        .ensure_event_sender::<SubscriptionEvent<EpisodeChangedEvent>>()
        .subscribe();
    Show::delete_by_id(&db, &series.id).await.unwrap();
    assert!(Episode::get(db.pool(), &ep.id).await.unwrap().is_none());
    assert_eq!(
        MediaFile::get(db.pool(), &episode_file.id)
            .await
            .unwrap()
            .unwrap()
            .episode_id,
        None
    );
    assert!(
        std::iter::from_fn(|| episode_events.try_recv().ok())
            .any(|e| e.event.action == ChangeAction::Deleted && e.event.id == ep.id)
    );
    // Bulk roots use the same explicit cleanup; the surviving Artist is not removed.
    Album::delete_where(
        &db,
        AlbumWhereInput {
            id: Some(AppMutationHook::string_eq(&record.id)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(Track::get(db.pool(), &song.id).await.unwrap().is_none());
    assert!(
        Artist::get(db.pool(), &performer.id)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        MediaFile::get(db.pool(), &song_file.id)
            .await
            .unwrap()
            .unwrap()
            .track_id,
        None
    );
    Audiobook::delete_by_id(&db, &book.id).await.unwrap();
    assert!(Chapter::get(db.pool(), &part.id).await.unwrap().is_none());
    assert_eq!(
        MediaFile::get(db.pool(), &chapter_file.id)
            .await
            .unwrap()
            .unwrap()
            .chapter_id,
        None
    );
    Movie::delete_by_id(&db, &film.id).await.unwrap();
    assert!(
        MovieCastCredit::get(db.pool(), &credit.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(Person::get(db.pool(), &cast.id).await.unwrap().is_some());
    assert_eq!(
        MediaFile::get(db.pool(), &movie_file.id)
            .await
            .unwrap()
            .unwrap()
            .movie_id,
        None
    );
}

#[tokio::test]
async fn summary_links_and_generated_events_rollback_together() {
    let db = database(true).await;
    let (_, lib, _, record) = music_fixture(&db).await;
    let song = track(&db, &record, None).await;
    let file = media_file(&db, &lib, 55, None, None).await;
    let events = Arc::new(std::sync::Mutex::new(
        db.ensure_event_sender::<SubscriptionEvent<AlbumChangedEvent>>()
            .subscribe(),
    ));
    let observed = events.clone();
    let parent_id = record.id.clone();
    let song_id = song.id.clone();
    let file_id = file.id.clone();
    let failed = db
        .transaction(TransactionMode::StateMachine, move |tx| {
            Box::pin(async move {
                tx.update_by_id::<Track>(
                    &song_id,
                    UpdateTrackInput {
                        media_file_id: Some(Some(file_id)),
                        ..Default::default()
                    },
                )
                .await?;
                let parent = tx.find_by_id::<Album>(&parent_id).await?.unwrap();
                assert_eq!((parent.has_files, parent.size_bytes), (true, Some(55)));
                assert!(
                    observed.lock().unwrap().try_recv().is_err(),
                    "events must wait for commit"
                );
                Err::<(), _>(OrmPublicError::new(OrmErrorCode::Conflict))
            })
        })
        .await;
    assert!(failed.is_err());
    assert_album(&db, &record.id, false, 0).await;
    assert_eq!(
        Track::get(db.pool(), &song.id)
            .await
            .unwrap()
            .unwrap()
            .media_file_id,
        None
    );
    assert_eq!(
        MediaFile::get(db.pool(), &file.id)
            .await
            .unwrap()
            .unwrap()
            .track_id,
        None
    );
    assert!(events.lock().unwrap().try_recv().is_err());
}

#[tokio::test]
async fn backfill_is_explicit_dry_run_safe_idempotent_and_deduplicates_shared_files() {
    let raw = database(false).await;
    let (_, lib, _, record) = music_fixture(&raw).await;
    let song_file = media_file(&raw, &lib, 100, None, None).await;
    track(&raw, &record, Some(&song_file.id)).await;
    let book = audiobook(&raw, &lib).await;
    let shared = media_file(&raw, &lib, 200, None, None).await;
    chapter(&raw, &book, Some(&shared.id)).await;
    chapter(&raw, &book, Some(&shared.id)).await;
    let db = DatabaseService::new(raw).pool().clone();
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<AlbumChangedEvent>>()
        .subscribe();
    let preview = reconcile_stored_media_summaries(&db, true).await.unwrap();
    assert_eq!(
        (
            preview.albums_examined,
            preview.albums_changed,
            preview.audiobooks_examined,
            preview.audiobooks_changed
        ),
        (1, 1, 1, 1)
    );
    assert!(preview.dry_run);
    assert!(
        !Album::get(db.pool(), &record.id)
            .await
            .unwrap()
            .unwrap()
            .has_files
    );
    assert!(events.try_recv().is_err());
    let applied = reconcile_stored_media_summaries(&db, false).await.unwrap();
    assert_eq!((applied.albums_changed, applied.audiobooks_changed), (1, 1));
    assert_album(&db, &record.id, true, 100).await;
    assert_book(&db, &book.id, true, 200).await;
    assert!(events.try_recv().is_ok());
    let again = reconcile_stored_media_summaries(&db, false).await.unwrap();
    assert_eq!((again.albums_changed, again.audiobooks_changed), (0, 0));
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn media_file_cleanup_retains_detached_scan_history_and_library_delete_removes_it() {
    let db = database(true).await;
    let (_, lib, _, _) = music_fixture(&db).await;
    let file = media_file(&db, &lib, 42, None, None).await;
    let run = library_scan_run(&db, &lib).await;
    let issue = library_scan_issue(&db, &lib, &run, &file).await;
    let video = video_stream(&db, &file).await;
    let audio = audio_stream(&db, &file).await;
    let subtitles = subtitle(&db, &file).await;
    let segments = media_chapter(&db, &file).await;
    MediaFile::delete_by_id(&db, &file.id).await.unwrap();
    assert!(
        VideoStream::get(db.pool(), &video.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        AudioStream::get(db.pool(), &audio.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        Subtitle::get(db.pool(), &subtitles.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        MediaChapter::get(db.pool(), &segments.id)
            .await
            .unwrap()
            .is_none()
    );
    let retained = LibraryScanIssue::get(db.pool(), &issue.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retained.media_file_id, None);
    assert_eq!(retained.message, "historical issue");
    assert!(
        LibraryScanRun::get(db.pool(), &run.id)
            .await
            .unwrap()
            .is_some()
    );
    Library::delete_by_id(&db, &lib.id).await.unwrap();
    assert!(
        LibraryScanRun::get(db.pool(), &run.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        LibraryScanIssue::get(db.pool(), &issue.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[derive(Default)]
struct SummaryQuery;
#[async_graphql::Object]
impl SummaryQuery {
    async fn ready(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn deleted_track_delivers_direct_surviving_album_payload_only_to_its_owner() {
    use futures::{FutureExt, StreamExt};
    let db = database(true).await;
    let (owner, lib, _, record) = music_fixture(&db).await;
    let outsider = user(&db).await;
    let song = track(&db, &record, None).await;
    // The summary remains false/zero: deletion still needs a directly authorized parent event.
    let schema = async_graphql::Schema::build(
        SummaryQuery,
        async_graphql::EmptyMutation,
        AlbumSubscriptions,
    )
    .data(db.clone())
    .finish();
    let request = |user: &User| {
        async_graphql::Request::new("subscription { albumChanged { action sourceEntity album { id name hasFiles sizeBytes } } }")
        .data(user.id.clone()).data(AuthUser { user_id: user.id.clone(), email: None, role: Some("member".to_string()) })
    };
    let mut owned = Box::pin(schema.execute_stream(request(&owner)));
    let mut foreign = Box::pin(schema.execute_stream(request(&outsider)));
    assert!(owned.next().now_or_never().is_none());
    assert!(foreign.next().now_or_never().is_none());
    Track::delete_by_id(&db, &song.id).await.unwrap();
    let response = tokio::time::timeout(Duration::from_secs(2), owned.next())
        .await
        .unwrap()
        .unwrap();
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let payload = response.data.into_json().unwrap();
    assert_eq!(payload["albumChanged"]["album"]["id"], record.id);
    assert_eq!(payload["albumChanged"]["album"]["name"], "fixture album");
    assert_eq!(payload["albumChanged"]["album"]["hasFiles"], false);
    assert_eq!(
        payload["albumChanged"]["sourceEntity"],
        serde_json::Value::Null
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(40), foreign.next())
            .await
            .is_err()
    );
    assert!(Library::get(db.pool(), &lib.id).await.unwrap().is_some());
}

#[tokio::test]
async fn summaries_and_backfill_read_every_transaction_page() {
    let raw = database(false).await;
    let (_, lib, performer, first) = music_fixture(&raw).await;
    let second = album(&raw, &lib, &performer).await;
    let third = album(&raw, &lib, &performer).await;
    for record in [&first, &second, &third] {
        for _ in 0..3 {
            let file = media_file(&raw, &lib, 7, None, None).await;
            track(&raw, record, Some(&file.id)).await;
        }
    }
    let mut db = DatabaseService::new(raw).pool().clone();
    db.set_pagination_config(PaginationConfig::explicit_only(2));
    let report = reconcile_stored_media_summaries(&db, false).await.unwrap();
    assert_eq!((report.albums_examined, report.albums_changed), (3, 3));
    for record in [&first, &second, &third] {
        assert_album(&db, &record.id, true, 21).await;
    }
}

#[tokio::test]
async fn backfill_failure_rolls_back_prior_repairs_and_all_generated_events() {
    let raw = database(false).await;
    let (_, lib, performer, valid) = music_fixture(&raw).await;
    let invalid = album(&raw, &lib, &performer).await;
    let small = media_file(&raw, &lib, 1, None, None).await;
    track(&raw, &valid, Some(&small.id)).await;
    let maximum = media_file(&raw, &lib, i64::MAX, None, None).await;
    track(&raw, &invalid, Some(&maximum.id)).await;
    track(&raw, &invalid, Some(&small.id)).await;
    let db = DatabaseService::new(raw).pool().clone();
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<AlbumChangedEvent>>()
        .subscribe();
    assert!(reconcile_stored_media_summaries(&db, false).await.is_err());
    for record in [valid, invalid] {
        let unchanged = Album::get(db.pool(), &record.id).await.unwrap().unwrap();
        assert!(!unchanged.has_files);
        assert_eq!(unchanged.size_bytes, None);
    }
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn media_file_delete_cleanup_and_summary_changes_rollback_together() {
    let db = database(true).await;
    let (_, lib, _, record) = music_fixture(&db).await;
    let file = media_file(&db, &lib, 41, None, None).await;
    let song = track(&db, &record, Some(&file.id)).await;
    let video = video_stream(&db, &file).await;
    let run = library_scan_run(&db, &lib).await;
    let issue = library_scan_issue(&db, &lib, &run, &file).await;
    let mut changes = db
        .ensure_event_sender::<SubscriptionEvent<AlbumChangedEvent>>()
        .subscribe();
    let deleted_id = file.id.clone();
    let failed = db
        .transaction(TransactionMode::StateMachine, move |tx| {
            Box::pin(async move {
                tx.delete_by_id::<MediaFile>(&deleted_id).await?;
                assert!(tx.find_by_id::<MediaFile>(&deleted_id).await?.is_none());
                Err::<(), _>(OrmPublicError::new(OrmErrorCode::Conflict))
            })
        })
        .await;
    assert!(failed.is_err());
    assert_album(&db, &record.id, true, 41).await;
    assert_eq!(
        Track::get(db.pool(), &song.id)
            .await
            .unwrap()
            .unwrap()
            .media_file_id,
        Some(file.id.clone())
    );
    assert_eq!(
        MediaFile::get(db.pool(), &file.id)
            .await
            .unwrap()
            .unwrap()
            .track_id,
        Some(song.id)
    );
    assert!(
        VideoStream::get(db.pool(), &video.id)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        LibraryScanIssue::get(db.pool(), &issue.id)
            .await
            .unwrap()
            .unwrap()
            .media_file_id,
        Some(file.id)
    );
    assert!(changes.try_recv().is_err());
}

#[tokio::test]
async fn episode_reparent_and_delete_directly_invalidate_surviving_shows() {
    let db = database(true).await;
    let owner = user(&db).await;
    let lib = library(&db, &owner).await;
    let first = show(&db, &lib).await;
    let second = show(&db, &lib).await;
    let ep = episode(&db, &first, None).await;
    let mut changes = db
        .ensure_event_sender::<SubscriptionEvent<ShowChangedEvent>>()
        .subscribe();
    Episode::update_by_id(
        &db,
        &ep.id,
        UpdateEpisodeInput {
            show_id: Some(second.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mut direct = BTreeSet::new();
    while let Ok(change) = changes.try_recv() {
        if change.event.source_entity.is_none() {
            direct.insert(change.event.id.clone());
        }
    }
    assert_eq!(
        direct,
        BTreeSet::from([first.id.clone(), second.id.clone()])
    );
    Episode::delete_by_id(&db, &ep.id).await.unwrap();
    let deleted_parent_event = std::iter::from_fn(|| changes.try_recv().ok())
        .any(|change| change.event.id == second.id && change.event.source_entity.is_none());
    assert!(deleted_parent_event);
    for series in [first, second] {
        assert_eq!(
            Show::get(db.pool(), &series.id)
                .await
                .unwrap()
                .unwrap()
                .name,
            "fixture show"
        );
    }
}

#[tokio::test]
async fn stream_reparent_and_delete_invalidate_surviving_parents_without_metadata_touches() {
    let db = database(true).await;
    let owner = user(&db).await;
    let old_library = library(&db, &owner).await;
    let new_library = library(&db, &owner).await;
    let first = media_file(&db, &old_library, 10, None, None).await;
    let second = media_file(&db, &new_library, 20, None, None).await;
    let video = video_stream(&db, &first).await;
    let mut file_events = db
        .ensure_event_sender::<SubscriptionEvent<MediaFileChangedEvent>>()
        .subscribe();
    let mut library_events = db
        .ensure_event_sender::<SubscriptionEvent<LibraryChangedEvent>>()
        .subscribe();
    VideoStream::update_by_id(
        &db,
        &video.id,
        UpdateVideoStreamInput {
            media_file_id: Some(second.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let direct_files = std::iter::from_fn(|| file_events.try_recv().ok())
        .filter(|change| change.event.source_entity.is_none())
        .map(|change| change.event.id.clone())
        .collect::<BTreeSet<_>>();
    let direct_libraries = std::iter::from_fn(|| library_events.try_recv().ok())
        .filter(|change| change.event.source_entity.is_none())
        .map(|change| change.event.id.clone())
        .collect::<BTreeSet<_>>();
    if <MediaFile as graphql_orm::graphql::orm::Entity>::metadata()
        .fields
        .iter()
        .any(|field| field.name == "updated_at")
    {
        assert_eq!(
            direct_files,
            BTreeSet::from([first.id.clone(), second.id.clone()])
        );
    } else {
        // Explicit compatibility boundary until the entity owner adds updatedAt.
        assert_eq!(
            direct_libraries,
            BTreeSet::from([old_library.id.clone(), new_library.id.clone()])
        );
    }
    VideoStream::delete_by_id(&db, &video.id).await.unwrap();
    if <MediaFile as graphql_orm::graphql::orm::Entity>::metadata()
        .fields
        .iter()
        .any(|field| field.name == "updated_at")
    {
        assert!(
            std::iter::from_fn(|| file_events.try_recv().ok())
                .any(|change| change.event.id == second.id && change.event.source_entity.is_none())
        );
    } else {
        assert!(std::iter::from_fn(|| library_events.try_recv().ok()).any(
            |change| change.event.id == new_library.id && change.event.source_entity.is_none()
        ));
    }
    for file in [first, second] {
        let unchanged = MediaFile::get(db.pool(), &file.id).await.unwrap().unwrap();
        assert_eq!(unchanged.analyzed_at, None);
        assert_eq!(unchanged.file_modified_at, None);
        assert_eq!(unchanged.path, file.path);
        assert_eq!(unchanged.size, file.size);
    }
}

async fn session_token(db: &Database, owner: &User, revoked: bool) -> RefreshToken {
    RefreshToken::insert(
        db,
        CreateRefreshTokenInput {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: owner.id.clone(),
            token_hash: uuid::Uuid::new_v4().to_string(),
            session_id: uuid::Uuid::new_v4().to_string(),
            session_family_id: uuid::Uuid::new_v4().to_string(),
            scopes: vec!["library.read".to_string()],
            session: "{}".to_string(),
            grant_source: Some("\"StoredUser\"".to_string()),
            roles: Some("[\"member\"]".to_string()),
            authorization_roles: Some("[\"member\"]".to_string()),
            ip_address: None,
            user_agent: None,
            expires_at: "2099-01-01T00:00:00Z".to_string(),
            last_used_at: None,
            revoked_at: revoked.then(|| "2026-01-01T00:00:00Z".to_string()),
            replaced_by_token_id: None,
            revocation_reason: revoked.then(|| "Rotated".to_string()),
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn password_change_revokes_only_the_changed_users_sessions() {
    let db = database(true).await;
    let owner = user(&db).await;
    let other = user(&db).await;
    let session = session_token(&db, &owner, false).await;
    let foreign = session_token(&db, &other, false).await;
    User::update_by_id(
        &db,
        &owner.id,
        UpdateUserInput {
            password_hash: Some("changed fixture credential".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        RefreshToken::get(db.pool(), &session.id)
            .await
            .unwrap()
            .unwrap()
            .revoked_at
            .is_some()
    );
    assert!(
        RefreshToken::get(db.pool(), &foreign.id)
            .await
            .unwrap()
            .unwrap()
            .revoked_at
            .is_none()
    );
}

#[tokio::test]
async fn role_change_and_deactivation_revoke_every_token_beyond_one_transaction_page() {
    let db = database(true).await;
    let owner = user(&db).await;
    let other = user(&db).await;
    let already_revoked = session_token(&db, &owner, true).await;
    let foreign = session_token(&db, &other, false).await;
    let mut active = Vec::new();
    for _ in 0..105 {
        active.push(session_token(&db, &owner, false).await);
    }
    User::update_by_id(
        &db,
        &owner.id,
        UpdateUserInput {
            role: Some("admin".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    for token in &active {
        let stored = RefreshToken::get(db.pool(), &token.id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            stored.revoked_at.is_some(),
            "token {} survived role revocation",
            token.id
        );
        assert_eq!(stored.revocation_reason.as_deref(), Some("AdminRevoked"));
    }
    let mut replacement_sessions = Vec::new();
    for _ in 0..105 {
        replacement_sessions.push(session_token(&db, &owner, false).await);
    }
    User::update_by_id(
        &db,
        &owner.id,
        UpdateUserInput {
            is_active: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    for token in replacement_sessions {
        assert!(
            RefreshToken::get(db.pool(), &token.id)
                .await
                .unwrap()
                .unwrap()
                .revoked_at
                .is_some(),
            "token {} survived deactivation",
            token.id
        );
    }
    let unchanged = RefreshToken::get(db.pool(), &already_revoked.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.revoked_at, already_revoked.revoked_at);
    assert_eq!(
        unchanged.revocation_reason,
        already_revoked.revocation_reason
    );
    assert!(
        RefreshToken::get(db.pool(), &foreign.id)
            .await
            .unwrap()
            .unwrap()
            .revoked_at
            .is_none()
    );
}

#[tokio::test]
async fn multi_page_session_revocation_rolls_back_with_the_user_change_and_events() {
    let db = database(true).await;
    let owner = user(&db).await;
    let mut tokens = Vec::new();
    for _ in 0..105 {
        tokens.push(session_token(&db, &owner, false).await);
    }
    let token_ids = tokens
        .iter()
        .map(|token| token.id.clone())
        .collect::<Vec<_>>();
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<RefreshTokenChangedEvent>>()
        .subscribe();
    let user_id = owner.id.clone();
    let failed = db
        .transaction(TransactionMode::StateMachine, move |tx| {
            Box::pin(async move {
                tx.update_by_id::<User>(
                    &user_id,
                    UpdateUserInput {
                        is_active: Some(false),
                        ..Default::default()
                    },
                )
                .await?;
                for id in token_ids {
                    assert!(
                        tx.find_by_id::<RefreshToken>(&id)
                            .await?
                            .unwrap()
                            .revoked_at
                            .is_some()
                    );
                }
                Err::<(), _>(OrmPublicError::new(OrmErrorCode::Conflict))
            })
        })
        .await;
    assert!(failed.is_err());
    assert!(
        User::get(db.pool(), &owner.id)
            .await
            .unwrap()
            .unwrap()
            .is_active
    );
    for token in tokens {
        assert!(
            RefreshToken::get(db.pool(), &token.id)
                .await
                .unwrap()
                .unwrap()
                .revoked_at
                .is_none()
        );
    }
    assert!(events.try_recv().is_err());
}
