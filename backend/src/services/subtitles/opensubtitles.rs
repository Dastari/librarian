//! OpenSubtitles.com REST API (`api.opensubtitles.com/api/v1`).
//!
//! Every request carries the user's API key (`Api-Key`) and names Librarian in `User-Agent`, as
//! the API requires. Searching needs only the key. Downloading needs a signed-in user: `/login`
//! gives a token and the host to use for that account (VIP accounts get their own), and
//! `/download` answers with a temporary link and how many downloads are left today. Free accounts
//! get a small daily allowance; when it is used up `/download` answers 406 with when it renews,
//! which is shown to the person as it is.
//!
//! Search is by the OpenSubtitles hash of the file (64 KiB from each end and the size) together
//! with the IMDb or TMDB id (and season and episode for an episode), so a hash match ranks first
//! and an id match still finds subtitles for a different release. Requests are rate limited to
//! the API's published allowance, and a 429 is reported, never retried in a loop.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Mutex;

use crate::services::http_client::{
    METADATA_RESPONSE_LIMIT, OutboundHttpProfile, outbound_client, response_bytes_limited,
    response_json_limited,
};
use crate::services::library_storage::{IntoStoragePath, library_storage};
use crate::services::transcode::ffmpeg::MAX_SUBTITLE_BYTES;

use super::language;

pub const DEFAULT_BASE_URL: &str = "https://api.opensubtitles.com/api/v1";
const USER_AGENT: &str = concat!("Librarian v", env!("CARGO_PKG_VERSION"));
const HASH_CHUNK: u64 = 64 * 1024;

pub const API_KEY_SETTING: &str = "subtitles.opensubtitles.api_key";
pub const USERNAME_SETTING: &str = "subtitles.opensubtitles.username";
pub const PASSWORD_SETTING: &str = "subtitles.opensubtitles.password";

/// The OpenSubtitles hash: the size plus the sum of the 64-bit little-endian words of the first
/// and last 64 KiB, wrapping. Files smaller than 128 KiB have no hash.
pub fn movie_hash(size: u64, head: &[u8], tail: &[u8]) -> Option<String> {
    if size < HASH_CHUNK * 2 || head.len() < HASH_CHUNK as usize || tail.len() < HASH_CHUNK as usize
    {
        return None;
    }
    let mut hash = size;
    for chunk in [head, tail] {
        for word in chunk[..HASH_CHUNK as usize].as_chunks::<8>().0 {
            hash = hash.wrapping_add(u64::from_le_bytes(*word));
        }
    }
    Some(format!("{hash:016x}"))
}

/// The hash of a library file, read through the storage layer (128 KiB, not the whole file).
pub async fn file_hash(path: impl IntoStoragePath) -> Result<Option<String>> {
    let mut opened = library_storage().open_read(path).await?;
    let size = opened.meta.size;
    if size < HASH_CHUNK * 2 {
        return Ok(None);
    }
    let mut head = vec![0u8; HASH_CHUNK as usize];
    opened.reader.read_exact(&mut head).await?;
    opened
        .reader
        .seek(std::io::SeekFrom::Start(size - HASH_CHUNK))
        .await?;
    let mut tail = vec![0u8; HASH_CHUNK as usize];
    opened.reader.read_exact(&mut tail).await?;
    Ok(movie_hash(size, &head, &tail))
}

/// What is being looked for.
#[derive(Clone, Debug, Default)]
pub struct SearchQuery {
    pub moviehash: Option<String>,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<i32>,
    /// For an episode: the show's ids, with season and episode.
    pub parent_imdb_id: Option<String>,
    pub parent_tmdb_id: Option<i32>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    /// Free text when no id is known.
    pub query: Option<String>,
    /// Stored (ISO 639-2/B) codes.
    pub languages: Vec<String>,
}

impl SearchQuery {
    /// Query parameters, sorted and in lower case as the API asks (it redirects otherwise).
    pub fn params(&self) -> Vec<(&'static str, String)> {
        let digits = |id: &str| {
            id.trim()
                .trim_start_matches("tt")
                .trim_start_matches('0')
                .to_owned()
        };
        let mut params = Vec::new();
        if let Some(episode) = self.episode {
            params.push(("episode_number", episode.to_string()));
        }
        if let Some(id) = self
            .imdb_id
            .as_deref()
            .map(digits)
            .filter(|id| !id.is_empty())
        {
            params.push(("imdb_id", id));
        }
        let mut codes = self
            .languages
            .iter()
            .flat_map(|code| language::opensubtitles_codes(code))
            .map(|code| code.to_ascii_lowercase())
            .collect::<Vec<_>>();
        codes.sort();
        codes.dedup();
        if !codes.is_empty() {
            params.push(("languages", codes.join(",")));
        }
        if let Some(hash) = self.moviehash.as_deref() {
            params.push(("moviehash", hash.to_ascii_lowercase()));
        }
        if let Some(id) = self
            .parent_imdb_id
            .as_deref()
            .map(digits)
            .filter(|id| !id.is_empty())
        {
            params.push(("parent_imdb_id", id));
        }
        if let Some(id) = self.parent_tmdb_id {
            params.push(("parent_tmdb_id", id.to_string()));
        }
        if let Some(query) = self.query.as_deref().filter(|q| !q.trim().is_empty()) {
            params.push(("query", query.trim().to_lowercase()));
        }
        if let Some(season) = self.season {
            params.push(("season_number", season.to_string()));
        }
        if let Some(id) = self.tmdb_id {
            params.push(("tmdb_id", id.to_string()));
        }
        params.sort_by(|a, b| a.0.cmp(b.0));
        params
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct SearchResponse {
    #[serde(default)]
    pub data: Vec<SearchItem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SearchItem {
    pub id: String,
    pub attributes: SubtitleAttributes,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SubtitleAttributes {
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub download_count: i64,
    #[serde(default)]
    pub hearing_impaired: bool,
    #[serde(default)]
    pub foreign_parts_only: bool,
    #[serde(default)]
    pub ratings: f64,
    #[serde(default)]
    pub votes: i64,
    #[serde(default)]
    pub from_trusted: bool,
    #[serde(default)]
    pub ai_translated: bool,
    #[serde(default)]
    pub machine_translated: bool,
    #[serde(default)]
    pub moviehash_match: bool,
    #[serde(default)]
    pub release: Option<String>,
    #[serde(default)]
    pub fps: Option<f64>,
    #[serde(default)]
    pub upload_date: Option<String>,
    #[serde(default)]
    pub uploader: Option<Uploader>,
    #[serde(default)]
    pub files: Vec<SubtitleFile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Uploader {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub rank: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SubtitleFile {
    pub file_id: i64,
    #[serde(default)]
    pub file_name: Option<String>,
}

/// Whether hearing-impaired subtitles are wanted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HearingImpaired {
    /// Take either, by the other measures.
    #[default]
    Include,
    /// Rank them first.
    Prefer,
    /// Rank them last.
    Avoid,
}

impl HearingImpaired {
    pub fn parse(raw: Option<&str>) -> Self {
        match raw
            .map(|value| value.trim().trim_matches('"').to_ascii_lowercase())
            .as_deref()
        {
            Some("prefer" | "only") => Self::Prefer,
            Some("avoid" | "exclude") => Self::Avoid,
            _ => Self::Include,
        }
    }
}

/// One subtitle that can be downloaded, as offered to the person.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub file_id: i64,
    /// Stored (ISO 639-2/B) code.
    pub language: Option<String>,
    /// The provider's own code (`en`, `pt-BR`).
    pub provider_language: Option<String>,
    pub release: Option<String>,
    pub file_name: Option<String>,
    pub hearing_impaired: bool,
    pub forced: bool,
    pub hash_match: bool,
    pub downloads: i64,
    pub rating: f64,
    pub votes: i64,
    pub trusted: bool,
    pub machine_translated: bool,
    pub uploader: Option<String>,
    pub upload_date: Option<String>,
    pub fps: Option<f64>,
}

impl Candidate {
    fn from_item(item: SearchItem) -> Vec<Candidate> {
        let attributes = item.attributes;
        let language = attributes
            .language
            .as_deref()
            .and_then(language::normalize)
            .map(str::to_owned);
        attributes
            .files
            .iter()
            .map(|file| Candidate {
                file_id: file.file_id,
                language: language.clone(),
                provider_language: attributes.language.clone(),
                release: attributes.release.clone().filter(|r| !r.trim().is_empty()),
                file_name: file.file_name.clone(),
                hearing_impaired: attributes.hearing_impaired,
                forced: attributes.foreign_parts_only,
                hash_match: attributes.moviehash_match,
                downloads: attributes.download_count,
                rating: attributes.ratings,
                votes: attributes.votes,
                trusted: attributes.from_trusted,
                machine_translated: attributes.ai_translated || attributes.machine_translated,
                uploader: attributes.uploader.as_ref().and_then(|u| u.name.clone()),
                upload_date: attributes.upload_date.clone(),
                fps: attributes.fps,
            })
            .collect()
    }
}

/// All candidates of a search response, one per file (a multi-CD subtitle is several).
pub fn candidates(response: SearchResponse) -> Vec<Candidate> {
    response
        .data
        .into_iter()
        .flat_map(Candidate::from_item)
        .collect()
}

/// Best first: a hash match (made for this very file), then the preferred languages in order,
/// then full subtitles over forced-only ones, the hearing-impaired preference, people's work over
/// machine translation, trusted uploaders, and then how often it was downloaded and how it is
/// rated (a rating counts only with a few votes behind it).
pub fn rank(
    mut list: Vec<Candidate>,
    preferred: &[String],
    hearing_impaired: HearingImpaired,
) -> Vec<Candidate> {
    let language_rank = |candidate: &Candidate| {
        candidate
            .language
            .as_deref()
            .and_then(|code| preferred.iter().position(|wanted| wanted == code))
            .unwrap_or(preferred.len())
    };
    let hi_rank = |candidate: &Candidate| match hearing_impaired {
        HearingImpaired::Include => 0,
        HearingImpaired::Prefer => u8::from(!candidate.hearing_impaired),
        HearingImpaired::Avoid => u8::from(candidate.hearing_impaired),
    };
    let quality = |candidate: &Candidate| {
        let rating = if candidate.votes >= 3 {
            candidate.rating
        } else {
            0.0
        };
        (candidate.downloads.max(0) as f64 + 1.0).log10() + rating / 2.0
    };
    list.sort_by(|a, b| {
        (
            !a.hash_match,
            language_rank(a),
            a.forced,
            hi_rank(a),
            a.machine_translated,
            !a.trusted,
        )
            .cmp(&(
                !b.hash_match,
                language_rank(b),
                b.forced,
                hi_rank(b),
                b.machine_translated,
                !b.trusted,
            ))
            .then_with(|| {
                quality(b)
                    .partial_cmp(&quality(a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.file_id.cmp(&b.file_id))
    });
    list
}

/// Why a request failed, in words a person can act on.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProviderError {
    #[error("OpenSubtitles is not set up. Add an API key in Settings › Metadata › Subtitles.")]
    NotConfigured,
    #[error(
        "OpenSubtitles wants a signed-in account to download. Add its username and password in Settings › Metadata › Subtitles."
    )]
    NeedsAccount,
    #[error(
        "OpenSubtitles did not accept the API key. Check it in Settings › Metadata › Subtitles."
    )]
    BadApiKey,
    #[error("OpenSubtitles did not accept the username or password.")]
    BadLogin,
    #[error("OpenSubtitles download limit reached: {0}")]
    Quota(String),
    #[error("OpenSubtitles is asking Librarian to slow down. Try again in {0} seconds.")]
    RateLimited(u64),
    #[error("OpenSubtitles answered: {0}")]
    Other(String),
}

/// The answer to `/download`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DownloadLink {
    pub link: String,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub remaining: Option<i64>,
    #[serde(default)]
    pub requests: Option<i64>,
    #[serde(default)]
    pub reset_time_utc: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    errors: Vec<String>,
    #[serde(default)]
    reset_time: Option<String>,
}

/// Map an unsuccessful answer to what went wrong. A 406 from `/download` is the daily quota.
pub fn error_for(status: StatusCode, retry_after: Option<u64>, body: &[u8]) -> ProviderError {
    let parsed = serde_json::from_slice::<ErrorBody>(body).ok();
    let message = parsed
        .as_ref()
        .and_then(|body| {
            body.message
                .clone()
                .or_else(|| body.errors.first().cloned())
        })
        .map(|message| message.trim().to_owned())
        .filter(|message| !message.is_empty());
    match status {
        StatusCode::UNAUTHORIZED => ProviderError::BadLogin,
        StatusCode::FORBIDDEN => ProviderError::BadApiKey,
        StatusCode::NOT_ACCEPTABLE => ProviderError::Quota(
            message
                .or_else(|| {
                    parsed
                        .and_then(|body| body.reset_time)
                        .map(|reset| format!("it renews in {reset}."))
                })
                .unwrap_or_else(|| "today's downloads are used up.".to_owned()),
        ),
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited(retry_after.unwrap_or(10)),
        _ => ProviderError::Other(message.unwrap_or_else(|| status.to_string())),
    }
}

/// An account's sign-in: the token and the host the account must use.
#[derive(Clone, Debug)]
struct Session {
    token: String,
    base_url: String,
    username: String,
    made: Instant,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
    #[serde(default)]
    base_url: Option<String>,
}

/// The credentials a client uses. The password is only ever held in memory.
#[derive(Clone, Default)]
pub struct Credentials {
    pub api_key: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("api_key", &"[REDACTED]")
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

/// A client shared by every request, so the rate limit and the sign-in are shared too.
pub struct OpenSubtitlesClient {
    base_url: String,
    http: reqwest::Client,
    limiter: DefaultDirectRateLimiter,
    session: Mutex<Option<Session>>,
}

/// Tokens last a day; sign in again well before.
const SESSION_LIFETIME: Duration = Duration::from_secs(12 * 3600);

impl OpenSubtitlesClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let quota =
            Quota::per_second(std::num::NonZeroU32::new(4).unwrap_or(std::num::NonZeroU32::MIN))
                .allow_burst(std::num::NonZeroU32::new(4).unwrap_or(std::num::NonZeroU32::MIN));
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            http: outbound_client(OutboundHttpProfile::Metadata)?,
            limiter: RateLimiter::direct(quota),
            session: Mutex::new(None),
        })
    }

    /// The process-wide client for the real API.
    pub fn shared() -> Result<Arc<Self>> {
        static CLIENT: std::sync::OnceLock<Arc<OpenSubtitlesClient>> = std::sync::OnceLock::new();
        if let Some(client) = CLIENT.get() {
            return Ok(client.clone());
        }
        let client = Arc::new(Self::new(DEFAULT_BASE_URL)?);
        Ok(CLIENT.get_or_init(|| client).clone())
    }

    fn request(
        &self,
        method: reqwest::Method,
        base: &str,
        path: &str,
        api_key: &str,
    ) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{base}{path}"))
            .header("Api-Key", api_key)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header(reqwest::header::ACCEPT, "application/json")
            .timeout(Duration::from_secs(20))
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, ProviderError> {
        self.limiter.until_ready().await;
        let response = request.send().await.map_err(|error| {
            ProviderError::Other(format!("could not be reached ({})", error.without_url()))
        })?;
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status();
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok());
        let body = response_bytes_limited(response, 64 * 1024)
            .await
            .unwrap_or_default();
        Err(error_for(status, retry_after, &body))
    }

    /// Search. Needs only the API key.
    pub async fn search(
        &self,
        credentials: &Credentials,
        query: &SearchQuery,
    ) -> Result<Vec<Candidate>, ProviderError> {
        if credentials.api_key.trim().is_empty() {
            return Err(ProviderError::NotConfigured);
        }
        let request = self
            .request(
                reqwest::Method::GET,
                &self.base_url,
                "/subtitles",
                credentials.api_key.trim(),
            )
            .query(&query.params());
        let response = self.send(request).await?;
        let body: SearchResponse = response_json_limited(response, METADATA_RESPONSE_LIMIT)
            .await
            .map_err(|_| {
                ProviderError::Other("search results that could not be read".to_owned())
            })?;
        Ok(candidates(body))
    }

    async fn session(&self, credentials: &Credentials) -> Result<Session, ProviderError> {
        let (Some(username), Some(password)) = (
            credentials
                .username
                .as_deref()
                .filter(|u| !u.trim().is_empty()),
            credentials.password.as_deref().filter(|p| !p.is_empty()),
        ) else {
            return Err(ProviderError::NeedsAccount);
        };
        let mut guard = self.session.lock().await;
        if let Some(session) = guard.as_ref()
            && session.username == username.trim()
            && session.made.elapsed() < SESSION_LIFETIME
        {
            return Ok(session.clone());
        }
        let request = self
            .request(
                reqwest::Method::POST,
                &self.base_url,
                "/login",
                credentials.api_key.trim(),
            )
            .json(&serde_json::json!({ "username": username.trim(), "password": password }));
        let response = self.send(request).await?;
        let login: LoginResponse =
            response_json_limited(response, 64 * 1024)
                .await
                .map_err(|_| {
                    ProviderError::Other("a sign-in answer that could not be read".to_owned())
                })?;
        // A VIP account is sent to its own host; a test server answers with nothing.
        let base_url = match login
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|host| !host.is_empty())
        {
            Some(host) if !self.base_url.starts_with("http://") => {
                format!(
                    "https://{}/api/v1",
                    host.trim_start_matches("https://").trim_end_matches('/')
                )
            }
            _ => self.base_url.clone(),
        };
        let session = Session {
            token: login.token,
            base_url,
            username: username.trim().to_owned(),
            made: Instant::now(),
        };
        *guard = Some(session.clone());
        Ok(session)
    }

    /// Forget the sign-in, after the account changed.
    pub async fn forget_session(&self) {
        *self.session.lock().await = None;
    }

    /// Download one subtitle file as SubRip: signed in when an account is saved (its own,
    /// larger allowance), else with the API key alone. Returns the bytes and what the API said
    /// about the allowance.
    pub async fn download(
        &self,
        credentials: &Credentials,
        file_id: i64,
    ) -> Result<(Vec<u8>, DownloadLink), ProviderError> {
        if credentials.api_key.trim().is_empty() {
            return Err(ProviderError::NotConfigured);
        }
        let has_account = credentials
            .username
            .as_deref()
            .is_some_and(|u| !u.trim().is_empty())
            && credentials
                .password
                .as_deref()
                .is_some_and(|p| !p.is_empty());
        let body = serde_json::json!({ "file_id": file_id, "sub_format": "srt" });
        let response = if has_account {
            let session = self.session(credentials).await?;
            let request = self
                .request(
                    reqwest::Method::POST,
                    &session.base_url,
                    "/download",
                    credentials.api_key.trim(),
                )
                .bearer_auth(&session.token)
                .json(&body);
            match self.send(request).await {
                Err(ProviderError::BadLogin) => {
                    // An expired token: sign in once more.
                    self.forget_session().await;
                    let session = self.session(credentials).await?;
                    let request = self
                        .request(
                            reqwest::Method::POST,
                            &session.base_url,
                            "/download",
                            credentials.api_key.trim(),
                        )
                        .bearer_auth(&session.token)
                        .json(&body);
                    self.send(request).await?
                }
                other => other?,
            }
        } else {
            let request = self
                .request(
                    reqwest::Method::POST,
                    &self.base_url,
                    "/download",
                    credentials.api_key.trim(),
                )
                .json(&body);
            match self.send(request).await {
                // The API wants a signed-in user for this key.
                Err(ProviderError::BadLogin) => return Err(ProviderError::NeedsAccount),
                other => other?,
            }
        };
        let link: DownloadLink =
            response_json_limited(response, 64 * 1024)
                .await
                .map_err(|_| {
                    ProviderError::Other("a download answer that could not be read".to_owned())
                })?;
        if link.link.is_empty() {
            return Err(ProviderError::Other(
                link.message
                    .clone()
                    .unwrap_or_else(|| "no download link".to_owned()),
            ));
        }
        self.limiter.until_ready().await;
        let file = self
            .http
            .get(&link.link)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|error| {
                ProviderError::Other(format!(
                    "the file could not be fetched ({})",
                    error.without_url()
                ))
            })?;
        if !file.status().is_success() {
            return Err(ProviderError::Other(format!(
                "the file could not be fetched ({})",
                file.status()
            )));
        }
        let bytes = response_bytes_limited(file, MAX_SUBTITLE_BYTES as usize)
            .await
            .map_err(|_| ProviderError::Other("the file was too large".to_owned()))?;
        Ok((bytes.to_vec(), link))
    }
}

/// Context-free helper for the settings test: sign in with these credentials, or just check the
/// key with a tiny search when there is no account.
pub async fn check(
    client: &OpenSubtitlesClient,
    credentials: &Credentials,
) -> Result<String, ProviderError> {
    if credentials
        .username
        .as_deref()
        .is_some_and(|u| !u.trim().is_empty())
    {
        client.forget_session().await;
        client.session(credentials).await?;
        return Ok("Signed in to OpenSubtitles.".to_owned());
    }
    let query = SearchQuery {
        imdb_id: Some("tt0133093".into()),
        languages: vec!["eng".into()],
        ..Default::default()
    };
    client.search(credentials, &query).await?;
    Ok("The API key works. Add your account to download subtitles.".to_owned())
}

#[allow(dead_code)]
fn _assert_send() {
    fn is_send<T: Send + Sync>() {}
    is_send::<OpenSubtitlesClient>();
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_FIXTURE: &str = include_str!("../../../tests/fixtures/opensubtitles/search.json");

    #[test]
    fn hash_of_a_known_buffer() {
        // 128 KiB of zeroes plus 8 bytes: the hash is the size plus the one non-zero word.
        let size = 2 * HASH_CHUNK + 8;
        let head = vec![0u8; HASH_CHUNK as usize];
        let mut tail = vec![0u8; HASH_CHUNK as usize];
        tail[HASH_CHUNK as usize - 8..].copy_from_slice(&1u64.to_le_bytes());
        assert_eq!(
            movie_hash(size, &head, &tail).as_deref(),
            Some("0000000000020009")
        );
        assert_eq!(movie_hash(1000, &head, &tail), None);
        // Wrapping, as the reference implementation does.
        let ones = vec![0xFFu8; HASH_CHUNK as usize];
        assert_eq!(
            movie_hash(size, &ones, &head).as_deref(),
            Some("000000000001e008")
        );
    }

    #[test]
    fn params_are_sorted_lower_case_and_unprefixed() {
        let query = SearchQuery {
            moviehash: Some("8E245D9679D31E12".into()),
            parent_imdb_id: Some("tt0903747".into()),
            season: Some(1),
            episode: Some(3),
            languages: vec!["eng".into(), "por".into(), "eng".into()],
            ..Default::default()
        };
        assert_eq!(
            query.params(),
            vec![
                ("episode_number", "3".to_owned()),
                ("languages", "en,pt-br,pt-pt".to_owned()),
                ("moviehash", "8e245d9679d31e12".to_owned()),
                ("parent_imdb_id", "903747".to_owned()),
                ("season_number", "1".to_owned()),
            ]
        );
    }

    #[test]
    fn recorded_search_ranks_hash_language_and_hearing_impaired() {
        let response: SearchResponse = serde_json::from_str(SEARCH_FIXTURE).unwrap();
        let list = candidates(response);
        assert_eq!(list.len(), 5);
        let preferred = vec!["eng".to_owned(), "fre".to_owned()];
        let ranked = rank(list.clone(), &preferred, HearingImpaired::Include);
        let ids = ranked.iter().map(|c| c.file_id).collect::<Vec<_>>();
        // The hash match first, then English (the trusted uploader before the hearing-impaired
        // one), then French, and the language nobody asked for last.
        assert_eq!(ids, vec![1001, 1002, 1003, 1005, 1004]);
        let prefer_hi = rank(list.clone(), &preferred, HearingImpaired::Prefer);
        assert_eq!(prefer_hi[0].file_id, 1001);
        assert_eq!(prefer_hi[1].file_id, 1003, "the SDH English one moves up");
        let first = &ranked[0];
        assert!(first.hash_match);
        assert_eq!(first.language.as_deref(), Some("eng"));
        assert_eq!(
            first.release.as_deref(),
            Some("Film.2020.1080p.BluRay.x264-GROUP")
        );
    }

    #[test]
    fn errors_read_as_actions() {
        let quota = br#"{"requests":21,"remaining":0,"message":"You have downloaded your allowed 20 subtitles for 24h.Your quota will be renewed in 07 hours and 33 minutes (2022-04-08 13:03:16 UTC) ","reset_time":"07 hours and 33 minutes","reset_time_utc":"2022-04-08T13:03:16.000Z"}"#;
        match error_for(StatusCode::NOT_ACCEPTABLE, None, quota) {
            ProviderError::Quota(message) => {
                assert!(message.contains("renewed in 07 hours"), "{message}")
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            error_for(StatusCode::TOO_MANY_REQUESTS, Some(4), b""),
            ProviderError::RateLimited(4)
        );
        assert_eq!(
            error_for(
                StatusCode::UNAUTHORIZED,
                None,
                br#"{"message":"Invalid username/password","status":401}"#
            ),
            ProviderError::BadLogin
        );
        assert_eq!(
            error_for(StatusCode::FORBIDDEN, None, b"{}"),
            ProviderError::BadApiKey
        );
        assert_eq!(
            ProviderError::Quota("it renews in 1 hour.".into()).to_string(),
            "OpenSubtitles download limit reached: it renews in 1 hour."
        );
    }

    #[tokio::test]
    async fn client_signs_in_downloads_and_reports_quota() {
        use axum::{
            Json, Router,
            http::HeaderMap,
            routing::{get, post},
        };
        use std::sync::atomic::{AtomicUsize, Ordering};
        let downloads = Arc::new(AtomicUsize::new(0));
        let counter = downloads.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let base = format!("http://{address}");
        let file_url = format!("{base}/file/1001.srt");
        let app = Router::new()
            .route(
                "/subtitles",
                get(|headers: HeaderMap| async move {
                    if headers.get("Api-Key").and_then(|v| v.to_str().ok()) != Some("key") {
                        return Err(StatusCode::FORBIDDEN);
                    }
                    assert!(headers.get("user-agent").unwrap().to_str().unwrap().starts_with("Librarian v"));
                    Ok(Json(serde_json::from_str::<serde_json::Value>(SEARCH_FIXTURE).unwrap()))
                }),
            )
            .route(
                "/login",
                post(|Json(body): Json<serde_json::Value>| async move {
                    if body["password"] != "secret" {
                        return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"message": "Invalid username/password", "status": 401}))));
                    }
                    Ok(Json(serde_json::json!({"user": {"allowed_downloads": 20}, "base_url": "", "token": "tok", "status": 200})))
                }),
            )
            .route(
                "/download",
                post(move |headers: HeaderMap| {
                    let counter = counter.clone();
                    let file_url = file_url.clone();
                    async move {
                        if headers.get("authorization").is_none() {
                            return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"message": "You must be logged in", "status": 401}))));
                        }
                        assert_eq!(headers.get("authorization").unwrap(), "Bearer tok");
                        if counter.fetch_add(1, Ordering::SeqCst) >= 1 {
                            return Err((StatusCode::NOT_ACCEPTABLE, Json(serde_json::json!({"requests": 21, "remaining": 0, "message": "You have downloaded your allowed 20 subtitles for 24h.", "reset_time": "07 hours and 33 minutes"}))));
                        }
                        Ok(Json(serde_json::json!({"link": file_url, "file_name": "x.srt", "requests": 1, "remaining": 19, "reset_time_utc": "2026-11-10T00:00:00.000Z"})))
                    }
                }),
            )
            .route("/file/1001.srt", get(|| async { "1\n00:00:01,000 --> 00:00:02,000\nHi\n" }));
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let client = OpenSubtitlesClient::new(&base).unwrap();
        let none = Credentials::default();
        assert_eq!(
            client
                .search(&none, &SearchQuery::default())
                .await
                .unwrap_err(),
            ProviderError::NotConfigured
        );
        let wrong_key = Credentials {
            api_key: "nope".into(),
            ..Default::default()
        };
        assert_eq!(
            client
                .search(&wrong_key, &SearchQuery::default())
                .await
                .unwrap_err(),
            ProviderError::BadApiKey
        );
        let key_only = Credentials {
            api_key: "key".into(),
            ..Default::default()
        };
        assert_eq!(
            client
                .search(&key_only, &SearchQuery::default())
                .await
                .unwrap()
                .len(),
            5
        );
        assert_eq!(
            client.download(&key_only, 1001).await.unwrap_err(),
            ProviderError::NeedsAccount
        );
        let wrong_password = Credentials {
            api_key: "key".into(),
            username: Some("me".into()),
            password: Some("bad".into()),
        };
        assert_eq!(
            client.download(&wrong_password, 1001).await.unwrap_err(),
            ProviderError::BadLogin
        );
        let account = Credentials {
            api_key: "key".into(),
            username: Some("me".into()),
            password: Some("secret".into()),
        };
        let (bytes, link) = client.download(&account, 1001).await.unwrap();
        assert!(String::from_utf8(bytes).unwrap().contains("Hi"));
        assert_eq!(link.remaining, Some(19));
        match client.download(&account, 1001).await.unwrap_err() {
            ProviderError::Quota(message) => assert!(message.contains("allowed 20")),
            other => panic!("{other:?}"),
        }
        assert!(
            format!("{account:?}").contains("[REDACTED]")
                && !format!("{account:?}").contains("secret")
        );
    }

    /// The only test that talks to OpenSubtitles: skipped unless `LIBRARIAN_OPENSUBTITLES_KEY_FILE`
    /// names a file holding a key. One search; one download only when
    /// `LIBRARIAN_OPENSUBTITLES_LIVE_DOWNLOAD=1` too, with the key alone, to learn whether this
    /// key can download without a signed-in user. Never prints the key.
    #[tokio::test]
    async fn live_opensubtitles_when_a_key_file_is_given() {
        let Ok(path) = std::env::var("LIBRARIAN_OPENSUBTITLES_KEY_FILE") else {
            return;
        };
        let api_key = std::fs::read_to_string(path)
            .expect("key file")
            .trim()
            .to_owned();
        let client = OpenSubtitlesClient::new(DEFAULT_BASE_URL).unwrap();
        let credentials = Credentials {
            api_key,
            ..Default::default()
        };
        let query = SearchQuery {
            imdb_id: Some("tt0133093".into()),
            languages: vec!["eng".into()],
            ..Default::default()
        };
        let found = client
            .search(&credentials, &query)
            .await
            .expect("live search");
        println!(
            "live search: {} candidates, first hash match {:?}",
            found.len(),
            found.first().map(|c| c.hash_match)
        );
        assert!(!found.is_empty());
        if std::env::var("LIBRARIAN_OPENSUBTITLES_LIVE_DOWNLOAD").as_deref() != Ok("1") {
            return;
        }
        let best = rank(found, &["eng".to_owned()], HearingImpaired::Include);
        match client.download(&credentials, best[0].file_id).await {
            Ok((bytes, link)) => println!(
                "live download with the key alone: {} bytes, requests {:?}, remaining {:?}, resets {:?}",
                bytes.len(),
                link.requests,
                link.remaining,
                link.reset_time_utc
            ),
            Err(error) => println!("live download with the key alone failed: {error}"),
        }
    }
}
