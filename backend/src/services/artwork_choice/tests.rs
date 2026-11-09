//! Provider lists parsed from recorded responses (TVmaze, Cover Art Archive, Wikidata, Deezer and
//! Open Library were recorded on 2026-11-09; TMDB's follows its documented shape, as it needs a
//! key), locks holding through refreshes, and upload checks. No network.

use std::sync::Arc;

use graphql_orm::DbPool;
use serde_json::Value;

use super::options::*;
use super::*;
use crate::graphql::entities::*;
use crate::services::metadata::musicbrainz::{CoverArtArchiveResult, MusicBrainzEdition};
use crate::services::metadata::tmdb::images::TmdbImages;
use crate::services::metadata::tvmaze::{TvMazeSeason, TvMazeShowImage};
use crate::services::{ObjectStorageService, storage::ObjectStorageServiceConfig};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/artwork/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|error| panic!("fixture {name}: {error}"))
}

fn json(name: &str) -> Value {
    serde_json::from_str(&fixture(name)).unwrap()
}

// ---------------------------------------------------------------------------
// Provider lists
// ---------------------------------------------------------------------------

#[test]
fn tmdb_lists_every_language_best_voted_first_at_original_size() {
    let images: TmdbImages = serde_json::from_str(&fixture("tmdb_movie_550_images.json")).unwrap();

    let posters = tmdb_options(&images, ArtworkKind::Poster);
    assert_eq!(posters.len(), 3, "the poster with no file is left out");
    assert_eq!(
        posters[0].url,
        "https://image.tmdb.org/t/p/original/a26cQPRhJPX6GbWfQbvZdrrp9j9.jpg"
    );
    assert_eq!(
        posters[0].preview_url,
        "https://image.tmdb.org/t/p/w342/a26cQPRhJPX6GbWfQbvZdrrp9j9.jpg"
    );
    assert_eq!(posters[0].language.as_deref(), Some("en"));
    assert_eq!(
        (posters[0].width, posters[0].height),
        (Some(1400), Some(2100))
    );
    assert_eq!(posters[0].vote_count, Some(25));
    assert!(
        posters
            .iter()
            .any(|poster| poster.language.as_deref() == Some("de"))
    );

    let backdrops = tmdb_options(&images, ArtworkKind::Backdrop);
    assert_eq!(backdrops.len(), 3);
    assert!(backdrops[0].preview_url.contains("/w780/"));
    assert!(
        backdrops
            .iter()
            .filter(|backdrop| backdrop.language.is_none())
            .count()
            == 2,
        "no language and TMDB's \"xx\" both mean textless"
    );

    let logos = tmdb_options(&images, ArtworkKind::Logo);
    assert_eq!(logos.len(), 1);
    assert!(logos[0].url.ends_with(".png"));
    assert!(tmdb_options(&images, ArtworkKind::Thumbnail).is_empty());
}

#[test]
fn tvmaze_lists_posters_and_backgrounds_with_the_main_poster_first() {
    let images: Vec<TvMazeShowImage> =
        serde_json::from_str(&fixture("tvmaze_show_82_images.json")).unwrap();

    let posters = tvmaze_show_options(&images, ArtworkKind::Poster);
    assert_eq!(posters.len(), 4);
    // The main poster (image 1230408) is stored as file 1245274.
    assert!(posters[0].url.ends_with("/1245274.jpg"), "{posters:?}");
    assert!(posters[0].preview_url.contains("medium_portrait"));
    assert!(posters.iter().all(|poster| poster.source == "TVmaze"));
    assert_eq!(
        tvmaze_show_default(&images, ArtworkKind::Poster).as_deref(),
        Some(posters[0].url.as_str())
    );

    let backgrounds = tvmaze_show_options(&images, ArtworkKind::Backdrop);
    assert_eq!(backgrounds.len(), 3, "banners are not backdrops");
    assert!(backgrounds.iter().all(|option| option.width.is_some()));
    let default = tvmaze_show_default(&images, ArtworkKind::Backdrop).unwrap();
    assert!(backgrounds.iter().any(|option| option.url == default));
}

#[test]
fn tvmaze_season_poster_is_the_seasons_own() {
    let seasons: Vec<TvMazeSeason> =
        serde_json::from_str(&fixture("tvmaze_show_82_seasons.json")).unwrap();
    let season = tvmaze_season_option(&seasons, 2).unwrap();
    assert!(season.url.ends_with("original_untouched/24/60660.jpg"));
    assert!(season.preview_url.contains("medium_portrait"));
    assert!(season.is_default);
    assert!(tvmaze_season_option(&seasons, 40).is_none());
}

#[test]
fn cover_art_archive_lists_front_back_and_the_rest_labelled() {
    let result: CoverArtArchiveResult =
        serde_json::from_str(&fixture("caa_release_group.json")).unwrap();
    let edition = MusicBrainzEdition {
        id: "956fbc58-362d-43b8-b880-3779e0508559".to_string(),
        official: true,
        track_count: 12,
        date: Some("1987-07-21".to_string()),
        country: Some("US".to_string()),
        disambiguation: None,
    };
    let label = edition_label(&edition);
    assert_eq!(label.as_deref(), Some("1987 · US"));

    let options = cover_art_options(&result, label.as_deref());
    assert_eq!(options.len(), result.images.len());
    assert_eq!(options[0].label.as_deref(), Some("Front · 1987 · US"));
    assert_eq!(options[1].label.as_deref(), Some("Back · 1987 · US"));
    assert!(
        options
            .iter()
            .all(|option| option.url.starts_with("https://"))
    );
    assert!(
        options
            .iter()
            .all(|option| option.preview_url.starts_with("https://"))
    );

    let default = cover_art_default(&result).unwrap();
    assert!(
        same_image(&default, &options[0].url),
        "the scan's cover is the front's 1200px copy, whatever the scheme"
    );
}

#[test]
fn wikidata_and_deezer_pictures_of_an_artist() {
    use crate::services::metadata::artist_image::{
        commons_files_from_claims, commons_image_url, deezer_artist_images,
    };
    let files = commons_files_from_claims(&json("wikidata_p18.json"));
    assert_eq!(files.len(), 2, "both photos Wikidata names");
    let urls = files
        .iter()
        .filter_map(|file| commons_image_url(file))
        .collect::<Vec<_>>();
    let options = url_options("Wikimedia Commons", &urls);
    assert!(options.iter().all(|option| {
        option
            .url
            .starts_with("https://commons.wikimedia.org/wiki/Special:FilePath/")
    }));

    let pictures = deezer_artist_images(&json("deezer_search_artist.json"), "Guns N' Roses");
    assert!(!pictures.is_empty());
    assert!(
        pictures.len() < 5,
        "only artists named exactly so ({pictures:?})"
    );
    assert!(pictures.iter().all(|picture| picture.contains("dzcdn.net")));
}

#[test]
fn open_library_work_and_search_covers() {
    let work = open_library_work_options(&json("openlibrary_work.json"));
    assert_eq!(work.len(), 9);
    assert_eq!(
        work[0].url,
        "https://covers.openlibrary.org/b/id/14625765-L.jpg"
    );
    assert_eq!(
        work[0].preview_url,
        "https://covers.openlibrary.org/b/id/14625765-M.jpg"
    );

    let found = open_library_search_options(&json("openlibrary_search.json"));
    assert_eq!(found.len(), 5, "a result with no cover is left out");
    assert_eq!(found[0].label.as_deref(), Some("The Hobbit · 1937"));
}

#[test]
fn the_same_picture_at_another_size_or_scheme_is_the_same_picture() {
    assert!(same_image(
        "https://image.tmdb.org/t/p/original/abc.jpg",
        "https://image.tmdb.org/t/p/w500/abc.jpg"
    ));
    assert!(same_image(
        "http://coverartarchive.org/release/x/1-1200.jpg",
        "https://coverartarchive.org/release/x/1-1200.jpg"
    ));
    assert!(!same_image(
        "https://image.tmdb.org/t/p/original/abc.jpg",
        "https://image.tmdb.org/t/p/original/abd.jpg"
    ));
}

#[test]
fn slots_name_their_field_and_path() {
    let poster = ArtworkSlot::new(ArtworkEntityKind::Movie, ArtworkKind::Poster).unwrap();
    assert_eq!(
        (poster.entity_type(), poster.artwork_type(), poster.field()),
        ("movie", "poster", Some("posterUrl"))
    );
    assert_eq!(
        ArtworkSlot::from_path("Episode", "thumbnail")
            .unwrap()
            .field(),
        Some("stillUrl")
    );
    assert_eq!(
        ArtworkSlot::from_path("artist", "poster").unwrap().field(),
        Some("imageUrl")
    );
    let logo = ArtworkSlot::from_path("movie", "logo").unwrap();
    assert!(logo.field().is_none() && !logo.refreshable());
    let playlist = ArtworkSlot::from_path("playlist", "cover").unwrap();
    assert!(!playlist.has_provider() && !playlist.refreshable());
    assert!(ArtworkSlot::new(ArtworkEntityKind::Album, ArtworkKind::Backdrop).is_none());
    assert!(ArtworkSlot::from_path("person", "profile").is_none());
}

// ---------------------------------------------------------------------------
// Uploads
// ---------------------------------------------------------------------------

fn png(width: u32, height: u32, alpha: bool) -> Vec<u8> {
    let image = if alpha {
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            width,
            height,
            image::Rgba([200, 30, 30, 128]),
        ))
    } else {
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            width,
            height,
            image::Rgb([30, 90, 200]),
        ))
    };
    let mut out = std::io::Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

#[test]
fn uploads_must_be_real_pictures_of_a_usable_size() {
    assert_eq!(prepare_upload(&[]).unwrap_err(), UploadError::Empty);
    assert_eq!(
        prepare_upload(b"<svg xmlns='http://www.w3.org/2000/svg'/>").unwrap_err(),
        UploadError::UnsupportedType
    );
    assert_eq!(
        prepare_upload(b"GIF89a\x01\x00\x01\x00").unwrap_err(),
        UploadError::UnsupportedType
    );
    let mut truncated = png(400, 600, false);
    truncated.truncate(60);
    assert_eq!(
        prepare_upload(&truncated).unwrap_err(),
        UploadError::Unreadable
    );
    assert_eq!(
        prepare_upload(&png(400, 60, false)).unwrap_err(),
        UploadError::TooSmall
    );
    let too_big = vec![0_u8; upload::MAX_UPLOAD_BYTES + 1];
    assert_eq!(prepare_upload(&too_big).unwrap_err(), UploadError::TooLarge);
}

#[test]
fn uploads_are_encoded_again_and_scaled_to_the_widest_screen() {
    let photo = prepare_upload(&png(400, 600, false)).unwrap();
    assert_eq!(
        photo.mime_type, "image/jpeg",
        "an opaque picture becomes JPEG"
    );
    assert_eq!((photo.width, photo.height), (400, 600));
    assert_eq!(
        image::guess_format(&photo.data).unwrap(),
        image::ImageFormat::Jpeg
    );
    assert_eq!(photo.sha256_hex.len(), 64);

    let logo = prepare_upload(&png(500, 120, true)).unwrap();
    assert_eq!(logo.mime_type, "image/png", "transparency is kept");

    let wide = prepare_upload(&png(4000, 200, false)).unwrap();
    assert_eq!((wide.width, wide.height), (3840, 192));
}

// ---------------------------------------------------------------------------
// Locks
// ---------------------------------------------------------------------------

async fn database() -> Database {
    let db = Database::new(DbPool::connect("sqlite::memory:").await.unwrap());
    crate::services::database::bootstrap_schema(&db)
        .await
        .unwrap();
    crate::services::DatabaseService::new(db).pool().clone()
}

async fn owner(db: &Database) -> (User, Library) {
    let user = User::insert(
        db,
        CreateUserInput {
            username: uuid::Uuid::new_v4().to_string(),
            email: None,
            password_hash: "fixture".to_string(),
            role: "admin".to_string(),
            display_name: None,
            avatar_url: None,
            is_active: true,
            last_login_at: None,
        },
    )
    .await
    .unwrap();
    let library = Library::insert(
        db,
        CreateLibraryInput {
            user_id: user.id.clone(),
            name: "Music".to_string(),
            path: format!("/fixture/{}", uuid::Uuid::new_v4()),
            library_type: "music".to_string(),
            icon: None,
            color: None,
            auto_scan: false,
            auto_organize: false,
            auto_resolve_duplicates: None,
            naming_pattern: "{title}".to_string(),
            scan_interval_minutes: 0,
            watch_for_changes: false,
            watch_status: None,
            watch_detail: None,
            watch_status_since: None,
            scanning: false,
            last_scanned_at: None,
            quality_profile_id: None,
        },
    )
    .await
    .unwrap();
    (user, library)
}

async fn album(db: &Database, library: &Library, cover: &str) -> Album {
    let artist = Artist::insert(
        db,
        CreateArtistInput {
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            name: "Guns N' Roses".to_string(),
            sort_name: None,
            musicbrainz_id: None,
            bio: None,
            disambiguation: None,
            image_url: None,
            locked_fields: None,
            album_count: None,
            track_count: None,
            total_duration_secs: None,
        },
    )
    .await
    .unwrap();
    Album::insert(
        db,
        CreateAlbumInput {
            artist_id: artist.id.clone(),
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            name: "Appetite for Destruction".to_string(),
            sort_name: None,
            year: None,
            musicbrainz_id: None,
            album_type: None,
            genres: Vec::new(),
            label: None,
            country: None,
            release_date: None,
            cover_url: Some(cover.to_string()),
            locked_fields: None,
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
    .await
    .unwrap()
}

fn artwork_service(db: &Database) -> (tempfile::TempDir, ArtworkService) {
    let dir = tempfile::tempdir().unwrap();
    let storage = ObjectStorageService::new(ObjectStorageServiceConfig {
        backend: "local".to_string(),
        local_path: dir.path().to_path_buf(),
    })
    .unwrap();
    (dir, ArtworkService::new(db.clone(), Arc::new(storage)))
}

const CHOSEN: &str = "https://coverartarchive.org/release/chosen/1-1200.jpg";
const PROVIDER: &str = "https://coverartarchive.org/release/default/2-1200.jpg";

#[tokio::test]
async fn a_refresh_cannot_write_locked_artwork_but_a_choice_can() {
    let db = database().await;
    let (_, library) = owner(&db).await;
    let album = album(&db, &library, PROVIDER).await;
    let slot = ArtworkSlot::new(ArtworkEntityKind::Album, ArtworkKind::Cover).unwrap();

    write_slot(&db, slot, &album.id, Some(CHOSEN.to_string()), true)
        .await
        .unwrap();
    let chosen = Album::get(db.pool(), &album.id).await.unwrap().unwrap();
    assert_eq!(chosen.cover_url.as_deref(), Some(CHOSEN));
    assert_eq!(
        chosen.locked_fields.as_deref(),
        Some(&["coverUrl".to_string()][..])
    );

    // The scan's cover backfill and every refresh write the field alone.
    let refreshed = Album::update_by_id(
        &db,
        &album.id,
        UpdateAlbumInput {
            cover_url: Some(Some(PROVIDER.to_string())),
            name: Some("Appetite for Destruction (Remastered)".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        refreshed.cover_url.as_deref(),
        Some(CHOSEN),
        "the lock keeps the chosen cover"
    );
    assert_eq!(
        refreshed.name, "Appetite for Destruction (Remastered)",
        "the rest of the refresh still lands"
    );

    // Unlocking is the generated update of lockedFields; then a refresh writes again.
    Album::update_by_id(
        &db,
        &album.id,
        UpdateAlbumInput {
            locked_fields: Some(Some(Vec::new())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let refreshed = Album::update_by_id(
        &db,
        &album.id,
        UpdateAlbumInput {
            cover_url: Some(Some(PROVIDER.to_string())),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(refreshed.cover_url.as_deref(), Some(PROVIDER));
}

#[tokio::test]
async fn a_movie_lock_covers_only_the_locked_picture() {
    let db = database().await;
    let (_, library) = owner(&db).await;
    let movie = Movie::insert(
        &db,
        CreateMovieInput {
            locked_fields: Some(vec!["backdropUrl".to_string()]),
            library_id: library.id.clone(),
            user_id: library.user_id.clone(),
            title: "Heat".to_string(),
            sort_title: None,
            original_title: None,
            year: Some(1995),
            tmdb_id: Some(949),
            digital_release_date: None,
            physical_release_date: None,
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
            poster_url: Some("https://image.tmdb.org/t/p/original/a.jpg".to_string()),
            backdrop_url: Some("https://image.tmdb.org/t/p/original/b.jpg".to_string()),
            collection_id: None,
            collection_name: None,
            collection_poster_url: None,
            release_date: None,
            certification: None,
            monitored: false,
            tmdb_status: None,
            wanted: false,
            ignored: None,
            download_status: None,
            has_file: false,
            media_file_id: None,
            quality_profile_id: None,
        },
    )
    .await
    .unwrap();
    let refreshed = Movie::update_by_id(
        &db,
        &movie.id,
        UpdateMovieInput {
            poster_url: Some(Some(
                "https://image.tmdb.org/t/p/original/c.jpg".to_string(),
            )),
            backdrop_url: Some(Some(
                "https://image.tmdb.org/t/p/original/d.jpg".to_string(),
            )),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        refreshed.poster_url.as_deref(),
        Some("https://image.tmdb.org/t/p/original/c.jpg"),
        "an unlocked poster follows the provider"
    );
    assert_eq!(
        refreshed.backdrop_url.as_deref(),
        Some("https://image.tmdb.org/t/p/original/b.jpg"),
        "the locked backdrop stays"
    );
}

async fn cached(db: &Database, artwork: &ArtworkService, album: &Album, source: &str) {
    // What an earlier cache of the chosen picture left: a stored original from that source.
    let stored = StorageObject::insert(
        db,
        CreateStorageObjectInput {
            object_id: uuid::Uuid::new_v4().to_string(),
            namespace: "derivatives".to_string(),
            backend: "local".to_string(),
            storage_key: "fixture/chosen.jpg".to_string(),
            original_file_name: Some("chosen.jpg".to_string()),
            mime_type: Some("image/jpeg".to_string()),
            size_bytes: 1,
            sha256_hex: format!("{:0>64}", 1),
        },
    )
    .await
    .unwrap();
    ArtworkCache::insert(
        db,
        CreateArtworkCacheInput {
            entity_type: "album".to_string(),
            entity_id: album.id.clone(),
            artwork_type: "cover".to_string(),
            storage_object_id: stored.id,
            source_url: Some(source.to_string()),
            width: Some(1200),
            height: Some(1200),
        },
    )
    .await
    .unwrap();
    let _ = artwork;
}

#[tokio::test]
async fn a_refresh_asking_the_cache_for_another_picture_keeps_the_chosen_one() {
    let db = database().await;
    let (_, library) = owner(&db).await;
    let album = album(&db, &library, CHOSEN).await;
    let slot = ArtworkSlot::new(ArtworkEntityKind::Album, ArtworkKind::Cover).unwrap();
    write_slot(&db, slot, &album.id, Some(CHOSEN.to_string()), true)
        .await
        .unwrap();
    let (_dir, artwork) = artwork_service(&db);
    cached(&db, &artwork, &album, CHOSEN).await;

    // A refresh caching the provider's cover returns before any download.
    let url = artwork.cache_album_artwork(&album.id, Some(PROVIDER)).await;
    assert_eq!(
        url.as_deref(),
        Some(format!("/api/artwork/album/{}/cover", album.id).as_str())
    );
    let row = cached_original(&db, slot, &album.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.source_url.as_deref(), Some(CHOSEN));
}

#[tokio::test]
async fn an_upload_is_stored_with_its_sizes_and_locked_then_reverting_a_playlist_empties_it() {
    let db = database().await;
    let (user, library) = owner(&db).await;
    let album = album(&db, &library, PROVIDER).await;
    let (_dir, artwork) = artwork_service(&db);
    let slot = ArtworkSlot::new(ArtworkEntityKind::Album, ArtworkKind::Cover).unwrap();

    let size = upload(&db, &artwork, slot, &album.id, &png(800, 800, false))
        .await
        .unwrap();
    assert_eq!(size, (800, 800));
    let saved = Album::get(db.pool(), &album.id).await.unwrap().unwrap();
    assert!(
        saved
            .cover_url
            .as_deref()
            .unwrap()
            .starts_with(UPLOAD_SOURCE_PREFIX)
    );
    assert!(
        saved
            .locked_fields
            .unwrap()
            .contains(&"coverUrl".to_string())
    );
    let rows = ArtworkCache::query(db.pool()).fetch_all().await.unwrap();
    let mut kinds = rows
        .iter()
        .filter(|row| row.entity_id == album.id)
        .map(|row| row.artwork_type.clone())
        .collect::<Vec<_>>();
    kinds.sort();
    assert_eq!(
        kinds,
        [
            "cover",
            "cover@w185",
            "cover@w342",
            "cover@w500",
            "cover@w780"
        ],
        "the sizes below the original are made at once"
    );

    // A refresh can't put the provider's cover back over an upload.
    artwork.cache_album_artwork(&album.id, Some(PROVIDER)).await;
    let state = describe_slot(
        &db,
        slot,
        &album.id,
        &slot_state(&db, slot, &album.id).await.unwrap().unwrap(),
    )
    .await
    .unwrap();
    assert!(state.locked && state.uploaded && state.has_artwork);

    // A playlist has no provider: reverting removes its cover.
    let list = PersonalList::insert(
        &db,
        CreatePersonalListInput {
            user_id: user.id.clone(),
            name: "Road trip".to_string(),
            description: None,
            cover_url: None,
            kind: PersonalListKind::Manual,
            saved_view_id: None,
        },
    )
    .await
    .unwrap();
    let cover = ArtworkSlot::new(ArtworkEntityKind::Playlist, ArtworkKind::Cover).unwrap();
    upload(&db, &artwork, cover, &list.id, &png(600, 600, false))
        .await
        .unwrap();
    assert!(
        PersonalList::get(db.pool(), &list.id)
            .await
            .unwrap()
            .unwrap()
            .cover_url
            .is_some()
    );
    revert(&db, &artwork, None, cover, &list.id).await.unwrap();
    assert!(
        PersonalList::get(db.pool(), &list.id)
            .await
            .unwrap()
            .unwrap()
            .cover_url
            .is_none()
    );
    assert!(
        cached_original(&db, cover, &list.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn reverting_puts_back_the_default_or_changes_nothing_when_a_provider_is_down() {
    let db = database().await;
    let (_, library) = owner(&db).await;
    let album = album(&db, &library, CHOSEN).await;
    let slot = ArtworkSlot::new(ArtworkEntityKind::Album, ArtworkKind::Cover).unwrap();
    write_slot(&db, slot, &album.id, Some(CHOSEN.to_string()), true)
        .await
        .unwrap();
    let (_dir, artwork) = artwork_service(&db);

    let down = ProviderOptions {
        options: Vec::new(),
        unavailable: vec!["Cover Art Archive".to_string()],
    };
    let error = revert(&db, &artwork, Some(&down), slot, &album.id)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Cover Art Archive"), "{error}");
    let kept = Album::get(db.pool(), &album.id).await.unwrap().unwrap();
    assert_eq!(kept.cover_url.as_deref(), Some(CHOSEN));
    assert!(
        kept.locked_fields
            .unwrap()
            .contains(&"coverUrl".to_string())
    );

    // A choice must be one the provider listed.
    let listed = ProviderOptions::default();
    assert!(
        choose(
            &db,
            &artwork,
            &listed,
            slot,
            &album.id,
            "https://coverartarchive.org/x.jpg"
        )
        .await
        .is_err()
    );
}
