//! The pictures a provider lists for a slot, fetched when the picker opens and kept briefly.
//!
//! - Movies: TMDB's posters, backdrops and logos, in every language.
//! - Shows: TVmaze's posters and backgrounds, and TMDB's when the show has a TMDB id.
//! - Seasons: TVmaze's season poster and TMDB's season posters.
//! - Episodes: TVmaze's still and TMDB's stills.
//! - Albums: Cover Art Archive for the release group and for its official editions (front,
//!   back and the rest).
//! - Artists: every Wikimedia Commons photo Wikidata names, and Deezer's pictures for artists of
//!   exactly that name (what the music scan already uses; Fanart.tv has no key here).
//! - Audiobooks: the provider record's cover, the Open Library work's covers, and the covers of
//!   Open Library's editions found by title and author.
//!
//! Each option says which one a metadata refresh would choose (`is_default`), which is what
//! reverting puts back. The parsing is pure so it is tested against recorded responses.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, warn};
use uuid::Uuid;

use super::{ArtworkEntityKind, ArtworkKind, ArtworkSlot};
use crate::db::Database;
use crate::graphql::entities::{Album, Artist, Audiobook, Episode, Movie, Season, Show};
use crate::services::http_client::{
    METADATA_RESPONSE_LIMIT, OutboundHttpProfile, outbound_client_builder, response_json_limited,
};
use crate::services::metadata::musicbrainz::{CoverArtArchiveResult, MusicBrainzEdition};
use crate::services::metadata::providers::{
    MetadataProvider, MetadataService, audiobook_provider_for_id,
};
use crate::services::metadata::tmdb::images::{TmdbImage, TmdbImageTarget, TmdbImages};
use crate::services::metadata::tvmaze::{TvMazeSeason, TvMazeShowImage, best_background};

/// How long a provider's list is kept: long enough to browse tabs and choose, short enough that
/// a new upload at the provider shows the next time.
const OPTIONS_TTL: Duration = Duration::from_secs(10 * 60);
/// Editions of an album asked about. Each is one Cover Art Archive request.
const MAX_EDITIONS: usize = 8;
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p/";

/// One picture a provider offers.
#[derive(Debug, Clone, PartialEq, async_graphql::SimpleObject, serde::Serialize)]
#[graphql(rename_fields = "camelCase")]
pub struct ArtworkOption {
    /// The full-size picture that is stored when chosen.
    pub url: String,
    /// A smaller copy of it for the picker's grid (the provider's own, not cached here).
    pub preview_url: String,
    /// Who offers it: "TMDB", "TVmaze", "Cover Art Archive"…
    pub source: String,
    /// The language of any text on it (ISO 639-1); none for textless art.
    pub language: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    /// TMDB's average vote and count.
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    /// What tells it apart: "Back", an edition's date and country.
    pub label: Option<String>,
    /// What a metadata refresh chooses, and what reverting puts back.
    pub is_default: bool,
    /// What the slot shows now.
    pub is_current: bool,
}

impl ArtworkOption {
    fn new(source: &str, url: String, preview_url: Option<String>) -> Self {
        Self {
            preview_url: preview_url.unwrap_or_else(|| url.clone()),
            url,
            source: source.to_string(),
            language: None,
            width: None,
            height: None,
            vote_average: None,
            vote_count: None,
            label: None,
            is_default: false,
            is_current: false,
        }
    }
}

/// A slot's provider pictures, and the providers that couldn't be asked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProviderOptions {
    pub options: Vec<ArtworkOption>,
    /// Plain names of what failed or isn't set up ("TMDB isn't set up", "Cover Art Archive").
    pub unavailable: Vec<String>,
}

impl ProviderOptions {
    fn push(&mut self, option: ArtworkOption) {
        if let Some(existing) = self
            .options
            .iter_mut()
            .find(|seen| same_image(&seen.url, &option.url))
        {
            existing.is_default |= option.is_default;
            return;
        }
        self.options.push(option);
    }

    fn extend(&mut self, options: impl IntoIterator<Item = ArtworkOption>) {
        for option in options {
            self.push(option);
        }
    }

    fn failed(&mut self, provider: &str, error: &anyhow::Error) {
        warn!(provider, error = %error, "Could not list artwork from {provider}");
        self.unavailable.push(provider.to_string());
    }

    /// Mark the picture a refresh would choose. A default the lists don't hold (a provider record
    /// older than its image list) is added, so reverting always has it.
    fn set_default(&mut self, source: &str, url: Option<&str>) {
        let Some(url) = url.filter(|url| !url.trim().is_empty()) else {
            return;
        };
        match self
            .options
            .iter_mut()
            .find(|option| same_image(&option.url, url))
        {
            Some(option) => option.is_default = true,
            None => {
                let mut option = ArtworkOption::new(source, url.to_string(), None);
                option.is_default = true;
                self.options.insert(0, option);
            }
        }
    }

    /// The default first, the rest in the providers' order.
    fn finish(mut self) -> Self {
        self.options.sort_by_key(|option| !option.is_default);
        self
    }

    /// Mark what the slot shows now.
    pub fn with_current(mut self, current: Option<&str>) -> Self {
        for option in &mut self.options {
            option.is_current = current.is_some_and(|current| same_image(&option.url, current));
        }
        self
    }
}

/// Whether two picture URLs are the same picture: TMDB serves one file at several sizes, and
/// Cover Art Archive is reached over http or https.
pub fn same_image(a: &str, b: &str) -> bool {
    fn key(url: &str) -> String {
        let url = url.trim();
        let url = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        if let Some(rest) = url.strip_prefix("image.tmdb.org/t/p/")
            && let Some((_, file)) = rest.split_once('/')
        {
            return format!("tmdb/{file}");
        }
        url.to_string()
    }
    key(a) == key(b)
}

fn https(url: &str) -> String {
    match url.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_string(),
    }
}

fn tmdb_url(size: &str, path: &str) -> String {
    format!("{TMDB_IMAGE_BASE}{size}{path}")
}

/// TMDB's list for a slot: original size to store, a grid-sized copy to preview, best voted first.
pub fn tmdb_options(images: &TmdbImages, kind: ArtworkKind) -> Vec<ArtworkOption> {
    let (list, preview_size): (&[TmdbImage], &str) = match kind {
        ArtworkKind::Poster | ArtworkKind::Cover => (&images.posters, "w342"),
        ArtworkKind::Backdrop => (&images.backdrops, "w780"),
        ArtworkKind::Logo => (&images.logos, "w500"),
        ArtworkKind::Thumbnail => (&images.stills, "w300"),
        // A person's photo is only ever uploaded.
        ArtworkKind::Avatar => return Vec::new(),
    };
    let mut list = list
        .iter()
        .filter(|image| image.file_path.starts_with('/'))
        .collect::<Vec<_>>();
    list.sort_by(|a, b| {
        let votes = |image: &TmdbImage| {
            (
                image.vote_average.unwrap_or(0.0),
                image.vote_count.unwrap_or(0),
            )
        };
        votes(b)
            .partial_cmp(&votes(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    list.into_iter()
        .map(|image| {
            let mut option = ArtworkOption::new(
                "TMDB",
                tmdb_url("original", &image.file_path),
                Some(tmdb_url(preview_size, &image.file_path)),
            );
            option.language = image
                .iso_639_1
                .clone()
                .filter(|language| !language.is_empty() && language != "xx");
            option.width = image.width;
            option.height = image.height;
            option.vote_average = image.vote_average;
            option.vote_count = image.vote_count;
            option
        })
        .collect()
}

/// TVmaze's show images of one kind: posters, or backgrounds for a backdrop. The main one first.
pub fn tvmaze_show_options(images: &[TvMazeShowImage], kind: ArtworkKind) -> Vec<ArtworkOption> {
    let wanted = match kind {
        ArtworkKind::Poster => "poster",
        ArtworkKind::Backdrop => "background",
        _ => return Vec::new(),
    };
    let mut list = images
        .iter()
        .filter(|image| image.image_type.as_deref() == Some(wanted))
        .filter(|image| image.resolutions.original.is_some())
        .collect::<Vec<_>>();
    list.sort_by_key(|image| !image.main);
    list.into_iter()
        .filter_map(|image| {
            let original = image.resolutions.original.as_ref()?;
            let mut option = ArtworkOption::new(
                "TVmaze",
                original.url.clone(),
                image
                    .resolutions
                    .medium
                    .as_ref()
                    .map(|medium| medium.url.clone()),
            );
            option.width = original.width.map(|width| width as i32);
            option.height = original.height.map(|height| height as i32);
            Some(option)
        })
        .collect()
}

/// The picture a show refresh takes from TVmaze: the main poster, or `best_background`.
pub fn tvmaze_show_default(images: &[TvMazeShowImage], kind: ArtworkKind) -> Option<String> {
    match kind {
        ArtworkKind::Backdrop => best_background(images),
        ArtworkKind::Poster => images
            .iter()
            .filter(|image| image.image_type.as_deref() == Some("poster"))
            .find(|image| image.main)
            .and_then(|image| image.resolutions.original.as_ref())
            .map(|original| original.url.clone()),
        _ => None,
    }
}

/// TVmaze's poster for one season, which is also what a refresh stores.
pub fn tvmaze_season_option(seasons: &[TvMazeSeason], number: i32) -> Option<ArtworkOption> {
    let image = seasons
        .iter()
        .find(|season| season.number as i32 == number)?
        .image
        .as_ref()?;
    let url = image.original.clone().or_else(|| image.medium.clone())?;
    let mut option = ArtworkOption::new("TVmaze", url, image.medium.clone());
    option.is_default = true;
    Some(option)
}

/// What tells an edition apart: when, where, and MusicBrainz's note.
pub fn edition_label(edition: &MusicBrainzEdition) -> Option<String> {
    let parts = [
        edition
            .date
            .as_deref()
            .map(|date| date.get(..4).unwrap_or(date)),
        edition.country.as_deref(),
        edition.disambiguation.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|part| !part.trim().is_empty())
    .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// Cover Art Archive's images for a release or release group: fronts first, then backs, then the
/// rest (booklets, media), each labelled by side and the edition. The stored copy is the 1200px
/// thumbnail when there is one, as for the album cover the scan finds, since originals are scans
/// of any size.
pub fn cover_art_options(
    result: &CoverArtArchiveResult,
    edition: Option<&str>,
) -> Vec<ArtworkOption> {
    let mut images = result.images.iter().collect::<Vec<_>>();
    images.sort_by_key(|image| (!image.front, !image.back));
    images
        .into_iter()
        .map(|image| {
            let url = image
                .thumbnails
                .large
                .clone()
                .unwrap_or_else(|| image.image.clone());
            let preview = image
                .thumbnails
                .medium
                .clone()
                .or_else(|| image.thumbnails.small.clone());
            let mut option = ArtworkOption::new(
                "Cover Art Archive",
                https(&url),
                preview.map(|url| https(&url)),
            );
            let side = if image.front {
                Some("Front".to_string())
            } else if image.back {
                Some("Back".to_string())
            } else {
                image
                    .comment
                    .clone()
                    .filter(|comment| !comment.trim().is_empty())
                    .or_else(|| Some("Other".to_string()))
            };
            option.label = match (side, edition) {
                (Some(side), Some(edition)) => Some(format!("{side} · {edition}")),
                (side, None) => side,
                (None, Some(edition)) => Some(edition.to_string()),
            };
            option
        })
        .collect()
}

/// The release group's front cover, which is what the scan stores for an album.
pub fn cover_art_default(result: &CoverArtArchiveResult) -> Option<String> {
    result.images.iter().find(|image| image.front).map(|image| {
        image
            .thumbnails
            .large
            .clone()
            .unwrap_or_else(|| image.image.clone())
    })
}

/// Pictures from a list of URLs, all from one provider.
pub fn url_options(source: &str, urls: &[String]) -> Vec<ArtworkOption> {
    urls.iter()
        .map(|url| ArtworkOption::new(source, url.clone(), None))
        .collect()
}

fn open_library_cover(id: i64) -> ArtworkOption {
    ArtworkOption::new(
        "Open Library",
        format!("https://covers.openlibrary.org/b/id/{id}-L.jpg"),
        Some(format!("https://covers.openlibrary.org/b/id/{id}-M.jpg")),
    )
}

/// The covers of an Open Library work (`/works/{id}.json`). Negative ids mean "no cover".
pub fn open_library_work_options(work: &Value) -> Vec<ArtworkOption> {
    work.get("covers")
        .and_then(Value::as_array)
        .map(|covers| {
            covers
                .iter()
                .filter_map(Value::as_i64)
                .filter(|id| *id > 0)
                .map(open_library_cover)
                .collect()
        })
        .unwrap_or_default()
}

/// The covers of Open Library search results (`search.json`), labelled by title and year.
pub fn open_library_search_options(body: &Value) -> Vec<ArtworkOption> {
    body.get("docs")
        .and_then(Value::as_array)
        .map(|docs| {
            docs.iter()
                .filter_map(|doc| {
                    let id = doc
                        .get("cover_i")
                        .and_then(Value::as_i64)
                        .filter(|id| *id > 0)?;
                    let mut option = open_library_cover(id);
                    let title = doc.get("title").and_then(Value::as_str);
                    let year = doc.get("first_publish_year").and_then(Value::as_i64);
                    option.label = match (title, year) {
                        (Some(title), Some(year)) => Some(format!("{title} · {year}")),
                        (Some(title), None) => Some(title.to_string()),
                        (None, Some(year)) => Some(year.to_string()),
                        (None, None) => None,
                    };
                    Some(option)
                })
                .collect()
        })
        .unwrap_or_default()
}

static CACHE: LazyLock<Mutex<HashMap<String, (Instant, ProviderOptions)>>> =
    LazyLock::new(Default::default);

fn cache_key(slot: ArtworkSlot, id: &str) -> String {
    format!("{}/{}/{}", slot.entity_type(), id, slot.artwork_type())
}

/// A slot's provider list, from the brief cache or asked for now. A list with a provider that
/// failed is not kept, so opening the picker again asks again.
pub async fn provider_options(
    metadata: &MetadataService,
    db: &Database,
    slot: ArtworkSlot,
    id: &str,
) -> Result<ProviderOptions> {
    let key = cache_key(slot, id);
    if let Some((at, options)) = CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key)
        && at.elapsed() < OPTIONS_TTL
    {
        return Ok(options.clone());
    }
    let options = fetch(metadata, db, slot, id).await?.finish();
    debug!(
        entity_type = slot.entity_type(),
        entity_id = %id,
        artwork_type = slot.artwork_type(),
        count = options.options.len(),
        unavailable = ?options.unavailable,
        "Listed provider artwork for {} {}",
        slot.entity_type(),
        slot.artwork_type()
    );
    if options.unavailable.is_empty() {
        let mut cache = CACHE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.retain(|_, (at, _)| at.elapsed() < OPTIONS_TTL);
        cache.insert(key, (Instant::now(), options.clone()));
    }
    Ok(options)
}

async fn fetch(
    metadata: &MetadataService,
    db: &Database,
    slot: ArtworkSlot,
    id: &str,
) -> Result<ProviderOptions> {
    let pool = db.pool();
    let id = id.to_string();
    let mut out = ProviderOptions::default();
    match slot.entity {
        ArtworkEntityKind::Movie => {
            let movie = Movie::get(pool, &id).await?.context("Movie not found")?;
            let Some(tmdb_id) = movie.tmdb_id else {
                return Ok(out);
            };
            match metadata.get_tmdb_client().await {
                Ok(tmdb) => match tmdb.get_images(TmdbImageTarget::Movie(tmdb_id)).await {
                    Ok(images) => out.extend(tmdb_options(&images, slot.kind)),
                    Err(error) => out.failed("TMDB", &error),
                },
                Err(_) => out.unavailable.push("TMDB isn't set up".to_string()),
            }
            if slot.kind != ArtworkKind::Logo
                && let Ok(details) = metadata.get_movie(tmdb_id as u32).await
            {
                let default = if slot.kind == ArtworkKind::Backdrop {
                    details.backdrop_url
                } else {
                    details.poster_url
                };
                out.set_default("TMDB", default.as_deref());
            }
        }
        ArtworkEntityKind::Show => {
            let show = Show::get(pool, &id).await?.context("Show not found")?;
            if let Some(tvmaze_id) = show.tvmaze_id {
                match tvmaze_images(metadata, tvmaze_id).await {
                    Ok(images) => {
                        out.extend(tvmaze_show_options(&images, slot.kind));
                        let default = match slot.kind {
                            // A refresh stores the show record's poster.
                            ArtworkKind::Poster => metadata
                                .get_tv_show(tvmaze_id as u32)
                                .await
                                .ok()
                                .and_then(|details| details.poster_url)
                                .or_else(|| tvmaze_show_default(&images, slot.kind)),
                            _ => tvmaze_show_default(&images, slot.kind),
                        };
                        out.set_default("TVmaze", default.as_deref());
                    }
                    Err(error) => out.failed("TVmaze", &error),
                }
            }
            if let Some(tmdb_id) = show.tmdb_id {
                tmdb_into(
                    metadata,
                    &mut out,
                    TmdbImageTarget::Show(tmdb_id),
                    slot.kind,
                )
                .await;
            }
        }
        ArtworkEntityKind::Season => {
            let season = Season::get(pool, &id).await?.context("Season not found")?;
            let show = Show::get(pool, &season.show_id)
                .await?
                .context("Show not found")?;
            if let Some(tvmaze_id) = show.tvmaze_id {
                match metadata.get_tvmaze_client().await {
                    Ok(client) => match client.get_seasons(tvmaze_id as u32).await {
                        Ok(seasons) => {
                            if let Some(option) = tvmaze_season_option(&seasons, season.season) {
                                out.push(option);
                            }
                        }
                        Err(error) => out.failed("TVmaze", &error),
                    },
                    Err(error) => out.failed("TVmaze", &error),
                }
            }
            if let Some(tmdb_id) = show.tmdb_id {
                let target = TmdbImageTarget::Season {
                    show: tmdb_id,
                    season: season.season,
                };
                tmdb_into(metadata, &mut out, target, ArtworkKind::Poster).await;
            }
        }
        ArtworkEntityKind::Episode => {
            let episode = Episode::get(pool, &id)
                .await?
                .context("Episode not found")?;
            let show = Show::get(pool, &episode.show_id)
                .await?
                .context("Show not found")?;
            if let Some(tvmaze_id) = show.tvmaze_id {
                match metadata.get_tv_show_episodes(tvmaze_id as u32).await {
                    Ok(episodes) => {
                        let still = episodes
                            .into_iter()
                            .find(|row| {
                                row.season as i32 == episode.season
                                    && row.number as i32 == episode.episode
                            })
                            .and_then(|row| row.still_url);
                        out.set_default("TVmaze", still.as_deref());
                    }
                    Err(error) => out.failed("TVmaze", &error),
                }
            }
            if let Some(tmdb_id) = show.tmdb_id {
                let target = TmdbImageTarget::Episode {
                    show: tmdb_id,
                    season: episode.season,
                    episode: episode.episode,
                };
                tmdb_into(metadata, &mut out, target, ArtworkKind::Thumbnail).await;
            }
        }
        ArtworkEntityKind::Album => {
            let album = Album::get(pool, &id).await?.context("Album not found")?;
            let Some(release_group) = album
                .musicbrainz_id
                .as_deref()
                .and_then(|id| Uuid::parse_str(id).ok())
            else {
                return Ok(out);
            };
            let mut listed_default = None;
            match cover_art(&format!("release-group/{release_group}")).await {
                Ok(Some(result)) => {
                    out.extend(cover_art_options(&result, None));
                    listed_default = cover_art_default(&result);
                }
                Ok(None) => {}
                Err(error) => out.failed("Cover Art Archive", &error),
            }
            // What the scan stores: the remembered cover (with its retries), else the listing's front.
            let remembered = metadata.album_cover(release_group).await.ok().flatten();
            out.set_default(
                "Cover Art Archive",
                remembered.or(listed_default).as_deref(),
            );
            match metadata.get_musicbrainz_client().await {
                Ok(musicbrainz) => {
                    match musicbrainz.get_release_group_editions(release_group).await {
                        Ok(mut editions) => {
                            editions.sort_by_key(|edition| !edition.official);
                            for edition in editions.iter().take(MAX_EDITIONS) {
                                match cover_art(&format!("release/{}", edition.id)).await {
                                    Ok(Some(result)) => out.extend(cover_art_options(
                                        &result,
                                        edition_label(edition).as_deref(),
                                    )),
                                    Ok(None) => {}
                                    Err(error) => {
                                        debug!(release_id = %edition.id, error = %error, "Cover Art Archive had nothing for an edition");
                                    }
                                }
                            }
                        }
                        Err(error) => out.failed("MusicBrainz", &error),
                    }
                }
                Err(error) => out.failed("MusicBrainz", &error),
            }
        }
        ArtworkEntityKind::Artist => {
            use crate::services::metadata::artist_image::{deezer_images, wikidata_images};
            let artist = Artist::get(pool, &id).await?.context("Artist not found")?;
            let musicbrainz_id = artist
                .musicbrainz_id
                .as_deref()
                .and_then(|id| Uuid::parse_str(id).ok());
            let musicbrainz = metadata.get_musicbrainz_client().await?;
            let user_agent = musicbrainz.user_agent().to_string();
            if let Some(mbid) = musicbrainz_id {
                match musicbrainz.get_artist_wikidata_id(mbid).await {
                    Ok(Some(item)) => match wikidata_images(&user_agent, &item).await {
                        Ok(urls) => out.extend(url_options("Wikimedia Commons", &urls)),
                        Err(error) => out.failed("Wikidata", &error),
                    },
                    Ok(None) => {}
                    Err(error) => out.failed("MusicBrainz", &error),
                }
            }
            match deezer_images(&user_agent, &artist.name).await {
                Ok(urls) => out.extend(url_options("Deezer", &urls)),
                Err(error) => out.failed("Deezer", &error),
            }
            if let Ok(Some(default)) = metadata.artist_image(musicbrainz_id, &artist.name).await {
                let source = if default.contains("dzcdn.net") {
                    "Deezer"
                } else {
                    "Wikimedia Commons"
                };
                out.set_default(source, Some(&default));
            }
        }
        ArtworkEntityKind::Audiobook => {
            let book = Audiobook::get(pool, &id)
                .await?
                .context("Audiobook not found")?;
            let provider_id = book.audible_id.clone().filter(|id| !id.trim().is_empty());
            let mut default = None;
            if let Some(provider_id) = &provider_id {
                let source = match audiobook_provider_for_id(provider_id) {
                    MetadataProvider::Audible => "Audible",
                    _ => "Open Library",
                };
                match metadata.get_audiobook(provider_id).await {
                    Ok(details) => default = details.cover_url.map(|url| (source, url)),
                    Err(error) => out.failed(source, &error),
                }
                if audiobook_provider_for_id(provider_id) == MetadataProvider::OpenLibrary {
                    match open_library_json(
                        &format!("https://openlibrary.org/works/{provider_id}.json"),
                        &[],
                    )
                    .await
                    {
                        Ok(work) => out.extend(open_library_work_options(&work)),
                        Err(error) => out.failed("Open Library", &error),
                    }
                }
            }
            let mut query = vec![
                ("title", book.title.clone()),
                ("fields", "cover_i,title,first_publish_year".to_string()),
                ("limit", "12".to_string()),
            ];
            if let Some(author) = book
                .author_name
                .clone()
                .filter(|name| !name.trim().is_empty())
            {
                query.push(("author", author));
            }
            match open_library_json("https://openlibrary.org/search.json", &query).await {
                Ok(body) => out.extend(open_library_search_options(&body)),
                Err(error) => {
                    if !out.unavailable.iter().any(|name| name == "Open Library") {
                        out.failed("Open Library", &error);
                    }
                }
            }
            if let Some((source, url)) = default {
                out.set_default(source, Some(&url));
            }
        }
        ArtworkEntityKind::Playlist | ArtworkEntityKind::User => {}
    }
    Ok(out)
}

async fn tmdb_into(
    metadata: &MetadataService,
    out: &mut ProviderOptions,
    target: TmdbImageTarget,
    kind: ArtworkKind,
) {
    // TMDB is extra for TV: without a key the TVmaze pictures are still offered.
    let Ok(tmdb) = metadata.get_tmdb_client().await else {
        return;
    };
    match tmdb.get_images(target).await {
        Ok(images) => out.extend(tmdb_options(&images, kind)),
        Err(error) => out.failed("TMDB", &error),
    }
}

async fn tvmaze_images(metadata: &MetadataService, tvmaze_id: i32) -> Result<Vec<TvMazeShowImage>> {
    metadata
        .get_tvmaze_client()
        .await?
        .get_show_images(tvmaze_id as u32)
        .await
}

/// A Cover Art Archive listing (`release/{id}` or `release-group/{id}`). `None` when it has none.
async fn cover_art(path: &str) -> Result<Option<CoverArtArchiveResult>> {
    let client = outbound_client_builder(OutboundHttpProfile::Metadata)
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(Duration::from_secs(20))
        .build()
        .context("Failed to build the Cover Art Archive client")?;
    let response = client
        .get(format!("https://coverartarchive.org/{path}"))
        .header("Accept", "application/json")
        .send()
        .await
        .with_context(|| format!("Cover Art Archive didn't answer for {path}"))?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    anyhow::ensure!(
        response.status().is_success(),
        "Cover Art Archive returned HTTP {} for {path}",
        response.status()
    );
    Ok(Some(
        response_json_limited(response, METADATA_RESPONSE_LIMIT)
            .await
            .with_context(|| format!("Failed to read Cover Art Archive's listing for {path}"))?,
    ))
}

async fn open_library_json(url: &str, query: &[(&str, String)]) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(transparent)]
    struct Body(Value);
    let client = outbound_client_builder(OutboundHttpProfile::Metadata)
        .timeout(Duration::from_secs(20))
        .build()
        .context("Failed to build the Open Library client")?;
    let response = client
        .get(url)
        .query(query)
        .send()
        .await
        .with_context(|| format!("Open Library didn't answer for {url}"))?;
    anyhow::ensure!(
        response.status().is_success(),
        "Open Library returned HTTP {} for {url}",
        response.status()
    );
    let Body(body) = response_json_limited(response, METADATA_RESPONSE_LIMIT).await?;
    Ok(body)
}
