//! Which album a set of music files is, and which of its tracks each file holds.
//!
//! Files are worked out together. One at a time, a file can only guess from its own name, and the
//! same album gets found over and over. Rules are in `music_rules`; both library scans and
//! imports from downloads come through here, so they agree.

use super::*;
use crate::graphql::entities::{
    Album, AlbumWhereInput, Artist, ArtistWhereInput, Track, TrackWhereInput,
};
use crate::services::metadata::musicbrainz::MusicBrainzTrack;
use crate::services::metadata::providers::{
    AddAlbumOptions, NoCreditedArtist, listing_track_facts,
};
use crate::services::music_rules::{self, ListingFit};
use crate::services::release_match::{ReleaseFile, TrackFacts};

/// How many search results get their editions read. Each is several requests to a slow service.
const CANDIDATES_READ: usize = 3;
/// With no artist to go on, this share of the files has to fit before a listing counts.
const FIT_WITHOUT_ARTIST: f64 = 0.6;

/// What a file's own tags and path say about its album, when nothing better is known.
fn hint_from_path(source: &str) -> (Option<String>, Option<String>) {
    let hint = LibraryScanService::parse_track_hint(source);
    let artist = hint
        .artist_name
        .filter(|artist| !music_rules::same_artist(artist, "Unknown Artist"));
    (
        hint.album_name
            .map(|album| music_rules::album_title(&album)),
        artist,
    )
}

fn track_facts(track: &Track) -> TrackFacts {
    TrackFacts {
        title: track.title.clone(),
        number: Some(track.track_number),
        disc: track.disc_number,
        seconds: track.duration_secs.map(f64::from),
    }
}

impl LibraryScanService {
    /// Link a scan's unmatched music files, a release at a time. Albums are looked up one after
    /// another, because the provider is rate limited and each album is only worth finding once.
    pub(super) async fn link_music_files(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        pending: &[PendingProviderFallback],
    ) -> ProviderFallbackSummary {
        let mut summary = ProviderFallbackSummary::default();
        let mut sizes = Vec::with_capacity(pending.len());
        for item in pending {
            let size = tokio::fs::metadata(&item.media_path)
                .await
                .map_or(0, |metadata| metadata.len() as i64);
            sizes.push((item.media_path.clone(), item.media_path.clone(), size, true));
        }
        // Read from the files rather than stored analysis, which may not have run yet.
        let files = self.release_file_facts(sizes).await;
        for group in music_rules::group_release_files(&files) {
            let mut members: Vec<ReleaseFile> = group
                .files
                .iter()
                .map(|index| files[*index].clone())
                .collect();
            let sources: Vec<&PendingProviderFallback> =
                group.files.iter().map(|index| &pending[*index]).collect();
            music_rules::fill_in_discs(&mut members);
            let names: Vec<Option<String>> = sources
                .iter()
                .map(|item| Self::parse_track_hint(&item.match_source).title)
                .collect();
            let (folder_title, folder_artist) = hint_from_path(&sources[0].match_source);
            let title = group.title.clone().or(folder_title);
            let artist = group.artist.clone().or(folder_artist);
            let placed = self
                .place_music_group(
                    library,
                    auth_user,
                    title.as_deref(),
                    artist.as_deref(),
                    &members,
                    &names,
                )
                .await;
            let placed = match placed {
                Ok(placed) => placed,
                Err(error) => {
                    summary.unmatched_files += members.len();
                    summary.error_files += members.len();
                    warn!(
                        library_id = %library.id,
                        album = title.as_deref().unwrap_or("unknown"),
                        error = %error,
                        "Could not work out a music release: {}",
                        error
                    );
                    continue;
                }
            };
            // Tracks a file of this release took just now: their rows were read before it did.
            let mut taken_now: HashSet<String> = HashSet::new();
            for (item, track) in sources.iter().zip(placed) {
                let Some(track) = track else {
                    summary.unmatched_files += 1;
                    continue;
                };
                if track
                    .media_file_id
                    .as_deref()
                    .is_some_and(|held| held != item.media_file_id)
                    || !taken_now.insert(track.id.clone())
                {
                    // Another copy of this track already holds it. Which to keep is a person's call.
                    summary.unmatched_files += 1;
                    debug!(
                        media_file_id = %item.media_file_id,
                        track_id = %track.id,
                        "Left a second copy of a track unlinked"
                    );
                    continue;
                }
                if let Err(error) = self
                    .link_track(
                        auth_user,
                        &item.media_file_id,
                        &library.id,
                        &track.id,
                        "auto",
                        None,
                    )
                    .await
                {
                    summary.unmatched_files += 1;
                    summary.error_files += 1;
                    warn!(media_file_id = %item.media_file_id, error = %error, "Could not link a track: {}", error);
                    continue;
                }
                summary.matched_files += 1;
                let result = MatchResult {
                    success: true,
                    auto_matched: true,
                    already_matched: false,
                    matched_type: Some("Track".to_string()),
                    matched_id: Some(track.id.clone()),
                    confidence: 1.0,
                    reason: Some(format!(
                        "Part of the album '{}'",
                        title.as_deref().unwrap_or("unknown")
                    )),
                    candidates: Vec::new(),
                };
                self.maybe_auto_organize_after_scan_match(
                    library,
                    auth_user,
                    &item.media_file_id,
                    &item.media_path,
                    &result,
                )
                .await;
            }
        }
        summary
    }

    /// Link one music file on its own, when someone asks for just that one.
    pub(super) async fn link_one_music_file(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        media_file_id: &str,
        media_path: &str,
        match_source: &str,
    ) -> Result<Option<MatchResult>> {
        let size = tokio::fs::metadata(media_path)
            .await
            .map_or(0, |metadata| metadata.len() as i64);
        let mut members = self
            .release_file_facts(vec![(
                media_path.to_string(),
                media_path.to_string(),
                size,
                true,
            )])
            .await;
        music_rules::fill_in_discs(&mut members);
        let names = vec![Self::parse_track_hint(match_source).title];
        let (folder_title, folder_artist) = hint_from_path(match_source);
        let tags = members
            .first()
            .map(|file| file.tags.clone())
            .unwrap_or_default();
        let title = tags
            .album
            .as_deref()
            .filter(|album| !album.trim().is_empty())
            .map(music_rules::album_title)
            .or(folder_title);
        let artist = tags.album_artist.or(tags.artist).or(folder_artist);
        let placed = self
            .place_music_group(
                library,
                auth_user,
                title.as_deref(),
                artist.as_deref(),
                &members,
                &names,
            )
            .await?;
        let Some(track) = placed.into_iter().next().flatten() else {
            return Ok(None);
        };
        self.link_track(
            auth_user,
            media_file_id,
            &library.id,
            &track.id,
            "auto",
            None,
        )
        .await?;
        Ok(Some(MatchResult {
            success: true,
            auto_matched: true,
            already_matched: false,
            matched_type: Some("Track".to_string()),
            matched_id: Some(track.id),
            confidence: 0.9,
            reason: Some("Found its album and placed it on a track".to_string()),
            candidates: Vec::new(),
        }))
    }

    /// The track each file belongs on, finding or making the album first.
    async fn place_music_group(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        title: Option<&str>,
        artist: Option<&str>,
        members: &[ReleaseFile],
        names: &[Option<String>],
    ) -> Result<Vec<Option<Track>>> {
        let Some(title) = title.filter(|title| !title.trim().is_empty()) else {
            return Ok(vec![None; members.len()]);
        };
        let album = self
            .album_for_files(library, auth_user, title, artist, members, names)
            .await?;
        self.place_on_album(auth_user, &album, members, names).await
    }

    /// The album these files are: one already here, then one a provider knows, then one made from
    /// their tags. A provider that couldn't be reached fails this, so a later scan can try again
    /// rather than settling for a made-up album.
    async fn album_for_files(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        title: &str,
        artist: Option<&str>,
        members: &[ReleaseFile],
        names: &[Option<String>],
    ) -> Result<Album> {
        let database = self.database().await?;
        let pool = database.pool().pool();
        let albums = Album::query(pool)
            .filter(AlbumWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        for album in albums {
            if !music_rules::same_album_title(&album.name, title) {
                continue;
            }
            let album_artist = Artist::get(pool, &album.artist_id)
                .await?
                .map(|row| row.name);
            let same = match (artist, album_artist.as_deref()) {
                (Some(a), Some(b)) => music_rules::same_artist(a, b),
                _ => true,
            };
            if same {
                return Ok(album);
            }
        }

        let metadata = self.metadata_service().await?;
        let query = artist.map_or_else(|| title.to_string(), |artist| format!("{title} {artist}"));
        let results = metadata
            .search_albums(&query, true, true, true, true, true)
            .await?;
        let candidates: Vec<_> = results
            .into_iter()
            .filter(|result| {
                music_rules::same_album_title(&result.title, title)
                    && match (artist, result.artist_name.as_deref()) {
                        (Some(a), Some(b)) => music_rules::same_artist(a, b),
                        _ => true,
                    }
            })
            .take(CANDIDATES_READ)
            .collect();
        let mut best: Option<(f64, String, Option<Vec<MusicBrainzTrack>>)> = None;
        for (position, candidate) in candidates.into_iter().enumerate() {
            let Ok(id) = Uuid::parse_str(&candidate.provider_id) else {
                continue;
            };
            let listing: Option<(Vec<MusicBrainzTrack>, ListingFit)> = metadata
                .album_listing_for_files(id, members, names, 0)
                .await?;
            let share = listing.as_ref().map_or(0.0, |(_, fit)| fit.share);
            if artist.is_none() && share < FIT_WITHOUT_ARTIST {
                continue;
            }
            // Fit decides. A studio album beats a single of the same name, then search order.
            let is_album = candidate
                .album_type
                .as_deref()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("album"));
            let score = share + if is_album { 0.1 } else { 0.0 } - position as f64 * 0.01;
            if best.as_ref().is_none_or(|(current, ..)| score > *current) {
                best = Some((
                    score,
                    candidate.provider_id,
                    listing.map(|(tracks, _)| tracks),
                ));
            }
        }
        if let Some((_, provider_id, listing)) = best {
            match metadata
                .add_album_from_provider(AddAlbumOptions {
                    provider: MetadataProvider::Musicbrainz,
                    provider_id,
                    library_id: Uuid::parse_str(&library.id)?,
                    user_id: Uuid::parse_str(&library.user_id)?,
                    monitor_type: AutoDownloadMode::None,
                    listing,
                })
                .await
            {
                Ok(album) => return Ok(album),
                // No credited artist means no artist folder: the files' own tags name it instead.
                Err(error) if error.downcast_ref::<NoCreditedArtist>().is_some() => warn!(
                    library_id = %library.id,
                    album = %title,
                    error = %error,
                    "Using the tags of '{}' because the provider credits it to no artist: {}",
                    title,
                    error
                ),
                Err(error) => return Err(error),
            }
        }
        self.album_from_tags(library, auth_user, title, artist, members)
            .await
    }

    /// An album no provider knows, from what its files say.
    async fn album_from_tags(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        title: &str,
        artist: Option<&str>,
        members: &[ReleaseFile],
    ) -> Result<Album> {
        let database = self.database().await?;
        let pool = database.pool().pool();
        let artist_name = artist.unwrap_or("Various Artists");
        let artists = Artist::query(pool)
            .filter(ArtistWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        let artist_id = match artists
            .into_iter()
            .find(|row| music_rules::same_artist(&row.name, artist_name))
        {
            Some(row) => row.id,
            None => {
                let data = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation ArtistFromTags($input: CreateArtistInput!) {
                            CreateArtist: createArtist(input: $input) { Success: success Error: error Artist: artist { Id: id } }
                        }"#,
                        serde_json::json!({ "input": {
                            "libraryId": library.id,
                            "userId": library.user_id,
                            "name": artist_name,
                            "sortName": artist_name,
                        }}),
                    )
                    .await?;
                data.pointer("/CreateArtist/Artist/Id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                    .ok_or_else(|| anyhow::anyhow!("Could not add the artist {artist_name}"))?
            }
        };
        let year = members
            .iter()
            .find_map(|file| file.tags.date.as_deref())
            .and_then(|date| date.get(..4))
            .and_then(|year| year.parse::<i32>().ok());
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation AlbumFromTags($input: CreateAlbumInput!) {
                    CreateAlbum: createAlbum(input: $input) { Success: success Error: error Album: album { Id: id } }
                }"#,
                serde_json::json!({ "input": {
                    "artistId": artist_id,
                    "libraryId": library.id,
                    "userId": library.user_id,
                    "name": title,
                    "sortName": title,
                    "year": year,
                    "albumType": "album",
                    "genres": Vec::<String>::new(),
                    "autoDownload": false,
                    "autoDownloadMode": "NONE",
                    "hasFiles": false,
                }}),
            )
            .await?;
        let id = data
            .pointer("/CreateAlbum/Album/Id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("Could not add the album {title}"))?;
        Album::get(pool, &id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Album not found after creating it"))
    }

    async fn album_track_rows(&self, album_id: &str) -> Result<Vec<Track>> {
        let database = self.database().await?;
        Ok(Track::query(database.pool().pool())
            .filter(TrackWhereInput {
                album_id: Some(StringFilter {
                    eq: Some(album_id.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?)
    }

    /// The track each file belongs on. An album grows to hold them: when files don't fit its
    /// tracks, the edition they do fit adds the tracks it's missing, and whatever no edition lists
    /// (bonus discs, a hidden track) gets a track of its own, so no file is left out or forced onto
    /// the wrong track.
    pub(super) async fn place_on_album(
        &self,
        auth_user: &AuthUser,
        album: &Album,
        members: &[ReleaseFile],
        names: &[Option<String>],
    ) -> Result<Vec<Option<Track>>> {
        let mut tracks = self.album_track_rows(&album.id).await?;
        let mut fit = music_rules::fit_listing(
            members,
            names,
            &tracks.iter().map(track_facts).collect::<Vec<_>>(),
        );
        let artist_name = {
            let database = self.database().await?;
            Artist::get(database.pool().pool(), &album.artist_id)
                .await?
                .map(|row| row.name)
        };
        if fit.share < 1.0
            && let Some(id) = album
                .musicbrainz_id
                .as_deref()
                .and_then(|id| Uuid::parse_str(id).ok())
        {
            let metadata = self.metadata_service().await?;
            if let Some((listing, _)) = metadata
                .album_listing_for_files(id, members, names, tracks.len())
                .await?
            {
                for listed in listing
                    .iter()
                    .map(|track| (track, listing_track_facts(track)))
                {
                    let (track, facts) = listed;
                    let wanted = release_match::normalize_words(&facts.title);
                    let present = tracks.iter().any(|row| {
                        let disc = row.disc_number.unwrap_or(1);
                        disc == track.disc_number
                            && (row.track_number == track.position
                                || release_match::normalize_words(&row.title) == wanted)
                    });
                    if !present {
                        self.add_album_track(
                            auth_user,
                            album,
                            artist_name.as_deref(),
                            track.disc_number,
                            track.position,
                            &track.title,
                            track.duration_secs,
                            track.recording_id.as_deref(),
                        )
                        .await?;
                    }
                }
                tracks = self.album_track_rows(&album.id).await?;
                fit = music_rules::fit_listing(
                    members,
                    names,
                    &tracks.iter().map(track_facts).collect::<Vec<_>>(),
                );
            }
        }
        // A file left over that has a track's title is that track: on a disc its tags don't name,
        // or a second copy of a song another file already took. It never gets a track of its own.
        let copies: Vec<Option<String>> = members
            .iter()
            .enumerate()
            .map(|(index, file)| {
                if fit.placed[index].is_some() {
                    return None;
                }
                let title = file.tags.title.as_deref().or(names[index].as_deref())?;
                let facts: Vec<TrackFacts> = tracks.iter().map(track_facts).collect();
                music_rules::track_with_title(title, file.duration, &facts)
                    .map(|found| tracks[found].id.clone())
            })
            .collect();
        if fit.share < 1.0 {
            let mut taken: Vec<(i32, i32)> = tracks
                .iter()
                .map(|row| (row.disc_number.unwrap_or(1), row.track_number))
                .collect();
            for (index, file) in members.iter().enumerate() {
                if fit.placed[index].is_some() || copies[index].is_some() {
                    continue;
                }
                let disc = file.tags.disc.unwrap_or(1);
                let number = music_rules::free_track_number(&taken, disc, file.tags.track);
                let title = file
                    .tags
                    .title
                    .clone()
                    .or_else(|| names[index].clone())
                    .unwrap_or_else(|| format!("Track {number}"));
                self.add_album_track(
                    auth_user,
                    album,
                    artist_name.as_deref(),
                    disc,
                    number,
                    &title,
                    file.duration.map(|seconds| seconds.round() as i32),
                    None,
                )
                .await?;
                taken.push((disc, number));
            }
            tracks = self.album_track_rows(&album.id).await?;
            fit = music_rules::fit_listing(
                members,
                names,
                &tracks.iter().map(track_facts).collect::<Vec<_>>(),
            );
        }
        self.update_album_totals(auth_user, album, &tracks).await?;
        Ok(fit
            .placed
            .iter()
            .zip(&copies)
            .map(|(place, copy)| match (place, copy) {
                (Some(index), _) => tracks.get(*index).cloned(),
                (None, Some(id)) => tracks.iter().find(|track| &track.id == id).cloned(),
                (None, None) => None,
            })
            .collect())
    }

    /// What an album's cards and identity show: how many tracks and discs, and how long.
    async fn update_album_totals(
        &self,
        auth_user: &AuthUser,
        album: &Album,
        tracks: &[Track],
    ) -> Result<()> {
        let count = tracks.len() as i32;
        let discs = tracks
            .iter()
            .map(|track| track.disc_number.unwrap_or(1))
            .max()
            .unwrap_or(1);
        let seconds: i32 = tracks.iter().filter_map(|track| track.duration_secs).sum();
        if album.track_count == Some(count)
            && album.disc_count == Some(discs)
            && album.total_duration_secs == Some(seconds)
        {
            return Ok(());
        }
        self.execute_mutation(
            auth_user,
            r#"mutation AlbumTotals($id: String!, $input: UpdateAlbumInput!) {
                UpdateAlbum: updateAlbum(id: $id, input: $input) { Success: success Error: error }
            }"#,
            serde_json::json!({ "id": album.id, "input": {
                "trackCount": count,
                "discCount": discs,
                "totalDurationSecs": seconds,
            }}),
        )
        .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn add_album_track(
        &self,
        auth_user: &AuthUser,
        album: &Album,
        artist_name: Option<&str>,
        disc: i32,
        number: i32,
        title: &str,
        seconds: Option<i32>,
        recording_id: Option<&str>,
    ) -> Result<()> {
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation AddAlbumTrack($input: CreateTrackInput!) {
                    CreateTrack: createTrack(input: $input) { Success: success Error: error }
                }"#,
                serde_json::json!({ "input": {
                    "albumId": album.id,
                    "libraryId": album.library_id,
                    "artistId": album.artist_id,
                    "artistName": artist_name,
                    "title": title,
                    "trackNumber": number,
                    "discNumber": disc,
                    "durationSecs": seconds,
                    "musicbrainzId": recording_id,
                    "explicit": false,
                    "wanted": false,
                }}),
            )
            .await?;
        if data
            .pointer("/CreateTrack/Success")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            anyhow::bail!(
                "Could not add the track {title}: {}",
                data.pointer("/CreateTrack/Error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown error")
            );
        }
        Ok(())
    }
}
