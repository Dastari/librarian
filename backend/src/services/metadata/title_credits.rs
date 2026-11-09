//! Fetching and keeping every credit and fact a provider has for a title (`docs/design.md`
//! Q129–Q132): movies, shows with their seasons and episodes, albums with their tracks, and
//! audiobooks, plus the people they name. Parsing is `credits`; saving is `credit_store`.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use anyhow::{Result, anyhow};
use graphql_orm::graphql::filters::{IntFilter, StringFilter};
use serde_json::Value;
use tracing::{debug, info, warn};

use super::MetadataService;
use crate::services::graphql::entities::{
    Album, AlbumWhereInput, Audiobook, AudiobookWhereInput, Credit, CreditWhereInput, Episode,
    EpisodeWhereInput, Movie, MovieCastCredit, MovieCastCreditWhereInput, MovieWhereInput, Person,
    PersonWhereInput, Season, SeasonWhereInput, Show, ShowWhereInput, TitleDetail,
    TitleDetailWhereInput, Track, TrackWhereInput,
};
use crate::services::metadata::credit_store::{CREDIT_LOCKS, CreditStore, TargetKind, TitleTarget};
use crate::services::metadata::credits::{
    CreditEntry, PersonRef, ProviderTitle, ReleaseCredits, TitleFacts, audiobook_credits,
    parse_tmdb_find_show, parse_tmdb_movie, parse_tmdb_person, parse_tmdb_season, parse_tmdb_show,
};

/// Everything a movie document is read with: one request per film.
pub const MOVIE_APPEND: &str =
    "credits,release_dates,keywords,alternative_titles,videos,external_ids";
const SHOW_APPEND: &str =
    "aggregate_credits,content_ratings,keywords,alternative_titles,videos,external_ids";
const PERSON_APPEND: &str = "external_ids,images";

/// Wakes the credits job early, when a title was added whose credits it should fetch.
static CREDIT_FILL: LazyLock<tokio::sync::Notify> = LazyLock::new(tokio::sync::Notify::new);

/// Ask the credits job for a pass soon (a title was just added).
pub fn request_credit_fill() {
    CREDIT_FILL.notify_one();
}

/// Resolves when [`request_credit_fill`] was called.
pub fn credit_fill_requested() -> tokio::sync::futures::Notified<'static> {
    CREDIT_FILL.notified()
}

/// Titles and people tried in this process, so one that fails is not retried every pass.
static ATTEMPTED: LazyLock<std::sync::Mutex<HashSet<String>>> = LazyLock::new(Default::default);

fn first_attempt(key: String) -> bool {
    ATTEMPTED
        .lock()
        .map(|mut set| set.insert(key))
        .unwrap_or(false)
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn credits_locked(locked: &[String]) -> bool {
    locked
        .iter()
        .any(|lock| CREDIT_LOCKS.contains(&lock.as_str()))
}

fn not_null() -> Option<StringFilter> {
    Some(StringFilter {
        is_null: Some(false),
        ..Default::default()
    })
}

fn string_eq(value: &str) -> Option<StringFilter> {
    Some(StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    })
}

/// How much one pass of the credits job may do. Each title is about one provider request
/// (a show is one plus one per season), each person one.
#[derive(Debug, Clone, Copy)]
pub struct CreditFillBudget {
    pub movies: usize,
    pub shows: usize,
    pub albums: usize,
    pub audiobooks: usize,
    pub people: usize,
}

impl Default for CreditFillBudget {
    fn default() -> Self {
        Self {
            movies: 30,
            shows: 3,
            albums: 4,
            audiobooks: 50,
            people: 120,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreditFillReport {
    pub movies: usize,
    pub shows: usize,
    pub albums: usize,
    pub audiobooks: usize,
    pub people: usize,
    /// Movies whose old cast (`MovieCastCredit`) was copied over because TMDB could not be read.
    pub carried_over: usize,
}

impl CreditFillReport {
    pub fn total(&self) -> usize {
        self.movies + self.shows + self.albums + self.audiobooks + self.people + self.carried_over
    }
}

impl MetadataService {
    async fn credit_store(&self) -> Result<CreditStore> {
        Ok(CreditStore::new(
            self.graphql_schema().await?,
            self.db.clone(),
        ))
    }

    /// Ids of the titles of one kind that already have details.
    async fn titles_with_details(&self, kind: TargetKind) -> Result<HashSet<String>> {
        let rows = TitleDetail::query(self.db.pool())
            .filter(TitleDetailWhereInput {
                target_kind: string_eq(kind.as_str()),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| match kind {
                TargetKind::Movie => row.movie_id,
                TargetKind::Show => row.show_id,
                TargetKind::Season => row.season_id,
                TargetKind::Episode => row.episode_id,
                TargetKind::Album => row.album_id,
                TargetKind::Track => row.track_id,
                TargetKind::Audiobook => row.audiobook_id,
            })
            .collect())
    }

    // -----------------------------------------------------------------------------------------
    // Movies

    /// Keep a movie's credits and details from the document its details were read from, or
    /// read it now when the details came from a cache older than that (best effort: a failure
    /// is logged and the credits job tries again).
    pub(super) async fn store_movie_extras(&self, movie: &Movie, extras: Option<&ProviderTitle>) {
        let result = match extras {
            Some(title) => self.save_movie_title(movie, title).await,
            None => self.sync_movie_credits(movie).await,
        };
        if let Err(error) = result {
            warn!(
                movie_id = %movie.id,
                title = %movie.title,
                error = %error,
                "Could not save credits and details of movie '{}': {error}",
                movie.title
            );
        }
    }

    async fn fetch_movie_document(&self, tmdb_id: i32) -> Result<Value> {
        let tmdb = self.get_tmdb_client().await?;
        tmdb.get_value(
            &format!("/movie/{tmdb_id}"),
            &[("append_to_response", MOVIE_APPEND)],
        )
        .await?
        .ok_or_else(|| anyhow!("Movie {tmdb_id} not found on TMDB"))
    }

    /// Read a movie's TMDB document and keep its credits and details.
    pub async fn sync_movie_credits(&self, movie: &Movie) -> Result<()> {
        let tmdb_id = movie
            .tmdb_id
            .ok_or_else(|| anyhow!("Movie '{}' has no TMDB id", movie.title))?;
        let document = self.fetch_movie_document(tmdb_id).await?;
        self.save_movie_title(movie, &parse_tmdb_movie(&document))
            .await
    }

    async fn save_movie_title(&self, movie: &Movie, title: &ProviderTitle) -> Result<()> {
        let locked = movie.locked_fields.clone().unwrap_or_default();
        let target = TitleTarget::new(TargetKind::Movie, &movie.id, &movie.user_id);
        let mut store = self.credit_store().await?;
        if !credits_locked(&locked) {
            let summary = store
                .replace_credits(&target, "tmdb", &title.credits)
                .await?;
            debug!(
                movie_id = %movie.id,
                created = summary.created,
                deleted = summary.deleted,
                "Saved {} credits of movie '{}'",
                title.credits.len(),
                movie.title
            );
        }
        store
            .save_title_detail(&target, "tmdb", &title.facts, &locked, &now())
            .await
    }

    /// A movie's cast from before credits were general, copied once into `Credit` so it still
    /// shows while TMDB cannot be read.
    async fn carry_over_movie_cast(&self, movie: &Movie) -> Result<bool> {
        let has_credits = !Credit::query(self.db.pool())
            .filter(CreditWhereInput {
                movie_id: string_eq(&movie.id),
                ..Default::default()
            })
            .limit(1)
            .fetch_all()
            .await?
            .is_empty();
        if has_credits {
            return Ok(false);
        }
        let old = MovieCastCredit::query(self.db.pool())
            .filter(MovieCastCreditWhereInput {
                movie_id: string_eq(&movie.id),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        if old.is_empty() {
            return Ok(false);
        }
        let mut entries = Vec::new();
        for credit in old {
            let Some(person) = Person::get(self.db.pool(), &credit.person_id).await? else {
                continue;
            };
            entries.push(CreditEntry {
                person: PersonRef {
                    name: person.name,
                    tmdb_id: (person.tmdb_person_id > 0).then_some(person.tmdb_person_id),
                    profile_url: person.profile_url,
                    ..Default::default()
                },
                kind: crate::services::metadata::credits::CreditKind::Cast,
                guest: false,
                department: Some("Acting".into()),
                job: None,
                character: credit.character_name,
                order: credit.cast_order,
                episode_count: None,
                provider_credit_id: None,
            });
        }
        let target = TitleTarget::new(TargetKind::Movie, &movie.id, &movie.user_id);
        self.credit_store()
            .await?
            .replace_credits(&target, "tmdb", &entries)
            .await?;
        Ok(true)
    }

    // -----------------------------------------------------------------------------------------
    // Shows, seasons and episodes

    /// The show's TMDB id: its own, else found from its TheTVDB or IMDb id.
    async fn tmdb_show_id(&self, show: &Show) -> Result<Option<i32>> {
        if let Some(id) = show.tmdb_id.filter(|id| *id > 0) {
            return Ok(Some(id));
        }
        let tmdb = self.get_tmdb_client().await?;
        let lookups = [
            show.tvdb_id.map(|id| (id.to_string(), "tvdb_id")),
            show.imdb_id.clone().map(|id| (id, "imdb_id")),
        ];
        for (id, source) in lookups.into_iter().flatten() {
            if let Some(document) = tmdb
                .get_value(&format!("/find/{id}"), &[("external_source", source)])
                .await?
                && let Some(found) = parse_tmdb_find_show(&document)
            {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    /// Read a show's TMDB documents (the show, then each season it has here) and keep the
    /// show's, every season's and every episode's credits, and the show's details.
    pub async fn sync_show_credits(&self, show: &Show) -> Result<()> {
        let Some(tmdb_show_id) = self.tmdb_show_id(show).await? else {
            anyhow::bail!("Show '{}' was not found on TMDB", show.name);
        };
        let tmdb = self.get_tmdb_client().await?;
        let document = tmdb
            .get_value(
                &format!("/tv/{tmdb_show_id}"),
                &[("append_to_response", SHOW_APPEND)],
            )
            .await?
            .ok_or_else(|| anyhow!("Show {tmdb_show_id} not found on TMDB"))?;
        let title = parse_tmdb_show(&document);
        let locked = show.locked_fields.clone().unwrap_or_default();
        let skip_credits = credits_locked(&locked);
        let mut store = self.credit_store().await?;
        let show_target = TitleTarget::new(TargetKind::Show, &show.id, &show.user_id);
        if !skip_credits {
            store
                .replace_credits(&show_target, "tmdb", &title.credits)
                .await?;
        }

        let seasons = Season::query(self.db.pool())
            .filter(SeasonWhereInput {
                show_id: string_eq(&show.id),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        let episodes = Episode::query(self.db.pool())
            .filter(EpisodeWhereInput {
                show_id: string_eq(&show.id),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        let season_ids: HashMap<i32, String> = seasons
            .iter()
            .map(|season| (season.season, season.id.clone()))
            .collect();
        let episode_ids: HashMap<(i32, i32), String> = episodes
            .iter()
            .map(|episode| ((episode.season, episode.episode), episode.id.clone()))
            .collect();
        let mut numbers: Vec<i32> = season_ids
            .keys()
            .copied()
            .chain(episodes.iter().map(|episode| episode.season))
            .collect();
        numbers.sort_unstable();
        numbers.dedup();

        let mut episode_credits = 0usize;
        if !skip_credits {
            for number in numbers {
                let season_document = match tmdb
                    .get_value(
                        &format!("/tv/{tmdb_show_id}/season/{number}"),
                        &[("append_to_response", "aggregate_credits")],
                    )
                    .await
                {
                    Ok(Some(document)) => document,
                    Ok(None) => continue,
                    Err(error) => {
                        warn!(
                            show_id = %show.id,
                            season = number,
                            error = %error,
                            "Could not read season {number} of '{}' from TMDB: {error}",
                            show.name
                        );
                        continue;
                    }
                };
                let season = parse_tmdb_season(&season_document);
                if let Some(season_id) = season_ids.get(&number) {
                    let target = TitleTarget::new(TargetKind::Season, season_id, &show.user_id);
                    store
                        .replace_credits(&target, "tmdb", &season.credits)
                        .await?;
                }
                for episode in &season.episodes {
                    let Some(episode_id) = episode_ids.get(&(number, episode.episode_number))
                    else {
                        continue;
                    };
                    let target = TitleTarget::new(TargetKind::Episode, episode_id, &show.user_id);
                    store
                        .replace_credits(&target, "tmdb", &episode.credits)
                        .await?;
                    episode_credits += episode.credits.len();
                }
            }
        }
        store
            .save_title_detail(&show_target, "tmdb", &title.facts, &locked, &now())
            .await?;
        info!(
            show_id = %show.id,
            show = %show.name,
            tmdb_id = tmdb_show_id,
            credits = title.credits.len(),
            episode_credits,
            "Saved credits and details of show '{}' from TMDB ({} show credits, {} episode credits)",
            show.name,
            title.credits.len(),
            episode_credits
        );
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Albums and tracks

    /// Keep an album's credits from a release already read (best effort).
    pub(super) async fn store_album_credits(&self, album: &Album, credits: &ReleaseCredits) {
        if let Err(error) = self.save_album_credits(album, credits).await {
            warn!(
                album_id = %album.id,
                album = %album.name,
                error = %error,
                "Could not save credits of album '{}': {error}",
                album.name
            );
        }
    }

    async fn save_album_credits(&self, album: &Album, credits: &ReleaseCredits) -> Result<()> {
        let mut store = self.credit_store().await?;
        let target = TitleTarget::new(TargetKind::Album, &album.id, &album.user_id);
        store
            .replace_credits(&target, "musicbrainz", &credits.credits)
            .await?;
        let tracks = Track::query(self.db.pool())
            .filter(TrackWhereInput {
                album_id: string_eq(&album.id),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        let mut track_credits = 0;
        for track in tracks {
            let Some(recording) = track.musicbrainz_id.as_deref() else {
                continue;
            };
            let entries = credits
                .recordings
                .get(recording)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let target = TitleTarget::new(TargetKind::Track, &track.id, &album.user_id);
            store
                .replace_credits(&target, "musicbrainz", entries)
                .await?;
            track_credits += entries.len();
        }
        let mut facts = TitleFacts {
            provider_id: Some(credits.release_id.clone()),
            studios: credits.labels.clone(),
            ..Default::default()
        };
        facts
            .external_ids
            .insert("musicbrainzRelease".into(), credits.release_id.clone());
        if let Some(group) = &album.musicbrainz_id {
            facts
                .external_ids
                .insert("musicbrainzReleaseGroup".into(), group.clone());
        }
        store
            .save_title_detail(&target, "musicbrainz", &facts, &[], &now())
            .await?;
        info!(
            album_id = %album.id,
            album = %album.name,
            release = %credits.release_id,
            credits = credits.credits.len(),
            track_credits,
            "Saved MusicBrainz credits of album '{}' ({} album credits, {} track credits)",
            album.name,
            credits.credits.len(),
            track_credits
        );
        Ok(())
    }

    /// Read an album's release from MusicBrainz and keep its credits.
    pub async fn sync_album_credits(&self, album: &Album) -> Result<bool> {
        let Some(group) = album
            .musicbrainz_id
            .as_deref()
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
        else {
            return Ok(false);
        };
        let musicbrainz = self.get_musicbrainz_client().await?;
        let Some(credits) = musicbrainz.get_release_group_credits(group).await? else {
            return Ok(false);
        };
        self.save_album_credits(album, &credits).await?;
        Ok(true)
    }

    // -----------------------------------------------------------------------------------------
    // Audiobooks

    /// Keep an audiobook's authors and narrators as people (best effort). The book's own
    /// `authorName`, `narratorName` and `narrators` stay as they are.
    pub(super) async fn store_audiobook_credits(&self, audiobook: &Audiobook, authors: &[String]) {
        if let Err(error) = self.save_audiobook_credits(audiobook, authors).await {
            warn!(
                audiobook_id = %audiobook.id,
                title = %audiobook.title,
                error = %error,
                "Could not save authors and narrators of audiobook '{}': {error}",
                audiobook.title
            );
        }
    }

    async fn save_audiobook_credits(
        &self,
        audiobook: &Audiobook,
        authors: &[String],
    ) -> Result<()> {
        let mut authors: Vec<String> = authors.to_vec();
        if authors.is_empty() {
            authors.extend(audiobook.author_name.clone());
        }
        let mut narrators = audiobook.narrators.clone();
        if narrators.is_empty() {
            narrators.extend(audiobook.narrator_name.clone());
        }
        let locked = audiobook.locked_fields.clone().unwrap_or_default();
        let target = TitleTarget::new(TargetKind::Audiobook, &audiobook.id, &audiobook.user_id);
        let mut store = self.credit_store().await?;
        if !credits_locked(&locked) {
            store
                .replace_credits(
                    &target,
                    "audiobook",
                    &audiobook_credits(&authors, &narrators),
                )
                .await?;
        }
        let mut facts = TitleFacts {
            provider_id: audiobook.audible_id.clone(),
            ..Default::default()
        };
        facts.studios.extend(audiobook.publisher.clone());
        for (site, id) in [
            ("asin", &audiobook.asin),
            ("isbn", &audiobook.isbn),
            ("goodreads", &audiobook.goodreads_id),
        ] {
            if let Some(id) = id.as_ref().filter(|id| !id.is_empty()) {
                facts.external_ids.insert(site.into(), id.clone());
            }
        }
        store
            .save_title_detail(&target, "audiobook", &facts, &locked, &now())
            .await
    }

    // -----------------------------------------------------------------------------------------
    // People

    /// Read a person's own TMDB record (biography, dates, place of birth, ids, pictures).
    pub async fn refresh_person_details(&self, person_id: &str) -> Result<Person> {
        let person = Person::get(self.db.pool(), &person_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("Person not found"))?;
        if person.tmdb_person_id <= 0 {
            anyhow::bail!("{} has no TMDB record to read", person.name);
        }
        let tmdb = self.get_tmdb_client().await?;
        let store = self.credit_store().await?;
        let fetched_at = now();
        match tmdb
            .get_value(
                &format!("/person/{}", person.tmdb_person_id),
                &[("append_to_response", PERSON_APPEND)],
            )
            .await?
            .as_ref()
            .and_then(parse_tmdb_person)
        {
            Some(details) => {
                store
                    .save_person_details(&person, &details, &fetched_at)
                    .await?
            }
            None => store.mark_person_fetched(&person.id, &fetched_at).await?,
        }
        Person::get(self.db.pool(), &person.id)
            .await?
            .ok_or_else(|| anyhow!("Person not found after refresh"))
    }

    // -----------------------------------------------------------------------------------------
    // The background fill

    /// One pass of the credits job: titles that have never had their credits and details read,
    /// then people whose own records have never been read, within `budget`.
    pub async fn fill_credits(&self, budget: CreditFillBudget) -> Result<CreditFillReport> {
        let mut report = CreditFillReport::default();
        let has_tmdb = self.has_tmdb().await;

        // Audiobooks first: they need no provider.
        let done = self.titles_with_details(TargetKind::Audiobook).await?;
        let books = Audiobook::query(self.db.pool())
            .filter(AudiobookWhereInput::default())
            .fetch_all()
            .await?;
        for book in books
            .iter()
            .filter(|book| !done.contains(&book.id))
            .filter(|book| first_attempt(format!("audiobook:{}", book.id)))
            .take(budget.audiobooks)
        {
            match self.save_audiobook_credits(book, &[]).await {
                Ok(()) => report.audiobooks += 1,
                Err(error) => warn!(
                    audiobook_id = %book.id,
                    title = %book.title,
                    error = %error,
                    "Credits job could not save audiobook '{}': {error}",
                    book.title
                ),
            }
        }

        let done = self.titles_with_details(TargetKind::Movie).await?;
        let movies = Movie::query(self.db.pool())
            .filter(MovieWhereInput {
                tmdb_id: Some(IntFilter {
                    is_null: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        for movie in movies
            .iter()
            .filter(|movie| !done.contains(&movie.id))
            .filter(|movie| first_attempt(format!("movie:{}", movie.id)))
            .take(budget.movies)
        {
            let fetched = if has_tmdb {
                self.sync_movie_credits(movie).await
            } else {
                Err(anyhow!("TMDB is not configured"))
            };
            match fetched {
                Ok(()) => report.movies += 1,
                Err(error) => {
                    debug!(
                        movie_id = %movie.id,
                        error = %error,
                        "Credits job could not read movie '{}' from TMDB: {error}",
                        movie.title
                    );
                    if self.carry_over_movie_cast(movie).await.unwrap_or(false) {
                        report.carried_over += 1;
                    }
                }
            }
        }

        if has_tmdb {
            let done = self.titles_with_details(TargetKind::Show).await?;
            let shows = Show::query(self.db.pool())
                .filter(ShowWhereInput::default())
                .fetch_all()
                .await?;
            for show in shows
                .iter()
                .filter(|show| !done.contains(&show.id))
                .filter(|show| first_attempt(format!("show:{}", show.id)))
                .take(budget.shows)
            {
                match self.sync_show_credits(show).await {
                    Ok(()) => report.shows += 1,
                    Err(error) => warn!(
                        show_id = %show.id,
                        show = %show.name,
                        error = %error,
                        "Credits job could not read show '{}' from TMDB: {error}",
                        show.name
                    ),
                }
            }
        }

        let done = self.titles_with_details(TargetKind::Album).await?;
        let albums = Album::query(self.db.pool())
            .filter(AlbumWhereInput {
                musicbrainz_id: not_null(),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        for album in albums
            .iter()
            .filter(|album| !done.contains(&album.id))
            .filter(|album| first_attempt(format!("album:{}", album.id)))
            .take(budget.albums)
        {
            match self.sync_album_credits(album).await {
                Ok(true) => report.albums += 1,
                Ok(false) => {}
                Err(error) => warn!(
                    album_id = %album.id,
                    album = %album.name,
                    error = %error,
                    "Credits job could not read album '{}' from MusicBrainz: {error}",
                    album.name
                ),
            }
        }

        if has_tmdb {
            let people = Person::query(self.db.pool())
                .filter(PersonWhereInput {
                    tmdb_person_id: Some(IntFilter {
                        gt: Some(0),
                        ..Default::default()
                    }),
                    details_fetched_at: Some(graphql_orm::graphql::filters::DateFilter {
                        is_null: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .limit((budget.people * 2) as i64)
                .fetch_all()
                .await?;
            for person in people
                .iter()
                .filter(|person| first_attempt(format!("person:{}", person.id)))
                .take(budget.people)
            {
                match self.refresh_person_details(&person.id).await {
                    Ok(_) => report.people += 1,
                    Err(error) => debug!(
                        person_id = %person.id,
                        error = %error,
                        "Credits job could not read person '{}' from TMDB: {error}",
                        person.name
                    ),
                }
            }
        }
        Ok(report)
    }
}
