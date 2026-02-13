use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::net::lookup_host;
use tokio::process::Command;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

use crate::db::Database;
use crate::services::manager::ServicesManager;

const CONFIG_KEY_PREFIX: &str = "network_mount.";
const NETWORK_COMMAND_TIMEOUT_SECS: u64 = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredNetworkPathConfig {
    pub target_path: String,
    pub remote_path: String,
    pub username: Option<String>,
    pub password_encrypted: Option<String>,
    pub platform: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ConfigureNetworkPathInput {
    pub path: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub mount_point: Option<String>,
    pub persist: bool,
    pub attempt_connect: bool,
}

#[derive(Debug, Clone)]
pub struct ConfigureNetworkPathResult {
    pub success: bool,
    pub error: Option<String>,
    pub resolved_path: String,
    pub connected: bool,
    pub stored: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PathAvailability {
    pub path: String,
    pub reachable: bool,
    pub exists: bool,
    pub is_directory: bool,
    pub needs_reconnect: bool,
    pub reconnect_attempted: bool,
    pub reconnect_succeeded: bool,
    pub message: Option<String>,
}

pub fn current_platform() -> String {
    if cfg!(windows) {
        "windows".to_string()
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else if cfg!(target_os = "macos") {
        "macos".to_string()
    } else {
        "other".to_string()
    }
}

pub fn supports_unc_credentials() -> bool {
    cfg!(windows)
}

pub fn supports_samba_mount() -> bool {
    cfg!(target_os = "linux")
}

pub fn default_mount_base() -> Option<&'static str> {
    if cfg!(target_os = "linux") {
        Some("/mnt")
    } else {
        None
    }
}

pub fn is_unc_path(path: &str) -> bool {
    let trimmed = path.trim();
    trimmed.starts_with("\\\\") || trimmed.starts_with("//")
}

fn normalize_path_key(path: &str) -> String {
    let normalized = path.trim().to_ascii_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{}{:x}", CONFIG_KEY_PREFIX, hasher.finalize())
}

fn normalize_unc_for_windows(path: &str) -> String {
    let p = path.trim().replace('/', "\\");
    if p.starts_with("\\\\") {
        p
    } else if let Some(stripped) = p.strip_prefix("\\") {
        format!("\\\\{}", stripped)
    } else {
        format!("\\\\{}", p.trim_start_matches('\\'))
    }
}

fn normalize_unc_for_linux(path: &str) -> String {
    let p = path.trim().replace('\\', "/");
    if p.starts_with("//") {
        p
    } else if let Some(stripped) = p.strip_prefix('/') {
        format!("//{}", stripped)
    } else {
        format!("//{}", p.trim_start_matches('/'))
    }
}

fn split_unc_linux(path: &str) -> Option<(String, String)> {
    let normalized = normalize_unc_for_linux(path);
    let trimmed = normalized.trim_start_matches('/');
    let mut parts = trimmed.splitn(2, '/');
    let host = parts.next()?.trim().to_string();
    let rest = parts.next()?.trim().to_string();
    if host.is_empty() || rest.is_empty() {
        return None;
    }
    Some((host, rest))
}

async fn resolve_unc_host_for_linux(path: &str) -> String {
    let Some((host, rest)) = split_unc_linux(path) else {
        return normalize_unc_for_linux(path);
    };

    let host_is_ip = host.parse::<std::net::IpAddr>().is_ok();
    if host_is_ip {
        return format!("//{}/{}", host, rest);
    }

    match lookup_host((host.as_str(), 445)).await {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                let ip = addr.ip();
                tracing::info!(
                    host = %host,
                    resolved_ip = %ip,
                    "Resolved UNC hostname for CIFS mount"
                );
                format!("//{}/{}", ip, rest)
            } else {
                normalize_unc_for_linux(path)
            }
        }
        Err(_) => normalize_unc_for_linux(path),
    }
}

fn default_mount_point_from_remote(remote_path: &str) -> String {
    let digest = normalize_path_key(remote_path);
    let suffix = digest
        .rsplit('.')
        .next()
        .unwrap_or("share")
        .chars()
        .take(12)
        .collect::<String>();
    format!("/mnt/librarian-{}", suffix)
}

async fn get_password_cipher(
    services: &Arc<ServicesManager>,
    password: Option<&str>,
) -> Result<Option<String>> {
    let Some(password) = password.filter(|p| !p.is_empty()) else {
        return Ok(None);
    };

    let sources_service = services
        .get_sources()
        .await
        .ok_or_else(|| anyhow!("Sources service not available for credential encryption"))?;

    let manager = sources_service
        .get_manager()
        .await
        .ok_or_else(|| anyhow!("Sources manager not initialized for credential encryption"))?;

    let (cipher, nonce) = manager
        .encryption()
        .encrypt(password)
        .context("Failed to encrypt network password")?;

    Ok(Some(format!("{}:{}", nonce, cipher)))
}

async fn decrypt_password_cipher(
    services: &Arc<ServicesManager>,
    password_encrypted: Option<&str>,
) -> Result<Option<String>> {
    let Some(password_encrypted) = password_encrypted.filter(|p| !p.is_empty()) else {
        return Ok(None);
    };

    let mut parts = password_encrypted.splitn(2, ':');
    let nonce = parts
        .next()
        .ok_or_else(|| anyhow!("Invalid encrypted password format"))?;
    let cipher = parts
        .next()
        .ok_or_else(|| anyhow!("Invalid encrypted password format"))?;

    let sources_service = services
        .get_sources()
        .await
        .ok_or_else(|| anyhow!("Sources service not available for credential decryption"))?;

    let manager = sources_service
        .get_manager()
        .await
        .ok_or_else(|| anyhow!("Sources manager not initialized for credential decryption"))?;

    let password = manager
        .encryption()
        .decrypt(cipher, nonce)
        .context("Failed to decrypt network password")?;

    Ok(Some(password))
}

async fn upsert_network_config(db: &Database, cfg: &StoredNetworkPathConfig) -> Result<()> {
    let key = normalize_path_key(&cfg.target_path);
    let value = serde_json::to_string(cfg)?;

    let updated = sqlx::query(
        "UPDATE app_settings SET value = ?, category = ?, description = ?, updated_at = ? WHERE key = ?",
    )
    .bind(&value)
    .bind("filesystem")
    .bind("Persisted network path configuration")
    .bind(&cfg.updated_at)
    .bind(&key)
    .execute(db)
    .await?
    .rows_affected();

    if updated == 0 {
        let now = cfg.updated_at.clone();
        sqlx::query(
            "INSERT INTO app_settings (id, key, value, description, category, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&key)
        .bind(&value)
        .bind("Persisted network path configuration")
        .bind("filesystem")
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;
    }

    Ok(())
}

pub async fn load_saved_network_configs(db: &Database) -> Result<Vec<StoredNetworkPathConfig>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM app_settings WHERE key LIKE ? ORDER BY updated_at DESC, created_at DESC, rowid DESC",
    )
    .bind(format!("{}%", CONFIG_KEY_PREFIX))
    .fetch_all(db)
    .await?;

    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for (key, value) in rows {
        if !seen.insert(key) {
            continue;
        }

        if let Ok(cfg) = serde_json::from_str::<StoredNetworkPathConfig>(&value) {
            out.push(cfg);
        }
    }

    Ok(out)
}

async fn find_config_for_target(
    db: &Database,
    target_path: &str,
) -> Result<Option<StoredNetworkPathConfig>> {
    let key = normalize_path_key(target_path);
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM app_settings WHERE key = ? ORDER BY updated_at DESC, created_at DESC, rowid DESC LIMIT 1",
    )
    .bind(key)
    .fetch_optional(db)
    .await?;

    if let Some((value,)) = row {
        Ok(serde_json::from_str::<StoredNetworkPathConfig>(&value).ok())
    } else {
        Ok(None)
    }
}

async fn run_command(cmd: &str, args: &[String]) -> Result<String> {
    let out = timeout(
        Duration::from_secs(NETWORK_COMMAND_TIMEOUT_SECS),
        Command::new(cmd).args(args).output(),
    )
    .await
    .with_context(|| format!("Command '{}' timed out", cmd))?
    .with_context(|| format!("Failed to execute '{}'", cmd))?;

    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(anyhow!(
            "{}",
            String::from_utf8_lossy(&out.stderr).to_string()
        ))
    }
}

async fn try_mount_cifs(
    remote_for_mount: &str,
    target_path: &str,
    base_opts: &[String],
) -> Result<()> {
    let dialects = ["3.1.1", "3.0", "2.1", "2.0", "1.0"];
    let mut errors = Vec::new();

    for dialect in dialects {
        let mut opts = base_opts.to_vec();
        opts.push(format!("vers={}", dialect));
        let args = vec![
            "-t".to_string(),
            "cifs".to_string(),
            remote_for_mount.to_string(),
            target_path.to_string(),
            "-o".to_string(),
            opts.join(","),
        ];

        match run_command("mount", &args).await {
            Ok(_) => {
                tracing::info!(
                    remote = %remote_for_mount,
                    target = %target_path,
                    dialect = %dialect,
                    "CIFS mount succeeded"
                );
                return Ok(());
            }
            Err(e) => {
                let msg = e.to_string();
                errors.push(format!("vers={}: {}", dialect, msg.trim()));
                tracing::warn!(
                    remote = %remote_for_mount,
                    target = %target_path,
                    dialect = %dialect,
                    error = %msg,
                    "CIFS mount attempt failed"
                );
            }
        }
    }

    Err(anyhow!(
        "All SMB dialect attempts failed: {}",
        errors.join(" | ")
    ))
}

async fn ensure_connected(
    remote_path: &str,
    target_path: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<()> {
    if cfg!(windows) {
        if !is_unc_path(remote_path) {
            return Ok(());
        }

        let mut args = vec!["use".to_string(), normalize_unc_for_windows(remote_path)];
        if let Some(user) = username.filter(|u| !u.is_empty()) {
            if let Some(pass) = password {
                args.push(pass.to_string());
            }
            args.push(format!("/user:{}", user));
        }
        args.push("/persistent:yes".to_string());

        let _ = run_command("net", &args).await?;
        return Ok(());
    }

    if cfg!(target_os = "linux") {
        if !is_unc_path(remote_path) {
            return Ok(());
        }

        let mount_point = Path::new(target_path);
        if !mount_point.exists() {
            fs::create_dir_all(mount_point)
                .await
                .with_context(|| format!("Failed to create mount point '{}'", target_path))?;
        }

        let already_mounted = Command::new("mountpoint")
            .arg("-q")
            .arg(target_path)
            .output()
            .await
            .ok()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if already_mounted {
            return Ok(());
        }

        let mut opts = vec!["iocharset=utf8".to_string(), "noperm".to_string()];
        if let Some(user) = username.filter(|u| !u.is_empty()) {
            opts.push(format!("username={}", user));
        }
        if let Some(pass) = password {
            opts.push(format!("password={}", pass));
        }

        let remote_for_mount = resolve_unc_host_for_linux(remote_path).await;
        try_mount_cifs(&remote_for_mount, target_path, &opts).await?;
        return Ok(());
    }

    Ok(())
}

pub async fn configure_network_path(
    db: &Database,
    services: &Arc<ServicesManager>,
    input: ConfigureNetworkPathInput,
) -> ConfigureNetworkPathResult {
    let requested_path = input.path.trim();
    if requested_path.is_empty() {
        return ConfigureNetworkPathResult {
            success: false,
            error: Some("Path is required".to_string()),
            resolved_path: String::new(),
            connected: false,
            stored: false,
            message: None,
        };
    }

    let platform = current_platform();
    let is_network = is_unc_path(requested_path);

    let remote_path = requested_path.to_string();
    let resolved_path = if cfg!(target_os = "linux") && is_network {
        input
            .mount_point
            .as_deref()
            .filter(|m| !m.trim().is_empty())
            .map(|m| m.trim().to_string())
            .unwrap_or_else(|| default_mount_point_from_remote(&remote_path))
    } else {
        requested_path.to_string()
    };

    let mut connected = false;
    let mut message = None;
    if input.attempt_connect && is_network {
        match ensure_connected(
            &remote_path,
            &resolved_path,
            input.username.as_deref(),
            input.password.as_deref(),
        )
        .await
        {
            Ok(_) => {
                connected = true;
                message = Some("Network path connected".to_string());
            }
            Err(e) => {
                return ConfigureNetworkPathResult {
                    success: false,
                    error: Some(format!("Failed to connect network path: {}", e)),
                    resolved_path,
                    connected: false,
                    stored: false,
                    message: None,
                };
            }
        }
    }

    let mut stored = false;
    if input.persist && is_network {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let password_encrypted =
            match get_password_cipher(services, input.password.as_deref()).await {
                Ok(v) => v,
                Err(e) => {
                    return ConfigureNetworkPathResult {
                        success: false,
                        error: Some(format!("Failed to store credentials securely: {}", e)),
                        resolved_path,
                        connected,
                        stored: false,
                        message,
                    };
                }
            };

        let cfg = StoredNetworkPathConfig {
            target_path: resolved_path.clone(),
            remote_path,
            username: input.username.filter(|u| !u.trim().is_empty()),
            password_encrypted,
            platform,
            created_at: now.clone(),
            updated_at: now,
        };

        if let Err(e) = upsert_network_config(db, &cfg).await {
            return ConfigureNetworkPathResult {
                success: false,
                error: Some(format!("Failed to persist network path config: {}", e)),
                resolved_path,
                connected,
                stored: false,
                message,
            };
        }
        stored = true;
    }

    ConfigureNetworkPathResult {
        success: true,
        error: None,
        resolved_path,
        connected,
        stored,
        message,
    }
}

pub async fn reconnect_target_path(
    db: &Database,
    services: &Arc<ServicesManager>,
    target_path: &str,
) -> Result<bool> {
    let Some(cfg) = find_config_for_target(db, target_path).await? else {
        return Ok(false);
    };

    let password = decrypt_password_cipher(services, cfg.password_encrypted.as_deref()).await?;

    ensure_connected(
        &cfg.remote_path,
        &cfg.target_path,
        cfg.username.as_deref(),
        password.as_deref(),
    )
    .await?;

    Ok(true)
}

pub async fn check_path_availability(
    db: &Database,
    services: &Arc<ServicesManager>,
    path: &str,
    attempt_reconnect: bool,
) -> PathAvailability {
    let path = path.trim().to_string();
    if path.is_empty() {
        return PathAvailability {
            path,
            reachable: false,
            exists: false,
            is_directory: false,
            needs_reconnect: false,
            reconnect_attempted: false,
            reconnect_succeeded: false,
            message: Some("Path is empty".to_string()),
        };
    }

    let metadata = fs::metadata(&path).await;
    if let Ok(md) = metadata {
        return PathAvailability {
            path,
            reachable: true,
            exists: true,
            is_directory: md.is_dir(),
            needs_reconnect: false,
            reconnect_attempted: false,
            reconnect_succeeded: false,
            message: None,
        };
    }

    let mut availability = PathAvailability {
        path: path.clone(),
        reachable: false,
        exists: false,
        is_directory: false,
        needs_reconnect: find_config_for_target(db, &path)
            .await
            .ok()
            .flatten()
            .is_some(),
        reconnect_attempted: false,
        reconnect_succeeded: false,
        message: Some("Path is not reachable".to_string()),
    };

    if attempt_reconnect && availability.needs_reconnect {
        availability.reconnect_attempted = true;
        match reconnect_target_path(db, services, &path).await {
            Ok(true) => {
                let metadata_after = fs::metadata(&path).await;
                if let Ok(md) = metadata_after {
                    availability.reachable = true;
                    availability.exists = true;
                    availability.is_directory = md.is_dir();
                    availability.reconnect_succeeded = true;
                    availability.message = Some("Reconnected successfully".to_string());
                } else {
                    availability.message = Some(
                        "Reconnect command succeeded but path is still unavailable".to_string(),
                    );
                }
            }
            Ok(false) => {
                availability.message = Some("No saved network config for this path".to_string());
            }
            Err(e) => {
                availability.message = Some(format!("Reconnect failed: {}", e));
            }
        }
    }

    availability
}

pub async fn reconnect_saved_network_paths(db: &Database, services: &Arc<ServicesManager>) {
    let configs = match load_saved_network_configs(db).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to load saved network paths at startup");
            return;
        }
    };

    if configs.is_empty() {
        return;
    }

    for cfg in configs {
        let password =
            match decrypt_password_cipher(services, cfg.password_encrypted.as_deref()).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        path = %cfg.target_path,
                        error = %e,
                        "Failed to decrypt saved network path credentials"
                    );
                    continue;
                }
            };

        if let Err(e) = ensure_connected(
            &cfg.remote_path,
            &cfg.target_path,
            cfg.username.as_deref(),
            password.as_deref(),
        )
        .await
        {
            tracing::warn!(
                path = %cfg.target_path,
                error = %e,
                "Failed to reconnect saved network path"
            );
        } else {
            tracing::info!(path = %cfg.target_path, "Reconnected saved network path");
        }
    }
}
