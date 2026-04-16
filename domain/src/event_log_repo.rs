use crate::{
    community::CommunityId,
    error::Error,
    event_log::Event,
    event_log::{Effect, EventPayload, Record, SequenceId, StateMutation},
};

/// Read port for the event and effect log.
pub trait EventLogProvider {
    /// Returns the log entry at `id`, or `None` if not found.
    fn get_record(&self, id: SequenceId) -> Result<Option<Record>, Error>;

    /// Returns the effect with the given ID (equal to its originating event's ID),
    /// or `None` if the event has not yet been processed.
    fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Error>;

    /// Returns up to `limit` effects for `community_id` whose sequence ID is strictly
    /// greater than `after`, sorted by sequence ID ascending.
    ///
    /// `after` acts as a keyset cursor: pass the sequence ID of the last effect you
    /// already have. To start from the beginning, pass [`SequenceId::zero()`].
    fn get_effects_after(
        &self,
        community_id: CommunityId,
        limit: usize,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Error>;

    /// Returns up to `limit` records for `community_id` whose sequence ID is strictly less
    /// than `before`, sorted by sequence ID descending. Each entry pairs the event with
    /// its computed effect, or `None` if the event has not yet been processed.
    ///
    /// `before` is a keyset cursor; pass `None` to start from the most recent record.
    fn get_records_before(
        &self,
        community_id: CommunityId,
        limit: usize,
        before: Option<SequenceId>,
    ) -> Result<Vec<Record>, Error>;
}

/// Write port for the event and effect log.
pub trait EventLogPersistor {
    /// Assign the next sequence ID to a new event and store it.
    fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Error>;

    /// Store an effect with the same sequence ID as its originating event.
    ///
    /// The effect's `id` will equal `event_id`. Returns an error if an effect for
    /// `event_id` has already been stored.
    fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Error>;
}

/// Combined read/write port for the event and effect log.
pub trait EventLogRepo: EventLogProvider + EventLogPersistor {}
