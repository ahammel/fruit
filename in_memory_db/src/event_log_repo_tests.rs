use super::*;

use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;

use fruit_domain::{
    community::HasCommunityId,
    event_log::{HasSequenceId, Record},
    fruit::STRAWBERRY,
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
    assert!(log.get_record(SequenceId::new(1)).unwrap().is_none());
    assert!(log
        .get_effect_for_event(SequenceId::new(1))
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
    assert!(log.get_record(SequenceId::new(1)).is_err());
}

#[test]
fn get_record_returns_err_when_effect_lock_is_poisoned() {
    use fruit_domain::event_log::EventPayload;
    let id = SequenceId::new(1);
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
    assert!(log.get_effect_for_event(SequenceId::new(1)).is_err());
}

#[test]
fn append_effect_returns_err_when_lock_is_poisoned() {
    let log = InMemoryEventLogRepo {
        sequence: AtomicU64::new(0),
        events: RwLock::new(HashMap::new()),
        effects: poisoned_effect_map(),
    };
    assert!(log
        .append_effect(SequenceId::new(1), community_id(), vec![])
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
    assert_eq!(
        log.get_effects_after(cid, 100, eff1.id).unwrap(),
        vec![eff2]
    );
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
        log.get_effects_after(cid1, 100, SequenceId::zero())
            .unwrap(),
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
    let effects = log.get_effects_after(cid, 100, SequenceId::zero()).unwrap();
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
        .get_effects_after(community_id(), 100, SequenceId::zero())
        .is_err());
}

#[test]
fn get_effects_after_respects_limit() {
    let log = log();
    let cid = community_id();
    let e1 = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    let e2 = log
        .append_event(cid, EventPayload::Grant { count: 2 })
        .unwrap();
    let eff1 = log.append_effect(e1.id, cid, vec![]).unwrap();
    log.append_effect(e2.id, cid, vec![]).unwrap();
    // limit=1 should return only the first effect
    assert_eq!(
        log.get_effects_after(cid, 1, SequenceId::zero()).unwrap(),
        vec![eff1]
    );
}

#[test]
fn get_records_before_returns_err_when_events_lock_is_poisoned() {
    let log = InMemoryEventLogRepo {
        sequence: AtomicU64::new(0),
        events: poisoned_event_map(),
        effects: RwLock::new(HashMap::new()),
    };
    assert!(log.get_records_before(community_id(), 5, None).is_err());
}

#[test]
fn get_records_before_returns_err_when_effects_lock_is_poisoned() {
    let log = InMemoryEventLogRepo {
        sequence: AtomicU64::new(0),
        events: RwLock::new(HashMap::new()),
        effects: poisoned_effect_map(),
    };
    assert!(log.get_records_before(community_id(), 5, None).is_err());
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
    assert!(log().get_record(SequenceId::new(99)).unwrap().is_none());
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
        .get_effect_for_event(SequenceId::new(1))
        .unwrap()
        .is_none());
}

// --- duplicate write rejection ---

#[test]
fn append_event_fails_on_duplicate_sequence_id() {
    let log = log();
    let cid = community_id();
    let dummy = Event {
        id: SequenceId::new(1),
        community_id: cid,
        payload: EventPayload::Grant { count: 0 },
    };
    log.events
        .write()
        .unwrap()
        .insert(SequenceId::new(1), dummy);
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
    assert_eq!(event.id, SequenceId::new(1));
    assert_eq!(effect.id, SequenceId::new(1));
    let event2 = log
        .append_event(cid, EventPayload::Grant { count: 2 })
        .unwrap();
    assert_eq!(event2.id, SequenceId::new(2));
}

// --- get_records_before ---

#[test]
fn get_records_before_includes_effects() {
    let log = log();
    let cid = community_id();
    let event = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    let effect = log.append_effect(event.id, cid, vec![]).unwrap();
    assert_eq!(
        log.get_records_before(cid, 10, None).unwrap(),
        vec![Record {
            event,
            effect: Some(effect),
        }]
    );
}

#[test]
fn get_records_before_pending_event_has_none_effect() {
    let log = log();
    let cid = community_id();
    let event = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    assert_eq!(
        log.get_records_before(cid, 10, None).unwrap(),
        vec![Record {
            event,
            effect: None,
        }]
    );
}

#[test]
fn get_records_before_returns_n_most_recent_descending() {
    let log = log();
    let cid = community_id();
    for i in 1..=5 {
        log.append_event(cid, EventPayload::Grant { count: i })
            .unwrap();
    }
    let records = log.get_records_before(cid, 3, None).unwrap();
    assert_eq!(records.len(), 3);
    assert!(records
        .windows(2)
        .all(|w| w[0].sequence_id() > w[1].sequence_id()));
}

#[test]
fn get_records_before_returns_fewer_when_not_enough() {
    let log = log();
    let cid = community_id();
    log.append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    assert_eq!(log.get_records_before(cid, 10, None).unwrap().len(), 1);
}

#[test]
fn get_records_before_filters_by_community() {
    let log = log();
    let cid1 = community_id();
    let cid2 = community_id();
    log.append_event(cid1, EventPayload::Grant { count: 1 })
        .unwrap();
    log.append_event(cid2, EventPayload::Grant { count: 1 })
        .unwrap();
    let records = log.get_records_before(cid1, 10, None).unwrap();
    assert!(records.iter().all(|e| e.community_id() == cid1));
}

#[test]
fn get_records_before_returns_empty_when_none_exist() {
    assert!(log()
        .get_records_before(community_id(), 5, None)
        .unwrap()
        .is_empty());
}

#[test]
fn get_records_before_excludes_record_at_exact_cursor() {
    let log = log();
    let cid = community_id();
    for _ in 0..3 {
        log.append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
    }
    // cursor == id of 3rd event; result must not include it
    let cursor = SequenceId::new(3);
    let records = log.get_records_before(cid, 10, Some(cursor)).unwrap();
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|r| r.sequence_id() < cursor));
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
    limit: usize,
    after: SequenceId,
) -> Result<Vec<Effect>, Error> {
    p.get_effects_after(cid, limit, after)
}

fn via_provider_get_records_before<T: EventLogProvider>(
    p: T,
    cid: CommunityId,
    limit: usize,
    before: Option<SequenceId>,
) -> Result<Vec<Record>, Error> {
    p.get_records_before(cid, limit, before)
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
        via_provider_get_effects_after(&log, cid, 100, SequenceId::zero()).unwrap(),
        vec![effect]
    );
}

#[test]
fn ref_delegates_get_records_before() {
    let log = log();
    let cid = community_id();
    let event = via_persistor_append_event(&log, cid, EventPayload::Grant { count: 1 }).unwrap();
    assert_eq!(
        via_provider_get_records_before(&log, cid, 5, None).unwrap(),
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

// --- get_latest_grant_events ---

#[test]
fn get_latest_grant_events_returns_most_recent_descending() {
    let log = log();
    let cid = community_id();
    log.append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    let g2 = log
        .append_event(cid, EventPayload::Grant { count: 2 })
        .unwrap();
    let g3 = log
        .append_event(cid, EventPayload::Grant { count: 3 })
        .unwrap();
    let events = log.get_latest_grant_events(cid, 2).unwrap();
    assert_eq!(events, vec![g3, g2]);
}

#[test]
fn get_latest_grant_events_ignores_gift_and_burn() {
    use fruit_domain::fruit::STRAWBERRY;
    use fruit_domain::member::MemberId;
    let log = log();
    let cid = community_id();
    let mid = MemberId::new();
    log.append_event(
        cid,
        EventPayload::Gift {
            sender_id: mid,
            recipient_id: mid,
            fruit: STRAWBERRY,
            message: None,
        },
    )
    .unwrap();
    log.append_event(
        cid,
        EventPayload::Burn {
            member_id: mid,
            fruits: vec![],
        },
    )
    .unwrap();
    let grant = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    let events = log.get_latest_grant_events(cid, 10).unwrap();
    assert_eq!(events, vec![grant]);
}

#[test]
fn get_latest_grant_events_respects_limit() {
    let log = log();
    let cid = community_id();
    for i in 1..=5 {
        log.append_event(cid, EventPayload::Grant { count: i })
            .unwrap();
    }
    assert_eq!(log.get_latest_grant_events(cid, 2).unwrap().len(), 2);
}

// --- get_latest_gift_records ---

#[test]
fn get_latest_gift_records_returns_most_recent_descending() {
    use fruit_domain::fruit::STRAWBERRY;
    use fruit_domain::member::MemberId;
    let log = log();
    let cid = community_id();
    let mid = MemberId::new();
    log.append_event(
        cid,
        EventPayload::Gift {
            sender_id: mid,
            recipient_id: mid,
            fruit: STRAWBERRY,
            message: None,
        },
    )
    .unwrap();
    let g2 = log
        .append_event(
            cid,
            EventPayload::Gift {
                sender_id: mid,
                recipient_id: mid,
                fruit: STRAWBERRY,
                message: None,
            },
        )
        .unwrap();
    let records = log.get_latest_gift_records(cid, 10).unwrap();
    assert_eq!(records.len(), 2);
    assert!(records[0].sequence_id() > records[1].sequence_id());
    assert_eq!(records[0].event.id, g2.id);
}

#[test]
fn get_latest_gift_records_ignores_grant_and_burn() {
    use fruit_domain::fruit::STRAWBERRY;
    use fruit_domain::member::MemberId;
    let log = log();
    let cid = community_id();
    let mid = MemberId::new();
    log.append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    log.append_event(
        cid,
        EventPayload::Burn {
            member_id: mid,
            fruits: vec![],
        },
    )
    .unwrap();
    let gift_event = log
        .append_event(
            cid,
            EventPayload::Gift {
                sender_id: mid,
                recipient_id: mid,
                fruit: STRAWBERRY,
                message: None,
            },
        )
        .unwrap();
    let eff = log.append_effect(gift_event.id, cid, vec![]).unwrap();
    let records = log.get_latest_gift_records(cid, 10).unwrap();
    assert_eq!(
        records,
        vec![Record {
            event: gift_event,
            effect: Some(eff)
        }]
    );
}

#[test]
fn get_latest_gift_records_respects_limit() {
    use fruit_domain::fruit::STRAWBERRY;
    use fruit_domain::member::MemberId;
    let log = log();
    let cid = community_id();
    let mid = MemberId::new();
    for _ in 0..5 {
        log.append_event(
            cid,
            EventPayload::Gift {
                sender_id: mid,
                recipient_id: mid,
                fruit: STRAWBERRY,
                message: None,
            },
        )
        .unwrap();
    }
    assert_eq!(log.get_latest_gift_records(cid, 3).unwrap().len(), 3);
}

// --- get_records_between ---

#[test]
fn get_records_between_returns_ascending_exclusive_bounds() {
    let log = log();
    let cid = community_id();
    let e1 = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    let e2 = log
        .append_event(cid, EventPayload::Grant { count: 2 })
        .unwrap();
    let e3 = log
        .append_event(cid, EventPayload::Grant { count: 3 })
        .unwrap();
    let e4 = log
        .append_event(cid, EventPayload::Grant { count: 4 })
        .unwrap();
    // ask for records strictly between e1 and e4
    let records = log.get_records_between(cid, e1.id, e4.id).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].event.id, e2.id);
    assert_eq!(records[1].event.id, e3.id);
}

#[test]
fn get_records_between_excludes_endpoints() {
    let log = log();
    let cid = community_id();
    let e1 = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    let e2 = log
        .append_event(cid, EventPayload::Grant { count: 2 })
        .unwrap();
    // strictly between e1 and e2 → empty
    assert!(log
        .get_records_between(cid, e1.id, e2.id)
        .unwrap()
        .is_empty());
}

#[test]
fn get_records_between_includes_records_with_no_effect() {
    let log = log();
    let cid = community_id();
    let e1 = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    log.append_effect(e1.id, cid, vec![]).unwrap();
    let e2 = log
        .append_event(cid, EventPayload::Grant { count: 2 })
        .unwrap();
    // no effect appended for e2
    let e3 = log
        .append_event(cid, EventPayload::Grant { count: 3 })
        .unwrap();
    log.append_effect(e3.id, cid, vec![]).unwrap();
    let records = log.get_records_between(cid, e1.id, e3.id).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].event.id, e2.id);
    assert!(records[0].effect.is_none());
}

fn via_provider_get_latest_grant_events<T: EventLogProvider>(
    p: T,
    cid: CommunityId,
    limit: usize,
) -> Result<Vec<Event>, Error> {
    p.get_latest_grant_events(cid, limit)
}

fn via_provider_get_latest_gift_records<T: EventLogProvider>(
    p: T,
    cid: CommunityId,
    limit: usize,
) -> Result<Vec<Record>, Error> {
    p.get_latest_gift_records(cid, limit)
}

fn via_provider_get_records_between<T: EventLogProvider>(
    p: T,
    cid: CommunityId,
    after: SequenceId,
    before: SequenceId,
) -> Result<Vec<Record>, Error> {
    p.get_records_between(cid, after, before)
}

#[test]
fn ref_delegates_get_latest_grant_events() {
    let log = log();
    let cid = community_id();
    let event = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    assert_eq!(
        via_provider_get_latest_grant_events(&log, cid, 10).unwrap(),
        vec![event]
    );
}

#[test]
fn ref_delegates_get_latest_gift_records() {
    let log = log();
    let cid = community_id();
    let mid = MemberId::new();
    let event = log
        .append_event(
            cid,
            EventPayload::Gift {
                sender_id: mid,
                recipient_id: mid,
                fruit: STRAWBERRY,
                message: None,
            },
        )
        .unwrap();
    let records = via_provider_get_latest_gift_records(&log, cid, 10).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].event.id, event.id);
}

#[test]
fn ref_delegates_get_records_between() {
    let log = log();
    let cid = community_id();
    let e1 = log
        .append_event(cid, EventPayload::Grant { count: 1 })
        .unwrap();
    let e2 = log
        .append_event(cid, EventPayload::Grant { count: 2 })
        .unwrap();
    let e3 = log
        .append_event(cid, EventPayload::Grant { count: 3 })
        .unwrap();
    let records = via_provider_get_records_between(&log, cid, e1.id, e3.id).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].event.id, e2.id);
}
