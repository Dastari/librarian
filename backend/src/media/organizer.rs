//! Media file organization and renaming
//!
//! NOTE: This is a legacy organizer module. The primary organizer is in `services/organizer.rs`.
//! This module is kept for potential future use with movies and direct file operations.

use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};

/// Media organizer for renaming and moving files (legacy - use services/organizer.rs)
#[allow(dead_code)]
pub struct MediaOrganizer {
    movies_path: PathBuf,
    tv_path: PathBuf,
}

#[allow(dead_code)]
impl MediaOrganizer {
    pub fn new(base_path: &Path) -> Self {
        Self {
            movies_path: base_path.join("Movies"),
            tv_path: base_path.join("TV"),
        }
    }

    /// Organize a movie file
    pub fn organize_movie(&self, title: &str, year: i32, extension: &str) -> PathBuf {
        let sanitized_title = sanitize_filename::sanitize(title);
        let folder_name = format!("{} ({})", sanitized_title, year);
        let file_name = format!("{} ({}){}", sanitized_title, year, extension);

        self.movies_path.join(&folder_name).join(file_name)
    }

    /// Organize a TV episode file
    pub fn organize_episode(
        &self,
        show_name: &str,
        season: i32,
        episode: i32,
        episode_title: Option<&str>,
        extension: &str,
    ) -> PathBuf {
        let sanitized_show = sanitize_filename::sanitize(show_name);
        let season_folder = format!("Season {:02}", season);

        let file_name = match episode_title {
            Some(title) => {
                let sanitized_title = sanitize_filename::sanitize(title);
                format!(
                    "{} - S{:02}E{:02} - {}{}",
                    sanitized_show, season, episode, sanitized_title, extension
                )
            }
            None => format!(
                "{} - S{:02}E{:02}{}",
                sanitized_show, season, episode, extension
            ),
        };

        self.tv_path
            .join(&sanitized_show)
            .join(&season_folder)
            .join(file_name)
    }

    /// Parse season and episode from filename
    pub fn parse_episode_info(filename: &str) -> Option<(i32, i32)> {
        // Common patterns: S01E01, 1x01, 101
        let patterns = [
            r"[Ss](\d{1,2})[Ee](\d{1,2})",
            r"(\d{1,2})[xX](\d{1,2})",
            r"[^0-9](\d{1,2})(\d{2})[^0-9]",
        ];

        for pattern in patterns {
            if let Ok(re) = Regex::new(pattern)
                && let Some(caps) = re.captures(filename) {
                    let season: i32 = caps.get(1)?.as_str().parse().ok()?;
                    let episode: i32 = caps.get(2)?.as_str().parse().ok()?;
                    return Some((season, episode));
                }
        }

        None
    }

    /// Move a file to its organized location
    pub async fn move_file(&self, source: &Path, destination: &Path) -> Result<()> {
        // Create parent directories
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Move the file
        tokio::fs::rename(source, destination).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_episode_sxxexx() {
        assert_eq!(MediaOrganizer::parse_episode_info("Show.S01E05.mkv"), Some((1, 5)));
        assert_eq!(MediaOrganizer::parse_episode_info("Show.S12E23.mkv"), Some((12, 23)));
    }

    #[test]
    fn test_parse_episode_xformat() {
        assert_eq!(MediaOrganizer::parse_episode_info("Show.1x05.mkv"), Some((1, 5)));
        assert_eq!(MediaOrganizer::parse_episode_info("Show.12X23.mkv"), Some((12, 23)));
    }
}
