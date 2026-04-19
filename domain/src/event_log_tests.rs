use super::*;

use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;

use crate::{
    community::{Community, CommunityId},
    fruit::STRAWBERRY,
    member::{Member, MemberId},
};

#[test]
fn sequence_id_as_u64_returns_inner_value() {
    assert_eq!(SequenceId::new(42).as_u64(), 42);
}

#[test]
fn sequence_id_display_formats_correctly() {
    assert_eq!(format!("{}", SequenceId::new(5)), "SequenceId(5)");
}

#[test]
fn event_record_getters_delegate_to_event_fields() {
    let event = Event {
        id: SequenceId::new(1),
        community_id: CommunityId::new(),
        payload: EventPayload::Grant { count: 1 },
    };
    let effect = Effect {
        id: SequenceId::new(1),
        community_id: event.community_id,
        mutations: Vec::new(),
    };
    let record = Record {
        event: event.clone(),
        effect: Some(effect),
    };
    assert_eq!(record.sequence_id(), event.id);
    assert_eq!(record.community_id(), event.community_id);
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
        id: SequenceId::new(2),

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
        id: SequenceId::new(2),

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
        id: SequenceId::new(2),

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
        id: SequenceId::new(1),

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
        id: SequenceId::new(2),

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
        id: SequenceId::new(2),

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
        id: SequenceId::new(2),

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
        id: SequenceId::new(2),

        community_id: community.id,
        mutations: vec![StateMutation::SetCommunityLuck { luck: 100 }],
    };
    effect.apply(&mut community);
    assert_eq!(community.luck(), 100.0 / u8::MAX as f64);
}

#[test]
fn apply_set_member_luck_updates_member_luck() {
    let (mut community, alice_id) = community_with_alice();
    let effect = Effect {
        id: SequenceId::new(2),

        community_id: community.id,
        mutations: vec![StateMutation::SetMemberLuck {
            member_id: alice_id,
            luck: 200,
        }],
    };
    effect.apply(&mut community);
    assert_eq!(community.members[&alice_id].luck(), 200.0 / u8::MAX as f64);
}

#[test]
fn apply_set_member_luck_skips_absent_member() {
    let (mut community, _) = community_with_alice();
    let absent_id = MemberId::new();
    let before = community.clone();
    let effect = Effect {
        id: SequenceId::new(2),

        community_id: community.id,
        mutations: vec![StateMutation::SetMemberLuck {
            member_id: absent_id,
            luck: 50,
        }],
    };
    effect.apply(&mut community);
    assert_eq!(community, before);
}
