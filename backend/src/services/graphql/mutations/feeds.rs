//! Actions on RSS feeds that are not plain edits: reading one now.
//!
//! Creating, editing and deleting feeds is the generated CRUD (`createRssFeed`, `updateRssFeed`,
//! `deleteRssFeed`, admin only; the write transform encrypts the address). Grabbing a catalogue
//! item is `addTorrent` with `feedItemId`.

use async_graphql::{Context, Object, SimpleObject};

use crate::db::Database;
use crate::jobs::auto_settings::AutoDownloadSettings;
use crate::jobs::rss_poll;
use crate::services::ServicesManager;
use crate::services::graphql::auth::AuthExt;
use crate::services::graphql::entities::RssFeed;

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "PollRssFeedPayload")]
#[graphql(rename_fields = "camelCase")]
pub struct PollRssFeedPayload {
    pub success: bool,
    pub error: Option<String>,
    /// Items not seen before, now in the catalogue.
    pub new_items: i32,
    /// Releases grabbed for wanted titles (only when automatic downloading is on).
    pub grabbed: i32,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "CreateRssFeedFromSourcePayload")]
#[graphql(rename_fields = "camelCase")]
pub struct CreateRssFeedFromSourcePayload {
    pub success: bool,
    pub error: Option<String>,
    pub feed_id: Option<String>,
    /// The new feed's address with every secret hidden.
    pub url_display: Option<String>,
}

fn failed(error: impl Into<String>) -> CreateRssFeedFromSourcePayload {
    CreateRssFeedFromSourcePayload {
        success: false,
        error: Some(crate::services::redact::redact_text(&error.into())),
        feed_id: None,
        url_display: None,
    }
}

#[derive(Default)]
pub struct FeedMutations;

#[Object]
impl FeedMutations {
    /// Read a feed now, whether or not it is due (and even if it is disabled). New items go into the
    /// catalogue; wanted titles are only grabbed when automatic downloading is on. Admin only.
    #[graphql(name = "pollRssFeed")]
    async fn poll_rss_feed(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: String,
    ) -> async_graphql::Result<PollRssFeedPayload> {
        ctx.require_admin()?;
        let db = ctx.data::<Database>()?;
        let manager = ctx.data::<std::sync::Arc<ServicesManager>>()?;
        if RssFeed::get(db.pool(), &id).await?.is_none() {
            return Ok(PollRssFeedPayload {
                success: false,
                error: Some("Feed not found".to_string()),
                new_items: 0,
                grabbed: 0,
            });
        }
        let grab = AutoDownloadSettings::load(db).await.enabled;
        match rss_poll::poll_feeds(manager, Some(&id), true, grab).await {
            Ok(summary) => {
                // The failure text is already free of the address (it never contains it).
                let error = RssFeed::get(db.pool(), &id)
                    .await?
                    .and_then(|feed| feed.last_error)
                    .filter(|_| summary.feeds_failed > 0);
                Ok(PollRssFeedPayload {
                    success: summary.feeds_failed == 0,
                    error,
                    new_items: summary.new_items as i32,
                    grabbed: summary.grabbed as i32,
                })
            }
            Err(error) => Ok(PollRssFeedPayload {
                success: false,
                error: Some(crate::services::redact::redact_text(&format!("{error:#}"))),
                new_items: 0,
                grabbed: 0,
            }),
        }
    }

    /// Read the personal RSS link from a signed-in IPTorrents source and make it an RSS feed. The
    /// link carries the account's passkey: it goes straight into the feed's encrypted address and
    /// only the redacted form comes back. Admin only.
    #[graphql(name = "createRssFeedFromSource")]
    async fn create_rss_feed_from_source(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "sourceId")] source_id: String,
        #[graphql(name = "name")] name: Option<String>,
        #[graphql(name = "pollIntervalMinutes")] poll_interval_minutes: Option<i32>,
        #[graphql(name = "libraryId")] library_id: Option<String>,
    ) -> async_graphql::Result<CreateRssFeedFromSourcePayload> {
        use crate::services::graphql::entities::{CreateRssFeedInput, Source};
        use crate::services::sources::definitions::iptorrents_login;

        let admin = ctx.require_admin()?;
        let db = ctx.data::<Database>()?;
        let manager = ctx.data::<std::sync::Arc<ServicesManager>>()?;
        let Some(source) = Source::get(db.pool(), &source_id).await? else {
            return Ok(failed("Source not found"));
        };
        if source.definition_id != "iptorrents" {
            return Ok(failed(
                "Only an IPTorrents source can make an RSS feed this way",
            ));
        }
        let Some(service) = manager.get_sources().await else {
            return Ok(failed("The sources service is not available"));
        };
        let Some(sources) = service.get_manager().await else {
            return Ok(failed(
                "Source credentials are not available: no credential key is configured",
            ));
        };
        let credentials: std::collections::HashMap<String, String> = match sources
            .encryption()
            .reveal(&source.credentials)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
        {
            Some(credentials) => credentials,
            None => return Ok(failed("The source's credentials could not be read")),
        };
        let password_login = credentials
            .get("AuthMethod")
            .is_none_or(|method| method != "cookie");
        let mut cookie = if password_login {
            credentials
                .get("SessionCookie")
                .cloned()
                .unwrap_or_default()
        } else {
            credentials.get("Cookie").cloned().unwrap_or_default()
        };
        let site = source
            .site_url
            .clone()
            .unwrap_or_else(|| "https://iptorrents.com/".to_string());
        let user_agent = credentials.get("UserAgent").cloned().unwrap_or_default();
        if cookie.is_empty() && password_login {
            match sources.refresh_session(&source.id).await {
                Ok(fresh) => cookie = fresh,
                Err(reason) => return Ok(failed(reason)),
            }
        }
        if cookie.is_empty() {
            return Ok(failed(
                "The source has no session yet: test it first so it can sign in",
            ));
        }
        let mut found = iptorrents_login::fetch_rss_link(&site, &cookie, &user_agent).await;
        if found.is_err() && password_login {
            // The session may have ended: renew it once (subject to the one-an-hour limit).
            if let Ok(fresh) = sources.refresh_session(&source.id).await {
                found = iptorrents_login::fetch_rss_link(&site, &fresh, &user_agent).await;
            }
        }
        let link = match found {
            Ok(Some(link)) => link,
            Ok(None) => {
                return Ok(failed(
                    "The site's RSS page did not show a link. Paste the feed address in by hand.",
                ));
            }
            Err(error) => return Ok(failed(format!("{error:#}"))),
        };
        let display = crate::services::redact::feed_url_display(&link);
        let feed = crate::services::graphql::entities::RssFeed::insert(
            db,
            CreateRssFeedInput {
                user_id: admin.user_id.clone(),
                library_id,
                name: name
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| format!("{} (feed)", source.name)),
                url: link,
                enabled: true,
                poll_interval_minutes: poll_interval_minutes.unwrap_or(30).max(15),
                post_download_action: None,
                last_polled_at: None,
                last_successful_at: None,
                last_error: None,
                consecutive_failures: None,
                url_display: Some(display.clone()),
                health_status: None,
                health_kind: None,
            },
        )
        .await;
        match feed {
            Ok(feed) => Ok(CreateRssFeedFromSourcePayload {
                success: true,
                error: None,
                feed_id: Some(feed.id),
                url_display: Some(display),
            }),
            Err(error) => Ok(failed(error.to_string())),
        }
    }
}
