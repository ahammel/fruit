use super::*;

use crate::{
    community::{Community, CommunityId},
    fruit::STRAWBERRY,
    id::{IntegerIdentifier, UuidIdentifier},
    member::{Member, MemberId},
};

#[test]
fn sequence_id_as_u64_returns_inner_value() {
    assert_eq!(SequenceId::from_u64(42).as_u64(), 42);
}

#[test]
fn sequence_id_display_formats_correctly() {
    assert_eq!(format!("{}", SequenceId::from_u64(5)), "SequenceId(5)");
}

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
        Record::from(event.clone()).sequence_id(),
        event.sequence_id()
    );
    assert_eq!(
        Record::from(effect.clone()).sequence_id(),
        effect.sequence_id()
    );
    assert_eq!(
        Record::from(event.clone()).community_id(),
        event.community_id
    );
    assert_eq!(
        Record::from(effect.clone()).community_id(),
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

#[test]
fn apply_add_member_inserts_member() {
    let mut community = Community::new();
    let member = Member::new("Bob");
    let bob_id = member.id;
    let effect = Effect {
        id: SequenceId::from_u64(1),
        event_id: SequenceId::from_u64(0),
        community_id: community.id,
        mutations: vec![StateMutation::AddMember {
            member: member.clone(),
        }],
    };
    effect.apply(&mut community);
    assert_eq!(community.members[&bob_id], member);
}

#[test]
fn apply_remove_member_removes_member() {
    let (mut community, alice_id) = community_with_alice();
    let effect = Effect {
        id: SequenceId::from_u64(2),
        event_id: SequenceId::from_u64(1),
        community_id: community.id,
        mutations: vec![StateMutation::RemoveMember {
            member_id: alice_id,
        }],
    };
    effect.apply(&mut community);
    assert_eq!(community, Community::new().with_id(community.id));
}

#[test]
fn apply_remove_fruit_from_member_removes_one() {
    let (mut community, alice_id) = community_with_alice();
    community
        .members
        .get_mut(&alice_id)
        .unwrap()
        .receive(STRAWBERRY);
    let effect = Effect {
        id: SequenceId::from_u64(2),
        event_id: SequenceId::from_u64(1),
        community_id: community.id,
        mutations: vec![StateMutation::RemoveFruitFromMember {
            member_id: alice_id,
            fruit: STRAWBERRY,
        }],
    };
    effect.apply(&mut community);
    assert_eq!(community.members[&alice_id].bag.count(STRAWBERRY), 0);
}

#[test]
fn apply_remove_fruit_from_member_skips_absent_member() {
    let (mut community, _) = community_with_alice();
    let absent_id = MemberId::new();
    let before = community.clone();
    let effect = Effect {
        id: SequenceId::from_u64(2),
        event_id: SequenceId::from_u64(1),
        community_id: community.id,
        mutations: vec![StateMutation::RemoveFruitFromMember {
            member_id: absent_id,
            fruit: STRAWBERRY,
        }],
    };
    effect.apply(&mut community);
    assert_eq!(community, before);
}

#[test]
fn apply_set_community_luck_updates_luck() {
    let (mut community, _) = community_with_alice();
    let effect = Effect {
        id: SequenceId::from_u64(2),
        event_id: SequenceId::from_u64(1),
        community_id: community.id,
        mutations: vec![StateMutation::SetCommunityLuck { luck: 1000 }],
    };
    effect.apply(&mut community);
    assert_eq!(community.luck(), 1000.0 / u16::MAX as f64);
}

#[test]
fn apply_set_member_luck_updates_member_luck() {
    let (mut community, alice_id) = community_with_alice();
    let effect = Effect {
        id: SequenceId::from_u64(2),
        event_id: SequenceId::from_u64(1),
        community_id: community.id,
        mutations: vec![StateMutation::SetMemberLuck {
            member_id: alice_id,
            luck: 2000,
        }],
    };
    effect.apply(&mut community);
    assert_eq!(
        community.members[&alice_id].luck(),
        2000.0 / u16::MAX as f64
    );
}

#[test]
fn apply_set_member_luck_skips_absent_member() {
    let (mut community, _) = community_with_alice();
    let absent_id = MemberId::new();
    let before = community.clone();
    let effect = Effect {
        id: SequenceId::from_u64(2),
        event_id: SequenceId::from_u64(1),
        community_id: community.id,
        mutations: vec![StateMutation::SetMemberLuck {
            member_id: absent_id,
            luck: 500,
        }],
    };
    effect.apply(&mut community);
    assert_eq!(community, before);
}
