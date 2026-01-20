use super::prelude::*;

#[derive(Default)]
pub struct RssFeedMutations;

#[Object]
impl RssFeedMutations {
    /// Create an RSS feed
    async fn create_rss_feed(
        &self,
        ctx: &Context<'_>,
        input: CreateRssFeedInput,
    ) -> Result<RssFeedResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let record = db
            .rss_feeds()
            .create(CreateRssFeed {
                user_id,
                library_id: input.library_id.and_then(|id| Uuid::parse_str(&id).ok()),
                name: input.name,
                url: input.url,
                enabled: input.enabled.unwrap_or(true),
                poll_interval_minutes: input.poll_interval_minutes.unwrap_or(15),
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(RssFeedResult {
            success: true,
            rss_feed: Some(RssFeed {
                id: record.id.to_string(),
                library_id: record.library_id.map(|id| id.to_string()),
                name: record.name,
                url: record.url,
                enabled: record.enabled,
                poll_interval_minutes: record.poll_interval_minutes,
                last_polled_at: record.last_polled_at.map(|t| t.to_rfc3339()),
                last_successful_at: record.last_successful_at.map(|t| t.to_rfc3339()),
                last_error: record.last_error,
                consecutive_failures: record.consecutive_failures.unwrap_or(0),
            }),
            error: None,
        })
    }

    /// Update an RSS feed
    async fn update_rss_feed(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateRssFeedInput,
    ) -> Result<RssFeedResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let feed_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid feed ID: {}", e)))?;

        let result = db
            .rss_feeds()
            .update(
                feed_id,
                UpdateRssFeed {
                    library_id: input.library_id.and_then(|id| Uuid::parse_str(&id).ok()),
                    name: input.name,
                    url: input.url,
                    enabled: input.enabled,
                    poll_interval_minutes: input.poll_interval_minutes,
                },
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(record) = result {
            Ok(RssFeedResult {
                success: true,
                rss_feed: Some(RssFeed {
                    id: record.id.to_string(),
                    library_id: record.library_id.map(|id| id.to_string()),
                    name: record.name,
                    url: record.url,
                    enabled: record.enabled,
                    poll_interval_minutes: record.poll_interval_minutes,
                    last_polled_at: record.last_polled_at.map(|t| t.to_rfc3339()),
                    last_successful_at: record.last_successful_at.map(|t| t.to_rfc3339()),
                    last_error: record.last_error,
                    consecutive_failures: record.consecutive_failures.unwrap_or(0),
                }),
                error: None,
            })
        } else {
            Ok(RssFeedResult {
                success: false,
                rss_feed: None,
                error: Some("RSS feed not found".to_string()),
            })
        }
    }

    /// Delete an RSS feed
    async fn delete_rss_feed(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let feed_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid feed ID: {}", e)))?;

        let deleted = db
            .rss_feeds()
            .delete(feed_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: deleted,
            error: if deleted {
                None
            } else {
                Some("RSS feed not found".to_string())
            },
        })
    }

    /// Test an RSS feed by fetching and parsing its items (without storing)
    async fn test_rss_feed(&self, ctx: &Context<'_>, url: String) -> Result<RssFeedTestResult> {
        let _user = ctx.auth_user()?;

        let rss_service = crate::services::RssService::new();
        match rss_service.fetch_feed(&url).await {
            Ok(items) => {
                let sample_items: Vec<RssItem> = items
                    .into_iter()
                    .take(10)
                    .map(|item| RssItem {
                        title: item.title,
                        link: item.link,
                        pub_date: item.pub_date.map(|d| d.to_rfc3339()),
                        description: item.description,
                        parsed_show_name: item.parsed_show_name,
                        parsed_season: item.parsed_season,
                        parsed_episode: item.parsed_episode,
                        parsed_resolution: item.parsed_resolution,
                        parsed_codec: item.parsed_codec,
                    })
                    .collect();

                Ok(RssFeedTestResult {
                    success: true,
                    item_count: sample_items.len() as i32,
                    sample_items,
                    error: None,
                })
            }
            Err(e) => Ok(RssFeedTestResult {
                success: false,
                item_count: 0,
                sample_items: vec![],
                error: Some(e.to_string()),
            }),
        }
    }

    /// Manually poll an RSS feed
    async fn poll_rss_feed(&self, ctx: &Context<'_>, id: String) -> Result<RssFeedResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let feed_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid feed ID: {}", e)))?;

        // Poll the feed using the same logic as the background job
        // This ensures items are stored AND matched to episodes
        match crate::jobs::rss_poller::poll_single_feed_by_id(db, feed_id).await {
            Ok((new_items, matched_episodes)) => {
                tracing::info!(
                    user_id = %_user.user_id,
                    feed_id = %feed_id,
                    new_items = new_items,
                    matched_episodes = matched_episodes,
                    "User manually polled RSS feed: {} new items, {} matched episodes",
                    new_items, matched_episodes
                );
                // Get updated feed
                let updated_feed = db
                    .rss_feeds()
                    .get_by_id(feed_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("RSS feed not found"))?;

                Ok(RssFeedResult {
                    success: true,
                    rss_feed: Some(RssFeed {
                        id: updated_feed.id.to_string(),
                        library_id: updated_feed.library_id.map(|id| id.to_string()),
                        name: updated_feed.name,
                        url: updated_feed.url,
                        enabled: updated_feed.enabled,
                        poll_interval_minutes: updated_feed.poll_interval_minutes,
                        last_polled_at: updated_feed.last_polled_at.map(|t| t.to_rfc3339()),
                        last_successful_at: updated_feed.last_successful_at.map(|t| t.to_rfc3339()),
                        last_error: updated_feed.last_error,
                        consecutive_failures: updated_feed.consecutive_failures.unwrap_or(0),
                    }),
                    error: None,
                })
            }
            Err(e) => {
                // Mark poll failure
                let _ = db
                    .rss_feeds()
                    .mark_poll_failure(feed_id, &e.to_string())
                    .await;

                Ok(RssFeedResult {
                    success: false,
                    rss_feed: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }
}
