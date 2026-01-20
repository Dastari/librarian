//! Archive extraction service
//!
//! Handles extraction of RAR, ZIP, and 7z archives from downloaded torrents.
//! Uses command-line tools (unrar, unzip, 7z) via spawn_blocking to avoid
//! blocking the async runtime.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::Command;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Archive file extensions we recognize
const RAR_EXTENSIONS: &[&str] = &["rar"];
const ZIP_EXTENSIONS: &[&str] = &["zip"];
const SEVEN_Z_EXTENSIONS: &[&str] = &["7z"];

/// Archive extraction service
pub struct ExtractorService {
    /// Directory for temporary extracted files
    temp_dir: PathBuf,
}

impl ExtractorService {
    /// Create a new extractor service
    pub fn new(temp_dir: PathBuf) -> Self {
        Self { temp_dir }
    }

    /// Check if a directory contains archives that need extraction
    pub fn needs_extraction(path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        if path.is_file() {
            return Self::is_archive(path);
        }

        // Walk directory looking for archives
        for entry in WalkDir::new(path)
            .max_depth(3) // Don't go too deep
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() && Self::is_archive(entry.path()) {
                return true;
            }
        }

        false
    }

    /// Check if a file is an archive
    fn is_archive(path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        match ext.as_deref() {
            Some(e) if RAR_EXTENSIONS.contains(&e) => true,
            Some(e) if ZIP_EXTENSIONS.contains(&e) => true,
            Some(e) if SEVEN_Z_EXTENSIONS.contains(&e) => true,
            _ => false,
        }
    }

    /// Get the archive type from extension
    fn get_archive_type(path: &Path) -> Option<ArchiveType> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        match ext.as_deref() {
            Some(e) if RAR_EXTENSIONS.contains(&e) => Some(ArchiveType::Rar),
            Some(e) if ZIP_EXTENSIONS.contains(&e) => Some(ArchiveType::Zip),
            Some(e) if SEVEN_Z_EXTENSIONS.contains(&e) => Some(ArchiveType::SevenZ),
            _ => None,
        }
    }

    /// Extract all archives in a directory to a temp location
    /// Returns the path to the extracted content
    pub async fn extract_archives(&self, source_path: &Path) -> Result<PathBuf> {
        // Create a unique temp directory for this extraction
        let extract_id = uuid::Uuid::new_v4();
        let extract_dir = self.temp_dir.join(format!("extract_{}", extract_id));
        tokio::fs::create_dir_all(&extract_dir)
            .await
            .context("Failed to create extraction directory")?;

        info!(
            source = %source_path.display(),
            destination = %extract_dir.display(),
            "Starting archive extraction"
        );

        // Find all archives
        let archives = self.find_archives(source_path)?;

        if archives.is_empty() {
            // No archives found, just return source path
            debug!("No archives found, returning source path");
            // Clean up the empty extract dir
            let _ = tokio::fs::remove_dir(&extract_dir).await;
            return Ok(source_path.to_path_buf());
        }

        // Extract each archive
        for archive in &archives {
            self.extract_single(archive, &extract_dir).await?;
        }

        info!(
            archive_count = archives.len(),
            destination = %extract_dir.display(),
            "Archive extraction complete"
        );

        Ok(extract_dir)
    }

    /// Find all archives in a path
    fn find_archives(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut archives = Vec::new();

        if path.is_file() {
            if Self::is_archive(path) {
                archives.push(path.to_path_buf());
            }
        } else {
            for entry in WalkDir::new(path)
                .max_depth(3)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() && Self::is_archive(entry.path()) {
                    // Skip .r00, .r01, etc. (RAR split volumes) - main .rar handles them
                    let ext = entry.path().extension().and_then(|e| e.to_str());
                    if let Some(e) = ext {
                        if e.starts_with('r') && e.len() == 3 && e[1..].parse::<u32>().is_ok() {
                            continue;
                        }
                    }
                    archives.push(entry.path().to_path_buf());
                }
            }
        }

        Ok(archives)
    }

    /// Extract a single archive
    async fn extract_single(&self, archive: &Path, dest_dir: &Path) -> Result<()> {
        let archive_type = Self::get_archive_type(archive).context("Unknown archive type")?;

        info!(
            archive = %archive.display(),
            archive_type = ?archive_type,
            destination = %dest_dir.display(),
            "Extracting archive"
        );

        match archive_type {
            ArchiveType::Rar => self.extract_rar(archive, dest_dir).await,
            ArchiveType::Zip => self.extract_zip(archive, dest_dir).await,
            ArchiveType::SevenZ => self.extract_7z(archive, dest_dir).await,
        }
    }

    /// Extract RAR archive using unrar
    async fn extract_rar(&self, archive: &Path, dest_dir: &Path) -> Result<()> {
        let output = Command::new("unrar")
            .arg("x") // Extract with full paths
            .arg("-o+") // Overwrite existing files
            .arg("-y") // Assume yes on all queries
            .arg(archive)
            .arg(dest_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run unrar. Is unrar installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("unrar failed: {}", stderr);
        }

        debug!(
            archive = %archive.display(),
            "RAR extraction successful"
        );

        Ok(())
    }

    /// Extract ZIP archive using unzip
    async fn extract_zip(&self, archive: &Path, dest_dir: &Path) -> Result<()> {
        let output = Command::new("unzip")
            .arg("-o") // Overwrite existing files
            .arg("-q") // Quiet mode
            .arg(archive)
            .arg("-d")
            .arg(dest_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run unzip. Is unzip installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("unzip failed: {}", stderr);
        }

        debug!(
            archive = %archive.display(),
            "ZIP extraction successful"
        );

        Ok(())
    }

    /// Extract 7z archive using 7z
    async fn extract_7z(&self, archive: &Path, dest_dir: &Path) -> Result<()> {
        let output = Command::new("7z")
            .arg("x") // Extract with full paths
            .arg("-y") // Assume yes on all queries
            .arg(format!("-o{}", dest_dir.display()))
            .arg(archive)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run 7z. Is p7zip-full installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("7z failed: {}", stderr);
        }

        debug!(
            archive = %archive.display(),
            "7z extraction successful"
        );

        Ok(())
    }

    /// Cleanup extracted temp files
    pub async fn cleanup(&self, temp_path: &Path) -> Result<()> {
        // Only delete if it's in our temp directory
        if !temp_path.starts_with(&self.temp_dir) {
            warn!(
                path = %temp_path.display(),
                temp_dir = %self.temp_dir.display(),
                "Refusing to cleanup path outside temp directory"
            );
            return Ok(());
        }

        if temp_path.exists() {
            info!(path = %temp_path.display(), "Cleaning up extracted files");
            tokio::fs::remove_dir_all(temp_path)
                .await
                .context("Failed to cleanup extraction directory")?;
        }

        Ok(())
    }

    /// Cleanup all old extraction directories (for garbage collection)
    pub async fn cleanup_old_extractions(&self, max_age_hours: u64) -> Result<usize> {
        let mut cleaned = 0;

        if !self.temp_dir.exists() {
            return Ok(0);
        }

        let now = std::time::SystemTime::now();
        let max_age = std::time::Duration::from_secs(max_age_hours * 3600);

        let mut entries = tokio::fs::read_dir(&self.temp_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > max_age {
                            let path = entry.path();
                            info!(
                                path = %path.display(),
                                age_hours = age.as_secs() / 3600,
                                "Cleaning up old extraction directory"
                            );
                            if path.is_dir() {
                                let _ = tokio::fs::remove_dir_all(&path).await;
                            } else {
                                let _ = tokio::fs::remove_file(&path).await;
                            }
                            cleaned += 1;
                        }
                    }
                }
            }
        }

        Ok(cleaned)
    }
}

#[derive(Debug, Clone, Copy)]
enum ArchiveType {
    Rar,
    Zip,
    SevenZ,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_archive() {
        assert!(ExtractorService::is_archive(Path::new("file.rar")));
        assert!(ExtractorService::is_archive(Path::new("file.RAR")));
        assert!(ExtractorService::is_archive(Path::new("file.zip")));
        assert!(ExtractorService::is_archive(Path::new("file.7z")));
        assert!(!ExtractorService::is_archive(Path::new("file.mkv")));
        assert!(!ExtractorService::is_archive(Path::new("file.mp4")));
    }

    #[test]
    fn test_get_archive_type() {
        assert!(matches!(
            ExtractorService::get_archive_type(Path::new("file.rar")),
            Some(ArchiveType::Rar)
        ));
        assert!(matches!(
            ExtractorService::get_archive_type(Path::new("file.zip")),
            Some(ArchiveType::Zip)
        ));
        assert!(matches!(
            ExtractorService::get_archive_type(Path::new("file.7z")),
            Some(ArchiveType::SevenZ)
        ));
        assert!(ExtractorService::get_archive_type(Path::new("file.mkv")).is_none());
    }
}
