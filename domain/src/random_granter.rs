use rand::{distributions::Distribution, Rng};

use crate::{
    community::Community,
    event_log::StateMutation,
    fruit::{Fruit, FRUITS},
    fruit_weights::{DefaultFruitWeights, FruitWeights},
    granter::Granter,
};

/// Grants fruits by weighted-random selection.
///
/// Each fruit's weight depends on its [`Category`] and within-category rarity
/// combined with the effective luck of the member
/// (`luck = member.luck() + community.luck()`, each normalised to `[0.0, 1.0]`).
/// See [`DefaultFruitWeights`][crate::fruit_weights::DefaultFruitWeights] for the
/// full weight formula.
///
/// The weight strategy is injectable via `W: FruitWeights`; the default is
/// [`DefaultFruitWeights`], which uses the standard drop-rate formula. Use
/// [`with_weights`][RandomGranter::with_weights] to substitute a custom strategy.
///
/// **Approximate category drop-shares:**
/// - At neutral luck: Standard ≈ 90%, Rare ≈ 9%, Exotic ≈ 1%
/// - At max luck (2.0): Standard ≈ 40%, Rare ≈ 40%, Exotic ≈ 20%
pub struct RandomGranter<R: Rng, W: FruitWeights = DefaultFruitWeights> {
    rng: R,
    fruits: &'static [Fruit],
    weights: W,
}

impl<R: Rng> RandomGranter<R> {
    /// Creates a new `RandomGranter` using `rng`, the full [`FRUITS`] pool, and
    /// [`DefaultFruitWeights`].
    pub fn new(rng: R) -> Self {
        Self {
            rng,
            fruits: FRUITS,
            weights: DefaultFruitWeights,
        }
    }
}

impl<R: Rng, W: FruitWeights> RandomGranter<R, W> {
    /// Replaces the fruit pool used for selection.
    ///
    /// # Panics
    ///
    /// Panics if `fruits` is empty.
    pub fn with_fruits(self, fruits: &'static [Fruit]) -> Self {
        assert!(!fruits.is_empty(), "fruit pool must not be empty");
        Self { fruits, ..self }
    }

    /// Replaces the weight strategy used for selection.
    pub fn with_weights<W2: FruitWeights>(self, weights: W2) -> RandomGranter<R, W2> {
        RandomGranter {
            rng: self.rng,
            fruits: self.fruits,
            weights,
        }
    }
}

impl<R: Rng, W: FruitWeights> Granter for RandomGranter<R, W> {
    fn grant(&mut self, community: &Community, count: usize) -> Vec<StateMutation> {
        let community_luck = community.luck();
        let mut mutations = Vec::new();
        for member in community.members.values() {
            let luck = member.luck() + community_luck;
            let dist = self.weights.fruit_weights(self.fruits, luck);
            for _ in 0..count {
                mutations.push(StateMutation::AddFruitToMember {
                    member_id: member.id,
                    fruit: self.fruits[dist.sample(&mut self.rng)],
                });
            }
        }
        mutations
    }
}

#[cfg(test)]
#[path = "random_granter_tests.rs"]
mod tests;
