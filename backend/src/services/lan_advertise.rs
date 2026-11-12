//! Announces this server on the local network over mDNS as `_librarian._tcp`, so the Android TV
//! app (and anything else on the LAN) can offer it without anyone typing an address
//! (docs/design.md Q138).
//!
//! The record carries the address people should open, which is not always this process's own
//! port: a release build serves the web app itself, a development backend sits behind the Vite
//! server, and a reverse proxy may front either. The address comes from, in order:
//! the `discovery.web_url` setting (Settings › General › Network), `LIBRARIAN_PUBLIC_WEB_URL`
//! (which only seeds that setting, see `bootstrap_defaults`), then `http://<LAN address>:<port>`
//! where the port is this backend's when the web app is embedded and the Vite port (3000)
//! otherwise. `discovery.advertise` turns the announcement off; saving either setting restarts
//! this service through the `AppSetting` hook in `services::database`.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::services::ServicesManager;
use crate::services::manager::{Service, ServiceHealth};
use crate::services::torrent::database::{get_setting, get_setting_string};

/// The DNS-SD service type, with the domain mdns-sd expects.
pub const SERVICE_TYPE: &str = "_librarian._tcp.local.";
/// Name of this service in the services manager.
pub const SERVICE_NAME: &str = "lan_advertise";
/// App setting: whether to announce at all (JSON boolean, default `true`).
pub const ADVERTISE_KEY: &str = "discovery.advertise";
/// App setting: the web address to announce (JSON string, empty for automatic).
pub const WEB_URL_KEY: &str = "discovery.web_url";
/// Port of the Vite dev server that serves the web app when it is not embedded.
pub const DEV_WEB_PORT: u16 = 3000;
/// Version of the TXT record layout below.
const TXT_VERSION: &str = "1";
/// A TXT entry is one length-prefixed string: `key=value` must fit in 255 bytes.
const TXT_ENTRY_MAX: usize = 255;

/// What the service needs from the process configuration.
#[derive(Clone, Debug)]
pub struct LanAdvertiseConfig {
    /// The backend's HTTP port.
    pub port: u16,
    /// The configured bind address (`HOST`), when it names one interface.
    pub host: Option<String>,
    /// Whether this binary serves the web app itself (`embed-frontend`).
    pub web_embedded: bool,
}

impl LanAdvertiseConfig {
    pub fn new(port: u16, host: Option<String>) -> Self {
        Self {
            port,
            host,
            web_embedded: cfg!(feature = "embed-frontend"),
        }
    }
}

/// One announcement: everything that goes into the mDNS record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Advertisement {
    /// DNS-SD instance name, as people see it in a list.
    pub instance: String,
    /// The `.local.` host name the SRV record points at.
    pub host_name: String,
    /// SRV port: the port of `url`.
    pub port: u16,
    /// The web address to open (an origin, no path).
    pub url: String,
    pub name: String,
    pub version: String,
}

impl Advertisement {
    /// TXT entries: `url`, `name`, `version`, `path` and `txtvers`.
    pub fn txt(&self) -> Vec<(String, String)> {
        vec![
            ("txtvers".to_string(), TXT_VERSION.to_string()),
            ("url".to_string(), self.url.clone()),
            ("name".to_string(), self.name.clone()),
            ("version".to_string(), self.version.clone()),
            ("path".to_string(), "/".to_string()),
        ]
    }

    /// The mdns-sd record, with addresses filled in (and kept current) by the daemon.
    pub fn service_info(&self) -> Result<ServiceInfo> {
        let properties: HashMap<String, String> = self.txt().into_iter().collect();
        Ok(ServiceInfo::new(
            SERVICE_TYPE,
            &self.instance,
            &self.host_name,
            (),
            self.port,
            properties,
        )
        .context("Invalid mDNS service record")?
        .enable_addr_auto())
    }
}

/// Checks an address to announce and reduces it to its origin (`scheme://host[:port]`).
/// Only http and https are accepted, without credentials, query or fragment; a path is dropped,
/// as the web app always lives at the root.
pub fn normalize_web_url(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("The web address is empty");
    }
    let parsed = url::Url::parse(trimmed)
        .context("The web address must be a full URL, like http://192.168.1.20:3000")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("The web address must start with http:// or https://");
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        bail!("The web address must not contain a user name or password");
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        bail!("The web address must not contain a query or fragment");
    }
    let host = parsed
        .host_str()
        .filter(|host| !host.is_empty())
        .ok_or_else(|| anyhow!("The web address has no host"))?;
    let origin = match parsed.port() {
        Some(port) => format!("{}://{}:{}", parsed.scheme(), host, port),
        None => format!("{}://{}", parsed.scheme(), host),
    };
    if "url=".len() + origin.len() > TXT_ENTRY_MAX {
        bail!("The web address is too long to announce");
    }
    Ok(origin)
}

/// The port someone opening `url` connects to.
pub fn url_port(url: &str) -> Option<u16> {
    url::Url::parse(url).ok()?.port_or_known_default()
}

/// `http://<address>:<port>` for the address this machine is reached at on the LAN.
pub fn automatic_web_url(config: &LanAdvertiseConfig, lan_ip: Option<IpAddr>) -> Option<String> {
    let configured = config
        .host
        .as_deref()
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .and_then(|host| host.parse::<IpAddr>().ok())
        .filter(|ip| !ip.is_unspecified() && !ip.is_loopback());
    let ip = configured.or(lan_ip.filter(|ip| !ip.is_unspecified() && !ip.is_loopback()))?;
    let port = if config.web_embedded {
        config.port
    } else {
        DEV_WEB_PORT
    };
    Some(match ip {
        IpAddr::V4(v4) => format!("http://{v4}:{port}"),
        IpAddr::V6(v6) => format!("http://[{v6}]:{port}"),
    })
}

/// The `.local.` host name for a machine name: DNS label characters only.
pub fn mdns_host_name(machine: &str) -> String {
    let label: String = machine
        .trim()
        .trim_end_matches('.')
        .trim_end_matches(".local")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let label = label.trim_matches('-');
    let label = if label.is_empty() { "librarian" } else { label };
    let label: String = label.chars().take(63).collect();
    format!("{label}.local.")
}

/// Builds the announcement from the saved address (if any) and the machine's facts.
pub fn build_advertisement(
    config: &LanAdvertiseConfig,
    saved_url: Option<&str>,
    machine: &str,
    lan_ip: Option<IpAddr>,
) -> Result<Advertisement> {
    let url = match saved_url.map(str::trim).filter(|url| !url.is_empty()) {
        Some(url) => normalize_web_url(url)?,
        None => automatic_web_url(config, lan_ip)
            .ok_or_else(|| anyhow!("No LAN address found; set the web address to announce"))?,
    };
    let port = url_port(&url).ok_or_else(|| anyhow!("The web address has no port"))?;
    let machine = machine.trim();
    let name = if machine.is_empty() {
        "Librarian".to_string()
    } else {
        machine.to_string()
    };
    Ok(Advertisement {
        instance: format!("Librarian on {name}"),
        host_name: mdns_host_name(&name),
        port,
        url,
        name,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Checks a write to one of this service's settings and returns the value to store, or `None`
/// for any other key. The address is stored as a JSON string of its origin (empty for
/// automatic); the switch as `true` or `false`. Called from the `AppSetting` write transform,
/// so a bad address is refused to whoever saves it.
pub fn prepare_setting_value(key: &str, value: &str) -> Result<Option<String>> {
    match key {
        WEB_URL_KEY => {
            let decoded = crate::services::torrent::database::decode_setting_string(value);
            let decoded = decoded.trim();
            let stored = if decoded.is_empty() || decoded == "null" {
                String::new()
            } else {
                normalize_web_url(decoded)?
            };
            Ok(Some(serde_json::Value::String(stored).to_string()))
        }
        ADVERTISE_KEY => match value.trim().trim_matches('"') {
            "true" => Ok(Some("true".to_string())),
            "false" => Ok(Some("false".to_string())),
            _ => bail!("Announcing on the network is either true or false"),
        },
        _ => Ok(None),
    }
}

/// Whether a saved setting is one this service reads, so saving it restarts the service.
pub fn is_own_setting(key: &str) -> bool {
    key == ADVERTISE_KEY || key == WEB_URL_KEY
}

struct Announced {
    daemon: ServiceDaemon,
    fullname: String,
}

/// Registers the announcement on start and withdraws it on stop.
pub struct LanAdvertiseService {
    manager: Arc<ServicesManager>,
    config: LanAdvertiseConfig,
    announced: Mutex<Option<Announced>>,
    /// Why nothing is announced, when that is not by choice.
    problem: RwLock<Option<String>>,
}

impl LanAdvertiseService {
    pub fn new(manager: Arc<ServicesManager>, config: LanAdvertiseConfig) -> Self {
        Self {
            manager,
            config,
            announced: Mutex::new(None),
            problem: RwLock::new(None),
        }
    }

    /// Reads the two settings: `(enabled, saved address)`.
    async fn settings(&self) -> Result<(bool, Option<String>)> {
        let database = self
            .manager
            .get_database()
            .await
            .ok_or_else(|| anyhow!("Database unavailable"))?;
        let enabled = get_setting::<bool>(database.pool(), ADVERTISE_KEY)
            .await
            .unwrap_or_else(|error| {
                warn!(service = SERVICE_NAME, error = %error, "Unreadable LAN announcement switch; announcing");
                None
            })
            .unwrap_or(true);
        let url = get_setting_string(database.pool(), WEB_URL_KEY).await?;
        Ok((enabled, url))
    }

    async fn announce(&self) -> Result<()> {
        let (enabled, saved_url) = self.settings().await?;
        if !enabled {
            info!(service = SERVICE_NAME, "LAN announcement is turned off");
            return Ok(());
        }
        let machine = sysinfo::System::host_name().unwrap_or_default();
        let lan_ip = local_ip_address::local_ip().ok();
        let advertisement =
            build_advertisement(&self.config, saved_url.as_deref(), &machine, lan_ip)?;
        let info = advertisement.service_info()?;
        let fullname = info.get_fullname().to_string();
        let daemon = ServiceDaemon::new().context("Could not start the mDNS responder")?;
        if let Err(error) = daemon.register(info) {
            let _ = daemon.shutdown();
            return Err(anyhow!(error).context("Could not register the mDNS service"));
        }
        info!(
            service = SERVICE_NAME,
            instance = %advertisement.instance,
            url = %advertisement.url,
            port = advertisement.port,
            "Announcing Librarian on the LAN as {SERVICE_TYPE}"
        );
        *self.announced.lock().await = Some(Announced { daemon, fullname });
        Ok(())
    }
}

#[async_trait]
impl Service for LanAdvertiseService {
    fn name(&self) -> &str {
        SERVICE_NAME
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string()]
    }

    async fn start(&self) -> Result<()> {
        // A settings change can restart the service before the manager first starts it.
        self.stop().await?;
        // Never fatal: a host without multicast still serves everyone who types the address.
        let outcome = self.announce().await;
        let mut problem = self.problem.write().await;
        match outcome {
            Ok(()) => *problem = None,
            Err(error) => {
                warn!(service = SERVICE_NAME, error = %error, "Not announcing Librarian on the LAN");
                *problem = Some(format!("{error:#}"));
            }
        }
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let Some(Announced { daemon, fullname }) = self.announced.lock().await.take() else {
            return Ok(());
        };
        // Unregistering sends the goodbye packets; wait briefly for them before shutting down.
        tokio::task::spawn_blocking(move || {
            if let Ok(receiver) = daemon.unregister(&fullname) {
                let _ = receiver.recv_timeout(Duration::from_secs(2));
            }
            if let Ok(receiver) = daemon.shutdown() {
                let _ = receiver.recv_timeout(Duration::from_secs(2));
            }
        })
        .await
        .ok();
        debug!(service = SERVICE_NAME, "Withdrew the LAN announcement");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        Ok(match self.problem.read().await.as_ref() {
            Some(problem) => ServiceHealth::degraded(problem.clone()),
            None => ServiceHealth::healthy(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn config(embedded: bool) -> LanAdvertiseConfig {
        LanAdvertiseConfig {
            port: 3001,
            host: None,
            web_embedded: embedded,
        }
    }

    const LAN: Option<IpAddr> = Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)));

    #[test]
    fn automatic_url_uses_the_backend_port_when_the_web_app_is_embedded() {
        assert_eq!(
            automatic_web_url(&config(true), LAN).as_deref(),
            Some("http://192.168.1.20:3001")
        );
    }

    #[test]
    fn automatic_url_uses_the_vite_port_for_a_development_backend() {
        assert_eq!(
            automatic_web_url(&config(false), LAN).as_deref(),
            Some("http://192.168.1.20:3000")
        );
    }

    #[test]
    fn automatic_url_prefers_a_specific_bind_address_and_ignores_loopback() {
        let mut bound = config(true);
        bound.host = Some("10.0.0.5".into());
        assert_eq!(
            automatic_web_url(&bound, LAN).as_deref(),
            Some("http://10.0.0.5:3001")
        );
        bound.host = Some("0.0.0.0".into());
        assert_eq!(
            automatic_web_url(&bound, LAN).as_deref(),
            Some("http://192.168.1.20:3001")
        );
        assert_eq!(
            automatic_web_url(&config(true), Some(IpAddr::V4(Ipv4Addr::LOCALHOST))),
            None
        );
        let v6 = Some(IpAddr::V6(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 7)));
        assert_eq!(
            automatic_web_url(&config(true), v6).as_deref(),
            Some("http://[fd00::7]:3001")
        );
    }

    #[test]
    fn saved_urls_are_reduced_to_their_origin() {
        assert_eq!(
            normalize_web_url(" https://media.example.net/library/ ").unwrap(),
            "https://media.example.net"
        );
        assert_eq!(
            normalize_web_url("http://192.168.1.20:3000").unwrap(),
            "http://192.168.1.20:3000"
        );
        assert!(normalize_web_url("ftp://host").is_err());
        assert!(normalize_web_url("http://user:pw@host").is_err());
        assert!(normalize_web_url("http://host/?a=1").is_err());
        assert!(normalize_web_url("192.168.1.20:3000").is_err());
        assert!(normalize_web_url("").is_err());
        let long = format!("https://{}.example", "a".repeat(260));
        assert!(normalize_web_url(&long).is_err());
    }

    #[test]
    fn the_srv_port_is_the_port_people_open() {
        let saved = build_advertisement(
            &config(false),
            Some("https://tv.example.net"),
            "media-box",
            LAN,
        )
        .unwrap();
        assert_eq!(saved.port, 443);
        assert_eq!(saved.url, "https://tv.example.net");
        let auto = build_advertisement(&config(true), None, "media-box", LAN).unwrap();
        assert_eq!(auto.port, 3001);
        assert_eq!(auto.url, "http://192.168.1.20:3001");
        let blank = build_advertisement(&config(true), Some("  "), "media-box", LAN).unwrap();
        assert_eq!(blank.url, auto.url);
        assert!(build_advertisement(&config(true), None, "media-box", None).is_err());
    }

    #[test]
    fn the_record_carries_url_name_and_version_in_txt() {
        let advertisement = build_advertisement(&config(true), None, "Media Box", LAN).unwrap();
        assert_eq!(advertisement.instance, "Librarian on Media Box");
        assert_eq!(advertisement.host_name, "media-box.local.");
        let info = advertisement.service_info().unwrap();
        assert_eq!(info.get_type(), SERVICE_TYPE);
        assert_eq!(info.get_port(), 3001);
        assert_eq!(
            info.get_fullname(),
            "Librarian on Media Box._librarian._tcp.local."
        );
        assert_eq!(info.get_hostname(), "media-box.local.");
        assert_eq!(
            info.get_property_val_str("url"),
            Some("http://192.168.1.20:3001")
        );
        assert_eq!(info.get_property_val_str("name"), Some("Media Box"));
        assert_eq!(
            info.get_property_val_str("version"),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(info.get_property_val_str("txtvers"), Some("1"));
        assert_eq!(info.get_property_val_str("path"), Some("/"));
        assert!(info.is_addr_auto());
    }

    #[test]
    fn settings_are_checked_and_stored_as_json() {
        assert_eq!(
            prepare_setting_value(WEB_URL_KEY, "\"http://192.168.1.20:3000/\"").unwrap(),
            Some("\"http://192.168.1.20:3000\"".to_string())
        );
        assert_eq!(
            prepare_setting_value(WEB_URL_KEY, "https://tv.example.net").unwrap(),
            Some("\"https://tv.example.net\"".to_string())
        );
        assert_eq!(
            prepare_setting_value(WEB_URL_KEY, "").unwrap(),
            Some("\"\"".to_string())
        );
        assert_eq!(
            prepare_setting_value(WEB_URL_KEY, "null").unwrap(),
            Some("\"\"".to_string())
        );
        assert!(prepare_setting_value(WEB_URL_KEY, "\"tv.local\"").is_err());
        assert_eq!(
            prepare_setting_value(ADVERTISE_KEY, "false").unwrap(),
            Some("false".to_string())
        );
        assert!(prepare_setting_value(ADVERTISE_KEY, "maybe").is_err());
        assert_eq!(
            prepare_setting_value("torrent.listen_port", "1").unwrap(),
            None
        );
        assert!(is_own_setting(WEB_URL_KEY) && !is_own_setting("llm.enabled"));
    }

    #[test]
    fn host_names_are_dns_labels() {
        assert_eq!(mdns_host_name("Media Box"), "media-box.local.");
        assert_eq!(mdns_host_name("nas.local"), "nas.local.");
        assert_eq!(mdns_host_name("  "), "librarian.local.");
        assert_eq!(mdns_host_name("Tóby's PC"), "t-by-s-pc.local.");
    }
}
