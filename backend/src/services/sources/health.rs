//! What a failure of a source or feed means, as pure rules.
//!
//! Part of the health checks: every outcome of testing, searching or polling a source is reduced to
//! a [`FailureKind`], which decides the status shown in Sources (`failing` or `auth_expired`), the
//! alert wording, and how the hourly check backs off.

use chrono::{DateTime, Duration, Utc};

/// Why a source or feed failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// Not signed in, a cookie or key that expired or was refused (HTTP 401/403).
    Auth,
    /// DNS, a refused connection, a timeout or a 5xx.
    Unreachable,
    /// HTTP 429, or the site saying to slow down.
    RateLimited,
    /// Anything else (a page that no longer parses, say).
    Other,
}

impl FailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::Unreachable => "unreachable",
            Self::RateLimited => "rate_limited",
            Self::Other => "other",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "auth" => Some(Self::Auth),
            "unreachable" => Some(Self::Unreachable),
            "rate_limited" => Some(Self::RateLimited),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    /// The `healthStatus` this failure gives: `auth_expired` for sign-in problems, `failing` for
    /// the rest.
    pub fn status(self) -> &'static str {
        match self {
            Self::Auth => STATUS_AUTH_EXPIRED,
            _ => STATUS_FAILING,
        }
    }

    /// A short phrase for a person: what is wrong and what to do about it.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Auth => "its login has expired or was refused. Update its credentials",
            Self::Unreachable => "it cannot be reached right now",
            Self::RateLimited => "it is rate limiting requests",
            Self::Other => "it is failing",
        }
    }
}

pub const STATUS_OK: &str = "ok";
pub const STATUS_FAILING: &str = "failing";
pub const STATUS_AUTH_EXPIRED: &str = "auth_expired";

/// The failure an HTTP status code means, if it means one.
pub fn classify_http_status(status: u16) -> Option<FailureKind> {
    match status {
        401 | 403 => Some(FailureKind::Auth),
        429 => Some(FailureKind::RateLimited),
        500..=599 => Some(FailureKind::Unreachable),
        _ => None,
    }
}

/// The failure an error message means. Sources report errors as text (`not logged in`, `Search
/// timed out after 45s`, `HTTP 403`), so the words are what there is to go on.
pub fn classify_error_text(text: &str) -> FailureKind {
    let lower = text.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|needle| lower.contains(needle));
    if has(&[
        "not logged in",
        "not signed in",
        "cookie",
        "expired",
        "unauthorized",
        "forbidden",
        "invalid api key",
        "invalid apikey",
        "incorrect api",
        "authentication",
        "credentials",
        "login",
        "sign in",
        "http 401",
        "http 403",
        "status: 401",
        "status: 403",
        " 401 ",
        " 403 ",
    ]) {
        return FailureKind::Auth;
    }
    if has(&[
        "429",
        "rate limit",
        "rate-limit",
        "too many requests",
        "slow down",
    ]) {
        return FailureKind::RateLimited;
    }
    if has(&[
        "timed out",
        "timeout",
        "dns",
        "could not be reached",
        "connection refused",
        "connection reset",
        "error sending request",
        "unreachable",
        "http 5",
        "status: 5",
        "bad gateway",
        "service unavailable",
        "gateway",
        "no route to host",
        "failed to connect",
    ]) {
        return FailureKind::Unreachable;
    }
    FailureKind::Other
}

/// How long to wait before testing a source again, given how many checks in a row have failed:
/// hourly while healthy, then 1h, 2h, 4h … up to 24h, so a dead source is not hammered.
pub fn check_interval(consecutive_failures: i32) -> Duration {
    let doublings = consecutive_failures.clamp(0, 5) as u32;
    Duration::hours((1_i64 << doublings).min(24))
}

/// Whether a source is due a health check. A real search or poll that succeeded recently counts as
/// a check, so `last_checked_at` is the later of the last test and the last use.
pub fn check_due(
    last_checked_at: Option<&str>,
    next_check_at: Option<&str>,
    now: DateTime<Utc>,
) -> bool {
    let parse = |text: Option<&str>| {
        text.and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|time| time.with_timezone(&Utc))
    };
    if let Some(next) = parse(next_check_at) {
        return now >= next;
    }
    match parse(last_checked_at) {
        Some(last) => now - last >= Duration::hours(1),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_statuses_map_to_failures() {
        assert_eq!(classify_http_status(401), Some(FailureKind::Auth));
        assert_eq!(classify_http_status(403), Some(FailureKind::Auth));
        assert_eq!(classify_http_status(429), Some(FailureKind::RateLimited));
        assert_eq!(classify_http_status(503), Some(FailureKind::Unreachable));
        assert_eq!(classify_http_status(404), None);
        assert_eq!(classify_http_status(200), None);
    }

    #[test]
    fn error_text_is_classified_by_what_it_says() {
        for (text, kind) in [
            (
                "The user is not logged in. The cookie may have expired or is incorrect.",
                FailureKind::Auth,
            ),
            (
                "Torznab request failed: HTTP 401 Unauthorized",
                FailureKind::Auth,
            ),
            (
                "Connection test failed - check your credentials",
                FailureKind::Auth,
            ),
            ("Search timed out after 45s", FailureKind::Unreachable),
            (
                "error sending request: dns error: failed to lookup address",
                FailureKind::Unreachable,
            ),
            (
                "the feed answered HTTP 503 Service Unavailable",
                FailureKind::Unreachable,
            ),
            (
                "the feed host is rate limiting requests",
                FailureKind::RateLimited,
            ),
            ("HTTP 429 Too Many Requests", FailureKind::RateLimited),
            ("the page changed shape", FailureKind::Other),
        ] {
            assert_eq!(classify_error_text(text), kind, "{text}");
        }
    }

    #[test]
    fn auth_failures_are_shown_as_expired_and_the_rest_as_failing() {
        assert_eq!(FailureKind::Auth.status(), "auth_expired");
        assert_eq!(FailureKind::Unreachable.status(), "failing");
        assert_eq!(
            FailureKind::parse(FailureKind::RateLimited.as_str()),
            Some(FailureKind::RateLimited)
        );
    }

    #[test]
    fn the_check_backs_off_after_repeated_failures_up_to_a_day() {
        assert_eq!(check_interval(0), Duration::hours(1));
        assert_eq!(check_interval(1), Duration::hours(2));
        assert_eq!(check_interval(3), Duration::hours(8));
        assert_eq!(check_interval(5), Duration::hours(24));
        assert_eq!(check_interval(40), Duration::hours(24));
    }

    #[test]
    fn a_source_is_due_an_hour_after_it_was_last_checked_or_used() {
        let now = DateTime::parse_from_rfc3339("2026-11-08T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(check_due(None, None, now));
        assert!(!check_due(Some("2026-11-08T11:30:00Z"), None, now));
        assert!(check_due(Some("2026-11-08T11:00:00Z"), None, now));
        // An explicit next check wins over the hour rule, in both directions.
        assert!(!check_due(
            Some("2026-11-08T09:00:00Z"),
            Some("2026-11-08T13:00:00Z"),
            now
        ));
        assert!(check_due(
            Some("2026-11-08T11:59:00Z"),
            Some("2026-11-08T11:00:00Z"),
            now
        ));
    }
}
