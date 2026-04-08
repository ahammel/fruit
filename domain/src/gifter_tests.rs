use crate::{
    community::Community, event_log::StateMutation, fruit::GRAPES, gifter::compute_gift,
    member::Member,
};

fn community_with_members() -> (Community, crate::member::MemberId, crate::member::MemberId) {
    let mut community = Community::new();
    let alice = Member::new("Alice");
    let bob = Member::new("Bob");
    let alice_id = alice.id;
    let bob_id = bob.id;
    community.add_member(alice);
    community.add_member(bob);
    (community, alice_id, bob_id)
}

#[test]
fn valid_gift_returns_remove_then_add() {
    let (mut community, alice_id, bob_id) = community_with_members();
    community
        .members
        .get_mut(&alice_id)
        .unwrap()
        .receive(GRAPES);

    let mutations = compute_gift(&community, alice_id, bob_id, GRAPES);

    assert_eq!(
        mutations,
        vec![
            StateMutation::RemoveFruitFromMember {
                member_id: alice_id,
                fruit: GRAPES,
            },
            StateMutation::AddFruitToMember {
                member_id: bob_id,
                fruit: GRAPES,
            },
        ]
    );
}

#[test]
fn sender_does_not_hold_fruit_returns_noop() {
    let (community, alice_id, bob_id) = community_with_members();
    // Alice has no fruits

    let mutations = compute_gift(&community, alice_id, bob_id, GRAPES);

    assert_eq!(mutations, vec![]);
}

#[test]
fn unknown_sender_returns_noop() {
    let (community, _, bob_id) = community_with_members();
    let unknown_id = Member::new("Ghost").id;

    let mutations = compute_gift(&community, unknown_id, bob_id, GRAPES);

    assert_eq!(mutations, vec![]);
}
