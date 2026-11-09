//! Credits, people and title facts in one normalized shape, whatever provider they came from
//! (`docs/design.md` Q129–Q132).
//!
//! This module only parses: TMDB documents (movie, show, season, person) and MusicBrainz
//! release relationships become [`CreditEntry`], [`PersonRef`] and [`TitleFacts`] values. Saving
//! them is `credit_store`. Everything here is pure, so the tests read recorded provider JSON from
//! `tests/fixtures/credits/` and never call a provider.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// TMDB serves pictures from here; `original` keeps the largest copy for the artwork cache.
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p/original";

/// The most cast and crew kept for one title. A long-running show lists thousands of crew;
/// past this the rest are minor roles nobody browses to, and every one costs writes.
pub const MAX_CAST: usize = 150;
pub const MAX_CREW: usize = 150;

/// Who a credit names, with every id a provider gave for them. People are matched across
/// providers by these ids (TMDB, then IMDb, MusicBrainz and Wikidata), never by name alone,
/// except that a person with no id at all is the same person as another with the same name and
/// no id (a book's narrator credited on two books).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonRef {
    pub name: String,
    pub tmdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub wikidata_id: Option<String>,
    pub profile_url: Option<String>,
    pub known_for_department: Option<String>,
}

impl PersonRef {
    pub fn named(name: &str) -> Self {
        Self {
            name: name.trim().to_string(),
            ..Default::default()
        }
    }

    pub fn has_external_id(&self) -> bool {
        self.tmdb_id.is_some_and(|id| id > 0)
            || self.imdb_id.is_some()
            || self.musicbrainz_id.is_some()
            || self.wikidata_id.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CreditKind {
    Cast,
    Crew,
}

impl CreditKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cast => "cast",
            Self::Crew => "crew",
        }
    }
}

/// One credit as a provider gave it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditEntry {
    pub person: PersonRef,
    pub kind: CreditKind,
    pub guest: bool,
    pub department: Option<String>,
    pub job: Option<String>,
    pub character: Option<String>,
    pub order: Option<i32>,
    pub episode_count: Option<i32>,
    pub provider_credit_id: Option<String>,
}

impl CreditEntry {
    fn cast(person: PersonRef, character: Option<String>, order: Option<i32>) -> Self {
        Self {
            person,
            kind: CreditKind::Cast,
            guest: false,
            department: Some("Acting".into()),
            job: None,
            character: blank_to_none(character),
            order,
            episode_count: None,
            provider_credit_id: None,
        }
    }

    fn crew(person: PersonRef, department: Option<String>, job: Option<String>) -> Self {
        Self {
            person,
            kind: CreditKind::Crew,
            guest: false,
            department: blank_to_none(department),
            job: blank_to_none(job),
            character: None,
            order: None,
            episode_count: None,
            provider_credit_id: None,
        }
    }

    /// What makes two credits the same credit on one title: a refresh updates the match in place
    /// and only adds or removes what really changed.
    pub fn identity(&self) -> (CreditKind, bool, String, String) {
        (
            self.kind,
            self.guest,
            self.job.clone().unwrap_or_default().to_lowercase(),
            self.character.clone().unwrap_or_default().to_lowercase(),
        )
    }
}

/// A rating in one country: a film's certification or a show's content rating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentRating {
    pub country: String,
    pub rating: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlternativeTitle {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// A trailer, teaser or featurette: a link to where the provider says it is hosted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoLink {
    pub site: String,
    pub key: String,
    pub url: Option<String>,
    pub name: String,
    pub kind: Option<String>,
    pub official: bool,
    pub language: Option<String>,
    pub published_at: Option<String>,
}

/// Everything about a title that is not a credit (stored as a `TitleDetail`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitleFacts {
    pub provider_id: Option<String>,
    pub studios: Vec<String>,
    pub networks: Vec<String>,
    pub keywords: Vec<String>,
    pub origin_countries: Vec<String>,
    pub content_ratings: Vec<ContentRating>,
    pub alternative_titles: Vec<AlternativeTitle>,
    pub videos: Vec<VideoLink>,
    pub external_ids: BTreeMap<String, String>,
    pub homepage: Option<String>,
}

/// One title's credits and facts from one provider document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderTitle {
    pub facts: TitleFacts,
    pub credits: Vec<CreditEntry>,
}

/// A TMDB season document: the season's own credits, and each episode's crew and guest stars.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeasonCredits {
    pub season_number: i32,
    pub credits: Vec<CreditEntry>,
    pub episodes: Vec<EpisodeCredits>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EpisodeCredits {
    pub episode_number: i32,
    pub tmdb_episode_id: Option<i32>,
    pub credits: Vec<CreditEntry>,
    pub facts: TitleFacts,
}

/// A person's own record (TMDB `/person/{id}`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonDetails {
    pub person: PersonRef,
    pub biography: Option<String>,
    pub birthday: Option<String>,
    pub deathday: Option<String>,
    pub place_of_birth: Option<String>,
    pub also_known_as: Vec<String>,
    pub image_urls: Vec<String>,
}

// ---------------------------------------------------------------------------------------------
// Names and stand-in ids

/// A name folded for comparison: lower case, letters and digits only, single spaces.
pub fn name_key(name: &str) -> String {
    let mut key = String::with_capacity(name.len());
    let mut space = false;
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            if space && !key.is_empty() {
                key.push(' ');
            }
            space = false;
            key.push(c);
        } else {
            space = true;
        }
    }
    key
}

/// `Person.tmdbPersonId` is a required, unique column from before people came from anywhere but
/// TMDB. A person with no TMDB id gets a negative number derived from their own key instead
/// (FNV-1a, folded into `-1 ..= -2^31+1`), so the same key always lands on the same row and
/// never on a real TMDB id. `attempt` moves to the next candidate on the rare collision.
pub fn stand_in_tmdb_id(key: &str, attempt: u32) -> i32 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in key.bytes().chain(attempt.to_le_bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    -(((hash % (i32::MAX as u64 - 1)) as i32) + 1)
}

/// The key a stand-in id is derived from: the strongest id the person has, else their name.
pub fn person_stand_in_key(person: &PersonRef) -> String {
    if let Some(id) = &person.musicbrainz_id {
        format!("mb:{id}")
    } else if let Some(id) = &person.imdb_id {
        format!("imdb:{id}")
    } else if let Some(id) = &person.wikidata_id {
        format!("wd:{id}")
    } else {
        format!("name:{}", name_key(&person.name))
    }
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn tmdb_image(path: Option<&str>) -> Option<String> {
    path.filter(|p| !p.is_empty())
        .map(|p| format!("{TMDB_IMAGE_BASE}{p}"))
}

fn push_unique(list: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|v| !v.is_empty())
        && !list.iter().any(|existing| existing == value)
    {
        list.push(value.to_string());
    }
}

// ---------------------------------------------------------------------------------------------
// TMDB documents

#[derive(Debug, Default, Deserialize)]
struct TmdbPersonCredit {
    id: i32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    profile_path: Option<String>,
    #[serde(default)]
    known_for_department: Option<String>,
    #[serde(default)]
    character: Option<String>,
    #[serde(default)]
    order: Option<i32>,
    #[serde(default)]
    credit_id: Option<String>,
    #[serde(default)]
    department: Option<String>,
    #[serde(default)]
    job: Option<String>,
    /// `aggregate_credits` cast: one entry per character played.
    #[serde(default)]
    roles: Vec<TmdbAggregateRole>,
    /// `aggregate_credits` crew: one entry per job.
    #[serde(default)]
    jobs: Vec<TmdbAggregateJob>,
    #[serde(default)]
    total_episode_count: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
struct TmdbAggregateRole {
    #[serde(default)]
    credit_id: Option<String>,
    #[serde(default)]
    character: Option<String>,
    #[serde(default)]
    episode_count: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
struct TmdbAggregateJob {
    #[serde(default)]
    credit_id: Option<String>,
    #[serde(default)]
    job: Option<String>,
    #[serde(default)]
    episode_count: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
struct TmdbCreditList {
    #[serde(default)]
    cast: Vec<TmdbPersonCredit>,
    #[serde(default)]
    crew: Vec<TmdbPersonCredit>,
}

impl TmdbPersonCredit {
    fn person(&self) -> PersonRef {
        PersonRef {
            name: self.name.trim().to_string(),
            tmdb_id: Some(self.id),
            profile_url: tmdb_image(self.profile_path.as_deref()),
            known_for_department: blank_to_none(self.known_for_department.clone()),
            ..Default::default()
        }
    }
}

/// Crew departments in the order a page lists them: the ones people look for first.
fn department_rank(department: Option<&str>, job: Option<&str>) -> u8 {
    match (department.unwrap_or_default(), job.unwrap_or_default()) {
        (_, "Director") => 0,
        ("Writing", _) | (_, "Creator") => 1,
        (_, "Executive Producer") | (_, "Producer") => 2,
        ("Directing", _) => 3,
        ("Production", _) => 4,
        ("Sound", "Original Music Composer" | "Music") => 5,
        ("Camera", "Director of Photography") | ("Editing", "Editor") => 6,
        _ => 9,
    }
}

/// Credits from a `credits` document (movie, episode guest stars come from elsewhere).
fn plain_credits(list: TmdbCreditList) -> Vec<CreditEntry> {
    let mut cast: Vec<CreditEntry> = list
        .cast
        .iter()
        .filter(|member| !member.name.trim().is_empty())
        .map(|member| {
            let mut credit =
                CreditEntry::cast(member.person(), member.character.clone(), member.order);
            credit.provider_credit_id = member.credit_id.clone();
            credit
        })
        .collect();
    cast.sort_by_key(|credit| credit.order.unwrap_or(i32::MAX));
    let crew = list
        .crew
        .iter()
        .filter(|member| !member.name.trim().is_empty())
        .map(|member| {
            let mut credit = CreditEntry::crew(
                member.person(),
                member.department.clone(),
                member.job.clone(),
            );
            credit.provider_credit_id = member.credit_id.clone();
            credit
        })
        .collect();
    finish_credits(cast, crew)
}

/// Credits from an `aggregate_credits` document (shows and seasons): one cast credit per
/// character and one crew credit per job, each with its episode count.
fn aggregate_credits(list: TmdbCreditList) -> Vec<CreditEntry> {
    let mut cast = Vec::new();
    for member in list
        .cast
        .iter()
        .filter(|member| !member.name.trim().is_empty())
    {
        let roles: Vec<&TmdbAggregateRole> = member.roles.iter().collect();
        if roles.is_empty() {
            let mut credit =
                CreditEntry::cast(member.person(), member.character.clone(), member.order);
            credit.episode_count = member.total_episode_count;
            cast.push(credit);
            continue;
        }
        for role in roles {
            let mut credit =
                CreditEntry::cast(member.person(), role.character.clone(), member.order);
            credit.episode_count = role.episode_count.or(member.total_episode_count);
            credit.provider_credit_id = role.credit_id.clone();
            cast.push(credit);
        }
    }
    cast.sort_by_key(|credit| credit.order.unwrap_or(i32::MAX));
    let mut crew = Vec::new();
    for member in list
        .crew
        .iter()
        .filter(|member| !member.name.trim().is_empty())
    {
        if member.jobs.is_empty() {
            let mut credit = CreditEntry::crew(
                member.person(),
                member.department.clone(),
                member.job.clone(),
            );
            credit.episode_count = member.total_episode_count;
            crew.push(credit);
            continue;
        }
        for job in &member.jobs {
            let mut credit =
                CreditEntry::crew(member.person(), member.department.clone(), job.job.clone());
            credit.episode_count = job.episode_count.or(member.total_episode_count);
            credit.provider_credit_id = job.credit_id.clone();
            crew.push(credit);
        }
    }
    // Most episodes first within a department, so the people who shaped the show lead.
    crew.sort_by_key(|credit| std::cmp::Reverse(credit.episode_count.unwrap_or(0)));
    finish_credits(cast, crew)
}

/// Drop repeats (a provider sometimes lists a person twice for one job), keep the crew that
/// matters first, and cap both lists. Cast keeps its billing order; crew gets an order too, so
/// a page can list it in the same sequence.
fn finish_credits(cast: Vec<CreditEntry>, mut crew: Vec<CreditEntry>) -> Vec<CreditEntry> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(cast.len().min(MAX_CAST) + crew.len().min(MAX_CREW));
    let mut kept = 0;
    for (index, mut credit) in cast.into_iter().enumerate() {
        if kept >= MAX_CAST {
            break;
        }
        let key = (
            credit.person.tmdb_id,
            credit.person.name.clone(),
            credit.identity(),
        );
        if !seen.insert(key) {
            continue;
        }
        credit.order = credit.order.or(Some(index as i32));
        out.push(credit);
        kept += 1;
    }
    crew.sort_by_key(|credit| department_rank(credit.department.as_deref(), credit.job.as_deref()));
    let mut kept = 0;
    for (index, mut credit) in crew.into_iter().enumerate() {
        if kept >= MAX_CREW {
            break;
        }
        let key = (
            credit.person.tmdb_id,
            credit.person.name.clone(),
            credit.identity(),
        );
        if !seen.insert(key) {
            continue;
        }
        credit.order = Some(index as i32);
        out.push(credit);
        kept += 1;
    }
    out
}

#[derive(Debug, Default, Deserialize)]
struct Named {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct TmdbVideo {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    site: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    official: Option<bool>,
    #[serde(default)]
    iso_639_1: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
}

fn video_url(site: &str, key: &str) -> Option<String> {
    match site.to_ascii_lowercase().as_str() {
        "youtube" => Some(format!("https://www.youtube.com/watch?v={key}")),
        "vimeo" => Some(format!("https://vimeo.com/{key}")),
        _ => None,
    }
}

fn parse_videos(value: Option<&Value>) -> Vec<VideoLink> {
    let Some(results) = value.and_then(|v| v.get("results")) else {
        return Vec::new();
    };
    let videos: Vec<TmdbVideo> = serde_json::from_value(results.clone()).unwrap_or_default();
    let mut out: Vec<VideoLink> = videos
        .into_iter()
        .filter_map(|video| {
            let site = video.site?.trim().to_string();
            let key = video.key?.trim().to_string();
            if site.is_empty() || key.is_empty() {
                return None;
            }
            Some(VideoLink {
                url: video_url(&site, &key),
                site,
                key,
                name: video.name.unwrap_or_default(),
                kind: video.kind,
                official: video.official.unwrap_or(false),
                language: video.iso_639_1,
                published_at: video.published_at,
            })
        })
        .collect();
    // Official trailers first, then everything else; newest first within each.
    out.sort_by(|a, b| {
        let rank = |v: &VideoLink| {
            (
                v.kind.as_deref() != Some("Trailer"),
                !v.official,
                std::cmp::Reverse(v.published_at.clone()),
            )
        };
        rank(a).cmp(&rank(b))
    });
    out
}

fn parse_keywords(value: Option<&Value>) -> Vec<String> {
    // Movies put them under `keywords`, shows under `results`.
    let list = value.and_then(|v| v.get("keywords").or_else(|| v.get("results")));
    let named: Vec<Named> = list
        .and_then(|l| serde_json::from_value(l.clone()).ok())
        .unwrap_or_default();
    let mut out = Vec::new();
    for keyword in named {
        push_unique(&mut out, keyword.name.as_deref());
    }
    out
}

#[derive(Debug, Default, Deserialize)]
struct TmdbAltTitle {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    iso_3166_1: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
}

fn parse_alternative_titles(value: Option<&Value>) -> Vec<AlternativeTitle> {
    let list = value.and_then(|v| v.get("titles").or_else(|| v.get("results")));
    let titles: Vec<TmdbAltTitle> = list
        .and_then(|l| serde_json::from_value(l.clone()).ok())
        .unwrap_or_default();
    let mut seen = HashSet::new();
    titles
        .into_iter()
        .filter_map(|t| {
            let title = t.title?.trim().to_string();
            if title.is_empty() || !seen.insert((title.clone(), t.iso_3166_1.clone())) {
                return None;
            }
            Some(AlternativeTitle {
                title,
                country: t.iso_3166_1,
                kind: blank_to_none(t.kind),
            })
        })
        .collect()
}

fn names(value: Option<&Value>) -> Vec<String> {
    let named: Vec<Named> = value
        .and_then(|l| serde_json::from_value(l.clone()).ok())
        .unwrap_or_default();
    let mut out = Vec::new();
    for item in named {
        push_unique(&mut out, item.name.as_deref());
    }
    out
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// TMDB's `external_ids`, keyed by site (`imdb`, `tvdb`, `wikidata`, `facebook`…).
fn parse_external_ids(value: Option<&Value>, tmdb_id: Option<String>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Some(id) = tmdb_id {
        out.insert("tmdb".to_string(), id);
    }
    if let Some(object) = value.and_then(Value::as_object) {
        for (key, value) in object {
            if key == "id" {
                continue;
            }
            let id = match value {
                Value::String(s) if !s.trim().is_empty() => s.trim().to_string(),
                Value::Number(n) => n.to_string(),
                _ => continue,
            };
            let site = key.strip_suffix("_id").unwrap_or(key).to_string();
            out.insert(site, id);
        }
    }
    out
}

fn release_type_name(kind: Option<i64>) -> Option<String> {
    Some(
        match kind? {
            1 => "premiere",
            2 => "limited",
            3 => "theatrical",
            4 => "digital",
            5 => "physical",
            6 => "tv",
            _ => return None,
        }
        .to_string(),
    )
}

/// Movie certifications: per country, the first non-empty one, preferring the theatrical
/// release.
fn parse_release_certifications(value: Option<&Value>) -> Vec<ContentRating> {
    let mut out = Vec::new();
    let Some(results) = value
        .and_then(|v| v.get("results"))
        .and_then(Value::as_array)
    else {
        return out;
    };
    for country in results {
        let Some(code) = country.get("iso_3166_1").and_then(Value::as_str) else {
            continue;
        };
        let mut dates: Vec<&Value> = country
            .get("release_dates")
            .and_then(Value::as_array)
            .map(|d| d.iter().collect())
            .unwrap_or_default();
        dates.sort_by_key(|d| {
            let kind = d.get("type").and_then(Value::as_i64).unwrap_or(9);
            if kind == 3 { 0 } else { kind }
        });
        if let Some(date) = dates.into_iter().find(|d| {
            d.get("certification")
                .and_then(Value::as_str)
                .is_some_and(|c| !c.trim().is_empty())
        }) {
            out.push(ContentRating {
                country: code.to_string(),
                rating: date["certification"]
                    .as_str()
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                kind: release_type_name(date.get("type").and_then(Value::as_i64)),
            });
        }
    }
    out
}

/// Show content ratings, one per country.
fn parse_content_ratings(value: Option<&Value>) -> Vec<ContentRating> {
    value
        .and_then(|v| v.get("results"))
        .and_then(Value::as_array)
        .map(|results| {
            results
                .iter()
                .filter_map(|r| {
                    let country = r.get("iso_3166_1")?.as_str()?.to_string();
                    let rating = r.get("rating")?.as_str()?.trim().to_string();
                    (!rating.is_empty()).then_some(ContentRating {
                        country,
                        rating,
                        kind: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn credit_list(value: Option<&Value>) -> TmdbCreditList {
    value
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// `/movie/{id}?append_to_response=credits,release_dates,keywords,alternative_titles,videos,external_ids`
pub fn parse_tmdb_movie(doc: &Value) -> ProviderTitle {
    let tmdb_id = doc.get("id").and_then(Value::as_i64);
    let mut facts = TitleFacts {
        provider_id: tmdb_id.map(|id| format!("movie/{id}")),
        studios: names(doc.get("production_companies")),
        keywords: parse_keywords(doc.get("keywords")),
        origin_countries: strings(doc.get("origin_country")),
        content_ratings: parse_release_certifications(doc.get("release_dates")),
        alternative_titles: parse_alternative_titles(doc.get("alternative_titles")),
        videos: parse_videos(doc.get("videos")),
        external_ids: parse_external_ids(doc.get("external_ids"), tmdb_id.map(|i| i.to_string())),
        homepage: blank_to_none(
            doc.get("homepage")
                .and_then(Value::as_str)
                .map(str::to_string),
        ),
        ..Default::default()
    };
    if facts.origin_countries.is_empty() {
        facts.origin_countries = doc
            .get("production_countries")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(|c| c.get("iso_3166_1")?.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
    }
    if let Some(imdb) = doc
        .get("imdb_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        facts
            .external_ids
            .entry("imdb".into())
            .or_insert_with(|| imdb.to_string());
    }
    ProviderTitle {
        facts,
        credits: plain_credits(credit_list(doc.get("credits"))),
    }
}

/// `/tv/{id}?append_to_response=aggregate_credits,content_ratings,keywords,alternative_titles,videos,external_ids`
pub fn parse_tmdb_show(doc: &Value) -> ProviderTitle {
    let tmdb_id = doc.get("id").and_then(Value::as_i64);
    let facts = TitleFacts {
        provider_id: tmdb_id.map(|id| format!("tv/{id}")),
        studios: names(doc.get("production_companies")),
        networks: names(doc.get("networks")),
        keywords: parse_keywords(doc.get("keywords")),
        origin_countries: strings(doc.get("origin_country")),
        content_ratings: parse_content_ratings(doc.get("content_ratings")),
        alternative_titles: parse_alternative_titles(doc.get("alternative_titles")),
        videos: parse_videos(doc.get("videos")),
        external_ids: parse_external_ids(doc.get("external_ids"), tmdb_id.map(|i| i.to_string())),
        homepage: blank_to_none(
            doc.get("homepage")
                .and_then(Value::as_str)
                .map(str::to_string),
        ),
    };
    let mut credits = match doc.get("aggregate_credits") {
        Some(aggregate) => aggregate_credits(credit_list(Some(aggregate))),
        None => plain_credits(credit_list(doc.get("credits"))),
    };
    // Creators are crew of the show itself; `aggregate_credits` only counts episode work. They
    // lead the crew (orders below every other crew credit's).
    if let Some(creators) = doc.get("created_by").and_then(Value::as_array) {
        for (index, creator) in creators.iter().enumerate() {
            let Ok(member) = serde_json::from_value::<TmdbPersonCredit>(creator.clone()) else {
                continue;
            };
            if member.name.trim().is_empty() {
                continue;
            }
            let mut credit = CreditEntry::crew(
                member.person(),
                Some("Writing".into()),
                Some("Creator".into()),
            );
            credit.provider_credit_id = member.credit_id.clone();
            if credits.iter().any(|c| {
                c.person.tmdb_id == credit.person.tmdb_id && c.identity() == credit.identity()
            }) {
                continue;
            }
            credit.order = Some(index as i32 - 1000);
            credits.push(credit);
        }
    }
    ProviderTitle { facts, credits }
}

/// `/tv/{id}/season/{n}?append_to_response=aggregate_credits`: the season's credits, and the crew
/// and guest stars TMDB lists on each episode.
pub fn parse_tmdb_season(doc: &Value) -> SeasonCredits {
    let season_number = doc
        .get("season_number")
        .and_then(Value::as_i64)
        .unwrap_or_default() as i32;
    let credits = match doc.get("aggregate_credits") {
        Some(aggregate) => aggregate_credits(credit_list(Some(aggregate))),
        None => plain_credits(credit_list(doc.get("credits"))),
    };
    let mut episodes = Vec::new();
    for episode in doc
        .get("episodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(number) = episode.get("episode_number").and_then(Value::as_i64) else {
            continue;
        };
        let guests: Vec<TmdbPersonCredit> = episode
            .get("guest_stars")
            .and_then(|g| serde_json::from_value(g.clone()).ok())
            .unwrap_or_default();
        let crew: Vec<TmdbPersonCredit> = episode
            .get("crew")
            .and_then(|g| serde_json::from_value(g.clone()).ok())
            .unwrap_or_default();
        let mut entries = plain_credits(TmdbCreditList { cast: guests, crew });
        for entry in entries.iter_mut().filter(|e| e.kind == CreditKind::Cast) {
            entry.guest = true;
        }
        let tmdb_episode_id = episode
            .get("id")
            .and_then(Value::as_i64)
            .map(|id| id as i32);
        episodes.push(EpisodeCredits {
            episode_number: number as i32,
            tmdb_episode_id,
            credits: entries,
            facts: TitleFacts {
                provider_id: tmdb_episode_id.map(|id| format!("tv/episode/{id}")),
                ..Default::default()
            },
        });
    }
    SeasonCredits {
        season_number,
        credits,
        episodes,
    }
}

/// `/person/{id}?append_to_response=external_ids,images`
pub fn parse_tmdb_person(doc: &Value) -> Option<PersonDetails> {
    let id = doc.get("id")?.as_i64()? as i32;
    let name = doc.get("name")?.as_str()?.trim().to_string();
    let text = |key: &str| blank_to_none(doc.get(key).and_then(Value::as_str).map(str::to_string));
    let external = doc.get("external_ids");
    let external_text = |key: &str| {
        blank_to_none(
            external
                .and_then(|e| e.get(key))
                .and_then(Value::as_str)
                .map(str::to_string),
        )
    };
    let mut images: Vec<(i64, String)> = doc
        .get("images")
        .and_then(|i| i.get("profiles"))
        .and_then(Value::as_array)
        .map(|profiles| {
            profiles
                .iter()
                .filter_map(|p| {
                    let path = p.get("file_path")?.as_str()?;
                    let votes = (p.get("vote_average").and_then(Value::as_f64).unwrap_or(0.0)
                        * 1000.0) as i64;
                    Some((votes, tmdb_image(Some(path))?))
                })
                .collect()
        })
        .unwrap_or_default();
    images.sort_by_key(|(votes, _)| std::cmp::Reverse(*votes));
    let profile_url = tmdb_image(doc.get("profile_path").and_then(Value::as_str));
    let mut image_urls: Vec<String> = Vec::new();
    push_unique(&mut image_urls, profile_url.as_deref());
    for (_, url) in images {
        push_unique(&mut image_urls, Some(&url));
    }
    Some(PersonDetails {
        person: PersonRef {
            name,
            tmdb_id: Some(id),
            imdb_id: text("imdb_id").or_else(|| external_text("imdb_id")),
            musicbrainz_id: None,
            wikidata_id: external_text("wikidata_id"),
            profile_url,
            known_for_department: text("known_for_department"),
        },
        biography: text("biography"),
        birthday: text("birthday"),
        deathday: text("deathday"),
        place_of_birth: text("place_of_birth"),
        also_known_as: strings(doc.get("also_known_as")),
        image_urls,
    })
}

/// The first show TMDB's `/find/{id}` returned.
pub fn parse_tmdb_find_show(doc: &Value) -> Option<i32> {
    doc.get("tv_results")?
        .as_array()?
        .first()?
        .get("id")?
        .as_i64()
        .map(|id| id as i32)
}

// ---------------------------------------------------------------------------------------------
// MusicBrainz release relationships

/// Credits MusicBrainz lists on a release (`inc=artist-rels+recording-level-rels+work-rels+
/// work-level-rels+labels`): the release's own credits, each recording's, and labels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReleaseCredits {
    pub release_id: String,
    pub credits: Vec<CreditEntry>,
    /// Recording MBID → that recording's credits (its own, and its work's writers).
    pub recordings: BTreeMap<String, Vec<CreditEntry>>,
    pub labels: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct MbRelation {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(rename = "target-type", default)]
    target_type: Option<String>,
    #[serde(default)]
    attributes: Vec<String>,
    #[serde(rename = "target-credit", default)]
    target_credit: Option<String>,
    #[serde(default)]
    artist: Option<MbArtist>,
    #[serde(default)]
    work: Option<MbWork>,
}

#[derive(Debug, Default, Deserialize)]
struct MbArtist {
    id: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Default, Deserialize)]
struct MbWork {
    #[serde(default)]
    relations: Vec<MbRelation>,
}

fn title_case(text: &str) -> String {
    text.split([' ', '-'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A MusicBrainz artist relationship as (kind, department, job). Performances are `cast`.
fn mb_role(kind: &str, attributes: &[String]) -> (CreditKind, &'static str, String) {
    let attrs = || {
        attributes
            .iter()
            .map(|a| title_case(a))
            .collect::<Vec<_>>()
            .join(", ")
    };
    match kind {
        "composer" => (CreditKind::Crew, "Writing", "Composer".into()),
        "lyricist" => (CreditKind::Crew, "Writing", "Lyricist".into()),
        "writer" => (CreditKind::Crew, "Writing", "Writer".into()),
        "librettist" => (CreditKind::Crew, "Writing", "Librettist".into()),
        "arranger" | "instrument arranger" | "vocal arranger" | "orchestrator" => {
            (CreditKind::Crew, "Writing", title_case(kind))
        }
        "producer" if attributes.iter().any(|a| a == "executive") => {
            (CreditKind::Crew, "Production", "Executive Producer".into())
        }
        "producer" => (CreditKind::Crew, "Production", "Producer".into()),
        "remixer" | "programming" => (CreditKind::Crew, "Production", title_case(kind)),
        "engineer" | "audio" => (CreditKind::Crew, "Sound", "Engineer".into()),
        "mix" => (CreditKind::Crew, "Sound", "Mixing Engineer".into()),
        "mastering" => (CreditKind::Crew, "Sound", "Mastering Engineer".into()),
        "recording" => (CreditKind::Crew, "Sound", "Recording Engineer".into()),
        "sound" => (CreditKind::Crew, "Sound", "Sound Engineer".into()),
        "instrument" if !attributes.is_empty() => (CreditKind::Cast, "Performance", attrs()),
        "instrument" => (CreditKind::Cast, "Performance", "Instruments".into()),
        "vocal" if !attributes.is_empty() => (CreditKind::Cast, "Performance", attrs()),
        "vocal" => (CreditKind::Cast, "Performance", "Vocals".into()),
        "performer" | "performing orchestra" | "chorus master" | "concertmaster" => {
            (CreditKind::Cast, "Performance", title_case(kind))
        }
        "conductor" => (CreditKind::Cast, "Performance", "Conductor".into()),
        other => (CreditKind::Crew, "Other", title_case(other)),
    }
}

fn mb_credits(relations: &[MbRelation], into: &mut Vec<CreditEntry>) {
    for relation in relations {
        if relation.target_type.as_deref() != Some("artist") {
            continue;
        }
        let (Some(kind), Some(artist)) = (relation.kind.as_deref(), relation.artist.as_ref())
        else {
            continue;
        };
        if artist.name.trim().is_empty() {
            continue;
        }
        let (credit_kind, department, job) = mb_role(kind, &relation.attributes);
        let person = PersonRef {
            name: artist.name.trim().to_string(),
            musicbrainz_id: Some(artist.id.clone()),
            ..Default::default()
        };
        let mut credit = match credit_kind {
            CreditKind::Cast => {
                let mut credit = CreditEntry::cast(person, None, None);
                credit.department = Some(department.into());
                credit.job = Some(job);
                credit
            }
            CreditKind::Crew => CreditEntry::crew(person, Some(department.into()), Some(job)),
        };
        // "as Slash": the name the release credits them under.
        credit.character = blank_to_none(relation.target_credit.clone())
            .filter(|credited| credited != &artist.name);
        let order = into.len() as i32;
        credit.order = Some(order);
        if !into.iter().any(|c| {
            c.person.musicbrainz_id == credit.person.musicbrainz_id
                && c.identity() == credit.identity()
        }) {
            into.push(credit);
        }
    }
}

/// `/release/{id}?inc=recordings+artist-rels+recording-level-rels+work-rels+work-level-rels+labels`
pub fn parse_musicbrainz_release(doc: &Value) -> ReleaseCredits {
    let release_id = doc
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let relations: Vec<MbRelation> = doc
        .get("relations")
        .and_then(|r| serde_json::from_value(r.clone()).ok())
        .unwrap_or_default();
    let mut credits = Vec::new();
    mb_credits(&relations, &mut credits);
    let mut recordings = BTreeMap::new();
    for track in doc
        .get("media")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| m.get("tracks").and_then(Value::as_array))
        .flatten()
    {
        let Some(recording) = track.get("recording") else {
            continue;
        };
        let Some(recording_id) = recording.get("id").and_then(Value::as_str) else {
            continue;
        };
        let relations: Vec<MbRelation> = recording
            .get("relations")
            .and_then(|r| serde_json::from_value(r.clone()).ok())
            .unwrap_or_default();
        let mut entries = Vec::new();
        mb_credits(&relations, &mut entries);
        // The song's writers sit on its work, which the recording performs.
        for relation in relations
            .iter()
            .filter(|r| r.target_type.as_deref() == Some("work"))
        {
            if let Some(work) = &relation.work {
                mb_credits(&work.relations, &mut entries);
            }
        }
        if !entries.is_empty() {
            recordings.insert(recording_id.to_string(), entries);
        }
    }
    let mut labels = Vec::new();
    for info in doc
        .get("label-info")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        push_unique(
            &mut labels,
            info.get("label")
                .and_then(|l| l.get("name"))
                .and_then(Value::as_str),
        );
    }
    ReleaseCredits {
        release_id,
        credits,
        recordings,
        labels,
    }
}

// ---------------------------------------------------------------------------------------------
// Audiobooks

/// A book's authors and narrators as credits: the authors are its writers, the narrators its
/// cast. The names come from the book's own record, so they are people known by name only.
pub fn audiobook_credits(authors: &[String], narrators: &[String]) -> Vec<CreditEntry> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (index, author) in authors.iter().enumerate() {
        let name = author.trim();
        if name.is_empty() || !seen.insert(("author", name_key(name))) {
            continue;
        }
        let mut credit = CreditEntry::crew(
            PersonRef::named(name),
            Some("Writing".into()),
            Some("Author".into()),
        );
        credit.order = Some(index as i32);
        out.push(credit);
    }
    for (index, narrator) in narrators.iter().enumerate() {
        let name = narrator.trim();
        if name.is_empty() || !seen.insert(("narrator", name_key(name))) {
            continue;
        }
        let mut credit = CreditEntry::cast(PersonRef::named(name), None, Some(index as i32));
        credit.department = Some("Narration".into());
        credit.job = Some("Narrator".into());
        out.push(credit);
    }
    out
}

#[cfg(test)]
#[path = "credits_tests.rs"]
mod tests;
