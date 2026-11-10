//! The GitHub Releases API: reading the release list, choosing this platform's asset, and fetching
//! assets without ever leaving GitHub.
//!
//! - **One request per check.** `GET /repos/<repo>/releases?per_page=30` with `If-None-Match`, so an
//!   unchanged list is a `304` that costs nothing against the hourly limit. `403`/`429` with
//!   `x-ratelimit-remaining: 0` or `retry-after` set a time before which nothing is asked again.
//! - **Only the configured repository.** Asset links must be
//!   `https://github.com/<repo>/releases/download/…`; anything else in the list is ignored.
//! - **Redirects** are followed only to GitHub hosts (`github.com`, `api.github.com`, and the asset
//!   CDN `objects.githubusercontent.com` / `release-assets.githubusercontent.com`), over HTTPS, at
//!   most five times.
//! - **Sizes.** A download must match the size the API reported, which must itself be under a cap.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, TimeZone, Utc};
use reqwest::{Client, StatusCode, Url, redirect};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use super::version::parse_tag;
use crate::services::http_client::{OutboundHttpProfile, outbound_client_builder};

pub const DEFAULT_REPOSITORY: &str = "Dastari/librarian";
/// The checksum list the release workflow publishes beside the binaries.
pub const CHECKSUMS_ASSET: &str = "SHA256SUMS";
/// Release notes longer than this are cut (they are shown, never interpreted).
const NOTES_LIMIT: usize = 20_000;
/// The largest server archive Librarian will download. Today's are about 35 MB.
pub const ARCHIVE_SIZE_CAP: u64 = 400 * 1024 * 1024;
/// The checksum list is a few lines.
pub const CHECKSUMS_SIZE_CAP: u64 = 64 * 1024;
/// The release list response.
const LIST_SIZE_CAP: usize = 4 * 1024 * 1024;
const ALLOWED_HOSTS: &[&str] = &[
    "github.com",
    "api.github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

/// `LIBRARIAN_UPDATE_REPOSITORY` (`owner/name`), else the project's own repository.
pub fn repository() -> String {
    std::env::var("LIBRARIAN_UPDATE_REPOSITORY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| valid_repository(value))
        .unwrap_or_else(|| DEFAULT_REPOSITORY.to_string())
}

pub fn valid_repository(value: &str) -> bool {
    let mut parts = value.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let ok = |part: &str| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    ok(owner) && ok(name)
}

// ===========================================================================
// The release list
// ===========================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRelease {
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub assets: Vec<ApiAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiAsset {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    pub browser_download_url: String,
}

/// One release as Librarian keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub tag: String,
    /// The tag as a version (`0.6.1-dev.2`); for a TV app tag, the part after `tv-v`.
    pub version: String,
    pub name: Option<String>,
    pub published_at: Option<String>,
    /// The release page on GitHub.
    pub url: String,
    pub notes: Option<String>,
    /// GitHub's pre-release flag.
    pub prerelease: bool,
    pub assets: Vec<AssetInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    pub name: String,
    pub size: u64,
    pub url: String,
}

impl ReleaseInfo {
    pub fn semver(&self) -> Option<semver::Version> {
        parse_tag(&self.version)
    }

    pub fn asset(&self, name: &str) -> Option<&AssetInfo> {
        self.assets.iter().find(|asset| asset.name == name)
    }
}

/// The newest releases worth knowing about, whatever the channel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalogue {
    /// Newest plain, unflagged `v*` release.
    pub newest_stable: Option<ReleaseInfo>,
    /// Newest `v*` release of any kind.
    pub newest_any: Option<ReleaseInfo>,
    /// Newest Android TV app (`tv-v*`).
    pub newest_tv: Option<ReleaseInfo>,
}

impl Catalogue {
    pub fn newest(&self, channel: super::version::Channel) -> Option<&ReleaseInfo> {
        match channel {
            super::version::Channel::Stable => self.newest_stable.as_ref(),
            super::version::Channel::Dev => self.newest_any.as_ref(),
        }
    }
}

pub fn parse_releases(json: &[u8]) -> Result<Vec<ApiRelease>> {
    serde_json::from_slice(json).context("GitHub returned a release list Librarian can't read")
}

fn asset_prefix(repo: &str) -> String {
    format!("https://github.com/{repo}/releases/download/")
}

fn release_info(release: &ApiRelease, version: String, repo: &str) -> ReleaseInfo {
    let prefix = asset_prefix(repo);
    let page = release
        .html_url
        .clone()
        .filter(|url| url.starts_with(&format!("https://github.com/{repo}/releases/")))
        .unwrap_or_else(|| {
            format!(
                "https://github.com/{repo}/releases/tag/{}",
                release.tag_name
            )
        });
    ReleaseInfo {
        tag: release.tag_name.clone(),
        version,
        name: release.name.clone().filter(|name| !name.trim().is_empty()),
        published_at: release.published_at.clone(),
        url: page,
        notes: release
            .body
            .as_ref()
            .map(|body| truncate(body, NOTES_LIMIT)),
        prerelease: release.prerelease,
        assets: release
            .assets
            .iter()
            .filter(|asset| asset.browser_download_url.starts_with(&prefix))
            .map(|asset| AssetInfo {
                name: asset.name.clone(),
                size: asset.size,
                url: asset.browser_download_url.clone(),
            })
            .collect(),
    }
}

fn truncate(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// Sort the list into the newest stable, newest overall and newest TV release. Drafts and tags
/// that are not versions are skipped.
pub fn catalogue(releases: &[ApiRelease], repo: &str) -> Catalogue {
    use super::version::Channel;
    let mut out = Catalogue::default();
    let newer = |slot: &Option<ReleaseInfo>, version: &semver::Version| {
        slot.as_ref()
            .and_then(|current| parse_tag(&current.version))
            .is_none_or(|current| *version > current)
    };
    for release in releases.iter().filter(|release| !release.draft) {
        if let Some(tv) = release.tag_name.strip_prefix("tv-") {
            if let Some(version) = parse_tag(tv)
                && newer(&out.newest_tv, &version)
            {
                out.newest_tv = Some(release_info(release, version.to_string(), repo));
            }
            continue;
        }
        if !release.tag_name.starts_with('v') {
            continue;
        }
        let Some(version) = parse_tag(&release.tag_name) else {
            continue;
        };
        if newer(&out.newest_any, &version) {
            out.newest_any = Some(release_info(release, version.to_string(), repo));
        }
        if Channel::Stable.includes(&version, release.prerelease)
            && newer(&out.newest_stable, &version)
        {
            out.newest_stable = Some(release_info(release, version.to_string(), repo));
        }
    }
    out
}

// ===========================================================================
// This platform's asset
// ===========================================================================

/// The archive name's suffix for a platform with a published build.
pub fn platform_asset_pattern(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Some("-linux-x86_64.tar.gz"),
        ("windows", "x86_64") => Some("-windows-server-x86_64.zip"),
        _ => None,
    }
}

/// The name of the binary inside that archive.
pub fn binary_name(os: &str) -> &'static str {
    if os == "windows" {
        "librarian.exe"
    } else {
        "librarian"
    }
}

/// The archive for this platform: `librarian-<version><suffix>`, exactly one of them.
pub fn platform_asset<'a>(release: &'a ReleaseInfo, os: &str, arch: &str) -> Option<&'a AssetInfo> {
    let suffix = platform_asset_pattern(os, arch)?;
    let mut matches = release.assets.iter().filter(|asset| {
        asset.name.starts_with("librarian-")
            && !asset.name.starts_with("librarian-tv")
            && asset.name.ends_with(suffix)
    });
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

/// What a self-upgrade to this release would download, or why it can't.
pub fn installable<'a>(
    release: &'a ReleaseInfo,
    os: &str,
    arch: &str,
) -> std::result::Result<(&'a AssetInfo, &'a AssetInfo), String> {
    let archive = platform_asset(release, os, arch).ok_or_else(|| {
        format!(
            "{} has no build for {os}-{arch}. Download a build from the release page instead.",
            release.tag
        )
    })?;
    let sums = release.asset(CHECKSUMS_ASSET).ok_or_else(|| {
        format!(
            "{} was published without a SHA256SUMS file, so Librarian can't verify it and won't install it. Download it from the release page and replace the binary by hand.",
            release.tag
        )
    })?;
    if archive.size == 0 || archive.size > ARCHIVE_SIZE_CAP {
        return Err(format!(
            "{} is {} bytes, outside what Librarian will download (up to {} MB).",
            archive.name,
            archive.size,
            ARCHIVE_SIZE_CAP / 1024 / 1024
        ));
    }
    Ok((archive, sums))
}

// ===========================================================================
// Checksums
// ===========================================================================

/// `sha256sum` output: `<64 hex>  <name>` (or `<hex> *<name>` in binary mode), one per line.
/// Lines that are not of that shape are ignored; names keep only their last path part.
pub fn parse_checksums(text: &str) -> HashMap<String, String> {
    let mut sums = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((hash, name)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let name = name.trim_start().trim_start_matches('*').trim();
        let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
        if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) && !name.is_empty() {
            sums.insert(name.to_string(), hash.to_ascii_lowercase());
        }
    }
    sums
}

/// Whether `actual` (lower-case hex) is the checksum the list gives for `name`.
pub fn verify_checksum(sums_text: &str, name: &str, actual: &str) -> Result<()> {
    let sums = parse_checksums(sums_text);
    let Some(expected) = sums.get(name) else {
        bail!("SHA256SUMS has no entry for {name}; refusing to install it");
    };
    if !expected.eq_ignore_ascii_case(actual) {
        bail!(
            "the checksum of {name} is wrong (expected {expected}, downloaded {actual}); refusing to install it"
        );
    }
    Ok(())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

// ===========================================================================
// HTTP
// ===========================================================================

pub fn allowed_host(url: &Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| ALLOWED_HOSTS.contains(&host.to_ascii_lowercase().as_str()))
}

/// An asset link must be on the configured repository's release downloads.
pub fn valid_asset_url(url: &str, repo: &str) -> bool {
    url.starts_with(&asset_prefix(repo)) && Url::parse(url).is_ok_and(|url| allowed_host(&url))
}

fn client(timeout: Duration) -> Result<Client> {
    outbound_client_builder(OutboundHttpProfile::Metadata)
        .timeout(timeout)
        .redirect(redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                attempt.error("too many redirects")
            } else if allowed_host(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("redirected away from GitHub")
            }
        }))
        .build()
        .context("Failed to build the GitHub client")
}

/// How a release-list request went.
#[derive(Debug)]
pub enum FetchOutcome {
    /// The list is unchanged since `etag`.
    NotModified {
        wait_until: Option<DateTime<Utc>>,
    },
    Fresh {
        releases: Vec<ApiRelease>,
        etag: Option<String>,
        /// This was the last request GitHub allows before its reset.
        wait_until: Option<DateTime<Utc>>,
    },
    RateLimited {
        until: DateTime<Utc>,
    },
}

fn header<'a>(headers: &'a reqwest::header::HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

/// When GitHub said to come back: `retry-after` seconds, or the `x-ratelimit-reset` time once
/// `x-ratelimit-remaining` reaches 0. A `429` without either waits a minute. `None` when nothing
/// needs waiting for.
pub fn rate_limit_until(
    status: u16,
    remaining: Option<&str>,
    reset: Option<&str>,
    retry_after: Option<&str>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let limited_status = status == 403 || status == 429;
    if limited_status
        && let Some(seconds) = retry_after.and_then(|value| value.trim().parse::<i64>().ok())
    {
        return Some(now + chrono::Duration::seconds(seconds.clamp(1, 86_400)));
    }
    if remaining.map(str::trim) == Some("0")
        && let Some(reset) = reset
            .and_then(|value| value.trim().parse::<i64>().ok())
            .and_then(|epoch| Utc.timestamp_opt(epoch, 0).single())
    {
        return Some(reset.max(now + chrono::Duration::seconds(1)));
    }
    (status == 429).then(|| now + chrono::Duration::seconds(60))
}

/// Read the release list, conditionally on `etag`.
pub async fn fetch_releases(repo: &str, etag: Option<&str>) -> Result<FetchOutcome> {
    let url = format!("https://api.github.com/repos/{repo}/releases?per_page=30");
    let mut request = client(Duration::from_secs(30))?
        .get(&url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(etag) = etag {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    let response = request
        .send()
        .await
        .context("Could not reach the GitHub Releases API")?;
    let status = response.status();
    let headers = response.headers().clone();
    let limited = rate_limit_until(
        status.as_u16(),
        header(&headers, "x-ratelimit-remaining"),
        header(&headers, "x-ratelimit-reset"),
        header(&headers, "retry-after"),
        Utc::now(),
    );
    if status == StatusCode::NOT_MODIFIED {
        return Ok(FetchOutcome::NotModified {
            wait_until: limited,
        });
    }
    if (status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS)
        && let Some(until) = limited
    {
        return Ok(FetchOutcome::RateLimited { until });
    }
    if !status.is_success() {
        bail!("The GitHub Releases API answered {status} for {repo}");
    }
    let etag = header(&headers, "etag").map(str::to_string);
    let body = read_capped(response, LIST_SIZE_CAP as u64).await?;
    Ok(FetchOutcome::Fresh {
        releases: parse_releases(&body)?,
        etag,
        wait_until: limited,
    })
}

async fn read_capped(mut response: reqwest::Response, cap: u64) -> Result<Vec<u8>> {
    if response.content_length().is_some_and(|length| length > cap) {
        bail!("the response is larger than {cap} bytes");
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() as u64 + chunk.len() as u64 > cap {
            bail!("the response is larger than {cap} bytes");
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Fetch a small asset (the checksum list) into memory.
pub async fn download_small(url: &str, repo: &str, cap: u64) -> Result<Vec<u8>> {
    if !valid_asset_url(url, repo) {
        bail!("refusing to download from outside {repo}'s releases");
    }
    let response = client(Duration::from_secs(60))?
        .get(url)
        .send()
        .await
        .context("Could not download from GitHub")?;
    if !response.status().is_success() {
        bail!("GitHub answered {} for the download", response.status());
    }
    read_capped(response, cap).await
}

/// Stream an asset to `dest`, hashing as it goes. The asset must be exactly `expected_size`
/// bytes, which must not exceed `cap`. Returns the SHA-256 as lower-case hex.
pub async fn download_to_file(
    url: &str,
    repo: &str,
    expected_size: u64,
    cap: u64,
    dest: &Path,
) -> Result<String> {
    if !valid_asset_url(url, repo) {
        bail!("refusing to download from outside {repo}'s releases");
    }
    if expected_size == 0 || expected_size > cap {
        bail!("the asset's size ({expected_size} bytes) is outside the {cap}-byte limit");
    }
    let mut response = client(Duration::from_secs(30 * 60))?
        .get(url)
        .send()
        .await
        .context("Could not download the release from GitHub")?;
    if !response.status().is_success() {
        bail!("GitHub answered {} for the download", response.status());
    }
    if response
        .content_length()
        .is_some_and(|length| length != expected_size)
    {
        bail!("GitHub is sending a different size than the release lists");
    }
    let mut file = tokio::fs::File::create(dest)
        .await
        .with_context(|| format!("Could not create {}", dest.display()))?;
    let mut hasher = Sha256::new();
    let mut received: u64 = 0;
    while let Some(chunk) = response.chunk().await? {
        received += chunk.len() as u64;
        if received > expected_size {
            bail!("the download is larger than the release lists ({expected_size} bytes)");
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    file.sync_all().await?;
    if received != expected_size {
        bail!("the download stopped at {received} of {expected_size} bytes");
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::updates::version::Channel;

    const RECORDED: &str = include_str!("fixtures/releases-2026-11-09.json");
    const WITH_SUMS: &str = include_str!("fixtures/releases-with-checksums.json");

    fn recorded() -> Catalogue {
        catalogue(
            &parse_releases(RECORDED.as_bytes()).unwrap(),
            DEFAULT_REPOSITORY,
        )
    }

    fn with_sums() -> Catalogue {
        catalogue(
            &parse_releases(WITH_SUMS.as_bytes()).unwrap(),
            DEFAULT_REPOSITORY,
        )
    }

    #[test]
    fn the_recorded_list_sorts_into_channels() {
        let catalogue = recorded();
        assert_eq!(catalogue.newest_any.as_ref().unwrap().tag, "v0.6.0-dev.1");
        // v0.3.2 is a plain tag but flagged as a pre-release, so Stable's newest is v0.2.0.
        assert_eq!(catalogue.newest_stable.as_ref().unwrap().tag, "v0.2.0");
        assert_eq!(
            catalogue.newest(Channel::Dev).unwrap().version,
            "0.6.0-dev.1"
        );
        let tv = catalogue.newest_tv.unwrap();
        assert_eq!(tv.version, "1.0.0");
        assert_eq!(tv.assets[0].name, "librarian-tv-1.0.0.apk");
    }

    #[test]
    fn recorded_releases_have_no_checksums_and_say_so() {
        let catalogue = recorded();
        let release = catalogue.newest_any.as_ref().unwrap();
        let archive = platform_asset(release, "linux", "x86_64").unwrap();
        assert_eq!(archive.name, "librarian-0.6.0-linux-x86_64.tar.gz");
        assert_eq!(archive.size, 32_371_327);
        let reason = installable(release, "linux", "x86_64").unwrap_err();
        assert!(reason.contains("without a SHA256SUMS"), "{reason}");
    }

    #[test]
    fn drafts_and_non_version_tags_are_skipped() {
        let catalogue = with_sums();
        assert_eq!(catalogue.newest_any.as_ref().unwrap().tag, "v0.7.1-dev.1");
        assert_eq!(catalogue.newest_stable.as_ref().unwrap().tag, "v0.7.0");
        assert_eq!(catalogue.newest_tv.as_ref().unwrap().version, "1.1.0");
    }

    #[test]
    fn the_asset_is_chosen_per_platform() {
        let catalogue = with_sums();
        let release = catalogue.newest_stable.as_ref().unwrap();
        let (linux, sums) = installable(release, "linux", "x86_64").unwrap();
        assert_eq!(linux.name, "librarian-0.7.0-linux-x86_64.tar.gz");
        assert_eq!(sums.name, CHECKSUMS_ASSET);
        let (windows, _) = installable(release, "windows", "x86_64").unwrap();
        assert_eq!(windows.name, "librarian-0.7.0-windows-server-x86_64.zip");
        assert!(installable(release, "linux", "aarch64").is_err());
        assert!(installable(release, "macos", "x86_64").is_err());
        assert_eq!(binary_name("windows"), "librarian.exe");
        assert_eq!(binary_name("linux"), "librarian");
    }

    #[test]
    fn notes_are_kept_as_text_and_capped() {
        let catalogue = with_sums();
        let notes = catalogue.newest_stable.unwrap().notes.unwrap();
        // Kept verbatim: the page renders it as text, so the tag is harmless there.
        assert!(notes.contains("<script>"));
        let long = "é".repeat(NOTES_LIMIT);
        let cut = truncate(&long, NOTES_LIMIT);
        assert!(cut.len() <= NOTES_LIMIT + '…'.len_utf8());
    }

    #[test]
    fn assets_from_another_repository_are_ignored() {
        let json = r#"[{"tag_name":"v1.0.0","draft":false,"prerelease":false,
            "html_url":"https://evil.example/release",
            "assets":[
              {"name":"librarian-1.0.0-linux-x86_64.tar.gz","size":10,"browser_download_url":"https://github.com/someone/else/releases/download/v1.0.0/librarian-1.0.0-linux-x86_64.tar.gz"},
              {"name":"SHA256SUMS","size":10,"browser_download_url":"https://github.com/Dastari/librarian/releases/download/v1.0.0/SHA256SUMS"}
            ]}]"#;
        let catalogue = catalogue(
            &parse_releases(json.as_bytes()).unwrap(),
            DEFAULT_REPOSITORY,
        );
        let release = catalogue.newest_stable.unwrap();
        assert_eq!(release.assets.len(), 1);
        assert_eq!(
            release.url,
            "https://github.com/Dastari/librarian/releases/tag/v1.0.0"
        );
        assert!(installable(&release, "linux", "x86_64").is_err());
    }

    #[test]
    fn checksums_are_read_and_checked() {
        let good = sha256_hex(b"hello");
        let text = format!(
            "{good}  librarian-0.7.0-linux-x86_64.tar.gz\n{}  *librarian-0.7.0-windows-server-x86_64.zip\nnot a line\n{}  linux/librarian\n",
            "a".repeat(64),
            "B".repeat(64)
        );
        let sums = parse_checksums(&text);
        assert_eq!(sums.len(), 3);
        assert_eq!(sums["librarian"], "b".repeat(64));
        verify_checksum(&text, "librarian-0.7.0-linux-x86_64.tar.gz", &good).unwrap();
        let wrong = verify_checksum(
            &text,
            "librarian-0.7.0-linux-x86_64.tar.gz",
            &sha256_hex(b"x"),
        )
        .unwrap_err();
        assert!(wrong.to_string().contains("checksum"), "{wrong}");
        let missing = verify_checksum(&text, "librarian-9.tar.gz", &good).unwrap_err();
        assert!(missing.to_string().contains("no entry"), "{missing}");
        assert!(verify_checksum("", "x", &good).is_err());
    }

    #[test]
    fn only_github_hosts_are_followed() {
        let ok = |url: &str| allowed_host(&Url::parse(url).unwrap());
        assert!(ok(
            "https://github.com/Dastari/librarian/releases/download/v1/x"
        ));
        assert!(ok(
            "https://objects.githubusercontent.com/github-production-release-asset/1"
        ));
        assert!(ok(
            "https://release-assets.githubusercontent.com/github-production-release-asset/1"
        ));
        assert!(!ok("http://objects.githubusercontent.com/x"));
        assert!(!ok("https://evil.example/x"));
        assert!(!ok("https://github.com.evil.example/x"));
        assert!(valid_asset_url(
            "https://github.com/Dastari/librarian/releases/download/v1/SHA256SUMS",
            DEFAULT_REPOSITORY
        ));
        assert!(!valid_asset_url(
            "https://github.com/Dastari/other/releases/download/v1/SHA256SUMS",
            DEFAULT_REPOSITORY
        ));
    }

    #[test]
    fn repositories_must_be_owner_slash_name() {
        assert!(valid_repository("Dastari/librarian"));
        assert!(!valid_repository("Dastari"));
        assert!(!valid_repository("a/b/c"));
        assert!(!valid_repository("../x"));
        assert!(!valid_repository("a/b?x=1"));
    }

    #[test]
    fn rate_limits_set_a_time_to_wait_for() {
        let now = Utc.timestamp_opt(1_000_000, 0).unwrap();
        assert_eq!(
            rate_limit_until(403, Some("0"), Some("1000600"), None, now),
            Some(Utc.timestamp_opt(1_000_600, 0).unwrap())
        );
        assert_eq!(
            rate_limit_until(429, None, None, Some("120"), now),
            Some(now + chrono::Duration::seconds(120))
        );
        assert_eq!(
            rate_limit_until(429, None, None, None, now),
            Some(now + chrono::Duration::seconds(60))
        );
        // A 403 for another reason is an error, not a wait.
        assert_eq!(
            rate_limit_until(403, Some("12"), Some("1000600"), None, now),
            None
        );
        // The last request of the hour succeeded: the next one waits for the reset.
        assert_eq!(
            rate_limit_until(200, Some("0"), Some("1000600"), None, now),
            Some(Utc.timestamp_opt(1_000_600, 0).unwrap())
        );
        assert_eq!(
            rate_limit_until(200, Some("59"), Some("1000600"), None, now),
            None
        );
    }
}
