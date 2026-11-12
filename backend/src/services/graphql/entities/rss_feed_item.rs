use std::collections::HashMap;
use std::sync::Arc;

use super::super::auth::AuthUser;
use crate::graphql::entities::*;
use crate::services::ServicesManager;
use crate::services::sources::{
    PagedSourceSearchResult, SourceContinuationSupport, SourceRelease, SourceSearchPageInfo,
    SourceSearchPageState, SourceSearchResult,
};
use async_graphql::SimpleObject;
use graphql_orm::graphql::filters::StringFilter;
use graphql_orm::graphql::orm::OrderDirection;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    async_graphql::SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(complex)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "rss_feed_items",
    plural = "RssFeedItems",
    default_sort = "seen_at",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct RssFeedItem {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "feedId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub feed_id: String,

    #[graphql(name = "guid")]
    #[filterable(type = "string")]
    pub guid: Option<String>,

    #[graphql(name = "linkHash")]
    #[filterable(type = "string")]
    pub link_hash: String,

    #[graphql(name = "titleHash")]
    #[filterable(type = "string")]
    pub title_hash: String,

    #[graphql(name = "title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    /// The download link, which carries the tracker passkey. Stored as an encrypted envelope;
    /// never readable through GraphQL. Grabbing goes through `grabFeedItem`.
    #[graphql(skip)]
    #[input_only]
    #[graphql_orm(sensitive)]
    pub link: String,

    #[graphql(name = "infoHash")]
    #[filterable(type = "string")]
    pub info_hash: Option<String>,

    /// Bytes, from the feed's size or the description (`12.1 GB; Movie/HD/Bluray`).
    #[graphql(name = "size")]
    #[filterable(type = "number")]
    #[sortable]
    pub size: Option<i64>,

    #[graphql(name = "seeders")]
    #[filterable(type = "number")]
    pub seeders: Option<i32>,

    /// The tracker's category label, from the description.
    #[graphql(name = "category")]
    #[filterable(type = "string")]
    pub category: Option<String>,

    /// `episode`, `season_pack`, `series_pack`, `movie`, `album`, `book` or `unknown`.
    #[graphql(name = "parsedScope")]
    #[filterable(type = "string")]
    pub parsed_scope: Option<String>,

    #[graphql(name = "parsedYear")]
    #[filterable(type = "number")]
    pub parsed_year: Option<i32>,

    #[graphql(name = "parsedSeasonEnd")]
    #[filterable(type = "number")]
    pub parsed_season_end: Option<i32>,

    #[graphql(name = "parsedGroup")]
    #[filterable(type = "string")]
    pub parsed_group: Option<String>,

    #[graphql(name = "parsedArtist")]
    #[filterable(type = "string")]
    pub parsed_artist: Option<String>,

    #[graphql(name = "parsedAlbum")]
    #[filterable(type = "string")]
    pub parsed_album: Option<String>,

    #[graphql(name = "parsedFormat")]
    #[filterable(type = "string")]
    pub parsed_format: Option<String>,

    #[graphql(name = "parsedBitrate")]
    pub parsed_bitrate: Option<String>,

    /// What it was taken to be: `movie`, `show`, `season`, `episode`, `album` or `audiobook`;
    /// empty when nothing in the library matched (see `matchReason`).
    #[graphql(name = "matchKind")]
    #[filterable(type = "string")]
    pub match_kind: Option<String>,

    #[graphql(name = "matchShowId")]
    #[filterable(type = "string")]
    pub match_show_id: Option<String>,

    #[graphql(name = "matchMovieId")]
    #[filterable(type = "string")]
    pub match_movie_id: Option<String>,

    #[graphql(name = "matchAlbumId")]
    #[filterable(type = "string")]
    pub match_album_id: Option<String>,

    #[graphql(name = "matchAudiobookId")]
    #[filterable(type = "string")]
    pub match_audiobook_id: Option<String>,

    #[graphql(name = "matchSeason")]
    #[filterable(type = "number")]
    pub match_season: Option<i32>,

    #[graphql(name = "matchEpisode")]
    #[filterable(type = "number")]
    pub match_episode: Option<i32>,

    #[graphql(name = "matchLabel")]
    #[filterable(type = "string")]
    pub match_label: Option<String>,

    /// Percent.
    #[graphql(name = "matchConfidence")]
    #[filterable(type = "number")]
    #[sortable]
    pub match_confidence: Option<i32>,

    /// Why it matched, or why it did not.
    #[graphql(name = "matchReason")]
    pub match_reason: Option<String>,

    #[graphql(name = "matchedAt")]
    #[filterable(type = "date")]
    pub matched_at: Option<String>,

    #[graphql(name = "pubDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub pub_date: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,

    #[graphql(name = "parsedShowName")]
    #[filterable(type = "string")]
    pub parsed_show_name: Option<String>,

    #[graphql(name = "parsedSeason")]
    #[filterable(type = "number")]
    pub parsed_season: Option<i32>,

    #[graphql(name = "parsedEpisode")]
    #[filterable(type = "number")]
    pub parsed_episode: Option<i32>,

    #[graphql(name = "parsedResolution")]
    #[filterable(type = "string")]
    pub parsed_resolution: Option<String>,

    #[graphql(name = "parsedCodec")]
    #[filterable(type = "string")]
    pub parsed_codec: Option<String>,

    #[graphql(name = "parsedSource")]
    #[filterable(type = "string")]
    pub parsed_source: Option<String>,

    #[graphql(name = "parsedAudio")]
    #[filterable(type = "string")]
    pub parsed_audio: Option<String>,

    #[graphql(name = "parsedHdr")]
    #[filterable(type = "string")]
    pub parsed_hdr: Option<String>,

    #[graphql(name = "processed")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub processed: bool,

    #[graphql(name = "torrentId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub torrent_id: Option<String>,

    #[graphql(name = "skippedReason")]
    pub skipped_reason: Option<String>,

    #[graphql(name = "seenAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub seen_at: String,

    // Query relations do not add physical FKs or change existing delete behavior.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "RssFeed",
        from = "feed_id",
        to = "id",
        on_delete = "cascade",
        propagate_change = "up",
        emit_fk = false
    )]
    pub feed: Option<Box<RssFeed>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Torrent",
        from = "torrent_id",
        to = "id",
        on_delete = "set_null",
        emit_fk = false
    )]
    pub torrent: Option<Box<Torrent>>,
}

#[derive(Default)]
pub struct RssFeedItemCustomOperations;

// =============================================================================
// Feed items as release-search results, and grabbing one
// =============================================================================

/// The prefix of the `guid` a feed item carries in search results: the release itself has no link
/// a client may see, so a result is identified (and grabbed, `addTorrent.feedItemId`) by item id.
pub const FEED_ITEM_GUID_PREFIX: &str = "feed-item:";

/// How many catalogue items one search brings back.
const FEED_SEARCH_LIMIT: i64 = 150;

fn eq_filter(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

/// A catalogue item as a release the search can grade and rank like any other. It has no link:
/// the link holds the tracker passkey.
pub fn feed_item_release(item: &RssFeedItem, feed: &RssFeed) -> SourceRelease {
    let published = item
        .pub_date
        .as_deref()
        .or(Some(item.seen_at.as_str()))
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|time| time.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);
    let mut release = SourceRelease::new(
        item.title.clone(),
        format!("{FEED_ITEM_GUID_PREFIX}{}", item.id),
        published,
    );
    release.info_hash = item.info_hash.clone();
    release.size = item.size;
    release.seeders = item.seeders;
    release.description = item.description.clone();
    release.source_id = Some(feed.id.clone());
    release.source_name = Some(format!("{} (feed)", feed.name));
    release
}

/// Whether a catalogue item answers a search narrowed by season and episode.
fn answers_numbers(item: &RssFeedItem, season: Option<i32>, episode: Option<&str>) -> bool {
    let wanted_episode = episode.and_then(|text| text.trim().parse::<i32>().ok());
    match (item.match_kind.as_deref(), season) {
        (_, None) => true,
        (Some("show"), _) => true,
        (Some("season"), Some(wanted)) => item.match_season == Some(wanted),
        (Some("episode"), Some(wanted)) => {
            item.match_season == Some(wanted)
                && wanted_episode.is_none_or(|number| item.match_episode == Some(number))
        }
        _ => false,
    }
}

/// The catalogue's answer to a release search: for a search of one item (show, movie, album,
/// audiobook), the feed items already matched to it; for a typed search, items whose titles hold
/// every word. One result per feed, marked as from a feed by its source id being a feed id.
pub(crate) async fn feed_search_pages(
    db: &crate::db::Database,
    user: &AuthUser,
    input: &SearchSourcesInput,
) -> async_graphql::Result<(Vec<PagedSourceSearchResult>, Vec<String>)> {
    let feeds: Vec<RssFeed> = RssFeed::query(db.pool())
        .filter(RssFeedWhereInput {
            user_id: (!user.is_admin()).then(|| eq_filter(&user.user_id)),
            enabled: Some(graphql_orm::graphql::filters::BoolFilter {
                eq: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .map_err(|error| async_graphql::Error::new(error.to_string()))?;
    if feeds.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut filter = RssFeedItemWhereInput {
        feed_id: Some(StringFilter {
            in_list: Some(feeds.iter().map(|feed| feed.id.clone()).collect()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let targeted = if let Some(id) = &input.show_id {
        filter.match_show_id = Some(eq_filter(id));
        true
    } else if let Some(id) = &input.movie_id {
        filter.match_movie_id = Some(eq_filter(id));
        true
    } else if let Some(id) = &input.album_id {
        filter.match_album_id = Some(eq_filter(id));
        true
    } else if let Some(id) = &input.audiobook_id {
        filter.match_audiobook_id = Some(eq_filter(id));
        true
    } else {
        false
    };
    let words = crate::services::release_match::normalize_words(&input.query);
    if !targeted {
        let Some(first) = words
            .split_whitespace()
            .max_by_key(|word| word.len())
            .map(str::to_string)
        else {
            return Ok((Vec::new(), Vec::new()));
        };
        filter.title = Some(StringFilter {
            contains: Some(first),
            ..Default::default()
        });
    }
    let items = RssFeedItem::query(db.pool())
        .filter(filter)
        .order_by(RssFeedItemOrderByInput {
            pub_date: Some(OrderDirection::Desc),
            ..Default::default()
        })
        .limit(FEED_SEARCH_LIMIT)
        .fetch_all()
        .await
        .map_err(|error| async_graphql::Error::new(error.to_string()))?;
    let by_feed: HashMap<&str, &RssFeed> =
        feeds.iter().map(|feed| (feed.id.as_str(), feed)).collect();
    let mut grouped: HashMap<String, Vec<SourceRelease>> = HashMap::new();
    for item in &items {
        let keep = if targeted {
            answers_numbers(item, input.season, input.episode.as_deref())
        } else {
            crate::services::release_match::title_has_all_words(&input.query, &item.title)
        };
        if !keep {
            continue;
        }
        if let Some(feed) = by_feed.get(item.feed_id.as_str()) {
            grouped
                .entry(feed.id.clone())
                .or_default()
                .push(feed_item_release(item, feed));
        }
    }
    let mut feed_ids = Vec::new();
    let mut pages = Vec::new();
    for feed in &feeds {
        let Some(releases) = grouped.remove(&feed.id) else {
            continue;
        };
        feed_ids.push(feed.id.clone());
        let count = releases.len() as i32;
        pages.push(PagedSourceSearchResult {
            result: SourceSearchResult {
                source_id: feed.id.clone(),
                source_name: format!("{} (feed)", feed.name),
                releases,
                elapsed_ms: 0,
                from_cache: true,
                error: None,
            },
            page_info: SourceSearchPageInfo {
                support: SourceContinuationSupport::Unsupported,
                state: SourceSearchPageState::Exhausted,
                has_more: Some(false),
                next_cursor: None,
                retry_cursor: None,
                returned_count: count,
            },
        });
    }
    Ok((pages, feed_ids))
}

/// A feed item's download link and its feed, for a caller allowed to grab from that feed (its
/// owner, or an admin). The same answer for "no such item" and "not yours".
pub(crate) async fn resolve_feed_item_for_grab(
    db: &crate::db::Database,
    manager: &Arc<ServicesManager>,
    user: &AuthUser,
    item_id: &str,
) -> async_graphql::Result<(String, RssFeed)> {
    let deny = || async_graphql::Error::new("Feed item not found or access denied");
    let item = RssFeedItem::get(db.pool(), &item_id.to_string())
        .await
        .map_err(|_| deny())?
        .ok_or_else(deny)?;
    let feed = RssFeed::get(db.pool(), &item.feed_id)
        .await
        .map_err(|_| deny())?
        .ok_or_else(deny)?;
    if !user.is_admin() && feed.user_id != user.user_id {
        return Err(deny());
    }
    let link = crate::jobs::rss_poll::reveal_item_link(manager, &item.link)
        .await
        .map_err(|error| {
            async_graphql::Error::new(crate::services::redact::redact_text(&format!("{error:#}")))
        })?;
    Ok((link, feed))
}
