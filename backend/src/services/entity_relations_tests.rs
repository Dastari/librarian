//! Relation propagation, delete policies and dangling ids, driven through the generated
//! entities, the lifecycle hooks and the generated `Changed` subscription resolvers. No
//! application SQL and no fabricated events.
use super::parent_summary_tests::{member_request, pending, source_torrent};
use super::summary_tests::{
    album, artist, audiobook, chapter, database, episode, library, media_file, movie,
    movie_cast_credit, music_fixture, person, show, track, user,
};
use super::*;
use futures::StreamExt;
use graphql_orm::DbPool;
use graphql_orm::graphql::subscriptions::SubscriptionEvent;

/// Every event on a raw channel since the last call as (id, source entity). A source entity
/// means the event was propagated from that child; none means a direct write to the row.
macro_rules! drain {
    ($rx:expr) => {{
        let mut seen = std::iter::from_fn(|| $rx.try_recv().ok())
            .map(|change| (change.event.id.clone(), change.event.source_entity.clone()))
            .collect::<Vec<(String, Option<String>)>>();
        seen.sort();
        seen
    }};
}
macro_rules! tap {
    ($db:expr, $event:ty) => {
        $db.ensure_event_sender::<SubscriptionEvent<$event>>()
            .subscribe()
    };
}

fn event(id: &str, source: Option<&str>) -> (String, Option<String>) {
    (id.to_string(), source.map(str::to_string))
}

#[derive(Default, Clone)]
struct Links {
    library: Option<String>,
    movie: Option<String>,
    show: Option<String>,
    episode: Option<String>,
    track: Option<String>,
    chapter: Option<String>,
    album: Option<String>,
    audiobook: Option<String>,
    feed: Option<String>,
}

async fn torrent(db: &Database, owner: &User, links: Links) -> Torrent {
    Torrent::insert(
        db,
        CreateTorrentInput {
            user_id: owner.id.clone(),
            info_hash: format!("{:040x}", uuid::Uuid::new_v4().as_u128()),
            magnet_uri: None,
            name: "relation fixture".into(),
            state: "downloading".into(),
            progress: 0.0,
            total_bytes: 5,
            downloaded_bytes: 0,
            uploaded_bytes: 0,
            uploaded_bytes_total: 0,
            delete_files_on_remove: false,
            save_path: "/fixture".into(),
            download_path: None,
            source_url: None,
            source_feed_id: links.feed,
            source_indexer_id: None,
            library_id: links.library,
            identification_kind: None,
            identification_status: None,
            identification_summary: None,
            post_process_status: None,
            post_process_error: None,
            processed_at: None,
            episode_id: links.episode,
            movie_id: links.movie,
            track_id: links.track,
            chapter_id: links.chapter,
            show_id: links.show,
            album_id: links.album,
            audiobook_id: links.audiobook,
            minimum_ratio: None,
            minimum_seed_time_minutes: None,
            season: None,
            excluded_files: vec![],
            added_at: AppMutationHook::now_rfc3339(),
            completed_at: None,
        },
    )
    .await
    .unwrap()
}

async fn set_state(db: &Database, torrent: &Torrent, state: &str) {
    Torrent::update_by_id(
        db,
        &torrent.id,
        UpdateTorrentInput {
            state: Some(state.into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
}

/// A torrent change reaches each linked parent exactly once, and the 10-second runtime sync
/// that produces those changes writes no parent row.
#[tokio::test]
async fn torrent_sync_writes_no_parent_rows_and_each_parent_hears_once() {
    let db = database(true).await;
    let (owner, lib, performer, record) = music_fixture(&db).await;
    let song = track(&db, &record, None).await;
    Track::update_by_id(
        &db,
        &song.id,
        UpdateTrackInput {
            artist_id: Some(Some(performer.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let series = show(&db, &lib).await;
    let ep = episode(&db, &series, None).await;
    let film = movie(&db, &lib).await;
    let book = audiobook(&db, &lib).await;
    let part = chapter(&db, &book, None).await;
    let row = torrent(
        &db,
        &owner,
        Links {
            library: Some(lib.id.clone()),
            movie: Some(film.id.clone()),
            show: Some(series.id.clone()),
            episode: Some(ep.id.clone()),
            track: Some(song.id.clone()),
            chapter: Some(part.id.clone()),
            album: Some(record.id.clone()),
            audiobook: Some(book.id.clone()),
            feed: None,
        },
    )
    .await;
    let mut movies = tap!(db, MovieChangedEvent);
    let mut shows = tap!(db, ShowChangedEvent);
    let mut episodes = tap!(db, EpisodeChangedEvent);
    let mut tracks = tap!(db, TrackChangedEvent);
    let mut chapters = tap!(db, ChapterChangedEvent);
    let mut albums = tap!(db, AlbumChangedEvent);
    let mut audiobooks = tap!(db, AudiobookChangedEvent);
    let mut artists = tap!(db, ArtistChangedEvent);
    let mut libraries = tap!(db, LibraryChangedEvent);
    let movie_before = Movie::get(db.pool(), &film.id).await.unwrap().unwrap();

    for state in ["seeding", "downloading", "seeding"] {
        set_state(&db, &row, state).await;
    }

    let thrice = |id: &str| vec![event(id, Some("Torrent")); 3];
    assert_eq!(drain!(movies), thrice(&film.id));
    assert_eq!(drain!(shows), thrice(&series.id));
    assert_eq!(drain!(episodes), thrice(&ep.id));
    assert_eq!(drain!(tracks), thrice(&song.id));
    assert_eq!(drain!(chapters), thrice(&part.id));
    assert_eq!(drain!(albums), thrice(&record.id));
    assert_eq!(drain!(audiobooks), thrice(&book.id));
    assert_eq!(drain!(artists), thrice(&performer.id));
    // The library is reachable by a dozen routes and still hears each change once.
    assert_eq!(drain!(libraries), thrice(&lib.id));
    let movie_after = Movie::get(db.pool(), &film.id).await.unwrap().unwrap();
    assert_eq!(movie_before.updated_at, movie_after.updated_at);
}

/// The same change through the generated `movieChanged` subscription: authorized, once, and
/// not delivered to another member.
#[tokio::test]
async fn movie_stream_hears_file_torrent_cast_changes_once_and_deletes_directly() {
    let db = database(true).await;
    let owner = user(&db).await;
    let outsider = user(&db).await;
    let lib = library(&db, &owner).await;
    let film = movie(&db, &lib).await;
    let other = movie(&db, &lib).await;
    let file = media_file(&db, &lib, 10, None, None).await;
    MediaFile::update_by_id(
        &db,
        &file.id,
        UpdateMediaFileInput {
            movie_id: Some(Some(film.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let row = torrent(
        &db,
        &owner,
        Links {
            movie: Some(film.id.clone()),
            ..Default::default()
        },
    )
    .await;
    let schema =
        async_graphql::Schema::build(Query, async_graphql::EmptyMutation, MovieSubscriptions)
            .data(db.clone())
            .finish();
    let request = |user: &User| {
        async_graphql::Request::new(
            "subscription { movieChanged { id action changeKind sourceEntity sourceId } }",
        )
        .data(user.id.clone())
        .data(AuthUser {
            user_id: user.id.clone(),
            email: None,
            role: Some("member".into()),
        })
    };
    let mut owned = Box::pin(schema.execute_stream(request(&owner)));
    let mut foreign = Box::pin(schema.execute_stream(request(&outsider)));
    use futures::FutureExt;
    assert!(owned.next().now_or_never().is_none());
    assert!(foreign.next().now_or_never().is_none());

    async fn heard<S>(stream: &mut S) -> Vec<(String, Option<String>)>
    where
        S: futures::Stream<Item = async_graphql::Response> + Unpin,
    {
        let mut out = Vec::new();
        while let Ok(Some(response)) =
            tokio::time::timeout(Duration::from_millis(150), stream.next()).await
        {
            assert!(response.errors.is_empty(), "{:?}", response.errors);
            let event = &response.data.into_json().unwrap()["movieChanged"];
            out.push((
                event["id"].as_str().unwrap().to_string(),
                event["sourceEntity"].as_str().map(str::to_string),
            ));
        }
        out.sort();
        out
    }

    // A file created already linked to a movie reaches it by propagation too.
    let created = MediaFile::insert(
        &db,
        CreateMediaFileInput {
            library_id: Some(lib.id.clone()),
            episode_id: None,
            movie_id: Some(other.id.clone()),
            track_id: None,
            chapter_id: None,
            path: format!("/fixture/{}.bin", uuid::Uuid::new_v4()),
            relative_path: None,
            original_name: None,
            size: 1,
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
            added_at: AppMutationHook::now_rfc3339(),
            analyzed_at: None,
            file_modified_at: None,
            match_type: None,
            matched_by_user_id: None,
            match_confirmed_at: None,
            quality_status: None,
            picture_crop: None,
            storyboard: None,
            previews_checked_at: None,
        },
    )
    .await
    .unwrap();
    let after_create = heard(&mut owned).await;
    assert!(
        after_create.contains(&event(&other.id, Some("MediaFile"))),
        "{after_create:?}"
    );
    MediaFile::delete_by_id(&db, &created.id).await.unwrap();
    heard(&mut owned).await;

    // File analysis, quality and size changes reach the movie once, from the file.
    MediaFile::update_by_id(
        &db,
        &file.id,
        UpdateMediaFileInput {
            size: Some(99),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        heard(&mut owned).await,
        vec![event(&film.id, Some("MediaFile"))]
    );
    // So does a torrent state change.
    set_state(&db, &row, "seeding").await;
    assert_eq!(
        heard(&mut owned).await,
        vec![event(&film.id, Some("Torrent"))]
    );
    // A deleted torrent has no surviving source: the movie is written directly.
    Torrent::delete_by_id(&db, &row.id).await.unwrap();
    let after_delete = heard(&mut owned).await;
    assert!(!after_delete.is_empty());
    assert!(
        after_delete
            .iter()
            .all(|(id, source)| id == &film.id && source.is_none()),
        "{after_delete:?}"
    );
    // Deleting a person deletes its credits through the entity layer, which tells the movie.
    let cast = person(&db).await;
    movie_cast_credit(&db, &film, &cast).await;
    heard(&mut owned).await;
    Person::delete_by_id(&db, &cast.id).await.unwrap();
    assert!(
        MovieCastCredit::query(db.pool())
            .filter(MovieCastCreditWhereInput {
                person_id: Some(AppMutationHook::string_eq(&cast.id)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap()
            .is_empty()
    );
    let after_person = heard(&mut owned).await;
    assert!(
        after_person
            .iter()
            .any(|(id, source)| id == &film.id && source.is_none()),
        "deleting a person must invalidate the movies it was credited on: {after_person:?}"
    );
    // A deleted file still reaches the movie, directly.
    MediaFile::delete_by_id(&db, &file.id).await.unwrap();
    let after_file = heard(&mut owned).await;
    assert!(
        after_file
            .iter()
            .any(|(id, source)| id == &film.id && source.is_none()),
        "{after_file:?}"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), foreign.next())
            .await
            .is_err(),
        "another member must hear nothing"
    );
}

#[derive(Default)]
struct Query;
#[async_graphql::Object]
impl Query {
    async fn ready(&self) -> bool {
        true
    }
}

/// Moves and deletes have no propagation, so they keep writing the surviving parents.
#[tokio::test]
async fn moving_and_deleting_children_still_reaches_the_old_parent() {
    let db = database(true).await;
    let (owner, lib, performer, first) = music_fixture(&db).await;
    let other = artist(&db, &lib).await;
    let second = album(&db, &lib, &other).await;
    let song = track(&db, &first, None).await;
    let mut albums = tap!(db, AlbumChangedEvent);
    let mut artists = tap!(db, ArtistChangedEvent);
    let _ = (&owner, &performer);

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
    let moved = drain!(albums);
    assert!(moved.contains(&event(&first.id, None)), "{moved:?}");
    assert!(moved.contains(&event(&second.id, None)), "{moved:?}");

    Track::update_by_id(
        &db,
        &song.id,
        UpdateTrackInput {
            artist_id: Some(Some(performer.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    drain!(artists);
    Track::update_by_id(
        &db,
        &song.id,
        UpdateTrackInput {
            artist_id: Some(Some(other.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let recredited = drain!(artists);
    assert!(
        recredited.contains(&event(&performer.id, None)),
        "{recredited:?}"
    );
    Track::delete_by_id(&db, &song.id).await.unwrap();
    let deleted = drain!(artists);
    assert!(deleted.contains(&event(&other.id, None)), "{deleted:?}");
}

async fn rss_feed(db: &Database, owner: &User, library: &Library) -> RssFeed {
    RssFeed::insert(
        db,
        CreateRssFeedInput {
            user_id: owner.id.clone(),
            library_id: Some(library.id.clone()),
            name: "fixture feed".into(),
            url: "https://fixture.invalid/feed".into(),
            enabled: true,
            poll_interval_minutes: 30,
            post_download_action: None,
            last_polled_at: None,
            last_successful_at: None,
            last_error: None,
            consecutive_failures: None,
            url_display: None,
            health_status: None,
            health_kind: None,
        },
    )
    .await
    .unwrap()
}

/// Every dangling-id site: deleting the target clears the id and keeps the row.
#[tokio::test]
async fn deleting_a_target_clears_the_ids_that_pointed_at_it() {
    let db = database(true).await;
    let (owner, lib, _, record) = music_fixture(&db).await;
    let song = track(&db, &record, None).await;
    let series = show(&db, &lib).await;
    let ep = episode(&db, &series, None).await;
    let film = movie(&db, &lib).await;
    let book = audiobook(&db, &lib).await;
    let part = chapter(&db, &book, None).await;
    let feed = rss_feed(&db, &owner, &lib).await;
    let file = media_file(&db, &lib, 5, None, None).await;
    let row = torrent(
        &db,
        &owner,
        Links {
            library: Some(lib.id.clone()),
            movie: Some(film.id.clone()),
            show: Some(series.id.clone()),
            episode: Some(ep.id.clone()),
            track: Some(song.id.clone()),
            chapter: Some(part.id.clone()),
            album: Some(record.id.clone()),
            audiobook: Some(book.id.clone()),
            feed: None,
        },
    )
    .await;
    let from_feed = torrent(
        &db,
        &owner,
        Links {
            feed: Some(feed.id.clone()),
            ..Default::default()
        },
    )
    .await;
    let now = AppMutationHook::now_rfc3339();
    let session = PlaybackSession::insert(
        &db,
        CreatePlaybackSessionInput {
            user_id: owner.id.clone(),
            content_type: None,
            media_file_id: Some(file.id.clone()),
            episode_id: Some(ep.id.clone()),
            movie_id: Some(film.id.clone()),
            track_id: Some(song.id.clone()),
            audiobook_id: Some(book.id.clone()),
            tv_show_id: Some(series.id.clone()),
            album_id: Some(record.id.clone()),
            current_position: 0.0,
            duration: None,
            volume: 1.0,
            is_muted: false,
            is_playing: false,
            started_at: now.clone(),
            last_updated_at: now.clone(),
            completed_at: None,
        },
    )
    .await
    .unwrap();
    let usenet = UsenetDownload::insert(
        &db,
        CreateUsenetDownloadInput {
            user_id: owner.id.clone(),
            nzb_name: "fixture.nzb".into(),
            nzb_hash: None,
            nzb_url: None,
            nzb_data: None,
            state: "queued".into(),
            progress: None,
            size_bytes: None,
            downloaded_bytes: None,
            download_speed: None,
            eta_seconds: None,
            error_message: None,
            retry_count: 0,
            download_path: None,
            library_id: Some(lib.id.clone()),
            episode_id: Some(ep.id.clone()),
            movie_id: Some(film.id.clone()),
            album_id: Some(record.id.clone()),
            audiobook_id: Some(book.id.clone()),
            indexer_id: None,
            post_process_status: None,
            completed_at: None,
        },
    )
    .await
    .unwrap();
    let blocked = ReleaseBlocklist::insert(
        &db,
        CreateReleaseBlocklistInput {
            info_hash: Some("fixture".into()),
            guid: None,
            title: "blocked".into(),
            source_id: None,
            reason: "stalled".into(),
            show_id: Some(series.id.clone()),
            movie_id: Some(film.id.clone()),
            album_id: Some(record.id.clone()),
            audiobook_id: Some(book.id.clone()),
            expires_at: None,
        },
    )
    .await
    .unwrap();
    let notification = Notification::insert(
        &db,
        CreateNotificationInput {
            user_id: owner.id.clone(),
            notification_type: "INFO".into(),
            category: "TEST".into(),
            title: "fixture".into(),
            message: "fixture".into(),
            library_id: Some(lib.id.clone()),
            torrent_id: Some(row.id.clone()),
            media_file_id: Some(file.id.clone()),
            pending_match_id: None,
            action_type: None,
            action_data: None,
            read_at: None,
            resolved_at: None,
            resolution: None,
        },
    )
    .await
    .unwrap();
    let torrent_file = TorrentFile::insert(
        &db,
        CreateTorrentFileInput {
            torrent_id: row.id.clone(),
            file_index: 0,
            file_path: "/fixture/a.bin".into(),
            relative_path: "a.bin".into(),
            file_size: 5,
            downloaded_bytes: 5,
            progress: 1.0,
            media_file_id: Some(file.id.clone()),
            is_excluded: false,
        },
    )
    .await
    .unwrap();
    let item = RssFeedItem::insert(
        &db,
        CreateRssFeedItemInput {
            feed_id: feed.id.clone(),
            guid: None,
            link_hash: "link".into(),
            title_hash: "title".into(),
            title: "item".into(),
            link: "https://fixture.invalid/item".into(),
            pub_date: None,
            description: None,
            parsed_show_name: None,
            parsed_season: None,
            parsed_episode: None,
            parsed_resolution: None,
            parsed_codec: None,
            parsed_source: None,
            parsed_audio: None,
            parsed_hdr: None,
            processed: false,
            torrent_id: Some(row.id.clone()),
            skipped_reason: None,
            seen_at: now,
            info_hash: None,
            size: None,
            seeders: None,
            category: None,
            parsed_scope: None,
            parsed_year: None,
            parsed_season_end: None,
            parsed_group: None,
            parsed_artist: None,
            parsed_album: None,
            parsed_format: None,
            parsed_bitrate: None,
            match_kind: None,
            match_show_id: None,
            match_movie_id: None,
            match_album_id: None,
            match_audiobook_id: None,
            match_season: None,
            match_episode: None,
            match_label: None,
            match_confidence: None,
            match_reason: None,
            matched_at: None,
        },
    )
    .await
    .unwrap();

    macro_rules! gone {
        ($ty:ident, $id:expr, $($field:ident),+) => {{
            let row = $ty::get(db.pool(), &$id).await.unwrap().unwrap();
            $( assert_eq!(row.$field, None, concat!(stringify!($ty), ".", stringify!($field))); )+
        }};
    }
    macro_rules! kept {
        ($ty:ident, $id:expr, $($field:ident),+) => {{
            let row = $ty::get(db.pool(), &$id).await.unwrap().unwrap();
            $( assert!(row.$field.is_some(), concat!(stringify!($ty), ".", stringify!($field), " must survive")); )+
        }};
    }

    Movie::delete_by_id(&db, &film.id).await.unwrap();
    gone!(Torrent, row.id, movie_id);
    gone!(PlaybackSession, session.id, movie_id);
    gone!(UsenetDownload, usenet.id, movie_id);
    gone!(ReleaseBlocklist, blocked.id, movie_id);
    kept!(
        Torrent,
        row.id,
        show_id,
        episode_id,
        track_id,
        chapter_id,
        album_id,
        audiobook_id
    );
    kept!(
        PlaybackSession,
        session.id,
        episode_id,
        tv_show_id,
        album_id
    );

    Episode::delete_by_id(&db, &ep.id).await.unwrap();
    gone!(Torrent, row.id, episode_id);
    gone!(PlaybackSession, session.id, episode_id);
    gone!(UsenetDownload, usenet.id, episode_id);

    Show::delete_by_id(&db, &series.id).await.unwrap();
    gone!(Torrent, row.id, show_id);
    gone!(PlaybackSession, session.id, tv_show_id);
    gone!(ReleaseBlocklist, blocked.id, show_id);

    Track::delete_by_id(&db, &song.id).await.unwrap();
    gone!(Torrent, row.id, track_id);
    gone!(PlaybackSession, session.id, track_id);

    Chapter::delete_by_id(&db, &part.id).await.unwrap();
    gone!(Torrent, row.id, chapter_id);

    Album::delete_by_id(&db, &record.id).await.unwrap();
    gone!(Torrent, row.id, album_id);
    gone!(PlaybackSession, session.id, album_id);
    gone!(UsenetDownload, usenet.id, album_id);
    gone!(ReleaseBlocklist, blocked.id, album_id);

    Audiobook::delete_by_id(&db, &book.id).await.unwrap();
    gone!(Torrent, row.id, audiobook_id);
    gone!(PlaybackSession, session.id, audiobook_id);
    gone!(UsenetDownload, usenet.id, audiobook_id);
    gone!(ReleaseBlocklist, blocked.id, audiobook_id);

    MediaFile::delete_by_id(&db, &file.id).await.unwrap();
    gone!(PlaybackSession, session.id, media_file_id);
    gone!(Notification, notification.id, media_file_id);
    gone!(TorrentFile, torrent_file.id, media_file_id);

    Torrent::delete_by_id(&db, &row.id).await.unwrap();
    gone!(Notification, notification.id, torrent_id);
    gone!(RssFeedItem, item.id, torrent_id);

    // The feed delete removes its items; a torrent grabbed from it only loses the link.
    RssFeed::delete_by_id(&db, &feed.id).await.unwrap();
    gone!(Torrent, from_feed.id, source_feed_id);
    assert!(
        RssFeedItem::get(db.pool(), &item.id)
            .await
            .unwrap()
            .is_none()
    );
    // History survives: every row above is still there, only without its dead id.
    assert!(
        UsenetDownload::get(db.pool(), &usenet.id)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn deleting_a_pending_match_clears_the_notification_link() {
    let db = database(true).await;
    let (owner, _, _, record) = music_fixture(&db).await;
    let song = track(&db, &record, None).await;
    let row = pending(&db, &owner, &song).await;
    let notification = Notification::insert(
        &db,
        CreateNotificationInput {
            user_id: owner.id.clone(),
            notification_type: "INFO".into(),
            category: "TEST".into(),
            title: "fixture".into(),
            message: "fixture".into(),
            library_id: None,
            torrent_id: None,
            media_file_id: None,
            pending_match_id: Some(row.id.clone()),
            action_type: None,
            action_data: None,
            read_at: None,
            resolved_at: None,
            resolution: None,
        },
    )
    .await
    .unwrap();
    PendingFileMatch::delete_by_id(&db, &row.id).await.unwrap();
    assert_eq!(
        Notification::get(db.pool(), &notification.id)
            .await
            .unwrap()
            .unwrap()
            .pending_match_id,
        None
    );
}

/// `Album.artist` says `restrict`; nothing but this hook enforces it.
#[tokio::test]
async fn artist_with_albums_cannot_be_deleted_and_credits_are_cleared() {
    let db = database(true).await;
    let (_, lib, owner_of_album, record) = music_fixture(&db).await;
    let guest = artist(&db, &lib).await;
    let song = track(&db, &record, None).await;
    Track::update_by_id(
        &db,
        &song.id,
        UpdateTrackInput {
            artist_id: Some(Some(guest.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let refused = Artist::delete_by_id(&db, &owner_of_album.id).await;
    assert!(
        refused.is_err(),
        "an artist with albums must not be deleted"
    );
    assert!(
        Artist::get(db.pool(), &owner_of_album.id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(Album::get(db.pool(), &record.id).await.unwrap().is_some());

    // A credit on someone else's track is only a nullable link: it is cleared.
    Artist::delete_by_id(&db, &guest.id).await.unwrap();
    let kept = Track::get(db.pool(), &song.id).await.unwrap().unwrap();
    assert_eq!(kept.artist_id, None);

    // A library delete removes albums first, so it is not blocked by the guard.
    Library::delete_by_id(&db, &lib.id).await.unwrap();
    assert!(
        Artist::get(db.pool(), &owner_of_album.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn deleting_a_quality_profile_clears_the_profile_assigned_to_titles() {
    let db = database(true).await;
    let (_, lib, _, record) = music_fixture(&db).await;
    let film = movie(&db, &lib).await;
    let series = show(&db, &lib).await;
    let book = audiobook(&db, &lib).await;
    let profile = QualityProfile::insert(
        &db,
        CreateQualityProfileInput {
            name: "fixture".into(),
            media_kind: MediaKind::Video,
            allowed_resolutions: vec![],
            allowed_video_codecs: vec![],
            allowed_audio_formats: vec![],
            allowed_hdr_types: vec![],
            allowed_sources: vec![],
            release_group_blacklist: vec![],
            release_group_whitelist: vec![],
            require_hdr: false,
            preferred_languages: vec![],
            require_language_match: false,
            min_size_mb: None,
            max_size_mb: None,
            min_seeders: 1,
            max_release_age_days: None,
            preferred_release_groups: vec![],
            allow_season_packs: true,
            prefer_proper_repack: true,
            resolution_preference: vec![],
            cutoff_resolution: None,
            upgrade_until_cutoff: false,
            is_default: false,
        },
    )
    .await
    .unwrap();
    let id = Some(Some(profile.id.clone()));
    Library::update_by_id(
        &db,
        &lib.id,
        UpdateLibraryInput {
            quality_profile_id: id.clone(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    Movie::update_by_id(
        &db,
        &film.id,
        UpdateMovieInput {
            quality_profile_id: id.clone(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    Show::update_by_id(
        &db,
        &series.id,
        UpdateShowInput {
            quality_profile_id: id.clone(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    Album::update_by_id(
        &db,
        &record.id,
        UpdateAlbumInput {
            quality_profile_id: id.clone(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    Audiobook::update_by_id(
        &db,
        &book.id,
        UpdateAudiobookInput {
            quality_profile_id: id.clone(),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    QualityProfile::delete_by_id(&db, &profile.id)
        .await
        .unwrap();

    assert_eq!(
        Library::get(db.pool(), &lib.id)
            .await
            .unwrap()
            .unwrap()
            .quality_profile_id,
        None
    );
    assert_eq!(
        Movie::get(db.pool(), &film.id)
            .await
            .unwrap()
            .unwrap()
            .quality_profile_id,
        None
    );
    assert_eq!(
        Show::get(db.pool(), &series.id)
            .await
            .unwrap()
            .unwrap()
            .quality_profile_id,
        None
    );
    assert_eq!(
        Album::get(db.pool(), &record.id)
            .await
            .unwrap()
            .unwrap()
            .quality_profile_id,
        None
    );
    assert_eq!(
        Audiobook::get(db.pool(), &book.id)
            .await
            .unwrap()
            .unwrap()
            .quality_profile_id,
        None
    );
}

/// Deletes that the hooks run per row, matching the declared `on_delete` of the relations that
/// say `cascade` although no physical foreign key backs them.
#[tokio::test]
async fn library_delete_removes_files_feeds_notifications_and_playback_progress() {
    let db = database(true).await;
    let (owner, lib, _, _) = music_fixture(&db).await;
    let file = media_file(&db, &lib, 5, None, None).await;
    let feed = rss_feed(&db, &owner, &lib).await;
    let notification = Notification::insert(
        &db,
        CreateNotificationInput {
            user_id: owner.id.clone(),
            notification_type: "INFO".into(),
            category: "TEST".into(),
            title: "fixture".into(),
            message: "fixture".into(),
            library_id: Some(lib.id.clone()),
            torrent_id: None,
            media_file_id: None,
            pending_match_id: None,
            action_type: None,
            action_data: None,
            read_at: None,
            resolved_at: None,
            resolution: None,
        },
    )
    .await
    .unwrap();
    let progress = PlaybackProgress::insert(
        &db,
        CreatePlaybackProgressInput {
            user_id: owner.id.clone(),
            media_file_id: Some(file.id.clone()),
            current_position: 1.0,
            duration: None,
            progress_percent: 0.0,
            is_watched: false,
            watched_at: None,
        },
    )
    .await
    .unwrap();

    MediaFile::delete_by_id(&db, &file.id).await.unwrap();
    assert!(
        PlaybackProgress::get(db.pool(), &progress.id)
            .await
            .unwrap()
            .is_none()
    );
    Library::delete_by_id(&db, &lib.id).await.unwrap();
    assert!(RssFeed::get(db.pool(), &feed.id).await.unwrap().is_none());
    assert!(
        Notification::get(db.pool(), &notification.id)
            .await
            .unwrap()
            .is_none()
    );
}

/// Every `emit_fk = false` relation's `on_delete` says what the hooks really do. These are
/// the declarations that used to contradict them.
#[test]
fn declared_delete_policies_match_what_the_hooks_do() {
    use graphql_orm::graphql::orm::{DeletePolicy, Entity};
    let policy = |entity: &'static graphql_orm::graphql::orm::EntityMetadata, field: &str| {
        let relation = entity
            .relations
            .iter()
            .find(|relation| relation.field_name == field)
            .unwrap_or_else(|| panic!("{}.{field}", entity.entity_name));
        assert!(
            !relation.emit_foreign_key,
            "{}.{field} gained a physical key",
            entity.entity_name
        );
        relation.on_delete.clone()
    };
    // The library delete removes these rows; it does not null them.
    assert_eq!(
        policy(<MediaFile as Entity>::metadata(), "library"),
        DeletePolicy::Cascade
    );
    assert_eq!(
        policy(<RssFeed as Entity>::metadata(), "library"),
        DeletePolicy::Cascade
    );
    assert_eq!(
        policy(<Notification as Entity>::metadata(), "library"),
        DeletePolicy::Cascade
    );
    assert_eq!(
        policy(<PlaybackProgress as Entity>::metadata(), "mediaFile"),
        DeletePolicy::Cascade
    );
    // Enforced by the Artist delete guard.
    assert_eq!(
        policy(<Album as Entity>::metadata(), "artist"),
        DeletePolicy::Restrict
    );
    // Cleared by `detach_deleted_references` and `cleanup_media_file_dependents`.
    assert_eq!(
        policy(<Torrent as Entity>::metadata(), "track"),
        DeletePolicy::SetNull
    );
    assert_eq!(
        policy(<Notification as Entity>::metadata(), "torrent"),
        DeletePolicy::SetNull
    );
    assert_eq!(
        policy(<TorrentFile as Entity>::metadata(), "mediaFile"),
        DeletePolicy::SetNull
    );
    assert_eq!(
        policy(<RssFeedItem as Entity>::metadata(), "torrent"),
        DeletePolicy::SetNull
    );
}

/// Relations are not indexed automatically: the ORM indexes only what is declared. Every
/// column a relation joins on, or a hook filters by when it clears or deletes children, must
/// lead an index, or each parent delete scans the child table.
#[test]
fn every_relation_join_column_leads_an_index() {
    use graphql_orm::graphql::orm::SchemaModel;
    use std::collections::BTreeMap;
    let entities = entity_metadata();
    let model = SchemaModel::from_entities(&entities);
    let tables: BTreeMap<_, _> = model
        .tables
        .iter()
        .map(|table| (table.entity_name.clone(), table))
        .collect();
    let mut gaps = Vec::new();
    for entity in &entities {
        for relation in entity.relations.iter() {
            let (child, columns) = if relation.is_multiple {
                (relation.target_type, relation.target_columns)
            } else {
                (entity.entity_name, relation.source_columns)
            };
            let table = tables[child];
            let first = columns[0];
            let indexed = table.indexes.iter().any(|index| {
                index.columns.first().map(|c| c.to_string()).as_deref() == Some(first)
            }) || table.primary_keys().first().map(String::as_str) == Some(first)
                || table
                    .composite_unique_indexes
                    .iter()
                    .any(|columns| columns.first().map(String::as_str) == Some(first));
            if !indexed {
                gaps.push(format!(
                    "{}.{} -> {}.{first}",
                    entity.entity_name, relation.field_name, table.table_name
                ));
            }
        }
    }
    gaps.sort();
    gaps.dedup();
    assert!(gaps.is_empty(), "unindexed relation columns: {gaps:#?}");
}

/// Production startup raises the per-entity channel capacity before anything subscribes
/// (`from_config` calls this outside unit tests, which keep the ORM default).
#[tokio::test]
async fn startup_raises_the_event_channel_capacity_for_every_clone() {
    let db = Database::new(DbPool::connect("sqlite::memory:").await.unwrap());
    assert_eq!(
        db.event_channel_capacity(),
        graphql_orm::db::DEFAULT_EVENT_CHANNEL_CAPACITY
    );
    let clone = db.clone();
    apply_event_channel_capacity(&db);
    assert_eq!(clone.event_channel_capacity(), EVENT_CHANNEL_CAPACITY);
}

/// The capacity above is a memory decision: one pre-allocated slot per event, per channel.
#[test]
fn event_channel_slot_size_bounds_memory() {
    let slot = std::mem::size_of::<SubscriptionEvent<MediaFileChangedEvent>>();
    let per_channel = slot * EVENT_CHANNEL_CAPACITY;
    eprintln!(
        "event slot {slot} bytes; {per_channel} bytes per channel at {EVENT_CHANNEL_CAPACITY} slots"
    );
    assert!(slot <= 256, "an event slot grew to {slot} bytes");
    // Ten times the events one busy title writes (about a thousand).
    const { assert!(EVENT_CHANNEL_CAPACITY >= 10_000) };
    assert!(per_channel <= 4 * 1024 * 1024);
}

/// New relations resolve through the generated schema, including the composite and
/// conditional ones.
#[tokio::test]
async fn new_relations_resolve_through_the_generated_schema() {
    let db = database(true).await;
    let (owner, lib, _, record) = music_fixture(&db).await;
    let song = track(&db, &record, None).await;
    let series = show(&db, &lib).await;
    let ep = episode(&db, &series, None).await;
    let film = movie(&db, &lib).await;
    Movie::update_by_id(
        &db,
        &film.id,
        UpdateMovieInput {
            collection_id: Some(Some(77)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let collection = super::parent_summary_tests::collection(&db, &lib, 77).await;
    let cast = person(&db).await;
    let credit = movie_cast_credit(&db, &film, &cast).await;
    let source = source_torrent(&db, &owner).await;
    let row = pending(&db, &owner, &song).await;
    PendingFileMatch::update_by_id(
        &db,
        &row.id,
        UpdatePendingFileMatchInput {
            source_type: Some("torrent".into()),
            source_id: Some(Some(source.info_hash.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let ignored = pending(&db, &owner, &song).await;
    let feed = rss_feed(&db, &owner, &lib).await;
    Torrent::update_by_id(
        &db,
        &source.id,
        UpdateTorrentInput {
            source_feed_id: Some(Some(feed.id.clone())),
            track_id: Some(Some(song.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let season = Season::insert(
        &db,
        CreateSeasonInput {
            locked_fields: None,
            show_id: series.id.clone(),
            season: ep.season,
            tvmaze_id: None,
            name: None,
            overview: None,
            premiere_date: None,
            end_date: None,
            episode_order: None,
            poster_url: None,
        },
    )
    .await
    .unwrap();

    let schema = crate::graphql::build_schema(db.clone(), (), Arc::new(ServicesManager::new()));
    let response = schema
        .execute(member_request(
            format!(
                "query {{
                    movie(id: {film:?}) {{ collection {{ id }} castCredits {{ edges {{ node {{ id }} }} }} }}
                    person(id: {cast:?}) {{ castCredits {{ edges {{ node {{ id }} }} }} }}
                    season(id: {season:?}) {{ episodes {{ edges {{ node {{ id }} }} }} }}
                    torrent(id: {torrent:?}) {{ track {{ id }} sourceFeed {{ id }}
                        pendingMatches {{ edges {{ node {{ id }} }} }} }}
                }}",
                film = film.id,
                cast = cast.id,
                season = season.id,
                torrent = source.id,
            ),
            &owner,
            "admin",
        ))
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap();
    assert_eq!(data["movie"]["collection"]["id"], collection.id);
    assert_eq!(
        data["person"]["castCredits"]["edges"][0]["node"]["id"],
        credit.id
    );
    assert_eq!(data["season"]["episodes"]["edges"][0]["node"]["id"], ep.id);
    assert_eq!(data["torrent"]["track"]["id"], song.id);
    assert_eq!(data["torrent"]["sourceFeed"]["id"], feed.id);
    let matches = data["torrent"]["pendingMatches"]["edges"]
        .as_array()
        .unwrap();
    assert_eq!(
        matches.len(),
        1,
        "only the torrent-sourced match: {matches:?}"
    );
    assert_eq!(matches[0]["node"]["id"], row.id);
    assert_ne!(matches[0]["node"]["id"], ignored.id);
}
