use std::collections::{hash_map::Entry, HashMap};

use crate::fruit::Fruit;

/// A multiset of [`Fruit`], tracking how many of each type are held.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bag {
    counts: HashMap<Fruit, usize>,
}

impl Bag {
    /// Creates an empty bag.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one instance of `fruit` and returns the bag for chaining.
    pub fn insert(mut self, fruit: Fruit) -> Self {
        *self.counts.entry(fruit).or_insert(0) += 1;
        self
    }

    /// Removes one instance of `fruit`. Returns `false` if the fruit was not present.
    pub fn remove(&mut self, fruit: Fruit) -> bool {
        match self.counts.entry(fruit) {
            Entry::Vacant(_) => false,
            Entry::Occupied(mut e) => {
                if *e.get() == 1 {
                    e.remove();
                } else {
                    *e.get_mut() -= 1;
                }
                true
            }
        }
    }

    /// Returns the number of instances of `fruit` in the bag.
    pub fn count(&self, fruit: Fruit) -> usize {
        self.counts.get(&fruit).copied().unwrap_or(0)
    }

    /// Returns the total number of fruits in the bag (with multiplicity).
    pub fn total(&self) -> usize {
        self.counts.values().sum()
    }

    /// Returns `true` if the bag contains no fruits.
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    /// Iterates over each distinct fruit and its count.
    pub fn iter(&self) -> impl Iterator<Item = (Fruit, usize)> + '_ {
        self.counts.iter().map(|(&fruit, &count)| (fruit, count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fruit::{GRAPES, OLIVE};

    #[test]
    fn insert_and_count() {
        let bag = Bag::new().insert(GRAPES).insert(GRAPES);
        assert_eq!(bag.count(GRAPES), 2);
        assert_eq!(bag.total(), 2);
    }

    #[test]
    fn remove_decrements_and_cleans_up() {
        let mut bag = Bag::new().insert(GRAPES);
        assert!(bag.remove(GRAPES));
        assert_eq!(bag, Bag::new());
        assert!(!bag.remove(GRAPES));
    }

    #[test]
    fn remove_decrements_without_removing_when_count_gt_1() {
        let mut bag = Bag::new().insert(GRAPES).insert(GRAPES);
        assert!(bag.remove(GRAPES));
        assert_eq!(bag, Bag::new().insert(GRAPES));
    }

    #[test]
    fn total_across_distinct_fruits() {
        let bag = Bag::new().insert(GRAPES).insert(OLIVE).insert(OLIVE);
        assert_eq!(bag.total(), 3);
        assert_eq!(bag.count(OLIVE), 2);
    }

    #[test]
    fn is_empty_reflects_contents() {
        assert!(Bag::new().is_empty());
        assert!(!Bag::new().insert(GRAPES).is_empty());
    }

    #[test]
    fn iter_yields_each_fruit_with_count() {
        let bag = Bag::new().insert(GRAPES).insert(GRAPES).insert(OLIVE);
        let mut items: Vec<(Fruit, usize)> = bag.iter().collect();
        items.sort_by_key(|(f, _)| f.name);
        assert_eq!(items, vec![(GRAPES, 2), (OLIVE, 1)]);
    }
}
