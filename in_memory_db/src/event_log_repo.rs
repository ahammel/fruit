use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        RwLock,
    },
};

use async_trait::async_trait;
use exn::Exn;
use fruit_domain::{
    community::CommunityId,
    event_log::{
        Effect, Event, EventPayload, HasSequenceId as _, Record, SequenceId, StateMutation,
    },
    event_log_repo::{EventLogPersistor, EventLogProvider, EventLogRepo},
};

use crate::error::{AlreadyExists, Error, Lock, LockPoisoned};

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
        SequenceId::new(self.sequence.fetch_add(1, Ordering::SeqCst) + 1)
    }
}

impl Default for InMemoryEventLogRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventLogProvider for InMemoryEventLogRepo {
    type Error = Error;

    async fn get_record(
        &self,
        community_id: CommunityId,
        id: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        let event = match self
            .events
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EventLogRead))?
            .get(&id)
            .filter(|e| e.community_id == community_id)
            .cloned()
        {
            Some(e) => e,
            None => return Ok(None),
        };
        let effect = self
            .effects
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EffectLogRead))?
            .get(&id)
            .cloned();
        Ok(Some(Record { event, effect }))
    }

    async fn get_effect_for_event(
        &self,
        community_id: CommunityId,
        event_id: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(self
            .effects
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EffectLogRead))?
            .get(&event_id)
            .filter(|e| e.community_id == community_id)
            .cloned())
    }

    async fn get_effects_after(
        &self,
        community_id: CommunityId,
        limit: usize,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        let mut effects: Vec<Effect> = self
            .effects
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EffectLogRead))?
            .values()
            .filter(|e| e.community_id == community_id && e.id > after)
            .cloned()
            .collect();
        effects.sort_by_key(|e| e.id);
        effects.truncate(limit);
        Ok(effects)
    }

    async fn get_records_before(
        &self,
        community_id: CommunityId,
        limit: usize,
        before: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        let effects = self
            .effects
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EffectLogRead))?;
        let mut records: Vec<Record> = self
            .events
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EventLogRead))?
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

    async fn get_latest_grant_events(
        &self,
        community_id: CommunityId,
        limit: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        let mut events: Vec<Event> = self
            .events
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EventLogRead))?
            .values()
            .filter(|e| e.community_id == community_id)
            .filter(|e| {
                matches!(
                    e.payload,
                    fruit_domain::event_log::EventPayload::Grant { .. }
                )
            })
            .cloned()
            .collect();
        events.sort_by_key(|e| std::cmp::Reverse(e.id));
        events.truncate(limit);
        Ok(events)
    }

    async fn get_latest_gift_records(
        &self,
        community_id: CommunityId,
        limit: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        let effects = self
            .effects
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EffectLogRead))?;
        let mut records: Vec<Record> = self
            .events
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EventLogRead))?
            .values()
            .filter(|e| e.community_id == community_id)
            .filter(|e| {
                matches!(
                    e.payload,
                    fruit_domain::event_log::EventPayload::Gift { .. }
                )
            })
            .map(|e| Record {
                effect: effects.get(&e.id).cloned(),
                event: e.clone(),
            })
            .collect();
        records.sort_by_key(|r| std::cmp::Reverse(r.sequence_id()));
        records.truncate(limit);
        Ok(records)
    }

    async fn get_records_between(
        &self,
        community_id: CommunityId,
        after: SequenceId,
        before: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        let effects = self
            .effects
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EffectLogRead))?;
        let mut records: Vec<Record> = self
            .events
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::EventLogRead))?
            .values()
            .filter(|e| e.community_id == community_id && e.id > after && e.id < before)
            .map(|e| Record {
                effect: effects.get(&e.id).cloned(),
                event: e.clone(),
            })
            .collect();
        records.sort_by_key(|r| r.sequence_id());
        Ok(records)
    }
}

#[async_trait]
impl EventLogPersistor for InMemoryEventLogRepo {
    type Error = Error;

    async fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        let id = self.next_id();
        let event = Event {
            id,
            community_id,
            payload,
        };
        let mut events = self
            .events
            .write()
            .map_err(|e| LockPoisoned::build(&e, Lock::EventLogWrite))?;
        if events.contains_key(&id) {
            return Err(AlreadyExists::event(community_id, id).into());
        }
        events.insert(id, event.clone());
        Ok(event)
    }

    async fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        let effect = Effect {
            id: event_id,
            community_id,
            mutations,
        };
        let mut effects = self
            .effects
            .write()
            .map_err(|e| LockPoisoned::build(&e, Lock::EffectLogWrite))?;
        if effects.contains_key(&event_id) {
            return Err(AlreadyExists::effect(community_id, event_id).into());
        }
        effects.insert(event_id, effect.clone());
        Ok(effect)
    }
}

impl EventLogRepo for InMemoryEventLogRepo {}

#[cfg(test)]
#[path = "event_log_repo_tests.rs"]
mod tests;
