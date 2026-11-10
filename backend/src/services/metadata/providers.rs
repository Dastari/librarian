//! Unified metadata service for TMDB movie search
//!
//! Provides a simplified interface for searching movies on TMDB.
//! API key is loaded from app_settings database table.

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::musicbrainz::MusicBrainzClient;
use super::settings::MetadataSettings;
use super::tmdb::TmdbClient;
use super::tvmaze::TvMazeClient;
use crate::db::Database;
use crate::services::graphql::entities::common::AutoDownloadMode;
use crate::services::graphql::entities::{
    Album, Artist, Audiobook, Episode, EpisodeWhereInput, Library, Movie, Season, SeasonWhereInput,
    Show,
};
use crate::services::graphql::{AuthUser, LibrarianSchema};
use crate::services::manager::ServicesManager;
use crate::services::write_rules;
use async_graphql::{Request, Variables};
use graphql_orm::graphql::filters::StringFilter;

/// Transport success is not mutation success. These internal documents select a
/// success/error payload for every mutation root, including compatibility aliases.
fn check_mutation_payloads(data: &serde_json::Value) -> anyhow::Result<()> {
    let roots = data
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Mutation response is not an object"))?;
    if roots.is_empty() {
        anyhow::bail!("Mutation response has no result payload");
    }
    let mut checked = false;
    for (name, payload) in roots {
        if name == "__typename" {
            continue;
        }
        checked = true;
        let success = payload
            .get("success")
            .or_else(|| payload.get("Success"))
            .and_then(serde_json::Value::as_bool);
        if success != Some(true) {
            let error = payload
                .get("error")
                .or_else(|| payload.get("Error"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("missing or unsuccessful mutation payload");
            anyhow::bail!("{name}: {error}");
        }
    }
    if !checked {
        anyhow::bail!("Mutation response has no result payload");
    }
    Ok(())
}

const AUDIBLE_API: &str = "https://api.audible.com/1.0";
const AUDNEXUS_API: &str = "https://api.audnex.us";
const AUDIBLE_RESPONSE_GROUPS: &str =
    "contributors,media,product_attrs,product_desc,product_extended_attrs,series";

#[derive(Debug, Deserialize)]
struct AudibleSearchResponse {
    #[serde(default)]
    products: Vec<AudibleProduct>,
}

#[derive(Debug, Deserialize)]
struct AudibleProductResponse {
    product: AudibleProduct,
}

#[derive(Debug, Default, Deserialize)]
struct AudibleProduct {
    asin: String,
    title: Option<String>,
    subtitle: Option<String>,
    #[serde(default)]
    authors: Option<Vec<AudibleContributor>>,
    #[serde(default)]
    narrators: Option<Vec<AudibleContributor>>,
    content_delivery_type: Option<String>,
    language: Option<String>,
    publisher_name: Option<String>,
    publisher_summary: Option<String>,
    merchandising_summary: Option<String>,
    release_date: Option<String>,
    runtime_length_min: Option<i64>,
    #[serde(default)]
    product_images: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    series: Option<Vec<AudibleSeries>>,
}

#[derive(Debug, Default, Deserialize)]
struct AudibleSeries {
    title: Option<String>,
    sequence: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudibleContributor {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudnexusChapters {
    #[serde(rename = "runtimeLengthSec")]
    runtime_length_sec: Option<i64>,
    #[serde(default)]
    chapters: Vec<AudnexusChapter>,
}

#[derive(Debug, Deserialize)]
struct AudnexusChapter {
    title: Option<String>,
    #[serde(rename = "startOffsetMs")]
    start_offset_ms: i64,
    #[serde(rename = "lengthMs")]
    length_ms: i64,
}

/// Audible's ids are ten letters and digits. Open Library's look like `OL45171322W`.
fn is_asin(id: &str) -> bool {
    id.len() == 10 && id.bytes().all(|byte| byte.is_ascii_alphanumeric()) && !id.starts_with("OL")
}

/// Which provider an audiobook id belongs to. Results from either can reach the same paths.
pub fn audiobook_provider_for_id(id: &str) -> MetadataProvider {
    if is_asin(id) {
        MetadataProvider::Audible
    } else {
        MetadataProvider::OpenLibrary
    }
}

/// Books only: the catalogue also holds podcast episodes.
fn is_audible_book(product: &AudibleProduct) -> bool {
    matches!(
        product.content_delivery_type.as_deref(),
        Some("SinglePartBook" | "MultiPartBook")
    )
}

fn first_contributor(people: &Option<Vec<AudibleContributor>>) -> Option<String> {
    people
        .as_ref()?
        .iter()
        .find_map(|person| person.name.clone())
}

/// The largest cover offered: the keys are pixel sizes.
fn audible_cover(product: &AudibleProduct) -> Option<String> {
    let images = product.product_images.as_ref()?;
    images
        .iter()
        .max_by_key(|(size, _)| size.parse::<u32>().unwrap_or(0))
        .map(|(_, url)| url.clone())
}

fn release_year(date: Option<&str>) -> Option<i32> {
    date?.get(..4)?.parse().ok()
}

/// Audible's summaries are HTML. What's wanted is the text.
fn strip_html(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut in_tag = false;
    for character in text.chars() {
        match character {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                if !plain.ends_with(char::is_whitespace) {
                    plain.push(' ');
                }
            }
            _ if !in_tag => plain.push(character),
            _ => {}
        }
    }
    plain
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn audible_search_results(mut products: Vec<AudibleProduct>) -> Vec<AudiobookSearchResult> {
    // Audible's relevance can rank a translation above the original. Stable, so otherwise its
    // order stands.
    products.sort_by_key(|product| {
        !product
            .language
            .as_deref()
            .is_some_and(|language| language.eq_ignore_ascii_case("english"))
    });
    products
        .into_iter()
        .filter(is_audible_book)
        .filter_map(|product| {
            let cover_url = audible_cover(&product);
            let author_name = first_contributor(&product.authors);
            let narrator = first_contributor(&product.narrators);
            let year = release_year(product.release_date.as_deref());
            let description = product
                .merchandising_summary
                .as_deref()
                .or(product.publisher_summary.as_deref())
                .map(strip_html);
            Some(AudiobookSearchResult {
                provider: MetadataProvider::Audible,
                provider_id: product.asin,
                title: product.title?,
                author_name,
                year,
                cover_url,
                isbn: None,
                description,
                duration_secs: product.runtime_length_min.map(|minutes| minutes * 60),
                narrator,
            })
        })
        .collect()
}

fn audible_details(product: AudibleProduct) -> AudiobookDetails {
    let cover_url = audible_cover(&product);
    let author_name = first_contributor(&product.authors);
    let authors = product
        .authors
        .iter()
        .flatten()
        .filter_map(|person| person.name.clone())
        .collect();
    let narrators = product
        .narrators
        .iter()
        .flatten()
        .filter_map(|person| person.name.clone())
        .collect();
    let description = product
        .publisher_summary
        .as_deref()
        .or(product.merchandising_summary.as_deref())
        .map(strip_html);
    let series = product.series.iter().flatten().find(|series| {
        series
            .title
            .as_deref()
            .is_some_and(|name| !name.trim().is_empty())
    });
    let series_name = series.and_then(|series| series.title.clone());
    let series_position = series.and_then(|series| series.sequence.clone());
    let title = match (product.title, product.subtitle) {
        (Some(title), Some(subtitle)) if !title.contains(&subtitle) => {
            format!("{title}: {subtitle}")
        }
        (Some(title), _) => title,
        (None, _) => "Unknown Audiobook".to_string(),
    };
    AudiobookDetails {
        provider: MetadataProvider::Audible,
        provider_id: product.asin,
        title,
        sort_title: None,
        author_name,
        description,
        publisher: product.publisher_name,
        language: product.language,
        isbn: None,
        published_date: product.release_date,
        cover_url,
        narrators,
        total_duration_secs: product.runtime_length_min.map(|minutes| minutes * 60),
        chapters: Vec::new(),
        series_name,
        series_position,
        authors,
    }
}

fn audiobook_chapters(chapters: Vec<AudnexusChapter>) -> Vec<AudiobookChapterDetails> {
    chapters
        .into_iter()
        .enumerate()
        .map(|(index, chapter)| AudiobookChapterDetails {
            title: chapter
                .title
                .map(|title| title.replace('\u{a0}', " ").trim().to_string())
                .filter(|title| !title.is_empty())
                .unwrap_or_else(|| format!("Chapter {}", index + 1)),
            start_secs: chapter.start_offset_ms as f64 / 1000.0,
            length_secs: chapter.length_ms as f64 / 1000.0,
        })
        .collect()
}

#[cfg(test)]
mod audible_tests {
    use super::*;

    fn product(asin: &str, title: &str, delivery: &str) -> AudibleProduct {
        AudibleProduct {
            asin: asin.to_string(),
            title: Some(title.to_string()),
            content_delivery_type: Some(delivery.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn search_keeps_books_and_drops_podcast_episodes() {
        let results = audible_search_results(vec![
            product("B017V6627U", "A book", "SinglePartBook"),
            product("B0HQDSCZZ6", "Episode 15", "PodcastEpisode"),
            product("B0AAAAAAAA", "In parts", "MultiPartBook"),
        ]);
        let ids: Vec<_> = results
            .iter()
            .map(|book| book.provider_id.as_str())
            .collect();
        assert_eq!(ids, ["B017V6627U", "B0AAAAAAAA"]);
        assert!(
            results
                .iter()
                .all(|book| book.provider == MetadataProvider::Audible)
        );
    }

    #[test]
    fn english_editions_come_first_and_otherwise_keep_their_rank() {
        let edition = |asin: &str, language: &str| AudibleProduct {
            language: Some(language.to_string()),
            ..product(asin, asin, "SinglePartBook")
        };
        let results = audible_search_results(vec![
            edition("1781105030", "spanish"),
            edition("B017V6627U", "English"),
            edition("B07T3PFL59", "japanese"),
            edition("B0F1553GZJ", "english"),
        ]);
        let ids: Vec<_> = results
            .iter()
            .map(|book| book.provider_id.as_str())
            .collect();
        assert_eq!(
            ids,
            ["B017V6627U", "B0F1553GZJ", "1781105030", "B07T3PFL59"]
        );
    }

    #[test]
    fn ids_name_their_provider() {
        assert_eq!(
            audiobook_provider_for_id("B017V6627U"),
            MetadataProvider::Audible
        );
        assert_eq!(
            audiobook_provider_for_id("OL45171322W"),
            MetadataProvider::OpenLibrary
        );
        assert_eq!(
            audiobook_provider_for_id("OL1W"),
            MetadataProvider::OpenLibrary
        );
        assert!(!is_asin("../../etc"));
    }

    #[test]
    fn the_largest_cover_wins() {
        let mut book = product("B017V6627U", "A book", "SinglePartBook");
        book.product_images = Some(
            [
                ("500".to_string(), "small".to_string()),
                ("1000".to_string(), "large".to_string()),
            ]
            .into(),
        );
        assert_eq!(audible_cover(&book).as_deref(), Some("large"));
    }

    #[test]
    fn summaries_lose_their_markup() {
        assert_eq!(
            strip_html("<p>One &amp; two</p><p>Three&nbsp;four</p>"),
            "One & two Three four"
        );
    }

    #[test]
    fn chapters_read_cleanly_and_none_is_left_without_a_name() {
        let chapters = audiobook_chapters(vec![
            AudnexusChapter {
                title: Some("Chapter One -\u{a0}The Worst Birthday".into()),
                start_offset_ms: 29_970,
                length_ms: 1_099_285,
            },
            AudnexusChapter {
                title: None,
                start_offset_ms: 1_129_255,
                length_ms: 1000,
            },
        ]);
        assert_eq!(chapters[0].title, "Chapter One - The Worst Birthday");
        assert!((chapters[0].start_secs - 29.97).abs() < 0.001);
        assert_eq!(chapters[1].title, "Chapter 2");
    }
}

#[cfg(test)]
fn timestamp_touch_variables(id: &str) -> serde_json::Value {
    serde_json::json!({ "id": id, "input": {} })
}

/// Metadata provider enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataProvider {
    Tmdb,
    Tvmaze,
    Musicbrainz,
    OpenLibrary,
    /// Audible's catalogue, with chapters from Audnexus. Audiobooks only.
    Audible,
}

/// Unified movie search result (from TMDB search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieSearchResult {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    #[serde(default)]
    pub release_date: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub imdb_id: Option<String>,
    pub vote_average: Option<f64>,
    pub popularity: Option<f64>,
}

/// Unified movie collection search result (from TMDB search/collection)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieCollectionSearchResult {
    pub provider: MetadataProvider,
    pub collection_id: i32,
    pub name: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
}

/// Unified movie details (from TMDB get_movie)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieDetails {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub tmdb_status: Option<String>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub genres: Vec<String>,
    pub runtime: Option<i32>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub imdb_id: Option<String>,
    pub director: Option<String>,
    pub cast_names: Vec<String>,
    pub cast_members: Vec<MovieCastMemberDetails>,
    pub production_countries: Vec<String>,
    pub spoken_languages: Vec<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub certification: Option<String>,
    pub collection_id: Option<i32>,
    pub collection_name: Option<String>,
    pub collection_poster_url: Option<String>,
    pub release_date: Option<String>,
    /// Every credit and fact of the TMDB document (Q129–Q130). `None` in details cached before
    /// they were kept; those titles are filled by the credits job.
    #[serde(default)]
    pub extras: Option<super::credits::ProviderTitle>,
    /// Earliest digital and physical release dates (TMDB release types 4 and 5), `YYYY-MM-DD`.
    #[serde(default)]
    pub digital_release_date: Option<String>,
    #[serde(default)]
    pub physical_release_date: Option<String>,
}

/// Cast member details from provider credits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieCastMemberDetails {
    pub tmdb_person_id: i32,
    pub name: String,
    pub character_name: Option<String>,
    pub cast_order: Option<i32>,
    pub profile_url: Option<String>,
}

/// Unified TV show search result (from TVMaze search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvShowSearchResult {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub name: String,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub network: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub tvdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub score: Option<f64>,
}

/// Unified TV show details (from TVMaze get_show)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvShowDetails {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub name: String,
    pub sort_name: Option<String>,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub overview: Option<String>,
    pub network: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Vec<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub tvdb_id: Option<i32>,
    pub imdb_id: Option<String>,
}

/// The metadata cache's operation name for a show's episodes, shared with the artwork lookup.
pub const TV_SHOW_EPISODES_OPERATION: &str = "GetTvShowEpisodes";

/// One episode of a show that may not be in any library, from TVMaze.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvShowEpisodeDetails {
    pub id: u32,
    pub season: u32,
    pub number: u32,
    pub name: String,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub runtime: Option<u32>,
    pub still_url: Option<String>,
}

/// Unified album search result (from MusicBrainz search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumSearchResult {
    pub provider: MetadataProvider,
    pub provider_id: String,
    pub title: String,
    pub artist_name: Option<String>,
    pub year: Option<i32>,
    pub album_type: Option<String>,
    pub cover_url: Option<String>,
    pub score: Option<f64>,
}

/// Unified audiobook search result (from OpenLibrary search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudiobookSearchResult {
    pub provider: MetadataProvider,
    pub provider_id: String,
    pub title: String,
    pub author_name: Option<String>,
    pub year: Option<i32>,
    pub cover_url: Option<String>,
    pub isbn: Option<String>,
    pub description: Option<String>,
    /// How long the listing runs, which tells editions and same-named books apart.
    #[serde(default)]
    pub duration_secs: Option<i64>,
    #[serde(default)]
    pub narrator: Option<String>,
}

/// Unified audiobook details (from OpenLibrary work details)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudiobookDetails {
    pub provider: MetadataProvider,
    pub provider_id: String,
    pub title: String,
    pub sort_title: Option<String>,
    pub author_name: Option<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub isbn: Option<String>,
    pub published_date: Option<String>,
    pub cover_url: Option<String>,
    #[serde(default)]
    pub narrators: Vec<String>,
    #[serde(default)]
    pub total_duration_secs: Option<i64>,
    #[serde(default)]
    pub chapters: Vec<AudiobookChapterDetails>,
    #[serde(default)]
    pub series_name: Option<String>,
    #[serde(default)]
    pub series_position: Option<String>,
    /// Every author the provider lists (`authorName` keeps the first).
    #[serde(default)]
    pub authors: Vec<String>,
}

/// One chapter of an audiobook, where the provider publishes them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudiobookChapterDetails {
    pub title: String,
    pub start_secs: f64,
    pub length_secs: f64,
}

/// Options for adding a movie from a metadata provider
#[derive(Debug, Clone)]
pub struct AddMovieOptions {
    /// Metadata provider to use (should be Tmdb for movies)
    pub provider: MetadataProvider,
    /// Provider-specific ID (e.g., TMDB ID)
    pub provider_id: u32,
    /// Library to add the movie to
    pub library_id: Uuid,
    /// User who owns the library
    pub user_id: Uuid,
    /// Whether to monitor for releases (sets wanted = monitored)
    pub monitored: bool,
}

/// Options for importing a movie collection from a metadata provider
#[derive(Debug, Clone)]
pub struct AddMovieCollectionOptions {
    pub provider: MetadataProvider,
    pub collection_id: i32,
    pub library_id: Uuid,
    pub user_id: Uuid,
    /// If true, mark missing imported movies as wanted.
    pub wanted_missing: bool,
}

/// Result summary for importing a movie collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieCollectionImportSummary {
    pub collection_id: i32,
    pub collection_name: String,
    pub imported_count: i32,
    pub existing_count: i32,
    pub wanted_updated_count: i32,
}

/// A collection movie merged from TMDB and local library state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieCollectionMovieDetails {
    pub tmdb_id: i32,
    pub title: String,
    pub year: Option<i32>,
    pub poster_url: Option<String>,
    pub library_movie_id: Option<String>,
    pub media_file_id: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub resolution: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<String>,
    pub wanted: bool,
}

/// Detailed view of a movie collection with all TMDB parts and local overlay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieCollectionDetails {
    pub collection_id: i32,
    pub name: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub movies: Vec<MovieCollectionMovieDetails>,
}

#[derive(Debug, Clone)]
struct LocalCollectionMovieRow {
    id: String,
    tmdb_id: Option<i32>,
    collection_id: Option<i32>,
    title: String,
    year: Option<i32>,
    poster_url: Option<String>,
    media_file_id: Option<String>,
    file_size_bytes: Option<i64>,
    resolution: Option<String>,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    audio_channels: Option<String>,
    wanted: bool,
}

/// Options for adding a TV show from a metadata provider
#[derive(Debug, Clone)]
pub struct AddTvShowOptions {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub monitor_type: AutoDownloadMode,
    pub path: Option<String>,
}

/// A MusicBrainz release group that credits no artist, so no Artist record can be made for it.
#[derive(Debug)]
pub struct NoCreditedArtist {
    pub release_group_id: String,
    pub title: String,
}

impl std::fmt::Display for NoCreditedArtist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MusicBrainz release group {} ('{}') lists no artist",
            self.release_group_id, self.title
        )
    }
}

impl std::error::Error for NoCreditedArtist {}

/// Options for adding an album from a metadata provider
#[derive(Debug, Clone)]
pub struct AddAlbumOptions {
    pub provider: MetadataProvider,
    pub provider_id: String,
    pub library_id: Uuid,
    pub user_id: Uuid,
    /// Auto-download policy for the new album. `None` (the default) means the
    /// album is created unmonitored, exactly as before.
    pub monitor_type: AutoDownloadMode,
    /// The edition's tracks, when it's been chosen to fit some files. Otherwise the first official
    /// release's are used.
    pub listing: Option<Vec<crate::services::metadata::musicbrainz::MusicBrainzTrack>>,
}

/// Options for adding an audiobook from a metadata provider
#[derive(Debug, Clone)]
pub struct AddAudiobookOptions {
    pub provider: MetadataProvider,
    pub provider_id: String,
    pub library_id: Uuid,
    pub user_id: Uuid,
    /// Auto-download policy for the new audiobook.
    pub monitor_type: AutoDownloadMode,
}

/// GraphQL enum name for an [`AutoDownloadMode`], for use in JSON mutation
/// variables (the Rust `Display` impl is lowercase and not the schema name).
fn auto_download_mode_graphql_name(mode: AutoDownloadMode) -> &'static str {
    match mode {
        AutoDownloadMode::None => "NONE",
        AutoDownloadMode::All => "ALL",
        AutoDownloadMode::Wanted => "WANTED",
    }
}

/// Metadata service configuration
#[derive(Debug, Clone, Default)]
pub struct MetadataServiceConfig {
    pub tmdb_api_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetadataFetchPolicy {
    PreferCache,
    ForceRefresh,
}

/// Unified metadata service
pub struct MetadataService {
    /// TMDB client - wrapped in RwLock to allow dynamic reloading when settings change
    tmdb: RwLock<Option<TmdbClient>>,
    /// TVMaze client - no API key, but cached for reuse
    tvmaze: RwLock<Option<Arc<TvMazeClient>>>,
    /// MusicBrainz client - user agent may change via settings
    musicbrainz: RwLock<Option<Arc<MusicBrainzClient>>>,
    musicbrainz_user_agent: RwLock<Option<String>>,
    db: Database,
    services: Arc<ServicesManager>,
}

impl MetadataService {
    async fn artwork_service(&self) -> Option<crate::services::ArtworkService> {
        match self.services.get_storage().await {
            Some(storage) => Some(crate::services::ArtworkService::new(
                self.db.clone(),
                storage,
            )),
            None => {
                warn!("Object storage service unavailable; skipping artwork cache operation");
                None
            }
        }
    }

    fn normalize_title_for_collection_match(value: &str) -> String {
        value
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_lowercase()
                } else {
                    ' '
                }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn collection_match_score(
        local: &LocalCollectionMovieRow,
        tmdb_collection_id: i32,
        part_tmdb_id: i32,
        part_title: &str,
        part_year: Option<i32>,
    ) -> Option<i32> {
        if local.tmdb_id == Some(part_tmdb_id) {
            return Some(10_000);
        }

        let part_norm = Self::normalize_title_for_collection_match(part_title);
        let local_norm = Self::normalize_title_for_collection_match(&local.title);
        if part_norm.is_empty() || local_norm.is_empty() {
            return None;
        }

        let mut score = if local_norm == part_norm {
            80
        } else if local_norm.contains(&part_norm) || part_norm.contains(&local_norm) {
            65
        } else {
            return None;
        };

        match (local.year, part_year) {
            (Some(local_year), Some(target_year)) if local_year == target_year => score += 20,
            (Some(local_year), Some(target_year)) if (local_year - target_year).abs() == 1 => {
                score += 10;
            }
            (None, Some(_)) | (Some(_), None) => score += 4,
            _ => {}
        }

        if local.collection_id == Some(tmdb_collection_id) {
            score += 10;
        }
        if local.media_file_id.is_some() {
            score += 2;
        }

        if score >= 70 { Some(score) } else { None }
    }

    pub fn new(
        db: Database,
        services: Arc<ServicesManager>,
        config: MetadataServiceConfig,
    ) -> Self {
        let tmdb = config
            .tmdb_api_key
            .as_ref()
            .filter(|k| !k.is_empty())
            .map(|k| TmdbClient::new(k.clone()));

        Self {
            tmdb: RwLock::new(tmdb),
            tvmaze: RwLock::new(None),
            musicbrainz: RwLock::new(None),
            musicbrainz_user_agent: RwLock::new(None),
            db,
            services,
        }
    }

    async fn graphql_schema(&self) -> Result<LibrarianSchema> {
        let graphql = self
            .services
            .get_graphql()
            .await
            .ok_or_else(|| anyhow::anyhow!("GraphQL service not available"))?;
        graphql
            .schema()
            .await
            .ok_or_else(|| anyhow::anyhow!("GraphQL schema not available"))
    }

    async fn execute_query(
        &self,
        auth_user: &AuthUser,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let schema = self.graphql_schema().await?;
        let request = Request::new(query)
            .variables(Variables::from_json(variables))
            .data(auth_user.clone())
            .data(graphql_orm::graphql::auth::AuthSubject::from_parts(
                auth_user.user_id.clone(),
                auth_user.role.iter().cloned().collect(),
                Vec::new(),
                None,
            ))
            .data(auth_user.user_id.clone());
        let response = schema.execute(request).await;
        if !response.errors.is_empty() {
            let msg = response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(anyhow::anyhow!(msg));
        }
        Ok(serde_json::to_value(&response.data)?)
    }

    async fn execute_mutation(
        &self,
        auth_user: &AuthUser,
        mutation: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let data = self.execute_query(auth_user, mutation, variables).await?;
        check_mutation_payloads(&data)?;
        Ok(data)
    }

    /// Check the actual caller before entering trusted metadata orchestration.
    /// Imports belong to the target library's owner, including admin imports.
    pub async fn authorize_library_import(
        &self,
        caller: &AuthUser,
        library_id: Uuid,
    ) -> Result<Uuid> {
        if !caller.has_role(crate::services::graphql::auth::Role::Member) {
            anyhow::bail!("Library not found or access denied");
        }
        let data = self
            .execute_query(
                caller,
                "query ImportLibrary($id: String!) { library(id: $id) { userId } }",
                serde_json::json!({ "id": library_id.to_string() }),
            )
            .await?;
        let owner = data
            .get("library")
            .and_then(|library| library.get("userId"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Library not found or access denied"))?;
        Ok(Uuid::parse_str(owner)?)
    }

    /// Trusted callers must still supply the target library's owner. Check
    /// before provider calls, cache writes, or creation of an admin context.
    async fn require_import_owner(&self, library_id: Uuid, user_id: Uuid) -> Result<()> {
        let library = Library::get(self.db.pool(), &library_id.to_string()).await?;
        if library.is_none_or(|library| library.user_id != user_id.to_string()) {
            anyhow::bail!("Library not found or access denied");
        }
        Ok(())
    }

    async fn execute_internal_query(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let schema = self.graphql_schema().await?;
        let auth_user = AuthUser {
            user_id: "system-internal".to_string(),
            email: None,
            role: Some("admin".to_string()),
        };
        let request = Request::new(query)
            .variables(Variables::from_json(variables))
            .data(auth_user.clone())
            .data(graphql_orm::graphql::auth::AuthSubject::from_parts(
                auth_user.user_id.clone(),
                auth_user.role.iter().cloned().collect(),
                Vec::new(),
                None,
            ))
            .data(auth_user.user_id.clone());
        let response = schema.execute(request).await;
        if !response.errors.is_empty() {
            let msg = response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(anyhow::anyhow!(msg));
        }
        Ok(serde_json::to_value(&response.data)?)
    }

    async fn execute_internal_mutation(
        &self,
        mutation: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.execute_internal_query(mutation, variables).await
    }

    fn provider_cache_key(provider: MetadataProvider) -> &'static str {
        match provider {
            MetadataProvider::Tmdb => "tmdb",
            MetadataProvider::Tvmaze => "tvmaze",
            MetadataProvider::Musicbrainz => "musicbrainz",
            MetadataProvider::OpenLibrary => "openlibrary",
            MetadataProvider::Audible => "audible",
        }
    }

    fn normalized_cache_component(value: &str) -> String {
        value.trim().to_ascii_lowercase()
    }

    async fn provider_cache_days(&self, provider: MetadataProvider) -> i32 {
        match MetadataSettings::load(&self.db).await {
            Ok(settings) => match provider {
                MetadataProvider::Tmdb => settings.tmdb_cache_days,
                MetadataProvider::Tvmaze => settings.tvmaze_cache_days,
                MetadataProvider::Musicbrainz => settings.musicbrainz_cache_days,
                MetadataProvider::OpenLibrary => settings.openlibrary_cache_days,
                MetadataProvider::Audible => settings.audible_cache_days,
            },
            Err(error) => {
                warn!(
                    provider = Self::provider_cache_key(provider),
                    error = %error,
                    "Failed to load metadata settings for cache staleness; defaulting to 7 days"
                );
                7
            }
        }
    }

    fn parse_cached_timestamp(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(value)
            .map(|ts| ts.with_timezone(&chrono::Utc))
            .ok()
            .or_else(|| {
                chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .map(|ts| ts.and_utc())
            })
            .or_else(|| {
                value
                    .parse::<i64>()
                    .ok()
                    .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            })
    }

    fn is_cache_stale(fetched_at: &str, max_age_days: i32) -> bool {
        if max_age_days <= 0 {
            return true;
        }

        let Some(ts) = Self::parse_cached_timestamp(fetched_at) else {
            return true;
        };

        let now = chrono::Utc::now();
        let max_age = chrono::Duration::days(max_age_days as i64);
        now.signed_duration_since(ts) > max_age
    }

    async fn read_metadata_cache<T: DeserializeOwned>(
        &self,
        provider: MetadataProvider,
        operation: &str,
        cache_key: &str,
        max_age_days: i32,
    ) -> Result<Option<T>> {
        if max_age_days <= 0 {
            return Ok(None);
        }

        let data = self
            .execute_internal_query(
                r#"query MetadataCacheLookup($where: MetadataCacheWhereInput, $page: PageInput) {
                    MetadataCaches: metadataCaches(where: $where, page: $page) {
                        Edges: edges { Node: node { Payload: payload FetchedAt: fetchedAt } }
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "provider": { "eq": Self::provider_cache_key(provider) },
                        "operation": { "eq": operation },
                        "cacheKey": { "eq": cache_key }
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await?;

        let node = data
            .get("MetadataCaches")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"));

        let Some(node) = node else {
            return Ok(None);
        };

        let fetched_at = node
            .get("FetchedAt")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if Self::is_cache_stale(fetched_at, max_age_days) {
            return Ok(None);
        }

        let payload = node
            .get("Payload")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Metadata cache payload missing"))?;

        let parsed = serde_json::from_str::<T>(payload)?;
        Ok(Some(parsed))
    }

    async fn upsert_metadata_cache<T: Serialize>(
        &self,
        provider: MetadataProvider,
        operation: &str,
        cache_key: &str,
        payload: &T,
    ) -> Result<()> {
        let payload_json = serde_json::to_string(payload)?;
        let now = Self::now_iso_string();

        let existing = self
            .execute_internal_query(
                r#"query MetadataCacheExisting($where: MetadataCacheWhereInput, $page: PageInput) {
                    MetadataCaches: metadataCaches(where: $where, page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "provider": { "eq": Self::provider_cache_key(provider) },
                        "operation": { "eq": operation },
                        "cacheKey": { "eq": cache_key }
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await?;

        let existing_id = existing
            .get("MetadataCaches")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
            .map(|id| id.to_string());

        if let Some(cache_id) = existing_id {
            let _ = self
                .execute_internal_mutation(
                    r#"mutation UpdateMetadataCachePayload($id: String!, $input: UpdateMetadataCacheInput!) {
                        UpdateMetadataCache: updateMetadataCache(id: $id, input: $input) {
                            Success: success
                            Error: error
                        }
                    }"#,
                    serde_json::json!({
                        "id": cache_id,
                        "input": {
                            "payload": payload_json,
                            "payloadVersion": 1,
                            "fetchedAt": now,
                        }
                    }),
                )
                .await?;
            return Ok(());
        }

        let _ = self
            .execute_internal_mutation(
                r#"mutation CreateMetadataCachePayload($input: CreateMetadataCacheInput!) {
                    CreateMetadataCache: createMetadataCache(input: $input) {
                        Success: success
                        Error: error
                    }
                }"#,
                serde_json::json!({
                    "input": {
                        "provider": Self::provider_cache_key(provider),
                        "operation": operation,
                        "cacheKey": cache_key,
                        "payload": payload_json,
                        "payloadVersion": 1,
                        "fetchedAt": now,
                    }
                }),
            )
            .await?;
        Ok(())
    }

    /// Ensure a single collection entity exists and is up-to-date for a TMDB collection id.
    pub async fn ensure_movie_collection_entity(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        collection_id: i32,
    ) -> Result<()> {
        self.upsert_movie_collection_entity(library_id, user_id, collection_id)
            .await
    }

    /// Reconcile collection entities for a library by scanning movies with CollectionId and
    /// upserting each unique TMDB collection into the collections table.
    pub async fn ensure_movie_collections_for_library(
        &self,
        library_id: Uuid,
        user_id: Uuid,
    ) -> Result<()> {
        let auth_user = AuthUser {
            user_id: user_id.to_string(),
            email: None,
            role: Some("admin".to_string()),
        };

        let mut offset = 0usize;
        let limit = 500usize;
        let mut collection_ids: HashSet<i32> = HashSet::new();
        info!(
            library_id = %library_id,
            user_id = %user_id,
            "Starting library movie collection reconciliation: library_id={}, user_id={}, page_limit={}",
            library_id,
            user_id,
            limit
        );

        loop {
            let data = self
                .execute_query(
                    &auth_user,
                    r#"query LibraryMovieCollectionsForSync($where: MovieWhereInput, $page: PageInput) {
                        Movies: movies(where: $where, page: $page) {
                            Edges: edges {
                                Node: node {
                                    CollectionId: collectionId
                                }
                            }
                        }
                    }"#,
                    serde_json::json!({
                        "where": {
                            "libraryId": { "eq": library_id.to_string() }
                        },
                        "page": { "limit": limit, "offset": offset }
                    }),
                )
                .await?;

            let edges = data
                .get("Movies")
                .and_then(|v| v.get("Edges"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if edges.is_empty() {
                break;
            }

            for edge in &edges {
                if let Some(collection_id) = edge
                    .get("Node")
                    .and_then(|n| n.get("CollectionId"))
                    .and_then(|v| v.as_i64())
                {
                    collection_ids.insert(collection_id as i32);
                }
            }

            if edges.len() < limit {
                break;
            }
            offset += limit;
        }

        if collection_ids.is_empty() {
            debug!(
                library_id = %library_id,
                user_id = %user_id,
                "Skipping movie collection reconciliation because no movies in library have CollectionId values: library_id={}, user_id={}",
                library_id,
                user_id
            );
            return Ok(());
        }

        let mut existing_collection_ids: HashSet<i32> = HashSet::new();
        let mut existing_offset = 0usize;
        loop {
            let existing_data = self
                .execute_query(
                    &auth_user,
                    r#"query ExistingCollectionsForSync($where: CollectionWhereInput, $page: PageInput) {
                        Collections: collections(where: $where, page: $page) {
                            Edges: edges {
                                Node: node {
                                    TmdbCollectionId: tmdbCollectionId
                                }
                            }
                        }
                    }"#,
                    serde_json::json!({
                        "where": {
                            "libraryId": { "eq": library_id.to_string() }
                        },
                        "page": { "limit": limit, "offset": existing_offset }
                    }),
                )
                .await?;

            let existing_edges = existing_data
                .get("Collections")
                .and_then(|v| v.get("Edges"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if existing_edges.is_empty() {
                break;
            }

            for edge in &existing_edges {
                if let Some(tmdb_collection_id) = edge
                    .get("Node")
                    .and_then(|n| n.get("TmdbCollectionId"))
                    .and_then(|v| v.as_i64())
                {
                    existing_collection_ids.insert(tmdb_collection_id as i32);
                }
            }

            if existing_edges.len() < limit {
                break;
            }
            existing_offset += limit;
        }

        let missing_collection_ids: Vec<i32> = collection_ids
            .difference(&existing_collection_ids)
            .copied()
            .collect();

        if missing_collection_ids.is_empty() {
            debug!(
                library_id = %library_id,
                user_id = %user_id,
                discovered_collection_ids = collection_ids.len(),
                existing_collection_ids = existing_collection_ids.len(),
                "Skipping TMDB collection reconciliation because all movie collections already exist locally: library_id={}, user_id={}, discovered_collection_ids={}, existing_collection_ids={}",
                library_id,
                user_id,
                collection_ids.len(),
                existing_collection_ids.len()
            );
            return Ok(());
        }

        let discovered_total = collection_ids.len();
        let existing_total = existing_collection_ids.len();
        let missing_total = missing_collection_ids.len();
        let mut synced = 0usize;
        for collection_id in missing_collection_ids {
            match self
                .ensure_movie_collection_entity(library_id, user_id, collection_id)
                .await
            {
                Ok(()) => synced += 1,
                Err(error) => {
                    warn!(
                        library_id = %library_id,
                        user_id = %user_id,
                        collection_id = collection_id,
                        error = %error,
                        "Failed to upsert collection during library reconciliation: library_id={}, user_id={}, collection_id={}, error={}",
                        library_id,
                        user_id,
                        collection_id,
                        error
                    );
                }
            }
        }
        let failed = missing_total.saturating_sub(synced);

        info!(
            library_id = %library_id,
            user_id = %user_id,
            discovered_collections = discovered_total,
            existing_collections = existing_total,
            missing_collections = missing_total,
            synced_collections = synced,
            failed_collections = failed,
            "Completed library movie collection reconciliation: library_id={}, user_id={}, discovered_collections={}, existing_collections={}, missing_collections={}, synced_collections={}, failed_collections={}",
            library_id,
            user_id,
            discovered_total,
            existing_total,
            missing_total,
            synced,
            failed
        );

        Ok(())
    }

    fn now_iso_string() -> String {
        chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string()
    }

    /// Create with default config (no API keys)
    pub fn new_default(db: Database, services: Arc<ServicesManager>) -> Self {
        Self::new(db, services, MetadataServiceConfig::default())
    }

    /// Check if TMDB is configured by checking the database for an API key
    pub async fn has_tmdb(&self) -> bool {
        // First check cached client
        if self.tmdb.read().await.is_some() {
            return true;
        }

        // Query only the TMDB key directly - don't load all settings
        // (MetadataSettings::load can fail if other settings have JSON parse issues,
        //  which would silently make this return false)
        match crate::services::torrent::database::get_setting_string(
            &self.db,
            "metadata.tmdb_api_key",
        )
        .await
        {
            Ok(key) => key.is_some(),
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to check TMDB API key from database key 'metadata.tmdb_api_key': error={}",
                    e
                );
                false
            }
        }
    }

    /// Get TMDB client, always checking the database for the latest API key
    ///
    /// This ensures that if the user updates the API key via settings,
    /// the next request will use the new key without requiring a server restart.
    pub(crate) async fn get_tmdb_client(&self) -> Result<TmdbClient> {
        let settings = MetadataSettings::load(&self.db).await?;
        let Some(key) = settings.tmdb_api_key else {
            anyhow::bail!("TMDB API key not configured. Add tmdb_api_key to settings.")
        };

        // Check if we have a cached client with the same key
        {
            let guard = self.tmdb.read().await;
            if let Some(ref client) = *guard {
                if client.api_key() == key {
                    return Ok(client.clone());
                }
                // Key changed, will create new client below
                debug!("TMDB API key changed, creating new client");
            }
        }

        // Create new client with current key
        info!(
            "Creating TMDB client with API key loaded from app_settings key 'metadata.tmdb_api_key'"
        );
        let client = TmdbClient::new(key);

        // Cache the client
        let mut guard = self.tmdb.write().await;
        *guard = Some(client.clone());

        Ok(client)
    }

    /// Get TVMaze client (no API key required)
    pub async fn get_tvmaze_client(&self) -> Result<Arc<TvMazeClient>> {
        if let Some(client) = self.tvmaze.read().await.as_ref() {
            return Ok(client.clone());
        }

        let client = Arc::new(TvMazeClient::new());
        let mut guard = self.tvmaze.write().await;
        *guard = Some(client.clone());
        Ok(client)
    }

    /// A release group's front cover from Cover Art Archive, remembered in the metadata cache
    /// so an album removed and added again (or a second library holding it) doesn't depend on
    /// the archive answering a second time. Only found covers are remembered.
    pub async fn album_cover(&self, release_group_id: Uuid) -> Result<Option<String>> {
        let cache_key = release_group_id.to_string();
        let max_age_days = self
            .provider_cache_days(MetadataProvider::Musicbrainz)
            .await;
        if let Ok(Some(cached)) = self
            .read_metadata_cache::<String>(
                MetadataProvider::Musicbrainz,
                "GetCoverArt",
                &cache_key,
                max_age_days,
            )
            .await
        {
            return Ok(Some(cached));
        }
        let cover = self
            .get_musicbrainz_client()
            .await?
            .get_cover_art(release_group_id)
            .await?;
        if let Some(cover) = &cover
            && let Err(error) = self
                .upsert_metadata_cache(
                    MetadataProvider::Musicbrainz,
                    "GetCoverArt",
                    &cache_key,
                    cover,
                )
                .await
        {
            debug!(release_group_id = %release_group_id, error = %error, "Could not remember an album cover");
        }
        Ok(cover)
    }

    /// An artist's picture: Wikimedia Commons through the artist's MusicBrainz id and Wikidata,
    /// else Deezer's picture for an artist of exactly the same name. Found pictures are
    /// remembered in the metadata cache (by id, and by name for Deezer) so an artist removed and
    /// added again doesn't ask the sites a second time; nothing is remembered about a miss.
    /// `Err` means a site couldn't be asked and nothing else found a picture, so a later try may
    /// do better.
    pub async fn artist_image(
        &self,
        musicbrainz_id: Option<Uuid>,
        artist_name: &str,
    ) -> Result<Option<String>> {
        use super::artist_image::{
            IMAGE_BY_MBID_OPERATION, IMAGE_BY_NAME_OPERATION, deezer_image, name_cache_key,
            wikidata_image,
        };
        let max_age_days = self
            .provider_cache_days(MetadataProvider::Musicbrainz)
            .await;
        let id_key = musicbrainz_id.map(|id| id.to_string());
        let name_key = name_cache_key(artist_name);
        if let Some(id_key) = &id_key
            && let Ok(Some(cached)) = self
                .read_metadata_cache::<String>(
                    MetadataProvider::Musicbrainz,
                    IMAGE_BY_MBID_OPERATION,
                    id_key,
                    max_age_days,
                )
                .await
        {
            return Ok(Some(cached));
        }

        let musicbrainz = self.get_musicbrainz_client().await?;
        let user_agent = musicbrainz.user_agent().to_string();
        let mut failure: Option<anyhow::Error> = None;

        if let (Some(id), Some(id_key)) = (musicbrainz_id, &id_key) {
            let from_wikidata = async {
                match musicbrainz.get_artist_wikidata_id(id).await? {
                    Some(item) => wikidata_image(&user_agent, &item).await,
                    None => Ok(None),
                }
            }
            .await;
            match from_wikidata {
                Ok(Some(image)) => {
                    info!(
                        artist = %artist_name,
                        musicbrainz_id = %id,
                        image_url = %image,
                        "Found a Wikimedia Commons picture for artist '{}' (MusicBrainz {})",
                        artist_name,
                        id
                    );
                    self.remember_artist_image(
                        IMAGE_BY_MBID_OPERATION,
                        id_key,
                        &image,
                        artist_name,
                    )
                    .await;
                    return Ok(Some(image));
                }
                Ok(None) => debug!(
                    artist = %artist_name,
                    musicbrainz_id = %id,
                    "No Wikidata picture for artist '{}' (MusicBrainz {}); trying Deezer",
                    artist_name,
                    id
                ),
                Err(error) => {
                    debug!(
                        artist = %artist_name,
                        musicbrainz_id = %id,
                        error = %error,
                        "Could not look up the Wikidata picture for artist '{}' (MusicBrainz {}); trying Deezer",
                        artist_name,
                        id
                    );
                    failure = Some(error);
                }
            }
        }

        if !name_key.is_empty() {
            if let Ok(Some(cached)) = self
                .read_metadata_cache::<String>(
                    MetadataProvider::Musicbrainz,
                    IMAGE_BY_NAME_OPERATION,
                    &name_key,
                    max_age_days,
                )
                .await
            {
                return Ok(Some(cached));
            }
            match deezer_image(&user_agent, artist_name).await {
                Ok(Some(image)) => {
                    self.remember_artist_image(
                        IMAGE_BY_NAME_OPERATION,
                        &name_key,
                        &image,
                        artist_name,
                    )
                    .await;
                    // A lookup that couldn't reach Wikidata shouldn't pin this picture to the id
                    // and keep Wikidata from being asked again.
                    if failure.is_none()
                        && let Some(id_key) = &id_key
                    {
                        self.remember_artist_image(
                            IMAGE_BY_MBID_OPERATION,
                            id_key,
                            &image,
                            artist_name,
                        )
                        .await;
                    }
                    return Ok(Some(image));
                }
                Ok(None) => {}
                Err(error) => failure = Some(failure.unwrap_or(error)),
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(None),
        }
    }

    async fn remember_artist_image(
        &self,
        operation: &str,
        cache_key: &str,
        image: &String,
        artist_name: &str,
    ) {
        if let Err(error) = self
            .upsert_metadata_cache(MetadataProvider::Musicbrainz, operation, cache_key, image)
            .await
        {
            debug!(artist = %artist_name, error = %error, "Could not remember a picture for artist '{}'", artist_name);
        }
    }

    /// Get MusicBrainz client, honoring the user-agent setting
    pub async fn get_musicbrainz_client(&self) -> Result<Arc<MusicBrainzClient>> {
        let settings = MetadataSettings::load(&self.db).await?;
        let default_user_agent = MusicBrainzClient::default_user_agent();
        let desired_user_agent = settings
            .musicbrainz_user_agent
            .filter(|ua| !ua.trim().is_empty())
            .unwrap_or(default_user_agent);

        {
            let client_guard = self.musicbrainz.read().await;
            let ua_guard = self.musicbrainz_user_agent.read().await;
            if let (Some(client), Some(current_ua)) = (&*client_guard, &*ua_guard)
                && current_ua == &desired_user_agent
            {
                return Ok(client.clone());
            }
        }

        let client = Arc::new(MusicBrainzClient::new_with_user_agent(
            desired_user_agent.clone(),
        ));
        let mut client_guard = self.musicbrainz.write().await;
        let mut ua_guard = self.musicbrainz_user_agent.write().await;
        *client_guard = Some(client.clone());
        *ua_guard = Some(desired_user_agent);
        Ok(client)
    }

    /// Search for movies on TMDB
    pub async fn search_movies(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<MovieSearchResult>> {
        self.search_movies_with_policy(query, year, MetadataFetchPolicy::PreferCache)
            .await
    }

    async fn search_movies_with_policy(
        &self,
        query: &str,
        year: Option<i32>,
        policy: MetadataFetchPolicy,
    ) -> Result<Vec<MovieSearchResult>> {
        info!(
            "Metadata search requested for movie query='{}'{}",
            query,
            year.map(|y| format!(" ({})", y)).unwrap_or_default()
        );

        let cache_key = format!(
            "query={};year={}",
            Self::normalized_cache_component(query),
            year.map(|v| v.to_string())
                .unwrap_or_else(|| "none".to_string())
        );
        if policy == MetadataFetchPolicy::PreferCache {
            let max_age_days = self.provider_cache_days(MetadataProvider::Tmdb).await;
            if let Some(cached) = self
                .read_metadata_cache::<Vec<MovieSearchResult>>(
                    MetadataProvider::Tmdb,
                    "SearchMovies",
                    &cache_key,
                    max_age_days,
                )
                .await?
            {
                return Ok(cached);
            }
        }

        let tmdb = self.get_tmdb_client().await?;
        let movies = tmdb.search_movies(query, year).await?;

        let results: Vec<MovieSearchResult> = movies
            .into_iter()
            .map(|m| {
                // Compute year before moving fields
                let year = m.year();
                let poster_url = tmdb.poster_url(m.poster_path.as_deref());
                let backdrop_url = tmdb.backdrop_url(m.backdrop_path.as_deref());

                MovieSearchResult {
                    provider: MetadataProvider::Tmdb,
                    provider_id: m.id as u32,
                    title: m.title,
                    original_title: m.original_title,
                    year,
                    release_date: m.release_date,
                    overview: m.overview,
                    poster_url,
                    backdrop_url,
                    imdb_id: m.imdb_id,
                    vote_average: m.vote_average,
                    popularity: m.popularity,
                }
            })
            .collect();

        if let Err(error) = self
            .upsert_metadata_cache(MetadataProvider::Tmdb, "SearchMovies", &cache_key, &results)
            .await
        {
            warn!(error = %error, "Failed to update metadata cache for SearchMovies");
        }

        debug!(count = results.len(), "Found movies");
        Ok(results)
    }

    /// Region and page are part of the cache key; release lists refresh daily.
    pub async fn movie_releases(
        &self,
        kind: super::tmdb::MovieReleaseKind,
        region: Option<String>,
        page: i32,
    ) -> Result<Vec<MovieSearchResult>> {
        let region = region.map(|value| value.trim().to_ascii_uppercase());
        if region
            .as_ref()
            .is_some_and(|value| value.len() != 2 || !value.bytes().all(|c| c.is_ascii_uppercase()))
        {
            anyhow::bail!("Region must be a two-letter ISO country code");
        }
        if !(1..=500).contains(&page) {
            anyhow::bail!("Page must be between 1 and 500");
        }
        let key = format!(
            "kind={};region={};page={page}",
            kind.endpoint(),
            region.as_deref().unwrap_or("global")
        );
        if let Some(cached) = self
            .read_metadata_cache(MetadataProvider::Tmdb, "MovieReleases", &key, 1)
            .await?
        {
            return Ok(cached);
        }
        let tmdb = self.get_tmdb_client().await?;
        let movies = tmdb.movie_releases(kind, region.as_deref(), page).await?;
        let results: Vec<MovieSearchResult> = movies
            .into_iter()
            .map(|movie| {
                let year = movie.year();
                let poster_url = tmdb.poster_url(movie.poster_path.as_deref());
                let backdrop_url = tmdb.backdrop_url(movie.backdrop_path.as_deref());
                MovieSearchResult {
                    provider: MetadataProvider::Tmdb,
                    provider_id: movie.id as u32,
                    title: movie.title,
                    original_title: movie.original_title,
                    year,
                    release_date: movie.release_date,
                    overview: movie.overview,
                    poster_url,
                    backdrop_url,
                    imdb_id: movie.imdb_id,
                    vote_average: movie.vote_average,
                    popularity: movie.popularity,
                }
            })
            .collect();
        self.upsert_metadata_cache(MetadataProvider::Tmdb, "MovieReleases", &key, &results)
            .await?;
        Ok(results)
    }

    /// Search for movie collections on TMDB
    pub async fn search_movie_collections(
        &self,
        query: &str,
    ) -> Result<Vec<MovieCollectionSearchResult>> {
        self.search_movie_collections_with_policy(query, MetadataFetchPolicy::PreferCache)
            .await
    }

    async fn search_movie_collections_with_policy(
        &self,
        query: &str,
        policy: MetadataFetchPolicy,
    ) -> Result<Vec<MovieCollectionSearchResult>> {
        info!(
            "Metadata search requested for collections query='{}'",
            query
        );

        let cache_key = format!("query={}", Self::normalized_cache_component(query));
        if policy == MetadataFetchPolicy::PreferCache {
            let max_age_days = self.provider_cache_days(MetadataProvider::Tmdb).await;
            if let Some(cached) = self
                .read_metadata_cache::<Vec<MovieCollectionSearchResult>>(
                    MetadataProvider::Tmdb,
                    "SearchMovieCollections",
                    &cache_key,
                    max_age_days,
                )
                .await?
            {
                return Ok(cached);
            }
        }

        let tmdb = self.get_tmdb_client().await?;
        let collections = tmdb.search_collections(query).await?;

        let results = collections
            .into_iter()
            .map(|c| MovieCollectionSearchResult {
                provider: MetadataProvider::Tmdb,
                collection_id: c.id,
                name: c.name,
                overview: c.overview,
                poster_url: tmdb.poster_url(c.poster_path.as_deref()),
                backdrop_url: tmdb.backdrop_url(c.backdrop_path.as_deref()),
            })
            .collect::<Vec<_>>();

        if let Err(error) = self
            .upsert_metadata_cache(
                MetadataProvider::Tmdb,
                "SearchMovieCollections",
                &cache_key,
                &results,
            )
            .await
        {
            warn!(
                error = %error,
                "Failed to update metadata cache for SearchMovieCollections"
            );
        }

        debug!(count = results.len(), "Found collections");
        Ok(results)
    }

    /// Search for TV shows on TVMaze
    pub async fn search_tv_shows(&self, query: &str) -> Result<Vec<TvShowSearchResult>> {
        self.search_tv_shows_with_policy(query, MetadataFetchPolicy::PreferCache)
            .await
    }

    async fn search_tv_shows_with_policy(
        &self,
        query: &str,
        policy: MetadataFetchPolicy,
    ) -> Result<Vec<TvShowSearchResult>> {
        info!("Metadata search requested for TV shows query='{}'", query);

        let cache_key = format!("query={}", Self::normalized_cache_component(query));
        if policy == MetadataFetchPolicy::PreferCache {
            let max_age_days = self.provider_cache_days(MetadataProvider::Tvmaze).await;
            if let Some(cached) = self
                .read_metadata_cache::<Vec<TvShowSearchResult>>(
                    MetadataProvider::Tvmaze,
                    "SearchTvShows",
                    &cache_key,
                    max_age_days,
                )
                .await?
            {
                return Ok(cached);
            }
        }

        let tvmaze = self.get_tvmaze_client().await?;
        let results = tvmaze.search_shows(query).await?;

        let shows: Vec<TvShowSearchResult> = results
            .into_iter()
            .map(|r| {
                let show = r.show;
                let network = show
                    .network
                    .as_ref()
                    .map(|n| n.name.clone())
                    .or_else(|| show.web_channel.as_ref().map(|c| c.name.clone()));
                let overview = show.clean_summary();
                TvShowSearchResult {
                    provider: MetadataProvider::Tvmaze,
                    provider_id: show.id,
                    name: show.name.clone(),
                    year: show
                        .premiered
                        .as_ref()
                        .and_then(|d| d.get(0..4))
                        .and_then(|y| y.parse().ok()),
                    status: show.status.clone(),
                    network,
                    overview,
                    poster_url: show.image.as_ref().and_then(|i| i.medium.clone()),
                    tvdb_id: show
                        .externals
                        .as_ref()
                        .and_then(|e| e.thetvdb)
                        .map(|v| v as i32),
                    imdb_id: show.externals.as_ref().and_then(|e| e.imdb.clone()),
                    score: Some(r.score),
                }
            })
            .collect();

        if let Err(error) = self
            .upsert_metadata_cache(
                MetadataProvider::Tvmaze,
                "SearchTvShows",
                &cache_key,
                &shows,
            )
            .await
        {
            warn!(error = %error, "Failed to update metadata cache for SearchTvShows");
        }

        debug!(count = shows.len(), "Found TV shows");
        Ok(shows)
    }

    /// Search for albums on MusicBrainz
    pub async fn search_albums(
        &self,
        query: &str,
        include_eps: bool,
        include_singles: bool,
        include_compilations: bool,
        include_live: bool,
        include_soundtracks: bool,
    ) -> Result<Vec<AlbumSearchResult>> {
        self.search_albums_with_policy(
            query,
            include_eps,
            include_singles,
            include_compilations,
            include_live,
            include_soundtracks,
            MetadataFetchPolicy::PreferCache,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn search_albums_with_policy(
        &self,
        query: &str,
        include_eps: bool,
        include_singles: bool,
        include_compilations: bool,
        include_live: bool,
        include_soundtracks: bool,
        policy: MetadataFetchPolicy,
    ) -> Result<Vec<AlbumSearchResult>> {
        let cache_key = format!(
            "query={};eps={};singles={};compilations={};live={};soundtracks={}",
            Self::normalized_cache_component(query),
            include_eps,
            include_singles,
            include_compilations,
            include_live,
            include_soundtracks
        );
        if policy == MetadataFetchPolicy::PreferCache {
            let max_age_days = self
                .provider_cache_days(MetadataProvider::Musicbrainz)
                .await;
            if let Some(cached) = self
                .read_metadata_cache::<Vec<AlbumSearchResult>>(
                    MetadataProvider::Musicbrainz,
                    "SearchAlbums",
                    &cache_key,
                    max_age_days,
                )
                .await?
            {
                return Ok(cached);
            }
        }

        let musicbrainz = self.get_musicbrainz_client().await?;

        let mut types = vec!["Album".to_string()];
        if include_eps {
            types.push("EP".to_string());
        }
        if include_singles {
            types.push("Single".to_string());
        }
        if include_compilations {
            types.push("Compilation".to_string());
        }
        if include_live {
            types.push("Live".to_string());
        }
        if include_soundtracks {
            types.push("Soundtrack".to_string());
        }

        let albums = musicbrainz.search_albums_with_types(query, &types).await?;

        let results = albums
            .into_iter()
            .map(|album| {
                let provider_id = album.id.to_string();
                let title = album.title.clone();
                let artist_name = album.artist_names();
                let year = album.year();
                let album_type = Some(album.normalized_type());
                let score = album.score.map(|v| v as f64);
                // Use CAA thumbnail endpoint directly for search results.
                // It redirects when artwork exists and keeps search lightweight.
                let cover_url = Some(format!(
                    "https://coverartarchive.org/release-group/{}/front-250",
                    provider_id
                ));

                AlbumSearchResult {
                    provider: MetadataProvider::Musicbrainz,
                    provider_id,
                    title,
                    artist_name,
                    year,
                    album_type,
                    cover_url,
                    score,
                }
            })
            .collect::<Vec<_>>();

        if let Err(error) = self
            .upsert_metadata_cache(
                MetadataProvider::Musicbrainz,
                "SearchAlbums",
                &cache_key,
                &results,
            )
            .await
        {
            warn!(error = %error, "Failed to update metadata cache for SearchAlbums");
        }

        Ok(results)
    }

    /// Search for audiobooks on OpenLibrary
    pub async fn search_audiobooks(&self, query: &str) -> Result<Vec<AudiobookSearchResult>> {
        self.search_audiobooks_with_policy(query, MetadataFetchPolicy::PreferCache)
            .await
    }

    /// Audible first: it only lists audiobooks, and has chapters and narrators. Open Library, which
    /// lists every book ever printed, is what's left if Audible is unreachable or has nothing.
    async fn search_audiobooks_with_policy(
        &self,
        query: &str,
        policy: MetadataFetchPolicy,
    ) -> Result<Vec<AudiobookSearchResult>> {
        let settings = MetadataSettings::load(&self.db).await.ok();
        if settings.as_ref().is_none_or(|s| s.audible_enabled) {
            match self.search_audible(query, policy).await {
                Ok(results) if !results.is_empty() => return Ok(results),
                Ok(_) => {}
                Err(error) => {
                    warn!(error = %error, "Audible search failed; falling back to Open Library");
                }
            }
        }
        if settings.as_ref().is_some_and(|s| !s.openlibrary_enabled) {
            return Ok(Vec::new());
        }
        self.search_open_library_audiobooks(query, policy).await
    }

    async fn search_audible(
        &self,
        query: &str,
        policy: MetadataFetchPolicy,
    ) -> Result<Vec<AudiobookSearchResult>> {
        let cache_key = format!("query={}", Self::normalized_cache_component(query));
        if policy == MetadataFetchPolicy::PreferCache {
            let max_age_days = self.provider_cache_days(MetadataProvider::Audible).await;
            if let Some(cached) = self
                .read_metadata_cache::<Vec<AudiobookSearchResult>>(
                    MetadataProvider::Audible,
                    "SearchAudiobookListings",
                    &cache_key,
                    max_age_days,
                )
                .await?
            {
                return Ok(cached);
            }
        }
        let client = crate::services::http_client::outbound_client(
            crate::services::http_client::OutboundHttpProfile::Metadata,
        )?;
        let response = client
            .get(format!("{AUDIBLE_API}/catalog/products"))
            .query(&[
                ("keywords", query),
                ("num_results", "40"),
                ("products_sort_by", "Relevance"),
                ("response_groups", AUDIBLE_RESPONSE_GROUPS),
                ("image_sizes", "500,1000"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<AudibleSearchResponse>()
            .await?;
        let results = audible_search_results(response.products);
        if let Err(error) = self
            .upsert_metadata_cache(
                MetadataProvider::Audible,
                "SearchAudiobookListings",
                &cache_key,
                &results,
            )
            .await
        {
            warn!(error = %error, "Failed to update metadata cache for Audible search");
        }
        Ok(results)
    }

    async fn search_open_library_audiobooks(
        &self,
        query: &str,
        policy: MetadataFetchPolicy,
    ) -> Result<Vec<AudiobookSearchResult>> {
        #[derive(Debug, Deserialize)]
        struct OpenLibraryDoc {
            key: Option<String>,
            title: Option<String>,
            author_name: Option<Vec<String>>,
            first_publish_year: Option<i32>,
            cover_i: Option<i64>,
            isbn: Option<Vec<String>>,
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibrarySearchResponse {
            docs: Vec<OpenLibraryDoc>,
        }

        let cache_key = format!("query={}", Self::normalized_cache_component(query));
        if policy == MetadataFetchPolicy::PreferCache {
            let max_age_days = self
                .provider_cache_days(MetadataProvider::OpenLibrary)
                .await;
            if let Some(cached) = self
                .read_metadata_cache::<Vec<AudiobookSearchResult>>(
                    MetadataProvider::OpenLibrary,
                    "SearchAudiobooks",
                    &cache_key,
                    max_age_days,
                )
                .await?
            {
                return Ok(cached);
            }
        }

        let client = crate::services::http_client::outbound_client(
            crate::services::http_client::OutboundHttpProfile::Metadata,
        )?;
        let response = client
            .get("https://openlibrary.org/search.json")
            .query(&[("q", query)])
            .send()
            .await?
            .error_for_status()?
            .json::<OpenLibrarySearchResponse>()
            .await?;

        let results = response
            .docs
            .into_iter()
            .filter_map(|doc| {
                let key = doc.key?;
                let provider_id = key
                    .strip_prefix("/works/")
                    .unwrap_or(key.as_str())
                    .to_string();
                let title = doc.title?;
                let cover_url = doc
                    .cover_i
                    .map(|id| format!("https://covers.openlibrary.org/b/id/{}-L.jpg", id));
                let author_name = doc.author_name.and_then(|v| v.first().cloned());
                let isbn = doc.isbn.and_then(|v| v.first().cloned());

                Some(AudiobookSearchResult {
                    provider: MetadataProvider::OpenLibrary,
                    provider_id,
                    title,
                    author_name,
                    year: doc.first_publish_year,
                    cover_url,
                    isbn,
                    description: None,
                    duration_secs: None,
                    narrator: None,
                })
            })
            .collect::<Vec<_>>();

        if let Err(error) = self
            .upsert_metadata_cache(
                MetadataProvider::OpenLibrary,
                "SearchAudiobooks",
                &cache_key,
                &results,
            )
            .await
        {
            warn!(
                error = %error,
                "Failed to update metadata cache for SearchAudiobooks"
            );
        }

        Ok(results)
    }

    /// Get movie details from TMDB
    pub async fn get_movie(&self, tmdb_id: u32) -> Result<MovieDetails> {
        self.get_movie_with_policy(tmdb_id, MetadataFetchPolicy::PreferCache)
            .await
    }

    async fn get_movie_with_policy(
        &self,
        tmdb_id: u32,
        policy: MetadataFetchPolicy,
    ) -> Result<MovieDetails> {
        debug!("Fetching movie details from TMDB (ID: {})", tmdb_id);

        let cache_key = tmdb_id.to_string();
        if policy == MetadataFetchPolicy::PreferCache {
            let max_age_days = self.provider_cache_days(MetadataProvider::Tmdb).await;
            if let Some(cached) = self
                .read_metadata_cache::<MovieDetails>(
                    MetadataProvider::Tmdb,
                    "GetMovie",
                    &cache_key,
                    max_age_days,
                )
                .await?
            {
                return Ok(cached);
            }
        }

        let tmdb = self.get_tmdb_client().await?;

        // One document with everything TMDB has on the film (Q130): the details, credits,
        // certifications, keywords, alternative titles, videos and external ids.
        let document = tmdb
            .get_value(
                &format!("/movie/{tmdb_id}"),
                &[("append_to_response", MOVIE_APPEND)],
            )
            .await?
            .ok_or_else(|| anyhow::anyhow!("Movie not found on TMDB"))?;
        let movie: super::tmdb::TmdbMovie = serde_json::from_value(document.clone())
            .map_err(|error| anyhow::anyhow!("Failed to parse TMDB movie {tmdb_id}: {error}"))?;
        let extras = Some(super::credits::parse_tmdb_movie(&document));

        // Credits for director and cast
        let credits: Option<super::tmdb::TmdbCredits> = document
            .get("credits")
            .and_then(|credits| serde_json::from_value(credits.clone()).ok());
        let director = credits.as_ref().and_then(|c| c.director());
        let cast_names = credits.as_ref().map(|c| c.top_cast(10)).unwrap_or_default();
        let cast_members = credits
            .as_ref()
            .map(|c| {
                c.cast
                    .iter()
                    .take(20)
                    .cloned()
                    .map(|member| MovieCastMemberDetails {
                        tmdb_person_id: member.id,
                        name: member.name,
                        character_name: member.character,
                        cast_order: member.order,
                        profile_url: tmdb.original_url(member.profile_path.as_deref()),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Release dates for certification
        let release_dates: Option<super::tmdb::TmdbReleaseDates> = document
            .get("release_dates")
            .and_then(|dates| serde_json::from_value(dates.clone()).ok());
        let certification = release_dates.as_ref().and_then(|r| r.us_certification());
        let digital_release_date = release_dates
            .as_ref()
            .and_then(|r| r.earliest_of_type(super::tmdb::RELEASE_TYPE_DIGITAL));
        let physical_release_date = release_dates
            .as_ref()
            .and_then(|r| r.earliest_of_type(super::tmdb::RELEASE_TYPE_PHYSICAL));

        // Build collection info if available
        let (collection_id, collection_name, collection_poster_url) =
            if let Some(ref collection) = movie.belongs_to_collection {
                (
                    Some(collection.id),
                    Some(collection.name.clone()),
                    tmdb.poster_url(collection.poster_path.as_deref()),
                )
            } else {
                (None, None, None)
            };

        let details = MovieDetails {
            provider: MetadataProvider::Tmdb,
            provider_id: tmdb_id,
            title: movie.title.clone(),
            original_title: movie.original_title.clone(),
            year: movie.year(),
            tmdb_status: super::tmdb::normalize_movie_status(movie.status.as_deref()),
            overview: movie.overview.clone(),
            tagline: movie.tagline.clone(),
            genres: movie.genre_names(),
            runtime: movie.runtime,
            poster_url: tmdb.original_url(movie.poster_path.as_deref()),
            backdrop_url: tmdb.original_url(movie.backdrop_path.as_deref()),
            imdb_id: movie.imdb_id.clone(),
            director,
            cast_names,
            cast_members,
            production_countries: movie.country_codes(),
            spoken_languages: movie.language_codes(),
            vote_average: movie.vote_average,
            vote_count: movie.vote_count,
            certification,
            collection_id,
            collection_name,
            collection_poster_url,
            release_date: movie.release_date.clone(),
            extras,
            digital_release_date,
            physical_release_date,
        };

        if let Err(error) = self
            .upsert_metadata_cache(MetadataProvider::Tmdb, "GetMovie", &cache_key, &details)
            .await
        {
            warn!(error = %error, "Failed to update metadata cache for GetMovie");
        }

        Ok(details)
    }

    /// Get TV show details from TVMaze
    pub async fn get_tv_show(&self, tvmaze_id: u32) -> Result<TvShowDetails> {
        self.get_tv_show_with_policy(tvmaze_id, MetadataFetchPolicy::PreferCache)
            .await
    }

    async fn get_tv_show_with_policy(
        &self,
        tvmaze_id: u32,
        policy: MetadataFetchPolicy,
    ) -> Result<TvShowDetails> {
        debug!("Fetching TV show details from TVMaze (ID: {})", tvmaze_id);

        let cache_key = tvmaze_id.to_string();
        if policy == MetadataFetchPolicy::PreferCache {
            let max_age_days = self.provider_cache_days(MetadataProvider::Tvmaze).await;
            if let Some(cached) = self
                .read_metadata_cache::<TvShowDetails>(
                    MetadataProvider::Tvmaze,
                    "GetTvShow",
                    &cache_key,
                    max_age_days,
                )
                .await?
            {
                return Ok(cached);
            }
        }

        let tvmaze = self.get_tvmaze_client().await?;
        let show = tvmaze.get_show(tvmaze_id).await?;
        // The show resource carries only a poster; wide backgrounds live in its image list.
        let backdrop_url = match tvmaze.get_show_images(tvmaze_id).await {
            Ok(images) => super::tvmaze::best_background(&images),
            Err(error) => {
                warn!(tvmaze_id, error = %error, "Failed to fetch TVMaze show images; keeping the show without a backdrop");
                None
            }
        };

        let network = show
            .network
            .as_ref()
            .map(|n| n.name.clone())
            .or_else(|| show.web_channel.as_ref().map(|c| c.name.clone()));

        let details = TvShowDetails {
            provider: MetadataProvider::Tvmaze,
            provider_id: tvmaze_id,
            name: show.name.clone(),
            sort_name: Some(show.name.clone()),
            year: show
                .premiered
                .as_ref()
                .and_then(|d| d.get(0..4))
                .and_then(|y| y.parse().ok()),
            status: show.status.clone(),
            overview: show.clean_summary(),
            network,
            runtime: show.average_runtime.or(show.runtime).map(|v| v as i32),
            genres: show.genres.clone(),
            poster_url: show.image.as_ref().and_then(|i| i.original.clone()),
            backdrop_url,
            tvdb_id: show
                .externals
                .as_ref()
                .and_then(|e| e.thetvdb)
                .map(|v| v as i32),
            imdb_id: show.externals.as_ref().and_then(|e| e.imdb.clone()),
        };

        if let Err(error) = self
            .upsert_metadata_cache(MetadataProvider::Tvmaze, "GetTvShow", &cache_key, &details)
            .await
        {
            warn!(error = %error, "Failed to update metadata cache for GetTvShow");
        }

        Ok(details)
    }

    /// Every episode of a show from TVMaze, cached like the show itself.
    pub async fn get_tv_show_episodes(&self, tvmaze_id: u32) -> Result<Vec<TvShowEpisodeDetails>> {
        let cache_key = tvmaze_id.to_string();
        let max_age_days = self.provider_cache_days(MetadataProvider::Tvmaze).await;
        if let Some(cached) = self
            .read_metadata_cache::<Vec<TvShowEpisodeDetails>>(
                MetadataProvider::Tvmaze,
                TV_SHOW_EPISODES_OPERATION,
                &cache_key,
                max_age_days,
            )
            .await?
        {
            return Ok(cached);
        }

        let tvmaze = self.get_tvmaze_client().await?;
        let episodes: Vec<TvShowEpisodeDetails> = tvmaze
            .get_episodes(tvmaze_id)
            .await?
            .into_iter()
            .map(|episode| TvShowEpisodeDetails {
                overview: episode.clean_summary(),
                still_url: episode
                    .image
                    .as_ref()
                    .and_then(|image| image.original.clone()),
                id: episode.id,
                season: episode.season,
                number: episode.number,
                name: episode.name,
                air_date: episode.airdate,
                runtime: episode.runtime,
            })
            .collect();
        if let Err(error) = self
            .upsert_metadata_cache(
                MetadataProvider::Tvmaze,
                TV_SHOW_EPISODES_OPERATION,
                &cache_key,
                &episodes,
            )
            .await
        {
            warn!(error = %error, "Failed to update metadata cache for a show's episodes");
        }
        Ok(episodes)
    }

    /// Get audiobook details from OpenLibrary work API
    pub async fn get_audiobook(&self, openlibrary_id: &str) -> Result<AudiobookDetails> {
        self.get_audiobook_with_policy(openlibrary_id, MetadataFetchPolicy::PreferCache)
            .await
    }

    async fn get_audiobook_with_policy(
        &self,
        provider_id: &str,
        policy: MetadataFetchPolicy,
    ) -> Result<AudiobookDetails> {
        if audiobook_provider_for_id(provider_id) == MetadataProvider::Audible {
            self.get_audible_audiobook(provider_id, policy).await
        } else {
            self.get_open_library_audiobook(provider_id, policy).await
        }
    }

    async fn get_audible_audiobook(
        &self,
        asin: &str,
        policy: MetadataFetchPolicy,
    ) -> Result<AudiobookDetails> {
        anyhow::ensure!(is_asin(asin), "Not an Audible identifier: {asin}");
        let cache_key = Self::normalized_cache_component(asin);
        if policy == MetadataFetchPolicy::PreferCache {
            let max_age_days = self.provider_cache_days(MetadataProvider::Audible).await;
            if let Some(cached) = self
                .read_metadata_cache::<AudiobookDetails>(
                    MetadataProvider::Audible,
                    "GetAudiobook",
                    &cache_key,
                    max_age_days,
                )
                .await?
            {
                return Ok(cached);
            }
        }
        let client = crate::services::http_client::outbound_client(
            crate::services::http_client::OutboundHttpProfile::Metadata,
        )?;
        let product = client
            .get(format!("{AUDIBLE_API}/catalog/products/{asin}"))
            .query(&[
                ("response_groups", AUDIBLE_RESPONSE_GROUPS),
                ("image_sizes", "500,1000"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<AudibleProductResponse>()
            .await?
            .product;
        let mut details = audible_details(product);
        // Chapters are a bonus: the book is still worth having without them.
        let audnexus_enabled = MetadataSettings::load(&self.db)
            .await
            .map_or(true, |settings| settings.audnexus_enabled);
        if audnexus_enabled {
            match client
                .get(format!("{AUDNEXUS_API}/books/{asin}/chapters"))
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
            {
                Ok(response) => match response.json::<AudnexusChapters>().await {
                    Ok(listing) => {
                        if let Some(seconds) = listing.runtime_length_sec {
                            details.total_duration_secs = Some(seconds);
                        }
                        details.chapters = audiobook_chapters(listing.chapters);
                    }
                    Err(error) => warn!(asin, error = %error, "Could not read Audnexus chapters"),
                },
                Err(error) => warn!(asin, error = %error, "No chapter listing from Audnexus"),
            }
        }
        if let Err(error) = self
            .upsert_metadata_cache(
                MetadataProvider::Audible,
                "GetAudiobook",
                &cache_key,
                &details,
            )
            .await
        {
            warn!(error = %error, "Failed to update metadata cache for Audible details");
        }
        Ok(details)
    }

    async fn get_open_library_audiobook(
        &self,
        openlibrary_id: &str,
        policy: MetadataFetchPolicy,
    ) -> Result<AudiobookDetails> {
        #[derive(Debug, Deserialize)]
        struct OpenLibrarySearchCover {
            cover_i: Option<i64>,
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibrarySearchCovers {
            docs: Vec<OpenLibrarySearchCover>,
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibraryWorkAuthor {
            author: OpenLibraryAuthorRef,
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibraryAuthorRef {
            key: String,
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibraryDescriptionObject {
            value: String,
        }

        #[derive(Debug, Deserialize)]
        #[serde(untagged)]
        enum OpenLibraryDescription {
            Text(String),
            Object(OpenLibraryDescriptionObject),
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibraryWork {
            title: Option<String>,
            description: Option<OpenLibraryDescription>,
            covers: Option<Vec<i64>>,
            authors: Option<Vec<OpenLibraryWorkAuthor>>,
            first_publish_date: Option<String>,
            languages: Option<Vec<serde_json::Value>>,
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibraryAuthor {
            name: Option<String>,
        }

        let cache_key = Self::normalized_cache_component(openlibrary_id);
        if policy == MetadataFetchPolicy::PreferCache {
            let max_age_days = self
                .provider_cache_days(MetadataProvider::OpenLibrary)
                .await;
            if let Some(cached) = self
                .read_metadata_cache::<AudiobookDetails>(
                    MetadataProvider::OpenLibrary,
                    "GetAudiobook",
                    &cache_key,
                    max_age_days,
                )
                .await?
            {
                return Ok(cached);
            }
        }

        let client = crate::services::http_client::outbound_client(
            crate::services::http_client::OutboundHttpProfile::Metadata,
        )?;
        let work_url = format!("https://openlibrary.org/works/{}.json", openlibrary_id);
        let work = client
            .get(work_url)
            .send()
            .await?
            .error_for_status()?
            .json::<OpenLibraryWork>()
            .await?;

        let description = match work.description {
            Some(OpenLibraryDescription::Text(text)) => Some(text),
            Some(OpenLibraryDescription::Object(obj)) => Some(obj.value),
            None => None,
        };

        let mut cover_id = work.covers.and_then(|covers| covers.first().copied());
        if cover_id.is_none() {
            // A work's own record often has no cover though its search entry does.
            let found = client
                .get("https://openlibrary.org/search.json")
                .query(&[
                    ("q", format!("key:/works/{openlibrary_id}").as_str()),
                    ("fields", "cover_i"),
                    ("limit", "1"),
                ])
                .send()
                .await
                .and_then(reqwest::Response::error_for_status);
            if let Ok(response) = found
                && let Ok(covers) = response.json::<OpenLibrarySearchCovers>().await
            {
                cover_id = covers.docs.first().and_then(|doc| doc.cover_i);
            }
        }
        let cover_url =
            cover_id.map(|id| format!("https://covers.openlibrary.org/b/id/{}-L.jpg", id));

        let author_name = if let Some(authors) = work.authors {
            if let Some(author_ref) = authors.first() {
                let author_url = format!("https://openlibrary.org{}.json", author_ref.author.key);
                client
                    .get(author_url)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<OpenLibraryAuthor>()
                    .await?
                    .name
            } else {
                None
            }
        } else {
            None
        };

        let authors: Vec<String> = author_name.iter().cloned().collect();
        let details = AudiobookDetails {
            provider: MetadataProvider::OpenLibrary,
            provider_id: openlibrary_id.to_string(),
            title: work
                .title
                .unwrap_or_else(|| "Unknown Audiobook".to_string()),
            sort_title: None,
            author_name,
            description,
            publisher: None,
            language: work
                .languages
                .and_then(|langs| langs.first().cloned())
                .and_then(|v| v.get("key").and_then(|k| k.as_str()).map(|s| s.to_string())),
            isbn: None,
            published_date: work.first_publish_date,
            cover_url,
            authors,
            narrators: Vec::new(),
            total_duration_secs: None,
            chapters: Vec::new(),
            series_name: None,
            series_position: None,
        };

        if let Err(error) = self
            .upsert_metadata_cache(
                MetadataProvider::OpenLibrary,
                "GetAudiobook",
                &cache_key,
                &details,
            )
            .await
        {
            warn!(
                error = %error,
                "Failed to update metadata cache for GetAudiobook"
            );
        }

        Ok(details)
    }

    async fn create_movie_from_details(
        &self,
        actor: &AuthUser,
        details: &MovieDetails,
        options: &AddMovieOptions,
    ) -> Result<Movie> {
        let rating = details
            .vote_average
            .and_then(rust_decimal::Decimal::from_f64_retain)
            .map(|value| value.to_string());
        let data = self.execute_mutation(actor,
            r#"mutation CreateImportedMovie($input: CreateMovieInput!) {
                createMovie(input: $input) { success error movie { id } }
            }"#,
            serde_json::json!({ "input": {
                "libraryId": options.library_id.to_string(), "userId": options.user_id.to_string(),
                "title": details.title, "sortTitle": null, "originalTitle": details.original_title,
                "year": details.year, "tmdbId": details.provider_id as i32, "imdbId": details.imdb_id,
                "overview": details.overview, "tagline": details.tagline, "runtime": details.runtime,
                "genres": details.genres, "director": details.director, "castNames": details.cast_names,
                "productionCountries": details.production_countries, "spokenLanguages": details.spoken_languages,
                "tmdbRating": rating, "tmdbVoteCount": details.vote_count,
                "posterUrl": details.poster_url, "backdropUrl": details.backdrop_url,
                "collectionId": details.collection_id, "collectionName": details.collection_name,
                "collectionPosterUrl": details.collection_poster_url, "releaseDate": details.release_date,
                "certification": details.certification, "monitored": options.monitored,
                "digitalReleaseDate": details.digital_release_date,
                "physicalReleaseDate": details.physical_release_date,
                "tmdbStatus": details.tmdb_status, "wanted": options.monitored,
                "hasFile": false
            }})).await?;
        let id = data["createMovie"]["movie"]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("createMovie returned no movie identity"))?;
        Movie::get(self.db.pool(), &id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Imported movie disappeared after creation"))
    }

    /// Add a movie from TMDB to a library
    ///
    /// Fetches movie metadata from TMDB and persists it through checked generated
    /// GraphQL mutations after validating the target library's owner.
    pub async fn add_movie_from_provider(&self, options: AddMovieOptions) -> Result<Movie> {
        self.require_import_owner(options.library_id, options.user_id)
            .await?;
        debug!(
            "Adding movie from {:?} (ID: {}) to library",
            options.provider, options.provider_id
        );

        // Only TMDB is supported for movies
        if options.provider != MetadataProvider::Tmdb {
            anyhow::bail!("Only TMDB is supported for movie metadata");
        }

        // Check if movie already exists in this library
        let auth_user = AuthUser {
            user_id: options.user_id.to_string(),
            email: None,
            role: Some("admin".to_string()),
        };
        let existing_id = self
            .movie_id_in_library_by_tmdb(&auth_user, options.library_id, options.provider_id as i32)
            .await?;

        if let Some(movie_id_str) = existing_id {
            let existing_movie = Movie::get(self.db.pool(), &movie_id_str)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Movie not found after query"))?;
            if let Some(collection_id) = existing_movie.collection_id
                && let Err(error) = self
                    .ensure_movie_collection_entity(
                        options.library_id,
                        options.user_id,
                        collection_id,
                    )
                    .await
            {
                warn!(
                    library_id = %options.library_id,
                    user_id = %options.user_id,
                    collection_id = collection_id,
                    error = %error,
                    "Failed to ensure collection entity while returning existing movie"
                );
            }
            debug!("Movie '{}' already exists in library", existing_movie.title);
            return Ok(existing_movie);
        }

        // Get movie details from TMDB
        let movie_details = self.get_movie(options.provider_id).await?;
        let cached_collection_poster_url = self
            .cache_collection_poster_artwork(
                movie_details.collection_id,
                movie_details.collection_poster_url.as_deref(),
            )
            .await
            .or_else(|| movie_details.collection_poster_url.clone());

        let mut movie_details_for_create = movie_details.clone();
        movie_details_for_create.collection_poster_url = cached_collection_poster_url;
        let movie = match self
            .create_movie_from_details(&auth_user, &movie_details_for_create, &options)
            .await
        {
            Ok(movie) => movie,
            Err(error) => {
                let err = error.to_string();
                let is_unique_conflict = err.contains("UNIQUE constraint failed")
                    || err.contains("idx_movies_library_tmdb_unique");
                if is_unique_conflict
                    && let Some(existing_id) = self
                        .movie_id_in_library_by_tmdb(
                            &auth_user,
                            options.library_id,
                            options.provider_id as i32,
                        )
                        .await?
                {
                    let movie = Movie::get(self.db.pool(), &existing_id)
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("Movie not found after unique conflict"))?;
                    info!(
                        library_id = %options.library_id,
                        user_id = %options.user_id,
                        tmdb_id = options.provider_id as i32,
                        movie_id = %movie.id,
                        "Resolved concurrent movie create by returning existing movie"
                    );
                    return Ok(movie);
                }
                return Err(error);
            }
        };

        if let Some(collection_id) = movie_details.collection_id
            && let Err(error) = self
                .ensure_movie_collection_entity(options.library_id, options.user_id, collection_id)
                .await
        {
            warn!(
                library_id = %options.library_id,
                user_id = %options.user_id,
                collection_id = collection_id,
                error = %error,
                "Failed to ensure collection entity after movie creation"
            );
        }
        self.store_movie_extras(&movie, movie_details.extras.as_ref())
            .await;

        // Cache artwork, then emit a lightweight entity update so library routes
        // subscribed to change events refresh when artwork becomes available.
        if let Some(artwork_service) = self.artwork_service().await {
            let _ = artwork_service
                .cache_movie_artwork(
                    &movie.id,
                    movie_details.poster_url.as_deref(),
                    movie_details.backdrop_url.as_deref(),
                )
                .await;
        }

        Ok(movie)
    }

    /// Import all movies in a TMDB collection into a library.
    pub async fn add_movie_collection_from_provider(
        &self,
        options: AddMovieCollectionOptions,
    ) -> Result<MovieCollectionImportSummary> {
        self.require_import_owner(options.library_id, options.user_id)
            .await?;
        if options.provider != MetadataProvider::Tmdb {
            anyhow::bail!("Only TMDB is supported for movie collections");
        }

        let tmdb = self.get_tmdb_client().await?;
        let collection = tmdb.get_collection(options.collection_id).await?;
        if let Err(error) = self
            .ensure_movie_collection_entity(
                options.library_id,
                options.user_id,
                options.collection_id,
            )
            .await
        {
            warn!(
                library_id = %options.library_id,
                user_id = %options.user_id,
                collection_id = options.collection_id,
                error = %error,
                "Failed to ensure collection entity before collection import"
            );
        }
        let auth_user = AuthUser {
            user_id: options.user_id.to_string(),
            email: None,
            role: Some("admin".to_string()),
        };

        let mut imported_count = 0_i32;
        let mut existing_count = 0_i32;
        let mut wanted_updated_count = 0_i32;

        for part in collection.parts {
            let part_tmdb_id = part.id;
            let existed = self
                .movie_exists_in_library_by_tmdb(&auth_user, options.library_id, part_tmdb_id)
                .await?;

            let movie = self
                .add_movie_from_provider(AddMovieOptions {
                    provider: MetadataProvider::Tmdb,
                    provider_id: part_tmdb_id as u32,
                    library_id: options.library_id,
                    user_id: options.user_id,
                    monitored: options.wanted_missing,
                })
                .await?;

            if existed {
                existing_count += 1;
                // Ensure collection linkage and artwork stay fresh on collection imports.
                let _ = self
                    .refresh_movie_from_provider(&movie.id, options.user_id)
                    .await;
            } else {
                imported_count += 1;
            }

            if options.wanted_missing && movie.media_file_id.is_none() && !movie.wanted {
                let update_data = self
                    .execute_mutation(
                        &auth_user,
                        r#"mutation UpdateImportedMovieWanted($id: String!, $input: UpdateMovieInput!) {
                            UpdateMovie: updateMovie(id: $id, input: $input) { Success: success Error: error }
                        }"#,
                        serde_json::json!({
                            "id": movie.id,
                            "input": { "wanted": true }
                        }),
                    )
                    .await?;
                let updated = update_data
                    .get("UpdateMovie")
                    .and_then(|v| v.get("Success"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if updated {
                    wanted_updated_count += 1;
                }
            }
        }

        Ok(MovieCollectionImportSummary {
            collection_id: collection.id,
            collection_name: collection.name,
            imported_count,
            existing_count,
            wanted_updated_count,
        })
    }

    /// Get full TMDB collection details overlaid with current library state.
    pub async fn get_movie_collection_details(
        &self,
        collection_id: i32,
        library_id: Uuid,
        caller: &AuthUser,
    ) -> Result<MovieCollectionDetails> {
        self.authorize_library_import(caller, library_id).await?;
        let tmdb = self.get_tmdb_client().await?;
        let collection = tmdb.get_collection(collection_id).await?;
        self.overlay_movie_collection_details(&tmdb, collection, library_id, caller)
            .await
    }

    /// Overlay only authorized local reads onto an already fetched catalog.
    async fn overlay_movie_collection_details(
        &self,
        tmdb: &TmdbClient,
        collection: super::tmdb::TmdbCollection,
        library_id: Uuid,
        caller: &AuthUser,
    ) -> Result<MovieCollectionDetails> {
        let mut local_movies: Vec<LocalCollectionMovieRow> = Vec::new();
        let mut offset = 0usize;
        let page_limit = 500usize;
        loop {
            let local_data = self
                .execute_query(
                    caller,
                    r#"query CollectionLibraryMovies($where: MovieWhereInput, $page: PageInput) {
                        Movies: movies(where: $where, page: $page) {
                            Edges: edges {
                                Node: node {
                                    Id: id
                                    TmdbId: tmdbId
                                    CollectionId: collectionId
                                    Title: title
                                    Year: year
                                    MediaFileId: mediaFileId
                                    MediaFile: mediaFile {
                                        Size: size
                                        Resolution: resolution
                                        VideoCodec: videoCodec
                                        AudioCodec: audioCodec
                                        AudioChannels: audioChannels
                                    }
                                    Wanted: wanted
                                }
                            }
                        }
                    }"#,
                    serde_json::json!({
                        "where": {
                            "libraryId": { "eq": library_id.to_string() }
                        },
                        "page": { "limit": page_limit, "offset": offset }
                    }),
                )
                .await?;

            let edges = local_data
                .get("Movies")
                .and_then(|m| m.get("Edges"))
                .and_then(|e| e.as_array())
                .cloned()
                .unwrap_or_default();
            if edges.is_empty() {
                break;
            }

            for edge in &edges {
                let node = edge.get("Node").unwrap_or(&serde_json::Value::Null);
                let id = node
                    .get("Id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                if id.is_empty() {
                    continue;
                }

                local_movies.push(LocalCollectionMovieRow {
                    id,
                    tmdb_id: node
                        .get("TmdbId")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32),
                    collection_id: node
                        .get("CollectionId")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32),
                    title: node
                        .get("Title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                    year: node.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32),
                    poster_url: None,
                    media_file_id: node
                        .get("MediaFileId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    file_size_bytes: node
                        .get("MediaFile")
                        .and_then(|v| v.get("Size"))
                        .and_then(|v| v.as_i64()),
                    resolution: node
                        .get("MediaFile")
                        .and_then(|v| v.get("Resolution"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    video_codec: node
                        .get("MediaFile")
                        .and_then(|v| v.get("VideoCodec"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    audio_codec: node
                        .get("MediaFile")
                        .and_then(|v| v.get("AudioCodec"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    audio_channels: node
                        .get("MediaFile")
                        .and_then(|v| v.get("AudioChannels"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    wanted: node
                        .get("Wanted")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                });
            }

            if edges.len() < page_limit {
                break;
            }
            offset += page_limit;
        }

        Ok(Self::movie_collection_overlay(
            tmdb,
            collection,
            library_id,
            local_movies,
        ))
    }

    /// Pure catalog/local overlay: no service/database handle, no persistence.
    fn movie_collection_overlay(
        tmdb: &TmdbClient,
        collection: super::tmdb::TmdbCollection,
        library_id: Uuid,
        local_movies: Vec<LocalCollectionMovieRow>,
    ) -> MovieCollectionDetails {
        let collection_id = collection.id;
        let collection_poster_url = tmdb.poster_url(collection.poster_path.as_deref());
        let collection_backdrop_url = tmdb.backdrop_url(collection.backdrop_path.as_deref());
        let mut merged_movies: Vec<MovieCollectionMovieDetails> = Vec::new();
        let mut used_local_movie_ids: HashSet<String> = HashSet::new();
        let mut exact_tmdb_matches = 0usize;
        let mut heuristic_matches = 0usize;
        for part in collection.parts.iter() {
            let mut best_index: Option<usize> = None;
            let mut best_score = i32::MIN;
            for (index, local) in local_movies.iter().enumerate() {
                if used_local_movie_ids.contains(&local.id) {
                    continue;
                }
                let Some(score) = Self::collection_match_score(
                    local,
                    collection_id,
                    part.id,
                    &part.title,
                    part.year(),
                ) else {
                    continue;
                };
                if score > best_score {
                    best_score = score;
                    best_index = Some(index);
                }
            }

            let local = best_index.and_then(|index| local_movies.get(index).cloned());
            if let Some(local_match) = local.as_ref() {
                if local_match.tmdb_id == Some(part.id) {
                    exact_tmdb_matches += 1;
                } else {
                    heuristic_matches += 1;
                }
                used_local_movie_ids.insert(local_match.id.clone());
            }

            merged_movies.push(MovieCollectionMovieDetails {
                tmdb_id: part.id,
                title: part.title.clone(),
                year: part.year(),
                poster_url: local
                    .as_ref()
                    .and_then(|m| m.poster_url.clone())
                    .or_else(|| tmdb.poster_url(part.poster_path.as_deref())),
                library_movie_id: local.as_ref().map(|m| m.id.clone()),
                media_file_id: local.as_ref().and_then(|m| m.media_file_id.clone()),
                file_size_bytes: local.as_ref().and_then(|m| m.file_size_bytes),
                resolution: local.as_ref().and_then(|m| m.resolution.clone()),
                video_codec: local.as_ref().and_then(|m| m.video_codec.clone()),
                audio_codec: local.as_ref().and_then(|m| m.audio_codec.clone()),
                audio_channels: local.as_ref().and_then(|m| m.audio_channels.clone()),
                wanted: local.as_ref().map(|m| m.wanted).unwrap_or(false),
            });
        }

        // Include local rows explicitly linked to this collection that are not
        // present in the current TMDB collection response.
        for local in local_movies {
            if used_local_movie_ids.contains(&local.id) {
                continue;
            }
            if local.collection_id != Some(collection_id) {
                continue;
            }
            merged_movies.push(MovieCollectionMovieDetails {
                tmdb_id: local.tmdb_id.unwrap_or_default(),
                title: local.title,
                year: local.year,
                poster_url: local.poster_url,
                library_movie_id: Some(local.id),
                media_file_id: local.media_file_id,
                file_size_bytes: local.file_size_bytes,
                resolution: local.resolution,
                video_codec: local.video_codec,
                audio_codec: local.audio_codec,
                audio_channels: local.audio_channels,
                wanted: local.wanted,
            });
        }

        debug!(
            library_id = %library_id,
            collection_id,
            exact_tmdb_matches,
            heuristic_matches,
            "Built read-only collection details overlay"
        );

        MovieCollectionDetails {
            collection_id: collection.id,
            name: collection.name,
            overview: collection.overview,
            poster_url: collection_poster_url,
            backdrop_url: collection_backdrop_url,
            movies: merged_movies,
        }
    }

    /// Add a TV show from TVMaze to a library
    pub async fn add_tv_show_from_provider(&self, options: AddTvShowOptions) -> Result<Show> {
        self.require_import_owner(options.library_id, options.user_id)
            .await?;
        debug!(
            "Adding TV show from {:?} (ID: {}) to library",
            options.provider, options.provider_id
        );

        if options.provider != MetadataProvider::Tvmaze {
            anyhow::bail!("Only TVMaze is supported for TV show metadata");
        }

        let auth_user = AuthUser {
            user_id: options.user_id.to_string(),
            email: None,
            role: Some("admin".to_string()),
        };

        let data = self
            .execute_query(
                &auth_user,
                r#"query ShowByTvmaze($where: ShowWhereInput, $page: PageInput) {
                    Shows: shows(where: $where, page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "libraryId": { "eq": options.library_id.to_string() },
                        "tvmazeId": { "eq": options.provider_id as i32 }
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await?;
        let existing_id = data
            .get("Shows")
            .and_then(|m| m.get("Edges"))
            .and_then(|e| e.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string());

        if let Some(show_id) = existing_id {
            let existing_show = Show::get(self.db.pool(), &show_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Show not found after query"))?;
            let wanted_default = options.monitor_type != AutoDownloadMode::None;
            // Ensure episodes are backfilled even when show already exists.
            self.sync_show_episodes_from_tvmaze(
                &auth_user,
                &existing_show.id,
                options.provider_id,
                wanted_default,
            )
            .await?;
            self.sync_show_seasons_best_effort(&auth_user, &existing_show.id, options.provider_id)
                .await;
            debug!("Show '{}' already exists in library", existing_show.name);
            return Ok(existing_show);
        }

        let details = self.get_tv_show(options.provider_id).await?;

        let auto_download = options.monitor_type != AutoDownloadMode::None;
        let data = self.execute_mutation(&auth_user,
            r#"mutation UpsertImportedShow($input: CreateShowInput!) {
                upsertShow(input: $input) { success error show { id } }
            }"#,
            serde_json::json!({ "input": {
                "libraryId": options.library_id.to_string(), "userId": options.user_id.to_string(),
                "name": details.name, "sortName": details.sort_name, "year": details.year,
                "tvmazeId": details.provider_id as i32, "tmdbId": null, "tvdbId": details.tvdb_id,
                "imdbId": details.imdb_id, "overview": details.overview, "network": details.network,
                "runtime": details.runtime, "genres": details.genres, "posterUrl": details.poster_url,
                "backdropUrl": details.backdrop_url, "status": details.status,
                "autoDownload": auto_download, "autoDownloadMode": options.monitor_type.to_string().to_uppercase(),
                "qualityProfileId": null, "path": options.path
            }})).await?;
        let id = data["upsertShow"]["show"]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("upsertShow returned no show identity"))?;
        let show = Show::get(self.db.pool(), &id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Imported show disappeared after upsert"))?;

        // Populate episodes through generated mutations so subscription notifications fire.
        self.sync_show_episodes_from_tvmaze(
            &auth_user,
            &show.id,
            options.provider_id,
            auto_download,
        )
        .await?;
        self.sync_show_seasons_best_effort(&auth_user, &show.id, options.provider_id)
            .await;
        // A show's credits take a request per season; the credits job reads them (Q132).
        request_credit_fill();

        // Cache artwork, then emit a lightweight entity update so library routes
        // subscribed to change events refresh when artwork becomes available.
        if let Some(artwork_service) = self.artwork_service().await {
            let _ = artwork_service
                .cache_show_artwork(
                    &show.id,
                    details.poster_url.as_deref(),
                    details.backdrop_url.as_deref(),
                )
                .await;
        }

        Ok(show)
    }

    /// Season sync never blocks adding or refreshing a show: posters arrive later otherwise.
    async fn sync_show_seasons_best_effort(
        &self,
        auth_user: &AuthUser,
        show_id: &str,
        tvmaze_id: u32,
    ) {
        if let Err(error) = self
            .sync_show_seasons_from_tvmaze(auth_user, show_id, tvmaze_id)
            .await
        {
            warn!(show_id, tvmaze_id, error = %error, "Failed to sync seasons from TVMaze");
        }
    }

    /// Upsert a show's seasons from TVMaze through the generated Season mutation and cache
    /// each season poster into object storage.
    async fn sync_show_seasons_from_tvmaze(
        &self,
        auth_user: &AuthUser,
        show_id: &str,
        tvmaze_id: u32,
    ) -> Result<usize> {
        let tvmaze = self.get_tvmaze_client().await?;
        let seasons = tvmaze.get_seasons(tvmaze_id).await?;
        let artwork = self.artwork_service().await;
        let mut synced = 0;
        for season in &seasons {
            let poster_url = season
                .image
                .as_ref()
                .and_then(|image| image.original.clone().or_else(|| image.medium.clone()));
            let data = self
                .execute_mutation(
                    auth_user,
                    r#"mutation UpsertSeasonFromTvmaze($input: CreateSeasonInput!) {
                        UpsertSeason: upsertSeason(input: $input) {
                            Success: success
                            Error: error
                            Season: season { Id: id }
                        }
                    }"#,
                    serde_json::json!({ "input": {
                        "showId": show_id,
                        "season": season.number as i32,
                        "tvmazeId": season.id as i32,
                        "name": season.name.as_deref().map(str::trim).filter(|name| !name.is_empty()),
                        "overview": season.clean_summary(),
                        "premiereDate": season.premiere_date,
                        "endDate": season.end_date,
                        "episodeOrder": season.episode_order.map(|count| count as i32),
                        "posterUrl": poster_url,
                    }}),
                )
                .await?;
            let result = &data["UpsertSeason"];
            if result["Success"].as_bool() != Some(true) {
                anyhow::bail!(
                    "{}",
                    result["Error"].as_str().unwrap_or("Failed to save season")
                );
            }
            synced += 1;
            if let (Some(artwork), Some(season_id), Some(url)) = (
                &artwork,
                result["Season"]["Id"].as_str(),
                poster_url.as_deref(),
            ) {
                artwork.cache_season_artwork(season_id, Some(url)).await;
            }
        }
        debug!(show_id, count = synced, "Synced seasons from TVMaze");
        Ok(synced)
    }

    /// Bring shows added before seasons, backdrops and stills were stored up to date: up
    /// to `limit` shows per call, each attempted once per process so a show TVMaze has no
    /// art for is not fetched again every run. Returns how many shows were processed.
    pub async fn backfill_show_artwork(&self, limit: usize) -> Result<usize> {
        static ATTEMPTED: LazyLock<std::sync::Mutex<HashSet<String>>> =
            LazyLock::new(Default::default);
        let shows = Show::query(self.db.pool()).fetch_all().await?;
        let mut processed = 0;
        for show in shows {
            if processed >= limit {
                break;
            }
            let Some(tvmaze_id) = show.tvmaze_id else {
                continue;
            };
            if ATTEMPTED
                .lock()
                .map(|set| set.contains(&show.id))
                .unwrap_or(true)
            {
                continue;
            }
            let has_seasons = !Season::query(self.db.pool())
                .filter(SeasonWhereInput {
                    show_id: Some(StringFilter {
                        eq: Some(show.id.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .limit(1)
                .fetch_all()
                .await?
                .is_empty();
            let missing_stills = !Episode::query(self.db.pool())
                .filter(EpisodeWhereInput {
                    show_id: Some(StringFilter {
                        eq: Some(show.id.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .fetch_all()
                .await?
                .iter()
                .any(|episode| episode.still_url.is_some());
            if has_seasons && show.backdrop_url.is_some() && !missing_stills {
                continue;
            }
            if let Ok(mut set) = ATTEMPTED.lock() {
                set.insert(show.id.clone());
            }
            processed += 1;

            let auth_user = AuthUser {
                user_id: show.user_id.clone(),
                email: None,
                role: Some("admin".to_string()),
            };
            if show.backdrop_url.is_none() {
                match self
                    .get_tv_show_with_policy(tvmaze_id as u32, MetadataFetchPolicy::ForceRefresh)
                    .await
                {
                    Ok(details) if details.backdrop_url.is_some() => {
                        let saved = self
                            .execute_mutation(
                                &auth_user,
                                r#"mutation BackfillShowBackdrop($id: String!, $input: UpdateShowInput!) {
                                    UpdateShow: updateShow(id: $id, input: $input) { Success: success Error: error }
                                }"#,
                                serde_json::json!({ "id": show.id, "input": { "backdropUrl": details.backdrop_url } }),
                            )
                            .await;
                        if let Err(error) = saved {
                            warn!(show_id = %show.id, error = %error, "Failed to save backfilled show backdrop");
                        } else if let Some(artwork) = self.artwork_service().await {
                            artwork
                                .cache_show_artwork(
                                    &show.id,
                                    details.poster_url.as_deref(),
                                    details.backdrop_url.as_deref(),
                                )
                                .await;
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        warn!(show_id = %show.id, error = %error, "Failed to fetch show details for artwork backfill")
                    }
                }
            }
            if !has_seasons {
                self.sync_show_seasons_best_effort(&auth_user, &show.id, tvmaze_id as u32)
                    .await;
            }
            if missing_stills
                && let Err(error) = self
                    .sync_show_episodes_from_tvmaze(
                        &auth_user,
                        &show.id,
                        tvmaze_id as u32,
                        show.auto_download,
                    )
                    .await
            {
                warn!(show_id = %show.id, error = %error, "Failed to backfill episode stills");
            }
        }
        if processed > 0 {
            info!(
                processed,
                "Backfilled show seasons, backdrops and episode stills"
            );
        }
        Ok(processed)
    }

    /// Sync episodes for a show from TVMaze using generated Episode mutations.
    async fn sync_show_episodes_from_tvmaze(
        &self,
        auth_user: &AuthUser,
        show_id: &str,
        tvmaze_id: u32,
        wanted_default: bool,
    ) -> Result<()> {
        let tvmaze = self.get_tvmaze_client().await?;
        let episodes = tvmaze.get_episodes(tvmaze_id).await?;

        for ep in episodes {
            // Unnumbered specials remain in the public guide; they cannot be
            // assigned a unique season/episode slot in a library yet.
            if ep.number == 0 {
                continue;
            }
            let season = ep.season as i32;
            let episode_number = ep.number as i32;
            let tvmaze_ep_id = Some(ep.id as i32);
            let existing = self
                .execute_query(
                    auth_user,
                    r#"query EpisodeByIdentity($where: EpisodeWhereInput, $page: PageInput) {
                        Episodes: episodes(where: $where, page: $page) {
                            Edges: edges { Node: node { Id: id Wanted: wanted } }
                        }
                    }"#,
                    serde_json::json!({
                        "where": {
                            "showId": { "eq": show_id },
                            "or": [
                                { "tvmazeId": { "eq": ep.id as i32 } },
                                {
                                    "and": [
                                        { "season": { "eq": season } },
                                        { "episode": { "eq": episode_number } }
                                    ]
                                }
                            ]
                        },
                        "page": { "limit": 1, "offset": 0 }
                    }),
                )
                .await?;

            let existing_edge = existing
                .get("Episodes")
                .and_then(|v| v.get("Edges"))
                .and_then(|v| v.as_array())
                .and_then(|edges| edges.first())
                .and_then(|edge| edge.get("Node"));

            let title = Some(ep.name.clone());
            let overview = ep.clean_summary();
            let runtime = ep.runtime.map(|r| r as i32);
            let still_url = ep
                .image
                .as_ref()
                .and_then(|image| image.original.clone().or_else(|| image.medium.clone()));

            if let Some(node) = existing_edge {
                let existing_id = node
                    .get("Id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Episode id missing in existing lookup"))?;
                let existing_wanted = node
                    .get("Wanted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(wanted_default);

                let updated = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation UpdateEpisodeFromTvmaze($id: String!, $input: UpdateEpisodeInput!) {
                            UpdateEpisode: updateEpisode(id: $id, input: $input) {
                                Success: success
                                Error: error
                            }
                        }"#,
                        serde_json::json!({
                            "id": existing_id,
                            "input": {
                                "showId": show_id,
                                "season": season,
                                "episode": episode_number,
                                "title": title,
                                "overview": overview,
                                "airDate": ep.airdate,
                                "airStamp": ep.air_stamp,
                                "runtime": runtime,
                                "stillUrl": still_url,
                                "tvmazeId": tvmaze_ep_id,
                                "wanted": existing_wanted
                            }
                        }),
                    )
                    .await?;

                let ok = updated
                    .get("UpdateEpisode")
                    .and_then(|v| v.get("Success"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !ok {
                    let err = updated
                        .get("UpdateEpisode")
                        .and_then(|v| v.get("Error"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Failed to update episode");
                    anyhow::bail!(err.to_string());
                }
            } else {
                let created = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation CreateEpisodeFromTvmaze($input: CreateEpisodeInput!) {
                            CreateEpisode: createEpisode(input: $input) {
                                Success: success
                                Error: error
                            }
                        }"#,
                        serde_json::json!({
                            "input": {
                                "showId": show_id,
                                "season": season,
                                "episode": episode_number,
                                "title": title,
                                "overview": overview,
                                "airDate": ep.airdate,
                                "airStamp": ep.air_stamp,
                                "runtime": runtime,
                                "stillUrl": still_url,
                                "tvmazeId": tvmaze_ep_id,
                                "wanted": wanted_default
                            }
                        }),
                    )
                    .await?;

                let ok = created
                    .get("CreateEpisode")
                    .and_then(|v| v.get("Success"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !ok {
                    let err = created
                        .get("CreateEpisode")
                        .and_then(|v| v.get("Error"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Failed to create episode");
                    anyhow::bail!(err.to_string());
                }
            }
        }

        Ok(())
    }

    /// Add an album from MusicBrainz to a library.
    /// The edition of an album that best fits some files, with how well they fit. Each edition read
    /// is a request to a rate-limited service, so only the likeliest few are.
    pub async fn album_listing_for_files(
        &self,
        release_group_id: Uuid,
        files: &[crate::services::release_match::ReleaseFile],
        names: &[Option<String>],
        held_tracks: usize,
    ) -> Result<
        Option<(
            Vec<crate::services::metadata::musicbrainz::MusicBrainzTrack>,
            crate::services::music_rules::ListingFit,
        )>,
    > {
        use crate::services::music_rules;
        const EDITIONS_READ: usize = 4;
        let musicbrainz = self.get_musicbrainz_client().await?;
        let editions = musicbrainz
            .get_release_group_editions(release_group_id)
            .await?;
        let counts: Vec<(usize, bool)> = editions
            .iter()
            .map(|edition| (edition.track_count, edition.official))
            .collect();
        let mut listings = Vec::new();
        let mut fits = Vec::new();
        let read = music_rules::editions_to_read(&counts, files.len(), EDITIONS_READ);
        for index in read.iter().copied() {
            let tracks = musicbrainz.get_release_tracks(&editions[index].id).await?;
            fits.push(music_rules::fit_listing(
                files,
                names,
                &tracks.iter().map(listing_track_facts).collect::<Vec<_>>(),
            ));
            listings.push(tracks);
        }
        // A deluxe edition's bonus disc on its own is nearer in count to the standard album, so
        // when the nearest editions fit badly, the next bigger one is read too.
        if fits
            .iter()
            .all(|fit| fit.share < music_rules::NEAREST_FIT_BADLY)
            && let Some(index) =
                music_rules::next_bigger_edition(&counts, &read, files.len() + held_tracks)
        {
            let tracks = musicbrainz.get_release_tracks(&editions[index].id).await?;
            fits.push(music_rules::fit_listing(
                files,
                names,
                &tracks.iter().map(listing_track_facts).collect::<Vec<_>>(),
            ));
            listings.push(tracks);
        }
        Ok(music_rules::best_listing(&fits).map(|best| {
            let tracks = listings.swap_remove(best);
            let fit = fits.swap_remove(best);
            let discs: Vec<i32> = tracks.iter().map(|track| track.disc_number).collect();
            let keep = music_rules::tracks_on_held_discs(&fit, &discs);
            let tracks = keep
                .into_iter()
                .map(|index| tracks[index].clone())
                .collect();
            (tracks, fit)
        }))
    }

    pub async fn add_album_from_provider(&self, options: AddAlbumOptions) -> Result<Album> {
        self.require_import_owner(options.library_id, options.user_id)
            .await?;
        // One at a time: two callers checking at once would both find nothing and both create it.
        static ADDING_ALBUM: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
        let _adding = ADDING_ALBUM.lock().await;
        if options.provider != MetadataProvider::Musicbrainz {
            anyhow::bail!("Only MusicBrainz is supported for album metadata");
        }

        let provider_id = options.provider_id.clone();
        let release_group_id = Uuid::parse_str(&options.provider_id)
            .map_err(|e| anyhow::anyhow!("Invalid MusicBrainz ID: {}", e))?;
        let musicbrainz = self.get_musicbrainz_client().await?;
        let release_group = musicbrainz.get_release_group(release_group_id).await?;

        let auth_user = AuthUser {
            user_id: options.user_id.to_string(),
            email: None,
            role: Some("admin".to_string()),
        };

        let existing = self
            .execute_query(
                &auth_user,
                r#"query AlbumByMusicbrainz($where: AlbumWhereInput, $page: PageInput) {
                    Albums: albums(where: $where, page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "libraryId": { "eq": options.library_id.to_string() },
                        "musicbrainzId": { "eq": provider_id.clone() }
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await?;

        if let Some(existing_id) = existing
            .get("Albums")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
        {
            let existing_id = existing_id.to_string();
            return Album::get(self.db.pool(), &existing_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Album not found after lookup"));
        }

        // An album with no credited artist is not added under a made-up "Unknown Artist": its files
        // would be filed in that folder. The caller keeps the album's own tags instead, and a later
        // scan can look again.
        let artist_name = release_group
            .artist_names()
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| {
                anyhow::Error::new(NoCreditedArtist {
                    release_group_id: provider_id.clone(),
                    title: release_group.title.clone(),
                })
            })?;
        let artist_mbid = release_group
            .artist_credit
            .as_ref()
            .and_then(|credits| credits.first())
            .map(|credit| credit.artist.id.to_string());

        let existing_artist = self
            .execute_query(
                &auth_user,
                r#"query ArtistLookup($where: ArtistWhereInput, $page: PageInput) {
                    Artists: artists(where: $where, page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                // MusicBrainz credits one artist with different spellings, so its id comes first.
                serde_json::json!({
                    "where": match artist_mbid.as_deref() {
                        Some(id) => serde_json::json!({
                            "libraryId": { "eq": options.library_id.to_string() },
                            "musicbrainzId": { "eq": id }
                        }),
                        None => serde_json::json!({
                            "libraryId": { "eq": options.library_id.to_string() },
                            "name": { "eq": artist_name.clone() }
                        }),
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await?;

        let artist_id = if let Some(id) = existing_artist
            .get("Artists")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string())
        {
            id
        } else {
            // Best effort and bounded: a slow or failing picture lookup never holds up or fails
            // the import. The post-scan backfill asks again for artists left without one.
            let image_url = match artist_mbid
                .as_deref()
                .and_then(|id| Uuid::parse_str(id).ok())
            {
                Some(id) => {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(25),
                        self.artist_image(Some(id), &artist_name),
                    )
                    .await
                    {
                        Ok(Ok(image)) => image,
                        Ok(Err(error)) => {
                            debug!(artist = %artist_name, error = %error, "No picture for new artist '{}' yet: {}", artist_name, error);
                            None
                        }
                        Err(_) => {
                            debug!(artist = %artist_name, "Picture lookup for new artist '{}' timed out", artist_name);
                            None
                        }
                    }
                }
                None => None,
            };
            let created = self
                .execute_mutation(
                    &auth_user,
                    r#"mutation CreateArtist($input: CreateArtistInput!) {
                        CreateArtist: createArtist(input: $input) { Success: success Error: error Artist: artist { Id: id } }
                    }"#,
                    serde_json::json!({
                        "input": {
                            "libraryId": options.library_id.to_string(),
                            "userId": options.user_id.to_string(),
                            "name": artist_name.clone(),
                            "sortName": artist_name.clone(),
                            "musicbrainzId": artist_mbid,
                            "imageUrl": image_url
                        }
                    }),
                )
                .await?;

            let success = created
                .get("CreateArtist")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !success {
                let err = created
                    .get("CreateArtist")
                    .and_then(|v| v.get("Error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Failed to create artist");
                anyhow::bail!(err.to_string());
            }

            created
                .get("CreateArtist")
                .and_then(|v| v.get("Artist"))
                .and_then(|v| v.get("Id"))
                .and_then(|id| id.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow::anyhow!("Artist not found after creation"))?
        };

        let cover_url = self.album_cover(release_group_id).await.ok().flatten();
        // Honour the caller's monitoring choice. Previously this was hardcoded
        // to `false`/`NONE`, so album auto-download could never fire.
        let auto_download = options.monitor_type != AutoDownloadMode::None;
        let created_album = self
            .execute_mutation(
                &auth_user,
                r#"mutation CreateAlbum($input: CreateAlbumInput!) {
                    CreateAlbum: createAlbum(input: $input) { Success: success Error: error Album: album { Id: id } }
                }"#,
                serde_json::json!({
                    "input": {
                        "artistId": artist_id,
                        "libraryId": options.library_id.to_string(),
                        "userId": options.user_id.to_string(),
                        "name": release_group.title,
                        "sortName": release_group.title,
                        "year": release_group.year(),
                        "musicbrainzId": provider_id.clone(),
                        "albumType": release_group.normalized_type(),
                        "genres": Vec::<String>::new(),
                        "autoDownload": auto_download,
                        "autoDownloadMode": auto_download_mode_graphql_name(options.monitor_type),
                        "hasFiles": false,
                        "coverUrl": cover_url.clone()
                    }
                }),
            )
            .await?;

        let success = created_album
            .get("CreateAlbum")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !success {
            let err = created_album
                .get("CreateAlbum")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to create album");
            anyhow::bail!(err.to_string());
        }

        let created_album_id = created_album
            .get("CreateAlbum")
            .and_then(|v| v.get("Album"))
            .and_then(|v| v.get("Id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| anyhow::anyhow!("Album not found after creation"))?;

        let created_album_id = created_album_id.to_string();
        let album = Album::get(self.db.pool(), &created_album_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Album not found after creation"))?;

        // An album with no Track rows has nothing to mark wanted and nothing
        // for the import matcher to link files to, so create the track list
        // from the provider. Tracks start `wanted = false`; the hunt job
        // promotes them according to the album's auto-download mode.
        let listing = match options.listing.clone() {
            Some(listing) => Ok(listing),
            None => musicbrainz.get_release_group_tracks(release_group_id).await,
        };
        let listing_release = listing
            .as_ref()
            .ok()
            .and_then(|tracks| tracks.first())
            .and_then(|track| track.release_id.clone());
        match listing {
            Ok(tracks) if !tracks.is_empty() => {
                let created = self
                    .create_album_tracks(&auth_user, &options, &album, &artist_name, &tracks)
                    .await?;
                info!(
                    album_id = %album.id,
                    album = %album.name,
                    artist = %artist_name,
                    tracks = created,
                    "Created {} track(s) for album '{}' by '{}' from MusicBrainz",
                    created,
                    album.name,
                    artist_name
                );
            }
            Ok(_) => warn!(
                album_id = %album.id,
                album = %album.name,
                artist = %artist_name,
                musicbrainz_id = %provider_id,
                "MusicBrainz returned no track listing for album '{}' by '{}' (release group {}); album added without tracks",
                album.name,
                artist_name,
                provider_id
            ),
            Err(error) => warn!(
                album_id = %album.id,
                album = %album.name,
                artist = %artist_name,
                error = %error,
                "Failed to fetch the MusicBrainz track listing for album '{}' by '{}': {}; album added without tracks",
                album.name,
                artist_name,
                error
            ),
        }

        // Credits of the release the listing came from, already read with it (Q130).
        if let Some(release_id) = listing_release
            && let Some(credits) = musicbrainz.cached_release_credits(&release_id)
        {
            self.store_album_credits(&album, &credits).await;
        }

        // Cache artwork, then emit a lightweight entity update so library/media
        // subscribers refresh once cached artwork is ready.
        if let Some(artwork_service) = self.artwork_service().await {
            let _ = artwork_service
                .cache_album_artwork(&album.id, cover_url.as_deref())
                .await;
        }

        Ok(album)
    }

    /// Create `Track` rows for a freshly added album through the generated
    /// GraphQL mutation (never raw SQL) so subscriptions fire.
    ///
    /// Tracks are created with `wanted = false`: the album's auto-download
    /// mode is what decides whether they become wanted, and that promotion is
    /// the hunt job's job, not this one's.
    async fn create_album_tracks(
        &self,
        auth_user: &AuthUser,
        options: &AddAlbumOptions,
        album: &Album,
        artist_name: &str,
        tracks: &[crate::services::metadata::musicbrainz::MusicBrainzTrack],
    ) -> Result<usize> {
        let mut created = 0usize;
        for track in tracks {
            let response = self
                .execute_mutation(
                    auth_user,
                    r#"mutation CreateAlbumTrack($input: CreateTrackInput!) {
                        CreateTrack: createTrack(input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({
                        "input": {
                            "albumId": album.id.clone(),
                            "libraryId": options.library_id.to_string(),
                            "artistId": album.artist_id.clone(),
                            "artistName": artist_name,
                            "title": track.title.clone(),
                            "trackNumber": track.position,
                            "discNumber": track.disc_number,
                            "durationSecs": track.duration_secs,
                            "musicbrainzId": track.recording_id.clone(),
                            "explicit": false,
                            "wanted": false
                        }
                    }),
                )
                .await?;
            let ok = response
                .get("CreateTrack")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if ok {
                created += 1;
            } else {
                warn!(
                    album_id = %album.id,
                    album = %album.name,
                    track = %track.title,
                    track_number = track.position,
                    error = %response
                        .get("CreateTrack")
                        .and_then(|v| v.get("Error"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("createTrack failed"),
                    "Failed to create track {} '{}' on album '{}': {}",
                    track.position,
                    track.title,
                    album.name,
                    response
                        .get("CreateTrack")
                        .and_then(|v| v.get("Error"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("createTrack failed")
                );
            }
        }
        Ok(created)
    }

    /// Add an audiobook from OpenLibrary to a library.
    pub async fn add_audiobook_from_provider(
        &self,
        options: AddAudiobookOptions,
    ) -> Result<Audiobook> {
        self.require_import_owner(options.library_id, options.user_id)
            .await?;
        if !matches!(
            options.provider,
            MetadataProvider::OpenLibrary | MetadataProvider::Audible
        ) {
            anyhow::bail!("Audiobook metadata comes from Audible or Open Library");
        }

        let details = self.get_audiobook(&options.provider_id).await?;
        let auth_user = AuthUser {
            user_id: options.user_id.to_string(),
            email: None,
            role: Some("admin".to_string()),
        };

        let existing = self
            .execute_query(
                &auth_user,
                r#"query AudiobookByProvider($where: AudiobookWhereInput, $page: PageInput) {
                    Audiobooks: audiobooks(where: $where, page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "libraryId": { "eq": options.library_id.to_string() },
                        "audibleId": { "eq": options.provider_id.clone() }
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await?;

        if let Some(existing_id) = existing
            .get("Audiobooks")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
        {
            let existing_id = existing_id.to_string();
            return Audiobook::get(self.db.pool(), &existing_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Audiobook not found after lookup"));
        }

        let created = self
            .execute_mutation(
                &auth_user,
                r#"mutation CreateAudiobook($input: CreateAudiobookInput!) {
                    CreateAudiobook: createAudiobook(input: $input) { Success: success Error: error Audiobook: audiobook { Id: id } }
                }"#,
                serde_json::json!({
                    "input": {
                        "libraryId": options.library_id.to_string(),
                        "userId": options.user_id.to_string(),
                        "title": details.title,
                        "sortTitle": details.sort_title,
                        "authorName": details.author_name,
                        "description": details.description,
                        "publisher": details.publisher,
                        "language": details.language,
                        "isbn": details.isbn,
                        "publishedDate": details.published_date,
                        "coverUrl": details.cover_url,
                        // The provider's id for the book, whichever provider that is.
                        "audibleId": details.provider_id,
                        "asin": (details.provider == MetadataProvider::Audible).then_some(&details.provider_id),
                        "narratorName": details.narrators.first(),
                        "totalDurationSecs": details.total_duration_secs,
                        "chapterCount": details.chapters.len(),
                        "seriesName": details.series_name,
                        "seriesPosition": details.series_position,
                        "autoDownload": options.monitor_type != AutoDownloadMode::None,
                        "autoDownloadMode": auto_download_mode_graphql_name(options.monitor_type),
                        "hasFiles": false,
                        "narrators": details.narrators
                    }
                }),
            )
            .await?;

        let success = created
            .get("CreateAudiobook")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !success {
            let err = created
                .get("CreateAudiobook")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to create audiobook");
            anyhow::bail!(err.to_string());
        }

        let created_audiobook_id = created
            .get("CreateAudiobook")
            .and_then(|v| v.get("Audiobook"))
            .and_then(|v| v.get("Id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| anyhow::anyhow!("Audiobook ID missing after creation"))?;

        let created_audiobook_id = created_audiobook_id.to_string();
        let audiobook = Audiobook::get(self.db.pool(), &created_audiobook_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Audiobook not found after creation"))?;

        // Chapters come only from real provider data (docs/design.md Q52): Audnexus has them, Open
        // Library doesn't, and made-up rows would never line up with a real release. Import still
        // links the release's files once chapters exist.
        let wanted = options.monitor_type != AutoDownloadMode::None;
        for (index, chapter) in details.chapters.iter().enumerate() {
            let created = self
                .execute_mutation(
                    &auth_user,
                    r#"mutation CreateChapter($input: CreateChapterInput!) {
                        CreateChapter: createChapter(input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({
                        "input": {
                            "audiobookId": audiobook.id,
                            "chapterNumber": index as i32 + 1,
                            "title": chapter.title,
                            "startTimeSecs": chapter.start_secs,
                            "endTimeSecs": chapter.start_secs + chapter.length_secs,
                            "durationSecs": chapter.length_secs.round() as i32,
                            "wanted": wanted,
                        }
                    }),
                )
                .await;
            if let Err(error) = created {
                warn!(audiobook_id = %audiobook.id, chapter = index + 1, error = %error, "Could not create audiobook chapter");
            }
        }
        self.store_audiobook_credits(&audiobook, &details.authors)
            .await;
        if details.chapters.is_empty() {
            warn!(
                audiobook_id = %audiobook.id,
                title = %audiobook.title,
                provider = ?details.provider,
                "No chapter listing available for audiobook '{}'; a scan can create them from its files",
                audiobook.title
            );
        }

        // Cache artwork, then emit a lightweight entity update so library/media
        // subscribers refresh once cached artwork is ready.
        if let Some(artwork_service) = self.artwork_service().await {
            let _ = artwork_service
                .cache_audiobook_artwork(&audiobook.id, details.cover_url.as_deref())
                .await;
        }

        Ok(audiobook)
    }

    /// Refresh movie metadata from provider and persist via generated GraphQL UpdateMovie mutation.
    pub async fn refresh_movie_from_provider(
        &self,
        movie_id: &str,
        user_id: Uuid,
    ) -> Result<Movie> {
        let movie_id_string = movie_id.to_string();
        let movie = Movie::get(self.db.pool(), &movie_id_string)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Movie not found"))?;

        if movie.user_id != user_id.to_string() {
            anyhow::bail!("Movie not found");
        }

        let tmdb_id = movie
            .tmdb_id
            .ok_or_else(|| anyhow::anyhow!("Movie has no TmdbId"))?;

        let details = self
            .get_movie_with_policy(tmdb_id as u32, MetadataFetchPolicy::ForceRefresh)
            .await?;
        let cached_collection_poster_url = self
            .cache_collection_poster_artwork(
                details.collection_id,
                details.collection_poster_url.as_deref(),
            )
            .await
            .or_else(|| details.collection_poster_url.clone());

        let auth_user = AuthUser {
            user_id: user_id.to_string(),
            email: None,
            role: Some("admin".to_string()),
        };

        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation RefreshMovie($id: String!, $input: UpdateMovieInput!) {
                    UpdateMovie: updateMovie(id: $id, input: $input) { Success: success Error: error }
                }"#,
                serde_json::json!({
                    "id": movie.id,
                    "input": write_rules::without_locked_fields(serde_json::json!({
                        "title": details.title,
                        "sortTitle": write_rules::sort_key(&details.title),
                        "originalTitle": details.original_title,
                        "year": details.year,
                        "tmdbId": details.provider_id as i32,
                        "imdbId": details.imdb_id,
                        "overview": details.overview,
                        "tagline": details.tagline,
                        "runtime": details.runtime,
                        "genres": details.genres,
                        "director": details.director,
                        "castNames": details.cast_names,
                        "productionCountries": details.production_countries,
                        "spokenLanguages": details.spoken_languages,
                        "tmdbRating": details.vote_average.map(|v| v.to_string()),
                        "tmdbVoteCount": details.vote_count,
                        "releaseDate": details.release_date,
                        "certification": details.certification,
                        "digitalReleaseDate": details.digital_release_date,
                        "physicalReleaseDate": details.physical_release_date,
                        "collectionId": details.collection_id,
                        "collectionName": details.collection_name,
                        "collectionPosterUrl": cached_collection_poster_url,
                        "tmdbStatus": details.tmdb_status,
                    }), movie.locked_fields.as_deref().unwrap_or_default())
                }),
            )
            .await?;

        let updated = data
            .get("UpdateMovie")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !updated {
            let err = data
                .get("UpdateMovie")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to refresh movie");
            anyhow::bail!(err.to_string());
        }

        let movie_id_string = movie_id.to_string();
        let movie = Movie::get(self.db.pool(), &movie_id_string)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Movie not found after refresh"))?;

        if let Some(collection_id) = details.collection_id
            && let Err(error) = self
                .ensure_movie_collection_entity(
                    Uuid::parse_str(&movie.library_id)?,
                    user_id,
                    collection_id,
                )
                .await
        {
            warn!(
                movie_id = %movie.id,
                library_id = %movie.library_id,
                user_id = %user_id,
                collection_id = collection_id,
                error = %error,
                "Failed to ensure collection entity after movie refresh"
            );
        }
        self.store_movie_extras(&movie, details.extras.as_ref())
            .await;

        // Refresh cached artwork, then emit a lightweight entity update so
        // subscribers refresh once cached artwork is ready.
        if let Some(artwork_service) = self.artwork_service().await {
            let _ = artwork_service
                .cache_movie_artwork(
                    &movie.id,
                    details.poster_url.as_deref(),
                    details.backdrop_url.as_deref(),
                )
                .await;
        }

        Ok(movie)
    }

    async fn cache_collection_poster_artwork(
        &self,
        collection_id: Option<i32>,
        collection_poster_url: Option<&str>,
    ) -> Option<String> {
        self.cache_collection_artwork(collection_id, collection_poster_url, "poster")
            .await
    }

    async fn cache_collection_backdrop_artwork(
        &self,
        collection_id: Option<i32>,
        collection_backdrop_url: Option<&str>,
    ) -> Option<String> {
        self.cache_collection_artwork(collection_id, collection_backdrop_url, "backdrop")
            .await
    }

    async fn cache_collection_artwork(
        &self,
        collection_id: Option<i32>,
        source_url: Option<&str>,
        artwork_type: &str,
    ) -> Option<String> {
        let collection_id = collection_id?;
        let source_url = source_url?;
        let artwork_service = self.artwork_service().await?;
        match artwork_service
            .cache_image(
                source_url,
                "collection",
                &collection_id.to_string(),
                artwork_type,
            )
            .await
        {
            Ok(cached_url) => Some(cached_url),
            Err(error) => {
                warn!(
                    collection_id = collection_id,
                    artwork_type = artwork_type,
                    error = %error,
                    "Failed to cache collection artwork"
                );
                None
            }
        }
    }

    async fn upsert_movie_collection_entity(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        collection_id: i32,
    ) -> Result<()> {
        let auth_user = AuthUser {
            user_id: user_id.to_string(),
            email: None,
            role: Some("admin".to_string()),
        };

        let existing = self
            .execute_query(
                &auth_user,
                r#"query CollectionByTmdb($where: CollectionWhereInput, $page: PageInput) {
                    Collections: collections(where: $where, page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "libraryId": { "eq": library_id.to_string() },
                        "tmdbCollectionId": { "eq": collection_id }
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await?;

        let existing_id = existing
            .get("Collections")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string());

        if let Some(existing_entity_id) = existing_id {
            debug!(
                library_id = %library_id,
                user_id = %user_id,
                collection_id = collection_id,
                collection_entity_id = %existing_entity_id,
                "Skipping TMDB collection lookup because collection entity already exists: library_id={}, user_id={}, collection_id={}, collection_entity_id={}",
                library_id,
                user_id,
                collection_id,
                existing_entity_id
            );
            return Ok(());
        }

        info!(
            library_id = %library_id,
            user_id = %user_id,
            collection_id = collection_id,
            "Attempting movie collection upsert from TMDB: library_id={}, user_id={}, collection_id={}",
            library_id,
            user_id,
            collection_id
        );
        let tmdb = self.get_tmdb_client().await?;
        let details = tmdb.get_collection(collection_id).await?;
        let poster_url = tmdb.poster_url(details.poster_path.as_deref());
        let backdrop_url = tmdb.backdrop_url(details.backdrop_path.as_deref());
        let cached_poster_url = self
            .cache_collection_poster_artwork(Some(collection_id), poster_url.as_deref())
            .await
            .or(poster_url);
        let cached_backdrop_url = self
            .cache_collection_backdrop_artwork(Some(collection_id), backdrop_url.as_deref())
            .await
            .or(backdrop_url);

        let create = self
            .execute_mutation(
                &auth_user,
                r#"mutation CreateCollectionFromTmdb($input: CreateCollectionInput!) {
                    CreateCollection: createCollection(input: $input) { Success: success Error: error Collection: collection { Id: id } }
                }"#,
                serde_json::json!({
                    "input": {
                        "libraryId": library_id.to_string(),
                        "userId": user_id.to_string(),
                        "tmdbCollectionId": collection_id,
                        "name": details.name,
                        "overview": details.overview,
                        "posterUrl": cached_poster_url,
                        "backdropUrl": cached_backdrop_url,
                        "movieCount": details.parts.len() as i32,
                        "lastSyncedAt": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
                    }
                }),
            )
            .await?;

        let success = create
            .get("CreateCollection")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !success {
            let error = create
                .get("CreateCollection")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error creating collection");
            anyhow::bail!(error.to_string());
        }
        let created_id = create
            .get("CreateCollection")
            .and_then(|v| v.get("Collection"))
            .and_then(|v| v.get("Id"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        info!(
            library_id = %library_id,
            user_id = %user_id,
            collection_id = collection_id,
            collection_entity_id = %created_id,
            collection_name = %details.name,
            movie_count = details.parts.len(),
            "Created movie collection entity from TMDB: library_id={}, user_id={}, collection_id={}, collection_entity_id={}, collection_name='{}', movie_count={}",
            library_id,
            user_id,
            collection_id,
            created_id,
            details.name,
            details.parts.len()
        );

        Ok(())
    }

    async fn movie_exists_in_library_by_tmdb(
        &self,
        auth_user: &AuthUser,
        library_id: Uuid,
        tmdb_id: i32,
    ) -> Result<bool> {
        Ok(self
            .movie_id_in_library_by_tmdb(auth_user, library_id, tmdb_id)
            .await?
            .is_some())
    }

    async fn movie_id_in_library_by_tmdb(
        &self,
        auth_user: &AuthUser,
        library_id: Uuid,
        tmdb_id: i32,
    ) -> Result<Option<String>> {
        let data = self
            .execute_query(
                auth_user,
                r#"query MovieByTmdbInLibrary($where: MovieWhereInput, $page: PageInput) {
                    Movies: movies(where: $where, page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "libraryId": { "eq": library_id.to_string() },
                        "tmdbId": { "eq": tmdb_id }
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await?;

        let movie_id = data
            .get("Movies")
            .and_then(|m| m.get("Edges"))
            .and_then(|e| e.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
            .map(|id| id.to_string());

        Ok(movie_id)
    }

    /// Refresh TV show metadata from provider and persist via generated GraphQL UpdateShow mutation.
    pub async fn refresh_tv_show_from_provider(
        &self,
        show_id: &str,
        user_id: Uuid,
    ) -> Result<Show> {
        let show_id = show_id.to_string();
        let show = Show::get(self.db.pool(), &show_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Show not found"))?;

        if show.user_id != user_id.to_string() {
            anyhow::bail!("Show not found");
        }

        let tvmaze_id = show
            .tvmaze_id
            .ok_or_else(|| anyhow::anyhow!("Show has no TvmazeId"))?;

        let details = self
            .get_tv_show_with_policy(tvmaze_id as u32, MetadataFetchPolicy::ForceRefresh)
            .await?;

        let auth_user = AuthUser {
            user_id: user_id.to_string(),
            email: None,
            role: Some("admin".to_string()),
        };

        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation RefreshShow($id: String!, $input: UpdateShowInput!) {
                    UpdateShow: updateShow(id: $id, input: $input) { Success: success Error: error }
                }"#,
                serde_json::json!({
                    "id": show.id,
                    "input": write_rules::without_locked_fields(serde_json::json!({
                        "name": details.name,
                        "sortName": write_rules::sort_key(&details.name),
                        "year": details.year,
                        "tvmazeId": details.provider_id as i32,
                        "tvdbId": details.tvdb_id,
                        "imdbId": details.imdb_id,
                        "overview": details.overview,
                        "network": details.network,
                        "runtime": details.runtime,
                        "genres": details.genres,
                        "posterUrl": details.poster_url,
                        "backdropUrl": details.backdrop_url,
                        "status": details.status,
                    }), show.locked_fields.as_deref().unwrap_or_default())
                }),
            )
            .await?;

        let updated = data
            .get("UpdateShow")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !updated {
            let err = data
                .get("UpdateShow")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to refresh show");
            anyhow::bail!(err.to_string());
        }

        let show_id = show_id.to_string();
        let show = Show::get(self.db.pool(), &show_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Show not found after refresh"))?;

        // Keep episodes in sync with provider metadata via generated mutations.
        self.sync_show_episodes_from_tvmaze(
            &auth_user,
            &show.id,
            tvmaze_id as u32,
            show.auto_download,
        )
        .await?;
        self.sync_show_seasons_best_effort(&auth_user, &show.id, tvmaze_id as u32)
            .await;
        // Cast and crew of the show, its seasons and episodes, and its details (Q130).
        if self.has_tmdb().await
            && let Err(error) = self.sync_show_credits(&show).await
        {
            warn!(
                show_id = %show.id,
                show = %show.name,
                error = %error,
                "Could not refresh credits of show '{}': {error}",
                show.name
            );
        }

        // Refresh cached artwork, then emit a lightweight entity update so
        // subscribers refresh once cached artwork is ready.
        if let Some(artwork_service) = self.artwork_service().await {
            let _ = artwork_service
                .cache_show_artwork(
                    &show.id,
                    details.poster_url.as_deref(),
                    details.backdrop_url.as_deref(),
                )
                .await;
        }

        Ok(show)
    }
}

/// A listed track in the terms the matching rules use.
pub fn listing_track_facts(
    track: &crate::services::metadata::musicbrainz::MusicBrainzTrack,
) -> crate::services::release_match::TrackFacts {
    crate::services::release_match::TrackFacts {
        title: track.title.clone(),
        number: Some(track.position),
        disc: Some(track.disc_number),
        seconds: track.duration_secs.map(f64::from),
    }
}

/// Options for creating a movie from metadata
#[derive(Debug, Clone)]
pub struct CreateMovieFromMetadataOptions {
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub monitored: bool,
}

/// Credits, people and title details for every kind of title (Q129–Q131).
#[path = "title_credits.rs"]
mod title_credits;
pub use title_credits::{
    CreditFillBudget, CreditFillReport, MOVIE_APPEND, credit_fill_requested, request_credit_fill,
};

/// Create a sharable metadata service
pub fn create_metadata_service(
    db: Database,
    services: Arc<ServicesManager>,
    config: MetadataServiceConfig,
) -> Arc<MetadataService> {
    Arc::new(MetadataService::new(db, services, config))
}

#[cfg(test)]
mod guide_cache_tests {
    use super::super::tmdb::MovieReleaseKind;
    use super::*;
    use crate::services::{AuthConfig, DatabaseServiceConfig, GraphqlServiceConfig};

    #[tokio::test]
    async fn guide_release_cache_is_region_page_and_kind_specific() {
        let temp = tempfile::tempdir().unwrap();
        let services = ServicesManager::builder()
            .add_service(DatabaseServiceConfig {
                database_url: format!("sqlite://{}", temp.path().join("releases.db").display()),
                connect_timeout: std::time::Duration::from_secs(5),
            })
            .add_service(AuthConfig::for_tests())
            .add_service(GraphqlServiceConfig { server_port: 0 })
            .start()
            .await
            .unwrap();
        let service = services.get_database().await.unwrap();
        let metadata = MetadataService::new_default(service.pool().clone(), services.clone());
        let fixture = vec![MovieSearchResult {
            provider: MetadataProvider::Tmdb,
            provider_id: 42,
            title: "Upcoming fixture".into(),
            original_title: None,
            year: Some(2026),
            release_date: Some("2026-10-01".into()),
            overview: None,
            poster_url: None,
            backdrop_url: None,
            imdb_id: None,
            vote_average: None,
            popularity: None,
        }];
        metadata
            .upsert_metadata_cache(
                MetadataProvider::Tmdb,
                "MovieReleases",
                "kind=upcoming;region=GB;page=2",
                &fixture,
            )
            .await
            .unwrap();
        let results = metadata
            .movie_releases(MovieReleaseKind::Upcoming, Some("gb".into()), 2)
            .await
            .unwrap();
        assert_eq!(results[0].provider_id, 42);
        assert_eq!(results[0].release_date.as_deref(), Some("2026-10-01"));
        // No TMDB key is configured: these distinct requests must miss the cached fixture.
        assert!(
            metadata
                .movie_releases(MovieReleaseKind::Upcoming, Some("US".into()), 2)
                .await
                .is_err()
        );
        assert!(
            metadata
                .movie_releases(MovieReleaseKind::NowPlaying, Some("GB".into()), 2)
                .await
                .is_err()
        );
        assert!(
            metadata
                .movie_releases(MovieReleaseKind::Upcoming, Some("GB".into()), 1)
                .await
                .is_err()
        );
        assert!(
            metadata
                .movie_releases(MovieReleaseKind::Upcoming, Some("invalid".into()), 2)
                .await
                .is_err()
        );
        assert!(
            metadata
                .movie_releases(MovieReleaseKind::Upcoming, None, 0)
                .await
                .is_err()
        );
        assert!(!MetadataService::is_cache_stale(
            &chrono::Utc::now().timestamp().to_string(),
            1
        ));
        assert!(MetadataService::is_cache_stale(
            &(chrono::Utc::now() - chrono::Duration::days(2)).to_rfc3339(),
            1
        ));
        services.stop_all().await.unwrap();
    }
}

#[cfg(test)]
#[path = "providers_alignment_tests.rs"]
mod alignment_tests;

#[cfg(test)]
mod timestamp_notification_tests {
    use super::*;
    use crate::graphql::entities::{
        CreateLibraryInput, CreateMovieInput, CreateUserInput, Library, Movie, MovieMutations,
        UpdateMovieInput, User,
    };
    use graphql_orm::DbPool;

    async fn database() -> Database {
        let db = Database::new(DbPool::connect("sqlite::memory:").await.unwrap());
        crate::services::database::bootstrap_schema(&db)
            .await
            .unwrap();
        crate::services::DatabaseService::new(db).pool().clone()
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
                phone: None,
                genre_interests: None,
                must_change_password: false,
                two_factor_enabled_at: None,
                recovery_codes_remaining: 0,
                password_changed_at: None,
                totp_secret: None,
                totp_pending_secret: None,
                recovery_codes: None,
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
                auto_resolve_duplicates: None,
                naming_pattern: "fixture naming_pattern".to_string(),
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
        .unwrap()
    }

    async fn movie(db: &Database, library: &Library) -> Movie {
        Movie::insert(
            db,
            CreateMovieInput {
                locked_fields: None,
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
                digital_release_date: None,
                physical_release_date: None,
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
    #[derive(Default)]
    struct TouchQuery;
    #[async_graphql::Object]
    impl TouchQuery {
        async fn ready(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn generated_timestamp_notification_preserves_a_newer_title_and_owner() {
        let db = database().await;
        let owner = user(&db).await;
        let lib = library(&db, &owner).await;
        let stale = movie(&db, &lib).await;
        Movie::update_by_id(
            &db,
            &stale.id,
            UpdateMovieInput {
                title: Some("newer title".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let schema = async_graphql::Schema::build(
            TouchQuery,
            MovieMutations,
            async_graphql::EmptySubscription,
        )
        .data(db.clone())
        .finish();
        let actor = AuthUser::system_admin_for(&owner.id);
        let response = schema.execute(Request::new(
            "mutation Notify($id: String!, $input: UpdateMovieInput!) { updateMovie(id: $id, input: $input) { success error } }")
            .variables(Variables::from_json(timestamp_touch_variables(&stale.id)))
            .data(graphql_orm::graphql::auth::AuthSubject::from_parts(actor.user_id.clone(), vec!["admin".to_string()], Vec::new(), None))
            .data(actor.clone()).data(actor.user_id)).await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        check_mutation_payloads(&response.data.into_json().unwrap()).unwrap();
        let current = Movie::get(db.pool(), &stale.id).await.unwrap().unwrap();
        assert_eq!(current.title, "newer title");
        assert_eq!(current.user_id, owner.id);
        assert_eq!(current.library_id, lib.id);
    }

    #[test]
    fn notification_payload_failure_is_not_reported_as_success() {
        for invalid in [
            serde_json::json!({}),
            serde_json::json!({"UpdateMovie": null}),
            serde_json::json!({"UpdateMovie": {"Success": false, "Error": "denied"}}),
            serde_json::json!({"updateShow": {"success": false, "error": "not found"}}),
        ] {
            assert!(check_mutation_payloads(&invalid).is_err());
        }
        assert!(
            check_mutation_payloads(&serde_json::json!({"UpdateMovie": {"Success": true}})).is_ok()
        );
    }
}
