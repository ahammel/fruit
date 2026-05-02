use anomalies::{category::Unavailable, status::Status};
use async_trait::async_trait;
use exn::Exn;
use newtype_ids_uuid::UuidIdentifier;

use super::*;
use crate::{
    error::{Error, StorageLayerError},
    event_log::Record,
    event_log_repo::{EventLogPersistor, EventLogProvider},
};

fn err() -> Exn<Error> {
    Exn::new(StorageLayerError::build(
        "failed",
        Unavailable,
        Status::Temporary,
    ))
}

struct ErrorRepo;

#[async_trait]
impl EventLogProvider for ErrorRepo {
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

#[async_trait]
impl EventLogPersistor for ErrorRepo {
    type Error = Error;
    async fn append_event(&self, _: CommunityId, _: EventPayload) -> Result<Event, Exn<Error>> {
        Err(err())
    }
    async fn append_effect(
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

#[tokio::test]
async fn get_record_propagates_error() {
    assert_eq!(
        store()
            .get_record(CommunityId::new(), SequenceId::zero())
            .await
            .unwrap_err()
            .to_string(),
        "Storage layer error: failed to read event log record"
    );
}

#[tokio::test]
async fn get_effect_for_event_propagates_error() {
    assert_eq!(
        store()
            .get_effect_for_event(CommunityId::new(), SequenceId::zero())
            .await
            .unwrap_err()
            .to_string(),
        "Storage layer error: failed to read effect"
    );
}

#[tokio::test]
async fn get_effects_after_propagates_error() {
    assert_eq!(
        store()
            .get_effects_after()
            .community_id(CommunityId::new())
            .limit(10)
            .call()
            .await
            .unwrap_err()
            .to_string(),
        "Storage layer error: failed to read effects"
    );
}

#[tokio::test]
async fn get_records_before_propagates_error() {
    assert_eq!(
        store()
            .get_records_before()
            .community_id(CommunityId::new())
            .limit(5)
            .call()
            .await
            .unwrap_err()
            .to_string(),
        "Storage layer error: failed to read records"
    );
}

#[tokio::test]
async fn append_event_propagates_error() {
    assert_eq!(
        store()
            .append_event(CommunityId::new(), EventPayload::Grant { count: 1 })
            .await
            .unwrap_err()
            .to_string(),
        "Storage layer error: failed to create event"
    );
}

#[tokio::test]
async fn append_effect_propagates_error() {
    assert_eq!(
        store()
            .append_effect(SequenceId::zero(), CommunityId::new(), vec![])
            .await
            .unwrap_err()
            .to_string(),
        "Storage layer error: failed to create effect"
    );
}
