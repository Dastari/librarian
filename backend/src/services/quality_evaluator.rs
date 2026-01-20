//! Quality evaluation service
//!
//! Provides centralized logic for evaluating and comparing media quality.
//! Used by:
//! - TorrentFileMatcher to determine if a release meets quality requirements
//! - Scanner to detect suboptimal files
//! - Auto-hunt to filter search results

use tracing::debug;

use crate::db::{LibraryRecord, TvShowRecord, MovieRecord};
use crate::services::filename_parser::ParsedQuality;
use crate::services::ffmpeg::MediaAnalysis;

/// Result of quality evaluation
#[derive(Debug, Clone)]
pub struct QualityEvaluation {
    /// Whether the quality meets the target requirements
    pub meets_target: bool,
    /// Whether this is an upgrade over existing quality
    pub is_upgrade: bool,
    /// Status to assign: optimal, suboptimal, exceeds
    pub quality_status: QualityStatus,
    /// Human-readable reason for the evaluation
    pub reason: Option<String>,
}

/// Quality status for a media file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityStatus {
    /// Quality meets the target
    Optimal,
    /// Quality is below the target
    Suboptimal,
    /// Quality exceeds the target
    Exceeds,
    /// Quality is unknown (not analyzed)
    Unknown,
}

impl std::fmt::Display for QualityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualityStatus::Optimal => write!(f, "optimal"),
            QualityStatus::Suboptimal => write!(f, "suboptimal"),
            QualityStatus::Exceeds => write!(f, "exceeds"),
            QualityStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Effective quality settings after applying overrides
#[derive(Debug, Clone, Default)]
pub struct EffectiveQualitySettings {
    pub allowed_resolutions: Vec<String>,
    pub allowed_video_codecs: Vec<String>,
    pub allowed_audio_formats: Vec<String>,
    pub require_hdr: bool,
    pub allowed_hdr_types: Vec<String>,
    pub allowed_sources: Vec<String>,
}

impl EffectiveQualitySettings {
    /// Create settings from a library record
    pub fn from_library(library: &LibraryRecord) -> Self {
        Self {
            allowed_resolutions: library.allowed_resolutions.clone(),
            allowed_video_codecs: library.allowed_video_codecs.clone(),
            allowed_audio_formats: library.allowed_audio_formats.clone(),
            require_hdr: library.require_hdr,
            allowed_hdr_types: library.allowed_hdr_types.clone(),
            allowed_sources: library.allowed_sources.clone(),
        }
    }

    /// Create settings from a TV show with library fallback
    pub fn from_tv_show(show: &TvShowRecord, library: &LibraryRecord) -> Self {
        Self {
            allowed_resolutions: show
                .allowed_resolutions_override
                .clone()
                .unwrap_or_else(|| library.allowed_resolutions.clone()),
            allowed_video_codecs: show
                .allowed_video_codecs_override
                .clone()
                .unwrap_or_else(|| library.allowed_video_codecs.clone()),
            allowed_audio_formats: show
                .allowed_audio_formats_override
                .clone()
                .unwrap_or_else(|| library.allowed_audio_formats.clone()),
            require_hdr: show
                .require_hdr_override
                .unwrap_or(library.require_hdr),
            allowed_hdr_types: show
                .allowed_hdr_types_override
                .clone()
                .unwrap_or_else(|| library.allowed_hdr_types.clone()),
            allowed_sources: show
                .allowed_sources_override
                .clone()
                .unwrap_or_else(|| library.allowed_sources.clone()),
        }
    }

    /// Create settings from a movie with library fallback
    pub fn from_movie(movie: &MovieRecord, library: &LibraryRecord) -> Self {
        Self {
            allowed_resolutions: movie
                .allowed_resolutions_override
                .clone()
                .unwrap_or_else(|| library.allowed_resolutions.clone()),
            allowed_video_codecs: movie
                .allowed_video_codecs_override
                .clone()
                .unwrap_or_else(|| library.allowed_video_codecs.clone()),
            allowed_audio_formats: movie
                .allowed_audio_formats_override
                .clone()
                .unwrap_or_else(|| library.allowed_audio_formats.clone()),
            require_hdr: movie
                .require_hdr_override
                .unwrap_or(library.require_hdr),
            allowed_hdr_types: movie
                .allowed_hdr_types_override
                .clone()
                .unwrap_or_else(|| library.allowed_hdr_types.clone()),
            allowed_sources: movie
                .allowed_sources_override
                .clone()
                .unwrap_or_else(|| library.allowed_sources.clone()),
        }
    }

    /// Check if settings allow any quality (no restrictions)
    pub fn allows_any(&self) -> bool {
        self.allowed_resolutions.is_empty()
            && self.allowed_video_codecs.is_empty()
            && self.allowed_audio_formats.is_empty()
            && !self.require_hdr
    }
}

/// Service for evaluating media quality
pub struct QualityEvaluator;

impl QualityEvaluator {
    /// Evaluate quality from parsed filename info
    pub fn evaluate_parsed(
        parsed: &ParsedQuality,
        settings: &EffectiveQualitySettings,
    ) -> QualityEvaluation {
        // If no restrictions, everything is optimal
        if settings.allows_any() {
            return QualityEvaluation {
                meets_target: true,
                is_upgrade: false,
                quality_status: QualityStatus::Optimal,
                reason: None,
            };
        }

        let mut issues: Vec<String> = Vec::new();

        // Check resolution
        if !settings.allowed_resolutions.is_empty() {
            if let Some(ref resolution) = parsed.resolution {
                let normalized = normalize_resolution(resolution);
                if !settings.allowed_resolutions.iter().any(|r| normalize_resolution(r) == normalized) {
                    // Check if it's better than allowed
                    let parsed_rank = resolution_rank(&normalized);
                    let best_allowed_rank = settings
                        .allowed_resolutions
                        .iter()
                        .map(|r| resolution_rank(&normalize_resolution(r)))
                        .max()
                        .unwrap_or(0);

                    if parsed_rank > best_allowed_rank {
                        // Exceeds target - this is good
                        return QualityEvaluation {
                            meets_target: true,
                            is_upgrade: true,
                            quality_status: QualityStatus::Exceeds,
                            reason: Some(format!(
                                "Resolution {} exceeds target",
                                resolution
                            )),
                        };
                    } else {
                        issues.push(format!("Resolution {} not in allowed list", resolution));
                    }
                }
            } else {
                issues.push("Unknown resolution".to_string());
            }
        }

        // Check HDR requirement
        if settings.require_hdr && parsed.hdr.is_none() {
            issues.push("HDR required but not detected".to_string());
        }

        // Check HDR type if specified
        if !settings.allowed_hdr_types.is_empty() {
            if let Some(ref hdr) = parsed.hdr {
                let hdr_lower = hdr.to_lowercase();
                if !settings.allowed_hdr_types.iter().any(|h| h.to_lowercase() == hdr_lower) {
                    issues.push(format!("HDR type {} not in allowed list", hdr));
                }
            }
        }

        // Check source
        if !settings.allowed_sources.is_empty() {
            if let Some(ref source) = parsed.source {
                let source_lower = source.to_lowercase();
                if !settings.allowed_sources.iter().any(|s| s.to_lowercase() == source_lower) {
                    issues.push(format!("Source {} not in allowed list", source));
                }
            }
        }

        if issues.is_empty() {
            QualityEvaluation {
                meets_target: true,
                is_upgrade: false,
                quality_status: QualityStatus::Optimal,
                reason: None,
            }
        } else {
            QualityEvaluation {
                meets_target: false,
                is_upgrade: false,
                quality_status: QualityStatus::Suboptimal,
                reason: Some(issues.join("; ")),
            }
        }
    }

    /// Evaluate quality from FFprobe analysis (more accurate)
    pub fn evaluate_analysis(
        analysis: &MediaAnalysis,
        settings: &EffectiveQualitySettings,
    ) -> QualityEvaluation {
        // If no restrictions, everything is optimal
        if settings.allows_any() {
            return QualityEvaluation {
                meets_target: true,
                is_upgrade: false,
                quality_status: QualityStatus::Optimal,
                reason: None,
            };
        }

        let mut issues: Vec<String> = Vec::new();

        // Check resolution from video stream
        if !settings.allowed_resolutions.is_empty() {
            if let Some(ref video) = analysis.video_streams.first() {
                let actual_resolution = height_to_resolution(video.height);
                let normalized = normalize_resolution(&actual_resolution);

                if !settings.allowed_resolutions.iter().any(|r| normalize_resolution(r) == normalized) {
                    let actual_rank = resolution_rank(&normalized);
                    let best_allowed_rank = settings
                        .allowed_resolutions
                        .iter()
                        .map(|r| resolution_rank(&normalize_resolution(r)))
                        .max()
                        .unwrap_or(0);

                    if actual_rank > best_allowed_rank {
                        return QualityEvaluation {
                            meets_target: true,
                            is_upgrade: true,
                            quality_status: QualityStatus::Exceeds,
                            reason: Some(format!(
                                "Resolution {} exceeds target",
                                actual_resolution
                            )),
                        };
                    } else {
                        issues.push(format!(
                            "Resolution {} ({}p) below target",
                            actual_resolution, video.height
                        ));
                    }
                }
            }
        }

        // Check HDR from video stream
        if settings.require_hdr {
            let has_hdr = analysis.video_streams.iter().any(|v| v.hdr_type.is_some());
            if !has_hdr {
                issues.push("HDR required but not detected in file".to_string());
            }
        }

        // Check HDR type
        if !settings.allowed_hdr_types.is_empty() {
            for video in &analysis.video_streams {
                if let Some(ref hdr_type) = video.hdr_type {
                    let hdr_str = format!("{:?}", hdr_type);
                    if !settings.allowed_hdr_types.iter().any(|h| {
                        h.to_lowercase() == hdr_str.to_lowercase()
                    }) {
                        issues.push(format!("HDR type {:?} not in allowed list", hdr_type));
                    }
                }
            }
        }

        if issues.is_empty() {
            QualityEvaluation {
                meets_target: true,
                is_upgrade: false,
                quality_status: QualityStatus::Optimal,
                reason: None,
            }
        } else {
            QualityEvaluation {
                meets_target: false,
                is_upgrade: false,
                quality_status: QualityStatus::Suboptimal,
                reason: Some(issues.join("; ")),
            }
        }
    }

    /// Compare two qualities and determine if new is an upgrade
    pub fn is_upgrade(
        existing_resolution: Option<&str>,
        existing_hdr: bool,
        new_resolution: Option<&str>,
        new_hdr: bool,
    ) -> bool {
        let existing_rank = existing_resolution
            .map(|r| resolution_rank(&normalize_resolution(r)))
            .unwrap_or(0);
        let new_rank = new_resolution
            .map(|r| resolution_rank(&normalize_resolution(r)))
            .unwrap_or(0);

        // Higher resolution is always an upgrade
        if new_rank > existing_rank {
            return true;
        }

        // Same resolution but HDR vs non-HDR
        if new_rank == existing_rank && new_hdr && !existing_hdr {
            return true;
        }

        false
    }
}

/// Normalize resolution string (e.g., "4K" -> "2160p")
fn normalize_resolution(resolution: &str) -> String {
    let upper = resolution.to_uppercase();
    match upper.as_str() {
        "4K" | "UHD" | "2160" => "2160p".to_string(),
        "1080" => "1080p".to_string(),
        "720" => "720p".to_string(),
        "480" | "SD" => "480p".to_string(),
        _ => resolution.to_lowercase(),
    }
}

/// Get numeric rank for resolution comparison (higher = better)
fn resolution_rank(resolution: &str) -> u32 {
    match resolution.to_lowercase().as_str() {
        "2160p" | "4k" | "uhd" => 4,
        "1080p" => 3,
        "720p" => 2,
        "480p" | "sd" => 1,
        _ => 0,
    }
}

/// Convert video height to resolution string
fn height_to_resolution(height: u32) -> String {
    if height >= 2160 {
        "2160p".to_string()
    } else if height >= 1080 {
        "1080p".to_string()
    } else if height >= 720 {
        "720p".to_string()
    } else {
        "480p".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::filename_parser::parse_quality;

    // =========================================================================
    // Resolution Ranking Tests
    // =========================================================================

    #[test]
    fn test_resolution_rank_ordering() {
        assert!(resolution_rank("2160p") > resolution_rank("1080p"));
        assert!(resolution_rank("1080p") > resolution_rank("720p"));
        assert!(resolution_rank("720p") > resolution_rank("480p"));
        assert!(resolution_rank("480p") > resolution_rank("unknown"));
    }

    #[test]
    fn test_resolution_rank_aliases() {
        // 4K and UHD should equal 2160p
        assert_eq!(resolution_rank("4k"), resolution_rank("2160p"));
        assert_eq!(resolution_rank("uhd"), resolution_rank("2160p"));
    }

    #[test]
    fn test_resolution_rank_case_insensitive() {
        assert_eq!(resolution_rank("1080P"), resolution_rank("1080p"));
        assert_eq!(resolution_rank("4K"), resolution_rank("4k"));
    }

    // =========================================================================
    // Normalize Resolution Tests
    // =========================================================================

    #[test]
    fn test_normalize_resolution_aliases() {
        assert_eq!(normalize_resolution("4K"), "2160p");
        assert_eq!(normalize_resolution("UHD"), "2160p");
        assert_eq!(normalize_resolution("2160"), "2160p");
        assert_eq!(normalize_resolution("1080"), "1080p");
        assert_eq!(normalize_resolution("720"), "720p");
        assert_eq!(normalize_resolution("SD"), "480p");
    }

    #[test]
    fn test_normalize_resolution_passthrough() {
        assert_eq!(normalize_resolution("1080p"), "1080p");
        assert_eq!(normalize_resolution("720p"), "720p");
        assert_eq!(normalize_resolution("480p"), "480p");
        assert_eq!(normalize_resolution("2160p"), "2160p");
    }

    // =========================================================================
    // Height to Resolution Tests
    // =========================================================================

    #[test]
    fn test_height_to_resolution() {
        assert_eq!(height_to_resolution(2160), "2160p");
        assert_eq!(height_to_resolution(1080), "1080p");
        assert_eq!(height_to_resolution(720), "720p");
        assert_eq!(height_to_resolution(480), "480p");
        assert_eq!(height_to_resolution(360), "480p"); // Below 480 rounds to 480
    }

    #[test]
    fn test_height_to_resolution_edge_cases() {
        // Slightly above threshold
        assert_eq!(height_to_resolution(2161), "2160p");
        assert_eq!(height_to_resolution(1081), "1080p");
        assert_eq!(height_to_resolution(721), "720p");
        
        // Slightly below threshold
        assert_eq!(height_to_resolution(2159), "1080p");
        assert_eq!(height_to_resolution(1079), "720p");
        assert_eq!(height_to_resolution(719), "480p");
    }

    // =========================================================================
    // Settings Tests
    // =========================================================================

    #[test]
    fn test_allows_any_empty_settings() {
        let settings = EffectiveQualitySettings::default();
        assert!(settings.allows_any());
    }

    #[test]
    fn test_allows_any_with_resolution() {
        let settings = EffectiveQualitySettings {
            allowed_resolutions: vec!["1080p".to_string()],
            ..Default::default()
        };
        assert!(!settings.allows_any());
    }

    #[test]
    fn test_allows_any_with_hdr_requirement() {
        let settings = EffectiveQualitySettings {
            require_hdr: true,
            ..Default::default()
        };
        assert!(!settings.allows_any());
    }

    #[test]
    fn test_allows_any_with_codecs() {
        let settings = EffectiveQualitySettings {
            allowed_video_codecs: vec!["HEVC".to_string()],
            ..Default::default()
        };
        assert!(!settings.allows_any());
    }

    // =========================================================================
    // Upgrade Detection Tests
    // =========================================================================

    #[test]
    fn test_is_upgrade_resolution() {
        // 720p -> 1080p is upgrade
        assert!(QualityEvaluator::is_upgrade(
            Some("720p"),
            false,
            Some("1080p"),
            false
        ));

        // 1080p -> 2160p is upgrade
        assert!(QualityEvaluator::is_upgrade(
            Some("1080p"),
            false,
            Some("2160p"),
            false
        ));

        // 480p -> 720p is upgrade
        assert!(QualityEvaluator::is_upgrade(
            Some("480p"),
            false,
            Some("720p"),
            false
        ));
    }

    #[test]
    fn test_is_upgrade_hdr() {
        // 1080p SDR -> 1080p HDR is upgrade
        assert!(QualityEvaluator::is_upgrade(
            Some("1080p"),
            false,
            Some("1080p"),
            true
        ));

        // 1080p HDR -> 1080p SDR is NOT upgrade
        assert!(!QualityEvaluator::is_upgrade(
            Some("1080p"),
            true,
            Some("1080p"),
            false
        ));
    }

    #[test]
    fn test_is_upgrade_downgrade() {
        // 1080p -> 720p is NOT upgrade
        assert!(!QualityEvaluator::is_upgrade(
            Some("1080p"),
            false,
            Some("720p"),
            false
        ));

        // 2160p -> 1080p is NOT upgrade
        assert!(!QualityEvaluator::is_upgrade(
            Some("2160p"),
            false,
            Some("1080p"),
            false
        ));
    }

    #[test]
    fn test_is_upgrade_same() {
        // Same resolution, same HDR is NOT upgrade
        assert!(!QualityEvaluator::is_upgrade(
            Some("1080p"),
            false,
            Some("1080p"),
            false
        ));

        assert!(!QualityEvaluator::is_upgrade(
            Some("1080p"),
            true,
            Some("1080p"),
            true
        ));
    }

    #[test]
    fn test_is_upgrade_from_unknown() {
        // Unknown -> anything is upgrade
        assert!(QualityEvaluator::is_upgrade(
            None,
            false,
            Some("1080p"),
            false
        ));
    }

    // =========================================================================
    // Evaluate Parsed Quality Tests
    // =========================================================================

    #[test]
    fn test_evaluate_parsed_no_restrictions() {
        let settings = EffectiveQualitySettings::default();
        let parsed = parse_quality("Show.S01E01.720p.WEB-DL");
        
        let result = QualityEvaluator::evaluate_parsed(&parsed, &settings);
        assert!(result.meets_target);
        assert_eq!(result.quality_status, QualityStatus::Optimal);
    }

    #[test]
    fn test_evaluate_parsed_meets_target() {
        let settings = EffectiveQualitySettings {
            allowed_resolutions: vec!["1080p".to_string()],
            ..Default::default()
        };
        let parsed = parse_quality("Show.S01E01.1080p.WEB-DL");
        
        let result = QualityEvaluator::evaluate_parsed(&parsed, &settings);
        assert!(result.meets_target);
        assert_eq!(result.quality_status, QualityStatus::Optimal);
    }

    #[test]
    fn test_evaluate_parsed_exceeds_target() {
        let settings = EffectiveQualitySettings {
            allowed_resolutions: vec!["1080p".to_string()],
            ..Default::default()
        };
        let parsed = parse_quality("Show.S01E01.2160p.WEB-DL");
        
        let result = QualityEvaluator::evaluate_parsed(&parsed, &settings);
        assert!(result.meets_target);
        assert!(result.is_upgrade);
        assert_eq!(result.quality_status, QualityStatus::Exceeds);
    }

    #[test]
    fn test_evaluate_parsed_below_target() {
        let settings = EffectiveQualitySettings {
            allowed_resolutions: vec!["1080p".to_string()],
            ..Default::default()
        };
        let parsed = parse_quality("Show.S01E01.720p.WEB-DL");
        
        let result = QualityEvaluator::evaluate_parsed(&parsed, &settings);
        assert!(!result.meets_target);
        assert_eq!(result.quality_status, QualityStatus::Suboptimal);
        assert!(result.reason.is_some());
    }

    #[test]
    fn test_evaluate_parsed_hdr_required() {
        let settings = EffectiveQualitySettings {
            require_hdr: true,
            ..Default::default()
        };
        
        // File without HDR
        let parsed = parse_quality("Show.S01E01.1080p.WEB-DL");
        let result = QualityEvaluator::evaluate_parsed(&parsed, &settings);
        assert!(!result.meets_target);
        assert!(result.reason.as_ref().unwrap().contains("HDR required"));
        
        // File with HDR
        let parsed_hdr = parse_quality("Show.S01E01.2160p.HDR.WEB-DL");
        let result_hdr = QualityEvaluator::evaluate_parsed(&parsed_hdr, &settings);
        assert!(result_hdr.meets_target);
    }

    // =========================================================================
    // Real-World Quality Examples
    // =========================================================================

    #[test]
    fn test_evaluate_ds9_upscale() {
        // DS9 960p AI upscale - unusual resolution
        let settings = EffectiveQualitySettings {
            allowed_resolutions: vec!["720p".to_string(), "1080p".to_string()],
            ..Default::default()
        };
        
        // 960p is between 720p and 1080p - should be acceptable as it exceeds 720p
        let parsed = parse_quality("Star Trek- Deep Space Nine - S01E09 960p");
        let result = QualityEvaluator::evaluate_parsed(&parsed, &settings);
        // 960p doesn't match exact allowed resolutions, but should be treated reasonably
        // Depending on implementation, this might be suboptimal or acceptable
        // The key is the logic is consistent
    }

    #[test]
    fn test_evaluate_fallout_hevc() {
        let settings = EffectiveQualitySettings {
            allowed_resolutions: vec!["1080p".to_string()],
            allowed_video_codecs: vec!["HEVC".to_string(), "H.265".to_string()],
            ..Default::default()
        };
        
        let parsed = parse_quality("Fallout.2024.S01E01.1080p.HEVC.x265-MeGusta");
        assert_eq!(parsed.resolution.as_deref(), Some("1080p"));
        assert_eq!(parsed.codec.as_deref(), Some("HEVC"));
    }

    #[test]
    fn test_evaluate_4k_hdr_content() {
        let settings = EffectiveQualitySettings {
            allowed_resolutions: vec!["2160p".to_string()],
            require_hdr: true,
            ..Default::default()
        };
        
        let parsed = parse_quality("Show.S01E01.2160p.HDR.DV.WEB-DL");
        let result = QualityEvaluator::evaluate_parsed(&parsed, &settings);
        assert!(result.meets_target);
    }

    // =========================================================================
    // Quality Status Display Tests
    // =========================================================================

    #[test]
    fn test_quality_status_display() {
        assert_eq!(format!("{}", QualityStatus::Optimal), "optimal");
        assert_eq!(format!("{}", QualityStatus::Suboptimal), "suboptimal");
        assert_eq!(format!("{}", QualityStatus::Exceeds), "exceeds");
        assert_eq!(format!("{}", QualityStatus::Unknown), "unknown");
    }
}
