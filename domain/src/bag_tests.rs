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
