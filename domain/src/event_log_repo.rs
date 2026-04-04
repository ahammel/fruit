use crate::{
    community::CommunityId,
    error::Error,
    event_log::Record,
    event_log::{Effect, StateMutation},
    event_log::{Event, EventPayload, SequenceId},
};

/// Read port for the event and effect log.
pub trait EventLogProvider {
    /// Returns the log entry with the given sequence ID, or `None` if not found.
    fn get_record(&self, id: SequenceId) -> Result<Option<Record>, Error>;

    /// Returns the effect whose `event_id` matches the given ID, or `None` if the
    /// event has not yet been processed.
    fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Error>;

    /// Returns all effects for `community_id` whose sequence ID is strictly greater
    /// than `after`, sorted by sequence ID ascending.
    fn get_effects_after(
        &self,
        community_id: CommunityId,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Error>;

    /// Returns the `n` most recent events for `community_id`, sorted by sequence ID
    /// descending.
    fn get_latest_records(&self, community_id: CommunityId, n: usize)
        -> Result<Vec<Record>, Error>;
}

/// Write port for the event and effect log.
pub trait EventLogPersistor {
    /// Assign the next sequence ID to a new event and store it.
    fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Error>;

    /// Assign the next sequence ID to a new effect and store it.
    fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Error>;
}

/// Combined read/write port for the event and effect log.
pub trait EventLogRepo: EventLogProvider + EventLogPersistor {}
