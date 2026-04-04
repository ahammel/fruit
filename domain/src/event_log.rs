use std::fmt;

use crate::{
    community::{Community, CommunityId, HasCommunityId},
    fruit::Fruit,
    id::IntegerIdentifier,
    member::MemberId,
};

/// A position in the shared event/effect log sequence.
///
/// Sequence IDs start at 1 and increase monotonically. Both events and effects
/// draw from the same counter, so their IDs are globally ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SequenceId(u64);

impl IntegerIdentifier for SequenceId {
    fn zero() -> Self {
        Self(0)
    }

    fn from_u64(id: u64) -> Self {
        Self(id)
    }

    fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for SequenceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SequenceId({})", self.0)
    }
}

pub trait HasSequenceId {
    fn sequence_id(&self) -> SequenceId;
}

/// A single entry in the shared event/effect log, identified by a [`SequenceId`](crate::event::SequenceId).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    /// A player intention that was recorded.
    Event(Event),
    /// The computed consequences of an event.
    Effect(Effect),
}

impl HasSequenceId for Record {
    fn sequence_id(&self) -> SequenceId {
        match self {
            Record::Event(e) => e.sequence_id(),
            Record::Effect(e) => e.sequence_id(),
        }
    }
}

impl HasCommunityId for Record {
    fn community_id(&self) -> CommunityId {
        match self {
            Record::Event(e) => e.community_id(),
            Record::Effect(e) => e.community_id(),
        }
    }
}

impl From<Event> for Record {
    fn from(event: Event) -> Self {
        Record::Event(event)
    }
}

impl From<Effect> for Record {
    fn from(effect: Effect) -> Self {
        Record::Effect(effect)
    }
}

/// The action a player intended to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPayload {
    /// Distribute `count` fruits to every member of the community.
    Grant { count: usize },
}

/// A recorded player intention. Events do not modify
/// [`Community`](crate::community::Community) state directly; their consequences are
/// computed as [`Effect`](crate::effect::Effect)s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Position in the shared event/effect sequence.
    pub id: SequenceId,
    /// The community this event applies to.
    pub community_id: CommunityId,
    /// The action the player intended.
    pub payload: EventPayload,
}

impl HasSequenceId for Event {
    fn sequence_id(&self) -> SequenceId {
        self.id
    }
}

impl HasCommunityId for Event {
    fn community_id(&self) -> CommunityId {
        self.community_id
    }
}

/// An atomic change to [`Community`] state produced as part of an [`Effect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateMutation {
    /// Add one instance of `fruit` to the bag of the member identified by `member_id`.
    AddFruitToMember { member_id: MemberId, fruit: Fruit },
}

/// The computed consequence of an [`Event`](crate::event::Event). An effect may contain
/// zero mutations (a no-op, e.g. when the event violated an invariant) or many.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    /// Position in the shared event/effect sequence.
    pub id: SequenceId,
    /// The event that caused this effect.
    pub event_id: SequenceId,
    /// The community this effect applies to.
    pub community_id: CommunityId,
    /// The state changes produced by this effect. Empty if the event was a no-op.
    pub mutations: Vec<StateMutation>,
}

impl Effect {
    /// Apply all mutations in this effect to `community` in order.
    ///
    /// Mutations for members no longer present in the community are silently skipped.
    pub fn apply(&self, community: &mut Community) {
        for mutation in &self.mutations {
            match mutation {
                StateMutation::AddFruitToMember { member_id, fruit } => {
                    if let Some(member) = community.members.get_mut(member_id) {
                        member.receive(*fruit);
                    }
                }
            }
        }
    }
}

impl HasSequenceId for Effect {
    fn sequence_id(&self) -> SequenceId {
        self.id
    }
}

impl HasCommunityId for Effect {
    fn community_id(&self) -> CommunityId {
        self.community_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        community::{Community, CommunityId},
        fruit::STRAWBERRY,
        id::{IntegerIdentifier, UuidIdentifier},
        member::{Member, MemberId},
    };

    #[test]
    fn getters_delegate_to_field_values() {
        let event = Event {
            id: SequenceId::from_u64(1),
            community_id: CommunityId::new(),
            payload: EventPayload::Grant { count: 1 },
        };
        let effect = Effect {
            id: SequenceId::from_u64(2),
            community_id: event.community_id,
            event_id: event.id,
            mutations: Vec::new(),
        };
        assert_eq!(
            Into::<Record>::into(event).sequence_id(),
            event.sequence_id()
        );
        assert_eq!(
            Into::<Record>::into(effect.clone()).sequence_id(),
            effect.sequence_id()
        );
        assert_eq!(
            Into::<Record>::into(event).community_id(),
            event.community_id
        );
        assert_eq!(
            Into::<Record>::into(effect.clone()).community_id(),
            effect.community_id
        );
    }

    fn community_with_alice() -> (Community, MemberId) {
        let mut community = Community::new();
        let member = Member::new("Alice");
        let id = member.id;
        community.add_member(member);
        (community, id)
    }

    #[test]
    fn apply_adds_fruit_to_member_bag() {
        let (mut community, alice_id) = community_with_alice();
        let effect = Effect {
            id: SequenceId::from_u64(2),
            event_id: SequenceId::from_u64(1),
            community_id: community.id,
            mutations: vec![StateMutation::AddFruitToMember {
                member_id: alice_id,
                fruit: STRAWBERRY,
            }],
        };
        effect.apply(&mut community);
        assert_eq!(community.members[&alice_id].bag.count(STRAWBERRY), 1);
    }

    #[test]
    fn apply_skips_mutation_for_absent_member() {
        let (mut community, _) = community_with_alice();
        let absent_id = MemberId::new();
        let before = community.clone();
        let effect = Effect {
            id: SequenceId::from_u64(2),
            event_id: SequenceId::from_u64(1),
            community_id: community.id,
            mutations: vec![StateMutation::AddFruitToMember {
                member_id: absent_id,
                fruit: STRAWBERRY,
            }],
        };
        effect.apply(&mut community);
        assert_eq!(community, before);
    }

    #[test]
    fn apply_with_no_mutations_leaves_community_unchanged() {
        let (mut community, _) = community_with_alice();
        let before = community.clone();
        let effect = Effect {
            id: SequenceId::from_u64(2),
            event_id: SequenceId::from_u64(1),
            community_id: community.id,
            mutations: vec![],
        };
        effect.apply(&mut community);
        assert_eq!(community, before);
    }
}
