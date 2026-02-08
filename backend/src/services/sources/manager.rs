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
use super::definitions::x1337::Source1337x;
use super::definitions::yts::YtsSource;
use super::encryption::CredentialEncryption;
use super::{Source, SourceQuery, SourceRelease, SourceSearchResult};

/// Default cache TTL (5 minutes)
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
/// Maximum concurrent searches per source
const MAX_CONCURRENT_SEARCHES: usize = 2;

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
                Self::search_single(source_id, source, &query, cache, rate_limiter).await
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
                Self::search_single(source_id, source, &query, cache, rate_limiter).await
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
        if query.cache {
            if let Some(cached) = cache.get(&cache_key) {
                return SourceSearchResult {
                    source_id: source.id().to_string(),
                    source_name: source.name().to_string(),
                    releases: cached,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    from_cache: true,
                    error: None,
                };
            }
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

    #[allow(dead_code)]
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
