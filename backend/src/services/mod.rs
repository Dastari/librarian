//! External service integrations

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
pub mod musicbrainz;
pub mod opensubtitles;
pub mod organizer;
pub mod prowlarr;
pub mod queues;
pub mod rate_limiter;
pub mod rss;
pub mod scanner;
pub mod supabase_storage;
pub mod tmdb;
pub mod torrent;
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
pub use metadata::{
    AddMovieOptions, AddTvShowOptions, MetadataProvider, MetadataService, MetadataServiceConfig,
    MovieDetails, MovieSearchResult, create_metadata_service_with_artwork,
};
pub use opensubtitles::{
    DownloadedSubtitle, OpenSubtitlesClient, SubtitleSearchQuery, SubtitleSearchResult,
};
pub use organizer::{OrganizerService, TorrentFileForOrganize};
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
    PeerStats, TorrentDetails, TorrentEvent, TorrentInfo, TorrentService, TorrentServiceConfig,
    TorrentState,
};
pub use file_utils::{
    format_bytes, format_bytes_i64, get_container, is_archive_file, is_audio_file,
    is_subtitle_file, is_video_file, sanitize_for_filename, ARCHIVE_EXTENSIONS,
    AUDIO_EXTENSIONS, SUBTITLE_EXTENSIONS, VIDEO_EXTENSIONS,
};
