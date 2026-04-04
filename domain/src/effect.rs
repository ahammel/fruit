use crate::{
    community::{Community, CommunityId},
    event::SequenceId,
    fruit::Fruit,
    member::MemberId,
};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fruit::STRAWBERRY,
        id::{IntegerIdentifier, UuidIdentifier},
        member::Member,
    };

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
