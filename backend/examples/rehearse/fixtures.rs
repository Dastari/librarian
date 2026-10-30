//! Generated typed fixture operations, intentionally without lifecycle hooks to model legacy drift.
use super::*;

pub(super) async fn user(db: &Database) -> User {
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

pub(super) async fn library(db: &Database, user: &User) -> Library {
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

pub(super) async fn artist(db: &Database, library: &Library) -> Artist {
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

pub(super) async fn album(db: &Database, library: &Library, artist: &Artist) -> Album {
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

pub(super) async fn audiobook(db: &Database, library: &Library) -> Audiobook {
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

pub(super) async fn track(db: &Database, album: &Album, file_id: Option<&str>) -> Track {
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

pub(super) async fn chapter(db: &Database, book: &Audiobook, file_id: Option<&str>) -> Chapter {
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

pub(super) async fn media_file(
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
            size: size,
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
            added_at: "2026-10-26T00:00:00Z".to_string(),
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
