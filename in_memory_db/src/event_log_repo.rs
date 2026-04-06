use std::{
    collections::HashMap,
    io,
    sync::{
        atomic::{AtomicU64, Ordering},
        RwLock,
    },
};

use fruit_domain::{
    community::CommunityId,
    error::Error,
    event_log::{
        Effect, Event, EventPayload, HasSequenceId as _, Record, SequenceId, StateMutation,
    },
    event_log_repo::{EventLogPersistor, EventLogProvider, EventLogRepo},
    id::IntegerIdentifier,
};

/// In-memory implementation of [`EventLogRepo`].
///
/// Events and effects share a single auto-incrementing sequence, giving a total
/// ordering across all entries. Both collections are protected by separate
/// [`RwLock`]s to allow concurrent reads.
pub struct InMemoryEventLogRepo {
    sequence: AtomicU64,
    events: RwLock<HashMap<SequenceId, Event>>,
    effects_by_event: RwLock<HashMap<SequenceId, Effect>>,
}

impl InMemoryEventLogRepo {
    /// Creates a new empty `InMemoryEventLogRepo`.
    pub fn new() -> Self {
        Self {
            sequence: AtomicU64::new(0),
            events: RwLock::new(HashMap::new()),
            effects_by_event: RwLock::new(HashMap::new()),
        }
    }

    fn next_id(&self) -> SequenceId {
        SequenceId::from_u64(self.sequence.fetch_add(1, Ordering::SeqCst) + 1)
    }
}

impl Default for InMemoryEventLogRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLogProvider for InMemoryEventLogRepo {
    fn get_record(&self, id: SequenceId) -> Result<Option<Record>, Error> {
        if let Some(event) = self.events.read()?.get(&id).cloned() {
            return Ok(Some(event.into()));
        }
        let effect = self
            .effects_by_event
            .read()?
            .values()
            .find(|e| e.id == id)
            .cloned();
        Ok(effect.map(Into::into))
    }

    fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Error> {
        Ok(self.effects_by_event.read()?.get(&event_id).cloned())
    }

    fn get_effects_after(
        &self,
        community_id: CommunityId,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Error> {
        let mut effects: Vec<Effect> = self
            .effects_by_event
            .read()?
            .values()
            .filter(|e| e.community_id == community_id && e.id > after)
            .cloned()
            .collect();
        effects.sort_by_key(|e| e.id);
        Ok(effects)
    }

    fn get_latest_records(
        &self,
        community_id: CommunityId,
        n: usize,
    ) -> Result<Vec<Record>, Error> {
        let mut records: Vec<Record> = Vec::new();
        self.events
            .read()?
            .values()
            .filter(|e| e.community_id == community_id)
            .for_each({
                |e| {
                    records.push(e.clone().into());
                }
            });

        self.effects_by_event
            .read()?
            .values()
            .filter(|e| e.community_id == community_id)
            .for_each(|e| {
                records.push(e.clone().into());
            });

        records.sort_by_key(|e| std::cmp::Reverse(e.sequence_id()));
        records.truncate(n);
        Ok(records)
    }
}

impl EventLogPersistor for InMemoryEventLogRepo {
    fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Error> {
        let id = self.next_id();
        let event = Event {
            id,
            community_id,
            payload,
        };
        let mut events = self.events.write()?;
        if events.contains_key(&id) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("sequence ID {id} already written"),
            )
            .into());
        }
        events.insert(id, event.clone());
        Ok(event)
    }

    fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Error> {
        let id = self.next_id();
        let effect = Effect {
            id,
            event_id,
            community_id,
            mutations,
        };
        let mut effects = self.effects_by_event.write()?;
        if effects.contains_key(&event_id) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("effect for event ID {event_id} already written"),
            )
            .into());
        }
        effects.insert(event_id, effect.clone());
        Ok(effect)
    }
}

impl EventLogRepo for InMemoryEventLogRepo {}

impl EventLogProvider for &InMemoryEventLogRepo {
    fn get_record(&self, id: SequenceId) -> Result<Option<Record>, Error> {
        (*self).get_record(id)
    }

    fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Error> {
        (*self).get_effect_for_event(event_id)
    }

    fn get_effects_after(
        &self,
        community_id: CommunityId,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Error> {
        (*self).get_effects_after(community_id, after)
    }

    fn get_latest_records(
        &self,
        community_id: CommunityId,
        n: usize,
    ) -> Result<Vec<Record>, Error> {
        (*self).get_latest_records(community_id, n)
    }
}

impl EventLogPersistor for &InMemoryEventLogRepo {
    fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Error> {
        (*self).append_event(community_id, payload)
    }

    fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Error> {
        (*self).append_effect(event_id, community_id, mutations)
    }
}

impl EventLogRepo for &InMemoryEventLogRepo {}

#[cfg(test)]
#[path = "event_log_repo_tests.rs"]
mod tests;
