//! Finding and downloading subtitles for a library file (Q119), shared by the player's
//! "Find subtitles…" and the automatic fetch after an import.
//!
//! A downloaded subtitle is saved as a sidecar beside its video (`Film (2020).en.srt`), so it
//! plays from any player, moves with the video when it is organized, and survives a database
//! restore. Its row is `downloaded`, with the provider's file id, release name and uploader.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};
use chrono::Utc;
use graphql_orm::graphql::filters::StringFilter;
use serde_json::json;

use crate::db::Database;
use crate::services::ServicesManager;
use crate::services::graphql::LibrarianSchema;
use crate::services::graphql::entities::{
    Episode, MediaFile, Movie, Show, Subtitle, SubtitleWhereInput,
};
use crate::services::library_storage::{PlaceMode, PlaceOutcome, StoragePath, library_storage};
use crate::services::torrent::database::{execute_mutation, get_setting_string, system_auth_user};

use super::cues::{self, TextFormat};
use super::opensubtitles::{
    self, Candidate, Credentials, DownloadLink, HearingImpaired, OpenSubtitlesClient,
    ProviderError, SearchQuery,
};
use super::sidecar::{NameFacts, file_stem, numbered, sidecar_name};
use super::{is_bitmap_codec, language};

pub const PREFERRED_LANGUAGES_SETTING: &str = "subtitles.preferred_languages";
pub const HEARING_IMPAIRED_SETTING: &str = "subtitles.hearing_impaired";
pub const AUTO_DOWNLOAD_SETTING: &str = "subtitles.auto_download";

/// Settings stored with a secret value, encrypted by the `AppSetting` write hook.
pub const SECRET_SETTING_KEYS: &[&str] = &[
    opensubtitles::API_KEY_SETTING,
    opensubtitles::PASSWORD_SETTING,
];

/// The provider settings, with the password decrypted for this use only.
#[derive(Clone, Debug, Default)]
pub struct ProviderSettings {
    pub credentials: Credentials,
    /// Stored codes, most wanted first. English when none are set.
    pub preferred: Vec<String>,
    pub hearing_impaired: HearingImpaired,
    pub auto_download: bool,
}

async fn setting(db: &Database, key: &str) -> Option<String> {
    get_setting_string(db, key)
        .await
        .ok()
        .flatten()
        .map(|value| value.trim().trim_matches('"').to_owned())
        .filter(|value| !value.is_empty() && value != "null")
}

pub async fn load_settings(db: &Database, services: &Arc<ServicesManager>) -> ProviderSettings {
    let password = match setting(db, opensubtitles::PASSWORD_SETTING).await {
        Some(stored) => reveal(services, &stored).await,
        None => None,
    };
    let api_key = match setting(db, opensubtitles::API_KEY_SETTING).await {
        Some(stored) => reveal(services, &stored).await,
        None => None,
    };
    let preferred = setting(db, PREFERRED_LANGUAGES_SETTING)
        .await
        .map(|raw| language::parse_preference_list(&raw))
        .unwrap_or_default();
    ProviderSettings {
        credentials: Credentials {
            api_key: api_key.unwrap_or_default(),
            username: setting(db, opensubtitles::USERNAME_SETTING).await,
            password,
        },
        preferred: if preferred.is_empty() {
            vec!["eng".to_owned()]
        } else {
            preferred.into_iter().map(str::to_owned).collect()
        },
        hearing_impaired: HearingImpaired::parse(
            setting(db, HEARING_IMPAIRED_SETTING).await.as_deref(),
        ),
        auto_download: setting(db, AUTO_DOWNLOAD_SETTING).await.as_deref() == Some("true"),
    }
}

/// A stored secret setting as text. A value written before the hook existed is plain.
async fn reveal(services: &Arc<ServicesManager>, stored: &str) -> Option<String> {
    let sources = services.get_sources().await?;
    let manager = sources.get_manager().await?;
    match manager.encryption().reveal(stored) {
        Ok(value) => Some(value),
        Err(_) => {
            tracing::warn!("A saved OpenSubtitles secret could not be decrypted");
            None
        }
    }
}

/// What a search is for: the ids of the file's movie or episode, and its hash.
pub async fn search_query(
    db: &Database,
    file: &MediaFile,
    languages: Vec<String>,
) -> Result<SearchQuery> {
    let mut query = SearchQuery {
        languages,
        ..Default::default()
    };
    match opensubtitles::file_hash(&file.path).await {
        Ok(hash) => query.moviehash = hash,
        Err(error) => {
            tracing::debug!(media_file_id = %file.id, %error, "Could not hash a file for a subtitle search")
        }
    }
    if let Some(movie_id) = file.movie_id.as_deref()
        && let Some(movie) = Movie::get(db.pool(), &movie_id.to_owned()).await?
    {
        query.imdb_id = movie.imdb_id.clone();
        query.tmdb_id = movie.tmdb_id;
        if query.imdb_id.is_none() && query.tmdb_id.is_none() {
            query.query = Some(match movie.year {
                Some(year) => format!("{} {year}", movie.title),
                None => movie.title.clone(),
            });
        }
    } else if let Some(episode_id) = file.episode_id.as_deref()
        && let Some(episode) = Episode::get(db.pool(), &episode_id.to_owned()).await?
    {
        query.season = Some(episode.season);
        query.episode = Some(episode.episode);
        if let Some(show) = Show::get(db.pool(), &episode.show_id).await? {
            query.parent_imdb_id = show.imdb_id.clone();
            query.parent_tmdb_id = show.tmdb_id;
            if query.parent_imdb_id.is_none() && query.parent_tmdb_id.is_none() {
                query.query = Some(show.name.clone());
            }
        }
    } else {
        // An unmatched file: its name is all there is.
        let name = StoragePath::new(&file.path).file_name().unwrap_or_default();
        query.query = Some(file_stem(&name).replace(['.', '_'], " "));
    }
    Ok(query)
}

/// Search for a file, ranked by the site's preferences. `languages` narrows the search.
pub async fn search(
    db: &Database,
    settings: &ProviderSettings,
    file: &MediaFile,
    languages: Option<Vec<String>>,
) -> Result<Vec<Candidate>, ProviderError> {
    let languages = languages
        .map(|list| {
            list.iter()
                .filter_map(|code| language::normalize(code).map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .filter(|list| !list.is_empty())
        .unwrap_or_else(|| settings.preferred.clone());
    let query = search_query(db, file, languages.clone())
        .await
        .map_err(|error| ProviderError::Other(error.to_string()))?;
    let client =
        OpenSubtitlesClient::shared().map_err(|error| ProviderError::Other(error.to_string()))?;
    let found = client.search(&settings.credentials, &query).await?;
    Ok(opensubtitles::rank(
        found,
        &languages,
        settings.hearing_impaired,
    ))
}

/// What to download and how to describe it.
#[derive(Clone, Debug, Default)]
pub struct DownloadChoice {
    pub file_id: i64,
    /// Stored code.
    pub language: Option<String>,
    pub hearing_impaired: bool,
    pub forced: bool,
    pub release: Option<String>,
    pub uploader: Option<String>,
}

impl From<&Candidate> for DownloadChoice {
    fn from(candidate: &Candidate) -> Self {
        Self {
            file_id: candidate.file_id,
            language: candidate.language.clone(),
            hearing_impaired: candidate.hearing_impaired,
            forced: candidate.forced,
            release: candidate.release.clone(),
            uploader: candidate.uploader.clone(),
        }
    }
}

/// A free name beside the video for a new subtitle.
async fn free_name(video: &StoragePath, facts: &NameFacts) -> Result<StoragePath> {
    let folder = video.parent().context("The video has no folder")?;
    let name = video.file_name().context("The video has no name")?;
    let base = sidecar_name(file_stem(&name), facts, "srt", None);
    let mut candidate = base.clone();
    for n in 2..50 {
        let path = folder.join(&candidate);
        if !library_storage().exists(&path).await? {
            return Ok(path);
        }
        candidate = numbered(&base, n);
    }
    anyhow::bail!("There are too many subtitles with that name beside the video already")
}

/// The result of a download.
#[derive(Clone, Debug)]
pub struct Downloaded {
    pub subtitle: Subtitle,
    pub link: DownloadLink,
}

/// Download a subtitle and save it beside the file's video.
pub async fn download(
    db: &Database,
    schema: &LibrarianSchema,
    settings: &ProviderSettings,
    file: &MediaFile,
    choice: &DownloadChoice,
) -> Result<Downloaded> {
    let client = OpenSubtitlesClient::shared()?;
    let (bytes, link) = client
        .download(&settings.credentials, choice.file_id)
        .await
        .map_err(anyhow::Error::new)?;
    let text = cues::decode_text(&bytes);
    cues::parse(TextFormat::Srt, &text, None)
        .map_err(|_| anyhow::anyhow!("OpenSubtitles sent a file that holds no subtitles"))?;
    let facts = NameFacts {
        language: choice.language.as_deref().and_then(language::normalize),
        forced: choice.forced,
        hearing_impaired: choice.hearing_impaired,
        ..Default::default()
    };
    let video = StoragePath::new(&file.path);
    let target = free_name(&video, &facts).await?;
    // Saved as UTF-8 whatever the provider sent, so every player reads it.
    let body = text.into_bytes();
    let storage = library_storage().for_path(&target).await?;
    match storage
        .put_from_reader(
            Box::new(std::io::Cursor::new(body)),
            &target,
            PlaceMode::NoClobber,
        )
        .await?
    {
        PlaceOutcome::Placed => {}
        PlaceOutcome::TargetExists => {
            anyhow::bail!("A subtitle appeared at {} while downloading", target)
        }
    }
    let metadata = json!({
        "provider": "opensubtitles",
        "release": choice.release,
        "uploader": choice.uploader,
        "remainingDownloads": link.remaining,
    });
    execute_mutation(
        schema,
        &system_auth_user(),
        r#"mutation CreateDownloadedSubtitle($input: CreateSubtitleInput!) {
            CreateSubtitle: createSubtitle(input: $input) { success error }
        }"#,
        json!({ "input": {
            "mediaFileId": file.id,
            "sourceType": super::store::SOURCE_DOWNLOADED,
            "filePath": target.as_str(),
            "codec": "subrip",
            "language": facts.language,
            "title": choice.release,
            "isDefault": false,
            "isForced": facts.forced,
            "isHearingImpaired": facts.hearing_impaired,
            "opensubtitlesId": choice.file_id.to_string(),
            "downloadedAt": Utc::now().to_rfc3339(),
            "metadata": metadata.to_string(),
        }}),
    )
    .await?;
    let subtitle = Subtitle::query(db.pool())
        .filter(SubtitleWhereInput {
            media_file_id: Some(StringFilter {
                eq: Some(file.id.clone()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .find(|row| row.file_path.as_deref() == Some(target.as_str()))
        .context("The subtitle was saved but its record could not be read back")?;
    Ok(Downloaded { subtitle, link })
}

/// Why automatic fetching leaves a file alone (Q119), or `None` when it should look. Many videos
/// come with subtitles: any embedded stream (text or picture), sidecar or download in a preferred
/// language counts, as does an embedded stream whose language is not marked (`und`) and closed
/// captions inside the picture. Forced-only subtitles (just the foreign parts) do not.
pub fn auto_skip_reason(
    preferred: &[String],
    rows: &[Subtitle],
    closed_captions: bool,
) -> Option<String> {
    let full = rows.iter().filter(|row| !row.is_forced);
    for row in full.clone() {
        let Some(code) = row.language.as_deref().and_then(language::normalize) else {
            continue;
        };
        if preferred.iter().any(|wanted| wanted == code) {
            let name = language::display_name(code);
            return Some(match row.source_type.as_str() {
                "embedded" => format!("Has {name} subtitles built in"),
                super::store::SOURCE_DOWNLOADED => format!("Has downloaded {name} subtitles"),
                _ => {
                    let article = if name.starts_with(['A', 'E', 'I', 'O', 'U']) {
                        "an"
                    } else {
                        "a"
                    };
                    format!("Has {article} {name} subtitle file")
                }
            });
        }
    }
    if full.clone().any(|row| {
        row.source_type == "embedded"
            && row
                .language
                .as_deref()
                .is_none_or(|code| code.trim().is_empty() || code.eq_ignore_ascii_case("und"))
    }) {
        return Some("Has subtitles built in (their language is not marked)".to_owned());
    }
    if closed_captions {
        return Some("Has closed captions built in".to_owned());
    }
    None
}

fn attempted() -> &'static Mutex<HashSet<String>> {
    static ATTEMPTED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    ATTEMPTED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// After a video is analysed: when automatic downloads are on, fetch the best subtitle for each
/// preferred language it lacks. Once per file per run; quota and rate errors stop it quietly.
pub async fn auto_fetch(
    db: &Database,
    schema: &LibrarianSchema,
    services: &Arc<ServicesManager>,
    file: &MediaFile,
) {
    if file.movie_id.is_none() && file.episode_id.is_none() {
        return;
    }
    if !attempted()
        .lock()
        .map(|mut seen| seen.insert(file.id.clone()))
        .unwrap_or(false)
    {
        return;
    }
    let settings = load_settings(db, services).await;
    if !settings.auto_download || settings.credentials.api_key.is_empty() {
        return;
    }
    let rows = match Subtitle::query(db.pool())
        .filter(SubtitleWhereInput {
            media_file_id: Some(StringFilter {
                eq: Some(file.id.clone()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
    {
        Ok(rows) => rows,
        Err(_) => return,
    };
    if let Some(reason) = auto_skip_reason(
        &settings.preferred,
        &rows,
        file.closed_captions == Some(true),
    ) {
        tracing::debug!(media_file_id = %file.id, %reason, "Automatic subtitle search skipped");
        return;
    }
    let candidates = match search(db, &settings, file, None).await {
        Ok(found) => found,
        Err(error) => {
            tracing::info!(media_file_id = %file.id, %error, "Automatic subtitle search did not run");
            return;
        }
    };
    // One download per file: the best human-made, full subtitle in the most preferred language
    // that has one.
    let best = settings.preferred.iter().find_map(|code| {
        candidates.iter().find(|c| {
            c.language.as_deref() == Some(code.as_str()) && !c.forced && !c.machine_translated
        })
    });
    let Some(best) = best else {
        tracing::info!(media_file_id = %file.id, "Automatic subtitle search found nothing in the preferred languages");
        return;
    };
    match download(db, schema, &settings, file, &DownloadChoice::from(best)).await {
        Ok(done) => tracing::info!(
            media_file_id = %file.id,
            language = ?best.language,
            remaining = ?done.link.remaining,
            "Downloaded a subtitle automatically"
        ),
        Err(error) => {
            tracing::warn!(media_file_id = %file.id, %error, "Automatic subtitle download failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(language: &str, forced: bool, codec: &str) -> Subtitle {
        Subtitle {
            id: language.into(),
            media_file_id: "f".into(),
            source_type: "embedded".into(),
            stream_index: Some(2),
            file_path: None,
            codec: Some(codec.into()),
            codec_long_name: None,
            language: Some(language.into()),
            title: None,
            is_default: false,
            is_forced: forced,
            is_hearing_impaired: false,
            opensubtitles_id: None,
            downloaded_at: None,
            metadata: None,
            created_at: String::new(),
            updated_at: String::new(),
            media_file: None,
        }
    }

    fn file_row(source: &str, language: Option<&str>) -> Subtitle {
        let mut subtitle = row(language.unwrap_or(""), false, "subrip");
        subtitle.source_type = source.into();
        subtitle.language = language.map(str::to_owned);
        subtitle
    }

    #[test]
    fn automatic_fetch_skips_files_that_have_subtitles() {
        let preferred = vec!["eng".to_owned(), "fre".to_owned()];
        // Embedded English subtitles.
        assert_eq!(
            auto_skip_reason(&preferred, &[row("eng", false, "subrip")], false).as_deref(),
            Some("Has English subtitles built in")
        );
        // A picture subtitle counts too.
        assert_eq!(
            auto_skip_reason(&preferred, &[row("fre", false, "hdmv_pgs_subtitle")], false)
                .as_deref(),
            Some("Has French subtitles built in")
        );
        // Closed captions only.
        assert_eq!(
            auto_skip_reason(&preferred, &[], true).as_deref(),
            Some("Has closed captions built in")
        );
        // An embedded track whose language is not marked.
        assert_eq!(
            auto_skip_reason(&preferred, &[row("und", false, "subrip")], false).as_deref(),
            Some("Has subtitles built in (their language is not marked)")
        );
        assert!(auto_skip_reason(&preferred, &[file_row("embedded", None)], false).is_some());
        // A sidecar file or a download in a preferred language.
        assert_eq!(
            auto_skip_reason(&preferred, &[file_row("sidecar", Some("en"))], false).as_deref(),
            Some("Has an English subtitle file")
        );
        assert_eq!(
            auto_skip_reason(&preferred, &[file_row("downloaded", Some("fre"))], false).as_deref(),
            Some("Has downloaded French subtitles")
        );
        // Nothing: look.
        assert_eq!(auto_skip_reason(&preferred, &[], false), None);
        // Only another language, or only forced parts: still look.
        assert_eq!(
            auto_skip_reason(
                &preferred,
                &[row("ger", false, "subrip"), row("eng", true, "subrip")],
                false
            ),
            None
        );
    }
}
