use rand::distributions::WeightedIndex;

use crate::fruit::{Category, Fruit};

/// Computes a per-fruit weight vector for `fruits` given effective luck `luck`.
///
/// Let `r = fruit.rarity() ∈ [0.0, 1.0]` and `l = luck ∈ [0.0, 2.0]`.
///
/// **Within-tier factor** (shared by all categories):
/// ```text
/// tier(r) = 1 + 2·r
/// ```
/// This gives a 3:1 ratio between max-rarity and min-rarity fruits within any
/// tier at any luck value — higher rarity means higher drop weight.
///
/// **Category scaling:**
/// ```text
/// Standard : tier(r) × 10 / (1 + 2·l)
/// Rare     : tier(r) × (1 + l/2)
/// Exotic   : tier(r) × 0.125 × (1 + l)²
/// ```
///
/// **Category drop-share at neutral luck (l = 0):** Standard ≈ 90%, Rare ≈ 9%, Exotic ≈ 1%.
///
/// **Category drop-share at max luck (l = 2):** Standard ≈ 40%, Rare ≈ 40%, Exotic ≈ 20%.
///
/// Every weight is floored at [`f64::EPSILON`] so no fruit is ever completely
/// excluded from sampling.
fn compute_raw_weights(fruits: &[Fruit], luck: f64) -> Vec<f64> {
    fruits
        .iter()
        .map(|f| {
            let r = f.rarity();
            let tier = 1.0 + 2.0 * r;
            (match f.category {
                Category::Standard => tier * 10.0 / (1.0 + 2.0 * luck),
                Category::Rare => tier * (1.0 + luck / 2.0),
                Category::Exotic => tier * 0.125 * (1.0 + luck).powi(2),
            })
            .max(f64::EPSILON)
        })
        .collect()
}

/// Computes a [`WeightedIndex`] for sampling from `fruits` given a luck value.
///
/// Implement this trait to substitute a custom weighting strategy into
/// [`RandomGranter`][crate::random_granter::RandomGranter]. The default
/// implementation uses the formula described in [`compute_raw_weights`].
pub trait FruitWeights {
    /// Returns a [`WeightedIndex`] over `fruits` for a recipient with total effective
    /// luck `luck` (`= member.luck() + community.luck()`, each in `[0.0, 1.0]`).
    ///
    /// # Panics
    ///
    /// Panics if `fruits` is empty.
    fn fruit_weights(&self, fruits: &[Fruit], luck: f64) -> WeightedIndex<f64> {
        WeightedIndex::new(compute_raw_weights(fruits, luck))
            .expect("weights are always valid with a non-empty fruit pool and finite luck")
    }
}

/// The default [`FruitWeights`] implementation, using the standard drop-rate formula.
pub struct DefaultFruitWeights;

impl FruitWeights for DefaultFruitWeights {}

#[cfg(test)]
#[path = "fruit_weights_tests.rs"]
mod tests;
