//! Shared file utility functions
//!
//! Common utilities for working with files across the codebase.
//! Centralizes file extension checks, formatting, and sanitization.

use sanitize_filename;

/// Video file extensions (lowercase)
// TODO: Consider using `file` command or `ffprobe` to determine file type
// more reliably instead of relying solely on extensions
pub const VIDEO_EXTENSIONS: &[&str] = &[
    ".mkv", ".mp4", ".avi", ".mov", ".wmv", ".flv", ".webm", ".m4v", ".ts", ".m2ts", ".mpg",
    ".mpeg",
];

/// Audio file extensions (lowercase)
pub const AUDIO_EXTENSIONS: &[&str] = &[
    ".mp3", ".flac", ".m4a", ".m4b", ".aac", ".ogg", ".opus", ".wav", ".wma", ".aiff", ".alac",
];

/// Archive file extensions (lowercase)
pub const ARCHIVE_EXTENSIONS: &[&str] = &[".rar", ".zip", ".7z", ".tar", ".gz", ".bz2"];

/// Subtitle file extensions (lowercase)
pub const SUBTITLE_EXTENSIONS: &[&str] = &[".srt", ".ass", ".ssa", ".sub", ".idx", ".vtt"];

/// Check if a file is a video file based on extension
///
/// # Arguments
/// * `path` - File path or filename to check
///
/// # Returns
/// `true` if the file has a video extension
///
/// # Example
/// ```
/// use librarian_backend::services::file_utils::is_video_file;
/// assert!(is_video_file("movie.mkv"));
/// assert!(is_video_file("/path/to/video.mp4"));
/// assert!(!is_video_file("music.mp3"));
/// ```
pub fn is_video_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    VIDEO_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Check if a file is an audio file based on extension
///
/// # Arguments
/// * `path` - File path or filename to check
///
/// # Returns
/// `true` if the file has an audio extension
///
/// # Example
/// ```
/// use librarian_backend::services::file_utils::is_audio_file;
/// assert!(is_audio_file("song.mp3"));
/// assert!(is_audio_file("/path/to/audiobook.m4b"));
/// assert!(!is_audio_file("video.mkv"));
/// ```
pub fn is_audio_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    AUDIO_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Check if a file is an archive based on extension
///
/// # Arguments
/// * `path` - File path or filename to check
///
/// # Returns
/// `true` if the file has an archive extension
pub fn is_archive_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    ARCHIVE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Check if a file is a subtitle file based on extension
///
/// # Arguments
/// * `path` - File path or filename to check
///
/// # Returns
/// `true` if the file has a subtitle extension
pub fn is_subtitle_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    SUBTITLE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Get the container format from a file's extension
///
/// # Arguments
/// * `path` - File path to extract extension from
///
/// # Returns
/// The lowercase extension without the dot, or None if no extension
///
/// # Example
/// ```
/// use librarian_backend::services::file_utils::get_container;
/// assert_eq!(get_container("video.mkv"), Some("mkv".to_string()));
/// assert_eq!(get_container("no_extension"), None);
/// ```
pub fn get_container(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
}

/// Sanitize a string for use as a filename
///
/// Uses the `sanitize_filename` crate which handles:
/// - Invalid characters for the current OS
/// - Reserved filenames (CON, PRN, etc. on Windows)
/// - Leading/trailing spaces and dots
///
/// # Arguments
/// * `name` - The string to sanitize
///
/// # Returns
/// A sanitized string safe to use as a filename
pub fn sanitize_for_filename(name: &str) -> String {
    sanitize_filename::sanitize(name)
}

/// Format bytes into a human-readable string
///
/// # Arguments
/// * `bytes` - Number of bytes
///
/// # Returns
/// Formatted string like "1.5 GB" or "256.0 MB"
///
/// # Example
/// ```
/// use librarian_backend::services::file_utils::format_bytes;
/// assert_eq!(format_bytes(1024), "1.0 KB");
/// assert_eq!(format_bytes(1073741824), "1.0 GB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format bytes (as i64) into a human-readable string
///
/// Wrapper for `format_bytes` that handles negative values
pub fn format_bytes_i64(bytes: i64) -> String {
    if bytes < 0 {
        format!("-{}", format_bytes((-bytes) as u64))
    } else {
        format_bytes(bytes as u64)
    }
}

/// Normalize paths for display (strip Windows verbatim prefixes).
pub fn normalize_display_path(path: &str) -> String {
    #[cfg(windows)]
    {
        if let Some(stripped) = path.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{}", stripped);
        }
        if let Some(stripped) = path.strip_prefix(r"\\?\") {
            return stripped.to_string();
        }
    }

    path.to_string()
}

/// Normalize a Path for display (strip Windows verbatim prefixes).
pub fn normalize_display_path_buf(path: &std::path::Path) -> String {
    normalize_display_path(&path.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_video_file() {
        assert!(is_video_file("movie.mkv"));
        assert!(is_video_file("MOVIE.MKV"));
        assert!(is_video_file("/path/to/video.mp4"));
        assert!(is_video_file("show.S01E01.1080p.ts"));
        assert!(!is_video_file("music.mp3"));
        assert!(!is_video_file("document.pdf"));
        assert!(!is_video_file("no_extension"));
    }

    #[test]
    fn test_is_audio_file() {
        assert!(is_audio_file("song.mp3"));
        assert!(is_audio_file("SONG.FLAC"));
        assert!(is_audio_file("/path/to/audiobook.m4b"));
        assert!(!is_audio_file("video.mkv"));
        assert!(!is_audio_file("document.pdf"));
    }

    #[test]
    fn test_is_archive_file() {
        assert!(is_archive_file("archive.rar"));
        assert!(is_archive_file("file.zip"));
        assert!(!is_archive_file("video.mkv"));
    }

    #[test]
    fn test_get_container() {
        assert_eq!(get_container("video.mkv"), Some("mkv".to_string()));
        assert_eq!(get_container("VIDEO.MKV"), Some("mkv".to_string()));
        assert_eq!(get_container("no_extension"), None);
    }

    #[test]
    fn test_sanitize_for_filename() {
        // Basic sanitization
        let result = sanitize_for_filename("Show: The Movie");
        assert!(!result.contains(':'));

        // Should handle slashes
        let result = sanitize_for_filename("path/to/file");
        assert!(!result.contains('/'));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
        assert_eq!(format_bytes(1099511627776), "1.0 TB");
    }
}
