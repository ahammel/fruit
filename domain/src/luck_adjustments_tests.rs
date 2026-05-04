use super::*;
use crate::{
    bag::Bag,
    community::Community,
    event_log::{Effect, Event, EventPayload, Record, SequenceId, StateMutation},
    fruit::{Fruit, GRAPES, MANGO, MELON, TOMATO},
    member::{Member, MemberId},
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
                message: None,
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
                message: None,
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

#[test]
fn record_with_absent_effect_is_skipped() {
    let community = Community::new();
    let sender = Member::new("Alice");
    let recipient = Member::new("Bob");
    // effect: None (not even an empty vec — the event was never processed)
    let record = Record {
        event: Event {
            id: seq(1),
            community_id: community.id,
            payload: EventPayload::Gift {
                sender_id: sender.id,
                recipient_id: recipient.id,
                fruit: GRAPES,
                message: None,
            },
        },
        effect: None,
    };
    let result = compute(&community, &[record], &[]);
    assert_eq!(result, vec![]);
}

#[test]
fn burn_effect_with_non_remove_mutation_is_ignored_in_value_sum() {
    // A burn effect that contains an AddFruitToMember mutation (e.g. from a
    // hypothetical side-effect) should not be counted toward burned_value.
    let mut community = Community::new();
    let burner = Member::new("Alice");
    let other_id = MemberId::new();
    community.add_member(burner.clone());

    let record = Record {
        event: Event {
            id: seq(1),
            community_id: community.id,
            payload: EventPayload::Burn {
                member_id: burner.id,
                fruits: vec![GRAPES],
                message: None,
            },
        },
        effect: Some(Effect {
            id: seq(1),
            community_id: community.id,
            // Only a non-Remove mutation — burned_value sums to 0 → no bonus emitted
            mutations: vec![StateMutation::AddFruitToMember {
                member_id: other_id,
                fruit: GRAPES,
            }],
        }),
    };
    let result = compute(&community, &[record], &[]);
    assert_eq!(result, vec![]);
}

#[test]
fn non_gift_non_burn_record_is_skipped() {
    let community = Community::new();
    let member = Member::new("Alice");
    // A Grant record with a non-empty effect — should produce no luck mutations.
    let record = Record {
        event: Event {
            id: seq(1),
            community_id: community.id,
            payload: EventPayload::Grant { count: 1 },
        },
        effect: Some(Effect {
            id: seq(1),
            community_id: community.id,
            mutations: vec![StateMutation::AddFruitToMember {
                member_id: member.id,
                fruit: GRAPES,
            }],
        }),
    };
    let result = compute(&community, &[record], &[]);
    assert_eq!(result, vec![]);
}

#[test]
fn qp_gift_with_absent_effect_is_skipped() {
    let community = Community::new();
    let alice = Member::new("Alice");
    let bob = Member::new("Bob");

    // Both gifts have effect: None — should not count as QP
    let r_ab = Record {
        event: Event {
            id: seq(1),
            community_id: community.id,
            payload: EventPayload::Gift {
                sender_id: alice.id,
                recipient_id: bob.id,
                fruit: GRAPES,
                message: None,
            },
        },
        effect: None,
    };
    let r_ba = Record {
        event: Event {
            id: seq(2),
            community_id: community.id,
            payload: EventPayload::Gift {
                sender_id: bob.id,
                recipient_id: alice.id,
                fruit: MELON,
                message: None,
            },
        },
        effect: None,
    };
    let result = compute(&community, &[], &[r_ab, r_ba]);
    assert_eq!(result, vec![]);
}

// --- ostentation boundary: excess == 0 emits no penalty ---

#[test]
fn gift_at_exact_ostentation_boundary_emits_no_penalty() {
    // MANGO value=10.0; recipient holds 5 GRAPES (bag_value=5.0)
    // excess = 10.0 - 2.0*5.0 = 0.0 → no OstentatiousGiftPenalty
    let mut community = Community::new();
    let sender = Member::new("Alice");
    let recipient = Member::new("Bob").with_bag(
        Bag::new()
            .insert(GRAPES)
            .insert(GRAPES)
            .insert(GRAPES)
            .insert(GRAPES)
            .insert(GRAPES),
    );
    community.add_member(sender.clone());
    community.add_member(recipient.clone());

    let record = gift_record(1, &community, &sender, &recipient, MANGO, true);
    let result = compute(&community, &[record], &[]);

    assert_eq!(
        result,
        vec![StateMutation::GiftLuckBonus {
            member_id: sender.id,
            delta: 100
        }]
    );
}

#[test]
fn burn_at_exact_ostentation_boundary_emits_no_penalty() {
    // burner holds MANGO (value=10.0) in the snapshot; other has empty bag
    // avg = (10.0 + 0.0) / 2 = 5.0; excess = 10.0 - 2.0*5.0 = 0.0 → no penalty
    let mut community = Community::new();
    let burner = Member::new("Alice").with_bag(Bag::new().insert(MANGO));
    let other = Member::new("Bob");
    community.add_member(burner.clone());
    community.add_member(other.clone());

    let record = burn_record(1, &community, &burner, vec![MANGO], true);
    let result = compute(&community, &[record], &[]);

    assert_eq!(result, vec![StateMutation::BurnLuckBonus { delta: 100 }]);
}

// --- qp_penalty: division vs multiplication / modulo ---

#[test]
fn mango_tomato_reciprocal_gifts_emit_qp_penalty() {
    // MANGO value=10.0, TOMATO value=10*(1+36/255)≈11.412
    // |10.0-11.412|/11.412 ≈ 0.124 < 0.2 → QP
    // With `/ → *`: 1.412*11.412≈16.1 > 0.2 → no QP (mutation caught)
    // With `/ → %`: 1.412%11.412=1.412 > 0.2 → no QP (mutation caught)
    let community = Community::new();
    let alice = Member::new("Alice");
    let bob = Member::new("Bob");

    let r_ab = gift_record(1, &community, &alice, &bob, MANGO, true);
    let r_ba = gift_record(2, &community, &bob, &alice, TOMATO, true);
    let result = compute(&community, &[], &[r_ab, r_ba]);

    assert_eq!(
        result,
        vec![StateMutation::QuidProQuoPenalty { delta: -64 }]
    );
}

#[test]
fn partial_qp_ratio_emits_scaled_penalty() {
    // 2 bidirectional pairs: MANGO↔TOMATO (QP) + MANGO↔GRAPES (non-QP: |10-1|/10=0.9)
    // qp_count=1, total=2, ratio=0.5 → delta = -(0.5*64).round() = -32
    // With `/ → *`: ratio=1*2=2 → delta = -(2*64).round() = -128 (caught)
    let community = Community::new();
    let alice = Member::new("Alice");
    let bob = Member::new("Bob");
    let carol = Member::new("Carol");
    let dave = Member::new("Dave");

    let r_ab = gift_record(1, &community, &alice, &bob, MANGO, true);
    let r_ba = gift_record(2, &community, &bob, &alice, TOMATO, true);
    let r_cd = gift_record(3, &community, &carol, &dave, MANGO, true);
    let r_dc = gift_record(4, &community, &dave, &carol, GRAPES, true);
    let result = compute(&community, &[], &[r_ab, r_ba, r_cd, r_dc]);

    assert_eq!(
        result,
        vec![StateMutation::QuidProQuoPenalty { delta: -32 }]
    );
}
