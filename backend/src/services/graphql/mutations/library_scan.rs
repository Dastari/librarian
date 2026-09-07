use async_graphql::{Context, Enum, InputObject, Object, SimpleObject};
use serde::Deserialize;

use crate::services::graphql::auth::AuthExt;
use crate::services::library_scan::{MatchMethod, MatchRequest, MatchWantedPolicy};
use crate::services::ollama::OllamaParsedHint;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "MatchMethod")]
pub enum MatchMethodGql {
    Filename,
    Metadata,
    Ollama,
}

impl From<MatchMethodGql> for MatchMethod {
    fn from(value: MatchMethodGql) -> Self {
        match value {
            MatchMethodGql::Filename => MatchMethod::Filename,
            MatchMethodGql::Metadata => MatchMethod::Metadata,
            MatchMethodGql::Ollama => MatchMethod::Ollama,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "MatchWantedPolicy")]
pub enum MatchWantedPolicyGql {
    PreferWanted,
    WantedOnly,
    All,
}

impl From<MatchWantedPolicyGql> for MatchWantedPolicy {
    fn from(value: MatchWantedPolicyGql) -> Self {
        match value {
            MatchWantedPolicyGql::PreferWanted => MatchWantedPolicy::PreferWanted,
            MatchWantedPolicyGql::WantedOnly => MatchWantedPolicy::WantedOnly,
            MatchWantedPolicyGql::All => MatchWantedPolicy::All,
        }
    }
}

#[derive(Debug, Clone, InputObject)]
#[graphql(name = "MatchMediaFileInput")]
#[graphql(rename_fields = "camelCase")]
pub struct MatchMediaFileInput {
    pub media_file_id: String,
    pub library_id: Option<String>,
    pub episode_id: Option<String>,
    pub movie_id: Option<String>,
    pub track_id: Option<String>,
    pub chapter_id: Option<String>,
    pub methods: Option<Vec<MatchMethodGql>>,
    pub force: Option<bool>,
    pub auto_match: Option<bool>,
    pub candidate_limit: Option<i32>,
    pub allow_provider_fallback: Option<bool>,
    pub wanted_policy: Option<MatchWantedPolicyGql>,
}

#[derive(Debug, Clone, InputObject)]
#[graphql(name = "OrganizeMediaFileInput")]
#[graphql(rename_fields = "camelCase")]
pub struct OrganizeMediaFileInput {
    pub media_file_id: String,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "ScanLibraryResult")]
#[graphql(rename_fields = "camelCase")]
pub struct ScanLibraryResult {
    pub success: bool,
    pub status: String,
    pub message: Option<String>,
    pub scan_run_id: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "AnalyzeMediaFileResult")]
#[graphql(rename_fields = "camelCase")]
pub struct AnalyzeMediaFileResult {
    pub success: bool,
    pub queued: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "ScanIssueActionResult")]
#[graphql(rename_fields = "camelCase")]
pub struct ScanIssueActionResult {
    pub success: bool,
    pub queued: bool,
    pub message: String,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "TrashDuplicateResult")]
#[graphql(rename_fields = "camelCase")]
pub struct TrashDuplicateResult {
    pub success: bool,
    pub message: String,
    pub old_path: Option<String>,
    pub trash_path: Option<String>,
}

#[derive(Debug, Clone, InputObject)]
#[graphql(name = "TestTmdbConnectionInput")]
#[graphql(rename_fields = "camelCase")]
pub struct TestTmdbConnectionInput {
    pub api_key: String,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "TestTmdbConnectionResult")]
#[graphql(rename_fields = "camelCase")]
pub struct TestTmdbConnectionResult {
    pub success: bool,
    pub message: String,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "MatchCandidate")]
#[graphql(rename_fields = "camelCase")]
pub struct MatchCandidateGql {
    pub target_type: String,
    pub target_id: String,
    pub target_name: Option<String>,
    pub score: f64,
    pub reason: Option<String>,
    pub wanted: Option<bool>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "MatchMediaFileResult")]
#[graphql(rename_fields = "camelCase")]
pub struct MatchMediaFileResult {
    pub success: bool,
    pub auto_matched: bool,
    pub already_matched: bool,
    pub matched_type: Option<String>,
    pub matched_id: Option<String>,
    pub confidence: f64,
    pub reason: Option<String>,
    pub candidates: Vec<MatchCandidateGql>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "UnmatchMediaFileResult")]
#[graphql(rename_fields = "camelCase")]
pub struct UnmatchMediaFileResult {
    pub success: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "OrganizeMediaFileResult")]
#[graphql(rename_fields = "camelCase")]
pub struct OrganizeMediaFileResult {
    pub success: bool,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, InputObject)]
#[graphql(name = "TestOllamaConnectionInput")]
#[graphql(rename_fields = "camelCase")]
pub struct TestOllamaConnectionInput {
    pub ollama_url: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "OllamaConnectionResult")]
#[graphql(rename_fields = "camelCase")]
pub struct OllamaConnectionResult {
    pub success: bool,
    pub models: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, InputObject)]
#[graphql(name = "TestLlmParserInput")]
#[graphql(rename_fields = "camelCase")]
pub struct TestLlmParserInput {
    pub filename: String,
    pub library_type: String,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "LlmParsedHintResult")]
#[graphql(rename_fields = "camelCase")]
pub struct LlmParsedHintResult {
    pub title: Option<String>,
    pub year: Option<i32>,
    pub show_title: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track: Option<String>,
    pub author: Option<String>,
    pub book: Option<String>,
    pub chapter: Option<String>,
    pub confidence: Option<f64>,
    pub match_source: Option<String>,
}

impl LlmParsedHintResult {
    fn from_hint(hint: OllamaParsedHint, library_type: &str) -> Self {
        let match_source = hint.to_match_source(library_type);
        Self {
            title: hint.title,
            year: hint.year,
            show_title: hint.show_name,
            season: hint.season,
            episode: hint.episode,
            artist: hint.artist_name,
            album: hint.album_name,
            track: hint.track_title,
            author: hint.author_name,
            book: hint.audiobook_title,
            chapter: hint.chapter_title,
            confidence: hint.confidence,
            match_source,
        }
    }
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "LlmParserTestResult")]
#[graphql(rename_fields = "camelCase")]
pub struct LlmParserTestResult {
    pub success: bool,
    pub regex_result: Option<String>,
    pub llm_result: Option<LlmParsedHintResult>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaModelTag>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelTag {
    name: String,
}

#[derive(Default)]
pub struct LibraryScanMutations;

#[Object]
impl LibraryScanMutations {
    #[graphql(name = "testTmdbConnection")]
    async fn test_tmdb_connection(
        &self,
        ctx: &Context<'_>,
        input: TestTmdbConnectionInput,
    ) -> async_graphql::Result<TestTmdbConnectionResult> {
        ctx.require_admin()?;
        let key = input.api_key.trim();
        if key.is_empty() {
            return Ok(TestTmdbConnectionResult {
                success: false,
                message: "Enter a TMDB API key before testing.".to_string(),
                correlation_id: None,
            });
        }
        let client = crate::services::metadata::tmdb::TmdbClient::new(key.to_string());
        match client.search_movies("The Matrix", Some(1999)).await {
            Ok(_) => Ok(TestTmdbConnectionResult {
                success: true,
                message: "TMDB accepted the key and responded successfully.".to_string(),
                correlation_id: None,
            }),
            Err(error) => {
                let correlation_id = uuid::Uuid::new_v4().to_string();
                tracing::warn!(
                    correlation_id,
                    error = %error,
                    "TMDB connection test failed"
                );
                Ok(TestTmdbConnectionResult {
                    success: false,
                    message:
                        "TMDB did not accept the key or could not be reached. Check the key and network."
                            .to_string(),
                    correlation_id: Some(correlation_id),
                })
            }
        }
    }

    #[graphql(name = "scanLibrary")]
    async fn scan_library(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: String,
    ) -> async_graphql::Result<ScanLibraryResult> {
        ctx.require_admin()?;
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;

        match scan_service.queue_scan(&id).await {
            Ok(outcome) => Ok(ScanLibraryResult {
                success: true,
                status: if outcome.queued {
                    "queued".to_string()
                } else {
                    "already_scanning".to_string()
                },
                message: Some(if outcome.queued {
                    "Library scan queued".to_string()
                } else {
                    "Library is already scanning".to_string()
                }),
                scan_run_id: outcome.scan_run_id,
            }),
            Err(error) => {
                tracing::error!(library_id = %id, error = %error, "Failed to queue library scan");
                Ok(ScanLibraryResult {
                    success: false,
                    status: "error".to_string(),
                    message: Some(
                        "The library scan could not be queued. Review server health and retry."
                            .to_string(),
                    ),
                    scan_run_id: None,
                })
            }
        }
    }

    #[graphql(name = "analyzeMediaFile")]
    async fn analyze_media_file(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "mediaFileId")] media_file_id: String,
        #[graphql(name = "path")] path: String,
    ) -> async_graphql::Result<AnalyzeMediaFileResult> {
        ctx.require_admin()?;
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;

        match scan_service.queue_analyze_job(&media_file_id, &path).await {
            Ok(queued) => Ok(AnalyzeMediaFileResult {
                success: true,
                queued,
                message: Some(if queued {
                    "Analysis job queued".to_string()
                } else {
                    "Analysis already queued".to_string()
                }),
            }),
            Err(e) => Ok(AnalyzeMediaFileResult {
                success: false,
                queued: false,
                message: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "retryScanIssue")]
    async fn retry_scan_issue(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "issueId")] issue_id: String,
    ) -> async_graphql::Result<ScanIssueActionResult> {
        ctx.require_admin()?;
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        match scan_service.retry_analysis_issue(&issue_id).await {
            Ok(queued) => Ok(ScanIssueActionResult {
                success: true,
                queued,
                message: if queued {
                    "Analysis retry queued.".to_string()
                } else {
                    "Analysis is already queued or the file is already analyzed.".to_string()
                },
            }),
            Err(error) => {
                tracing::warn!(
                    scan_issue_id = %issue_id,
                    error = %error,
                    "Failed to retry scan issue"
                );
                Ok(ScanIssueActionResult {
                    success: false,
                    queued: false,
                    message: "The analysis retry could not be queued. Review server health and the issue details.".to_string(),
                })
            }
        }
    }

    #[graphql(name = "resolveScanIssue")]
    async fn resolve_scan_issue(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "issueId")] issue_id: String,
        resolution: String,
    ) -> async_graphql::Result<ScanIssueActionResult> {
        ctx.require_admin()?;
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        match scan_service
            .resolve_scan_issue(&issue_id, &resolution)
            .await
        {
            Ok(()) => Ok(ScanIssueActionResult {
                success: true,
                queued: false,
                message: "Scan issue marked resolved.".to_string(),
            }),
            Err(error) => {
                tracing::warn!(
                    scan_issue_id = %issue_id,
                    error = %error,
                    "Failed to resolve scan issue"
                );
                Ok(ScanIssueActionResult {
                    success: false,
                    queued: false,
                    message: "The scan issue could not be resolved.".to_string(),
                })
            }
        }
    }

    #[graphql(name = "trashDuplicateScanIssue")]
    async fn trash_duplicate_scan_issue(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "issueId")] issue_id: String,
    ) -> async_graphql::Result<TrashDuplicateResult> {
        ctx.require_admin()?;
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        match scan_service.trash_duplicate_issue(&issue_id).await {
            Ok(result) => Ok(TrashDuplicateResult {
                success: true,
                message: "The duplicate was moved to recoverable library trash.".to_string(),
                old_path: Some(result.old_path),
                trash_path: Some(result.trash_path),
            }),
            Err(error) => {
                tracing::warn!(
                    scan_issue_id = %issue_id,
                    error = %error,
                    "Failed to move duplicate scan issue to trash"
                );
                Ok(TrashDuplicateResult {
                    success: false,
                    message: "The duplicate was not changed. Rescan it and review server logs before retrying.".to_string(),
                    old_path: None,
                    trash_path: None,
                })
            }
        }
    }

    #[graphql(name = "matchMediaFile")]
    async fn match_media_file(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: MatchMediaFileInput,
    ) -> async_graphql::Result<MatchMediaFileResult> {
        let auth_user = ctx.require_member()?.clone();
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;

        let methods = input
            .methods
            .unwrap_or_default()
            .into_iter()
            .map(MatchMethod::from)
            .collect::<Vec<_>>();

        match scan_service
            .match_media_file(MatchRequest {
                media_file_id: input.media_file_id,
                library_id: input.library_id,
                episode_id: input.episode_id,
                movie_id: input.movie_id,
                track_id: input.track_id,
                chapter_id: input.chapter_id,
                methods,
                requested_by_user_id: Some(auth_user.user_id),
                force: input.force.unwrap_or(false),
                auto_match: input.auto_match.unwrap_or(true),
                candidate_limit: input.candidate_limit.unwrap_or(10).max(0) as usize,
                allow_provider_fallback: input.allow_provider_fallback.unwrap_or(false),
                wanted_policy: input
                    .wanted_policy
                    .map(MatchWantedPolicy::from)
                    .unwrap_or(MatchWantedPolicy::PreferWanted),
            })
            .await
        {
            Ok(result) => Ok(MatchMediaFileResult {
                success: result.success,
                auto_matched: result.auto_matched,
                already_matched: result.already_matched,
                matched_type: result.matched_type,
                matched_id: result.matched_id,
                confidence: result.confidence,
                reason: result.reason,
                candidates: result
                    .candidates
                    .into_iter()
                    .map(|c| MatchCandidateGql {
                        target_type: c.target_type,
                        target_id: c.target_id,
                        target_name: c.target_name,
                        score: c.score,
                        reason: c.reason,
                        wanted: c.wanted,
                    })
                    .collect(),
            }),
            Err(e) => Ok(MatchMediaFileResult {
                success: false,
                auto_matched: false,
                already_matched: false,
                matched_type: None,
                matched_id: None,
                confidence: 0.0,
                reason: Some(e.to_string()),
                candidates: Vec::new(),
            }),
        }
    }

    #[graphql(name = "organizeMediaFile")]
    async fn organize_media_file(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: OrganizeMediaFileInput,
    ) -> async_graphql::Result<OrganizeMediaFileResult> {
        ctx.require_member()?;
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;

        match scan_service.organize_media_file(&input.media_file_id).await {
            Ok(result) => Ok(OrganizeMediaFileResult {
                success: result.success,
                old_path: result.old_path,
                new_path: result.new_path,
                reason: result.reason,
            }),
            Err(e) => Ok(OrganizeMediaFileResult {
                success: false,
                old_path: None,
                new_path: None,
                reason: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "unmatchMediaFile")]
    async fn unmatch_media_file(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "mediaFileId")] media_file_id: String,
    ) -> async_graphql::Result<UnmatchMediaFileResult> {
        ctx.require_member()?;
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;

        match scan_service.unmatch_media_file(&media_file_id).await {
            Ok(()) => Ok(UnmatchMediaFileResult {
                success: true,
                reason: Some("Media file unmatched".to_string()),
            }),
            Err(e) => Ok(UnmatchMediaFileResult {
                success: false,
                reason: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "testOllamaConnection")]
    async fn test_ollama_connection(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: TestOllamaConnectionInput,
    ) -> async_graphql::Result<OllamaConnectionResult> {
        ctx.require_admin()?;
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;

        let url = match input.ollama_url {
            Some(url) if !url.trim().is_empty() => url,
            _ => scan_service
                .load_ollama_settings("movies")
                .await
                .map(|settings| settings.ollama_url)
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
        };
        let endpoint = format!("{}/api/tags", url.trim_end_matches('/'));

        let client = crate::services::http_client::outbound_client(
            crate::services::http_client::OutboundHttpProfile::LocalService,
        )
        .map_err(|error| async_graphql::Error::new(error.to_string()))?;

        match client.get(endpoint).send().await {
            Ok(response) if response.status().is_success() => {
                match response.json::<OllamaTagsResponse>().await {
                    Ok(tags) => Ok(OllamaConnectionResult {
                        success: true,
                        models: tags.models.into_iter().map(|model| model.name).collect(),
                        error: None,
                    }),
                    Err(error) => Ok(OllamaConnectionResult {
                        success: false,
                        models: Vec::new(),
                        error: Some(format!("Failed to parse Ollama model list: {error}")),
                    }),
                }
            }
            Ok(response) => Ok(OllamaConnectionResult {
                success: false,
                models: Vec::new(),
                error: Some(format!("Ollama returned HTTP {}", response.status())),
            }),
            Err(error) => Ok(OllamaConnectionResult {
                success: false,
                models: Vec::new(),
                error: Some(error.to_string()),
            }),
        }
    }

    #[graphql(name = "testLlmParser")]
    async fn test_llm_parser(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: TestLlmParserInput,
    ) -> async_graphql::Result<LlmParserTestResult> {
        ctx.require_admin()?;
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;

        let regex_result =
            crate::services::library_scan::LibraryScanService::deterministic_parser_preview(
                &input.library_type,
                &input.filename,
            );

        match scan_service
            .test_ollama_parser(&input.library_type, &input.filename)
            .await
        {
            Ok(hint) => Ok(LlmParserTestResult {
                success: true,
                regex_result,
                llm_result: Some(LlmParsedHintResult::from_hint(hint, &input.library_type)),
                error: None,
            }),
            Err(error) => Ok(LlmParserTestResult {
                success: false,
                regex_result,
                llm_result: None,
                error: Some(error.to_string()),
            }),
        }
    }
}
