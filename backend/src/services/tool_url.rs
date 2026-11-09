//! Loopback URLs that let external tools (ffprobe, ffmpeg, fpcalc) read a file on a share.
//!
//! A tool opens a path; a file on a network share has none. For such a file the backend issues a
//! URL on its own address (`/internal/tool/<token>/<file name>`, served by `api::tool`) that reads
//! the file through the storage layer, with `Range` support so the tool can seek.
//!
//! The token is random, names exactly one file (the path was resolved by the backend, never
//! supplied by a client), expires after a period without use, and is only honoured for a peer on
//! the loopback address (or the backend's own address, when it is bound to a single one).

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use uuid::Uuid;

use crate::services::library_storage::{
    StorageError, StorageErrorKind, StoragePath, StorageResult, ToolUrlIssuer,
};

/// A URL is dropped after this long without a request. A tool that is still reading keeps its
/// URL alive, because every request counts as use.
const IDLE_LIFETIME: Duration = Duration::from_secs(30 * 60);

/// Path prefix of every tool URL.
pub const TOOL_URL_PATH: &str = "/internal/tool/";

struct Grant {
    path: StoragePath,
    last_used: Instant,
}

#[derive(Clone, Copy)]
struct Origin {
    /// The address tools are told to connect to.
    host: IpAddr,
    port: u16,
}

#[derive(Default)]
pub struct ToolUrlService {
    origin: RwLock<Option<Origin>>,
    grants: Mutex<HashMap<String, Grant>>,
}

/// The process-wide service.
pub fn tool_urls() -> &'static ToolUrlService {
    static SERVICE: OnceLock<ToolUrlService> = OnceLock::new();
    SERVICE.get_or_init(ToolUrlService::default)
}

impl ToolUrlService {
    /// Say where the HTTP server listens. An unspecified bind address (all interfaces) is
    /// reached through loopback.
    pub fn configure(&self, bind: IpAddr, port: u16) {
        let host = if bind.is_unspecified() {
            if bind.is_ipv4() {
                IpAddr::from([127, 0, 0, 1])
            } else {
                IpAddr::from(std::net::Ipv6Addr::LOCALHOST)
            }
        } else {
            bind
        };
        if let Ok(mut origin) = self.origin.write() {
            *origin = Some(Origin { host, port });
        }
    }

    /// Whether a connection from `peer` may use a tool URL.
    pub fn peer_allowed(&self, peer: IpAddr) -> bool {
        let peer = match peer {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
            other => other,
        };
        if peer.is_loopback() {
            return true;
        }
        self.origin
            .read()
            .ok()
            .and_then(|origin| *origin)
            .is_some_and(|origin| origin.host == peer)
    }

    /// A URL that reads `path` until it has gone unused for a while.
    pub fn issue_url(&self, path: &StoragePath) -> StorageResult<String> {
        let origin = self
            .origin
            .read()
            .ok()
            .and_then(|origin| *origin)
            .ok_or_else(|| {
                StorageError::new(
                    StorageErrorKind::Unavailable,
                    "The tool URL service is not listening yet",
                )
            })?;
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let name = path.file_name().unwrap_or_else(|| "file".to_string());
        {
            let mut grants = self.grants.lock().map_err(|_| {
                StorageError::new(StorageErrorKind::Other, "tool URL table is poisoned")
            })?;
            grants.retain(|_, grant| grant.last_used.elapsed() < IDLE_LIFETIME);
            grants.insert(
                token.clone(),
                Grant {
                    path: path.clone(),
                    last_used: Instant::now(),
                },
            );
        }
        let host = match origin.host {
            IpAddr::V4(v4) => v4.to_string(),
            IpAddr::V6(v6) => format!("[{v6}]"),
        };
        Ok(format!(
            "http://{host}:{}{TOOL_URL_PATH}{token}/{}",
            origin.port,
            urlencoding::encode(&name)
        ))
    }

    /// The file a token names, counting the request as use. `None` for an unknown or idle token.
    pub fn resolve(&self, token: &str) -> Option<StoragePath> {
        let mut grants = self.grants.lock().ok()?;
        let grant = grants.get_mut(token)?;
        if grant.last_used.elapsed() >= IDLE_LIFETIME {
            grants.remove(token);
            return None;
        }
        grant.last_used = Instant::now();
        Some(grant.path.clone())
    }

    /// Withdraw every URL for `path` (its tool has finished, or the file went away).
    pub fn revoke_path(&self, path: &StoragePath) {
        if let Ok(mut grants) = self.grants.lock() {
            grants.retain(|_, grant| &grant.path != path);
        }
    }
}

/// The process-wide service, as the issuer the storage registry asks.
pub struct GlobalToolUrls;

#[async_trait]
impl ToolUrlIssuer for GlobalToolUrls {
    async fn issue(&self, path: &StoragePath) -> StorageResult<String> {
        tool_urls().issue_url(path)
    }
}

#[async_trait]
impl ToolUrlIssuer for ToolUrlService {
    async fn issue(&self, path: &StoragePath) -> StorageResult<String> {
        self.issue_url(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> ToolUrlService {
        let service = ToolUrlService::default();
        service.configure(IpAddr::from([0, 0, 0, 0]), 3001);
        service
    }

    #[test]
    fn a_url_names_one_file_and_keeps_its_extension() {
        let service = service();
        let path = StoragePath::new("smb://nas/media/Movies/Film (2020)/Film.mkv");
        let url = service.issue_url(&path).unwrap();
        assert!(
            url.starts_with("http://127.0.0.1:3001/internal/tool/"),
            "{url}"
        );
        assert!(url.ends_with("/Film.mkv"), "{url}");
        let token = url
            .strip_prefix("http://127.0.0.1:3001/internal/tool/")
            .and_then(|rest| rest.split('/').next())
            .unwrap();
        assert!(token.len() >= 64);
        assert_eq!(service.resolve(token), Some(path));
        assert_eq!(service.resolve("not-a-token"), None);
    }

    #[test]
    fn each_url_has_its_own_token() {
        let service = service();
        let path = StoragePath::new("smb://nas/media/a.mkv");
        assert_ne!(
            service.issue_url(&path).unwrap(),
            service.issue_url(&path).unwrap()
        );
    }

    #[test]
    fn odd_file_names_are_encoded_into_one_path_segment() {
        let service = service();
        let url = service
            .issue_url(&StoragePath::new("smb://nas/media/Film #1 & 100%.mkv"))
            .unwrap();
        let name = url.rsplit('/').next().unwrap();
        assert!(
            !name.contains(' ') && !name.contains('#') && !name.contains('&'),
            "{name}"
        );
        assert!(name.ends_with(".mkv"));
    }

    #[test]
    fn only_loopback_peers_may_use_a_url() {
        let service = service();
        assert!(service.peer_allowed("127.0.0.1".parse().unwrap()));
        assert!(service.peer_allowed("::1".parse().unwrap()));
        assert!(service.peer_allowed("::ffff:127.0.0.1".parse().unwrap()));
        assert!(!service.peer_allowed("10.0.1.50".parse().unwrap()));
        assert!(!service.peer_allowed("192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn a_server_bound_to_one_address_trusts_itself_at_that_address() {
        let service = ToolUrlService::default();
        service.configure("10.0.1.203".parse().unwrap(), 3001);
        assert!(service.peer_allowed("10.0.1.203".parse().unwrap()));
        assert!(!service.peer_allowed("10.0.1.50".parse().unwrap()));
        let url = service
            .issue_url(&StoragePath::new("smb://nas/media/a.mkv"))
            .unwrap();
        assert!(url.starts_with("http://10.0.1.203:3001/"), "{url}");
    }

    #[test]
    fn a_revoked_file_loses_its_urls() {
        let service = service();
        let path = StoragePath::new("smb://nas/media/a.mkv");
        let other = StoragePath::new("smb://nas/media/b.mkv");
        let url = service.issue_url(&path).unwrap();
        let kept = service.issue_url(&other).unwrap();
        service.revoke_path(&path);
        let token = |url: &str| url.split('/').nth(5).unwrap().to_string();
        assert_eq!(service.resolve(&token(&url)), None);
        assert_eq!(service.resolve(&token(&kept)), Some(other));
    }

    #[test]
    fn nothing_is_issued_before_the_server_is_configured() {
        let service = ToolUrlService::default();
        assert!(
            service
                .issue_url(&StoragePath::new("smb://nas/media/a.mkv"))
                .is_err()
        );
    }
}
