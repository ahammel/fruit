use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use exn::Exn;
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

#[async_trait]
impl EventLogProvider for StubEventLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }

    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }

    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

#[async_trait]
impl EventLogPersistor for StubEventLog {
    type Error = Error;
    async fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        let mut n = self.next_id.lock().unwrap();
        *n += 1;
        Ok(Event {
            id: SequenceId::new(*n),
            community_id,
            payload,
        })
    }

    async fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
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

#[async_trait]
impl EventLogProvider for GiftBetweenGrantsLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }

    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }

    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![gift_record(
            self.community_id,
            1,
            self.sender_id,
            self.recipient_id,
        )])
    }

    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![gift_record(
            self.community_id,
            1,
            self.sender_id,
            self.recipient_id,
        )])
    }
}

#[async_trait]
impl EventLogPersistor for GiftBetweenGrantsLog {
    type Error = Error;
    async fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        let mut n = self.next_id.lock().unwrap();
        *n += 1;
        Ok(Event {
            id: SequenceId::new(*n),
            community_id,
            payload,
        })
    }

    async fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
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

#[async_trait]
impl EventLogProvider for ErrorEventLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }

    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }

    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }

    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }

    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }

    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }

    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }
}

#[async_trait]
impl EventLogPersistor for ErrorEventLog {
    type Error = Error;
    async fn append_event(&self, _: CommunityId, _: EventPayload) -> Result<Event, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }

    async fn append_effect(
        &self,
        _: SequenceId,
        _: CommunityId,
        _: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }
}

impl EventLogRepo for ErrorEventLog {}

// ── community providers ────────────────────────────────────────────────────

struct NoneProvider;

#[async_trait]
impl CommunityProvider for NoneProvider {
    type Error = Error;
    async fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }

    async fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }
}

#[async_trait]
impl CommunityPersistor for NoneProvider {
    type Error = Error;
    async fn put(&self, c: Community) -> Result<Community, Exn<Error>> {
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

#[async_trait]
impl EventLogProvider for OrphanedGrantLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }

    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }

    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![self.orphaned.clone()])
    }

    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

#[async_trait]
impl EventLogPersistor for OrphanedGrantLog {
    type Error = Error;
    async fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        *self.new_event_appended.lock().unwrap() = true;
        Ok(Event {
            id: SequenceId::new(2),
            community_id,
            payload,
        })
    }

    async fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        Ok(Effect {
            id: event_id,
            community_id,
            mutations,
        })
    }
}

impl EventLogRepo for OrphanedGrantLog {}

// ── tests ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn no_prior_history_mutations_equal_granter_output() {
    let community = Community::new();
    let (granter, _) = CapturingGranter::new(vec![StateMutation::BurnLuckBonus { delta: 5 }]);
    let mut providence = Providence::new(StubEventLog::new(), NoneProvider, granter);

    let result = providence.grant_fruit(&community, 1).await.unwrap();

    assert_eq!(result, vec![StateMutation::BurnLuckBonus { delta: 5 }]);
}

#[tokio::test]
async fn gift_luck_bonus_applied_before_granting() {
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

    providence.grant_fruit(&community, 1).await.unwrap();

    // GiftLuckBonus delta=10 and OstentatiousGiftPenalty delta=-5 → net luck_raw=5
    // luck() = 5/255 > 0
    let luck = luck_seen.lock().unwrap().unwrap();
    assert!(
        luck > 0.0,
        "expected sender luck > 0 after GiftLuckBonus, got {luck}"
    );
}

#[tokio::test]
async fn event_log_error_propagates() {
    let community = Community::new();
    let (granter, _) = CapturingGranter::new(vec![]);
    let mut providence = Providence::new(ErrorEventLog, NoneProvider, granter);

    assert!(providence.grant_fruit(&community, 1).await.is_err());
}

#[tokio::test]
async fn orphaned_grant_is_completed_without_appending_new_event() {
    // Simulate a crash between append_event(Grant) and append_effect: an orphaned
    // Grant event exists (id=1) with no effect. On retry, grant_fruit should
    // resume the orphaned event rather than appending a second Grant event.
    let community = Community::new();
    let (event_log, new_event_appended) = OrphanedGrantLog::new(community.id);
    let (granter, _) = CapturingGranter::new(vec![StateMutation::BurnLuckBonus { delta: 7 }]);
    let mut providence = Providence::new(event_log, NoneProvider, granter);

    let result = providence.grant_fruit(&community, 1).await.unwrap();

    assert!(
        !*new_event_appended.lock().unwrap(),
        "should resume orphaned grant, not append a new event"
    );
    assert_eq!(result, vec![StateMutation::BurnLuckBonus { delta: 7 }]);
}

// ── error propagation tests ────────────────────────────────────────────────────

// Returns one grant event so get_effect_for_event is reached; errors there.
struct GetEffectForEventErrorLog {
    community_id: CommunityId,
}

#[async_trait]
impl EventLogProvider for GetEffectForEventErrorLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![Event {
            id: SequenceId::new(1),
            community_id: self.community_id,
            payload: EventPayload::Grant { count: 1 },
        }])
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

#[async_trait]
impl EventLogPersistor for GetEffectForEventErrorLog {
    type Error = Error;
    async fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        Ok(Event {
            id: SequenceId::new(2),
            community_id,
            payload,
        })
    }
    async fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        Ok(Effect {
            id: event_id,
            community_id,
            mutations,
        })
    }
}

impl EventLogRepo for GetEffectForEventErrorLog {}

#[tokio::test]
async fn get_effect_for_event_error_propagates() {
    let community = Community::new();
    let (granter, _) = CapturingGranter::new(vec![]);
    let mut providence = Providence::new(
        GetEffectForEventErrorLog {
            community_id: community.id,
        },
        NoneProvider,
        granter,
    );
    assert!(providence.grant_fruit(&community, 1).await.is_err());
}

// No prior grants so append_event is reached; errors there.
struct AppendEventErrorLog;

#[async_trait]
impl EventLogProvider for AppendEventErrorLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

#[async_trait]
impl EventLogPersistor for AppendEventErrorLog {
    type Error = Error;
    async fn append_event(&self, _: CommunityId, _: EventPayload) -> Result<Event, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }
    async fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        Ok(Effect {
            id: event_id,
            community_id,
            mutations,
        })
    }
}

impl EventLogRepo for AppendEventErrorLog {}

#[tokio::test]
async fn append_event_error_propagates() {
    let community = Community::new();
    let (granter, _) = CapturingGranter::new(vec![]);
    let mut providence = Providence::new(AppendEventErrorLog, NoneProvider, granter);
    assert!(providence.grant_fruit(&community, 1).await.is_err());
}

// Community provider that errors on get.
struct ErrorCommunityProvider;

#[async_trait]
impl CommunityProvider for ErrorCommunityProvider {
    type Error = Error;
    async fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }
    async fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Ok(None)
    }
}

#[tokio::test]
async fn community_provider_error_propagates() {
    let community = Community::new();
    let (granter, _) = CapturingGranter::new(vec![]);
    let mut providence = Providence::new(StubEventLog::new(), ErrorCommunityProvider, granter);
    assert!(providence.grant_fruit(&community, 1).await.is_err());
}

// Succeeds through append_event; errors on get_records_between.
struct GetRecordsBetweenErrorLog {
    next_id: Mutex<u64>,
}

impl GetRecordsBetweenErrorLog {
    fn new() -> Self {
        Self {
            next_id: Mutex::new(0),
        }
    }
}

#[async_trait]
impl EventLogProvider for GetRecordsBetweenErrorLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }
}

#[async_trait]
impl EventLogPersistor for GetRecordsBetweenErrorLog {
    type Error = Error;
    async fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        let mut n = self.next_id.lock().unwrap();
        *n += 1;
        Ok(Event {
            id: SequenceId::new(*n),
            community_id,
            payload,
        })
    }
    async fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        Ok(Effect {
            id: event_id,
            community_id,
            mutations,
        })
    }
}

impl EventLogRepo for GetRecordsBetweenErrorLog {}

#[tokio::test]
async fn get_records_between_error_propagates() {
    let community = Community::new();
    let (granter, _) = CapturingGranter::new(vec![]);
    let mut providence = Providence::new(GetRecordsBetweenErrorLog::new(), NoneProvider, granter);
    assert!(providence.grant_fruit(&community, 1).await.is_err());
}

// Succeeds through get_records_between; errors on get_latest_gift_records.
struct GetLatestGiftRecordsErrorLog {
    next_id: Mutex<u64>,
}

impl GetLatestGiftRecordsErrorLog {
    fn new() -> Self {
        Self {
            next_id: Mutex::new(0),
        }
    }
}

#[async_trait]
impl EventLogProvider for GetLatestGiftRecordsErrorLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

#[async_trait]
impl EventLogPersistor for GetLatestGiftRecordsErrorLog {
    type Error = Error;
    async fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        let mut n = self.next_id.lock().unwrap();
        *n += 1;
        Ok(Event {
            id: SequenceId::new(*n),
            community_id,
            payload,
        })
    }
    async fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        Ok(Effect {
            id: event_id,
            community_id,
            mutations,
        })
    }
}

impl EventLogRepo for GetLatestGiftRecordsErrorLog {}

#[tokio::test]
async fn get_latest_gift_records_error_propagates() {
    let community = Community::new();
    let (granter, _) = CapturingGranter::new(vec![]);
    let mut providence =
        Providence::new(GetLatestGiftRecordsErrorLog::new(), NoneProvider, granter);
    assert!(providence.grant_fruit(&community, 1).await.is_err());
}

// Succeeds through get_latest_gift_records; errors on append_effect.
struct AppendEffectErrorLog {
    next_id: Mutex<u64>,
}

impl AppendEffectErrorLog {
    fn new() -> Self {
        Self {
            next_id: Mutex::new(0),
        }
    }
}

#[async_trait]
impl EventLogProvider for AppendEffectErrorLog {
    type Error = Error;
    async fn get_record(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Record>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effect_for_event(
        &self,
        _: CommunityId,
        _: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        Ok(None)
    }
    async fn get_effects_after(
        &self,
        _: CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_before(
        &self,
        _: CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_grant_events(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_latest_gift_records(
        &self,
        _: CommunityId,
        _: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
    async fn get_records_between(
        &self,
        _: CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }
}

#[async_trait]
impl EventLogPersistor for AppendEffectErrorLog {
    type Error = Error;
    async fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        let mut n = self.next_id.lock().unwrap();
        *n += 1;
        Ok(Event {
            id: SequenceId::new(*n),
            community_id,
            payload,
        })
    }
    async fn append_effect(
        &self,
        _: SequenceId,
        _: CommunityId,
        _: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        Err(Exn::new(Error::GrantInterrupted("err".to_string())))
    }
}

impl EventLogRepo for AppendEffectErrorLog {}

#[tokio::test]
async fn append_effect_error_propagates() {
    let community = Community::new();
    let (granter, _) = CapturingGranter::new(vec![]);
    let mut providence = Providence::new(AppendEffectErrorLog::new(), NoneProvider, granter);
    assert!(providence.grant_fruit(&community, 1).await.is_err());
}
