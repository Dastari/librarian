//! Simple in-memory cache with TTL support
//!
//! Used for caching external API responses like TVMaze schedules.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A cached entry with expiration time
#[derive(Clone)]
struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

/// Simple TTL-based cache
pub struct TtlCache<T: Clone + Send + Sync> {
    entries: RwLock<HashMap<String, CacheEntry<T>>>,
    default_ttl: Duration,
}

impl<T: Clone + Send + Sync> TtlCache<T> {
    /// Create a new cache with the specified default TTL
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            default_ttl,
        }
    }

    /// Get a cached value if it exists and hasn't expired
    pub fn get(&self, key: &str) -> Option<T> {
        let entries = self.entries.read();
        entries.get(key).and_then(|entry| {
            if Instant::now() < entry.expires_at {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    /// Set a cached value with the default TTL
    pub fn set(&self, key: String, value: T) {
        self.set_with_ttl(key, value, self.default_ttl);
    }

    /// Set a cached value with a custom TTL
    pub fn set_with_ttl(&self, key: String, value: T, ttl: Duration) {
        let mut entries = self.entries.write();
        entries.insert(
            key,
            CacheEntry {
                value,
                expires_at: Instant::now() + ttl,
            },
        );
    }

    /// Remove a cached value
    #[allow(dead_code)]
    pub fn remove(&self, key: &str) {
        let mut entries = self.entries.write();
        entries.remove(key);
    }

    /// Remove all expired entries
    #[allow(dead_code)]
    pub fn cleanup_expired(&self) {
        let mut entries = self.entries.write();
        let now = Instant::now();
        entries.retain(|_, entry| entry.expires_at > now);
    }

    /// Check if a key exists and is not expired
    #[allow(dead_code)]
    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
}

/// Shared cache instance type
pub type SharedCache<T> = Arc<TtlCache<T>>;

/// Create a new shared cache
pub fn create_cache<T: Clone + Send + Sync>(default_ttl: Duration) -> SharedCache<T> {
    Arc::new(TtlCache::new(default_ttl))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_cache_set_and_get() {
        let cache = TtlCache::new(Duration::from_secs(60));
        cache.set("key".to_string(), "value".to_string());
        assert_eq!(cache.get("key"), Some("value".to_string()));
    }

    #[test]
    fn test_cache_expiration() {
        let cache = TtlCache::new(Duration::from_millis(50));
        cache.set("key".to_string(), "value".to_string());
        assert_eq!(cache.get("key"), Some("value".to_string()));

        sleep(Duration::from_millis(60));
        assert_eq!(cache.get("key"), None);
    }
}
