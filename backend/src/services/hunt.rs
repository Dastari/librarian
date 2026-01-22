//! Hunt Service
//!
//! Priority-based search across all download source types (torrents, usenet, etc.).
//! This service:
//! - Loads user-defined priority rules
//! - Searches sources in priority order
//! - Optionally stops at first match (for efficiency)
//! - Provides a unified interface for manual and auto-hunt

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::priority_rules::{PriorityRuleRecord, SourceRef, SourceType};
use crate::db::Database;
use crate::indexer::manager::IndexerManager;
use crate::indexer::{IndexerSearchResult, ReleaseInfo, TorznabQuery};

/// Result of a priority-based hunt search
#[derive(Debug, Clone, Serialize)]
pub struct HuntSearchResult {
    /// All releases found across searched sources
    pub releases: Vec<ReleaseInfo>,
    /// Number of sources that were searched
    pub sources_searched: usize,
    /// Whether we stopped early (found results before searching all sources)
    pub stopped_early: bool,
    /// Total time taken for all searches
    pub elapsed_ms: u64,
    /// Which priority rule was applied (for debugging)
    pub rule_description: String,
    /// Per-source search results (for detailed UI display)
    pub source_results: Vec<IndexerSearchResult>,
}

/// Configuration for hunt behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuntConfig {
    /// Search all sources even after finding matches
    pub search_all_sources: bool,
    /// Maximum number of results to return per source
    pub max_results_per_source: Option<usize>,
}

impl Default for HuntConfig {
    fn default() -> Self {
        Self {
            search_all_sources: false,
            max_results_per_source: Some(100),
        }
    }
}

/// Priority-based search service
pub struct HuntService {
    db: Database,
    indexer_manager: Arc<IndexerManager>,
    // Future: usenet_indexer_manager: Arc<UsenetIndexerManager>,
}

impl HuntService {
    /// Create a new HuntService
    pub fn new(db: Database, indexer_manager: Arc<IndexerManager>) -> Self {
        Self {
            db,
            indexer_manager,
        }
    }

    /// Search for releases using priority rules
    ///
    /// # Arguments
    /// * `query` - The Torznab search query
    /// * `user_id` - User performing the search
    /// * `library_type` - Optional library type for priority lookup (e.g., "tv", "movies")
    /// * `library_id` - Optional specific library for priority lookup
    /// * `config` - Optional hunt configuration
    pub async fn search(
        &self,
        query: &TorznabQuery,
        user_id: Uuid,
        library_type: Option<&str>,
        library_id: Option<Uuid>,
        config: Option<HuntConfig>,
    ) -> Result<HuntSearchResult> {
        let start = Instant::now();
        let config = config.unwrap_or_default();

        // Load user's indexers
        self.indexer_manager.load_user_indexers(user_id).await?;

        // Get the applicable priority rule
        let (priority_order, search_all, rule_desc) =
            self.get_effective_priority(user_id, library_type, library_id)
                .await?;

        // Override search_all if config specifies
        let search_all = config.search_all_sources || search_all;

        debug!(
            user_id = %user_id,
            library_type = ?library_type,
            library_id = ?library_id,
            rule = %rule_desc,
            sources_count = priority_order.len(),
            search_all = search_all,
            "Starting priority-based search"
        );

        let mut all_releases = Vec::new();
        let mut source_results = Vec::new();
        let mut sources_searched = 0;
        let mut stopped_early = false;

        // Search sources in priority order
        for source_ref in &priority_order {
            sources_searched += 1;

            match source_ref.source_type {
                SourceType::TorrentIndexer => {
                    let indexer_id = match Uuid::parse_str(&source_ref.id) {
                        Ok(id) => id,
                        Err(e) => {
                            warn!(
                                source_id = %source_ref.id,
                                error = %e,
                                "Invalid indexer ID in priority order, skipping"
                            );
                            continue;
                        }
                    };

                    // Search this specific indexer
                    let results = self
                        .indexer_manager
                        .search_indexers(&[indexer_id], query)
                        .await;

                    for result in results {
                        let release_count = result.releases.len();
                        all_releases.extend(result.releases.clone());
                        source_results.push(result);

                        if release_count > 0 {
                            info!(
                                indexer_id = %indexer_id,
                                releases = release_count,
                                "Found releases from torrent indexer"
                            );
                        }
                    }
                }
                SourceType::UsenetIndexer => {
                    // TODO: Implement when UsenetIndexerManager is ready
                    debug!(
                        source_id = %source_ref.id,
                        "Usenet indexer search not yet implemented, skipping"
                    );
                }
            }

            // Check if we should stop early
            if !search_all && !all_releases.is_empty() {
                stopped_early = true;
                info!(
                    sources_searched = sources_searched,
                    total_sources = priority_order.len(),
                    releases_found = all_releases.len(),
                    "Stopping early - found results"
                );
                break;
            }
        }

        // Apply max results limit if configured
        if let Some(max) = config.max_results_per_source {
            let total_max = max * sources_searched;
            if all_releases.len() > total_max {
                all_releases.truncate(total_max);
            }
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;

        info!(
            sources_searched = sources_searched,
            releases_found = all_releases.len(),
            stopped_early = stopped_early,
            elapsed_ms = elapsed_ms,
            rule = %rule_desc,
            "Priority-based search complete"
        );

        Ok(HuntSearchResult {
            releases: all_releases,
            sources_searched,
            stopped_early,
            elapsed_ms,
            rule_description: rule_desc,
            source_results,
        })
    }

    /// Search all sources without priority (for backward compatibility)
    ///
    /// This is equivalent to the old `IndexerManager.search_all()` behavior
    pub async fn search_all(
        &self,
        query: &TorznabQuery,
        user_id: Uuid,
    ) -> Result<Vec<IndexerSearchResult>> {
        // Load user's indexers
        self.indexer_manager.load_user_indexers(user_id).await?;

        // Search all indexers
        Ok(self.indexer_manager.search_all(query).await)
    }

    /// Get the effective priority order for a search context
    ///
    /// Returns: (priority_order, search_all_sources, rule_description)
    async fn get_effective_priority(
        &self,
        user_id: Uuid,
        library_type: Option<&str>,
        library_id: Option<Uuid>,
    ) -> Result<(Vec<SourceRef>, bool, String)> {
        // Try to find an applicable rule
        let rule = self
            .db
            .priority_rules()
            .get_applicable_rule(user_id, library_type, library_id)
            .await?;

        if let Some(rule) = rule {
            let rule_desc = self.describe_rule(&rule);
            let priority_order: Vec<SourceRef> = rule.priority_order.clone();

            return Ok((priority_order, rule.search_all_sources, rule_desc));
        }

        // No rule found - build default from all enabled indexers
        self.build_default_priority(user_id).await
    }

    /// Build default priority order from all enabled indexers
    async fn build_default_priority(
        &self,
        user_id: Uuid,
    ) -> Result<(Vec<SourceRef>, bool, String)> {
        // Get all enabled indexers sorted by priority
        let indexers = self.db.indexers().list_enabled_by_user(user_id).await?;

        let priority_order: Vec<SourceRef> = indexers
            .into_iter()
            .map(|idx| SourceRef {
                source_type: SourceType::TorrentIndexer,
                id: idx.id.to_string(),
            })
            .collect();

        // TODO: Also include usenet indexers when available
        // let usenet_indexers = self.db.usenet_indexers().list_enabled_by_user(user_id).await?;
        // priority_order.extend(usenet_indexers.into_iter().map(|idx| SourceRef {
        //     source_type: SourceType::UsenetIndexer,
        //     id: idx.id.to_string(),
        // }));

        Ok((
            priority_order,
            true, // search all by default when no rule
            "default (all enabled sources by priority)".to_string(),
        ))
    }

    /// Create a human-readable description of a priority rule
    fn describe_rule(&self, rule: &PriorityRuleRecord) -> String {
        if let Some(ref lib_id) = rule.library_id {
            format!("library-specific ({})", lib_id)
        } else if let Some(ref lib_type) = rule.library_type {
            format!("library-type: {}", lib_type)
        } else {
            "user default".to_string()
        }
    }
}

impl std::fmt::Debug for HuntService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HuntService").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hunt_config_default() {
        let config = HuntConfig::default();
        assert!(!config.search_all_sources);
        assert_eq!(config.max_results_per_source, Some(100));
    }
}
