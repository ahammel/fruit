use std::{
    collections::HashMap,
    io,
    sync::{
        atomic::{AtomicU64, Ordering},
        RwLock,
    },
};

use gib_fruit_domain::{
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
        if let Some(event) = self.events.read()?.get(&id).copied() {
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
                    records.push((*e).into());
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
        events.insert(id, event);
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
mod tests {
    use super::*;
    use gib_fruit_domain::{
        community::HasCommunityId, fruit::STRAWBERRY, id::UuidIdentifier, member::MemberId,
    };

    fn log() -> InMemoryEventLogRepo {
        InMemoryEventLogRepo::new()
    }

    fn community_id() -> CommunityId {
        CommunityId::new()
    }

    // --- helpers ---

    fn poisoned_event_map() -> RwLock<HashMap<SequenceId, Event>> {
        use std::sync::Arc;
        let lock = Arc::new(RwLock::new(HashMap::new()));
        let l = Arc::clone(&lock);
        std::thread::spawn(move || {
            let _guard = l.write().unwrap();
            panic!("intentional poison");
        })
        .join()
        .ok();
        Arc::try_unwrap(lock).unwrap()
    }

    fn poisoned_effect_map() -> RwLock<HashMap<SequenceId, Effect>> {
        use std::sync::Arc;
        let lock = Arc::new(RwLock::new(HashMap::new()));
        let l = Arc::clone(&lock);
        std::thread::spawn(move || {
            let _guard = l.write().unwrap();
            panic!("intentional poison");
        })
        .join()
        .ok();
        Arc::try_unwrap(lock).unwrap()
    }

    // --- default ---

    #[test]
    fn default_produces_empty_log() {
        let log = InMemoryEventLogRepo::default();
        assert!(log.get_record(SequenceId::from_u64(1)).unwrap().is_none());
        assert!(log
            .get_effect_for_event(SequenceId::from_u64(1))
            .unwrap()
            .is_none());
    }

    // --- poisoned lock error paths ---

    #[test]
    fn get_record_returns_err_when_event_lock_is_poisoned() {
        let log = InMemoryEventLogRepo {
            sequence: AtomicU64::new(0),
            events: poisoned_event_map(),
            effects_by_event: RwLock::new(HashMap::new()),
        };
        assert!(log.get_record(SequenceId::from_u64(1)).is_err());
    }

    #[test]
    fn get_record_returns_err_when_effect_lock_is_poisoned() {
        let log = InMemoryEventLogRepo {
            sequence: AtomicU64::new(0),
            events: RwLock::new(HashMap::new()),
            effects_by_event: poisoned_effect_map(),
        };
        assert!(log.get_record(SequenceId::from_u64(1)).is_err());
    }

    #[test]
    fn append_event_returns_err_when_lock_is_poisoned() {
        let log = InMemoryEventLogRepo {
            sequence: AtomicU64::new(0),
            events: poisoned_event_map(),
            effects_by_event: RwLock::new(HashMap::new()),
        };
        assert!(log
            .append_event(community_id(), EventPayload::Grant { count: 1 })
            .is_err());
    }

    #[test]
    fn get_effect_for_event_returns_err_when_lock_is_poisoned() {
        let log = InMemoryEventLogRepo {
            sequence: AtomicU64::new(0),
            events: RwLock::new(HashMap::new()),
            effects_by_event: poisoned_effect_map(),
        };
        assert!(log.get_effect_for_event(SequenceId::from_u64(1)).is_err());
    }

    #[test]
    fn append_effect_returns_err_when_lock_is_poisoned() {
        let log = InMemoryEventLogRepo {
            sequence: AtomicU64::new(0),
            events: RwLock::new(HashMap::new()),
            effects_by_event: poisoned_effect_map(),
        };
        assert!(log
            .append_effect(SequenceId::from_u64(1), community_id(), vec![])
            .is_err());
    }

    #[test]
    fn get_effects_after_excludes_effect_at_exact_boundary() {
        // Tests the `>` vs `>=` boundary: an effect whose id == `after` must NOT be returned.
        let log = log();
        let cid = community_id();
        let e1 = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        let eff1 = log.append_effect(e1.id, cid, vec![]).unwrap();
        let e2 = log
            .append_event(cid, EventPayload::Grant { count: 2 })
            .unwrap();
        let eff2 = log.append_effect(e2.id, cid, vec![]).unwrap();
        assert_eq!(log.get_effects_after(cid, eff1.id).unwrap(), vec![eff2]);
    }

    #[test]
    fn get_effects_after_filters_by_community_id() {
        // Tests `&&` vs `||`: effects from a different community must not appear.
        let log = log();
        let cid1 = community_id();
        let cid2 = community_id();
        let e1 = log
            .append_event(cid1, EventPayload::Grant { count: 1 })
            .unwrap();
        let eff1 = log.append_effect(e1.id, cid1, vec![]).unwrap();
        let e2 = log
            .append_event(cid2, EventPayload::Grant { count: 1 })
            .unwrap();
        log.append_effect(e2.id, cid2, vec![]).unwrap();
        assert_eq!(
            log.get_effects_after(cid1, SequenceId::zero()).unwrap(),
            vec![eff1]
        );
    }

    #[test]
    fn get_effects_after_sorts_multiple_effects_ascending() {
        let log = log();
        let cid = community_id();
        let e1 = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        let e2 = log
            .append_event(cid, EventPayload::Grant { count: 2 })
            .unwrap();
        let eff1 = log.append_effect(e1.id, cid, vec![]).unwrap();
        let eff2 = log.append_effect(e2.id, cid, vec![]).unwrap();
        let effects = log.get_effects_after(cid, SequenceId::zero()).unwrap();
        assert_eq!(effects, vec![eff1, eff2]);
    }

    #[test]
    fn get_effects_after_returns_err_when_lock_is_poisoned() {
        let log = InMemoryEventLogRepo {
            sequence: AtomicU64::new(0),
            events: RwLock::new(HashMap::new()),
            effects_by_event: poisoned_effect_map(),
        };
        assert!(log
            .get_effects_after(community_id(), SequenceId::zero())
            .is_err());
    }

    #[test]
    fn get_latest_events_returns_err_when_events_lock_is_poisoned() {
        let log = InMemoryEventLogRepo {
            sequence: AtomicU64::new(0),
            events: poisoned_event_map(),
            effects_by_event: RwLock::new(HashMap::new()),
        };
        assert!(log.get_latest_records(community_id(), 5).is_err());
    }

    #[test]
    fn get_latest_events_returns_err_when_effects_lock_is_poisoned() {
        let log = InMemoryEventLogRepo {
            sequence: AtomicU64::new(0),
            events: RwLock::new(HashMap::new()),
            effects_by_event: poisoned_effect_map(),
        };
        assert!(log.get_latest_records(community_id(), 5).is_err());
    }

    // --- record round-trips ---

    #[test]
    fn get_record_returns_event_after_append() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 3 })
            .unwrap();
        assert_eq!(
            log.get_record(event.id).unwrap(),
            Some(
                Event {
                    id: event.id,
                    community_id: cid,
                    payload: EventPayload::Grant { count: 3 },
                }
                .into()
            )
        );
    }

    #[test]
    fn get_record_returns_effect_after_append() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        let effect = log.append_effect(event.id, cid, vec![]).unwrap();
        assert_eq!(
            log.get_record(effect.id).unwrap(),
            Some(
                Effect {
                    id: effect.id,
                    event_id: event.id,
                    community_id: cid,
                    mutations: vec![],
                }
                .into()
            )
        );
    }

    #[test]
    fn get_record_returns_none_for_unknown_id() {
        assert!(log()
            .get_record(SequenceId::from_u64(99))
            .unwrap()
            .is_none());
    }

    // --- effect round-trips ---

    #[test]
    fn append_effect_and_get_effect_round_trip() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        let mutations = vec![StateMutation::AddFruitToMember {
            member_id: MemberId::new(),
            fruit: STRAWBERRY,
        }];
        let effect = log.append_effect(event.id, cid, mutations.clone()).unwrap();
        assert_eq!(
            log.get_effect_for_event(event.id).unwrap(),
            Some(Effect {
                id: effect.id,
                event_id: event.id,
                community_id: cid,
                mutations,
            })
        );
    }

    #[test]
    fn get_effect_for_event_returns_none_for_unprocessed_event() {
        assert!(log()
            .get_effect_for_event(SequenceId::from_u64(1))
            .unwrap()
            .is_none());
    }

    // --- duplicate write rejection ---

    #[test]
    fn append_event_fails_on_duplicate_sequence_id() {
        let log = log();
        let cid = community_id();
        let dummy = Event {
            id: SequenceId::from_u64(1),
            community_id: cid,
            payload: EventPayload::Grant { count: 0 },
        };
        log.events
            .write()
            .unwrap()
            .insert(SequenceId::from_u64(1), dummy);
        assert!(log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .is_err());
    }

    #[test]
    fn append_effect_fails_on_duplicate_event_id() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        log.append_effect(event.id, cid, vec![]).unwrap();
        assert!(log.append_effect(event.id, cid, vec![]).is_err());
    }

    // --- shared sequence ---

    #[test]
    fn events_and_effects_share_sequence() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        let effect = log.append_effect(event.id, cid, vec![]).unwrap();
        assert_eq!(event.id, SequenceId::from_u64(1));
        assert_eq!(effect.id, SequenceId::from_u64(2));
    }

    // --- get_latest_events ---

    #[test]
    fn get_latest_events_returns_n_most_recent_descending() {
        let log = log();
        let cid = community_id();
        for i in 1..=5 {
            log.append_event(cid, EventPayload::Grant { count: i })
                .unwrap();
        }
        let events = log.get_latest_records(cid, 3).unwrap();
        assert_eq!(events.len(), 3);
        assert!(events
            .windows(2)
            .all(|w| w[0].sequence_id() > w[1].sequence_id()));
    }

    #[test]
    fn get_latest_events_returns_fewer_when_not_enough() {
        let log = log();
        let cid = community_id();
        log.append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        assert_eq!(log.get_latest_records(cid, 10).unwrap().len(), 1);
    }

    #[test]
    fn get_latest_events_filters_by_community() {
        let log = log();
        let cid1 = community_id();
        let cid2 = community_id();
        log.append_event(cid1, EventPayload::Grant { count: 1 })
            .unwrap();
        log.append_event(cid2, EventPayload::Grant { count: 1 })
            .unwrap();
        let events = log.get_latest_records(cid1, 10).unwrap();
        assert!(events.iter().all(|e| e.community_id() == cid1));
    }

    #[test]
    fn get_latest_events_returns_empty_when_none_exist() {
        assert!(log()
            .get_latest_records(community_id(), 5)
            .unwrap()
            .is_empty());
    }

    // --- &InMemoryEventLogRepo delegation ---

    #[test]
    fn ref_delegates_get_record() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        assert_eq!(log.get_record(event.id).unwrap(), Some(event.into()));
    }

    #[test]
    fn ref_delegates_get_effect_for_event() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        let effect = log.append_effect(event.id, cid, vec![]).unwrap();
        assert_eq!(log.get_effect_for_event(event.id).unwrap(), Some(effect));
    }

    #[test]
    fn ref_delegates_get_effects_after() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        let effect = log.append_effect(event.id, cid, vec![]).unwrap();
        assert_eq!(
            log.get_effects_after(cid, SequenceId::zero()).unwrap(),
            vec![effect]
        );
    }

    #[test]
    fn ref_delegates_get_latest_events() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        assert_eq!(log.get_latest_records(cid, 5).unwrap(), vec![event.into()]);
    }

    #[test]
    fn ref_delegates_append_event() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        assert_eq!(log.get_record(event.id).unwrap(), Some(event.into()));
    }

    #[test]
    fn ref_delegates_append_effect() {
        let log = log();
        let cid = community_id();
        let event = log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        let effect = log.append_effect(event.id, cid, vec![]).unwrap();
        assert_eq!(log.get_effect_for_event(event.id).unwrap(), Some(effect));
    }
}
