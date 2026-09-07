//! Sources Manager
//!
//! The SourcesManager is responsible for:
//! - Loading and managing configured source instances
//! - Orchestrating searches across multiple sources (priority-ordered)
//! - Caching search results
//! - Rate limiting to avoid tracker bans

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use tokio::sync::Semaphore;

use super::definitions::iptorrents::IPTorrentsSource;
use super::definitions::limetorrents::LimeTorrentsSource;
use super::definitions::thepiratebay::ThePirateBaySource;
use super::definitions::torznab::TorznabSource;
use super::definitions::x1337::Source1337x;
use super::definitions::yts::YtsSource;
use super::encryption::CredentialEncryption;
use super::{Source, SourceQuery, SourceRelease, SourceSearchResult};

/// Default cache TTL (5 minutes)
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
/// Maximum concurrent searches per source
const MAX_CONCURRENT_SEARCHES: usize = 2;
/// Deadline for a single source's search, so one stalled tracker can't hang
/// the whole `search_all`/`search_sources` call forever.
const SOURCE_SEARCH_TIMEOUT: Duration = Duration::from_secs(45);

/// Manages all configured source instances
pub struct SourcesManager {
    /// Credential encryption service
    encryption: CredentialEncryption,
    /// Loaded source instances by config ID
    sources: RwLock<HashMap<String, Arc<dyn Source>>>,
    /// Source priorities (source_id -> priority, lower = higher priority)
    priorities: RwLock<HashMap<String, i32>>,
    /// Search cache
    cache: RwLock<SearchCache>,
    /// Rate limiting semaphores per source
    rate_limiters: RwLock<HashMap<String, Arc<Semaphore>>>,
}

impl SourcesManager {
    /// Create a new SourcesManager
    pub fn new(encryption_key: &str) -> Result<Self> {
        let encryption = CredentialEncryption::from_base64_key(encryption_key)?;

        Ok(Self {
            encryption,
            sources: RwLock::new(HashMap::new()),
            priorities: RwLock::new(HashMap::new()),
            cache: RwLock::new(SearchCache::new(DEFAULT_CACHE_TTL)),
            rate_limiters: RwLock::new(HashMap::new()),
        })
    }

    /// Load a source from its configuration
    ///
    /// `credentials` is a HashMap of decrypted credential values (e.g., {"Cookie": "...", "UserAgent": "..."})
    /// `settings` is a HashMap of optional settings (e.g., {"Freeleech": "true", "Sort": "seeders"})
    #[allow(clippy::too_many_arguments)]
    pub fn load_source(
        &self,
        source_id: &str,
        definition_id: &str,
        name: &str,
        site_url: Option<String>,
        priority: i32,
        credentials: HashMap<String, String>,
        settings: HashMap<String, String>,
    ) -> Result<()> {
        let source: Arc<dyn Source> = match definition_id {
            "1337x" => Arc::new(Source1337x::new(
                source_id.to_string(),
                name.to_string(),
                site_url,
                settings,
            )?),
            "iptorrents" => {
                let cookie = credentials.get("Cookie").cloned().unwrap_or_default();
                let user_agent = credentials.get("UserAgent").cloned().unwrap_or_default();

                Arc::new(IPTorrentsSource::new(
                    source_id.to_string(),
                    name.to_string(),
                    site_url,
                    &cookie,
                    &user_agent,
                    settings,
                )?)
            }
            "limetorrents" => Arc::new(LimeTorrentsSource::new(
                source_id.to_string(),
                name.to_string(),
                site_url,
                settings,
            )?),
            "thepiratebay" => Arc::new(ThePirateBaySource::new(
                source_id.to_string(),
                name.to_string(),
                site_url,
                settings,
            )?),
            "yts" => Arc::new(YtsSource::new(
                source_id.to_string(),
                name.to_string(),
                site_url,
                settings,
            )?),
            "torznab" => {
                let api_key = credentials.get("ApiKey").cloned().unwrap_or_default();

                Arc::new(TorznabSource::new(
                    source_id.to_string(),
                    name.to_string(),
                    site_url,
                    &api_key,
                    settings,
                )?)
            }
            _ => {
                return Err(anyhow!("Unknown source definition: {}", definition_id));
            }
        };

        self.sources.write().insert(source_id.to_string(), source);
        self.priorities
            .write()
            .insert(source_id.to_string(), priority);
        self.rate_limiters.write().insert(
            source_id.to_string(),
            Arc::new(Semaphore::new(MAX_CONCURRENT_SEARCHES)),
        );

        tracing::info!(
            source_id = %source_id,
            source_name = %name,
            definition_id = %definition_id,
            priority = %priority,
            "Loaded source '{}' ({})",
            name, definition_id
        );

        Ok(())
    }

    /// Unload a source
    pub fn unload_source(&self, source_id: &str) {
        self.sources.write().remove(source_id);
        self.priorities.write().remove(source_id);
        self.rate_limiters.write().remove(source_id);
    }

    /// Clear all loaded sources before rebuilding from current database state.
    pub fn clear_sources(&self) {
        self.sources.write().clear();
        self.priorities.write().clear();
        self.rate_limiters.write().clear();
    }

    /// Get a loaded source by ID
    pub fn get_source(&self, source_id: &str) -> Option<Arc<dyn Source>> {
        self.sources.read().get(source_id).cloned()
    }

    /// Get all loaded sources sorted by priority (lower priority number = searched first)
    pub fn get_all_sources_by_priority(&self) -> Vec<(String, Arc<dyn Source>)> {
        let sources = self.sources.read();
        let priorities = self.priorities.read();

        let mut sorted: Vec<_> = sources
            .iter()
            .map(|(id, src)| {
                let priority = priorities.get(id).copied().unwrap_or(50);
                (id.clone(), src.clone(), priority)
            })
            .collect();

        sorted.sort_by_key(|(_, _, p)| *p);
        sorted.into_iter().map(|(id, src, _)| (id, src)).collect()
    }

    /// Search across all enabled sources, ordered by priority
    pub async fn search_all(&self, query: &SourceQuery) -> Vec<SourceSearchResult> {
        let sources: Vec<_> = self
            .get_all_sources_by_priority()
            .into_iter()
            .filter(|(_, src)| src.can_handle_query(query))
            .collect();

        let mut results = Vec::with_capacity(sources.len());

        // Search all sources concurrently
        let mut handles = vec![];
        for (source_id, source) in sources {
            let query = query.clone();
            let cache = self.cache.read().clone();
            let rate_limiter = self.rate_limiters.read().get(&source_id).cloned();

            let handle = tokio::spawn(async move {
                Self::search_single_with_timeout(source_id, source, &query, cache, rate_limiter)
                    .await
            });
            handles.push(handle);
        }

        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "Source search task failed to join (panic or cancellation): error={}",
                        e
                    );
                }
            }
        }

        results
    }

    /// Search specific sources
    pub async fn search_sources(
        &self,
        source_ids: &[String],
        query: &SourceQuery,
    ) -> Vec<SourceSearchResult> {
        let all_sources = self.get_all_sources_by_priority();
        let sources: Vec<_> = all_sources
            .into_iter()
            .filter(|(id, src)| source_ids.contains(id) && src.can_handle_query(query))
            .collect();

        let mut results = Vec::with_capacity(sources.len());

        let mut handles = vec![];
        for (source_id, source) in sources {
            let query = query.clone();
            let cache = self.cache.read().clone();
            let rate_limiter = self.rate_limiters.read().get(&source_id).cloned();

            let handle = tokio::spawn(async move {
                Self::search_single_with_timeout(source_id, source, &query, cache, rate_limiter)
                    .await
            });
            handles.push(handle);
        }

        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "Source search task failed to join (panic or cancellation): error={}",
                        e
                    );
                }
            }
        }

        results
    }

    /// Search a single source, bounded by [`SOURCE_SEARCH_TIMEOUT`] so a stalled
    /// tracker cannot hang the whole batch (and its GraphQL request) forever.
    async fn search_single_with_timeout(
        source_id: String,
        source: Arc<dyn Source>,
        query: &SourceQuery,
        cache: SearchCache,
        rate_limiter: Option<Arc<Semaphore>>,
    ) -> SourceSearchResult {
        let source_name = source.name().to_string();
        let start = Instant::now();
        match tokio::time::timeout(
            SOURCE_SEARCH_TIMEOUT,
            Self::search_single(source_id.clone(), source, query, cache, rate_limiter),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                tracing::warn!(
                    source_id = %source_id,
                    source_name = %source_name,
                    timeout_secs = SOURCE_SEARCH_TIMEOUT.as_secs(),
                    "Source search timed out; treating as empty result: source_id={}, source_name='{}', timeout_secs={}",
                    source_id,
                    source_name,
                    SOURCE_SEARCH_TIMEOUT.as_secs()
                );
                SourceSearchResult {
                    source_id,
                    source_name,
                    releases: vec![],
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    from_cache: false,
                    error: Some(format!(
                        "Search timed out after {}s",
                        SOURCE_SEARCH_TIMEOUT.as_secs()
                    )),
                }
            }
        }
    }

    /// Search a single source
    async fn search_single(
        source_id: String,
        source: Arc<dyn Source>,
        query: &SourceQuery,
        cache: SearchCache,
        rate_limiter: Option<Arc<Semaphore>>,
    ) -> SourceSearchResult {
        let start = Instant::now();
        let cache_key = format!("{}:{}", source_id, query.cache_key());

        // Check cache first
        if query.cache
            && let Some(cached) = cache.get(&cache_key)
        {
            return SourceSearchResult {
                source_id: source.id().to_string(),
                source_name: source.name().to_string(),
                releases: cached,
                elapsed_ms: start.elapsed().as_millis() as u64,
                from_cache: true,
                error: None,
            };
        }

        // Acquire rate limit permit
        let _permit = if let Some(ref limiter) = rate_limiter {
            Some(limiter.acquire().await)
        } else {
            None
        };

        // Perform search
        match source.search(query).await {
            Ok(mut releases) => {
                // Add source info to releases
                for release in &mut releases {
                    release.source_id = Some(source.id().to_string());
                    release.source_name = Some(source.name().to_string());
                }
                if query.cache {
                    cache.insert(cache_key, releases.clone());
                }

                SourceSearchResult {
                    source_id: source.id().to_string(),
                    source_name: source.name().to_string(),
                    releases,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    from_cache: false,
                    error: None,
                }
            }
            Err(e) => {
                tracing::error!(
                    source_id = source.id(),
                    source_name = source.name(),
                    error = %e,
                    "Source search failed: source_id={}, source_name='{}', error={}",
                    source.id(),
                    source.name(),
                    e
                );

                SourceSearchResult {
                    source_id: source.id().to_string(),
                    source_name: source.name().to_string(),
                    releases: vec![],
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    from_cache: false,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    /// Test a source connection
    pub async fn test_source(&self, source_id: &str) -> Result<bool> {
        let source = self
            .get_source(source_id)
            .ok_or_else(|| anyhow!("Source not loaded: {}", source_id))?;

        source.test_connection().await
    }

    /// Download a file using the appropriate source's authentication
    pub async fn download_from_source(&self, source_id: &str, link: &str) -> Result<Vec<u8>> {
        let source = self
            .get_source(source_id)
            .ok_or_else(|| anyhow!("Source not loaded: {}", source_id))?;

        tracing::debug!(
            source_id = %source_id,
            source_name = %source.name(),
            link = %link,
            "Downloading via source '{}'",
            source.name()
        );

        source.download(link).await
    }

    /// Get the encryption service
    pub fn encryption(&self) -> &CredentialEncryption {
        &self.encryption
    }
}

/// Simple in-memory search cache
#[derive(Clone)]
struct SearchCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    ttl: Duration,
}

struct CacheEntry {
    releases: Vec<SourceRelease>,
    expires_at: Instant,
}

impl SearchCache {
    fn new(ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    fn get(&self, key: &str) -> Option<Vec<SourceRelease>> {
        let entries = self.entries.read();
        entries.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.releases.clone())
            } else {
                None
            }
        })
    }

    fn insert(&self, key: String, releases: Vec<SourceRelease>) {
        let mut entries = self.entries.write();
        entries.insert(
            key,
            CacheEntry {
                releases,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }
}

impl std::fmt::Debug for SourcesManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourcesManager")
            .field("sources_count", &self.sources.read().len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::super::encryption::CredentialEncryption;
    use super::*;

    /// tier1-features-plan.md §3: "Confirm createSource already supports
    /// multiple Source rows with the same definition_id but different
    /// site_url/credentials (it should - load_source keys off source_id)."
    /// This exercises exactly that at the `SourcesManager` level (no DB
    /// needed): two independently configured "torznab" instances must coexist,
    /// each retaining its own site_url/credentials/priority, keyed by their
    /// distinct source_id rather than by definition_id.
    #[test]
    fn same_definition_id_supports_multiple_independent_instances() {
        let key = CredentialEncryption::generate_key();
        let manager = SourcesManager::new(&key).expect("manager should construct");

        let mut creds_a = HashMap::new();
        creds_a.insert("ApiKey".to_string(), "key-for-indexer-a".to_string());
        manager
            .load_source(
                "source-a",
                "torznab",
                "Indexer A",
                Some("https://indexer-a.example.com".to_string()),
                10,
                creds_a,
                HashMap::new(),
            )
            .expect("first torznab instance should load");

        let mut creds_b = HashMap::new();
        creds_b.insert("ApiKey".to_string(), "key-for-indexer-b".to_string());
        manager
            .load_source(
                "source-b",
                "torznab",
                "Indexer B",
                Some("https://indexer-b.example.com".to_string()),
                20,
                creds_b,
                HashMap::new(),
            )
            .expect("second torznab instance should load");

        let source_a = manager
            .get_source("source-a")
            .expect("source-a should be loaded");
        let source_b = manager
            .get_source("source-b")
            .expect("source-b should be loaded");

        assert_eq!(source_a.definition_id(), "torznab");
        assert_eq!(source_b.definition_id(), "torznab");
        assert_ne!(source_a.site_link(), source_b.site_link());
        assert_eq!(source_a.site_link(), "https://indexer-a.example.com/");
        assert_eq!(source_b.site_link(), "https://indexer-b.example.com/");

        // Both present simultaneously, ordered by priority (source-a first).
        let by_priority = manager.get_all_sources_by_priority();
        assert_eq!(by_priority.len(), 2);
        assert_eq!(by_priority[0].0, "source-a");
        assert_eq!(by_priority[1].0, "source-b");
    }

    #[test]
    fn torznab_missing_site_url_fails_to_load() {
        let key = CredentialEncryption::generate_key();
        let manager = SourcesManager::new(&key).expect("manager should construct");

        let mut creds = HashMap::new();
        creds.insert("ApiKey".to_string(), "some-key".to_string());
        let result = manager.load_source(
            "source-c",
            "torznab",
            "Indexer C",
            None,
            5,
            creds,
            HashMap::new(),
        );

        assert!(result.is_err());
        assert!(manager.get_source("source-c").is_none());
    }
}
