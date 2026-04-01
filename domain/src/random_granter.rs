use rand::{
    distributions::{Distribution, WeightedIndex},
    Rng,
};

use crate::{
    community::Community,
    fruit::{Fruit, FRUITS},
    granter::Granter,
};

/// Grants fruits by weighted-random selection.
///
/// The probability weight for fruit `i` given a member is:
///
/// ```text
/// weight_i = (1 - rarity_i + luck * rarity_i).max(ε)
/// luck     = (member.luck + community.luck).max(0.0)
/// ```
///
/// At `luck = 0` common fruits (low rarity) dominate. At `luck = 1` all
/// fruits are equally likely. Above `1` rare fruits dominate.
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
            let luck = (member.luck() + community_luck).max(0.0);
            let weights: Vec<f64> = self
                .fruits
                .iter()
                .map(|f| (1.0 - f.rarity + luck * f.rarity).max(f64::EPSILON))
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
        fruit::{CHERRIES, GRAPES, PEACH},
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
            Bag::new().insert(PEACH).insert(CHERRIES)
        );
    }

    static TWO_FRUITS: &[Fruit] = &[GRAPES, CHERRIES];

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
        assert!(bag.count(GRAPES) + bag.count(CHERRIES) == 3);
    }
}
