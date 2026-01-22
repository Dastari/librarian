//! External service integrations
//!
//! Re-exports are provided for convenience, even if not all are used within the crate.

#![allow(unused_imports)]

pub mod artwork;
pub mod audible;
pub mod cache;
pub mod cast;
pub mod download_source;
pub mod extractor;
pub mod ffmpeg;
pub mod file_matcher;
pub mod file_processor;
pub mod file_utils;
pub mod filename_parser;
pub mod filesystem;
pub mod hunt;
pub mod job_queue;
pub mod logging;
pub mod match_scorer;
pub mod metadata;
pub mod metrics;
pub mod musicbrainz;
pub mod notifications;
pub mod ollama;
pub mod opensubtitles;
pub mod organizer;
pub mod quality_evaluator;
pub mod queues;
pub mod rate_limiter;
pub mod rss;
pub mod scanner;
pub mod supabase_storage;
pub mod text_utils;
pub mod tmdb;
pub mod torrent;
pub mod torrent_completion_handler;
pub mod torrent_metadata;
pub mod track_matcher;
pub mod tvmaze;
pub mod usenet;

pub use artwork::ArtworkService;
pub use cast::{
    CastDevicesEvent, CastPlayerState, CastService, CastServiceConfig, CastSessionEvent,
};
pub use ffmpeg::{
    AudioStream, Chapter, FfmpegService, HdrType, MediaAnalysis, SubtitleStream, VideoStream,
};
pub use file_matcher::{
    FileInfo, FileMatcher, KnownMatchTarget, MatchSummary, MediaMetadata, VerificationResult,
    read_audio_metadata, read_video_metadata,
};
pub use file_processor::{FileProcessor, ProcessResult, ProcessTarget};
pub use file_utils::{
    ARCHIVE_EXTENSIONS, AUDIO_EXTENSIONS, SUBTITLE_EXTENSIONS, VIDEO_EXTENSIONS, format_bytes,
    format_bytes_i64, get_container, is_archive_file, is_audio_file, is_subtitle_file,
    is_video_file, sanitize_for_filename,
};
pub use filesystem::{DirectoryChangeEvent, FilesystemService, FilesystemServiceConfig};
pub use job_queue::{
    ConcurrencyLimiter, JobQueueConfig, MetadataQueue, WorkQueue, process_concurrent,
    process_in_chunks,
};
pub use logging::{DatabaseLoggerConfig, LogEvent, create_database_layer};
pub use metadata::{
    AddMovieOptions, AddTvShowOptions, MetadataProvider, MetadataService, MetadataServiceConfig,
    MovieDetails, MovieSearchResult, create_metadata_service_with_artwork,
};
pub use metrics::{
    DatabaseSnapshot, MetricsCollector, SharedMetrics, SystemSnapshot, create_metrics_collector,
    format_bytes_short, format_uptime,
};
pub use notifications::{
    NotificationCountEvent, NotificationEvent, NotificationEventType, NotificationService,
    NotificationServiceConfig, create_notification_service,
};
pub use ollama::{LlmParseResult, OllamaConfig, OllamaService};
pub use opensubtitles::{
    DownloadedSubtitle, OpenSubtitlesClient, SubtitleSearchQuery, SubtitleSearchResult,
};
pub use organizer::{
    CleanupResult, ConsolidateResult, DeduplicationResult, OrganizerService, TorrentFileForOrganize,
};
pub use quality_evaluator::{
    EffectiveQualitySettings, QualityEvaluation, QualityEvaluator, QualityStatus,
};
pub use queues::{
    MediaAnalysisJob, MediaAnalysisQueue, SubtitleDownloadJob, SubtitleDownloadQueue,
    create_media_analysis_queue, create_subtitle_download_queue, media_analysis_queue_config,
    subtitle_download_queue_config,
};
pub use rate_limiter::{RateLimitConfig, RateLimitedClient, RetryConfig, retry_async};
pub use rss::{ParsedRssItem, RssService, validate_url_for_ssrf};
pub use scanner::{
    ScannerConfig, ScannerService, create_scanner_service, create_scanner_service_with_config,
};
pub use supabase_storage::StorageClient;
pub use text_utils::{
    levenshtein_distance, normalize_quality, normalize_show_name, normalize_show_name_no_articles,
    normalize_title, normalize_track_title, show_name_similarity, string_similarity,
};
pub use tmdb::{
    TmdbClient, TmdbCollection, TmdbCredits, TmdbMovie, TmdbMovieSearchResult, TmdbReleaseDates,
    normalize_movie_status,
};
pub use torrent::{
    PeerStats, TorrentDetails, TorrentEvent, TorrentFile, TorrentInfo, TorrentService,
    TorrentServiceConfig, TorrentState,
};
pub use torrent_completion_handler::{
    CompletionHandlerConfig, CompletionHandlerHandle, TorrentCompletionHandler,
};
pub use torrent_metadata::{
    TorrentFileInfo, audio_summary, extract_audio_files, is_single_file_album, parse_torrent_files,
};
pub use track_matcher::{MatchType, TrackMatch, TrackMatchResult, match_tracks};
pub use hunt::{HuntConfig, HuntSearchResult, HuntService};
pub use download_source::{DownloadSource, DownloadSourceType, LinkedItem};
pub use usenet::{UsenetDownloadInfo, UsenetEvent, UsenetService};