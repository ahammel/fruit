use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use exn::Exn;
use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;

use super::*;
use crate::{
    community_repo::{CommunityPersistor, CommunityProvider},
    error::Error,
    event_log::{Effect, Event, Record},
};

fn err() -> Exn<Error> {
    Exn::new(Error::GrantInterrupted("test error".to_string()))
}

// --- mock repo that always errors ---

struct ErrorRepo;

#[async_trait]
impl CommunityProvider for ErrorRepo {
    type Error = Error;
    async fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Err(err())
    }
    async fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Err(err())
    }
}

#[async_trait]
impl CommunityPersistor for ErrorRepo {
    type Error = Error;
    async fn put(&self, _: Community) -> Result<Community, Exn<Error>> {
        Err(err())
    }
}

impl CommunityRepo for ErrorRepo {}

// --- mock repo that returns one community then fails puts ---

struct GetOkPutErrorRepo {
    community: Community,
}

#[async_trait]
impl CommunityProvider for GetOkPutErrorRepo {
    type Error = Error;
    async fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }
    async fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Ok(Some(self.community.clone()))
    }
}

#[async_trait]
impl CommunityPersistor for GetOkPutErrorRepo {
    type Error = Error;
    async fn put(&self, _: Community) -> Result<Community, Exn<Error>> {
        Err(err())
    }
}

impl CommunityRepo for GetOkPutErrorRepo {}

// --- mock event log that always errors ---

struct ErrorEventLog;

#[async_trait]
impl EventLogProvider for ErrorEventLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Err(err())
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Err(err())
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Err(err())
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Err(err())
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Err(err())
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Err(err())
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Err(err())
    }
}

// --- mock repo that returns None for get_latest ---

struct NoneLatestRepo;

#[async_trait]
impl CommunityProvider for NoneLatestRepo {
    type Error = Error;
    async fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }
    async fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }
}

#[async_trait]
impl CommunityPersistor for NoneLatestRepo {
    type Error = Error;
    async fn put(&self, c: Community) -> Result<Community, Exn<Error>> {
        Ok(c)
    }
}

impl CommunityRepo for NoneLatestRepo {}

// --- mock event log that returns empty effects ---

struct EmptyEffectsEventLog;

#[async_trait]
impl EventLogProvider for EmptyEffectsEventLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

// --- mock event log that returns one effect ---

struct OneEffectEventLog {
    effect: Effect,
}

#[async_trait]
impl EventLogProvider for OneEffectEventLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![self.effect.clone()])
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

// --- error propagation tests ---

#[tokio::test]
async fn init_propagates_put_error() {
    let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
    assert!(store.init().await.is_err());
}

#[tokio::test]
async fn get_propagates_repo_error() {
    let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
    assert!(store
        .get(CommunityId::new(), SequenceId::zero())
        .await
        .is_err());
}

#[tokio::test]
async fn get_latest_propagates_repo_error() {
    let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
    assert!(store.get_latest(CommunityId::new()).await.is_err());
}

#[tokio::test]
async fn get_latest_propagates_event_log_error() {
    let community = Community::new();
    let id = community.id;
    let store = CommunityStore::new(GetOkPutErrorRepo { community }, ErrorEventLog);
    assert!(store.get_latest(id).await.is_err());
}

#[tokio::test]
async fn get_latest_propagates_put_error_after_applying_effects() {
    let community = Community::new();
    let id = community.id;
    let effect = Effect {
        id: SequenceId::new(1),
        community_id: id,
        mutations: vec![],
    };
    let store = CommunityStore::new(
        GetOkPutErrorRepo { community },
        OneEffectEventLog { effect },
    );
    assert!(store.get_latest(id).await.is_err());
}

#[tokio::test]
async fn get_with_get_ok_put_error_repo_returns_none() {
    let community = Community::new();
    let id = community.id;
    let store = CommunityStore::new(GetOkPutErrorRepo { community }, ErrorEventLog);
    assert!(store.get(id, SequenceId::zero()).await.unwrap().is_none());
}

#[tokio::test]
async fn get_latest_returns_none_when_community_not_found() {
    // Covers the `else { return Ok(None) }` branch for the NoneLatestRepo monomorphization.
    let store = CommunityStore::new(NoneLatestRepo, EmptyEffectsEventLog);
    assert!(store
        .get_latest(CommunityId::new())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn get_latest_returns_community_when_no_pending_effects() {
    // Covers the `community.version == initial_version` early-return branch: no
    // effects are returned so version never advances and we skip the put.
    let community = Community::new();
    let id = community.id;
    let store = CommunityStore::new(
        GetOkPutErrorRepo {
            community: community.clone(),
        },
        EmptyEffectsEventLog,
    );
    assert_eq!(store.get_latest(id).await.unwrap(), Some(community));
}

// --- mock repo that returns one community and accepts puts ---

struct GetOkPutOkRepo {
    community: Community,
}

#[async_trait]
impl CommunityProvider for GetOkPutOkRepo {
    type Error = Error;
    async fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }
    async fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Ok(Some(self.community.clone()))
    }
}

#[async_trait]
impl CommunityPersistor for GetOkPutOkRepo {
    type Error = Error;
    async fn put(&self, c: Community) -> Result<Community, Exn<Error>> {
        Ok(c)
    }
}

impl CommunityRepo for GetOkPutOkRepo {}

// --- mock event log that returns exactly EFFECTS_PAGE_SIZE effects on the first
//     call, then returns empty on subsequent calls ---

struct MultiPageEffectsEventLog {
    effects: Vec<Effect>,
    call_count: AtomicUsize,
}

impl MultiPageEffectsEventLog {
    fn new(community_id: CommunityId) -> Self {
        let effects = (1..=EFFECTS_PAGE_SIZE as u64)
            .map(|i| Effect {
                id: SequenceId::new(i),
                community_id,
                mutations: vec![],
            })
            .collect();
        Self {
            effects,
            call_count: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl EventLogProvider for MultiPageEffectsEventLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        let n = self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(if n == 0 { self.effects.clone() } else { vec![] })
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

// --- mock event log that returns exactly EFFECTS_PAGE_SIZE effects on the first
//     call, then one more effect on the second call, then empty thereafter ---

struct TwoPageEffectsEventLog {
    first_page: Vec<Effect>,
    second_page: Vec<Effect>,
    call_count: AtomicUsize,
}

impl TwoPageEffectsEventLog {
    fn new(community_id: CommunityId) -> Self {
        let first_page = (1..=EFFECTS_PAGE_SIZE as u64)
            .map(|i| Effect {
                id: SequenceId::new(i),
                community_id,
                mutations: vec![],
            })
            .collect();
        let second_page = vec![Effect {
            id: SequenceId::new(EFFECTS_PAGE_SIZE as u64 + 1),
            community_id,
            mutations: vec![],
        }];
        Self {
            first_page,
            second_page,
            call_count: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl EventLogProvider for TwoPageEffectsEventLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        let n = self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(match n {
            0 => self.first_page.clone(),
            1 => self.second_page.clone(),
            _ => vec![],
        })
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn get_latest_paginates_through_effects_spanning_two_full_pages() {
    // First page is exactly EFFECTS_PAGE_SIZE (loop must continue); second page has
    // one more effect. Final version must be PAGE_SIZE+1, not PAGE_SIZE — killing
    // the `< → <=` mutation which would stop after the first page.
    let community = Community::new();
    let id = community.id;
    let event_log = TwoPageEffectsEventLog::new(id);
    let expected_version = SequenceId::new(EFFECTS_PAGE_SIZE as u64 + 1);
    let store = CommunityStore::new(GetOkPutOkRepo { community }, event_log);
    let result = store.get_latest(id).await.unwrap().unwrap();
    assert_eq!(result.version, expected_version);
}

// --- mock event log that returns exactly EFFECTS_PAGE_SIZE effects on the first
//     call, then errors on the second call ---

struct FirstPageThenErrorEventLog {
    effects: Vec<Effect>,
    call_count: AtomicUsize,
}

impl FirstPageThenErrorEventLog {
    fn new(community_id: CommunityId) -> Self {
        Self::with_first_page_size(community_id, EFFECTS_PAGE_SIZE)
    }

    fn with_first_page_size(community_id: CommunityId, n: usize) -> Self {
        let effects = (1..=n as u64)
            .map(|i| Effect {
                id: SequenceId::new(i),
                community_id,
                mutations: vec![],
            })
            .collect();
        Self {
            effects,
            call_count: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl EventLogProvider for FirstPageThenErrorEventLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        let n = self.call_count.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            Ok(self.effects.clone())
        } else {
            Err(err())
        }
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn get_latest_paginates_through_all_effects() {
    // Covers the loop-continues branch: the first page is exactly EFFECTS_PAGE_SIZE,
    // so the loop runs a second time before breaking on the empty second page.
    let community = Community::new();
    let id = community.id;
    let event_log = MultiPageEffectsEventLog::new(id);
    let expected_version = SequenceId::new(EFFECTS_PAGE_SIZE as u64);
    let store = CommunityStore::new(GetOkPutOkRepo { community }, event_log);
    let result = store.get_latest(id).await.unwrap().unwrap();
    assert_eq!(result.version, expected_version);
}

#[tokio::test]
async fn get_latest_partial_page_breaks_without_fetching_second_batch() {
    // With `||`, exhausted=true on the first (partial) batch causes an immediate break
    // before any second fetch. The `&&` mutant would not break here because the version
    // advanced, and would then hit the error on the second call.
    let community = Community::new();
    let id = community.id;
    let event_log = FirstPageThenErrorEventLog::with_first_page_size(id, 1);
    let store = CommunityStore::new(GetOkPutOkRepo { community }, event_log);
    let result = store.get_latest(id).await.unwrap().unwrap();
    assert_eq!(result.version, SequenceId::new(1));
}

#[tokio::test]
async fn get_latest_error_on_second_batch_includes_batch_number_in_message() {
    let community = Community::new();
    let id = community.id;
    let event_log = FirstPageThenErrorEventLog::new(id);
    let store = CommunityStore::new(GetOkPutOkRepo { community }, event_log);
    let err = store.get_latest(id).await.unwrap_err();
    assert_eq!(
        err.to_string(),
        "Storage layer error: failed to retrieve effects for community at batch number 2"
    );
}
