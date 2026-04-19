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
/// Events and their corresponding Effects share the same [`SequenceId`]: the counter
/// advances only when an Event is appended. An Effect is stored under the same ID as its
/// originating Event. Both collections are protected by separate [`RwLock`]s to allow
/// concurrent reads.
#[derive(Debug)]
pub struct InMemoryEventLogRepo {
    sequence: AtomicU64,
    events: RwLock<HashMap<SequenceId, Event>>,
    effects: RwLock<HashMap<SequenceId, Effect>>,
}

impl InMemoryEventLogRepo {
    /// Creates a new empty `InMemoryEventLogRepo`.
    pub fn new() -> Self {
        Self {
            sequence: AtomicU64::new(0),
            events: RwLock::new(HashMap::new()),
            effects: RwLock::new(HashMap::new()),
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
        let event = match self.events.read()?.get(&id).cloned() {
            Some(e) => e,
            None => return Ok(None),
        };
        let effect = self.effects.read()?.get(&id).cloned();
        Ok(Some(Record { event, effect }))
    }

    fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Error> {
        Ok(self.effects.read()?.get(&event_id).cloned())
    }

    fn get_effects_after(
        &self,
        community_id: CommunityId,
        limit: usize,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Error> {
        let mut effects: Vec<Effect> = self
            .effects
            .read()?
            .values()
            .filter(|e| e.community_id == community_id && e.id > after)
            .cloned()
            .collect();
        effects.sort_by_key(|e| e.id);
        effects.truncate(limit);
        Ok(effects)
    }

    fn get_records_before(
        &self,
        community_id: CommunityId,
        limit: usize,
        before: Option<SequenceId>,
    ) -> Result<Vec<Record>, Error> {
        let effects = self.effects.read()?;
        let mut records: Vec<Record> = self
            .events
            .read()?
            .values()
            .filter(|e| e.community_id == community_id)
            .filter(|e| before.is_none_or(|cursor| e.id < cursor))
            .map(|e| Record {
                effect: effects.get(&e.id).cloned(),
                event: e.clone(),
            })
            .collect();
        records.sort_by_key(|e| std::cmp::Reverse(e.sequence_id()));
        records.truncate(limit);
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
        let effect = Effect {
            id: event_id,
            community_id,
            mutations,
        };
        let mut effects = self.effects.write()?;
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
        limit: usize,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Error> {
        (*self).get_effects_after(community_id, limit, after)
    }

    fn get_records_before(
        &self,
        community_id: CommunityId,
        limit: usize,
        before: Option<SequenceId>,
    ) -> Result<Vec<Record>, Error> {
        (*self).get_records_before(community_id, limit, before)
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
