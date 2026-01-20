//! External service integrations
//!
//! Re-exports are provided for convenience, even if not all are used within the crate.

#![allow(unused_imports)]

pub mod artwork;
pub mod audible;
pub mod extractor;
pub mod cache;
pub mod cast;
pub mod ffmpeg;
pub mod file_utils;
pub mod filename_parser;
pub mod filesystem;
pub mod job_queue;
pub mod logging;
pub mod metadata;
pub mod metrics;
pub mod musicbrainz;
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
pub mod torrent_file_matcher;
pub mod torrent_metadata;
pub mod torrent_processor;
pub mod track_matcher;
pub mod tvmaze;

pub use artwork::ArtworkService;
pub use cast::{CastPlayerState, CastService, CastServiceConfig, CastSessionEvent, CastDevicesEvent};
pub use ffmpeg::{
    AudioStream, Chapter, FfmpegService, HdrType, MediaAnalysis, SubtitleStream, VideoStream,
};
pub use filesystem::{DirectoryChangeEvent, FilesystemService, FilesystemServiceConfig};
pub use job_queue::{
    ConcurrencyLimiter, JobQueueConfig, MetadataQueue, WorkQueue,
    process_concurrent, process_in_chunks,
};
pub use logging::{DatabaseLoggerConfig, LogEvent, create_database_layer};
pub use metrics::{
    DatabaseSnapshot, MetricsCollector, SharedMetrics, SystemSnapshot,
    create_metrics_collector, format_bytes_short, format_uptime,
};
pub use metadata::{
    AddMovieOptions, AddTvShowOptions, MetadataProvider, MetadataService, MetadataServiceConfig,
    MovieDetails, MovieSearchResult, create_metadata_service_with_artwork,
};
pub use ollama::{LlmParseResult, OllamaConfig, OllamaService};
pub use opensubtitles::{
    DownloadedSubtitle, OpenSubtitlesClient, SubtitleSearchQuery, SubtitleSearchResult,
};
pub use organizer::{CleanupResult, ConsolidateResult, DeduplicationResult, OrganizerService, TorrentFileForOrganize};
pub use quality_evaluator::{EffectiveQualitySettings, QualityEvaluation, QualityEvaluator, QualityStatus};
pub use queues::{
    MediaAnalysisJob, MediaAnalysisQueue, SubtitleDownloadJob, SubtitleDownloadQueue,
    create_media_analysis_queue, create_subtitle_download_queue,
    media_analysis_queue_config, subtitle_download_queue_config,
};
pub use rate_limiter::{RateLimitConfig, RateLimitedClient, RetryConfig, retry_async};
pub use rss::{ParsedRssItem, RssService, validate_url_for_ssrf};
pub use scanner::{ScannerConfig, ScannerService, create_scanner_service, create_scanner_service_with_config};
pub use supabase_storage::StorageClient;
pub use tmdb::{
    TmdbClient, TmdbMovie, TmdbMovieSearchResult, TmdbCredits, TmdbCollection,
    TmdbReleaseDates, normalize_movie_status,
};
pub use torrent::{
    PeerStats, TorrentDetails, TorrentEvent, TorrentFile, TorrentInfo, TorrentService,
    TorrentServiceConfig, TorrentState,
};
pub use torrent_completion_handler::{
    CompletionHandlerConfig, CompletionHandlerHandle, TorrentCompletionHandler,
};
pub use torrent_file_matcher::{FileMatchResult, FileMatchTarget, FileMatchType, TorrentFileMatcher};
pub use torrent_processor::{ProcessTorrentResult, TorrentProcessor};
pub use torrent_metadata::{TorrentFileInfo, parse_torrent_files, extract_audio_files, is_single_file_album, audio_summary};
pub use track_matcher::{TrackMatchResult, TrackMatch, MatchType, match_tracks};
pub use file_utils::{
    format_bytes, format_bytes_i64, get_container, is_archive_file, is_audio_file,
    is_subtitle_file, is_video_file, sanitize_for_filename, ARCHIVE_EXTENSIONS,
    AUDIO_EXTENSIONS, SUBTITLE_EXTENSIONS, VIDEO_EXTENSIONS,
};
pub use text_utils::{
    normalize_quality, normalize_show_name, normalize_show_name_no_articles,
    normalize_title, normalize_track_title, levenshtein_distance, string_similarity,
    show_name_similarity,
};
