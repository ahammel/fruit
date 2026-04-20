use super::*;
use crate::bag::Bag;
use crate::fruit::{GRAPES, PEAR};

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
    let v = SequenceId::new(42);
    assert_eq!(Community::new().with_version(v).version, v);
}

#[test]
fn community_avg_bag_value_returns_zero_for_empty_community() {
    assert_eq!(community_avg_bag_value(&Community::new()), 0.0);
}

#[test]
fn community_avg_bag_value_single_member() {
    // GRAPES: value 1.0; two GRAPES → bag_value = 2.0
    let mut community = Community::new();
    community.add_member(Member::new("Alice").with_bag(Bag::new().insert(GRAPES).insert(GRAPES)));
    assert_eq!(community_avg_bag_value(&community), 2.0);
}

#[test]
fn community_avg_bag_value_multiple_members() {
    // Alice: GRAPES ×2 = 2.0; Bob: PEAR ×2 = 6.0; mean = 4.0
    let mut community = Community::new();
    community.add_member(Member::new("Alice").with_bag(Bag::new().insert(GRAPES).insert(GRAPES)));
    community.add_member(Member::new("Bob").with_bag(Bag::new().insert(PEAR).insert(PEAR)));
    assert_eq!(community_avg_bag_value(&community), 4.0);
}

#[test]
fn apply_effects_applies_mutations_and_advances_version() {
    use crate::{
        event_log::{Effect, StateMutation},
        fruit::STRAWBERRY,
    };

    let mut community = Community::new();
    let member = Member::new("Alice");
    let alice_id = member.id;
    community.add_member(member);

    let v1 = SequenceId::new(1);
    let v2 = SequenceId::new(2);
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
