//! Simple in-memory brute-force protection for the login mutation.
//!
//! Tracks failed login attempts per (lowercased, trimmed) username/email in a sliding
//! window. Once a key accumulates `max_attempts` failures within `window`, further
//! attempts for that key are blocked until enough of them age out. A successful login
//! clears the key's history immediately.
//!
//! This is intentionally process-local, in-memory state (no persistence, no
//! distributed coordination) - adequate as a first line of defense against naive
//! credential-stuffing/brute-force scripts, not a substitute for a real WAF/edge rate
//! limiter in a multi-instance deployment.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Default policy: 10 failed attempts per 15-minute sliding window.
pub const DEFAULT_MAX_ATTEMPTS: usize = 10;
pub const DEFAULT_WINDOW: Duration = Duration::from_secs(15 * 60);

/// Message returned to the caller when a key is currently blocked. Deliberately generic
/// - it must not reveal whether the attempted username/email corresponds to a real user.
pub const RATE_LIMITED_MESSAGE: &str = "Too many attempts, try again later";

/// Whether an auth-layer error is a throttle/lockout rather than a credential mismatch.
/// `agql-auth` uses "throttled" / "locked"; this crate uses [`RATE_LIMITED_MESSAGE`].
pub fn is_rate_limit_message(detail: &str) -> bool {
    let lower = detail.to_ascii_lowercase();
    lower.contains("too many attempts")
        || lower.contains("throttled")
        || lower.contains("temporarily locked")
        || lower.contains("rate limited")
}

pub struct LoginRateLimiter {
    max_attempts: usize,
    window: Duration,
    attempts: Mutex<HashMap<String, Vec<Instant>>>,
}

impl LoginRateLimiter {
    pub fn new(max_attempts: usize, window: Duration) -> Self {
        Self {
            max_attempts,
            window,
            attempts: Mutex::new(HashMap::new()),
        }
    }

    fn normalize(key: &str) -> String {
        key.trim().to_ascii_lowercase()
    }

    /// Drop timestamps older than `window` for `key`, returning the remaining count.
    /// Opportunistic: only prunes the entry being touched, and removes it entirely from
    /// the map once empty so the map doesn't grow unbounded with stale keys.
    fn prune(
        attempts: &mut HashMap<String, Vec<Instant>>,
        key: &str,
        window: Duration,
        now: Instant,
    ) -> usize {
        let Some(timestamps) = attempts.get_mut(key) else {
            return 0;
        };
        timestamps.retain(|t| now.duration_since(*t) < window);
        if timestamps.is_empty() {
            attempts.remove(key);
            0
        } else {
            timestamps.len()
        }
    }

    /// Whether `key` currently has too many recorded failures within the window.
    pub fn is_blocked(&self, key: &str) -> bool {
        let key = Self::normalize(key);
        let now = Instant::now();
        let mut attempts = self.attempts.lock();
        Self::prune(&mut attempts, &key, self.window, now) >= self.max_attempts
    }

    /// Record a failed login attempt for `key`.
    pub fn record_failure(&self, key: &str) {
        let key = Self::normalize(key);
        let now = Instant::now();
        let mut attempts = self.attempts.lock();
        Self::prune(&mut attempts, &key, self.window, now);
        attempts.entry(key).or_default().push(now);
    }

    /// Clear recorded failures for `key` (called after a successful login).
    pub fn clear(&self, key: &str) {
        let key = Self::normalize(key);
        self.attempts.lock().remove(&key);
    }
}

impl Default for LoginRateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ATTEMPTS, DEFAULT_WINDOW)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_key_is_not_blocked() {
        let limiter = LoginRateLimiter::new(3, Duration::from_secs(60));
        assert!(!limiter.is_blocked("nobody@example.com"));
    }

    #[test]
    fn detects_throttle_and_lockout_messages() {
        assert!(is_rate_limit_message(RATE_LIMITED_MESSAGE));
        assert!(is_rate_limit_message(
            "authentication flow is temporarily throttled"
        ));
        assert!(is_rate_limit_message(
            "authentication flow is temporarily locked"
        ));
        assert!(!is_rate_limit_message("invalid credentials"));
        assert!(!is_rate_limit_message("user is disabled"));
    }

    #[test]
    fn blocks_after_reaching_max_attempts() {
        let limiter = LoginRateLimiter::new(3, Duration::from_secs(60));
        limiter.record_failure("user@example.com");
        assert!(!limiter.is_blocked("user@example.com"));
        limiter.record_failure("user@example.com");
        assert!(!limiter.is_blocked("user@example.com"));
        limiter.record_failure("user@example.com");
        assert!(limiter.is_blocked("user@example.com"));
    }

    #[test]
    fn successful_login_clears_recorded_failures() {
        let limiter = LoginRateLimiter::new(2, Duration::from_secs(60));
        limiter.record_failure("user@example.com");
        limiter.record_failure("user@example.com");
        assert!(limiter.is_blocked("user@example.com"));

        limiter.clear("user@example.com");
        assert!(!limiter.is_blocked("user@example.com"));
    }

    #[test]
    fn keys_are_normalized_by_case_and_whitespace() {
        let limiter = LoginRateLimiter::new(1, Duration::from_secs(60));
        limiter.record_failure("  User@Example.com  ");
        assert!(limiter.is_blocked("user@example.com"));
        assert!(limiter.is_blocked("USER@EXAMPLE.COM"));
    }

    #[test]
    fn distinct_keys_are_tracked_independently() {
        let limiter = LoginRateLimiter::new(1, Duration::from_secs(60));
        limiter.record_failure("alice@example.com");
        assert!(limiter.is_blocked("alice@example.com"));
        assert!(!limiter.is_blocked("bob@example.com"));
    }

    #[test]
    fn stale_attempts_outside_the_window_are_pruned() {
        let limiter = LoginRateLimiter::new(2, Duration::from_millis(30));
        limiter.record_failure("user@example.com");
        limiter.record_failure("user@example.com");
        assert!(limiter.is_blocked("user@example.com"));

        std::thread::sleep(Duration::from_millis(60));
        assert!(!limiter.is_blocked("user@example.com"));

        // A fresh failure after the window elapsed should not be immediately blocked.
        limiter.record_failure("user@example.com");
        assert!(!limiter.is_blocked("user@example.com"));
    }
}
