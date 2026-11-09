//! Hiding secrets that ride inside URLs.
//!
//! Private trackers put the account's passkey in the address of their RSS feed and of every
//! download link (`...?u=123456;tp=abcdef...`, `torrent_pass=...`). Those addresses must never reach a
//! log line, an error message shown to a person, a notification or the GraphQL schema.
//!
//! Two shapes of redaction:
//!
//! - [`redact_url`] keeps an address recognisable (host, path, parameter names) and replaces the
//!   values of parameters that look secret, long opaque path segments and any userinfo.
//!   `mask_all_values` also hides every other parameter value: that is what is *shown* for a feed.
//! - [`redact_text`] finds URLs inside free text (an error from the HTTP client, say) and redacts
//!   each one.
//!
//! Both are total: text that is not a URL is returned unchanged, never an error.

use once_cell::sync::Lazy;
use regex::Regex;

/// What replaces a hidden value.
pub const MASK: &str = "•••";

/// Parameter names whose values are secrets on the trackers this app talks to. `u` and `tp` are
/// IPTorrents' user id and passkey; the rest are the common spellings elsewhere.
const SECRET_KEYS: &[&str] = &[
    "u",
    "tp",
    "uid",
    "pass",
    "passkey",
    "pass_key",
    "password",
    "torrent_pass",
    "torrentpass",
    "rsskey",
    "rss_key",
    "apikey",
    "api_key",
    "key",
    "token",
    "auth",
    "authkey",
    "auth_key",
    "secret",
    "sid",
    "session",
    "cookie",
];

fn is_secret_key(key: &str) -> bool {
    let key = key.trim().to_ascii_lowercase();
    SECRET_KEYS.contains(&key.as_str())
        || key.ends_with("key")
        || key.ends_with("pass")
        || key.ends_with("token")
        || key.ends_with("secret")
}

/// A path segment that looks like an opaque token (a passkey in the path): long, one run of
/// letters and digits, with at least one of each.
fn is_opaque_segment(segment: &str) -> bool {
    let segment = segment
        .trim_end_matches(".torrent")
        .trim_end_matches(".rss");
    segment.len() >= 24
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && segment.chars().any(|c| c.is_ascii_digit())
        && segment.chars().any(|c| c.is_ascii_alphabetic())
}

/// A bare query token (no `=`), such as a category id or `download`, that is really a secret: a
/// long opaque run.
fn is_opaque_token(token: &str) -> bool {
    is_opaque_segment(token) && token.len() >= 20
}

/// Redact the query string. Separators are `&` and `;` (IPTorrents uses `;`), and each is kept.
fn redact_query(query: &str, mask_all_values: bool) -> String {
    let mut out = String::with_capacity(query.len());
    let mut token = String::new();
    let flush = |token: &mut String, out: &mut String| {
        if token.is_empty() {
            return;
        }
        match token.split_once('=') {
            Some((key, _)) if mask_all_values || is_secret_key(key) => {
                out.push_str(key);
                out.push('=');
                out.push_str(MASK);
            }
            Some(_) => out.push_str(token),
            None if is_opaque_token(token) => out.push_str(MASK),
            None => out.push_str(token),
        }
        token.clear();
    };
    for c in query.chars() {
        if c == '&' || c == ';' {
            flush(&mut token, &mut out);
            out.push(c);
        } else {
            token.push(c);
        }
    }
    flush(&mut token, &mut out);
    out
}

/// A URL with its secrets hidden. `mask_all_values` hides every query value, not only the ones
/// that look secret. Anything that is not an `http(s)` URL comes back unchanged, except that a
/// `magnet:` link keeps only its info hash.
pub fn redact_url(url: &str, mask_all_values: bool) -> String {
    let trimmed = url.trim();
    if trimmed.to_ascii_lowercase().starts_with("magnet:") {
        return redact_magnet(trimmed);
    }
    let lower = trimmed.to_ascii_lowercase();
    let scheme_len = if lower.starts_with("https://") {
        8
    } else if lower.starts_with("http://") {
        7
    } else {
        return url.to_string();
    };
    let (scheme, rest) = trimmed.split_at(scheme_len);
    let (before_fragment, fragment) = match rest.split_once('#') {
        Some((head, tail)) => (head, Some(tail)),
        None => (rest, None),
    };
    let (location, query) = match before_fragment.split_once('?') {
        Some((head, tail)) => (head, Some(tail)),
        None => (before_fragment, None),
    };
    let (authority, path) = match location.find('/') {
        Some(index) => location.split_at(index),
        None => (location, ""),
    };
    // Userinfo (`user:pass@host`) is dropped entirely.
    let host = authority.rsplit('@').next().unwrap_or(authority);
    let path: String = path
        .split('/')
        .map(|segment| {
            if is_opaque_segment(segment) {
                MASK.to_string()
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/");
    let mut out = format!("{scheme}{host}{path}");
    if let Some(query) = query {
        out.push('?');
        out.push_str(&redact_query(query, mask_all_values));
    }
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

/// A magnet link without its trackers (which can carry a passkey) or display name.
fn redact_magnet(magnet: &str) -> String {
    let lower = magnet.to_ascii_lowercase();
    match lower.find("urn:btih:") {
        Some(start) => {
            let hash: String = magnet[start + "urn:btih:".len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            format!("magnet:?xt=urn:btih:{hash}")
        }
        None => "magnet:?…".to_string(),
    }
}

static URL_IN_TEXT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\b(?:https?://|magnet:\?)[^\s"'<>)\]]+"#).expect("url pattern is valid")
});

/// Free text with every URL in it redacted. For messages that may quote an address: HTTP client
/// errors, a failed grab, a log line.
pub fn redact_text(text: &str) -> String {
    URL_IN_TEXT
        .replace_all(text, |caps: &regex::Captures<'_>| {
            redact_url(&caps[0], false)
        })
        .into_owned()
}

/// What to show of a feed address: host, path and parameter *names*; every value hidden.
pub fn feed_url_display(url: &str) -> String {
    redact_url(url, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const IPT: &str =
        "https://iptorrents.com/t.rss?u=123456;tp=0123456789abcdef0123456789abcdef;48;5;download";

    #[test]
    fn an_iptorrents_feed_keeps_its_shape_and_loses_its_passkey() {
        let shown = feed_url_display(IPT);
        assert_eq!(
            shown,
            format!("https://iptorrents.com/t.rss?u={MASK};tp={MASK};48;5;download")
        );
        assert!(!shown.contains("0123456789abcdef"));
        assert!(!shown.contains("123456;"));
        let plain = redact_url(IPT, false);
        assert!(!plain.contains("0123456789abcdef"), "{plain}");
        assert!(plain.contains("48;5;download"));
    }

    #[test]
    fn download_links_hide_torrent_pass_but_keep_ordinary_parameters() {
        let link = "https://tracker.example/download.php/55/some.torrent?torrent_pass=SECRET99&format=rss&id=7";
        let redacted = redact_url(link, false);
        assert!(!redacted.contains("SECRET99"));
        assert!(redacted.contains("format=rss"));
        assert!(redacted.contains("id=7"));
        assert!(redacted.contains(&format!("torrent_pass={MASK}")));
        let all = redact_url(link, true);
        assert!(!all.contains("format=rss"));
    }

    #[test]
    fn opaque_path_segments_and_userinfo_are_hidden() {
        let redacted = redact_url(
            "https://user:hunter2@host.example/rss/a1b2c3d4e5f6a7b8c9d0e1f2a3b4/feed.xml",
            false,
        );
        assert_eq!(
            redacted,
            format!("https://host.example/rss/{MASK}/feed.xml")
        );
        assert!(!redacted.contains("hunter2"));
    }

    #[test]
    fn magnets_keep_only_the_info_hash() {
        let magnet = "magnet:?xt=urn:btih:ABCDEF0123456789ABCDEF0123456789ABCDEF01&dn=Name&tr=https%3A%2F%2Ft.example%2Fannounce%3Fpasskey%3Dzzz";
        assert_eq!(
            redact_url(magnet, false),
            "magnet:?xt=urn:btih:ABCDEF0123456789ABCDEF0123456789ABCDEF01"
        );
    }

    #[test]
    fn text_with_urls_in_it_is_redacted_in_place() {
        let error = format!("error sending request for url ({IPT}): connection reset");
        let cleaned = redact_text(&error);
        assert!(!cleaned.contains("0123456789abcdef"), "{cleaned}");
        assert!(cleaned.contains("connection reset"));
        assert_eq!(redact_text("no addresses here"), "no addresses here");
    }

    #[test]
    fn things_that_are_not_urls_are_left_alone() {
        assert_eq!(redact_url("not a url", false), "not a url");
        assert_eq!(redact_url("ftp://x/y?tp=1", false), "ftp://x/y?tp=1");
    }
}
