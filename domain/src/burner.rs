use std::collections::HashMap;

use crate::{community::Community, event_log::StateMutation, fruit::Fruit, member::MemberId};

/// Computes the effect of burning a collection of fruits.
///
/// `fruits` may contain duplicates and may span multiple fruit types. For each distinct
/// fruit type, `min(requested, held)` instances are burned; any excess is silently
/// skipped. Returns one `RemoveFruitFromMember` mutation per fruit actually burned.
/// Returns an empty `Vec` (no-op) if the member is unknown, `fruits` is empty, or none
/// of the requested fruits are held.
pub fn compute_burn(
    community: &Community,
    member_id: MemberId,
    fruits: &[Fruit],
) -> Vec<StateMutation> {
    if fruits.is_empty() {
        return vec![];
    }
    let member = match community.members.get(&member_id) {
        Some(m) => m,
        None => return vec![],
    };

    // Count requested quantities per fruit type.
    let requested: HashMap<Fruit, usize> = fruits.iter().fold(HashMap::new(), |mut acc, &f| {
        *acc.entry(f).or_insert(0) += 1;
        acc
    });

    // Burn min(requested, held) of each type.
    let removes: Vec<StateMutation> = requested
        .into_iter()
        .flat_map(|(fruit, req)| {
            let actual = req.min(member.bag.count(fruit));
            (0..actual).map(move |_| StateMutation::RemoveFruitFromMember { member_id, fruit })
        })
        .collect();

    if removes.is_empty() {
        return vec![];
    }

    removes
}

#[cfg(test)]
#[path = "burner_tests.rs"]
mod tests;
