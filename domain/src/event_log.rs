use std::fmt;

use newtype_ids::IntegerIdentifier;

use crate::{
    community::{Community, CommunityId, HasCommunityId},
    fruit::Fruit,
    member::{Member, MemberId},
};

/// A position in the shared event/effect log sequence.
///
/// Sequence IDs start at 1 and increase monotonically. Both events and effects
/// draw from the same counter, so their IDs are globally ordered.
#[derive(IntegerIdentifier)]
#[allowed_values(all)]
pub struct SequenceId(u64);

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

/// A log entry: an [`Event`] paired with its computed [`Effect`], or `None` if the
/// event has not yet been processed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The recorded player intention.
    pub event: Event,
    /// The computed consequence, or `None` if not yet processed.
    pub effect: Option<Effect>,
}

impl HasSequenceId for Record {
    fn sequence_id(&self) -> SequenceId {
        self.event.id
    }
}

impl HasCommunityId for Record {
    fn community_id(&self) -> CommunityId {
        self.event.community_id
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
    /// Set the community's luck to `luck` (raw `u8`).
    SetCommunityLuck { luck: u8 },
    /// Set the luck of the member identified by `member_id` to `luck` (raw `u8`).
    SetMemberLuck { member_id: MemberId, luck: u8 },
    /// Transfer one instance of `fruit` from the member `sender_id` to `recipient_id`.
    Gift {
        /// The member giving the fruit.
        sender_id: MemberId,
        /// The member receiving the fruit.
        recipient_id: MemberId,
        /// The fruit being transferred.
        fruit: Fruit,
        /// An optional message from the sender to the recipient.
        message: Option<String>,
    },
    /// Destroy one or more fruits held by `member_id`, granting a community luck bonus.
    ///
    /// `fruits` may contain duplicates and may span multiple fruit types.
    /// If the member does not hold enough of a particular fruit, as many as they
    /// hold are burned and the remainder of that type is silently skipped.
    Burn {
        /// The member burning the fruits.
        member_id: MemberId,
        /// The fruits requested to be burned (duplicates allowed).
        fruits: Vec<Fruit>,
    },
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

/// An atomic change to [`Community`] state produced as part of an [`Effect`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMutation {
    /// Add one instance of `fruit` to the bag of the member identified by `member_id`.
    AddFruitToMember { member_id: MemberId, fruit: Fruit },
    /// Remove one instance of `fruit` from the bag of the member identified by `member_id`.
    ///
    /// Has no effect if the member does not hold the fruit.
    RemoveFruitFromMember { member_id: MemberId, fruit: Fruit },
    /// Add `member` to the community.
    AddMember { member: Member },
    /// Remove the member identified by `member_id` from the community.
    RemoveMember { member_id: MemberId },
    /// Set the community's luck to `luck` (raw `u8`).
    SetCommunityLuck { luck: u8 },
    /// Set the luck of the member identified by `member_id` to `luck` (raw `u8`).
    SetMemberLuck { member_id: MemberId, luck: u8 },
    /// Adjust the luck of `member_id` by `delta`, clamped to `[0, 255]`.
    ///
    /// Emitted when a member gifts fruit; `delta` is positive.
    GiftLuckBonus { member_id: MemberId, delta: i16 },
    /// Adjust community luck by `delta`, clamped to `[0, 255]`.
    ///
    /// Emitted when a member burns fruit; `delta` is positive.
    BurnLuckBonus { delta: i16 },
    /// Adjust the luck of `member_id` by `delta`, clamped to `[0, 255]`.
    ///
    /// Emitted when a gift is deemed ostentatious; `delta` is negative.
    OstentatiousGiftPenalty { member_id: MemberId, delta: i16 },
    /// Adjust the luck of `member_id` by `delta`, clamped to `[0, 255]`.
    ///
    /// Emitted when a burn is deemed ostentatious; `delta` is negative.
    OstentatiousBurnPenalty { member_id: MemberId, delta: i16 },
    /// Adjust community luck by `delta`, clamped to `[0, 255]`.
    ///
    /// Emitted when quid-pro-quo gifting is detected; `delta` is negative.
    QuidProQuoPenalty { delta: i16 },
}

/// The computed consequence of an [`Event`](crate::event::Event). An effect may contain
/// zero mutations (a no-op, e.g. when the event violated an invariant) or many.
///
/// An `Effect` carries the same [`SequenceId`] as its originating `Event`. Use the shared
/// ID to correlate the two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    /// Position in the shared event/effect sequence. Equals the originating `Event`'s ID.
    pub id: SequenceId,
    /// The community this effect applies to.
    pub community_id: CommunityId,
    /// The state changes produced by this effect. Empty if the event was a no-op.
    pub mutations: Vec<StateMutation>,
}

fn apply_luck_delta(raw: u8, delta: i16) -> u8 {
    (raw as i16 + delta).clamp(0, 255) as u8
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
                StateMutation::RemoveFruitFromMember { member_id, fruit } => {
                    if let Some(member) = community.members.get_mut(member_id) {
                        member.bag.remove(*fruit);
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
                StateMutation::GiftLuckBonus { member_id, delta }
                | StateMutation::OstentatiousGiftPenalty { member_id, delta }
                | StateMutation::OstentatiousBurnPenalty { member_id, delta } => {
                    if let Some(member) = community.members.remove(member_id) {
                        let new_luck = apply_luck_delta(member.luck_raw(), *delta);
                        community
                            .members
                            .insert(*member_id, member.with_luck(new_luck));
                    }
                }
                StateMutation::BurnLuckBonus { delta }
                | StateMutation::QuidProQuoPenalty { delta } => {
                    let new_luck = apply_luck_delta(community.luck_raw(), *delta);
                    *community = community.clone().with_luck(new_luck);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "event_log_tests.rs"]
mod tests;
