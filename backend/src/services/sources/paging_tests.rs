//! Fixture-only paging checks. No sockets, remote queries, or credentials.
use super::*;
use crate::services::sources::{QueryType, SourceCapabilities, SourceType, TrackerType};
use async_graphql::async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};

struct FakeSource {
    id: String,
    caps: SourceCapabilities,
    calls: parking_lot::Mutex<Vec<i32>>,
    fail_next: AtomicBool,
    repeat: bool,
    unsupported: bool,
}
impl FakeSource {
    fn new(id: &str) -> Self {
        Self {
            id: id.into(),
            caps: SourceCapabilities {
                search_available: true,
                ..Default::default()
            },
            calls: parking_lot::Mutex::new(vec![]),
            fail_next: AtomicBool::new(false),
            repeat: false,
            unsupported: false,
        }
    }
}
fn release(hash: &str) -> SourceRelease {
    let mut release = SourceRelease::new(
        format!("Fixture {hash}"),
        "shared-details-guid".into(),
        chrono::Utc::now(),
    );
    release.info_hash = Some(hash.into());
    release
}
#[async_trait]
impl Source for FakeSource {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.id
    }
    fn source_type(&self) -> SourceType {
        SourceType::TorrentIndexer
    }
    fn definition_id(&self) -> &str {
        "fixture"
    }
    fn site_link(&self) -> &str {
        "https://fixture.invalid/"
    }
    fn tracker_type(&self) -> TrackerType {
        TrackerType::Public
    }
    fn capabilities(&self) -> &SourceCapabilities {
        &self.caps
    }
    fn is_configured(&self) -> bool {
        true
    }
    async fn test_connection(&self) -> Result<bool> {
        Ok(true)
    }
    async fn download(&self, _: &str) -> Result<Vec<u8>> {
        unreachable!("no network in paging fixtures")
    }
    async fn search(&self, _: &SourceQuery) -> Result<Vec<SourceRelease>> {
        Ok(vec![release("a")])
    }
    async fn search_page(&self, query: &SourceQuery) -> Result<SourcePage> {
        let offset = query.offset.unwrap_or(0);
        self.calls.lock().push(offset);
        if self.fail_next.swap(false, Ordering::SeqCst) {
            anyhow::bail!("https://fixture.invalid/?apikey=DO-NOT-EXPOSE");
        }
        if self.unsupported {
            return Ok(SourcePage::unsupported(vec![release("a")]));
        }
        let (releases, state, next_offset) = if offset == 0 || self.repeat {
            (
                vec![release("a"), release("a"), release("b")],
                SourceSearchPageState::More,
                Some(offset + 2),
            )
        } else {
            (
                vec![release("b"), release("c")],
                SourceSearchPageState::Exhausted,
                None,
            )
        };
        Ok(SourcePage {
            releases,
            support: SourceContinuationSupport::Supported,
            state,
            next_offset,
        })
    }
}
fn manager(sources: &[Arc<FakeSource>]) -> SourcesManager {
    let manager = SourcesManager::new(&CredentialEncryption::generate_key()).unwrap();
    for source in sources {
        manager
            .sources
            .write()
            .insert(source.id.clone(), source.clone());
        manager
            .generations
            .write()
            .insert(source.id.clone(), format!("generation-{}", source.id));
    }
    manager
}
fn query() -> SourceQuery {
    SourceQuery {
        query_type: QueryType::Search,
        search_term: Some("fixture".into()),
        limit: Some(2),
        ..Default::default()
    }
}

#[tokio::test]
async fn source_pages_advance_and_deduplicate_without_collapsing_hash_variants() {
    let source = Arc::new(FakeSource::new("a"));
    let manager = manager(std::slice::from_ref(&source));
    let first = manager
        .search_paged("owner", "request", None, &query(), &[])
        .await
        .unwrap()
        .remove(0);
    assert_eq!(first.result.releases.len(), 2);
    assert_eq!(first.page_info.has_more, Some(true));
    let token = first.page_info.next_cursor.unwrap();
    assert!(!token.contains("fixture"));
    let second = manager
        .search_paged("owner", "request", None, &query(), &[("a".into(), token)])
        .await
        .unwrap()
        .remove(0);
    assert_eq!(second.result.releases.len(), 1);
    assert_eq!(second.result.releases[0].info_hash.as_deref(), Some("c"));
    assert_eq!(second.page_info.state, SourceSearchPageState::Exhausted);
    assert_eq!(second.page_info.has_more, Some(false));
    assert!(second.page_info.next_cursor.is_none());
    assert_eq!(*source.calls.lock(), [0, 2]);
    assert_ne!(release("a").stable_id("a"), release("b").stable_id("a"));
    assert_ne!(release("a").stable_id("a"), release("a").stable_id("b"));
    assert_eq!(release("AbC").stable_id("a"), release("abc").stable_id("a"));
}

#[tokio::test]
async fn cursor_is_bound_to_principal_request_source_epoch_and_query() {
    let a = Arc::new(FakeSource::new("a"));
    let b = Arc::new(FakeSource::new("b"));
    let manager = manager(&[a.clone(), b.clone()]);
    let first = manager
        .search_paged(
            "owner:member",
            "profile-v1",
            Some(&["a".into()]),
            &query(),
            &[],
        )
        .await
        .unwrap()
        .remove(0);
    let token = first.page_info.next_cursor.unwrap();
    for (principal, binding, source_id) in [
        ("other:member", "profile-v1", "a"),
        ("owner:admin", "profile-v1", "a"),
        ("owner:member", "profile-v2", "a"),
        ("owner:member", "profile-v1", "b"),
    ] {
        assert!(
            manager
                .search_paged(
                    principal,
                    binding,
                    None,
                    &query(),
                    &[(source_id.into(), token.clone())]
                )
                .await
                .is_err()
        );
    }
    let mut changed = query();
    changed.season = Some(2);
    assert!(
        manager
            .search_paged(
                "owner:member",
                "profile-v1",
                None,
                &changed,
                &[("a".into(), token.clone())]
            )
            .await
            .is_err()
    );
    manager
        .generations
        .write()
        .insert("a".into(), "reloaded".into());
    assert!(
        manager
            .search_paged(
                "owner:member",
                "profile-v1",
                None,
                &query(),
                &[("a".into(), token)]
            )
            .await
            .is_err()
    );
    assert_eq!(*a.calls.lock(), [0]);
    assert!(b.calls.lock().is_empty());
}

#[tokio::test]
async fn independent_sources_continue_or_retry_without_restarting_other_sources() {
    let a = Arc::new(FakeSource::new("a"));
    let b = Arc::new(FakeSource::new("b"));
    b.fail_next.store(true, Ordering::SeqCst);
    let manager = manager(&[a.clone(), b.clone()]);
    let first = manager
        .search_paged(
            "owner",
            "binding",
            Some(&["a".into(), "b".into()]),
            &query(),
            &[],
        )
        .await
        .unwrap();
    let a_page = first
        .iter()
        .find(|page| page.result.source_id == "a")
        .unwrap();
    let b_page = first
        .iter()
        .find(|page| page.result.source_id == "b")
        .unwrap();
    assert_eq!(b_page.page_info.state, SourceSearchPageState::Error);
    assert_ne!(b_page.page_info.has_more, Some(false));
    assert!(
        !b_page
            .result
            .error
            .as_deref()
            .unwrap()
            .contains("DO-NOT-EXPOSE")
    );
    let retry = b_page.page_info.retry_cursor.clone().unwrap();
    let retried = manager
        .search_paged(
            "owner",
            "binding",
            None,
            &query(),
            &[("b".into(), retry.clone())],
        )
        .await
        .unwrap();
    assert_eq!(retried.len(), 1);
    assert_eq!(retried[0].page_info.state, SourceSearchPageState::More);
    assert_eq!(*a.calls.lock(), [0]);
    assert_eq!(*b.calls.lock(), [0, 0]);
    let continued = manager
        .search_paged(
            "owner",
            "binding",
            None,
            &query(),
            &[("a".into(), a_page.page_info.next_cursor.clone().unwrap())],
        )
        .await
        .unwrap();
    assert_eq!(continued[0].page_info.has_more, Some(false));
    assert_eq!(*b.calls.lock(), [0, 0]);
    // The retry token is immutable and reusable; it never consumes/advances B.
    manager
        .search_paged("owner", "binding", None, &query(), &[("b".into(), retry)])
        .await
        .unwrap();
    assert_eq!(*b.calls.lock(), [0, 0, 0]);
}

#[tokio::test]
async fn unsupported_short_pages_and_repeated_pages_never_claim_exhaustion() {
    let mut unsupported = FakeSource::new("unsupported");
    unsupported.unsupported = true;
    let mut repeated = FakeSource::new("repeated");
    repeated.repeat = true;
    let manager = manager(&[Arc::new(unsupported), Arc::new(repeated)]);
    let first = manager
        .search_paged(
            "owner",
            "binding",
            Some(&["unsupported".into(), "repeated".into()]),
            &query(),
            &[],
        )
        .await
        .unwrap();
    assert_eq!(
        first[0].page_info.support,
        SourceContinuationSupport::Unsupported
    );
    assert_eq!(first[0].page_info.state, SourceSearchPageState::Unknown);
    assert_eq!(first[0].page_info.has_more, None);
    assert!(first[0].page_info.next_cursor.is_none());
    let second = manager
        .search_paged(
            "owner",
            "binding",
            None,
            &query(),
            &[(
                "repeated".into(),
                first[1].page_info.next_cursor.clone().unwrap(),
            )],
        )
        .await
        .unwrap();
    assert!(second[0].result.releases.is_empty());
    assert_eq!(second[0].page_info.state, SourceSearchPageState::Partial);
    assert_eq!(second[0].page_info.has_more, None);
    assert!(second[0].page_info.next_cursor.is_none());
}

#[test]
fn expired_and_capacity_limited_cursors_fail_honestly() {
    let mut store = ContinuationStore::default();
    let cursor = Cursor {
        principal: "owner".into(),
        binding: "request".into(),
        source_id: "source".into(),
        generation: "generation".into(),
        offset: 0,
        seen: Arc::new(Seen::default()),
        expires_at: Instant::now() - Duration::from_secs(1),
    };
    let token = store.insert(cursor).unwrap();
    assert!(
        store
            .resolve(&token, "owner", "request", "source", "generation")
            .is_err()
    );
    store.entries.clear();
    for i in 0..MAX_CURSORS {
        store.entries.insert(
            i.to_string(),
            Cursor {
                principal: "owner".into(),
                binding: "request".into(),
                source_id: "source".into(),
                generation: "generation".into(),
                offset: 0,
                seen: Arc::new(Seen::default()),
                expires_at: Instant::now() + CURSOR_TTL,
            },
        );
    }
    let cursor = store.entries.values().next().unwrap().clone();
    assert!(store.insert(cursor).is_none());
}

#[tokio::test]
async fn invalid_multi_source_batch_is_rejected_before_any_provider_call() {
    let a = Arc::new(FakeSource::new("a"));
    let b = Arc::new(FakeSource::new("b"));
    let manager = manager(&[a.clone(), b.clone()]);
    let first = manager
        .search_paged("owner", "binding", Some(&["a".into()]), &query(), &[])
        .await
        .unwrap()
        .remove(0);
    let token = first.page_info.next_cursor.unwrap();
    assert!(
        manager
            .search_paged(
                "owner",
                "binding",
                None,
                &query(),
                &[("a".into(), token), ("b".into(), "invalid".into())]
            )
            .await
            .is_err()
    );
    assert_eq!(*a.calls.lock(), [0]);
    assert!(b.calls.lock().is_empty());
    assert!(
        manager
            .search_paged(
                "owner",
                "binding",
                Some(&["a".into(), "a".into()]),
                &query(),
                &[]
            )
            .await
            .is_err()
    );
    let mut invalid_limit = query();
    invalid_limit.limit = Some(101);
    assert!(
        manager
            .search_paged("owner", "binding", None, &invalid_limit, &[])
            .await
            .is_err()
    );
}
