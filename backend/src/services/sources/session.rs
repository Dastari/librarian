//! Signing password-authenticated sources in again.
//!
//! [`SourceSessions`] is the [`SessionRefresher`] the sources service installs. When a source with a
//! username and password finds its session has ended, it asks here for a new one. This is where the
//! credentials are (decrypted only for the length of the call), where the one-attempt-an-hour limit is
//! enforced, and where the new session is stored, encrypted, with the source's other credentials.
//!
//! Never logged: the username, the password, the cookies.

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::graphql::entities::{Source, UpdateSourceInput};
use crate::services::ServicesManager;
use crate::services::redact::redact_text;
use crate::services::sources::SessionRefresher;
use crate::services::sources::definitions::iptorrents_login;

/// The least time between two sign-in attempts for one source.
pub const LOGIN_RETRY_GAP: Duration = Duration::hours(1);
/// Two requests that find the session ended together share one sign-in: a session made this
/// recently is handed to the second.
const SHARE_WINDOW: Duration = Duration::seconds(60);

/// Whether a sign-in may be tried now, given when the last was. Returns when the next may be, if not.
pub fn next_attempt_allowed(
    last_attempt: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), DateTime<Utc>> {
    let Some(last) = last_attempt
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|time| time.with_timezone(&Utc))
    else {
        return Ok(());
    };
    let allowed_at = last + LOGIN_RETRY_GAP;
    if now >= allowed_at {
        Ok(())
    } else {
        Err(allowed_at)
    }
}

/// Whether the last sign-in was so recent that its session should simply be shared.
pub fn session_is_fresh(last_attempt: Option<&str>, now: DateTime<Utc>) -> bool {
    last_attempt
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|time| now - time.with_timezone(&Utc) < SHARE_WINDOW)
}

pub struct SourceSessions {
    services: Weak<ServicesManager>,
    /// One sign-in at a time per source.
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl SourceSessions {
    pub fn new(services: &Arc<ServicesManager>) -> Self {
        Self {
            services: Arc::downgrade(services),
            locks: Mutex::new(HashMap::new()),
        }
    }

    async fn lock_for(&self, source_id: &str) -> Arc<Mutex<()>> {
        self.locks
            .lock()
            .await
            .entry(source_id.to_string())
            .or_default()
            .clone()
    }

    async fn attempt(&self, source_id: &str) -> Result<Result<String, String>> {
        let services = self
            .services
            .upgrade()
            .context("the server is shutting down")?;
        let db = services
            .get_database()
            .await
            .context("database service not available")?
            .pool()
            .clone();
        let manager = services
            .get_sources()
            .await
            .context("sources service not available")?
            .get_manager()
            .await
            .context("sources manager not initialized")?;
        let source = Source::get(db.pool(), &source_id.to_string())
            .await?
            .context("source not found")?;
        let stored = manager
            .encryption()
            .reveal(&source.credentials)
            .context("the stored credentials could not be decrypted")?;
        let mut credentials: HashMap<String, String> =
            serde_json::from_str(&stored).context("the stored credentials are not valid")?;
        let username = credentials.get("Username").cloned().unwrap_or_default();
        let password = credentials.get("Password").cloned().unwrap_or_default();
        if credentials
            .get("AuthMethod")
            .is_some_and(|method| method == "cookie")
            || username.is_empty()
            || password.is_empty()
        {
            return Ok(Err(
                "no username and password is stored for this source, so it cannot sign in again"
                    .to_string(),
            ));
        }

        let now = Utc::now();
        if session_is_fresh(source.login_attempted_at.as_deref(), now)
            && let Some(cookie) = credentials
                .get("SessionCookie")
                .filter(|cookie| !cookie.is_empty())
        {
            return Ok(Ok(cookie.clone()));
        }
        if let Err(allowed_at) = next_attempt_allowed(source.login_attempted_at.as_deref(), now) {
            return Ok(Err(format!(
                "signing in again is paused until {} (one attempt an hour, so the account is not locked)",
                allowed_at.format("%H:%M UTC")
            )));
        }

        // The attempt is recorded before it is made, so a crash in the middle cannot lead to a
        // second one inside the hour.
        let previous = source.login_attempted_at.clone();
        Source::update_by_id(
            &db,
            &source.id,
            UpdateSourceInput {
                login_attempted_at: Some(Some(now.to_rfc3339())),
                ..Default::default()
            },
        )
        .await
        .context("could not record the sign-in attempt")?;

        let site = source
            .site_url
            .clone()
            .unwrap_or_else(|| "https://iptorrents.com/".to_string());
        let user_agent = credentials.get("UserAgent").cloned().unwrap_or_default();
        info!(source = %source.name, "Signing '{}' in with its username and password", source.name);
        let outcome = iptorrents_login::sign_in(&site, &username, &password, &user_agent).await;
        match outcome {
            Err(network) => {
                // The site was not reached, so the credentials were not tried: give the attempt back.
                let _ = Source::update_by_id(
                    &db,
                    &source.id,
                    UpdateSourceInput {
                        login_attempted_at: Some(previous),
                        ..Default::default()
                    },
                )
                .await;
                Ok(Err(redact_text(&format!(
                    "{} could not be reached to sign in: {network:#}",
                    source.name
                ))))
            }
            Ok(Err(problem)) => {
                warn!(source = %source.name, "Sign-in for '{}' failed: {problem}", source.name);
                Ok(Err(format!(
                    "{} can't sign in: {}",
                    source.name,
                    problem.message()
                )))
            }
            Ok(Ok(session)) => {
                credentials.insert("SessionCookie".to_string(), session.cookie_header.clone());
                let json = serde_json::to_string(&credentials).map_err(|e| anyhow!(e))?;
                // Written through the entity layer, which encrypts it like the rest of the credentials.
                Source::update_by_id(
                    &db,
                    &source.id,
                    UpdateSourceInput {
                        credentials: Some(json),
                        ..Default::default()
                    },
                )
                .await
                .context("signed in, but could not store the session")?;
                info!(source = %source.name, "Signed '{}' in; its session is stored", source.name);
                Ok(Ok(session.cookie_header))
            }
        }
    }
}

#[async_trait]
impl SessionRefresher for SourceSessions {
    async fn refresh(&self, source_id: &str) -> Result<String, String> {
        let lock = self.lock_for(source_id).await;
        let _one_at_a_time = lock.lock().await;
        match self.attempt(source_id).await {
            Ok(result) => result,
            Err(error) => Err(redact_text(&format!("{error:#}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn a_sign_in_is_allowed_once_an_hour_after_the_last_attempt() {
        let now = at("2026-11-08T12:00:00Z");
        assert!(next_attempt_allowed(None, now).is_ok());
        assert!(next_attempt_allowed(Some("garbled"), now).is_ok());
        assert_eq!(
            next_attempt_allowed(Some("2026-11-08T11:30:00Z"), now),
            Err(at("2026-11-08T12:30:00Z"))
        );
        assert!(next_attempt_allowed(Some("2026-11-08T11:00:00Z"), now).is_ok());
    }

    #[test]
    fn two_requests_in_the_same_minute_share_one_sign_in() {
        let now = at("2026-11-08T12:00:30Z");
        assert!(session_is_fresh(Some("2026-11-08T12:00:00Z"), now));
        assert!(!session_is_fresh(Some("2026-11-08T11:58:00Z"), now));
        assert!(!session_is_fresh(None, now));
    }
}
