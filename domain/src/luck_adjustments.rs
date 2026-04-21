use std::collections::HashMap;

use newtype_ids_uuid::UuidIdentifier as _;

use crate::{
    bag::bag_value,
    community::{community_avg_bag_value, Community},
    event_log::{EventPayload, Record, StateMutation},
    member::MemberId,
};

const GIFT_LUCK_SCALE: f64 = 10.0;
const BURN_LUCK_SCALE: f64 = 10.0;
const OSTENTATION_RATIO: f64 = 2.0;
const OSTENTATION_SCALE: f64 = 5.0;
const QP_SIMILARITY_THRESHOLD: f64 = 0.2;
const QP_MAX_PENALTY: f64 = 64.0;

/// Computes luck adjustment mutations based on player actions since the last grant.
///
/// `community_at_last_grant` is the community snapshot immediately after the previous
/// grant. `records_since_last_grant` contains all records in ascending sequence-ID order
/// between the previous grant and the current one. `recent_gift_records` contains up to
/// 100 of the most recent gift records for quid-pro-quo detection.
///
/// Mutations are emitted in the order: gift bonuses → burn bonus → ostentatious gift
/// penalties → ostentatious burn penalties → quid-pro-quo penalty.
///
/// Records with absent or empty effects are skipped entirely (they are no-ops, e.g. a
/// gift where the sender did not hold the fruit).
pub fn compute(
    community_at_last_grant: &Community,
    records_since_last_grant: &[Record],
    recent_gift_records: &[Record],
) -> Vec<StateMutation> {
    let mut running = community_at_last_grant.clone();

    let mut gift_bonus_by_sender: HashMap<MemberId, f64> = HashMap::new();
    let mut burn_bonus_total: f64 = 0.0;
    let mut gift_penalty_records: Vec<(MemberId, i16)> = Vec::new();
    let mut burn_penalty_records: Vec<(MemberId, i16)> = Vec::new();

    for record in records_since_last_grant {
        let Some(effect) = record.effect.as_ref() else {
            continue;
        };
        if effect.mutations.is_empty() {
            continue;
        }

        match &record.event.payload {
            EventPayload::Gift {
                sender_id,
                recipient_id,
                fruit,
            } => {
                let gift_value = fruit.value();
                *gift_bonus_by_sender.entry(*sender_id).or_insert(0.0) += gift_value;

                let recipient_bag_val = running
                    .members
                    .get(recipient_id)
                    .map(|m| bag_value(&m.bag))
                    .unwrap_or(0.0);
                let excess = gift_value - OSTENTATION_RATIO * recipient_bag_val;
                let penalty_delta = -f64_to_i16(excess.max(0.0) * OSTENTATION_SCALE);
                if penalty_delta != 0 {
                    gift_penalty_records.push((*sender_id, penalty_delta));
                }
            }
            EventPayload::Burn { member_id, .. } => {
                let burned_value: f64 = effect
                    .mutations
                    .iter()
                    .filter_map(|m| match m {
                        StateMutation::RemoveFruitFromMember { fruit, .. } => Some(fruit.value()),
                        _ => None,
                    })
                    .sum();

                burn_bonus_total += burned_value;

                let avg = community_avg_bag_value(&running);
                let excess = burned_value - OSTENTATION_RATIO * avg;
                let penalty_delta = -f64_to_i16(excess.max(0.0) * OSTENTATION_SCALE);
                if penalty_delta != 0 {
                    burn_penalty_records.push((*member_id, penalty_delta));
                }
            }
            _ => {}
        }

        effect.apply(&mut running);
    }

    let mut mutations: Vec<StateMutation> = Vec::new();

    for (member_id, total_value) in gift_bonus_by_sender {
        let delta = f64_to_i16(total_value * GIFT_LUCK_SCALE);
        mutations.push(StateMutation::GiftLuckBonus { member_id, delta });
    }

    let burn_delta = f64_to_i16(burn_bonus_total * BURN_LUCK_SCALE);
    if burn_delta > 0 {
        mutations.push(StateMutation::BurnLuckBonus { delta: burn_delta });
    }

    for (member_id, delta) in gift_penalty_records {
        mutations.push(StateMutation::OstentatiousGiftPenalty { member_id, delta });
    }

    for (member_id, delta) in burn_penalty_records {
        mutations.push(StateMutation::OstentatiousBurnPenalty { member_id, delta });
    }

    if let Some(qp) = qp_penalty(recent_gift_records) {
        mutations.push(qp);
    }

    mutations
}

fn f64_to_i16(value: f64) -> i16 {
    value.round().clamp(i16::MIN as f64, i16::MAX as f64) as i16
}

fn qp_penalty(recent_gift_records: &[Record]) -> Option<StateMutation> {
    let mut directed_gifts: HashMap<(MemberId, MemberId), Vec<f64>> = HashMap::new();

    for record in recent_gift_records {
        let Some(effect) = record.effect.as_ref() else {
            continue;
        };
        if effect.mutations.is_empty() {
            continue;
        }
        if let EventPayload::Gift {
            sender_id,
            recipient_id,
            fruit,
        } = record.event.payload
        {
            directed_gifts
                .entry((sender_id, recipient_id))
                .or_default()
                .push(fruit.value());
        }
    }

    let mut seen: std::collections::HashSet<[u128; 2]> = Default::default();
    let mut total_bidirectional: usize = 0;
    let mut qp_count: usize = 0;

    for &(sender, recipient) in directed_gifts.keys() {
        let a = sender.as_uuid().as_u128();
        let b = recipient.as_uuid().as_u128();
        let key = if a < b { [a, b] } else { [b, a] };
        if !seen.insert(key) {
            continue;
        }

        let ab_vals = &directed_gifts[&(sender, recipient)];
        let Some(ba_vals) = directed_gifts.get(&(recipient, sender)) else {
            continue;
        };

        total_bidirectional += 1;
        let is_qp = ab_vals.iter().any(|&va| {
            ba_vals
                .iter()
                .any(|&vb| va != vb && (va - vb).abs() / va.max(vb) < QP_SIMILARITY_THRESHOLD)
        });
        if is_qp {
            qp_count += 1;
        }
    }

    if qp_count == 0 {
        return None;
    }

    let ratio = qp_count as f64 / total_bidirectional as f64;
    let delta = -(ratio * QP_MAX_PENALTY).round() as i16;
    (delta != 0).then_some(StateMutation::QuidProQuoPenalty { delta })
}

#[cfg(test)]
#[path = "luck_adjustments_tests.rs"]
mod tests;
