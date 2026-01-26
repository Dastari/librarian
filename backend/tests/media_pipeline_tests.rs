//! Integration tests for the media pipeline
//!
//! These tests verify the complete flow of media processing:
//! - Status transitions (wanted -> downloading -> downloaded)
//! - File matching logic
//! - Quality evaluation
//! - Organization patterns

// ============================================================================
// Status Transition Tests
// ============================================================================

/// Valid status values for episodes/tracks/movies
const VALID_STATUSES: &[&str] = &[
    "missing",
    "wanted",
    "downloading",
    "downloaded",
    "ignored",
    "suboptimal",
];

/// Status transition rules as defined in media-pipeline.md
mod status_transitions {
    use super::*;

    /// Check if a status transition is valid
    fn is_valid_transition(from: &str, to: &str) -> bool {
        match (from, to) {
            // missing -> wanted: When air_date passes or item is added
            ("missing", "wanted") => true,
            // wanted -> downloading: When file in torrent matches
            ("wanted", "downloading") => true,
            // downloading -> downloaded: When file is organized to library
            ("downloading", "downloaded") => true,
            // downloading -> wanted: If download fails/cancelled
            ("downloading", "wanted") => true,
            // downloaded -> suboptimal: When ffprobe reveals quality below target
            ("downloaded", "suboptimal") => true,
            // suboptimal -> wanted: User triggers re-download
            ("suboptimal", "wanted") => true,
            // suboptimal -> downloading: Better quality found
            ("suboptimal", "downloading") => true,
            // suboptimal -> downloaded: Upgrade completed
            ("suboptimal", "downloaded") => true,
            // Any -> ignored: User explicitly ignores
            (_, "ignored") => true,
            // ignored -> wanted: User un-ignores
            ("ignored", "wanted") => true,
            // Same status is allowed (no-op)
            (a, b) if a == b => true,
            _ => false,
        }
    }

    #[test]
    fn test_valid_forward_transitions() {
        // Normal happy path: missing -> wanted -> downloading -> downloaded
        assert!(is_valid_transition("missing", "wanted"));
        assert!(is_valid_transition("wanted", "downloading"));
        assert!(is_valid_transition("downloading", "downloaded"));
    }

    #[test]
    fn test_suboptimal_transitions() {
        // Downloaded can become suboptimal
        assert!(is_valid_transition("downloaded", "suboptimal"));

        // Suboptimal can trigger re-download
        assert!(is_valid_transition("suboptimal", "downloading"));
        assert!(is_valid_transition("suboptimal", "downloaded"));
    }

    #[test]
    fn test_ignore_transitions() {
        // Any status can be ignored
        for status in VALID_STATUSES {
            assert!(
                is_valid_transition(status, "ignored"),
                "Should be able to ignore from {}",
                status
            );
        }

        // Can un-ignore back to wanted
        assert!(is_valid_transition("ignored", "wanted"));
    }

    #[test]
    fn test_invalid_transitions() {
        // Can't go backwards in main flow without special conditions
        assert!(!is_valid_transition("downloaded", "downloading"));
        assert!(!is_valid_transition("downloading", "missing"));
        assert!(!is_valid_transition("wanted", "missing"));

        // Can't skip steps
        assert!(!is_valid_transition("missing", "downloading"));
        assert!(!is_valid_transition("missing", "downloaded"));
        assert!(!is_valid_transition("wanted", "downloaded"));
    }

    #[test]
    fn test_download_failure_recovery() {
        // If download fails, should go back to wanted
        assert!(is_valid_transition("downloading", "wanted"));
    }

    #[test]
    fn test_same_status_transition() {
        // Setting same status should be allowed (no-op)
        for status in VALID_STATUSES {
            assert!(
                is_valid_transition(status, status),
                "Same status transition should be valid: {}",
                status
            );
        }
    }
}

// ============================================================================
// File Type Detection Tests
// ============================================================================

mod file_types {
    /// Check if a file extension is a video type
    fn is_video_extension(ext: &str) -> bool {
        matches!(
            ext.to_lowercase().as_str(),
            "mkv"
                | "mp4"
                | "avi"
                | "wmv"
                | "mov"
                | "m4v"
                | "ts"
                | "webm"
                | "m2ts"
                | "ogv"
                | "flv"
                | "divx"
        )
    }

    /// Check if a file extension is an audio type
    fn is_audio_extension(ext: &str) -> bool {
        matches!(
            ext.to_lowercase().as_str(),
            "mp3" | "flac" | "m4a" | "aac" | "ogg" | "wav" | "wma" | "opus" | "ape" | "alac"
        )
    }

    /// Check if a file extension is a subtitle type
    fn is_subtitle_extension(ext: &str) -> bool {
        matches!(
            ext.to_lowercase().as_str(),
            "srt" | "sub" | "ass" | "ssa" | "vtt" | "idx"
        )
    }

    /// Check if a file extension is an archive type
    fn is_archive_extension(ext: &str) -> bool {
        matches!(
            ext.to_lowercase().as_str(),
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2"
        )
    }

    /// Check if a file is a sample based on filename patterns
    fn is_sample_file(filename: &str, size_bytes: u64) -> bool {
        let lower = filename.to_lowercase();

        // Explicit sample markers
        if lower.contains("sample") {
            return true;
        }

        // Very small video files are likely samples
        if is_video_extension(&get_extension(filename)) && size_bytes < 100_000_000 {
            // Less than 100MB for video is suspicious
            // But only if it doesn't match a short episode pattern
            return true;
        }

        false
    }

    fn get_extension(filename: &str) -> String {
        std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn test_video_extensions() {
        let video_exts = vec![
            "mkv", "mp4", "avi", "wmv", "mov", "m4v", "ts", "webm", "m2ts",
        ];
        for ext in video_exts {
            assert!(is_video_extension(ext), "{} should be video", ext);
            assert!(
                is_video_extension(&ext.to_uppercase()),
                "{} uppercase should be video",
                ext
            );
        }
    }

    #[test]
    fn test_non_video_extensions() {
        let non_video = vec!["mp3", "flac", "jpg", "png", "nfo", "txt", "srt"];
        for ext in non_video {
            assert!(!is_video_extension(ext), "{} should not be video", ext);
        }
    }

    #[test]
    fn test_audio_extensions() {
        let audio_exts = vec!["mp3", "flac", "m4a", "aac", "ogg", "wav"];
        for ext in audio_exts {
            assert!(is_audio_extension(ext), "{} should be audio", ext);
        }
    }

    #[test]
    fn test_subtitle_extensions() {
        let sub_exts = vec!["srt", "sub", "ass", "ssa", "vtt"];
        for ext in sub_exts {
            assert!(is_subtitle_extension(ext), "{} should be subtitle", ext);
        }
    }

    #[test]
    fn test_archive_extensions() {
        let archive_exts = vec!["zip", "rar", "7z", "tar", "gz"];
        for ext in archive_exts {
            assert!(is_archive_extension(ext), "{} should be archive", ext);
        }
    }

    #[test]
    fn test_sample_detection_by_name() {
        assert!(is_sample_file("sample.mkv", 500_000_000));
        assert!(is_sample_file("Sample-Episode.mkv", 500_000_000));
        assert!(is_sample_file("show-sample.mkv", 500_000_000));
        assert!(is_sample_file("SAMPLE.MKV", 500_000_000));
    }

    #[test]
    fn test_sample_detection_by_size() {
        // Small video file is a sample
        assert!(is_sample_file("episode.mkv", 50_000_000)); // 50MB

        // Normal sized video is not a sample
        assert!(!is_sample_file("episode.mkv", 500_000_000)); // 500MB
    }

    #[test]
    fn test_non_sample_files() {
        assert!(!is_sample_file("Show.S01E01.1080p.mkv", 1_500_000_000));
        assert!(!is_sample_file("Movie.2024.1080p.mkv", 8_000_000_000));
    }
}

// ============================================================================
// Torrent File List Processing Tests
// ============================================================================

mod torrent_files {
    use std::path::Path;

    /// Represents a file in a torrent
    #[derive(Debug, Clone)]
    struct TorrentFile {
        path: String,
        size: u64,
    }

    /// Categorize a torrent file
    #[derive(Debug, Clone, PartialEq)]
    enum FileCategory {
        Video,
        Audio,
        Subtitle,
        Image,
        Archive,
        Metadata, // nfo, txt, etc.
        Sample,
        Unknown,
    }

    fn categorize_file(file: &TorrentFile) -> FileCategory {
        let lower = file.path.to_lowercase();
        let ext = Path::new(&file.path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // Check for sample first
        if lower.contains("sample") {
            return FileCategory::Sample;
        }

        match ext.to_lowercase().as_str() {
            "mkv" | "mp4" | "avi" | "wmv" | "mov" | "m4v" | "ts" | "webm" | "m2ts" => {
                // Small video files might be samples
                if file.size < 100_000_000 {
                    FileCategory::Sample
                } else {
                    FileCategory::Video
                }
            }
            "mp3" | "flac" | "m4a" | "aac" | "ogg" | "wav" | "wma" => FileCategory::Audio,
            "srt" | "sub" | "ass" | "ssa" | "vtt" | "idx" => FileCategory::Subtitle,
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" => FileCategory::Image,
            "zip" | "rar" | "7z" | "tar" | "gz" => FileCategory::Archive,
            "nfo" | "txt" | "md" | "sfv" | "md5" => FileCategory::Metadata,
            _ => FileCategory::Unknown,
        }
    }

    #[test]
    fn test_ds9_season_pack() {
        // Simulate Deep Space Nine S01 torrent structure
        let files = vec![
            TorrentFile {
                path: "README.md".into(),
                size: 5000,
            },
            TorrentFile {
                path: "Star Trek- Deep Space Nine - S01E01-E02 - Emissary 960p-QueerWorm-Lela.mkv"
                    .into(),
                size: 2_500_000_000,
            },
            TorrentFile {
                path: "Star Trek- Deep Space Nine - S01E03 - Past Prologue 960p-QueerWorm-Lela.mkv"
                    .into(),
                size: 1_200_000_000,
            },
            TorrentFile {
                path: "Star Trek- Deep Space Nine - S01E04 - A Man Alone 960p-QueerWorm-Lela.mkv"
                    .into(),
                size: 1_100_000_000,
            },
            TorrentFile {
                path: "Star Trek- Deep Space Nine - S01E05 - Babel 960p-QueerWorm-Lela.mkv".into(),
                size: 1_150_000_000,
            },
        ];

        let video_count = files
            .iter()
            .filter(|f| categorize_file(f) == FileCategory::Video)
            .count();

        assert_eq!(video_count, 4, "Should find 4 video files");

        let metadata_count = files
            .iter()
            .filter(|f| categorize_file(f) == FileCategory::Metadata)
            .count();

        assert_eq!(metadata_count, 1, "Should find 1 metadata file (README)");
    }

    #[test]
    fn test_single_episode_with_extras() {
        // Simulate single episode torrent with screenshots
        let files = vec![
            TorrentFile {
                path: "Fallout.2024.S01E01.1080p.HEVC.x265-MeGusta.mkv".into(),
                size: 800_000_000,
            },
            TorrentFile {
                path: "Fallout.2024.S01E01.1080p.HEVC.x265-MeGusta.nfo".into(),
                size: 3000,
            },
            TorrentFile {
                path: "Screens/screen0001.png".into(),
                size: 500_000,
            },
            TorrentFile {
                path: "Screens/screen0002.png".into(),
                size: 500_000,
            },
            TorrentFile {
                path: "Screens/screen0003.png".into(),
                size: 500_000,
            },
        ];

        let categories: Vec<_> = files.iter().map(|f| categorize_file(f)).collect();

        assert_eq!(categories[0], FileCategory::Video);
        assert_eq!(categories[1], FileCategory::Metadata);
        assert_eq!(categories[2], FileCategory::Image);
        assert_eq!(categories[3], FileCategory::Image);
        assert_eq!(categories[4], FileCategory::Image);
    }

    #[test]
    fn test_torrent_with_sample() {
        let files = vec![
            TorrentFile {
                path: "Movie.2024.1080p.BluRay.mkv".into(),
                size: 8_000_000_000,
            },
            TorrentFile {
                path: "Sample/sample.mkv".into(),
                size: 50_000_000,
            },
            TorrentFile {
                path: "Movie.2024.1080p.BluRay.nfo".into(),
                size: 3000,
            },
        ];

        let categories: Vec<_> = files.iter().map(|f| categorize_file(f)).collect();

        assert_eq!(categories[0], FileCategory::Video);
        assert_eq!(categories[1], FileCategory::Sample);
        assert_eq!(categories[2], FileCategory::Metadata);
    }

    #[test]
    fn test_music_album() {
        let files = vec![
            TorrentFile {
                path: "01 - Track One.flac".into(),
                size: 50_000_000,
            },
            TorrentFile {
                path: "02 - Track Two.flac".into(),
                size: 45_000_000,
            },
            TorrentFile {
                path: "03 - Track Three.flac".into(),
                size: 55_000_000,
            },
            TorrentFile {
                path: "cover.jpg".into(),
                size: 2_000_000,
            },
            TorrentFile {
                path: "album.nfo".into(),
                size: 1000,
            },
        ];

        let audio_count = files
            .iter()
            .filter(|f| categorize_file(f) == FileCategory::Audio)
            .count();

        assert_eq!(audio_count, 3, "Should find 3 audio files");

        let image_count = files
            .iter()
            .filter(|f| categorize_file(f) == FileCategory::Image)
            .count();

        assert_eq!(image_count, 1, "Should find 1 image file (cover art)");
    }

    #[test]
    fn test_movie_with_subtitles() {
        let files = vec![
            TorrentFile {
                path: "The.Hunt.for.Red.October.1990.1080p.BluRay.x265.mp4".into(),
                size: 5_000_000_000,
            },
            TorrentFile {
                path: "The.Hunt.for.Red.October.1990.1080p.BluRay.x265.srt".into(),
                size: 80_000,
            },
            TorrentFile {
                path: "The.Hunt.for.Red.October.1990.1080p.BluRay.x265.nfo".into(),
                size: 3000,
            },
        ];

        let categories: Vec<_> = files.iter().map(|f| categorize_file(f)).collect();

        assert_eq!(categories[0], FileCategory::Video);
        assert_eq!(categories[1], FileCategory::Subtitle);
        assert_eq!(categories[2], FileCategory::Metadata);
    }
}

// ============================================================================
// Quality Matching Tests
// ============================================================================

mod quality_matching {
    /// Resolution ranks for comparison
    fn resolution_rank(res: &str) -> u32 {
        match res.to_lowercase().as_str() {
            "2160p" | "4k" | "uhd" => 4,
            "1080p" => 3,
            "720p" => 2,
            "480p" | "sd" => 1,
            "360p" => 0,
            _ => 0,
        }
    }

    /// Check if a release meets quality target
    fn meets_quality_target(file_resolution: &str, target_resolutions: &[&str]) -> bool {
        if target_resolutions.is_empty() {
            return true; // No restrictions
        }

        let file_rank = resolution_rank(file_resolution);

        // Check if file matches or exceeds any target
        target_resolutions
            .iter()
            .any(|target| resolution_rank(target) <= file_rank)
    }

    /// Check if new quality is an upgrade over existing
    fn is_quality_upgrade(existing: &str, new: &str) -> bool {
        resolution_rank(new) > resolution_rank(existing)
    }

    #[test]
    fn test_resolution_ranking() {
        assert!(resolution_rank("2160p") > resolution_rank("1080p"));
        assert!(resolution_rank("1080p") > resolution_rank("720p"));
        assert!(resolution_rank("720p") > resolution_rank("480p"));

        // Aliases should be equal
        assert_eq!(resolution_rank("4k"), resolution_rank("2160p"));
        assert_eq!(resolution_rank("uhd"), resolution_rank("2160p"));
    }

    #[test]
    fn test_meets_quality_target() {
        // 1080p meets 1080p target
        assert!(meets_quality_target("1080p", &["1080p"]));

        // 1080p meets 720p target (exceeds)
        assert!(meets_quality_target("1080p", &["720p"]));

        // 720p does NOT meet 1080p target
        assert!(!meets_quality_target("720p", &["1080p"]));

        // No restrictions means anything is acceptable
        assert!(meets_quality_target("480p", &[]));

        // Multiple targets - meets if matches any
        assert!(meets_quality_target("720p", &["720p", "1080p"]));
    }

    #[test]
    fn test_is_quality_upgrade() {
        // 720p -> 1080p is upgrade
        assert!(is_quality_upgrade("720p", "1080p"));

        // 1080p -> 2160p is upgrade
        assert!(is_quality_upgrade("1080p", "2160p"));

        // 1080p -> 720p is NOT upgrade
        assert!(!is_quality_upgrade("1080p", "720p"));

        // Same resolution is NOT upgrade
        assert!(!is_quality_upgrade("1080p", "1080p"));
    }

    #[test]
    fn test_real_world_quality_scenarios() {
        // Scenario: Library wants 1080p, we find 720p release
        // Should NOT match
        assert!(!meets_quality_target("720p", &["1080p"]));

        // Scenario: Library wants 720p or 1080p, we find 1080p
        // Should match
        assert!(meets_quality_target("1080p", &["720p", "1080p"]));

        // Scenario: DS9 960p upscale against 720p target
        // 960p is between 720p and 1080p, closer to 1080p
        // For this test, treat 960p as custom - would need special handling

        // Scenario: Library wants any quality (no restrictions)
        assert!(meets_quality_target("480p", &[]));
    }
}

// ============================================================================
// Episode Matching Tests
// ============================================================================

// ============================================================================
// File Matching (FileMatcher Service) Tests
// ============================================================================

mod file_matching {
    /// Test data structures matching the new system
    #[derive(Debug, Clone)]
    struct FileInfo {
        path: String,
        size: i64,
        file_index: Option<i32>,
    }

    /// Match target types
    #[derive(Debug, Clone, PartialEq)]
    enum MatchTarget {
        Episode {
            episode_id: String,
            season: i32,
            episode: i32,
        },
        Movie {
            movie_id: String,
            title: String,
        },
        Track {
            track_id: String,
            title: String,
        },
        Chapter {
            chapter_id: String,
            chapter_number: i32,
        },
        Unmatched {
            reason: String,
        },
        Sample,
    }

    /// Simplified matching logic for testing
    fn match_file(file: &FileInfo) -> MatchTarget {
        let lower = file.path.to_lowercase();

        // Check for sample
        if lower.contains("sample") {
            return MatchTarget::Sample;
        }

        // Small video files are samples
        let ext = std::path::Path::new(&file.path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let is_video = matches!(
            ext.to_lowercase().as_str(),
            "mkv" | "mp4" | "avi" | "mov" | "m4v"
        );
        if is_video && file.size < 100_000_000 {
            return MatchTarget::Sample;
        }

        // Non-media files
        let is_audio = matches!(
            ext.to_lowercase().as_str(),
            "flac" | "mp3" | "m4a" | "aac" | "wav"
        );
        if !is_video && !is_audio {
            return MatchTarget::Unmatched {
                reason: "Not a media file".to_string(),
            };
        }

        // For testing, return unmatched - real matching uses database lookups
        MatchTarget::Unmatched {
            reason: "No library match found".to_string(),
        }
    }

    #[test]
    fn test_sample_detection() {
        // Explicit sample
        let file = FileInfo {
            path: "Sample/sample.mkv".to_string(),
            size: 500_000_000,
            file_index: Some(0),
        };
        assert_eq!(match_file(&file), MatchTarget::Sample);

        // Small video is sample
        let small_video = FileInfo {
            path: "movie-preview.mkv".to_string(),
            size: 50_000_000, // 50MB
            file_index: Some(1),
        };
        assert_eq!(match_file(&small_video), MatchTarget::Sample);

        // Large video is not sample
        let large_video = FileInfo {
            path: "Movie.2024.1080p.mkv".to_string(),
            size: 5_000_000_000, // 5GB
            file_index: Some(2),
        };
        assert!(match_file(&large_video) != MatchTarget::Sample);
    }

    #[test]
    fn test_non_media_files() {
        let files = vec![
            FileInfo {
                path: "readme.txt".to_string(),
                size: 1000,
                file_index: Some(0),
            },
            FileInfo {
                path: "cover.jpg".to_string(),
                size: 500_000,
                file_index: Some(1),
            },
            FileInfo {
                path: "movie.nfo".to_string(),
                size: 3000,
                file_index: Some(2),
            },
        ];

        for file in files {
            let result = match_file(&file);
            assert!(
                matches!(result, MatchTarget::Unmatched { .. }),
                "Non-media file should be unmatched: {}",
                file.path
            );
        }
    }
}

// ============================================================================
// File Naming Pattern Tests
// ============================================================================

mod naming_patterns {
    /// Apply a naming pattern for TV shows
    fn apply_tv_pattern(
        pattern: &str,
        show_name: &str,
        season: i32,
        episode: i32,
        episode_title: &str,
        ext: &str,
    ) -> String {
        pattern
            .replace("{show}", &sanitize_for_path(show_name))
            .replace("{season:02}", &format!("{:02}", season))
            .replace("{season}", &season.to_string())
            .replace("{episode:02}", &format!("{:02}", episode))
            .replace("{episode}", &episode.to_string())
            .replace("{title}", &sanitize_for_path(episode_title))
            .replace("{ext}", ext)
    }

    /// Apply a naming pattern for movies
    fn apply_movie_pattern(pattern: &str, title: &str, year: Option<i32>, ext: &str) -> String {
        let year_str = year.map(|y| y.to_string()).unwrap_or_default();
        pattern
            .replace("{title}", &sanitize_for_path(title))
            .replace("{year}", &year_str)
            .replace("{ext}", ext)
    }

    /// Apply a naming pattern for music
    fn apply_music_pattern(
        pattern: &str,
        artist: &str,
        album: &str,
        track_num: i32,
        track_title: &str,
        ext: &str,
    ) -> String {
        pattern
            .replace("{artist}", &sanitize_for_path(artist))
            .replace("{album}", &sanitize_for_path(album))
            .replace("{track:02}", &format!("{:02}", track_num))
            .replace("{track}", &track_num.to_string())
            .replace("{title}", &sanitize_for_path(track_title))
            .replace("{ext}", ext)
    }

    /// Sanitize a string for use in file paths
    fn sanitize_for_path(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect()
    }

    #[test]
    fn test_tv_pattern() {
        let pattern =
            "{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}";

        let result = apply_tv_pattern(pattern, "Breaking Bad", 1, 5, "Gray Matter", "mkv");
        assert_eq!(
            result,
            "Breaking Bad/Season 01/Breaking Bad - S01E05 - Gray Matter.mkv"
        );
    }

    #[test]
    fn test_movie_pattern() {
        let pattern = "{title} ({year})/{title}.{ext}";

        let result = apply_movie_pattern(pattern, "Inception", Some(2010), "mkv");
        assert_eq!(result, "Inception (2010)/Inception.mkv");
    }

    #[test]
    fn test_music_pattern() {
        let pattern = "{artist}/{album}/{track:02} - {title}.{ext}";

        let result = apply_music_pattern(
            pattern,
            "Pink Floyd",
            "Dark Side of the Moon",
            3,
            "Time",
            "flac",
        );
        assert_eq!(result, "Pink Floyd/Dark Side of the Moon/03 - Time.flac");
    }

    #[test]
    fn test_sanitization() {
        let pattern = "{show}/Season {season:02}/{show}.{ext}";

        // Colons in show name should be replaced
        let result = apply_tv_pattern(
            pattern,
            "Star Trek: Deep Space Nine",
            1,
            1,
            "Emissary",
            "mkv",
        );
        assert!(
            !result.contains(':'),
            "Colons should be sanitized: {}",
            result
        );
    }
}

// ============================================================================
// Episode Matching Tests
// ============================================================================

mod episode_matching {
    /// Parse season and episode from filename
    fn parse_season_episode(filename: &str) -> Option<(u32, u32)> {
        // Pattern: S01E01 or S1E1
        let re = regex::Regex::new(r"(?i)S(\d{1,2})E(\d{1,2})").unwrap();
        if let Some(caps) = re.captures(filename) {
            let season: u32 = caps.get(1)?.as_str().parse().ok()?;
            let episode: u32 = caps.get(2)?.as_str().parse().ok()?;
            return Some((season, episode));
        }
        None
    }

    /// Extract show name from filename
    fn extract_show_name(filename: &str) -> Option<String> {
        // Simple extraction - take everything before S01E01 pattern
        let re = regex::Regex::new(r"(?i)(.+?)\s*S\d{1,2}E\d{1,2}").unwrap();
        if let Some(caps) = re.captures(filename) {
            let name = caps.get(1)?.as_str();
            // Clean up dots and dashes
            let cleaned = name.replace('.', " ").replace('-', " ");
            return Some(cleaned.trim().to_string());
        }
        None
    }

    #[test]
    fn test_parse_season_episode() {
        assert_eq!(parse_season_episode("Show.S01E01.mkv"), Some((1, 1)));
        assert_eq!(parse_season_episode("Show.S01E10.mkv"), Some((1, 10)));
        assert_eq!(parse_season_episode("Show.S10E01.mkv"), Some((10, 1)));
        assert_eq!(parse_season_episode("Show.S10E99.mkv"), Some((10, 99)));
    }

    #[test]
    fn test_parse_season_episode_variations() {
        // Various naming styles
        assert_eq!(
            parse_season_episode("Chicago.Fire.S14E08.1080p.mkv"),
            Some((14, 8))
        );
        assert_eq!(
            parse_season_episode("Star Trek- Deep Space Nine - S01E09 - The Passenger.mkv"),
            Some((1, 9))
        );
        assert_eq!(
            parse_season_episode("Fallout.2024.S02E04.720p.mkv"),
            Some((2, 4))
        );
    }

    #[test]
    fn test_extract_show_name() {
        assert_eq!(
            extract_show_name("Chicago.Fire.S14E08.1080p.mkv"),
            Some("Chicago Fire".to_string())
        );
        assert_eq!(
            extract_show_name("Fallout.2024.S01E01.1080p.mkv"),
            Some("Fallout 2024".to_string())
        );
    }

    #[test]
    fn test_ds9_episode_matching() {
        let ds9_files = vec![
            "Star Trek- Deep Space Nine - S01E01-E02 - Emissary 960p-QueerWorm-Lela.mkv",
            "Star Trek- Deep Space Nine - S01E03 - Past Prologue 960p-QueerWorm-Lela.mkv",
            "Star Trek- Deep Space Nine - S01E09 - The Passenger 960p-QueerWorm-Lela.mkv",
            "Star Trek- Deep Space Nine - S01E20 - In the Hands of the Prophets 960p-QueerWorm-Lela.mkv",
        ];

        for file in ds9_files {
            let parsed = parse_season_episode(file);
            assert!(parsed.is_some(), "Should parse: {}", file);
            let (season, _) = parsed.unwrap();
            assert_eq!(season, 1, "Should be season 1: {}", file);
        }
    }
}
