use std::io;

use super::*;
use crate::{
    event_log::Record,
    event_log_repo::{EventLogPersistor, EventLogProvider},
    id::{IntegerIdentifier, UuidIdentifier},
};

fn err() -> Error {
    io::Error::new(io::ErrorKind::Other, "test error").into()
}

struct ErrorRepo;

impl EventLogProvider for ErrorRepo {
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

impl EventLogPersistor for ErrorRepo {
    fn append_event(&self, _: CommunityId, _: EventPayload) -> Result<Event, Error> {
        Err(err())
    }
    fn append_effect(
        &self,
        _: SequenceId,
        _: CommunityId,
        _: Vec<StateMutation>,
    ) -> Result<Effect, Error> {
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
        .get_effects_after(CommunityId::new(), SequenceId::zero())
        .is_err());
}

#[test]
fn get_latest_records_propagates_error() {
    assert!(store().get_latest_records(CommunityId::new(), 5).is_err());
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
