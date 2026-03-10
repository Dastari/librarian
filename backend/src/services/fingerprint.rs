//! Audio fingerprinting service
//!
//! Generates audio fingerprints using Chromaprint/fpcalc for music identification.
//! These fingerprints can be used to:
//! - Identify unknown tracks via AcoustID
//! - Detect duplicates in a library
//! - Match tracks across different file formats/encodings
//!
//! ## Requirements
//!
//! The `fpcalc` command-line tool must be installed and available in PATH.
//! On most systems: `apt install libchromaprint-tools` or `brew install chromaprint`
//!
//! Alternatively, FFmpeg can be compiled with chromaprint support to use
//! `ffmpeg -f chromaprint` for fingerprint generation.

use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::Database;

/// Audio fingerprint result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFingerprint {
    /// The raw Chromaprint fingerprint (base64 encoded)
    pub fingerprint: String,
    /// Duration in seconds used for fingerprinting
    pub duration: f64,
}

/// AcoustID lookup result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcoustIdMatch {
    /// AcoustID recording ID
    pub acoustid_id: String,
    /// Match confidence (0.0 - 1.0)
    pub score: f64,
    /// MusicBrainz recording IDs if available
    pub musicbrainz_recording_ids: Vec<String>,
    /// Title if available from recordings
    pub title: Option<String>,
    /// Artist if available from recordings
    pub artist: Option<String>,
}

/// Fingerprint service for audio content identification
pub struct FingerprintService {
    /// Path to fpcalc executable
    fpcalc_path: String,
    /// AcoustID API key (optional, for lookups)
    acoustid_api_key: Option<String>,
    /// Database for storing results
    db: Database,
}

impl FingerprintService {
    /// Create a new fingerprint service
    pub fn new(db: Database) -> Self {
        Self {
            fpcalc_path: "fpcalc".to_string(),
            acoustid_api_key: None,
            db,
        }
    }

    /// Create with custom fpcalc path and AcoustID API key
    pub fn with_config(
        db: Database,
        fpcalc_path: Option<String>,
        acoustid_api_key: Option<String>,
    ) -> Self {
        Self {
            fpcalc_path: fpcalc_path.unwrap_or_else(|| "fpcalc".to_string()),
            acoustid_api_key,
            db,
        }
    }

    /// Check if fpcalc is available
    pub async fn is_available(&self) -> bool {
        Command::new(&self.fpcalc_path)
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Generate an audio fingerprint for a file
    ///
    /// Uses fpcalc (Chromaprint) to generate a fingerprint from the audio content.
    /// The fingerprint is content-based, so identical audio will produce identical
    /// fingerprints regardless of file format, bitrate, or metadata.
    pub async fn generate_fingerprint<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<AudioFingerprint> {
        let path = path.as_ref();
        
        debug!(path = %path.display(), "Generating audio fingerprint");

        // Run fpcalc with JSON output
        let output = Command::new(&self.fpcalc_path)
            .arg("-json")
            .arg("-length")
            .arg("120") // Use up to 120 seconds for fingerprinting
            .arg(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run fpcalc")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("fpcalc failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse JSON output
        let result: FpcalcOutput = serde_json::from_str(&stdout)
            .context("Failed to parse fpcalc JSON output")?;

        debug!(
            path = %path.display(),
            duration = result.duration,
            fingerprint_len = result.fingerprint.len(),
            "Fingerprint generated"
        );

        Ok(AudioFingerprint {
            fingerprint: result.fingerprint,
            duration: result.duration,
        })
    }

    /// Generate fingerprint and store in database
    pub async fn fingerprint_and_store(
        &self,
        media_file_id: Uuid,
        path: &Path,
    ) -> Result<AudioFingerprint> {
        let fingerprint = self.generate_fingerprint(path).await?;

        // Store in database
        self.db
            .media_files()
            .update_fingerprint(media_file_id, &fingerprint.fingerprint, None, None)
            .await?;

        info!(
            media_file_id = %media_file_id,
            path = %path.display(),
            "Fingerprint stored"
        );

        Ok(fingerprint)
    }

    /// Look up a fingerprint in AcoustID database
    ///
    /// Requires an AcoustID API key to be configured.
    /// Returns potential matches with MusicBrainz recording IDs.
    pub async fn lookup_acoustid(
        &self,
        fingerprint: &AudioFingerprint,
    ) -> Result<Vec<AcoustIdMatch>> {
        let api_key = self
            .acoustid_api_key
            .as_ref()
            .context("AcoustID API key not configured")?;

        let client = reqwest::Client::new();

        let response = client
            .post("https://api.acoustid.org/v2/lookup")
            .form(&[
                ("client", api_key.as_str()),
                ("fingerprint", &fingerprint.fingerprint),
                ("duration", &fingerprint.duration.to_string()),
                ("meta", "recordings releasegroups"),
            ])
            .send()
            .await
            .context("Failed to send AcoustID lookup request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("AcoustID API error {}: {}", status, body);
        }

        let result: AcoustIdResponse = response
            .json()
            .await
            .context("Failed to parse AcoustID response")?;

        if result.status != "ok" {
            anyhow::bail!(
                "AcoustID API error: {}",
                result.error.as_ref().map(|e| e.message.as_str()).unwrap_or("unknown")
            );
        }

        let matches: Vec<AcoustIdMatch> = result
            .results
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| {
                let recordings = r.recordings.unwrap_or_default();
                let first_recording = recordings.first();
                
                Some(AcoustIdMatch {
                    acoustid_id: r.id,
                    score: r.score,
                    musicbrainz_recording_ids: recordings.iter().map(|r| r.id.clone()).collect(),
                    title: first_recording.map(|r| r.title.clone()),
                    artist: first_recording.and_then(|r| {
                        r.artists.as_ref()?.first().map(|a| a.name.clone())
                    }),
                })
            })
            .collect();

        Ok(matches)
    }

    /// Fingerprint a file and look it up in AcoustID
    pub async fn identify_track(
        &self,
        media_file_id: Uuid,
        path: &Path,
    ) -> Result<Option<AcoustIdMatch>> {
        // Generate fingerprint
        let fingerprint = self.generate_fingerprint(path).await?;

        // Look up in AcoustID
        let matches = match self.lookup_acoustid(&fingerprint).await {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "AcoustID lookup failed");
                // Still store the fingerprint even if lookup fails
                self.db
                    .media_files()
                    .update_fingerprint(media_file_id, &fingerprint.fingerprint, None, None)
                    .await?;
                return Ok(None);
            }
        };

        // Get best match (highest score above threshold)
        let best_match = matches.into_iter().find(|m| m.score >= 0.8);

        // Store fingerprint and match data
        if let Some(ref m) = best_match {
            let mb_id = m.musicbrainz_recording_ids.first().map(|s| s.as_str());
            self.db
                .media_files()
                .update_fingerprint(
                    media_file_id,
                    &fingerprint.fingerprint,
                    Some(&m.acoustid_id),
                    mb_id,
                )
                .await?;
            
            info!(
                media_file_id = %media_file_id,
                acoustid_id = %m.acoustid_id,
                score = m.score,
                title = ?m.title,
                artist = ?m.artist,
                "Track identified via AcoustID"
            );
        } else {
            // Store fingerprint without match
            self.db
                .media_files()
                .update_fingerprint(media_file_id, &fingerprint.fingerprint, None, None)
                .await?;
        }

        Ok(best_match)
    }

    /// Find potential duplicates in a library by comparing fingerprints
    ///
    /// Returns pairs of media file IDs that have identical or very similar fingerprints.
    pub async fn find_duplicates(&self, library_id: Uuid) -> Result<Vec<(Uuid, Uuid)>> {
        // Get all files with fingerprints in the library
        let files = self.db.media_files().list_by_library(library_id).await?;
        
        let with_fingerprints: Vec<_> = files
            .iter()
            .filter_map(|f| f.audio_fingerprint.as_ref().map(|fp| (f.id, fp.clone())))
            .collect();

        let mut duplicates = Vec::new();

        // Simple comparison - in production you'd want a more efficient algorithm
        // (e.g., locality-sensitive hashing, or storing fingerprint hashes)
        for (i, (id1, fp1)) in with_fingerprints.iter().enumerate() {
            for (id2, fp2) in with_fingerprints.iter().skip(i + 1) {
                // Exact match check (could add fuzzy matching for near-duplicates)
                if fp1 == fp2 {
                    duplicates.push((*id1, *id2));
                }
            }
        }

        Ok(duplicates)
    }
}

// =============================================================================
// Internal types for JSON parsing
// =============================================================================

#[derive(Deserialize)]
struct FpcalcOutput {
    duration: f64,
    fingerprint: String,
}

#[derive(Deserialize)]
struct AcoustIdResponse {
    status: String,
    error: Option<AcoustIdError>,
    results: Option<Vec<AcoustIdResult>>,
}

#[derive(Deserialize)]
struct AcoustIdError {
    message: String,
}

#[derive(Deserialize)]
struct AcoustIdResult {
    id: String,
    score: f64,
    recordings: Option<Vec<AcoustIdRecording>>,
}

#[derive(Deserialize)]
struct AcoustIdRecording {
    id: String,
    title: String,
    artists: Option<Vec<AcoustIdArtist>>,
}

#[derive(Deserialize)]
struct AcoustIdArtist {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fpcalc_availability() {
        // This test checks if fpcalc is installed on the system
        // It's expected to fail on systems without chromaprint-tools
        let service = FingerprintService::new(
            // We'd need a mock DB here for a real test
            unsafe { std::mem::zeroed() }
        );
        
        // Just check if the command exists without panicking
        let _ = service.is_available().await;
    }
}
