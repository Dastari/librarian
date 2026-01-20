//! Indexer Manager
//!
//! The IndexerManager is responsible for:
//! - Loading and managing configured indexer instances
//! - Orchestrating searches across multiple indexers
//! - Caching search results
//! - Rate limiting to avoid tracker bans

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use tokio::sync::Semaphore;
use uuid::Uuid;

use super::definitions::iptorrents::IPTorrentsIndexer;
use super::encryption::CredentialEncryption;
use super::{Indexer, IndexerSearchResult, ReleaseInfo, TorznabQuery};
use crate::db::Database;

/// Default cache TTL (5 minutes)
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
/// Maximum concurrent searches per indexer
const MAX_CONCURRENT_SEARCHES: usize = 2;

/// Manages all configured indexer instances
pub struct IndexerManager {
    /// Database connection
    db: Database,
    /// Credential encryption service
    encryption: CredentialEncryption,
    /// Loaded indexer instances by config ID
    indexers: RwLock<HashMap<Uuid, Arc<dyn Indexer>>>,
    /// Search cache
    cache: RwLock<SearchCache>,
    /// Rate limiting semaphores per indexer
    rate_limiters: RwLock<HashMap<Uuid, Arc<Semaphore>>>,
}

impl IndexerManager {
    /// Create a new IndexerManager
    pub async fn new(db: Database, encryption_key: &str) -> Result<Self> {
        let encryption = CredentialEncryption::from_base64_key(encryption_key)?;

        Ok(Self {
            db,
            encryption,
            indexers: RwLock::new(HashMap::new()),
            cache: RwLock::new(SearchCache::new(DEFAULT_CACHE_TTL)),
            rate_limiters: RwLock::new(HashMap::new()),
        })
    }

    /// Load all enabled indexers for a user
    pub async fn load_user_indexers(&self, user_id: Uuid) -> Result<()> {
        let configs = self.db.indexers().list_by_user(user_id).await?;

        for config in configs {
            if config.enabled {
                if let Err(e) = self.load_indexer(config.id).await {
                    tracing::warn!(
                        indexer_id = %config.id,
                        indexer_name = %config.name,
                        error = %e,
                        "Failed to load indexer"
                    );
                }
            }
        }

        Ok(())
    }

    /// Load a specific indexer by config ID
    pub async fn load_indexer(&self, config_id: Uuid) -> Result<()> {
        let config = self
            .db
            .indexers()
            .get(config_id)
            .await?
            .ok_or_else(|| anyhow!("Indexer config not found: {}", config_id))?;

        let credentials = self.db.indexers().get_credentials(config_id).await?;

        let settings = self.db.indexers().get_settings(config_id).await?;

        // Decrypt credentials
        let mut decrypted_creds: HashMap<String, String> = HashMap::new();
        for cred in credentials {
            let value = self
                .encryption
                .decrypt(&cred.encrypted_value, &cred.nonce)?;
            decrypted_creds.insert(cred.credential_type, value);
        }

        // Convert settings to HashMap
        let settings_map: HashMap<String, String> = settings
            .into_iter()
            .map(|s| (s.setting_key, s.setting_value))
            .collect();

        // Create indexer instance based on type
        let indexer: Arc<dyn Indexer> = match config.indexer_type.as_str() {
            "iptorrents" => {
                let cookie = decrypted_creds
                    .get("cookie")
                    .ok_or_else(|| anyhow!("Cookie is required for IPTorrents"))?;
                let user_agent = decrypted_creds
                    .get("user_agent")
                    .map(|s| s.as_str())
                    .unwrap_or("");

                Arc::new(IPTorrentsIndexer::new(
                    config_id.to_string(),
                    config.name.clone(),
                    config.site_url.clone(),
                    cookie,
                    user_agent,
                    settings_map,
                )?)
            }
            // Add more indexer types here
            _ => {
                return Err(anyhow!("Unknown indexer type: {}", config.indexer_type));
            }
        };

        // Store the indexer
        self.indexers.write().insert(config_id, indexer);

        // Create rate limiter for this indexer
        self.rate_limiters
            .write()
            .insert(config_id, Arc::new(Semaphore::new(MAX_CONCURRENT_SEARCHES)));

        tracing::info!(
            indexer_id = %config_id,
            indexer_name = %config.name,
            indexer_type = %config.indexer_type,
            "Loaded indexer"
        );

        Ok(())
    }

    /// Unload an indexer
    pub fn unload_indexer(&self, config_id: Uuid) {
        self.indexers.write().remove(&config_id);
        self.rate_limiters.write().remove(&config_id);
    }

    /// Get a loaded indexer by config ID
    pub fn get_indexer(&self, config_id: Uuid) -> Option<Arc<dyn Indexer>> {
        self.indexers.read().get(&config_id).cloned()
    }

    /// Get all loaded indexers
    pub fn get_all_indexers(&self) -> Vec<Arc<dyn Indexer>> {
        self.indexers.read().values().cloned().collect()
    }

    /// Search across all enabled indexers
    pub async fn search_all(&self, query: &TorznabQuery) -> Vec<IndexerSearchResult> {
        let indexers: Vec<_> = self
            .indexers
            .read()
            .iter()
            .filter(|(_, idx)| idx.can_handle_query(query))
            .map(|(id, idx)| (*id, idx.clone()))
            .collect();

        let mut results = Vec::with_capacity(indexers.len());

        // Search all indexers concurrently
        let mut handles = vec![];
        for (config_id, indexer) in indexers {
            let query = query.clone();
            let cache = self.cache.read().clone();
            let rate_limiter = self.rate_limiters.read().get(&config_id).cloned();

            let handle = tokio::spawn(async move {
                Self::search_single(config_id, indexer, &query, cache, rate_limiter).await
            });
            handles.push(handle);
        }

        // Collect results
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!(error = %e, "Indexer search task panicked");
                }
            }
        }

        results
    }

    /// Search specific indexers
    pub async fn search_indexers(
        &self,
        indexer_ids: &[Uuid],
        query: &TorznabQuery,
    ) -> Vec<IndexerSearchResult> {
        let indexers: Vec<_> = self
            .indexers
            .read()
            .iter()
            .filter(|(id, idx)| indexer_ids.contains(id) && idx.can_handle_query(query))
            .map(|(id, idx)| (*id, idx.clone()))
            .collect();

        let mut results = Vec::with_capacity(indexers.len());

        let mut handles = vec![];
        for (config_id, indexer) in indexers {
            let query = query.clone();
            let cache = self.cache.read().clone();
            let rate_limiter = self.rate_limiters.read().get(&config_id).cloned();

            let handle = tokio::spawn(async move {
                Self::search_single(config_id, indexer, &query, cache, rate_limiter).await
            });
            handles.push(handle);
        }

        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!(error = %e, "Indexer search task panicked");
                }
            }
        }

        results
    }

    /// Search a single indexer
    async fn search_single(
        config_id: Uuid,
        indexer: Arc<dyn Indexer>,
        query: &TorznabQuery,
        cache: SearchCache,
        rate_limiter: Option<Arc<Semaphore>>,
    ) -> IndexerSearchResult {
        let start = Instant::now();
        let cache_key = format!("{}:{}", config_id, query.cache_key());

        // Check cache first
        if query.cache {
            if let Some(cached) = cache.get(&cache_key) {
                return IndexerSearchResult {
                    indexer_id: indexer.id().to_string(),
                    indexer_name: indexer.name().to_string(),
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
        match indexer.search(query).await {
            Ok(mut releases) => {
                // Add indexer info to releases
                for release in &mut releases {
                    release.indexer_id = Some(indexer.id().to_string());
                    release.indexer_name = Some(indexer.name().to_string());
                }

                // Cache results
                // Note: In a real implementation, we'd update the cache through proper locking
                // For now, this is a simplified version

                IndexerSearchResult {
                    indexer_id: indexer.id().to_string(),
                    indexer_name: indexer.name().to_string(),
                    releases,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    from_cache: false,
                    error: None,
                }
            }
            Err(e) => {
                tracing::error!(
                    indexer_id = indexer.id(),
                    indexer_name = indexer.name(),
                    error = %e,
                    "Search failed"
                );

                IndexerSearchResult {
                    indexer_id: indexer.id().to_string(),
                    indexer_name: indexer.name().to_string(),
                    releases: vec![],
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    from_cache: false,
                    error: Some(e.to_string()),
                }
            }
        }
    }

    /// Test an indexer connection
    pub async fn test_indexer(&self, config_id: Uuid) -> Result<bool> {
        let indexer = self
            .get_indexer(config_id)
            .ok_or_else(|| anyhow!("Indexer not loaded: {}", config_id))?;

        indexer.test_connection().await
    }

    /// Download a torrent file using the appropriate indexer's authentication
    ///
    /// This method downloads the torrent file with proper cookies/headers for
    /// private trackers. It should be used instead of directly fetching URLs.
    ///
    /// # Arguments
    /// * `indexer_id` - The indexer ID (string, from ReleaseInfo.indexer_id)
    /// * `link` - The download URL for the torrent file
    ///
    /// # Returns
    /// The torrent file bytes on success
    pub async fn download_torrent(&self, indexer_id: &str, link: &str) -> Result<Vec<u8>> {
        // Parse the indexer ID
        let config_id = Uuid::parse_str(indexer_id)
            .map_err(|e| anyhow!("Invalid indexer ID '{}': {}", indexer_id, e))?;

        // Get the indexer
        let indexer = self
            .get_indexer(config_id)
            .ok_or_else(|| anyhow!("Indexer not loaded: {}", indexer_id))?;

        tracing::debug!(
            indexer_id = %indexer_id,
            indexer_name = %indexer.name(),
            link = %link,
            "Downloading torrent via indexer"
        );

        // Use the indexer's download method (includes authentication)
        indexer.download(link).await
    }

    /// Download a torrent from a release
    ///
    /// Convenience method that extracts the indexer_id and link from a ReleaseInfo
    pub async fn download_release(&self, release: &ReleaseInfo) -> Result<Vec<u8>> {
        let indexer_id = release
            .indexer_id
            .as_ref()
            .ok_or_else(|| anyhow!("Release has no indexer_id"))?;

        let link = release
            .link
            .as_ref()
            .ok_or_else(|| anyhow!("Release has no download link"))?;

        self.download_torrent(indexer_id, link).await
    }

    /// Get the encryption service (for database operations)
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
    releases: Vec<ReleaseInfo>,
    expires_at: Instant,
}

impl SearchCache {
    fn new(ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    fn get(&self, key: &str) -> Option<Vec<ReleaseInfo>> {
        let entries = self.entries.read();
        entries.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.releases.clone())
            } else {
                None
            }
        })
    }

    fn insert(&self, key: String, releases: Vec<ReleaseInfo>) {
        let mut entries = self.entries.write();
        entries.insert(
            key,
            CacheEntry {
                releases,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }

    /// Remove expired entries
    fn cleanup(&self) {
        let mut entries = self.entries.write();
        let now = Instant::now();
        entries.retain(|_, entry| entry.expires_at > now);
    }
}

impl std::fmt::Debug for IndexerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexerManager")
            .field("indexers_count", &self.indexers.read().len())
            .finish()
    }
}
