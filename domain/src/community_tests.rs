use super::*;
use crate::id::UuidIdentifier;

#[test]
fn community_id_returns_id() {
    let community = Community::new();
    assert_eq!(community.community_id(), community.id);
}

#[test]
fn new_community_is_empty() {
    let community = Community::new();
    assert_eq!(community, Community::new().with_id(community.id));
}

#[test]
fn add_member_inserts_by_id() {
    let mut community = Community::new();
    let member = Member::new("Alice");
    let id = member.id;
    assert!(community.add_member(member));
    let mut expected = Community::new().with_id(community.id);
    expected.members = HashMap::from([(id, Member::new("Alice").with_id(id))]);
    assert_eq!(community, expected);
}

#[test]
fn add_member_returns_false_on_duplicate() {
    let mut community = Community::new();
    let member = Member::new("Alice");
    let id = member.id;
    let duplicate = Member::new("Alice2").with_id(id);
    assert!(community.add_member(member));
    assert!(!community.add_member(duplicate));
    let mut expected = Community::new().with_id(community.id);
    expected.members = HashMap::from([(id, Member::new("Alice").with_id(id))]);
    assert_eq!(community, expected);
}

#[test]
fn remove_member_returns_member() {
    let mut community = Community::new();
    let member = Member::new("Bob");
    let id = member.id;
    community.add_member(member);
    let removed = community.remove_member(id).unwrap();
    assert_eq!(removed, Member::new("Bob").with_id(id));
    assert_eq!(community, Community::new().with_id(community.id));
}

#[test]
fn remove_member_returns_none_for_unknown_id() {
    let mut community = Community::new();
    assert!(community.remove_member(MemberId::new()).is_none());
}

#[test]
fn community_id_as_uuid_roundtrips() {
    let id = CommunityId::new();
    assert_eq!(id, CommunityId(id.as_uuid()));
}

#[test]
fn default_equals_new() {
    let community = Community::default();
    assert_eq!(community, Community::new().with_id(community.id));
}

#[test]
fn with_luck_sets_luck() {
    let community = Community::new().with_luck(50);
    assert_eq!(community.luck(), 50.0 / u8::MAX as f64);
}

#[test]
fn with_luck_f64_sets_luck() {
    let community = Community::new().with_luck_f64(0.5);
    assert!((community.luck() - 0.5).abs() < 2e-3);
}

#[test]
fn with_version_sets_version() {
    use crate::id::IntegerIdentifier;
    let v = SequenceId::from_u64(42);
    assert_eq!(Community::new().with_version(v).version, v);
}

#[test]
fn apply_effects_applies_mutations_and_advances_version() {
    use crate::{
        event_log::{Effect, StateMutation},
        fruit::STRAWBERRY,
        id::IntegerIdentifier,
    };

    let mut community = Community::new();
    let member = Member::new("Alice");
    let alice_id = member.id;
    community.add_member(member);

    let v1 = SequenceId::from_u64(1);
    let v2 = SequenceId::from_u64(2);
    let effects = vec![
        Effect {
            id: v1,
            community_id: community.id,
            mutations: vec![StateMutation::AddFruitToMember {
                member_id: alice_id,
                fruit: STRAWBERRY,
            }],
        },
        Effect {
            id: v2,
            community_id: community.id,
            mutations: vec![StateMutation::AddFruitToMember {
                member_id: alice_id,
                fruit: STRAWBERRY,
            }],
        },
    ];

    community.apply_effects(effects);

    assert_eq!(community.version, v2);
    assert_eq!(community.members[&alice_id].bag.count(STRAWBERRY), 2);
}
