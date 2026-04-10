use super::*;
use fruit_domain::{
    community::HasCommunityId,
    event_log::{HasSequenceId, Record},
    fruit::STRAWBERRY,
    id::UuidIdentifier,
    member::MemberId,
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
        effects: RwLock::new(HashMap::new()),
    };
    assert!(log.get_record(SequenceId::from_u64(1)).is_err());
}

#[test]
fn get_record_returns_err_when_effect_lock_is_poisoned() {
    use fruit_domain::event_log::EventPayload;
    let id = SequenceId::from_u64(1);
    let cid = community_id();
    let mut events = HashMap::new();
    events.insert(
        id,
        Event {
            id,
            community_id: cid,
            payload: EventPayload::Grant { count: 1 },
        },
    );
    let log = InMemoryEventLogRepo {
        sequence: AtomicU64::new(1),
        events: RwLock::new(events),
        effects: poisoned_effect_map(),
    };
    assert!(log.get_record(id).is_err());
}

#[test]
fn append_event_returns_err_when_lock_is_poisoned() {
    let log = InMemoryEventLogRepo {
        sequence: AtomicU64::new(0),
        events: poisoned_event_map(),
        effects: RwLock::new(HashMap::new()),
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
        effects: poisoned_effect_map(),
    };
    assert!(log.get_effect_for_event(SequenceId::from_u64(1)).is_err());
}

#[test]
fn append_effect_returns_err_when_lock_is_poisoned() {
    let log = InMemoryEventLogRepo {
        sequence: AtomicU64::new(0),
        events: RwLock::new(HashMap::new()),
        effects: poisoned_effect_map(),
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
        effects: poisoned_effect_map(),
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
        effects: RwLock::new(HashMap::new()),
    };
    assert!(log.get_latest_records(community_id(), 5).is_err());
}

#[test]
fn get_latest_events_returns_err_when_effects_lock_is_poisoned() {
    let log = InMemoryEventLogRepo {
        sequence: AtomicU64::new(0),
        events: RwLock::new(HashMap::new()),
        effects: poisoned_effect_map(),
    };
    assert!(log.get_latest_records(community_id(), 5).is_err());
}

// --- record round-trips ---

#[test]
fn get_record_returns_pending_event_without_effect() {
    let log = log();
    let cid = community_id();
    let event = log
        .append_event(cid, EventPayload::Grant { count: 3 })
        .unwrap();
    assert_eq!(
        log.get_record(event.id).unwrap(),
        Some(Record {
            event,
            effect: None,
        })
    );
}

#[test]
fn get_record_returns_effect_once_processed() {
    let log = log();
    let cid = community_id();
    let event = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    let effect = log.append_effect(event.id, cid, vec![]).unwrap();
    assert_eq!(event.id, effect.id);
    assert_eq!(
        log.get_record(event.id).unwrap(),
        Some(Record {
            event,
            effect: Some(effect),
        })
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
    log.append_effect(event.id, cid, mutations.clone()).unwrap();
    assert_eq!(
        log.get_effect_for_event(event.id).unwrap(),
        Some(Effect {
            id: event.id,
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
fn events_and_effects_share_sequence_id() {
    let log = log();
    let cid = community_id();
    let event = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    let effect = log.append_effect(event.id, cid, vec![]).unwrap();
    assert_eq!(event.id, SequenceId::from_u64(1));
    assert_eq!(effect.id, SequenceId::from_u64(1));
    let event2 = log
        .append_event(cid, EventPayload::Grant { count: 2 })
        .unwrap();
    assert_eq!(event2.id, SequenceId::from_u64(2));
}

// --- get_latest_records includes effects ---

#[test]
fn get_latest_records_includes_effects() {
    let log = log();
    let cid = community_id();
    let event = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    let effect = log.append_effect(event.id, cid, vec![]).unwrap();
    assert_eq!(
        log.get_latest_records(cid, 10).unwrap(),
        vec![Record {
            event,
            effect: Some(effect),
        }]
    );
}

#[test]
fn get_latest_records_pending_event_has_none_effect() {
    let log = log();
    let cid = community_id();
    let event = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    assert_eq!(
        log.get_latest_records(cid, 10).unwrap(),
        vec![Record {
            event,
            effect: None,
        }]
    );
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
//
// These helpers force dispatch through the `impl EventLogProvider/Persistor for &InMemoryEventLogRepo`
// impls. When T is inferred as `&InMemoryEventLogRepo`, Rust monomorphizes through the reference impl
// rather than the owned one.

fn via_provider_get_record<T: EventLogProvider>(
    p: T,
    id: SequenceId,
) -> Result<Option<Record>, Error> {
    p.get_record(id)
}

fn via_provider_get_effect_for_event<T: EventLogProvider>(
    p: T,
    id: SequenceId,
) -> Result<Option<Effect>, Error> {
    p.get_effect_for_event(id)
}

fn via_provider_get_effects_after<T: EventLogProvider>(
    p: T,
    cid: CommunityId,
    after: SequenceId,
) -> Result<Vec<Effect>, Error> {
    p.get_effects_after(cid, after)
}

fn via_provider_get_latest_records<T: EventLogProvider>(
    p: T,
    cid: CommunityId,
    n: usize,
) -> Result<Vec<Record>, Error> {
    p.get_latest_records(cid, n)
}

fn via_persistor_append_event<T: EventLogPersistor>(
    p: T,
    cid: CommunityId,
    payload: EventPayload,
) -> Result<Event, Error> {
    p.append_event(cid, payload)
}

fn via_persistor_append_effect<T: EventLogPersistor>(
    p: T,
    event_id: SequenceId,
    cid: CommunityId,
    mutations: Vec<StateMutation>,
) -> Result<Effect, Error> {
    p.append_effect(event_id, cid, mutations)
}

#[test]
fn ref_delegates_get_record() {
    let log = log();
    let cid = community_id();
    let event = via_persistor_append_event(&log, cid, EventPayload::Grant { count: 1 }).unwrap();
    assert_eq!(
        via_provider_get_record(&log, event.id).unwrap(),
        Some(Record {
            event,
            effect: None,
        })
    );
}

#[test]
fn ref_delegates_get_effect_for_event() {
    let log = log();
    let cid = community_id();
    let event = via_persistor_append_event(&log, cid, EventPayload::Grant { count: 1 }).unwrap();
    let effect = via_persistor_append_effect(&log, event.id, cid, vec![]).unwrap();
    assert_eq!(
        via_provider_get_effect_for_event(&log, event.id).unwrap(),
        Some(effect)
    );
}

#[test]
fn ref_delegates_get_effects_after() {
    let log = log();
    let cid = community_id();
    let event = via_persistor_append_event(&log, cid, EventPayload::Grant { count: 1 }).unwrap();
    let effect = via_persistor_append_effect(&log, event.id, cid, vec![]).unwrap();
    assert_eq!(
        via_provider_get_effects_after(&log, cid, SequenceId::zero()).unwrap(),
        vec![effect]
    );
}

#[test]
fn ref_delegates_get_latest_events() {
    let log = log();
    let cid = community_id();
    let event = via_persistor_append_event(&log, cid, EventPayload::Grant { count: 1 }).unwrap();
    assert_eq!(
        via_provider_get_latest_records(&log, cid, 5).unwrap(),
        vec![Record {
            event,
            effect: None,
        }]
    );
}

#[test]
fn ref_delegates_append_event() {
    let log = log();
    let cid = community_id();
    let event = via_persistor_append_event(&log, cid, EventPayload::Grant { count: 1 }).unwrap();
    assert_eq!(
        via_provider_get_record(&log, event.id).unwrap(),
        Some(Record {
            event,
            effect: None,
        })
    );
}

#[test]
fn ref_delegates_append_effect() {
    let log = log();
    let cid = community_id();
    let event = via_persistor_append_event(&log, cid, EventPayload::Grant { count: 1 }).unwrap();
    let effect = via_persistor_append_effect(&log, event.id, cid, vec![]).unwrap();
    assert_eq!(
        via_provider_get_effect_for_event(&log, event.id).unwrap(),
        Some(effect)
    );
}
