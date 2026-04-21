use std::{
    io,
    sync::{Arc, Mutex},
};

use newtype_ids_uuid::UuidIdentifier as _;

use super::*;
use crate::{
    community::{Community, CommunityId},
    community_repo::{CommunityPersistor, CommunityProvider},
    error::Error,
    event_log::{Effect, Event, EventPayload, Record, SequenceId, StateMutation},
    event_log_repo::{EventLogPersistor, EventLogProvider, EventLogRepo},
    fruit::GRAPES,
    granter::Granter,
    member::Member,
};

// ── stub event log (reads return empty, writes succeed) ───────────────────

#[derive(Default)]
struct StubEventLog {
    next_id: Mutex<u64>,
}

impl StubEventLog {
    fn new() -> Self {
        Self::default()
    }
}

impl EventLogProvider for StubEventLog {
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Error> {
        Ok(None)
    }

    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Error> {
        Ok(None)
    }

    fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Error> {
        Ok(vec![])
    }

    fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Error> {
        Ok(vec![])
    }

    fn get_latest_grant_events(&self, _: CommunityId, _: usize) -> Result<Vec<Event>, Error> {
        Ok(vec![])
    }

    fn get_latest_gift_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Error> {
        Ok(vec![])
    }

    fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Error> {
        Ok(vec![])
    }
}

impl EventLogPersistor for StubEventLog {
    fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Error> {
        let mut n = self.next_id.lock().unwrap();
        *n += 1;
        Ok(Event {
            id: SequenceId::new(*n),
            community_id,
            payload,
        })
    }

    fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Error> {
        Ok(Effect {
            id: event_id,
            community_id,
            mutations,
        })
    }
}

impl EventLogRepo for StubEventLog {}

// ── event log with a gift record between the grants ───────────────────────

struct GiftBetweenGrantsLog {
    community_id: CommunityId,
    sender_id: crate::member::MemberId,
    recipient_id: crate::member::MemberId,
    next_id: Mutex<u64>,
}

impl GiftBetweenGrantsLog {
    fn new(
        community_id: CommunityId,
        sender_id: crate::member::MemberId,
        recipient_id: crate::member::MemberId,
    ) -> Self {
        Self {
            community_id,
            sender_id,
            recipient_id,
            next_id: Mutex::new(1),
        }
    }
}

impl EventLogProvider for GiftBetweenGrantsLog {
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Error> {
        Ok(None)
    }

    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Error> {
        Ok(None)
    }

    fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Error> {
        Ok(vec![])
    }

    fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Error> {
        Ok(vec![])
    }

    fn get_latest_grant_events(&self, _: CommunityId, _: usize) -> Result<Vec<Event>, Error> {
        Ok(vec![])
    }

    fn get_latest_gift_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Error> {
        Ok(vec![gift_record(
            self.community_id,
            1,
            self.sender_id,
            self.recipient_id,
        )])
    }

    fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Error> {
        Ok(vec![gift_record(
            self.community_id,
            1,
            self.sender_id,
            self.recipient_id,
        )])
    }
}

impl EventLogPersistor for GiftBetweenGrantsLog {
    fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Error> {
        let mut n = self.next_id.lock().unwrap();
        *n += 1;
        Ok(Event {
            id: SequenceId::new(*n),
            community_id,
            payload,
        })
    }

    fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Error> {
        Ok(Effect {
            id: event_id,
            community_id,
            mutations,
        })
    }
}

impl EventLogRepo for GiftBetweenGrantsLog {}

fn gift_record(
    community_id: CommunityId,
    id: u64,
    sender_id: crate::member::MemberId,
    recipient_id: crate::member::MemberId,
) -> Record {
    Record {
        event: Event {
            id: SequenceId::new(id),
            community_id,
            payload: EventPayload::Gift {
                sender_id,
                recipient_id,
                fruit: GRAPES,
                message: None,
            },
        },
        effect: Some(Effect {
            id: SequenceId::new(id),
            community_id,
            mutations: vec![
                StateMutation::RemoveFruitFromMember {
                    member_id: sender_id,
                    fruit: GRAPES,
                },
                StateMutation::AddFruitToMember {
                    member_id: recipient_id,
                    fruit: GRAPES,
                },
            ],
        }),
    }
}

// ── error event log ────────────────────────────────────────────────────────

struct ErrorEventLog;

impl EventLogProvider for ErrorEventLog {
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Error> {
        Err(io::Error::other("err").into())
    }

    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Error> {
        Err(io::Error::other("err").into())
    }

    fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Error> {
        Err(io::Error::other("err").into())
    }

    fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Error> {
        Err(io::Error::other("err").into())
    }

    fn get_latest_grant_events(&self, _: CommunityId, _: usize) -> Result<Vec<Event>, Error> {
        Err(io::Error::other("err").into())
    }

    fn get_latest_gift_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Error> {
        Err(io::Error::other("err").into())
    }

    fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Error> {
        Err(io::Error::other("err").into())
    }
}

impl EventLogPersistor for ErrorEventLog {
    fn append_event(&self, _: CommunityId, _: EventPayload) -> Result<Event, Error> {
        Err(io::Error::other("err").into())
    }

    fn append_effect(
        &self,
        _: SequenceId,
        _: CommunityId,
        _: Vec<StateMutation>,
    ) -> Result<Effect, Error> {
        Err(io::Error::other("err").into())
    }
}

impl EventLogRepo for ErrorEventLog {}

// ── community providers ────────────────────────────────────────────────────

struct NoneProvider;

impl CommunityProvider for NoneProvider {
    fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Error> {
        Ok(None)
    }

    fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Error> {
        Ok(None)
    }
}

impl CommunityPersistor for NoneProvider {
    fn put(&self, c: Community) -> Result<Community, Error> {
        Ok(c)
    }
}

// ── capturing granter ──────────────────────────────────────────────────────

struct CapturingGranter {
    observed_luck: Arc<Mutex<Option<f64>>>,
    fixed_mutations: Vec<StateMutation>,
}

impl CapturingGranter {
    fn new(fixed_mutations: Vec<StateMutation>) -> (Self, Arc<Mutex<Option<f64>>>) {
        let cell = Arc::new(Mutex::new(None));
        let granter = Self {
            observed_luck: cell.clone(),
            fixed_mutations,
        };
        (granter, cell)
    }
}

impl Granter for CapturingGranter {
    fn grant(&mut self, community: &Community, _: usize) -> Vec<StateMutation> {
        let luck = community
            .members
            .values()
            .next()
            .map(|m| m.luck())
            .unwrap_or_else(|| community.luck());
        *self.observed_luck.lock().unwrap() = Some(luck);
        self.fixed_mutations.clone()
    }
}

// ── event log with one orphaned grant (event written, no effect yet) ─────────

struct OrphanedGrantLog {
    orphaned: Event,
    /// Set to true if append_event is called (it should not be on retry).
    new_event_appended: Arc<Mutex<bool>>,
}

impl OrphanedGrantLog {
    fn new(community_id: CommunityId) -> (Self, Arc<Mutex<bool>>) {
        let flag = Arc::new(Mutex::new(false));
        let log = Self {
            orphaned: Event {
                id: SequenceId::new(1),
                community_id,
                payload: EventPayload::Grant { count: 1 },
            },
            new_event_appended: flag.clone(),
        };
        (log, flag)
    }
}

impl EventLogProvider for OrphanedGrantLog {
    fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Error> {
        Ok(None)
    }

    fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Error> {
        Ok(None)
    }

    fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Error> {
        Ok(vec![])
    }

    fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Error> {
        Ok(vec![])
    }

    fn get_latest_grant_events(&self, _: CommunityId, _: usize) -> Result<Vec<Event>, Error> {
        Ok(vec![self.orphaned.clone()])
    }

    fn get_latest_gift_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Error> {
        Ok(vec![])
    }

    fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Error> {
        Ok(vec![])
    }
}

impl EventLogPersistor for OrphanedGrantLog {
    fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Error> {
        *self.new_event_appended.lock().unwrap() = true;
        Ok(Event {
            id: SequenceId::new(2),
            community_id,
            payload,
        })
    }

    fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Error> {
        Ok(Effect {
            id: event_id,
            community_id,
            mutations,
        })
    }
}

impl EventLogRepo for OrphanedGrantLog {}

// ── tests ──────────────────────────────────────────────────────────────────

#[test]
fn no_prior_history_mutations_equal_granter_output() {
    let community = Community::new();
    let (granter, _) = CapturingGranter::new(vec![StateMutation::BurnLuckBonus { delta: 5 }]);
    let mut providence = Providence::new(StubEventLog::new(), NoneProvider, granter);

    let result = providence.grant_fruit(&community, 1).unwrap();

    assert_eq!(result, vec![StateMutation::BurnLuckBonus { delta: 5 }]);
}

#[test]
fn gift_luck_bonus_applied_before_granting() {
    // Only Alice (sender) is in the community so CapturingGranter deterministically
    // sees her luck. The recipient is outside the community, so recipient_bag_val=0
    // and the gift is ostentatious, but net luck = clamp(0+10-5, 0, 255) = 5 > 0.
    let mut community = Community::new();
    let sender = Member::new("Alice");
    let recipient_id = crate::member::MemberId::new();
    community.add_member(sender.clone());

    let event_log = GiftBetweenGrantsLog::new(community.id, sender.id, recipient_id);
    let (granter, luck_seen) = CapturingGranter::new(vec![]);
    let mut providence = Providence::new(event_log, NoneProvider, granter);

    providence.grant_fruit(&community, 1).unwrap();

    // GiftLuckBonus delta=10 and OstentatiousGiftPenalty delta=-5 → net luck_raw=5
    // luck() = 5/255 > 0
    let luck = luck_seen.lock().unwrap().unwrap();
    assert!(
        luck > 0.0,
        "expected sender luck > 0 after GiftLuckBonus, got {luck}"
    );
}

#[test]
fn event_log_error_propagates() {
    let community = Community::new();
    let (granter, _) = CapturingGranter::new(vec![]);
    let mut providence = Providence::new(ErrorEventLog, NoneProvider, granter);

    assert!(providence.grant_fruit(&community, 1).is_err());
}

#[test]
fn orphaned_grant_is_completed_without_appending_new_event() {
    // Simulate a crash between append_event(Grant) and append_effect: an orphaned
    // Grant event exists (id=1) with no effect. On retry, grant_fruit should
    // resume the orphaned event rather than appending a second Grant event.
    let community = Community::new();
    let (event_log, new_event_appended) = OrphanedGrantLog::new(community.id);
    let (granter, _) = CapturingGranter::new(vec![StateMutation::BurnLuckBonus { delta: 7 }]);
    let mut providence = Providence::new(event_log, NoneProvider, granter);

    let result = providence.grant_fruit(&community, 1).unwrap();

    assert!(
        !*new_event_appended.lock().unwrap(),
        "should resume orphaned grant, not append a new event"
    );
    assert_eq!(result, vec![StateMutation::BurnLuckBonus { delta: 7 }]);
}
