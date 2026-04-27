use std::cell::Cell;

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
    Exn::new(Error::RetryableStorageLayerError {
        message: "test error".to_string(),
    })
}

// --- mock repo that always errors ---

struct ErrorRepo;

impl CommunityProvider for ErrorRepo {
    type Error = Error;
    fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Err(err())
    }
    fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Err(err())
    }
}

impl CommunityPersistor for ErrorRepo {
    type Error = Error;
    fn put(&self, _: Community) -> Result<Community, Exn<Error>> {
        Err(err())
    }
}

impl CommunityRepo for ErrorRepo {}

// --- mock repo that returns one community then fails puts ---

struct GetOkPutErrorRepo {
    community: Community,
}

impl CommunityProvider for GetOkPutErrorRepo {
    type Error = Error;
    fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }
    fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Ok(Some(self.community.clone()))
    }
}

impl CommunityPersistor for GetOkPutErrorRepo {
    type Error = Error;
    fn put(&self, _: Community) -> Result<Community, Exn<Error>> {
        Err(err())
    }
}

impl CommunityRepo for GetOkPutErrorRepo {}

// --- mock event log that always errors ---

struct ErrorEventLog;

impl EventLogProvider for ErrorEventLog {
    type Error = Error;
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Exn<Error>> {
        Err(err())
    }
    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Exn<Error>> {
        Err(err())
    }
    fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Err(err())
    }
    fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Err(err())
    }
    fn get_latest_grant_events(&self, _: CommunityId, _: usize) -> Result<Vec<Event>, Exn<Error>> {
        Err(err())
    }
    fn get_latest_gift_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Exn<Error>> {
        Err(err())
    }
    fn get_records_between(
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

impl CommunityProvider for NoneLatestRepo {
    type Error = Error;
    fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }
    fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }
}

impl CommunityPersistor for NoneLatestRepo {
    type Error = Error;
    fn put(&self, c: Community) -> Result<Community, Exn<Error>> {
        Ok(c)
    }
}

impl CommunityRepo for NoneLatestRepo {}

// --- mock event log that returns empty effects ---

struct EmptyEffectsEventLog;

impl EventLogProvider for EmptyEffectsEventLog {
    type Error = Error;
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_latest_grant_events(&self, _: CommunityId, _: usize) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_latest_gift_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_records_between(
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

impl EventLogProvider for OneEffectEventLog {
    type Error = Error;
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![self.effect.clone()])
    }
    fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_latest_grant_events(&self, _: CommunityId, _: usize) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_latest_gift_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

// --- error propagation tests ---

#[test]
fn init_propagates_put_error() {
    let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
    assert!(store.init().is_err());
}

#[test]
fn get_propagates_repo_error() {
    let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
    assert!(store.get(CommunityId::new(), SequenceId::zero()).is_err());
}

#[test]
fn get_latest_propagates_repo_error() {
    let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
    assert!(store.get_latest(CommunityId::new()).is_err());
}

#[test]
fn get_latest_propagates_event_log_error() {
    let community = Community::new();
    let id = community.id;
    let store = CommunityStore::new(GetOkPutErrorRepo { community }, ErrorEventLog);
    assert!(store.get_latest(id).is_err());
}

#[test]
fn get_latest_propagates_put_error_after_applying_effects() {
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
    assert!(store.get_latest(id).is_err());
}

#[test]
fn get_with_get_ok_put_error_repo_returns_none() {
    let community = Community::new();
    let id = community.id;
    let store = CommunityStore::new(GetOkPutErrorRepo { community }, ErrorEventLog);
    assert!(store.get(id, SequenceId::zero()).unwrap().is_none());
}

#[test]
fn get_latest_returns_none_when_community_not_found() {
    // Covers the `else { return Ok(None) }` branch for the NoneLatestRepo monomorphization.
    let store = CommunityStore::new(NoneLatestRepo, EmptyEffectsEventLog);
    assert!(store.get_latest(CommunityId::new()).unwrap().is_none());
}

#[test]
fn get_latest_returns_community_when_no_pending_effects() {
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
    assert_eq!(store.get_latest(id).unwrap(), Some(community));
}

// --- mock repo that returns one community and accepts puts ---

struct GetOkPutOkRepo {
    community: Community,
}

impl CommunityProvider for GetOkPutOkRepo {
    type Error = Error;
    fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }
    fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Ok(Some(self.community.clone()))
    }
}

impl CommunityPersistor for GetOkPutOkRepo {
    type Error = Error;
    fn put(&self, c: Community) -> Result<Community, Exn<Error>> {
        Ok(c)
    }
}

impl CommunityRepo for GetOkPutOkRepo {}

// --- mock event log that returns exactly EFFECTS_PAGE_SIZE effects on the first
//     call, then returns empty on subsequent calls ---

struct MultiPageEffectsEventLog {
    effects: Vec<Effect>,
    call_count: Cell<usize>,
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
            call_count: Cell::new(0),
        }
    }
}

impl EventLogProvider for MultiPageEffectsEventLog {
    type Error = Error;
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        let n = self.call_count.get();
        self.call_count.set(n + 1);
        Ok(if n == 0 { self.effects.clone() } else { vec![] })
    }
    fn get_latest_grant_events(&self, _: CommunityId, _: usize) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_latest_gift_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_records_before(
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
    call_count: Cell<usize>,
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
            call_count: Cell::new(0),
        }
    }
}

impl EventLogProvider for TwoPageEffectsEventLog {
    type Error = Error;
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        let n = self.call_count.get();
        self.call_count.set(n + 1);
        Ok(match n {
            0 => self.first_page.clone(),
            1 => self.second_page.clone(),
            _ => vec![],
        })
    }
    fn get_latest_grant_events(&self, _: CommunityId, _: usize) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_latest_gift_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

#[test]
fn get_latest_paginates_through_effects_spanning_two_full_pages() {
    // First page is exactly EFFECTS_PAGE_SIZE (loop must continue); second page has
    // one more effect. Final version must be PAGE_SIZE+1, not PAGE_SIZE — killing
    // the `< → <=` mutation which would stop after the first page.
    let community = Community::new();
    let id = community.id;
    let event_log = TwoPageEffectsEventLog::new(id);
    let expected_version = SequenceId::new(EFFECTS_PAGE_SIZE as u64 + 1);
    let store = CommunityStore::new(GetOkPutOkRepo { community }, event_log);
    let result = store.get_latest(id).unwrap().unwrap();
    assert_eq!(result.version, expected_version);
}

// --- mock event log that returns exactly EFFECTS_PAGE_SIZE effects on the first
//     call, then errors on the second call ---

struct FirstPageThenErrorEventLog {
    effects: Vec<Effect>,
    call_count: Cell<usize>,
}

impl FirstPageThenErrorEventLog {
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
            call_count: Cell::new(0),
        }
    }
}

impl EventLogProvider for FirstPageThenErrorEventLog {
    type Error = Error;
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        let n = self.call_count.get();
        self.call_count.set(n + 1);
        if n == 0 {
            Ok(self.effects.clone())
        } else {
            Err(err())
        }
    }
    fn get_latest_grant_events(&self, _: CommunityId, _: usize) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_latest_gift_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

#[test]
fn get_latest_paginates_through_all_effects() {
    // Covers the loop-continues branch: the first page is exactly EFFECTS_PAGE_SIZE,
    // so the loop runs a second time before breaking on the empty second page.
    let community = Community::new();
    let id = community.id;
    let event_log = MultiPageEffectsEventLog::new(id);
    let expected_version = SequenceId::new(EFFECTS_PAGE_SIZE as u64);
    let store = CommunityStore::new(GetOkPutOkRepo { community }, event_log);
    let result = store.get_latest(id).unwrap().unwrap();
    assert_eq!(result.version, expected_version);
}

#[test]
fn get_latest_error_on_second_batch_includes_batch_number_in_message() {
    let community = Community::new();
    let id = community.id;
    let event_log = FirstPageThenErrorEventLog::new(id);
    let store = CommunityStore::new(GetOkPutOkRepo { community }, event_log);
    let err = store.get_latest(id).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Storage layer error: failed to retrieve effects for community at batch number 1"
    );
}
