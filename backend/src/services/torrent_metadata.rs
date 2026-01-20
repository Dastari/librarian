//! Torrent metadata parsing utilities
//!
//! This module provides functions to parse .torrent files and extract
//! file information without actually downloading the content.
//! Used for validating torrent contents before initiating downloads.

use anyhow::{Context, Result};
use std::path::PathBuf;

use super::file_utils::is_audio_file;

/// Information about a file within a torrent
#[derive(Debug, Clone)]
pub struct TorrentFileInfo {
    /// Relative path within the torrent
    pub path: PathBuf,
    /// File name (last component of path)
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// Index within the torrent's file list
    pub index: usize,
}

impl TorrentFileInfo {
    /// Check if this file is an audio file based on extension
    pub fn is_audio(&self) -> bool {
        is_audio_file(&self.name)
    }

    /// Get the file extension (lowercase, without dot)
    pub fn extension(&self) -> Option<String> {
        self.path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
    }
}

/// Parse a .torrent file and extract file information without downloading
///
/// # Arguments
/// * `torrent_bytes` - Raw bytes of the .torrent file
///
/// # Returns
/// A vector of `TorrentFileInfo` describing all files in the torrent
///
/// # Example
/// ```ignore
/// let torrent_bytes = std::fs::read("album.torrent")?;
/// let files = parse_torrent_files(&torrent_bytes)?;
/// for file in files {
///     println!("{}: {} bytes", file.name, file.size);
/// }
/// ```
pub fn parse_torrent_files(torrent_bytes: &[u8]) -> Result<Vec<TorrentFileInfo>> {
    // Parse using bencode decoder, then extract file info
    // We use a manual approach since librqbit's torrent_from_bytes has lifetime constraints
    parse_torrent_files_internal(torrent_bytes)
}

/// Internal implementation that parses the bencoded torrent
fn parse_torrent_files_internal(torrent_bytes: &[u8]) -> Result<Vec<TorrentFileInfo>> {
    use serde::Deserialize;
    
    // Minimal bencode structures for extracting file info
    #[derive(Debug, Deserialize)]
    struct TorrentFile {
        length: u64,
        #[serde(default)]
        path: Vec<String>,
    }
    
    #[derive(Debug, Deserialize)]
    struct TorrentInfo {
        name: String,
        #[serde(default)]
        length: Option<u64>,
        #[serde(default)]
        files: Option<Vec<TorrentFile>>,
    }
    
    #[derive(Debug, Deserialize)]
    struct Torrent {
        info: TorrentInfo,
    }
    
    // Parse the bencoded torrent file
    let torrent: Torrent = serde_bencode::from_bytes(torrent_bytes)
        .context("Failed to parse torrent file")?;

    let mut files = Vec::new();

    // Handle multi-file vs single-file torrents
    if let Some(ref torrent_files) = torrent.info.files {
        // Multi-file torrent
        for (idx, file) in torrent_files.iter().enumerate() {
            // Build path from path components
            let path: PathBuf = file.path.iter().collect();
            let name = path
                .file_name()
                .map(|n: &std::ffi::OsStr| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            files.push(TorrentFileInfo {
                path,
                name,
                size: file.length,
                index: idx,
            });
        }
    } else if let Some(length) = torrent.info.length {
        // Single-file torrent
        let path = PathBuf::from(&torrent.info.name);
        let name = torrent.info.name.clone();
        
        files.push(TorrentFileInfo {
            path,
            name,
            size: length,
            index: 0,
        });
    }

    Ok(files)
}

/// Extract only audio files from a list of torrent files
///
/// Filters out non-audio files like cover art (.jpg, .png), NFO files,
/// cue sheets, and other metadata files.
///
/// # Arguments
/// * `files` - All files from the torrent
///
/// # Returns
/// References to only the audio files
pub fn extract_audio_files(files: &[TorrentFileInfo]) -> Vec<&TorrentFileInfo> {
    files.iter().filter(|f| f.is_audio()).collect()
}

/// Check if a torrent appears to be a single-file album
///
/// Some albums are distributed as a single .flac or .ape file that needs
/// to be split using a .cue file. These are harder to match against
/// individual tracks.
///
/// # Arguments
/// * `files` - All files from the torrent
///
/// # Returns
/// `true` if the torrent has only one audio file (likely needs splitting)
pub fn is_single_file_album(files: &[TorrentFileInfo]) -> bool {
    let audio_files = extract_audio_files(files);
    audio_files.len() == 1
}

/// Get a summary of the torrent's audio content
///
/// # Arguments
/// * `files` - All files from the torrent
///
/// # Returns
/// A tuple of (audio_file_count, total_file_count, total_audio_size_bytes)
pub fn audio_summary(files: &[TorrentFileInfo]) -> (usize, usize, u64) {
    let audio_files = extract_audio_files(files);
    let audio_size: u64 = audio_files.iter().map(|f| f.size).sum();
    (audio_files.len(), files.len(), audio_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_audio_detection() {
        let audio_file = TorrentFileInfo {
            path: PathBuf::from("01 - Track One.flac"),
            name: "01 - Track One.flac".to_string(),
            size: 50_000_000,
            index: 0,
        };
        assert!(audio_file.is_audio());

        let cover_file = TorrentFileInfo {
            path: PathBuf::from("cover.jpg"),
            name: "cover.jpg".to_string(),
            size: 500_000,
            index: 1,
        };
        assert!(!cover_file.is_audio());

        let nfo_file = TorrentFileInfo {
            path: PathBuf::from("album.nfo"),
            name: "album.nfo".to_string(),
            size: 1000,
            index: 2,
        };
        assert!(!nfo_file.is_audio());
    }

    #[test]
    fn test_extract_audio_files() {
        let files = vec![
            TorrentFileInfo {
                path: PathBuf::from("01 - Track One.flac"),
                name: "01 - Track One.flac".to_string(),
                size: 50_000_000,
                index: 0,
            },
            TorrentFileInfo {
                path: PathBuf::from("cover.jpg"),
                name: "cover.jpg".to_string(),
                size: 500_000,
                index: 1,
            },
            TorrentFileInfo {
                path: PathBuf::from("02 - Track Two.flac"),
                name: "02 - Track Two.flac".to_string(),
                size: 45_000_000,
                index: 2,
            },
        ];

        let audio = extract_audio_files(&files);
        assert_eq!(audio.len(), 2);
        assert_eq!(audio[0].name, "01 - Track One.flac");
        assert_eq!(audio[1].name, "02 - Track Two.flac");
    }

    #[test]
    fn test_single_file_album_detection() {
        // Single audio file (likely needs splitting)
        let single_file = vec![
            TorrentFileInfo {
                path: PathBuf::from("Album.flac"),
                name: "Album.flac".to_string(),
                size: 500_000_000,
                index: 0,
            },
            TorrentFileInfo {
                path: PathBuf::from("Album.cue"),
                name: "Album.cue".to_string(),
                size: 2000,
                index: 1,
            },
        ];
        assert!(is_single_file_album(&single_file));

        // Multiple audio files (normal album)
        let multi_file = vec![
            TorrentFileInfo {
                path: PathBuf::from("01 - Track.flac"),
                name: "01 - Track.flac".to_string(),
                size: 50_000_000,
                index: 0,
            },
            TorrentFileInfo {
                path: PathBuf::from("02 - Track.flac"),
                name: "02 - Track.flac".to_string(),
                size: 45_000_000,
                index: 1,
            },
        ];
        assert!(!is_single_file_album(&multi_file));
    }

    #[test]
    fn test_audio_summary() {
        let files = vec![
            TorrentFileInfo {
                path: PathBuf::from("01 - Track.flac"),
                name: "01 - Track.flac".to_string(),
                size: 50_000_000,
                index: 0,
            },
            TorrentFileInfo {
                path: PathBuf::from("cover.jpg"),
                name: "cover.jpg".to_string(),
                size: 500_000,
                index: 1,
            },
            TorrentFileInfo {
                path: PathBuf::from("02 - Track.flac"),
                name: "02 - Track.flac".to_string(),
                size: 45_000_000,
                index: 2,
            },
        ];

        let (audio_count, total_count, audio_size) = audio_summary(&files);
        assert_eq!(audio_count, 2);
        assert_eq!(total_count, 3);
        assert_eq!(audio_size, 95_000_000);
    }
}
