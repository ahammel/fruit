use std::collections::HashMap;

use crate::{
    burner::compute_burn,
    community::Community,
    event_log::StateMutation,
    fruit::{Fruit, GRAPES, STRAWBERRY},
    member::Member,
};

fn community_with_member() -> (Community, crate::member::MemberId) {
    let mut community = Community::new();
    let alice = Member::new("Alice");
    let id = alice.id;
    community.add_member(alice);
    (community, id)
}

/// Partition a mutation list into removes keyed by fruit.
fn collect_removes(mutations: Vec<StateMutation>) -> HashMap<Fruit, usize> {
    let mut removes: HashMap<Fruit, usize> = HashMap::new();
    for m in mutations {
        match m {
            StateMutation::RemoveFruitFromMember { fruit, .. } => {
                *removes.entry(fruit).or_insert(0) += 1;
            }
            _ => panic!("unexpected mutation"),
        }
    }
    removes
}

#[test]
fn burn_single_fruit_returns_remove() {
    let (mut community, member_id) = community_with_member();
    community
        .members
        .get_mut(&member_id)
        .unwrap()
        .receive(GRAPES);

    let mutations = compute_burn(&community, member_id, &[GRAPES]);

    assert_eq!(
        mutations,
        vec![StateMutation::RemoveFruitFromMember {
            member_id,
            fruit: GRAPES,
        }]
    );
}

#[test]
fn burn_multiple_of_same_type() {
    let (mut community, member_id) = community_with_member();
    let member = community.members.get_mut(&member_id).unwrap();
    member.receive(GRAPES);
    member.receive(GRAPES);
    member.receive(GRAPES);

    let mutations = compute_burn(&community, member_id, &[GRAPES, GRAPES, GRAPES]);

    assert_eq!(collect_removes(mutations), HashMap::from([(GRAPES, 3)]));
}

#[test]
fn burn_mixed_fruit_types() {
    let (mut community, member_id) = community_with_member();
    let member = community.members.get_mut(&member_id).unwrap();
    member.receive(GRAPES);
    member.receive(STRAWBERRY);

    let mutations = compute_burn(&community, member_id, &[GRAPES, STRAWBERRY]);

    assert_eq!(
        collect_removes(mutations),
        HashMap::from([(GRAPES, 1), (STRAWBERRY, 1)])
    );
}

#[test]
fn insufficient_of_one_type_burns_what_is_held() {
    let (mut community, member_id) = community_with_member();
    let member = community.members.get_mut(&member_id).unwrap();
    // Hold 1 GRAPES, request 3; hold 2 STRAWBERRY, request 2.
    member.receive(GRAPES);
    member.receive(STRAWBERRY);
    member.receive(STRAWBERRY);

    let mutations = compute_burn(
        &community,
        member_id,
        &[GRAPES, GRAPES, GRAPES, STRAWBERRY, STRAWBERRY],
    );

    assert_eq!(
        collect_removes(mutations),
        HashMap::from([(GRAPES, 1), (STRAWBERRY, 2)])
    );
}

#[test]
fn empty_fruits_returns_noop() {
    let (community, member_id) = community_with_member();

    assert_eq!(compute_burn(&community, member_id, &[]), vec![]);
}

#[test]
fn member_holds_none_of_requested_returns_noop() {
    let (community, member_id) = community_with_member();

    assert_eq!(compute_burn(&community, member_id, &[GRAPES]), vec![]);
}

#[test]
fn unknown_member_returns_noop() {
    let (community, _) = community_with_member();
    let unknown_id = Member::new("Ghost").id;

    assert_eq!(compute_burn(&community, unknown_id, &[GRAPES]), vec![]);
}
