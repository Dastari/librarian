//! Request-bound external search continuation. These bounded, ephemeral handles
//! contain no domain persistence, credentials, provider URLs, or claimed snapshot.
use super::*;
use crate::services::sources::{
    PagedSourceSearchResult, SourceContinuationSupport, SourcePage, SourceSearchPageInfo,
    SourceSearchPageState,
};
use std::collections::HashSet;

const CURSOR_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_CURSORS: usize = 1024;
const MAX_PAGE_RELEASES: usize = 1000;
const MAX_SEEN_RELEASES: usize = 10_000;
const MAX_PAGES: usize = 100;
const MAX_SOURCES: usize = 64;

#[derive(Default)]
struct Seen {
    parent: Option<Arc<Seen>>,
    ids: Vec<String>,
    total: usize,
    depth: usize,
}

impl Seen {
    fn known_ids(&self) -> HashSet<&str> {
        let mut ids = HashSet::with_capacity(self.total);
        let mut current = Some(self);
        while let Some(seen) = current {
            ids.extend(seen.ids.iter().map(String::as_str));
            current = seen.parent.as_deref();
        }
        ids
    }
}

#[derive(Clone)]
struct Cursor {
    principal: String,
    binding: String,
    source_id: String,
    generation: String,
    offset: i32,
    seen: Arc<Seen>,
    expires_at: Instant,
}

#[derive(Default)]
pub(super) struct ContinuationStore {
    entries: HashMap<String, Cursor>,
}

impl ContinuationStore {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }

    fn insert(&mut self, cursor: Cursor) -> Option<String> {
        self.entries
            .retain(|_, value| value.expires_at > Instant::now());
        if self.entries.len() >= MAX_CURSORS {
            return None;
        }
        let token = uuid::Uuid::new_v4().to_string();
        self.entries.insert(token.clone(), cursor);
        Some(token)
    }

    fn resolve(
        &self,
        token: &str,
        principal: &str,
        binding: &str,
        source: &str,
        generation: &str,
    ) -> Result<Cursor> {
        if token.len() != 36 || uuid::Uuid::parse_str(token).is_err() {
            anyhow::bail!("Invalid or expired source continuation; restart this search");
        }
        let cursor = self
            .entries
            .get(token)
            .filter(|cursor| cursor.expires_at > Instant::now())
            .filter(|cursor| {
                cursor.principal == principal
                    && cursor.binding == binding
                    && cursor.source_id == source
                    && cursor.generation == generation
            })
            .ok_or_else(|| {
                anyhow!("Invalid or expired source continuation; restart this search")
            })?;
        Ok(cursor.clone())
    }
}

struct Plan {
    source_id: String,
    source: Option<Arc<dyn Source>>,
    cursor: Cursor,
    retry_cursor: Option<String>,
}

impl SourcesManager {
    /// Continue only the named sources. Request/principal/profile binding is
    /// supplied by the authenticated resolver, never accepted from a client.
    /// Remote result sets can change: stable IDs suppress repeats, but this is
    /// not a snapshot or an exhaustive count. Legacy release-only cache is not
    /// used because it cannot preserve remote page metadata.
    pub async fn search_paged(
        &self,
        principal: &str,
        binding: &str,
        source_ids: Option<&[String]>,
        query: &SourceQuery,
        continuations: &[(String, String)],
    ) -> Result<Vec<PagedSourceSearchResult>> {
        if source_ids.is_some_and(|ids| ids.len() > MAX_SOURCES)
            || continuations.len() > MAX_SOURCES
        {
            anyhow::bail!("At most {MAX_SOURCES} search sources may be requested");
        }
        if query.limit.is_some_and(|limit| !(1..=100).contains(&limit)) {
            anyhow::bail!("Source search limit must be between 1 and 100");
        }
        if source_ids.is_some_and(|ids| ids.iter().collect::<HashSet<_>>().len() != ids.len()) {
            anyhow::bail!("Duplicate search source IDs");
        }
        let binding = format!("{binding}:{}", query.cache_key());
        let ids = if continuations.is_empty() {
            source_ids.map(|ids| ids.to_vec()).unwrap_or_else(|| {
                self.get_all_sources_by_priority()
                    .into_iter()
                    .filter(|(_, source)| source.can_handle_query(query))
                    .map(|(id, _)| id)
                    .take(MAX_SOURCES + 1)
                    .collect()
            })
        } else {
            let ids: Vec<_> = continuations.iter().map(|(id, _)| id.clone()).collect();
            if let Some(selected) = source_ids {
                let selected: HashSet<_> = selected.iter().collect();
                let continued: HashSet<_> = ids.iter().collect();
                if selected != continued {
                    anyhow::bail!("Continuation sources must match sourceIds");
                }
            }
            ids
        };
        if ids.len() > MAX_SOURCES {
            anyhow::bail!("Select at most {MAX_SOURCES} search sources");
        }
        if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
            anyhow::bail!("Duplicate search source IDs");
        }
        // Validate the entire batch before any network work, so a bad second
        // cursor cannot advance the first source as a side effect.
        let mut plans = Vec::new();
        for id in ids {
            let (source, generation) = {
                let sources = self.sources.read();
                (
                    sources.get(&id).cloned(),
                    self.generations
                        .read()
                        .get(&id)
                        .cloned()
                        .unwrap_or_default(),
                )
            };
            let mut store = self.continuations.write();
            let (cursor, retry_cursor) =
                if let Some((_, token)) = continuations.iter().find(|(source, _)| *source == id) {
                    (
                        store.resolve(token, principal, &binding, &id, &generation)?,
                        Some(token.clone()),
                    )
                } else {
                    let cursor = Cursor {
                        principal: principal.into(),
                        binding: binding.clone(),
                        source_id: id.clone(),
                        generation,
                        offset: 0,
                        seen: Arc::new(Seen::default()),
                        expires_at: Instant::now() + CURSOR_TTL,
                    };
                    let token = store.insert(cursor.clone());
                    (cursor, token)
                };
            plans.push(Plan {
                source_id: id,
                source,
                cursor,
                retry_cursor,
            });
        }
        Ok(
            futures::future::join_all(plans.into_iter().map(|plan| self.search_plan(plan, query)))
                .await,
        )
    }

    async fn search_plan(&self, plan: Plan, query: &SourceQuery) -> PagedSourceSearchResult {
        let start = Instant::now();
        let name = plan
            .source
            .as_ref()
            .map(|source| source.name().to_string())
            .unwrap_or_else(|| plan.source_id.clone());
        let mut page_query = query.clone();
        page_query.offset = Some(plan.cursor.offset);
        page_query.limit = Some(query.limit.unwrap_or(50));
        let limiter = self.rate_limiters.read().get(&plan.source_id).cloned();
        let response = if let Some(source) = plan
            .source
            .as_ref()
            .filter(|source| source.is_configured() && source.can_handle_query(query))
        {
            tokio::time::timeout(SOURCE_SEARCH_TIMEOUT, async {
                let _permit = if let Some(limiter) = &limiter {
                    Some(limiter.acquire().await)
                } else {
                    None
                };
                source.search_page(&page_query).await
            })
            .await
            .map_err(|_| anyhow!("Source search timed out"))
            .and_then(|result| result)
        } else {
            Err(anyhow!("Source unavailable for this search"))
        };
        let (mut page, mut error) = match response {
            Ok(page) => (page, None),
            Err(_) => {
                // Provider errors can contain passkeys, API URLs, cookies, or
                // response bodies. Keep the public error stable and redacted.
                tracing::warn!(source_id = %plan.source_id, source_name = %name, "Source page failed; retry remains on the same page");
                (
                    SourcePage {
                        releases: vec![],
                        support: SourceContinuationSupport::Unknown,
                        state: SourceSearchPageState::Error,
                        next_offset: None,
                    },
                    Some("Source search failed or is unavailable; retry this page".into()),
                )
            }
        };
        if self
            .generations
            .read()
            .get(&plan.source_id)
            .map(String::as_str)
            != Some(plan.cursor.generation.as_str())
            && plan.source.is_some()
        {
            page.releases.clear();
            page.state = SourceSearchPageState::Error;
            page.next_offset = None;
            error = Some("Source configuration changed; restart this search".into());
        }
        let raw_count = page.releases.len();
        let over_limit = raw_count > MAX_PAGE_RELEASES;
        page.releases.truncate(MAX_PAGE_RELEASES);
        let known_ids = plan.cursor.seen.known_ids();
        let mut ids = HashSet::new();
        page.releases.retain(|release| {
            let id = release.stable_id(&plan.source_id);
            !known_ids.contains(id.as_str()) && ids.insert(id)
        });
        for release in &mut page.releases {
            release.source_id = Some(plan.source_id.clone());
            release.source_name = Some(name.clone());
        }
        let mut next_cursor = None;
        if raw_count > 0 && ids.is_empty() && page.next_offset.is_some() {
            page.state = SourceSearchPageState::Partial;
            page.next_offset = None;
            error = Some("Source repeated an earlier page; refine or restart the search".into());
        }
        if page.state == SourceSearchPageState::More && page.next_offset.is_none() {
            page.state = SourceSearchPageState::Partial;
        }
        if over_limit
            || plan.cursor.seen.total + ids.len() > MAX_SEEN_RELEASES
            || (page.next_offset.is_some() && plan.cursor.seen.depth + 1 >= MAX_PAGES)
        {
            page.state = SourceSearchPageState::Partial;
            page.next_offset = None;
            error = Some("Search continuation budget reached; narrow the search".into());
        } else if let Some(offset) = page.next_offset {
            if offset <= plan.cursor.offset {
                page.state = SourceSearchPageState::Partial;
                error = Some("Source paging did not advance; retry or refine the search".into());
            } else {
                let next = Cursor {
                    offset,
                    seen: Arc::new(Seen {
                        total: plan.cursor.seen.total + ids.len(),
                        depth: plan.cursor.seen.depth + 1,
                        parent: Some(plan.cursor.seen.clone()),
                        ids: ids.into_iter().collect(),
                    }),
                    ..plan.cursor.clone()
                };
                next_cursor = self.continuations.write().insert(next);
                if next_cursor.is_none() {
                    page.state = SourceSearchPageState::Partial;
                    error = Some("Search continuation capacity reached; retry later".into());
                }
            }
        }
        let has_more = match page.state {
            SourceSearchPageState::More if next_cursor.is_some() => Some(true),
            SourceSearchPageState::Exhausted => Some(false),
            _ => None,
        };
        let returned_count = page.releases.len() as i32;
        PagedSourceSearchResult {
            result: SourceSearchResult {
                source_id: plan.source_id,
                source_name: name,
                releases: page.releases,
                elapsed_ms: start.elapsed().as_millis() as u64,
                from_cache: false,
                error,
            },
            page_info: SourceSearchPageInfo {
                support: page.support,
                state: page.state,
                has_more,
                next_cursor,
                retry_cursor: plan.retry_cursor,
                returned_count,
            },
        }
    }
}

#[cfg(test)]
#[path = "paging_tests.rs"]
mod tests;
