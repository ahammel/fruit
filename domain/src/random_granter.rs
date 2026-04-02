use rand::{
    distributions::{Distribution, WeightedIndex},
    Rng,
};

use crate::{
    community::Community,
    fruit::{Category, Fruit, FRUITS},
    granter::Granter,
};

/// Grants fruits by weighted-random selection.
///
/// Each fruit's weight depends on its [`Category`] and within-category
/// `rarity` (`r ∈ [0, 1]`), combined with the effective luck of the member
/// (`luck = member.luck() + community.luck()`, each normalised to `[0, 1]`):
///
/// ```text
/// Standard : (0.065 − 0.030·r) / (1 + luck)
/// Rare     : (0.050 − 0.030·r) · (1 + luck)
/// Exotic   : lerp(0.010 − 0.009·r,  0.050 − 0.040·r,  luck)
/// ```
///
/// At neutral luck (`luck = 0`) the approximate per-fruit probabilities are:
/// - Standard : most common ~1/15, rarest ~1/22
/// - Rare     : most common ~1/19, rarest ~1/48
/// - Exotic   : most common ~1/96, rarest ~1/958
///
/// At `luck = 1` the exotic range compresses to roughly 1/21 – 1/104, with
/// the rarest exotics receiving the largest boost.
pub struct RandomGranter<R: Rng> {
    rng: R,
    fruits: &'static [Fruit],
}

impl<R: Rng> RandomGranter<R> {
    /// Creates a new `RandomGranter` using `rng` and the full [`FRUITS`] pool.
    pub fn new(rng: R) -> Self {
        Self {
            rng,
            fruits: FRUITS,
        }
    }

    /// Replaces the fruit pool used for selection.
    ///
    /// # Panics
    ///
    /// Panics if `fruits` is empty.
    pub fn with_fruits(self, fruits: &'static [Fruit]) -> Self {
        assert!(!fruits.is_empty(), "fruit pool must not be empty");
        Self { fruits, ..self }
    }
}

impl<R: Rng> Granter for RandomGranter<R> {
    fn grant(&mut self, community: &mut Community, count: usize) {
        let community_luck = community.luck();
        for member in community.members.values_mut() {
            let luck = member.luck() + community_luck;
            let weights: Vec<f64> = self
                .fruits
                .iter()
                .map(|f| {
                    let r = f.rarity();
                    (match f.category {
                        Category::Standard => (0.065 - 0.030 * r) / (1.0 + luck),
                        Category::Rare => (0.050 - 0.030 * r) * (1.0 + luck),
                        Category::Exotic => {
                            let base = 0.010 - 0.009 * r;
                            let high = 0.050 - 0.040 * r;
                            base + luck * (high - base)
                        }
                    })
                    .max(f64::EPSILON)
                })
                .collect();
            let dist = WeightedIndex::new(&weights)
                .expect("weights are always valid with a non-empty fruit pool and finite luck");
            for _ in 0..count {
                let fruit = self.fruits[dist.sample(&mut self.rng)];
                member.receive(fruit);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;
    use crate::{
        bag::Bag,
        community::Community,
        fruit::{GRAPES, STRAWBERRY},
        member::Member,
    };

    #[test]
    fn grants_fruits_to_each_member() {
        let mut granter = RandomGranter::new(StdRng::seed_from_u64(0));
        let mut community = Community::new();
        community.add_member(Member::new("Alice"));
        community.add_member(Member::new("Bob"));
        granter.grant(&mut community, 3);
        assert!(community.members.values().all(|m| m.bag.total() == 3));
    }

    #[test]
    fn grants_correct_fruits_with_fixed_seed() {
        let mut granter = RandomGranter::new(StdRng::seed_from_u64(0));
        let mut community = Community::new();
        let member = Member::new("Alice");
        let id = member.id;
        community.add_member(member);

        granter.grant(&mut community, 2);

        // Expected value determined by running with seed 0 and neutral luck.
        // Update this if FRUITS or the weight formula changes.
        assert_eq!(
            community.members[&id].bag,
            Bag::new().insert(STRAWBERRY).insert(STRAWBERRY)
        );
    }

    static TWO_FRUITS: &[Fruit] = &[GRAPES, STRAWBERRY];

    #[test]
    #[should_panic(expected = "fruit pool must not be empty")]
    fn with_fruits_rejects_empty_pool() {
        RandomGranter::new(StdRng::seed_from_u64(0)).with_fruits(&[]);
    }

    #[test]
    fn respects_custom_fruit_pool() {
        let mut granter = RandomGranter::new(StdRng::seed_from_u64(0)).with_fruits(TWO_FRUITS);
        let mut community = Community::new();
        let member = Member::new("Alice");
        let id = member.id;
        community.add_member(member);

        granter.grant(&mut community, 3);

        let bag = &community.members[&id].bag;
        assert_eq!(bag.total(), 3);
        assert!(bag.count(GRAPES) + bag.count(STRAWBERRY) == 3);
    }
}
