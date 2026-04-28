use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use exn::Exn;

use super::*;
use crate::{
    bag::Bag,
    community::{Community, CommunityId},
    community_repo::{CommunityPersistor, CommunityProvider},
    error::Error,
    event_log::{Effect, Event, EventPayload, Record, SequenceId, StateMutation},
    event_log_repo::EventLogProvider,
    fruit::GRAPES,
    member::Member,
};

// ── minimal mock plumbing ──────────────────────────────────────────────────

struct MockEventLog {
    grant_events: Vec<Event>,
    gift_records: Vec<Record>,
    between_records: Vec<Record>,
    gift_limit_seen: Arc<Mutex<Option<usize>>>,
}

impl MockEventLog {
    fn new() -> Self {
        Self {
            grant_events: vec![],
            gift_records: vec![],
            between_records: vec![],
            gift_limit_seen: Arc::new(Mutex::new(None)),
        }
    }

    fn with_grant_events(mut self, events: Vec<Event>) -> Self {
        self.grant_events = events;
        self
    }

    fn with_gift_records(mut self, records: Vec<Record>) -> Self {
        self.gift_records = records;
        self
    }

    fn with_between_records(mut self, records: Vec<Record>) -> Self {
        self.between_records = records;
        self
    }

    fn gift_limit_seen(&self) -> Arc<Mutex<Option<usize>>> {
        self.gift_limit_seen.clone()
    }
}

#[async_trait]
impl EventLogProvider for MockEventLog {
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
        _: crate::community::CommunityId,
        _: usize,
        _: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_records_before(
        &self,
        _: crate::community::CommunityId,
        _: usize,
        _: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(vec![])
    }

    async fn get_latest_grant_events(
        &self,
        _: crate::community::CommunityId,
        _: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        Ok(self.grant_events.clone())
    }

    async fn get_latest_gift_records(
        &self,
        _: crate::community::CommunityId,
        limit: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        *self.gift_limit_seen.lock().unwrap() = Some(limit);
        Ok(self.gift_records.iter().take(limit).cloned().collect())
    }

    async fn get_records_between(
        &self,
        _: crate::community::CommunityId,
        _: SequenceId,
        _: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        Ok(self.between_records.clone())
    }
}

struct MockCommunityProvider {
    community: Option<Community>,
}

impl MockCommunityProvider {
    fn none() -> Self {
        Self { community: None }
    }

    fn with(community: Community) -> Self {
        Self {
            community: Some(community),
        }
    }
}

#[async_trait]
impl CommunityProvider for MockCommunityProvider {
    type Error = Error;
    async fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Exn<Error>> {
        Ok(self.community.clone())
    }

    async fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Ok(self.community.clone())
    }
}

#[async_trait]
impl CommunityPersistor for MockCommunityProvider {
    type Error = Error;
    async fn put(&self, c: Community) -> Result<Community, Exn<Error>> {
        Ok(c)
    }
}

fn seq(n: u64) -> SequenceId {
    SequenceId::new(n)
}

fn grant_event(community_id: CommunityId, id: u64) -> Event {
    Event {
        id: seq(id),
        community_id,
        payload: EventPayload::Grant { count: 1 },
    }
}

fn gift_record_nonempty(
    community_id: CommunityId,
    id: u64,
    sender_id: crate::member::MemberId,
    recipient_id: crate::member::MemberId,
) -> Record {
    Record {
        event: Event {
            id: seq(id),
            community_id,
            payload: EventPayload::Gift {
                sender_id,
                recipient_id,
                fruit: GRAPES,
                message: None,
            },
        },
        effect: Some(Effect {
            id: seq(id),
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

fn gift_record_noop(
    community_id: CommunityId,
    id: u64,
    sender_id: crate::member::MemberId,
    recipient_id: crate::member::MemberId,
) -> Record {
    Record {
        event: Event {
            id: seq(id),
            community_id,
            payload: EventPayload::Gift {
                sender_id,
                recipient_id,
                fruit: GRAPES,
                message: None,
            },
        },
        effect: Some(Effect {
            id: seq(id),
            community_id,
            mutations: vec![],
        }),
    }
}

// ── tests ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn no_prior_grants_no_gifts_returns_empty() {
    let community = Community::new();
    let adjuster = LuckAdjuster::new(MockEventLog::new(), MockCommunityProvider::none());
    let result = adjuster.compute(&community, seq(10)).await.unwrap();
    assert_eq!(result, vec![]);
}

#[tokio::test]
async fn gift_between_grants_produces_bonus() {
    let mut community = Community::new();
    let sender = Member::new("Alice");
    // recipient holds enough fruit to avoid ostentation penalty
    let recipient =
        Member::new("Bob").with_bag(Bag::new().insert(GRAPES).insert(GRAPES).insert(GRAPES));
    community.add_member(sender.clone());
    community.add_member(recipient.clone());

    // Grant at seq 1, gift at seq 2, current grant at seq 10
    let grant = grant_event(community.id, 1);
    let gift = gift_record_nonempty(community.id, 2, sender.id, recipient.id);

    let adjuster = LuckAdjuster::new(
        MockEventLog::new()
            .with_grant_events(vec![grant])
            .with_between_records(vec![gift.clone()])
            .with_gift_records(vec![gift]),
        MockCommunityProvider::with(community),
    );

    let result = adjuster.compute(&Community::new(), seq(10)).await.unwrap();

    // GRAPES value=1.0, delta = round(1.0 * 10.0) = 10
    assert_eq!(
        result,
        vec![StateMutation::GiftLuckBonus {
            member_id: sender.id,
            delta: 10
        }]
    );
}

#[tokio::test]
async fn recent_gift_records_capped_at_100() {
    let community = Community::new();
    let limit_seen = {
        let event_log = MockEventLog::new();
        let seen = event_log.gift_limit_seen();
        let adjuster = LuckAdjuster::new(event_log, MockCommunityProvider::none());
        adjuster.compute(&community, seq(1)).await.unwrap();
        seen
    };
    assert_eq!(*limit_seen.lock().unwrap(), Some(100));
}

#[tokio::test]
async fn noop_gift_between_grants_excluded_from_bonus() {
    let mut community = Community::new();
    let sender = Member::new("Alice");
    let recipient = Member::new("Bob");
    community.add_member(sender.clone());
    community.add_member(recipient.clone());

    let grant = grant_event(community.id, 1);
    let noop_gift = gift_record_noop(community.id, 2, sender.id, recipient.id);

    let adjuster = LuckAdjuster::new(
        MockEventLog::new()
            .with_grant_events(vec![grant])
            .with_between_records(vec![noop_gift.clone()])
            .with_gift_records(vec![noop_gift]),
        MockCommunityProvider::with(community),
    );

    let result = adjuster.compute(&Community::new(), seq(10)).await.unwrap();
    assert_eq!(result, vec![]);
}
