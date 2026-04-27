use bon::bon;

use newtype_ids::IntegerIdentifier;

use crate::{
    community::CommunityId,
    error::{DbError, Error, StorageLayerError},
    event_log::{Effect, Event, EventPayload, Record, SequenceId, StateMutation},
    event_log_repo::{EventLogProvider, EventLogRepo},
};

use exn::Exn;

/// Reads and writes the event and effect log via an [`EventLogRepo`].
pub struct EventLogStore<ELR: EventLogRepo> {
    repo: ELR,
}

#[bon]
impl<E: DbError, ELR: EventLogRepo + EventLogProvider<Error = E>> EventLogStore<ELR> {
    /// Creates a new `EventLogStore` backed by `repo`.
    pub fn new(repo: ELR) -> Self {
        Self { repo }
    }

    /// Returns the log entry at `id`, or `None` if not found.
    pub fn get_record(&self, id: SequenceId) -> Result<Option<Record>, Exn<Error>> {
        self.repo
            .get_record(id)
            .map_err(|e| StorageLayerError::raise("failed to read event log record", e))
    }

    /// Returns the effect with the given ID (equal to its originating event's ID), or `None` if not yet processed.
    pub fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Exn<Error>> {
        self.repo
            .get_effect_for_event(event_id)
            .map_err(|e| StorageLayerError::raise("failed to read effect", e))
    }

    /// Returns up to `limit` effects for `community_id` after `after`, sorted by
    /// sequence ID ascending. `after` defaults to [`SequenceId::zero()`] (start from
    /// the beginning).
    #[builder]
    pub fn get_effects_after(
        &self,
        community_id: CommunityId,
        limit: usize,
        #[builder(default = SequenceId::zero())] after: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        self.repo
            .get_effects_after(community_id, limit, after)
            .map_err(|e| StorageLayerError::raise("failed to read effects", e))
    }

    /// Returns up to `limit` records for `community_id` before `before`, sorted by
    /// sequence ID descending. Each entry pairs the event with its computed effect, or
    /// `None` if not yet processed. `before` defaults to `None` (start from the most
    /// recent record).
    #[builder]
    pub fn get_records_before(
        &self,
        community_id: CommunityId,
        limit: usize,
        before: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        self.repo
            .get_records_before(community_id, limit, before)
            .map_err(|e| StorageLayerError::raise("failed to read records", e))
    }

    /// Assigns the next sequence ID to a new event and stores it.
    pub fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        self.repo
            .append_event(community_id, payload)
            .map_err(|e| StorageLayerError::raise("failed to create event", e))
    }

    /// Stores an effect with the same sequence ID as its originating event.
    pub fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        self.repo
            .append_effect(event_id, community_id, mutations)
            .map_err(|e| StorageLayerError::raise("failed to create effect", e))
    }
}

#[cfg(test)]
#[path = "event_log_store_tests.rs"]
mod tests;
