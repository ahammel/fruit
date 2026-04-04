use crate::{
    community::CommunityId,
    error::Error,
    event_log::{Effect, Event, EventPayload, Record, SequenceId, StateMutation},
    event_log_repo::EventLogRepo,
};

/// Reads and writes the event and effect log via an [`EventLogRepo`].
pub struct EventLogStore<ELR: EventLogRepo> {
    repo: ELR,
}

impl<ELR: EventLogRepo> EventLogStore<ELR> {
    /// Creates a new `EventLogStore` backed by `repo`.
    pub fn new(repo: ELR) -> Self {
        Self { repo }
    }

    /// Returns the log entry with the given sequence ID, or `None` if not found.
    pub fn get_record(&self, id: SequenceId) -> Result<Option<Record>, Error> {
        self.repo.get_record(id)
    }

    /// Returns the effect whose `event_id` matches the given ID, or `None` if not yet processed.
    pub fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Error> {
        self.repo.get_effect_for_event(event_id)
    }

    /// Returns all effects for `community_id` after `after`, sorted by sequence ID ascending.
    pub fn get_effects_after(
        &self,
        community_id: CommunityId,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Error> {
        self.repo.get_effects_after(community_id, after)
    }

    /// Returns the `n` most recent records for `community_id`, sorted by sequence ID descending.
    pub fn get_latest_records(
        &self,
        community_id: CommunityId,
        n: usize,
    ) -> Result<Vec<Record>, Error> {
        self.repo.get_latest_records(community_id, n)
    }

    /// Assigns the next sequence ID to a new event and stores it.
    pub fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Error> {
        self.repo.append_event(community_id, payload)
    }

    /// Assigns the next sequence ID to a new effect and stores it.
    pub fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Error> {
        self.repo.append_effect(event_id, community_id, mutations)
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;
    use crate::{
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
}
