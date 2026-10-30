//! Transactional canonical Artist summaries and nested-only parent invalidations.
//! Source-only additions: runtime validation is owned by the parent.
use super::*;
use graphql_orm::DbPool;
use graphql_orm::graphql::errors::{OrmErrorCode, OrmPublicError};
use graphql_orm::graphql::subscriptions::SubscriptionEvent;

async fn assert_artist(db: &Database, id: &str, albums: i32, tracks: i32, duration: i32) {
    let row = Artist::get(db.pool(), &id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (row.album_count, row.track_count, row.total_duration_secs),
        (Some(albums), Some(tracks), Some(duration))
    );
}

async fn credited_track(db: &Database, record: &Album, performer: &Artist, duration: i32) -> Track {
    let row = track(db, record, None).await;
    Track::update_by_id(
        db,
        &row.id,
        UpdateTrackInput {
            artist_id: Some(Some(performer.id.clone())),
            duration_secs: Some(Some(duration)),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap()
}

#[tokio::test]
async fn artist_summaries_follow_explicit_membership_and_old_new_parents() {
    let db = database(true).await;
    let owner = user(&db).await;
    let lib = library(&db, &owner).await;
    let first = artist(&db, &lib).await;
    let second = artist(&db, &lib).await;
    let record = album(&db, &lib, &first).await;
    let uncredited = track(&db, &record, None).await;
    assert_artist(&db, &first.id, 1, 0, 0).await;
    let song = credited_track(&db, &record, &first, 13).await;
    assert_artist(&db, &first.id, 1, 1, 13).await;
    Track::update_by_id(
        &db,
        &song.id,
        UpdateTrackInput {
            artist_id: Some(Some(second.id.clone())),
            duration_secs: Some(Some(21)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_artist(&db, &first.id, 1, 0, 0).await;
    assert_artist(&db, &second.id, 0, 1, 21).await;
    Album::update_by_id(
        &db,
        &record.id,
        UpdateAlbumInput {
            artist_id: Some(second.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_artist(&db, &first.id, 0, 0, 0).await;
    assert_artist(&db, &second.id, 1, 1, 21).await;
    Track::delete_by_id(&db, &song.id).await.unwrap();
    assert_artist(&db, &second.id, 1, 0, 0).await;
    Album::delete_by_id(&db, &record.id).await.unwrap();
    assert!(
        Track::get(db.pool(), &uncredited.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_artist(&db, &second.id, 0, 0, 0).await;
}

#[tokio::test]
async fn artist_backfill_pages_past_100_and_excludes_cross_library_and_uncredited_tracks() {
    let raw = database(false).await;
    let owner = user(&raw).await;
    let lib = library(&raw, &owner).await;
    let foreign_lib = library(&raw, &owner).await;
    let performer = artist(&raw, &lib).await;
    let record = album(&raw, &lib, &performer).await;
    for _ in 0..105 {
        album(&raw, &lib, &performer).await;
        credited_track(&raw, &record, &performer, 2).await;
    }
    track(&raw, &record, None).await; // No implicit album-artist membership.
    let foreign = album(&raw, &foreign_lib, &performer).await;
    credited_track(&raw, &foreign, &performer, 999).await;
    let mut db = DatabaseService::new(raw).pool().clone();
    db.set_pagination_config(PaginationConfig::explicit_only(17));
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<ArtistChangedEvent>>()
        .subscribe();
    let preview = reconcile_stored_media_summaries(&db, true).await.unwrap();
    assert_eq!((preview.artists_examined, preview.artists_changed), (1, 1));
    assert_eq!(
        Artist::get(db.pool(), &performer.id)
            .await
            .unwrap()
            .unwrap()
            .album_count,
        None
    );
    assert!(events.try_recv().is_err());
    let applied = reconcile_stored_media_summaries(&db, false).await.unwrap();
    assert_eq!((applied.artists_examined, applied.artists_changed), (1, 1));
    assert_artist(&db, &performer.id, 106, 105, 210).await;
    assert_eq!(
        reconcile_stored_media_summaries(&db, false)
            .await
            .unwrap()
            .artists_changed,
        0
    );
    // Moving the Artist itself changes the membership scope without moving rows.
    Artist::update_by_id(
        &db,
        &performer.id,
        UpdateArtistInput {
            library_id: Some(foreign_lib.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_artist(&db, &performer.id, 1, 1, 999).await;
}

#[tokio::test]
async fn artist_reparent_summaries_and_events_rollback_with_child_write() {
    let db = database(true).await;
    let owner = user(&db).await;
    let lib = library(&db, &owner).await;
    let first = artist(&db, &lib).await;
    let second = artist(&db, &lib).await;
    let record = album(&db, &lib, &first).await;
    let song = credited_track(&db, &record, &first, 8).await;
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<ArtistChangedEvent>>()
        .subscribe();
    let (song_id, first_id, second_id) = (song.id.clone(), first.id.clone(), second.id.clone());
    let failed = db
        .transaction(TransactionMode::StateMachine, move |tx| {
            Box::pin(async move {
                tx.update_by_id::<Track>(
                    &song_id,
                    UpdateTrackInput {
                        artist_id: Some(Some(second_id.clone())),
                        ..Default::default()
                    },
                )
                .await?;
                assert_eq!(
                    tx.find_by_id::<Artist>(&first_id)
                        .await?
                        .unwrap()
                        .track_count,
                    Some(0)
                );
                assert_eq!(
                    tx.find_by_id::<Artist>(&second_id)
                        .await?
                        .unwrap()
                        .track_count,
                    Some(1)
                );
                Err::<(), _>(OrmPublicError::new(OrmErrorCode::Conflict))
            })
        })
        .await;
    assert!(failed.is_err());
    assert_artist(&db, &first.id, 1, 1, 8).await;
    assert_artist(&db, &second.id, 0, 0, 0).await;
    assert_eq!(
        Track::get(db.pool(), &song.id)
            .await
            .unwrap()
            .unwrap()
            .artist_id,
        Some(first.id)
    );
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn artist_duration_overflow_rejects_the_whole_reconciliation() {
    let raw = database(false).await;
    let owner = user(&raw).await;
    let lib = library(&raw, &owner).await;
    let good = artist(&raw, &lib).await;
    let bad = artist(&raw, &lib).await;
    let record = album(&raw, &lib, &bad).await;
    credited_track(&raw, &record, &bad, i32::MAX).await;
    credited_track(&raw, &record, &bad, 1).await;
    let db = DatabaseService::new(raw).pool().clone();
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<ArtistChangedEvent>>()
        .subscribe();
    assert!(reconcile_stored_media_summaries(&db, false).await.is_err());
    for performer in [good, bad] {
        let row = Artist::get(db.pool(), &performer.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (row.album_count, row.track_count, row.total_duration_secs),
            (None, None, None)
        );
    }
    assert!(events.try_recv().is_err());
}

#[derive(Default)]
struct ParentQuery;
#[async_graphql::Object]
impl ParentQuery {
    async fn ready(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn unchanged_artist_totals_still_deliver_metadata_invalidation_only_to_owner() {
    use futures::{FutureExt, StreamExt};
    let db = database(true).await;
    let owner = user(&db).await;
    let outsider = user(&db).await;
    let lib = library(&db, &owner).await;
    let performer = artist(&db, &lib).await;
    let record = album(&db, &lib, &performer).await;
    let song = credited_track(&db, &record, &performer, 8).await;
    let schema = async_graphql::Schema::build(
        ParentQuery,
        async_graphql::EmptyMutation,
        ArtistSubscriptions,
    )
    .data(db.clone())
    .finish();
    let request = |user: &User| {
        async_graphql::Request::new(
        "subscription { artistChanged { sourceEntity artist { id albumCount trackCount totalDurationSecs } } }"
    ).data(user.id.clone()).data(AuthUser {
        user_id: user.id.clone(), email: None, role: Some("member".into())
    })
    };
    let mut owned = Box::pin(schema.execute_stream(request(&owner)));
    let mut foreign = Box::pin(schema.execute_stream(request(&outsider)));
    assert!(owned.next().now_or_never().is_none());
    assert!(foreign.next().now_or_never().is_none());
    Track::update_by_id(
        &db,
        &song.id,
        UpdateTrackInput {
            title: Some("changed metadata, identical totals".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let response = tokio::time::timeout(Duration::from_secs(2), owned.next())
        .await
        .unwrap()
        .unwrap();
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let payload = response.data.into_json().unwrap();
    assert_eq!(payload["artistChanged"]["artist"]["id"], performer.id);
    assert_eq!(payload["artistChanged"]["artist"]["trackCount"], 1);
    assert_eq!(
        payload["artistChanged"]["sourceEntity"],
        serde_json::Value::Null
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(40), foreign.next())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn metadata_only_artist_events_leave_legacy_totals_for_explicit_repair() {
    let raw = database(false).await;
    let owner = user(&raw).await;
    let lib = library(&raw, &owner).await;
    let performer = artist(&raw, &lib).await;
    let record = album(&raw, &lib, &performer).await;
    let song = credited_track(&raw, &record, &performer, 8).await;
    // A legacy stored summary distinguishes timestamp invalidation from repair
    // without relying on SQL text or an observer that misses transaction reads.
    Artist::update_by_id(
        &raw,
        &performer.id,
        UpdateArtistInput {
            album_count: Some(Some(99)),
            track_count: Some(Some(99)),
            total_duration_secs: Some(Some(999)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let db = DatabaseService::new(raw).pool().clone();
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<ArtistChangedEvent>>()
        .subscribe();
    Track::update_by_id(
        &db,
        &song.id,
        UpdateTrackInput {
            title: Some("metadata-only title".into()),
            wanted: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_artist(&db, &performer.id, 99, 99, 999).await;
    assert!(
        std::iter::from_fn(|| events.try_recv().ok())
            .any(|change| change.event.id == performer.id && change.event.source_entity.is_none())
    );
    Album::update_by_id(
        &db,
        &record.id,
        UpdateAlbumInput {
            name: Some("metadata-only album".into()),
            has_files: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_artist(&db, &performer.id, 99, 99, 999).await;
    assert!(
        std::iter::from_fn(|| events.try_recv().ok())
            .any(|change| change.event.id == performer.id && change.event.source_entity.is_none())
    );
    let report = reconcile_stored_media_summaries(&db, false).await.unwrap();
    assert_eq!(report.artists_changed, 1);
    assert_artist(&db, &performer.id, 1, 1, 8).await;
    // Duration changes alone must still repair, even with identical membership.
    Track::update_by_id(
        &db,
        &song.id,
        UpdateTrackInput {
            duration_secs: Some(Some(13)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_artist(&db, &performer.id, 1, 1, 13).await;
    Track::update_by_id(
        &db,
        &song.id,
        UpdateTrackInput {
            duration_secs: Some(None),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_artist(&db, &performer.id, 1, 1, 0).await;
}

async fn pending(db: &Database, owner: &User, song: &Track) -> PendingFileMatch {
    PendingFileMatch::insert(
        db,
        CreatePendingFileMatchInput {
            user_id: owner.id.clone(),
            source_path: "/fixture/pending.flac".into(),
            source_type: "manual".into(),
            source_id: None,
            source_file_index: None,
            file_size: 5,
            episode_id: None,
            movie_id: None,
            track_id: Some(song.id.clone()),
            chapter_id: None,
            unmatched_reason: None,
            match_type: None,
            match_confidence: None,
            match_attempts: 0,
            verification_status: None,
            verification_reason: None,
            parsed_resolution: None,
            parsed_codec: None,
            parsed_source: None,
            parsed_audio: None,
            copied_at: None,
            copy_error: None,
            copy_attempts: 0,
        },
    )
    .await
    .unwrap()
}

async fn source_torrent(db: &Database, owner: &User) -> Torrent {
    Torrent::insert(
        db,
        CreateTorrentInput {
            user_id: owner.id.clone(),
            info_hash: format!("{:040x}", uuid::Uuid::new_v4().as_u128()),
            magnet_uri: None,
            name: "pending source fixture".into(),
            state: "downloading".into(),
            progress: 0.0,
            total_bytes: 5,
            downloaded_bytes: 0,
            uploaded_bytes: 0,
            uploaded_bytes_total: 0,
            save_path: "/fixture".into(),
            download_path: None,
            source_url: None,
            source_feed_id: None,
            source_indexer_id: None,
            library_id: None,
            post_process_status: None,
            post_process_error: None,
            processed_at: None,
            episode_id: None,
            movie_id: None,
            track_id: None,
            chapter_id: None,
            show_id: None,
            album_id: None,
            audiobook_id: None,
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

fn member_request(document: String, owner: &User, role: &str) -> async_graphql::Request {
    async_graphql::Request::new(document)
        .data(owner.id.clone())
        .data(AuthUser {
            user_id: owner.id.clone(),
            email: None,
            role: Some(role.into()),
        })
}

#[tokio::test]
async fn pending_torrent_hash_resolves_actual_pk_and_invalidates_old_new_owned_sources() {
    let db = database(true).await;
    let owner = user(&db).await;
    let lib = library(&db, &owner).await;
    let performer = artist(&db, &lib).await;
    let record = album(&db, &lib, &performer).await;
    let song = track(&db, &record, None).await;
    let pending = pending(&db, &owner, &song).await;
    let first = source_torrent(&db, &owner).await;
    let second = source_torrent(&db, &owner).await;
    let schema = crate::graphql::build_schema(db.clone(), (), Arc::new(ServicesManager::new()));
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<TorrentChangedEvent>>()
        .subscribe();
    let mut direct = || {
        std::iter::from_fn(|| events.try_recv().ok())
            .filter(|change| change.event.source_entity.is_none())
            .map(|change| change.event.id.clone())
            .collect::<BTreeSet<_>>()
    };
    for (source, expected) in [
        (&first, BTreeSet::from([first.id.clone()])),
        (
            &second,
            BTreeSet::from([first.id.clone(), second.id.clone()]),
        ),
    ] {
        let response = schema.execute(member_request(format!(
            "mutation {{ updatePendingFileMatch(id: {:?}, input: {{ sourceType: \"torrent\", sourceId: {:?} }}) {{ success }} }}",
            pending.id, source.info_hash
        ), &owner, "member")).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        assert_eq!(
            response.data.into_json().unwrap()["updatePendingFileMatch"]["success"],
            true
        );
        assert_eq!(direct(), expected);
        let response = schema.execute(member_request(format!(
            "query {{ pendingFileMatch(id: {:?}) {{ sourceId torrent {{ id infoHash }} }} }}", pending.id
        ), &owner, "member")).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let value = response.data.into_json().unwrap();
        assert_eq!(value["pendingFileMatch"]["sourceId"], source.info_hash);
        assert_eq!(value["pendingFileMatch"]["torrent"]["id"], source.id);
    }
    let response = schema.execute(member_request(format!(
        "mutation {{ updatePendingFileMatch(id: {:?}, input: {{ sourceType: \"manual\" }}) {{ success }} }}", pending.id
    ), &owner, "member")).await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert_eq!(direct(), BTreeSet::from([second.id.clone()]));
    let response = schema
        .execute(member_request(
            format!(
                "query {{ pendingFileMatch(id: {:?}) {{ torrent {{ id }} }} }}",
                pending.id
            ),
            &owner,
            "admin",
        ))
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    assert!(response.data.into_json().unwrap()["pendingFileMatch"]["torrent"].is_null());
}

#[tokio::test]
async fn pending_foreign_uuid_or_hash_cannot_write_or_navigate_foreign_torrent_even_as_admin() {
    let db = database(true).await;
    let owner = user(&db).await;
    let foreign_owner = user(&db).await;
    let lib = library(&db, &owner).await;
    let performer = artist(&db, &lib).await;
    let record = album(&db, &lib, &performer).await;
    let song = track(&db, &record, None).await;
    let pending = pending(&db, &owner, &song).await;
    let foreign = source_torrent(&db, &foreign_owner).await;
    let schema = crate::graphql::build_schema(db.clone(), (), Arc::new(ServicesManager::new()));
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<TorrentChangedEvent>>()
        .subscribe();
    for forged in [&foreign.id, &foreign.info_hash] {
        let response = schema.execute(member_request(format!(
            "mutation {{ updatePendingFileMatch(id: {:?}, input: {{ sourceType: \"torrent\", sourceId: {:?}, verificationStatus: \"matched\" }}) {{ success }} }}",
            pending.id, forged
        ), &owner, "member")).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        assert_eq!(
            response.data.into_json().unwrap()["updatePendingFileMatch"]["success"],
            true
        );
        assert!(events.try_recv().is_err());
        assert_eq!(
            Torrent::get(db.pool(), &foreign.id)
                .await
                .unwrap()
                .unwrap()
                .updated_at,
            foreign.updated_at
        );
        // Admin may read both rows, but the pending row still cannot manufacture
        // source provenance across owners through a scalar hash/UUID relation.
        let response = schema
            .execute(member_request(
                format!(
                    "query {{ pendingFileMatch(id: {:?}) {{ torrent {{ id }} }} }}",
                    pending.id
                ),
                &owner,
                "admin",
            ))
            .await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        assert!(response.data.into_json().unwrap()["pendingFileMatch"]["torrent"].is_null());
    }
}

#[test]
fn pending_torrent_relation_metadata_qualifies_hash_by_owner_without_physical_fk() {
    let relation = PendingFileMatch::relation_metadata()
        .iter()
        .find(|relation| relation.field_name == "torrent")
        .unwrap();
    assert_eq!(relation.source_columns, ["source_id", "user_id"]);
    assert_eq!(relation.target_columns, ["info_hash", "user_id"]);
    assert!(!relation.emit_foreign_key);
}

fn direct_tracks(
    events: &mut tokio::sync::broadcast::Receiver<SubscriptionEvent<TrackChangedEvent>>,
) -> BTreeSet<String> {
    std::iter::from_fn(|| events.try_recv().ok())
        .filter(|change| change.event.source_entity.is_none())
        .map(|change| change.event.id.clone())
        .collect()
}

#[tokio::test]
async fn torrent_lifecycle_invalidates_old_new_typed_targets_without_changing_summaries() {
    let db = database(true).await;
    let owner = user(&db).await;
    let lib = library(&db, &owner).await;
    let performer = artist(&db, &lib).await;
    let record = album(&db, &lib, &performer).await;
    let first = credited_track(&db, &record, &performer, 5).await;
    let second = credited_track(&db, &record, &performer, 6).await;
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<TrackChangedEvent>>()
        .subscribe();
    let torrent = Torrent::insert(
        &db,
        CreateTorrentInput {
            user_id: owner.id.clone(),
            info_hash: uuid::Uuid::new_v4().to_string(),
            magnet_uri: None,
            name: "fixture torrent".into(),
            state: "downloading".into(),
            progress: 0.0,
            total_bytes: 5,
            downloaded_bytes: 0,
            uploaded_bytes: 0,
            uploaded_bytes_total: 0,
            save_path: "/fixture".into(),
            download_path: None,
            source_url: None,
            source_feed_id: None,
            source_indexer_id: None,
            library_id: Some(lib.id.clone()),
            post_process_status: None,
            post_process_error: None,
            processed_at: None,
            episode_id: None,
            movie_id: None,
            track_id: Some(first.id.clone()),
            chapter_id: None,
            show_id: None,
            album_id: None,
            audiobook_id: None,
            minimum_ratio: None,
            minimum_seed_time_minutes: None,
            season: None,
            excluded_files: vec![],
            added_at: AppMutationHook::now_rfc3339(),
            completed_at: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        direct_tracks(&mut events),
        BTreeSet::from([first.id.clone()])
    );
    Torrent::update_by_id(
        &db,
        &torrent.id,
        UpdateTorrentInput {
            state: Some("seeding".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        direct_tracks(&mut events),
        BTreeSet::from([first.id.clone()])
    );
    Torrent::update_by_id(
        &db,
        &torrent.id,
        UpdateTorrentInput {
            track_id: Some(Some(second.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        direct_tracks(&mut events),
        BTreeSet::from([first.id, second.id.clone()])
    );
    Torrent::delete_by_id(&db, &torrent.id).await.unwrap();
    assert_eq!(direct_tracks(&mut events), BTreeSet::from([second.id]));
    assert_artist(&db, &performer.id, 1, 2, 11).await;
}

#[tokio::test]
async fn rss_library_status_reparent_and_delete_invalidate_surviving_libraries() {
    let db = database(true).await;
    let owner = user(&db).await;
    let first = library(&db, &owner).await;
    let second = library(&db, &owner).await;
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<LibraryChangedEvent>>()
        .subscribe();
    let mut direct = || {
        std::iter::from_fn(|| events.try_recv().ok())
            .filter(|change| change.event.source_entity.is_none())
            .map(|change| change.event.id.clone())
            .collect::<BTreeSet<_>>()
    };
    let feed = RssFeed::insert(
        &db,
        CreateRssFeedInput {
            user_id: owner.id.clone(),
            library_id: Some(first.id.clone()),
            name: "fixture feed".into(),
            url: "https://fixture.invalid/feed".into(),
            enabled: true,
            poll_interval_minutes: 30,
            post_download_action: None,
            last_polled_at: None,
            last_successful_at: None,
            last_error: None,
            consecutive_failures: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(direct(), BTreeSet::from([first.id.clone()]));
    RssFeed::update_by_id(
        &db,
        &feed.id,
        UpdateRssFeedInput {
            last_error: Some(Some("fixture failure".into())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(direct(), BTreeSet::from([first.id.clone()]));
    RssFeed::update_by_id(
        &db,
        &feed.id,
        UpdateRssFeedInput {
            library_id: Some(Some(second.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(direct(), BTreeSet::from([first.id, second.id.clone()]));
    RssFeed::delete_by_id(&db, &feed.id).await.unwrap();
    assert_eq!(direct(), BTreeSet::from([second.id]));
}

async fn collection(db: &Database, lib: &Library, key: i32) -> Collection {
    Collection::insert(
        db,
        CreateCollectionInput {
            library_id: lib.id.clone(),
            user_id: lib.user_id.clone(),
            tmdb_collection_id: key,
            name: "fixture collection".into(),
            overview: None,
            poster_url: None,
            backdrop_url: None,
            movie_count: 17,
            last_synced_at: None,
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn movie_collection_invalidation_uses_library_and_provider_key_and_old_new_memberships() {
    let db = database(true).await;
    let owner = user(&db).await;
    let lib = library(&db, &owner).await;
    let other_lib = library(&db, &owner).await;
    let first = collection(&db, &lib, 71).await;
    let second = collection(&db, &lib, 72).await;
    let foreign = collection(&db, &other_lib, 71).await;
    let row = movie(&db, &lib).await;
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<CollectionChangedEvent>>()
        .subscribe();
    let mut direct = || {
        std::iter::from_fn(|| events.try_recv().ok())
            .filter(|change| change.event.source_entity.is_none())
            .map(|change| change.event.id.clone())
            .collect::<BTreeSet<_>>()
    };
    Movie::update_by_id(
        &db,
        &row.id,
        UpdateMovieInput {
            collection_id: Some(Some(71)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(direct(), BTreeSet::from([first.id.clone()]));
    Movie::update_by_id(
        &db,
        &row.id,
        UpdateMovieInput {
            title: Some("changed metadata".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(direct(), BTreeSet::from([first.id.clone()]));
    Movie::update_by_id(
        &db,
        &row.id,
        UpdateMovieInput {
            collection_id: Some(Some(72)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(direct(), BTreeSet::from([first.id, second.id.clone()]));
    Movie::delete_by_id(&db, &row.id).await.unwrap();
    assert_eq!(direct(), BTreeSet::from([second.id]));
    // Catalog size is provider metadata, not a local-membership summary.
    assert_eq!(
        Collection::get(db.pool(), &foreign.id)
            .await
            .unwrap()
            .unwrap()
            .movie_count,
        17
    );
}

#[tokio::test]
async fn pending_status_retarget_and_delete_invalidate_only_typed_old_new_targets() {
    let db = database(true).await;
    let owner = user(&db).await;
    let lib = library(&db, &owner).await;
    let performer = artist(&db, &lib).await;
    let record = album(&db, &lib, &performer).await;
    let first = track(&db, &record, None).await;
    let second = track(&db, &record, None).await;
    let mut events = db
        .ensure_event_sender::<SubscriptionEvent<TrackChangedEvent>>()
        .subscribe();
    let row = pending(&db, &owner, &first).await;
    assert_eq!(
        direct_tracks(&mut events),
        BTreeSet::from([first.id.clone()])
    );
    PendingFileMatch::update_by_id(
        &db,
        &row.id,
        UpdatePendingFileMatchInput {
            verification_status: Some(Some("verified".into())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        direct_tracks(&mut events),
        BTreeSet::from([first.id.clone()])
    );
    PendingFileMatch::update_by_id(
        &db,
        &row.id,
        UpdatePendingFileMatchInput {
            track_id: Some(Some(second.id.clone())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        direct_tracks(&mut events),
        BTreeSet::from([first.id, second.id.clone()])
    );
    PendingFileMatch::delete_by_id(&db, &row.id).await.unwrap();
    assert_eq!(direct_tracks(&mut events), BTreeSet::from([second.id]));
}

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
