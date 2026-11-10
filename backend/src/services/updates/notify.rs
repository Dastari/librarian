//! Telling admins a new release exists: one notification per version per admin, never repeated.
//!
//! - Category and action type `server_update`; `actionData` is JSON `{version, tag, url,
//!   canUpgrade}` so the Activity page can offer "Release notes" and "Upgrade".
//! - The versions already announced are kept in the update record (`notified`), so a dismissed or
//!   deleted notification does not come back on the next check.
//! - An open notification for an older version is resolved as `superseded` when a newer one is
//!   announced, and as `installed` once this server runs that version or newer.

use chrono::Utc;
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use semver::Version;
use tracing::{info, warn};

use super::github::ReleaseInfo;
use super::version::parse_tag;
use crate::db::Database;
use crate::graphql::entities::{
    CreateNotificationInput, Notification, NotificationWhereInput, UpdateNotificationInput, User,
    UserWhereInput,
};

pub const CATEGORY: &str = "server_update";
pub const ACTION_TYPE: &str = "server_update";
/// How many announced versions are remembered.
const NOTIFIED_LIMIT: usize = 30;

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

/// The version to announce, if the newest release is newer than what runs and was not announced
/// before. Pure.
pub fn version_to_announce<'a>(
    latest: Option<&'a ReleaseInfo>,
    running: &Version,
    notified: &[String],
) -> Option<&'a ReleaseInfo> {
    let latest = latest?;
    let version = latest.semver()?;
    (version > *running && !notified.iter().any(|seen| seen == &latest.version)).then_some(latest)
}

/// Remember `version` as announced, keeping the list short.
pub fn remember(notified: &mut Vec<String>, version: &str) {
    if !notified.iter().any(|seen| seen == version) {
        notified.push(version.to_string());
    }
    if notified.len() > NOTIFIED_LIMIT {
        let excess = notified.len() - NOTIFIED_LIMIT;
        notified.drain(..excess);
    }
}

/// The notification's title and message.
pub fn announcement(
    release: &ReleaseInfo,
    running: &Version,
    blocked: Option<&str>,
) -> (String, String) {
    let title = format!("Librarian {} is available", release.version);
    let message = match blocked {
        None => format!(
            "This server runs {running}. Read the release notes, then upgrade from Settings › Updates."
        ),
        Some(why) => format!("This server runs {running}. {why}"),
    };
    (title, message)
}

pub async fn admins(db: &Database) -> Vec<String> {
    match User::query(db.pool())
        .filter(UserWhereInput {
            role: Some(string_eq("admin")),
            ..Default::default()
        })
        .fetch_all()
        .await
    {
        Ok(admins) => admins
            .into_iter()
            .filter(|admin| admin.is_active)
            .map(|admin| admin.id)
            .collect(),
        Err(error) => {
            warn!(error = %error, "Updates: could not list the admins to tell about a release");
            Vec::new()
        }
    }
}

fn version_of(notification: &Notification) -> Option<Version> {
    let data: serde_json::Value =
        serde_json::from_str(notification.action_data.as_deref()?).ok()?;
    parse_tag(data.get("version")?.as_str()?)
}

/// Resolve the open release notifications that no longer apply: older than `newest`, or already
/// installed (not newer than `running`).
pub async fn resolve_outdated(db: &Database, running: &Version, newest: Option<&Version>) {
    let open = match Notification::query(db.pool())
        .filter(NotificationWhereInput {
            category: Some(string_eq(CATEGORY)),
            resolved_at: Some(DateFilter {
                is_null: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
    {
        Ok(rows) => rows,
        Err(error) => {
            warn!(error = %error, "Updates: could not look for open release notifications");
            return;
        }
    };
    for row in open {
        let Some(version) = version_of(&row) else {
            continue;
        };
        let resolution = if version <= *running {
            "installed"
        } else if newest.is_some_and(|newest| version < *newest) {
            "superseded"
        } else {
            continue;
        };
        if let Err(error) = Notification::update_by_id(
            db,
            &row.id,
            UpdateNotificationInput {
                resolved_at: Some(Some(Utc::now().to_rfc3339())),
                resolution: Some(Some(resolution.to_string())),
                ..Default::default()
            },
        )
        .await
        {
            warn!(version = %version, error = %error, "Updates: could not resolve the notification about Librarian {version}");
        }
    }
}

/// Announce `release` to every admin. Returns whether anyone was told.
pub async fn announce(
    db: &Database,
    release: &ReleaseInfo,
    running: &Version,
    blocked: Option<&str>,
) -> bool {
    let (title, message) = announcement(release, running, blocked);
    let data = serde_json::json!({
        "version": release.version,
        "tag": release.tag,
        "url": release.url,
        "canUpgrade": blocked.is_none(),
    })
    .to_string();
    let recipients = admins(db).await;
    let mut told = false;
    for user_id in &recipients {
        match Notification::insert(
            db,
            CreateNotificationInput {
                user_id: user_id.clone(),
                notification_type: "INFO".to_string(),
                category: CATEGORY.to_string(),
                title: title.clone(),
                message: message.clone(),
                library_id: None,
                torrent_id: None,
                media_file_id: None,
                pending_match_id: None,
                action_type: Some(ACTION_TYPE.to_string()),
                action_data: Some(data.clone()),
                read_at: None,
                resolved_at: None,
                resolution: None,
            },
        )
        .await
        {
            Ok(_) => told = true,
            Err(error) => {
                warn!(version = %release.version, error = %error, "Updates: could not tell an admin about Librarian {}", release.version)
            }
        }
    }
    if told {
        info!(
            version = %release.version,
            admins = recipients.len(),
            "Updates: told {} admins that Librarian {} is available",
            recipients.len(),
            release.version
        );
    }
    told
}

/// Tell the admins something about an upgrade that already happened (it rolled back, say). Plain
/// information: no action to take.
pub async fn tell_admins(db: &Database, level: &str, title: &str, message: &str) {
    for user_id in admins(db).await {
        if let Err(error) = Notification::insert(
            db,
            CreateNotificationInput {
                user_id,
                notification_type: level.to_string(),
                category: CATEGORY.to_string(),
                title: title.to_string(),
                message: message.to_string(),
                library_id: None,
                torrent_id: None,
                media_file_id: None,
                pending_match_id: None,
                action_type: None,
                action_data: None,
                read_at: None,
                resolved_at: None,
                resolution: None,
            },
        )
        .await
        {
            warn!(error = %error, "Updates: could not tell an admin '{title}'");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphql::entities::CreateUserInput;
    use crate::services::database::DatabaseService;
    use crate::services::manager::Service;

    fn release(version: &str) -> ReleaseInfo {
        ReleaseInfo {
            tag: format!("v{version}"),
            version: version.to_string(),
            name: None,
            published_at: None,
            url: format!("https://github.com/Dastari/librarian/releases/tag/v{version}"),
            notes: None,
            prerelease: false,
            assets: Vec::new(),
        }
    }

    #[test]
    fn a_version_is_announced_once_and_only_when_newer() {
        let running = Version::parse("0.6.0").unwrap();
        let newer = release("0.7.0");
        assert_eq!(
            version_to_announce(Some(&newer), &running, &[]).map(|r| r.version.as_str()),
            Some("0.7.0")
        );
        assert!(version_to_announce(Some(&newer), &running, &["0.7.0".to_string()]).is_none());
        assert!(version_to_announce(Some(&release("0.6.0")), &running, &[]).is_none());
        assert!(version_to_announce(Some(&release("0.6.0-dev.3")), &running, &[]).is_none());
        assert!(version_to_announce(None, &running, &[]).is_none());
        let mut notified = Vec::new();
        for n in 0..40 {
            remember(&mut notified, &format!("0.{n}.0"));
        }
        remember(&mut notified, "0.39.0");
        assert_eq!(notified.len(), NOTIFIED_LIMIT);
        assert_eq!(notified.last().unwrap(), "0.39.0");
    }

    #[test]
    fn the_message_says_how_to_upgrade_or_why_not() {
        let running = Version::parse("0.6.0").unwrap();
        let (title, message) = announcement(&release("0.7.0"), &running, None);
        assert_eq!(title, "Librarian 0.7.0 is available");
        assert!(message.contains("Settings › Updates"));
        let (_, blocked) = announcement(&release("0.7.0"), &running, Some("Pull the new image."));
        assert!(blocked.ends_with("Pull the new image."));
    }

    async fn database() -> Database {
        let pool = graphql_orm::DbPool::connect("sqlite::memory:")
            .await
            .unwrap();
        let service = DatabaseService::new(Database::new(pool));
        service.start().await.unwrap();
        service.pool().clone()
    }

    async fn user(db: &Database, name: &str, role: &str) -> String {
        User::insert(
            db,
            CreateUserInput {
                username: name.to_string(),
                email: None,
                password_hash: "hash".to_string(),
                role: role.to_string(),
                display_name: None,
                avatar_url: None,
                is_active: true,
                last_login_at: None,
            },
        )
        .await
        .unwrap()
        .id
    }

    async fn release_notifications(db: &Database) -> Vec<Notification> {
        Notification::query(db.pool())
            .filter(NotificationWhereInput {
                category: Some(string_eq(CATEGORY)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn admins_hear_once_per_version_and_old_news_is_resolved() {
        let db = database().await;
        let admin = user(&db, "admin-one", "admin").await;
        user(&db, "member-one", "member").await;
        let running = Version::parse("0.6.0").unwrap();
        let mut notified = Vec::new();

        let first = release("0.7.0");
        let due = version_to_announce(Some(&first), &running, &notified).unwrap();
        assert!(announce(&db, due, &running, None).await);
        remember(&mut notified, &due.version);
        // The next check finds the same release: nothing new.
        assert!(version_to_announce(Some(&first), &running, &notified).is_none());
        let rows = release_notifications(&db).await;
        assert_eq!(rows.len(), 1, "only the admin hears");
        assert_eq!(rows[0].user_id, admin);
        assert_eq!(rows[0].action_type.as_deref(), Some(ACTION_TYPE));
        let data: serde_json::Value =
            serde_json::from_str(rows[0].action_data.as_deref().unwrap()).unwrap();
        assert_eq!(data["version"], "0.7.0");
        assert_eq!(data["canUpgrade"], true);

        // A newer release supersedes the open one.
        let second = release("0.7.1");
        let due = version_to_announce(Some(&second), &running, &notified).unwrap();
        assert!(announce(&db, due, &running, Some("Pull the new image.")).await);
        remember(&mut notified, &due.version);
        resolve_outdated(&db, &running, second.semver().as_ref()).await;
        let rows = release_notifications(&db).await;
        let old = rows.iter().find(|row| row.title.contains("0.7.0")).unwrap();
        assert_eq!(old.resolution.as_deref(), Some("superseded"));
        let new = rows.iter().find(|row| row.title.contains("0.7.1")).unwrap();
        assert!(new.resolved_at.is_none());

        // Once the server runs 0.7.1 the last one is resolved as installed.
        let upgraded = Version::parse("0.7.1").unwrap();
        resolve_outdated(&db, &upgraded, second.semver().as_ref()).await;
        let rows = release_notifications(&db).await;
        let new = rows.iter().find(|row| row.title.contains("0.7.1")).unwrap();
        assert_eq!(new.resolution.as_deref(), Some("installed"));
        assert_eq!(rows.len(), 2);
    }
}
