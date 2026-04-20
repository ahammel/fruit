use super::*;
use crate::{
    bag::Bag,
    community::Community,
    event_log::{Effect, Event, EventPayload, Record, SequenceId, StateMutation},
    fruit::{Fruit, GRAPES, MANGO, MELON},
    member::Member,
};

fn seq(n: u64) -> SequenceId {
    SequenceId::new(n)
}

fn gift_record(
    id: u64,
    community: &Community,
    sender: &Member,
    recipient: &Member,
    fruit: Fruit,
    nonempty: bool,
) -> Record {
    let mutations = if nonempty {
        vec![
            StateMutation::RemoveFruitFromMember {
                member_id: sender.id,
                fruit,
            },
            StateMutation::AddFruitToMember {
                member_id: recipient.id,
                fruit,
            },
        ]
    } else {
        vec![]
    };
    Record {
        event: Event {
            id: seq(id),
            community_id: community.id,
            payload: EventPayload::Gift {
                sender_id: sender.id,
                recipient_id: recipient.id,
                fruit,
            },
        },
        effect: Some(Effect {
            id: seq(id),
            community_id: community.id,
            mutations,
        }),
    }
}

fn burn_record(
    id: u64,
    community: &Community,
    member: &Member,
    fruits: Vec<Fruit>,
    nonempty: bool,
) -> Record {
    let mutations: Vec<StateMutation> = if nonempty {
        fruits
            .iter()
            .map(|&f| StateMutation::RemoveFruitFromMember {
                member_id: member.id,
                fruit: f,
            })
            .collect()
    } else {
        vec![]
    };
    let payload_fruits = fruits;
    Record {
        event: Event {
            id: seq(id),
            community_id: community.id,
            payload: EventPayload::Burn {
                member_id: member.id,
                fruits: payload_fruits,
            },
        },
        effect: Some(Effect {
            id: seq(id),
            community_id: community.id,
            mutations,
        }),
    }
}

#[test]
fn no_records_returns_empty() {
    let community = Community::new();
    let result = compute(&community, &[], &[]);
    assert_eq!(result, vec![]);
}

#[test]
fn noop_gift_record_returns_empty() {
    let mut community = Community::new();
    let sender = Member::new("Alice");
    let recipient = Member::new("Bob");
    community.add_member(sender.clone());
    community.add_member(recipient.clone());

    let record = gift_record(1, &community, &sender, &recipient, GRAPES, false);
    let result = compute(&community, &[record], &[]);
    assert_eq!(result, vec![]);
}

#[test]
fn single_gift_below_threshold_emits_only_bonus() {
    // recipient has a large bag so the gift is well below the ostentation threshold
    let mut community = Community::new();
    let sender = Member::new("Alice");
    let big_bag = Bag::new()
        .insert(MANGO)
        .insert(MANGO)
        .insert(MANGO)
        .insert(MANGO)
        .insert(MANGO);
    let recipient = Member::new("Bob").with_bag(big_bag);
    community.add_member(sender.clone());
    community.add_member(recipient.clone());

    let record = gift_record(1, &community, &sender, &recipient, GRAPES, true);
    let result = compute(&community, &[record], &[]);

    // GRAPES has _rarity=0, value = 1.0*(1+0) = 1.0
    // delta = round(1.0 * 10.0) = 10
    assert_eq!(
        result,
        vec![StateMutation::GiftLuckBonus {
            member_id: sender.id,
            delta: 10
        }]
    );
}

#[test]
fn single_gift_above_threshold_emits_bonus_and_penalty() {
    // recipient has an empty bag so MANGO is very ostentatious
    let mut community = Community::new();
    let sender = Member::new("Alice");
    let recipient = Member::new("Bob");
    community.add_member(sender.clone());
    community.add_member(recipient.clone());

    // MANGO: category=Exotic, _rarity=0, value = 10.0*(1+0) = 10.0
    let record = gift_record(1, &community, &sender, &recipient, MANGO, true);
    let result = compute(&community, &[record], &[]);

    // gift_value = 10.0; recipient_bag_val = 0.0; excess = 10.0 - 2.0*0.0 = 10.0
    // gift_delta = round(10.0 * 10.0) = 100
    // penalty_delta = -round(10.0 * 5.0) = -50
    assert_eq!(
        result,
        vec![
            StateMutation::GiftLuckBonus {
                member_id: sender.id,
                delta: 100
            },
            StateMutation::OstentatiousGiftPenalty {
                member_id: sender.id,
                delta: -50
            },
        ]
    );
}

#[test]
fn multiple_gifts_same_sender_emits_one_combined_bonus() {
    let mut community = Community::new();
    let sender = Member::new("Alice");
    // recipient holds 2 GRAPES (value=2.0) so GRAPES gift (1.0) is below ostentation
    // threshold of 2 * 2.0 = 4.0 and no penalty is emitted
    let recipient = Member::new("Bob").with_bag(Bag::new().insert(GRAPES).insert(GRAPES));
    community.add_member(sender.clone());
    community.add_member(recipient.clone());

    // Two GRAPES gifts: each value = 1.0, combined = 2.0
    // delta = round(2.0 * 10.0) = 20
    let r1 = gift_record(1, &community, &sender, &recipient, GRAPES, true);
    let r2 = gift_record(2, &community, &sender, &recipient, GRAPES, true);
    let result = compute(&community, &[r1, r2], &[]);

    assert_eq!(
        result,
        vec![StateMutation::GiftLuckBonus {
            member_id: sender.id,
            delta: 20
        }]
    );
}

#[test]
fn single_burn_below_threshold_emits_only_bonus() {
    // community avg is high so the burn is not ostentatious
    let mut community = Community::new();
    let big_bag = Bag::new()
        .insert(MANGO)
        .insert(MANGO)
        .insert(MANGO)
        .insert(MANGO);
    let burner = Member::new("Alice").with_bag(big_bag.clone().insert(GRAPES));
    let other = Member::new("Bob").with_bag(big_bag);
    community.add_member(burner.clone());
    community.add_member(other.clone());

    let record = burn_record(1, &community, &burner, vec![GRAPES], true);
    let result = compute(&community, &[record], &[]);

    // burned_value = GRAPES.value() = 1.0
    // delta = round(1.0 * 10.0) = 10
    // avg_bag = large, so no ostentation penalty
    assert_eq!(result, vec![StateMutation::BurnLuckBonus { delta: 10 }]);
}

#[test]
fn single_burn_above_threshold_emits_bonus_and_penalty() {
    let mut community = Community::new();
    let burner = Member::new("Alice");
    community.add_member(burner.clone());

    // MANGO value=10.0; avg=0.0 (burner has empty bag before the burn)
    // excess = 10.0 - 2.0*0.0 = 10.0; penalty = -round(10.0 * 5.0) = -50
    let record = burn_record(1, &community, &burner, vec![MANGO], true);
    let result = compute(&community, &[record], &[]);

    assert_eq!(
        result,
        vec![
            StateMutation::BurnLuckBonus { delta: 100 },
            StateMutation::OstentatiousBurnPenalty {
                member_id: burner.id,
                delta: -50
            },
        ]
    );
}

#[test]
fn noop_burn_record_returns_empty() {
    let mut community = Community::new();
    let burner = Member::new("Alice");
    community.add_member(burner.clone());

    let record = burn_record(1, &community, &burner, vec![MANGO], false);
    let result = compute(&community, &[record], &[]);
    assert_eq!(result, vec![]);
}

#[test]
fn reciprocal_gifts_of_similar_value_emits_qp_penalty() {
    let community = Community::new();
    let alice = Member::new("Alice");
    let bob = Member::new("Bob");

    // GRAPES value=1.0, MELON value=1.0*(1+32/255)≈1.125
    // |1.0-1.125|/1.125 ≈ 0.111 < 0.2 ✓  and 1.0 ≠ 1.125 ✓
    let r_ab = gift_record(1, &community, &alice, &bob, GRAPES, true);
    let r_ba = gift_record(2, &community, &bob, &alice, MELON, true);
    let result = compute(&community, &[], &[r_ab, r_ba]);

    // ratio=1/1=1.0; delta = -(1.0 * 64.0).round() = -64
    assert_eq!(
        result,
        vec![StateMutation::QuidProQuoPenalty { delta: -64 }]
    );
}

#[test]
fn no_reciprocal_gifts_no_qp_penalty() {
    let community = Community::new();
    let alice = Member::new("Alice");
    let bob = Member::new("Bob");
    let carol = Member::new("Carol");

    let r1 = gift_record(1, &community, &alice, &bob, GRAPES, true);
    let r2 = gift_record(2, &community, &carol, &bob, GRAPES, true);
    let result = compute(&community, &[], &[r1, r2]);
    assert_eq!(result, vec![]);
}

#[test]
fn equal_value_reciprocal_gifts_not_counted_as_qp() {
    let community = Community::new();
    let alice = Member::new("Alice");
    let bob = Member::new("Bob");

    // Both gift GRAPES (same value) — va == vb so the `va != vb` guard excludes them
    let r_ab = gift_record(1, &community, &alice, &bob, GRAPES, true);
    let r_ba = gift_record(2, &community, &bob, &alice, GRAPES, true);
    let result = compute(&community, &[], &[r_ab, r_ba]);
    assert_eq!(result, vec![]);
}
