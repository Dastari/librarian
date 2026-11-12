//! Saved file servers (`NetworkServer`, `docs/design.md` Q102): what a location on a share signs
//! in with, which libraries and download folders use a server, and carrying over the sign-ins
//! saved before servers were an entity.
//!
//! A location is still one `smb://host[:port]/share/folder` string. The server for it is the
//! saved `NetworkServer` with the same host and port; its encrypted password is decrypted only
//! here, in memory, to sign in, and is never logged, returned or shown.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use serde_json::json;

use crate::db::Database;
use crate::graphql::entities::{
    AppSetting, AppSettingWhereInput, Library, NetworkServer, NetworkServerWhereInput, Torrent,
    TorrentWhereInput,
};
use crate::services::graphql::LibrarianSchema;
use crate::services::library_storage::{SmbCredentials, SmbLocation, StorageKind, StorageLocation};
use crate::services::manager::ServicesManager;
use graphql_orm::graphql::filters::StringFilter;

/// The standard SMB port; a server on it is stored without one.
pub const DEFAULT_PORT: u16 = 445;

/// A host as Librarian keeps it: trimmed, lower case, without `smb://`, slashes or a port.
pub fn normalize_host(raw: &str) -> Result<String, String> {
    let host = raw
        .trim()
        .trim_start_matches("smb://")
        .trim_start_matches(r"\\")
        .trim_start_matches("//")
        .trim_end_matches(['/', '\\'])
        .to_ascii_lowercase();
    if host.is_empty() {
        return Err("Enter the server's name or IP address".to_string());
    }
    if host.contains(['/', '\\', ' ', '@']) {
        return Err(format!(
            "'{}' is not a server name or IP address. Enter only the server (for example 10.0.1.190), and choose the share separately",
            raw.trim()
        ));
    }
    if host.contains(':') && host.parse::<std::net::Ipv6Addr>().is_err() {
        return Err("Put the port in the Port box, not after the server name".to_string());
    }
    Ok(host)
}

/// A port as Librarian keeps it: `None` for the standard port.
pub fn normalize_port(port: Option<i32>) -> Result<Option<i32>, String> {
    match port {
        None => Ok(None),
        Some(port) if port == i32::from(DEFAULT_PORT) => Ok(None),
        Some(port) if (1..=65535).contains(&port) => Ok(Some(port)),
        Some(port) => Err(format!("{port} is not a port number (1 to 65535)")),
    }
}

/// Share names lower-cased (as `smb://` locations keep them), without blanks or repeats.
pub fn normalize_shares(shares: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for share in shares {
        let share = share.trim().trim_matches(['/', '\\']).to_ascii_lowercase();
        if !share.is_empty() && !share.contains(['/', '\\']) && !out.contains(&share) {
            out.push(share);
        }
    }
    out.sort();
    out
}

/// The host and port of a location on a share, if it is one.
pub fn server_of(path: &str) -> Option<(String, Option<u16>)> {
    let location = StorageLocation::parse(path).ok()?;
    if location.kind() != StorageKind::Smb {
        return None;
    }
    let smb = location.root_path().as_smb()?;
    Some((smb.host, smb.port))
}

fn port_of(server: &NetworkServer) -> Option<u16> {
    server.port.and_then(|port| u16::try_from(port).ok())
}

fn same_server(path: &str, host: &str, port: Option<u16>) -> bool {
    server_of(path).is_some_and(|(h, p)| {
        h == host && p.unwrap_or(DEFAULT_PORT) == port.unwrap_or(DEFAULT_PORT)
    })
}

fn eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

/// The saved server for a host and port.
pub async fn find_server(
    db: &Database,
    host: &str,
    port: Option<u16>,
) -> Result<Option<NetworkServer>> {
    let rows = NetworkServer::query(db.pool())
        .filter(NetworkServerWhereInput {
            host: Some(eq(&host.to_ascii_lowercase())),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    Ok(rows
        .into_iter()
        .find(|server| port_of(server).unwrap_or(DEFAULT_PORT) == port.unwrap_or(DEFAULT_PORT)))
}

/// What still uses a server, in words: its libraries, the download folder and torrents stored on
/// it. Empty when nothing does.
pub async fn server_usage(db: &Database, host: &str, port: Option<u16>) -> Result<Vec<String>> {
    let mut uses = Vec::new();
    let libraries: Vec<String> = Library::query(db.pool())
        .filter(crate::graphql::entities::LibraryWhereInput {
            path: Some(StringFilter {
                starts_with: Some("smb://".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .filter(|library| same_server(&library.path, host, port))
        .map(|library| library.name)
        .collect();
    match libraries.len() {
        0 => {}
        1 => uses.push(format!("the library {}", libraries[0])),
        _ => uses.push(format!("the libraries {}", libraries.join(", "))),
    }
    let download_dir = AppSetting::query(db.pool())
        .filter(AppSettingWhereInput {
            key: Some(eq("torrent.download_dir")),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .next()
        .map(|setting| crate::services::torrent::database::decode_setting_string(&setting.value));
    if download_dir
        .as_deref()
        .is_some_and(|dir| same_server(dir, host, port))
    {
        uses.push("the download folder".to_string());
    }
    let torrents = Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            save_path: Some(StringFilter {
                starts_with: Some("smb://".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .filter(|torrent| same_server(&torrent.save_path, host, port))
        .count();
    if torrents > 0 {
        uses.push(format!(
            "{torrents} {}",
            if torrents == 1 { "torrent" } else { "torrents" }
        ));
    }
    Ok(uses)
}

/// "a, b and c".
pub fn join_words(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

async fn encryption(
    services: &Arc<ServicesManager>,
) -> Result<crate::services::sources::encryption::CredentialEncryption> {
    // At start a share can be used (by a restored torrent) a moment before the key is loaded.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Some(sources) = services.get_sources().await
            && let Some(manager) = sources.get_manager().await
        {
            return Ok(manager.encryption().clone());
        }
        if std::time::Instant::now() >= deadline {
            return Err(anyhow!(
                "The credential key is not loaded, so the saved sign-in cannot be read yet"
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

/// The sign-in saved on a server, decrypted for one connection.
pub async fn credentials_of(
    services: &Arc<ServicesManager>,
    server: &NetworkServer,
) -> Result<SmbCredentials> {
    let password = if server.password.is_empty() {
        String::new()
    } else {
        encryption(services)
            .await?
            .reveal(&server.password)
            .context("The saved password could not be decrypted")?
    };
    Ok(SmbCredentials {
        username: server.username.clone().unwrap_or_default(),
        password,
    })
}

/// Make sure a share name is among the server's known shares.
pub async fn remember_share(schema: &LibrarianSchema, server: &NetworkServer, share: &str) {
    remember_shares(schema, server, &[share.to_string()]).await;
}

/// Make sure these share names are among the server's known shares, in one write.
pub async fn remember_shares(schema: &LibrarianSchema, server: &NetworkServer, names: &[String]) {
    let mut shares = server.known_shares.clone();
    shares.extend(names.iter().cloned());
    let shares = normalize_shares(&shares);
    if shares == normalize_shares(&server.known_shares) {
        return;
    }
    let _ = crate::services::torrent::database::execute_mutation(
        schema,
        &crate::services::torrent::database::system_auth_user(),
        r#"mutation RememberShare($id: String!, $input: UpdateNetworkServerInput!) {
            UpdateNetworkServer: updateNetworkServer(id: $id, input: $input) { success error }
        }"#,
        json!({ "id": server.id, "input": { "knownShares": shares } }),
    )
    .await;
}

/// Record the outcome of a test or a connection on the server's row.
pub async fn record_check(schema: &LibrarianSchema, server_id: &str, error: Option<&str>) {
    let now = chrono::Utc::now().to_rfc3339();
    let input = json!({
        "status": if error.is_some() { "failing" } else { "ok" },
        "lastError": error,
        "lastCheckedAt": now,
    });
    if let Err(failure) = crate::services::torrent::database::execute_mutation(
        schema,
        &crate::services::torrent::database::system_auth_user(),
        r#"mutation RecordServerCheck($id: String!, $input: UpdateNetworkServerInput!) {
            UpdateNetworkServer: updateNetworkServer(id: $id, input: $input) { success error }
        }"#,
        json!({ "id": server_id, "input": input }),
    )
    .await
    {
        tracing::debug!(error = %failure, "Could not record a server check");
    }
}

/// Carry the share sign-ins saved before servers were an entity (`network_mount.<digest>` app
/// settings with platform `smb-native`) over to `NetworkServer` rows, once each. The encrypted
/// password is copied as it is stored, never decrypted, and the old record is left in place.
pub async fn carry_over_saved_sign_ins(db: &Database, schema: &LibrarianSchema) -> Result<usize> {
    use crate::services::graphql::filesystem_network::{
        NATIVE_SHARE_PLATFORM, StoredNetworkPathConfig,
    };
    let legacy: Vec<StoredNetworkPathConfig> = AppSetting::query(db.pool())
        .filter(AppSettingWhereInput {
            key: Some(StringFilter {
                starts_with: Some("network_mount.".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .filter_map(|setting| serde_json::from_str::<StoredNetworkPathConfig>(&setting.value).ok())
        .filter(|config| config.platform == NATIVE_SHARE_PLATFORM)
        .collect();
    let mut created = 0;
    for config in legacy {
        let Ok(share) = SmbLocation::parse(&config.target_path) else {
            continue;
        };
        match find_server(db, &share.host, share.port).await? {
            Some(server) => remember_share(schema, &server, &share.share).await,
            None => {
                let input = json!({
                    "name": share.host,
                    "host": share.host,
                    "port": share.port.map(i32::from),
                    "username": config.username,
                    // Already an encrypted envelope: the write transform keeps it as it is.
                    "password": config.password_encrypted.unwrap_or_default(),
                    "hasPassword": false,
                    "knownShares": [share.share],
                    "status": null,
                    "lastError": null,
                    "lastCheckedAt": null,
                });
                crate::services::torrent::database::execute_mutation(
                    schema,
                    &crate::services::torrent::database::system_auth_user(),
                    r#"mutation CarryOverSignIn($input: CreateNetworkServerInput!) {
                        CreateNetworkServer: createNetworkServer(input: $input) { success error }
                    }"#,
                    json!({ "input": input }),
                )
                .await
                .with_context(|| format!("Could not save the server {}", share.host))?;
                created += 1;
                tracing::info!(
                    host = %share.host,
                    share = %share.share,
                    "Carried a saved share sign-in over to a saved server"
                );
            }
        }
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_are_kept_lower_case_without_scheme_or_slashes() {
        assert_eq!(
            normalize_host(" TrueNAS.LocalDomain ").unwrap(),
            "truenas.localdomain"
        );
        assert_eq!(normalize_host("smb://10.0.1.190/").unwrap(), "10.0.1.190");
        assert_eq!(normalize_host(r"\\nas").unwrap(), "nas");
        assert!(normalize_host("nas/media").is_err());
        assert!(normalize_host("nas:445").is_err());
        assert!(normalize_host("  ").is_err());
    }

    #[test]
    fn the_standard_port_is_not_stored() {
        assert_eq!(normalize_port(Some(445)).unwrap(), None);
        assert_eq!(normalize_port(Some(4455)).unwrap(), Some(4455));
        assert!(normalize_port(Some(70000)).is_err());
    }

    #[test]
    fn shares_are_lower_case_sorted_and_unique() {
        assert_eq!(
            normalize_shares(&[
                "Media".into(),
                "media".into(),
                " Backups/".into(),
                "".into()
            ]),
            vec!["backups".to_string(), "media".to_string()]
        );
    }

    #[test]
    fn a_location_names_its_server() {
        assert_eq!(
            server_of("smb://10.0.1.190/media/Movies"),
            Some(("10.0.1.190".to_string(), None))
        );
        assert_eq!(
            server_of("smb://nas:4455/dl"),
            Some(("nas".to_string(), Some(4455)))
        );
        assert_eq!(server_of("/data/downloads"), None);
        assert!(same_server("smb://NAS:445/media", "nas", None));
        assert!(!same_server("smb://nas:4455/media", "nas", None));
    }

    #[test]
    fn several_uses_read_as_a_sentence() {
        assert_eq!(
            join_words(&["the library Movies".into(), "the download folder".into()]),
            "the library Movies and the download folder"
        );
    }

    mod saved_servers {
        use super::super::*;
        use crate::graphql::entities::{CreateAppSettingInput, NetworkServer};
        use crate::services::graphql::{AuthUser, GraphqlServiceConfig};
        use crate::services::{DatabaseServiceConfig, ServicesManager};
        use async_graphql::{Request, Variables};
        use graphql_orm::graphql::auth::AuthSubject;
        use serde_json::{Value, json};

        struct Fixture {
            _temp: tempfile::TempDir,
            db: Database,
            schema: LibrarianSchema,
        }

        async fn fixture() -> Fixture {
            let temp = tempfile::tempdir().unwrap();
            let services = ServicesManager::builder()
                .add_service(DatabaseServiceConfig {
                    database_url: format!("sqlite://{}", temp.path().join("servers.db").display()),
                    connect_timeout: std::time::Duration::from_secs(5),
                })
                .add_service(crate::services::auth::AuthConfig::for_tests())
                .add_service(GraphqlServiceConfig { server_port: 0 })
                .start()
                .await
                .unwrap();
            let db = services.get_database().await.unwrap().pool().clone();
            let schema = services
                .get_graphql()
                .await
                .unwrap()
                .schema()
                .await
                .unwrap();
            Fixture {
                _temp: temp,
                db,
                schema,
            }
        }

        /// The refusal of a mutation: the GraphQL error a hook raised, or the payload's error.
        async fn refusal(schema: &LibrarianSchema, document: &str, variables: Value) -> String {
            let response = schema
                .execute(
                    Request::new(document)
                        .variables(Variables::from_json(variables))
                        .data(AuthSubject::from_parts(
                            "system",
                            vec!["admin".into()],
                            vec![],
                            None,
                        ))
                        .data(AuthUser::system_admin()),
                )
                .await;
            if let Some(error) = response.errors.first() {
                return error.message.clone();
            }
            let data = response.data.into_json().unwrap();
            let payload = data.as_object().unwrap().values().next().unwrap().clone();
            assert_eq!(payload["success"], false, "expected a refusal: {payload}");
            payload["error"].as_str().unwrap_or_default().to_string()
        }

        async fn run(schema: &LibrarianSchema, document: &str, variables: Value) -> Value {
            let response = schema
                .execute(
                    Request::new(document)
                        .variables(Variables::from_json(variables))
                        .data(AuthSubject::from_parts(
                            "system",
                            vec!["admin".into()],
                            vec![],
                            None,
                        ))
                        .data(AuthUser::system_admin()),
                )
                .await;
            assert!(response.errors.is_empty(), "{:?}", response.errors);
            response.data.into_json().unwrap()
        }

        fn server_input(host: &str, password: &str) -> Value {
            json!({
                "name": "", "host": host, "port": 445, "username": " toby ", "password": password,
                "hasPassword": false, "knownShares": ["Media", "media"], "status": null,
                "lastError": null, "lastCheckedAt": null,
            })
        }

        const CREATE: &str = r#"mutation($input: CreateNetworkServerInput!) {
            createNetworkServer(input: $input) { success error networkServer { id name host port username hasPassword knownShares } }
        }"#;

        #[tokio::test]
        async fn a_server_is_saved_once_normalised_and_refuses_removal_while_used() {
            let f = fixture().await;
            // An already-encrypted envelope is kept as it is (a carried-over sign-in).
            let envelope = "AAAAAAAAAAAAAAAA:YWJjZGVmZ2hpamtsbW5vcA==";
            let created = run(
                &f.schema,
                CREATE,
                json!({ "input": server_input(" NAS ", envelope) }),
            )
            .await;
            let created = &created["createNetworkServer"];
            assert_eq!(created["success"], true, "{created}");
            let saved = &created["networkServer"];
            assert_eq!(saved["host"], "nas");
            assert_eq!(saved["name"], "nas");
            assert_eq!(
                saved["port"],
                Value::Null,
                "the standard port is not stored"
            );
            assert_eq!(saved["username"], "toby");
            assert_eq!(saved["hasPassword"], true);
            assert_eq!(saved["knownShares"], json!(["media"]));
            let id = saved["id"].as_str().unwrap().to_string();
            let row = NetworkServer::get(f.db.pool(), &id).await.unwrap().unwrap();
            assert_eq!(
                row.password, envelope,
                "the stored secret is the envelope given"
            );

            let again = refusal(
                &f.schema,
                CREATE,
                json!({ "input": server_input("nas", "") }),
            )
            .await;
            assert!(again.contains("already saved"), "{again}");

            // The download folder on one of its shares: it cannot be removed or readdressed.
            crate::services::torrent::database::put_setting_value(
                &f.db,
                "torrent.download_dir",
                "\"smb://nas/dl/Downloads\"",
                "torrent",
                "",
            )
            .await
            .unwrap();
            let reason = refusal(
                &f.schema,
                r#"mutation($id: String!) { deleteNetworkServer(id: $id) { success error } }"#,
                json!({ "id": id }),
            )
            .await;
            assert!(reason.contains("the download folder"), "{reason}");
            assert!(
                NetworkServer::get(f.db.pool(), &id)
                    .await
                    .unwrap()
                    .is_some(),
                "not removed"
            );
            let moved = refusal(
                &f.schema,
                r#"mutation($id: String!, $input: UpdateNetworkServerInput!) { updateNetworkServer(id: $id, input: $input) { success error } }"#,
                json!({ "id": id, "input": { "host": "other" } }),
            )
            .await;
            assert!(moved.contains("cannot change"), "{moved}");

            // Renaming and a new sign-in are fine while it is used.
            let renamed = run(
                &f.schema,
                r#"mutation($id: String!, $input: UpdateNetworkServerInput!) { updateNetworkServer(id: $id, input: $input) { success error networkServer { name hasPassword } } }"#,
                json!({ "id": id, "input": { "name": "TrueNAS", "password": "" } }),
            )
            .await;
            assert_eq!(renamed["updateNetworkServer"]["success"], true, "{renamed}");
            assert_eq!(
                renamed["updateNetworkServer"]["networkServer"]["hasPassword"],
                false
            );
            assert_eq!(
                server_usage(&f.db, "nas", Some(445)).await.unwrap(),
                vec!["the download folder".to_string()]
            );
        }

        #[tokio::test]
        async fn a_sign_in_saved_for_a_share_becomes_a_saved_server_once() {
            let f = fixture().await;
            let config = crate::services::graphql::filesystem_network::StoredNetworkPathConfig {
                target_path: "smb://10.0.1.190/media".into(),
                remote_path: "smb://10.0.1.190/media".into(),
                username: Some("toby".into()),
                password_encrypted: Some("AAAAAAAAAAAAAAAA:YWJjZGVmZ2hpamtsbW5vcA==".into()),
                platform: crate::services::graphql::filesystem_network::NATIVE_SHARE_PLATFORM
                    .into(),
                created_at: String::new(),
                updated_at: String::new(),
            };
            AppSetting::insert(
                &f.db,
                CreateAppSettingInput {
                    key: "network_mount.abc".into(),
                    value: serde_json::to_string(&config).unwrap(),
                    description: None,
                    category: "filesystem".into(),
                },
            )
            .await
            .unwrap();
            assert_eq!(
                carry_over_saved_sign_ins(&f.db, &f.schema).await.unwrap(),
                1
            );
            assert_eq!(
                carry_over_saved_sign_ins(&f.db, &f.schema).await.unwrap(),
                0,
                "once"
            );
            let server = find_server(&f.db, "10.0.1.190", None)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(server.username.as_deref(), Some("toby"));
            assert_eq!(
                server.password, "AAAAAAAAAAAAAAAA:YWJjZGVmZ2hpamtsbW5vcA==",
                "copied, never decrypted"
            );
            assert!(server.has_password);
            assert_eq!(server.known_shares, vec!["media".to_string()]);
        }
    }
}
