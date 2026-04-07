use std::fmt;

use crate::{
    community::{Community, CommunityId, HasCommunityId},
    fruit::Fruit,
    id::IntegerIdentifier,
    member::{Member, MemberId},
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

/// Marker trait for types that occupy a position in the shared event/effect log sequence.
pub trait HasSequenceId {
    /// Returns the position of this entry in the shared event/effect log sequence.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventPayload {
    /// Distribute `count` fruits to every member of the community.
    Grant { count: usize },
    /// Add a new member with the given display name to the community.
    AddMember { display_name: String },
    /// Remove the member identified by `member_id` from the community.
    RemoveMember { member_id: MemberId },
    /// Set the community's luck to `luck` (raw `u16`).
    SetCommunityLuck { luck: u16 },
    /// Set the luck of the member identified by `member_id` to `luck` (raw `u16`).
    SetMemberLuck { member_id: MemberId, luck: u16 },
}

/// A recorded player intention. Events do not modify
/// [`Community`](crate::community::Community) state directly; their consequences are
/// computed as [`Effect`](crate::effect::Effect)s.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMutation {
    /// Add one instance of `fruit` to the bag of the member identified by `member_id`.
    AddFruitToMember { member_id: MemberId, fruit: Fruit },
    /// Add `member` to the community.
    AddMember { member: Member },
    /// Remove the member identified by `member_id` from the community.
    RemoveMember { member_id: MemberId },
    /// Set the community's luck to `luck` (raw `u16`).
    SetCommunityLuck { luck: u16 },
    /// Set the luck of the member identified by `member_id` to `luck` (raw `u16`).
    SetMemberLuck { member_id: MemberId, luck: u16 },
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
                StateMutation::AddMember { member } => {
                    community.add_member(member.clone());
                }
                StateMutation::RemoveMember { member_id } => {
                    community.remove_member(*member_id);
                }
                StateMutation::SetCommunityLuck { luck } => {
                    *community = community.clone().with_luck(*luck);
                }
                StateMutation::SetMemberLuck { member_id, luck } => {
                    if let Some(member) = community.members.remove(member_id) {
                        community
                            .members
                            .insert(*member_id, member.with_luck(*luck));
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
#[path = "event_log_tests.rs"]
mod tests;
