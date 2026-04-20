use std::collections::{hash_map::Entry, HashMap};

use crate::fruit::Fruit;

/// A multiset of [`Fruit`], tracking how many of each type are held.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

/// Sum of `fruit.value() × count` for every distinct fruit held in `bag`.
pub fn bag_value(bag: &Bag) -> f64 {
    bag.iter()
        .map(|(fruit, count)| fruit.value() * count as f64)
        .sum()
}

#[cfg(test)]
#[path = "bag_tests.rs"]
mod tests;
