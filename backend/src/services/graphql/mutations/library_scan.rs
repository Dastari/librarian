use async_graphql::{Context, Enum, InputObject, Object, SimpleObject};

use crate::services::graphql::auth::AuthExt;
use crate::services::library_scan::{MatchMethod, MatchRequest};

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

#[derive(Debug, Clone, InputObject)]
#[graphql(name = "MatchMediaFileInput")]
pub struct MatchMediaFileInput {
    #[graphql(name = "MediaFileId")]
    pub media_file_id: String,
    #[graphql(name = "LibraryId")]
    pub library_id: Option<String>,
    #[graphql(name = "EpisodeId")]
    pub episode_id: Option<String>,
    #[graphql(name = "MovieId")]
    pub movie_id: Option<String>,
    #[graphql(name = "TrackId")]
    pub track_id: Option<String>,
    #[graphql(name = "ChapterId")]
    pub chapter_id: Option<String>,
    #[graphql(name = "Methods")]
    pub methods: Option<Vec<MatchMethodGql>>,
}

#[derive(Debug, Clone, InputObject)]
#[graphql(name = "OrganizeMediaFileInput")]
pub struct OrganizeMediaFileInput {
    #[graphql(name = "MediaFileId")]
    pub media_file_id: String,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "ScanLibraryResult")]
pub struct ScanLibraryResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "Status")]
    pub status: String,
    #[graphql(name = "Message")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "AnalyzeMediaFileResult")]
pub struct AnalyzeMediaFileResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "Queued")]
    pub queued: bool,
    #[graphql(name = "Message")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "MatchMediaFileResult")]
pub struct MatchMediaFileResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "MatchedType")]
    pub matched_type: Option<String>,
    #[graphql(name = "MatchedId")]
    pub matched_id: Option<String>,
    #[graphql(name = "Confidence")]
    pub confidence: f64,
    #[graphql(name = "Reason")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "OrganizeMediaFileResult")]
pub struct OrganizeMediaFileResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "OldPath")]
    pub old_path: Option<String>,
    #[graphql(name = "NewPath")]
    pub new_path: Option<String>,
    #[graphql(name = "Reason")]
    pub reason: Option<String>,
}

#[derive(Default)]
pub struct LibraryScanMutations;

#[Object]
impl LibraryScanMutations {
    #[graphql(name = "ScanLibrary")]
    async fn scan_library(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Id")] id: String,
    ) -> async_graphql::Result<ScanLibraryResult> {
        let _user = ctx.auth_user()?;
        let services = ctx.data_unchecked::<std::sync::Arc<crate::services::ServicesManager>>();
        let scan_service = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;

        match scan_service.queue_scan(&id).await {
            Ok(started) => Ok(ScanLibraryResult {
                success: true,
                status: if started {
                    "queued".to_string()
                } else {
                    "already_scanning".to_string()
                },
                message: Some(if started {
                    "Library scan queued".to_string()
                } else {
                    "Library is already scanning".to_string()
                }),
            }),
            Err(e) => Ok(ScanLibraryResult {
                success: false,
                status: "error".to_string(),
                message: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "AnalyzeMediaFile")]
    async fn analyze_media_file(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "MediaFileId")] media_file_id: String,
        #[graphql(name = "Path")] path: String,
    ) -> async_graphql::Result<AnalyzeMediaFileResult> {
        let _user = ctx.auth_user()?;
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

    #[graphql(name = "MatchMediaFile")]
    async fn match_media_file(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: MatchMediaFileInput,
    ) -> async_graphql::Result<MatchMediaFileResult> {
        let _user = ctx.auth_user()?;
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
            })
            .await
        {
            Ok(result) => Ok(MatchMediaFileResult {
                success: result.success,
                matched_type: result.matched_type,
                matched_id: result.matched_id,
                confidence: result.confidence,
                reason: result.reason,
            }),
            Err(e) => Ok(MatchMediaFileResult {
                success: false,
                matched_type: None,
                matched_id: None,
                confidence: 0.0,
                reason: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "OrganizeMediaFile")]
    async fn organize_media_file(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: OrganizeMediaFileInput,
    ) -> async_graphql::Result<OrganizeMediaFileResult> {
        let _user = ctx.auth_user()?;
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
}
