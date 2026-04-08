use crate::{community::Community, event_log::StateMutation, fruit::Fruit, member::MemberId};

/// Computes the effect of a gift from one member to another.
///
/// Returns the mutations to apply if the gift is valid (sender holds the fruit),
/// or an empty `Vec` (no-op) if the invariant is violated.
pub fn compute_gift(
    community: &Community,
    sender_id: MemberId,
    recipient_id: MemberId,
    fruit: Fruit,
) -> Vec<StateMutation> {
    let sender = match community.members.get(&sender_id) {
        Some(m) => m,
        None => return vec![],
    };
    if sender.bag.count(fruit) == 0 {
        return vec![];
    }
    vec![
        StateMutation::RemoveFruitFromMember {
            member_id: sender_id,
            fruit,
        },
        StateMutation::AddFruitToMember {
            member_id: recipient_id,
            fruit,
        },
    ]
}

#[cfg(test)]
#[path = "gifter_tests.rs"]
mod tests;
