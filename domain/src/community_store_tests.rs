use std::io;

use super::*;
use crate::{
    community_repo::{CommunityPersistor, CommunityProvider},
    event_log::{Effect, Record},
    id::{IntegerIdentifier, UuidIdentifier},
};

fn err() -> Error {
    io::Error::new(io::ErrorKind::Other, "test error").into()
}

// --- mock repo that always errors ---

struct ErrorRepo;

impl CommunityProvider for ErrorRepo {
    fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Error> {
        Err(err())
    }
    fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Error> {
        Err(err())
    }
}

impl CommunityPersistor for ErrorRepo {
    fn put(&self, _: Community) -> Result<Community, Error> {
        Err(err())
    }
}

impl CommunityRepo for ErrorRepo {}

// --- mock repo that returns one community then fails puts ---

struct GetOkPutErrorRepo {
    community: Community,
}

impl CommunityProvider for GetOkPutErrorRepo {
    fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Error> {
        Ok(None)
    }
    fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Error> {
        Ok(Some(self.community.clone()))
    }
}

impl CommunityPersistor for GetOkPutErrorRepo {
    fn put(&self, _: Community) -> Result<Community, Error> {
        Err(err())
    }
}

impl CommunityRepo for GetOkPutErrorRepo {}

// --- mock event log that always errors ---

struct ErrorEventLog;

impl EventLogProvider for ErrorEventLog {
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Error> {
        Err(err())
    }
    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Error> {
        Err(err())
    }
    fn get_effects_after(&self, _: CommunityId, _: SequenceId) -> Result<Vec<Effect>, Error> {
        Err(err())
    }
    fn get_latest_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Error> {
        Err(err())
    }
}

// --- mock repo that returns None for get_latest ---

struct NoneLatestRepo;

impl CommunityProvider for NoneLatestRepo {
    fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Error> {
        Ok(None)
    }
    fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Error> {
        Ok(None)
    }
}

impl CommunityPersistor for NoneLatestRepo {
    fn put(&self, c: Community) -> Result<Community, Error> {
        Ok(c)
    }
}

impl CommunityRepo for NoneLatestRepo {}

// --- mock event log that returns empty effects ---

struct EmptyEffectsEventLog;

impl EventLogProvider for EmptyEffectsEventLog {
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Error> {
        Ok(None)
    }
    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Error> {
        Ok(None)
    }
    fn get_effects_after(&self, _: CommunityId, _: SequenceId) -> Result<Vec<Effect>, Error> {
        Ok(vec![])
    }
    fn get_latest_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Error> {
        Ok(vec![])
    }
}

// --- mock event log that returns one effect ---

struct OneEffectEventLog {
    effect: Effect,
}

impl EventLogProvider for OneEffectEventLog {
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Error> {
        Ok(None)
    }
    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Error> {
        Ok(None)
    }
    fn get_effects_after(&self, _: CommunityId, _: SequenceId) -> Result<Vec<Effect>, Error> {
        Ok(vec![self.effect.clone()])
    }
    fn get_latest_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Error> {
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
    use crate::id::IntegerIdentifier;
    let community = Community::new();
    let id = community.id;
    let effect = Effect {
        id: SequenceId::from_u64(1),
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
    // Covers the `if unapplied.is_empty() { return Ok(Some(community)) }` branch
    // for the GetOkPutErrorRepo + EmptyEffectsEventLog monomorphization.
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
