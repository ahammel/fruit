use std::io;

use exn::Exn;
use newtype_ids_uuid::UuidIdentifier;

use super::*;
use crate::{
    error::Error,
    event_log::Record,
    event_log_repo::{EventLogPersistor, EventLogProvider},
};

fn err() -> Exn<Error> {
    Exn::new(io::Error::other("test error").into())
}

struct ErrorRepo;

impl EventLogProvider for ErrorRepo {
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

impl EventLogPersistor for ErrorRepo {
    type Error = Error;
    fn append_event(&self, _: CommunityId, _: EventPayload) -> Result<Event, Exn<Error>> {
        Err(err())
    }
    fn append_effect(
        &self,
        _: SequenceId,
        _: CommunityId,
        _: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        Err(err())
    }
}

impl EventLogRepo for ErrorRepo {}

fn store() -> EventLogStore<ErrorRepo> {
    EventLogStore::new(ErrorRepo)
}

#[test]
fn get_record_propagates_error() {
    assert!(store().get_record(SequenceId::zero()).is_err());
}

#[test]
fn get_effect_for_event_propagates_error() {
    assert!(store().get_effect_for_event(SequenceId::zero()).is_err());
}

#[test]
fn get_effects_after_propagates_error() {
    assert!(store()
        .get_effects_after()
        .community_id(CommunityId::new())
        .limit(10)
        .call()
        .is_err());
}

#[test]
fn get_records_before_propagates_error() {
    assert!(store()
        .get_records_before()
        .community_id(CommunityId::new())
        .limit(5)
        .call()
        .is_err());
}

#[test]
fn append_event_propagates_error() {
    assert!(store()
        .append_event(CommunityId::new(), EventPayload::Grant { count: 1 })
        .is_err());
}

#[test]
fn append_effect_propagates_error() {
    assert!(store()
        .append_effect(SequenceId::zero(), CommunityId::new(), vec![])
        .is_err());
}
