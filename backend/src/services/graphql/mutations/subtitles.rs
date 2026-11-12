//! Subtitle and lyrics actions (Q119, Q121). The subtitles and lyrics themselves are read through
//! the generated `subtitles` and `lyrics` queries; only these actions and the server write them.
//!
//! - `searchSubtitles` asks OpenSubtitles for a file's subtitles, ranked by the site's language and
//!   hearing-impaired preferences. It is an action rather than a query because each call spends
//!   the provider's request allowance.
//! - `downloadSubtitle` downloads one into the library beside the video.
//! - `testOpenSubtitles` checks the key and account typed in Settings.
//! - `fetchLyrics` finds a track's lyrics (sidecar, tags, LRCLIB) and caches them.

use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::db::Database;
use crate::services::ServicesManager;
use crate::services::graphql::auth::AuthExt;
use crate::services::graphql::entities::{Lyric, Subtitle, Track};
use crate::services::subtitles::language;
use crate::services::subtitles::opensubtitles::{self, Credentials, OpenSubtitlesClient};
use crate::services::subtitles::provider::{self, DownloadChoice};

use super::authorized_media_file;

async fn schema(ctx: &Context<'_>) -> Result<crate::services::graphql::LibrarianSchema> {
    let services = ctx.data::<Arc<ServicesManager>>()?;
    services
        .get_graphql()
        .await
        .ok_or_else(|| async_graphql::Error::new("GraphQL service not available"))?
        .schema()
        .await
        .ok_or_else(|| async_graphql::Error::new("GraphQL schema not available"))
}

/// One subtitle OpenSubtitles offers for a file.
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "SubtitleCandidate")]
#[graphql(rename_fields = "camelCase")]
pub struct SubtitleCandidate {
    /// The provider's file id, passed back to `downloadSubtitle`.
    pub file_id: String,
    /// ISO 639-2/B, as subtitle rows store it.
    pub language: Option<String>,
    pub language_name: Option<String>,
    pub release: Option<String>,
    pub file_name: Option<String>,
    pub hearing_impaired: bool,
    /// Only the foreign-language parts.
    pub forced: bool,
    /// Made for this very file (its OpenSubtitles hash matched).
    pub hash_match: bool,
    pub downloads: i32,
    pub rating: f64,
    pub votes: i32,
    pub trusted: bool,
    pub machine_translated: bool,
    pub uploader: Option<String>,
    pub upload_date: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "SearchSubtitlesPayload")]
#[graphql(rename_fields = "camelCase")]
pub struct SearchSubtitlesPayload {
    pub success: bool,
    pub error: Option<String>,
    /// Best first.
    pub candidates: Vec<SubtitleCandidate>,
    /// The languages searched, as stored codes.
    pub languages: Vec<String>,
    /// Why the automatic fetch leaves this file alone ("Has English subtitles built in"), when
    /// it does. Searching by hand still works.
    pub auto_skip_reason: Option<String>,
}

#[derive(Debug, Clone, InputObject)]
#[graphql(name = "DownloadSubtitleInput")]
#[graphql(rename_fields = "camelCase")]
pub struct DownloadSubtitleInput {
    pub media_file_id: String,
    pub file_id: String,
    pub language: Option<String>,
    pub hearing_impaired: Option<bool>,
    pub forced: Option<bool>,
    pub release: Option<String>,
    pub uploader: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "DownloadSubtitlePayload")]
#[graphql(rename_fields = "camelCase")]
pub struct DownloadSubtitlePayload {
    pub success: bool,
    pub error: Option<String>,
    pub subtitle: Option<Subtitle>,
    /// Downloads left today on the account, as OpenSubtitles counts them.
    pub remaining_downloads: Option<i32>,
    /// When the allowance renews.
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, InputObject)]
#[graphql(name = "TestOpenSubtitlesInput")]
#[graphql(rename_fields = "camelCase")]
pub struct TestOpenSubtitlesInput {
    /// What was typed; the saved value when left out.
    pub api_key: Option<String>,
    pub username: Option<String>,
    /// What was typed; the saved password when left out or empty.
    pub password: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "TestOpenSubtitlesPayload")]
#[graphql(rename_fields = "camelCase")]
pub struct TestOpenSubtitlesPayload {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "FetchLyricsPayload")]
#[graphql(rename_fields = "camelCase")]
pub struct FetchLyricsPayload {
    pub success: bool,
    pub error: Option<String>,
    pub lyric: Option<Lyric>,
}

#[derive(Default)]
pub struct SubtitleActionMutations;

#[Object]
impl SubtitleActionMutations {
    /// Search OpenSubtitles for a file's subtitles, in `languages` or the preferred ones.
    #[graphql(name = "searchSubtitles")]
    async fn search_subtitles(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "mediaFileId")] media_file_id: String,
        languages: Option<Vec<String>>,
    ) -> Result<SearchSubtitlesPayload> {
        ctx.require_member()?;
        let file = authorized_media_file(ctx, &media_file_id).await?;
        let db = ctx.data::<Database>()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let settings = provider::load_settings(db, services).await;
        let searched = languages
            .clone()
            .map(|list| {
                list.iter()
                    .filter_map(|code| language::normalize(code).map(str::to_owned))
                    .collect::<Vec<_>>()
            })
            .filter(|list| !list.is_empty())
            .unwrap_or_else(|| settings.preferred.clone());
        let rows = crate::services::graphql::entities::Subtitle::query(db.pool())
            .filter(crate::services::graphql::entities::SubtitleWhereInput {
                media_file_id: Some(graphql_orm::graphql::filters::StringFilter {
                    eq: Some(file.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap_or_default();
        let auto_skip_reason = provider::auto_skip_reason(
            &settings.preferred,
            &rows,
            file.closed_captions == Some(true),
        );
        match provider::search(db, &settings, &file, Some(searched.clone())).await {
            Ok(found) => Ok(SearchSubtitlesPayload {
                success: true,
                error: None,
                candidates: found
                    .into_iter()
                    .take(60)
                    .map(|candidate| SubtitleCandidate {
                        file_id: candidate.file_id.to_string(),
                        language_name: candidate.language.as_deref().map(language::display_name),
                        language: candidate.language,
                        release: candidate.release,
                        file_name: candidate.file_name,
                        hearing_impaired: candidate.hearing_impaired,
                        forced: candidate.forced,
                        hash_match: candidate.hash_match,
                        downloads: i32::try_from(candidate.downloads).unwrap_or(i32::MAX),
                        rating: candidate.rating,
                        votes: i32::try_from(candidate.votes).unwrap_or(i32::MAX),
                        trusted: candidate.trusted,
                        machine_translated: candidate.machine_translated,
                        uploader: candidate.uploader,
                        upload_date: candidate.upload_date,
                    })
                    .collect(),
                languages: searched,
                auto_skip_reason,
            }),
            Err(error) => Ok(SearchSubtitlesPayload {
                success: false,
                error: Some(error.to_string()),
                candidates: Vec::new(),
                languages: searched,
                auto_skip_reason,
            }),
        }
    }

    /// Download a subtitle beside the file's video and list it with the file's subtitles.
    #[graphql(name = "downloadSubtitle")]
    async fn download_subtitle(
        &self,
        ctx: &Context<'_>,
        input: DownloadSubtitleInput,
    ) -> Result<DownloadSubtitlePayload> {
        ctx.require_member()?;
        let file = authorized_media_file(ctx, &input.media_file_id).await?;
        let db = ctx.data::<Database>()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let schema = schema(ctx).await?;
        let failed = |error: String| DownloadSubtitlePayload {
            success: false,
            error: Some(error),
            subtitle: None,
            remaining_downloads: None,
            resets_at: None,
        };
        let Ok(file_id) = input.file_id.trim().parse::<i64>() else {
            return Ok(failed("That is not an OpenSubtitles file id".to_owned()));
        };
        let settings = provider::load_settings(db, services).await;
        let choice = DownloadChoice {
            file_id,
            language: input
                .language
                .as_deref()
                .and_then(language::normalize)
                .map(str::to_owned),
            hearing_impaired: input.hearing_impaired.unwrap_or(false),
            forced: input.forced.unwrap_or(false),
            release: input.release.filter(|release| !release.trim().is_empty()),
            uploader: input.uploader,
        };
        match provider::download(db, &schema, &settings, &file, &choice).await {
            Ok(done) => Ok(DownloadSubtitlePayload {
                success: true,
                error: None,
                subtitle: Some(done.subtitle),
                remaining_downloads: done.link.remaining.and_then(|n| i32::try_from(n).ok()),
                resets_at: done.link.reset_time_utc,
            }),
            Err(error) => {
                tracing::warn!(media_file_id = %file.id, %error, "Subtitle download failed");
                Ok(failed(error.to_string()))
            }
        }
    }

    /// Check an OpenSubtitles key and account. Signs in when an account is given.
    #[graphql(name = "testOpenSubtitles")]
    async fn test_open_subtitles(
        &self,
        ctx: &Context<'_>,
        input: TestOpenSubtitlesInput,
    ) -> Result<TestOpenSubtitlesPayload> {
        ctx.require_admin()?;
        let db = ctx.data::<Database>()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let saved = provider::load_settings(db, services).await;
        let typed =
            |value: Option<String>| value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        let credentials = Credentials {
            api_key: typed(input.api_key).unwrap_or(saved.credentials.api_key),
            username: typed(input.username).or(saved.credentials.username),
            password: input
                .password
                .filter(|p| !p.is_empty())
                .or(saved.credentials.password),
        };
        let client = OpenSubtitlesClient::shared()?;
        Ok(match opensubtitles::check(&client, &credentials).await {
            Ok(message) => TestOpenSubtitlesPayload {
                success: true,
                message,
            },
            Err(error) => TestOpenSubtitlesPayload {
                success: false,
                message: error.to_string(),
            },
        })
    }

    /// A track's lyrics: what is cached, or a fresh look (`refresh`) at its `.lrc`, its tags and
    /// LRCLIB, cached for next time.
    #[graphql(name = "fetchLyrics")]
    async fn fetch_lyrics(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "mediaFileId")] media_file_id: String,
        refresh: Option<bool>,
    ) -> Result<FetchLyricsPayload> {
        ctx.require_member()?;
        let file = authorized_media_file(ctx, &media_file_id).await?;
        let db = ctx.data::<Database>()?;
        let Some(track_id) = file.track_id.clone() else {
            return Ok(FetchLyricsPayload {
                success: false,
                error: Some("This file is not a music track".to_owned()),
                lyric: None,
            });
        };
        let Some(track) = Track::get(db.pool(), &track_id).await? else {
            return Ok(FetchLyricsPayload {
                success: false,
                error: Some("This file's track is gone".to_owned()),
                lyric: None,
            });
        };
        let schema = schema(ctx).await?;
        match crate::services::lyrics::ensure(db, &schema, &track, &file, refresh.unwrap_or(false))
            .await
        {
            Ok(lyric) => Ok(FetchLyricsPayload {
                success: true,
                error: None,
                lyric: Some(lyric),
            }),
            Err(error) => Ok(FetchLyricsPayload {
                success: false,
                error: Some(error.to_string()),
                lyric: None,
            }),
        }
    }
}
