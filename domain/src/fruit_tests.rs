use super::*;

// ── Helpers used by both passing and failure tests ────────────────────────

fn assert_ordered_by_category_then_rarity(fruits: &[Fruit]) {
    let expected = [Category::Standard, Category::Rare, Category::Exotic];
    let mut cat_idx = 0;
    let mut prev_rarity = 0.0f64;
    for f in fruits {
        if f.category != expected[cat_idx] {
            cat_idx += 1;
            prev_rarity = 0.0;
            assert_eq!(
                f.category, expected[cat_idx],
                "unexpected category order at fruit {}",
                f.name
            );
        }
        assert!(
            f.rarity() >= prev_rarity,
            "rarity not monotonically increasing within {:?} (at {})",
            f.category,
            f.name
        );
        prev_rarity = f.rarity();
    }
}

fn assert_category_counts_balanced(fruits: &[Fruit]) {
    let counts = [Category::Standard, Category::Rare, Category::Exotic]
        .map(|c| fruits.iter().filter(|f| f.category == c).count());
    let min = *counts.iter().min().unwrap();
    let max = *counts.iter().max().unwrap();
    assert!(
        max - min <= 1,
        "category counts are unbalanced: Standard={}, Rare={}, Exotic={}",
        counts[0],
        counts[1],
        counts[2]
    );
}

// ── Passing tests ─────────────────────────────────────────────────────────

#[test]
fn rarity_bounds() {
    assert!(FRUITS.iter().all(|f| f.rarity() <= 1.0));
}

#[test]
fn fruits_ordered_by_category_then_rarity() {
    assert_ordered_by_category_then_rarity(FRUITS);
}

#[test]
fn category_counts_are_balanced() {
    assert_category_counts_balanced(FRUITS);
}

// ── Failure-path tests (cover the panic messages) ─────────────────────────

#[test]
#[should_panic(expected = "unexpected category order at fruit Grapes")]
fn out_of_order_category_is_detected() {
    // PEAR is Rare; GRAPES after it is Standard, which is out of order.
    assert_ordered_by_category_then_rarity(&[PEAR, GRAPES]);
}

#[test]
#[should_panic(expected = "rarity not monotonically increasing within Standard")]
fn decreasing_rarity_within_category_is_detected() {
    let high = Fruit {
        _rarity: 200,
        ..GRAPES
    };
    let low = Fruit {
        _rarity: 1,
        ..GRAPES
    };
    assert_ordered_by_category_then_rarity(&[high, low]);
}

#[test]
#[should_panic(expected = "category counts are unbalanced: Standard=3, Rare=0, Exotic=0")]
fn imbalanced_category_counts_are_detected() {
    assert_category_counts_balanced(&[GRAPES, GRAPES, GRAPES]);
}

#[test]
fn all_emojis_are_single_codepoint() {
    assert!(FRUITS.iter().all(|f| f.emoji.chars().count() == 1));
}

#[test]
fn equality_uses_all_fields() {
    let a = Fruit {
        name: "Test",
        emoji: "🍇",
        category: Category::Standard,
        _rarity: 0,
    };
    let b = Fruit { _rarity: 50, ..a };
    assert_ne!(a, b);

    let c = Fruit {
        category: Category::Rare,
        ..a
    };
    assert_ne!(a, c);

    assert_eq!(a, a);
}

#[test]
fn hash_consistent_with_eq() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let hash = |f: &Fruit| {
        let mut h = DefaultHasher::new();
        f.hash(&mut h);
        h.finish()
    };

    let a = GRAPES;
    let b = GRAPES;
    assert_eq!(a, b);
    assert_eq!(hash(&a), hash(&b));

    let tweaked = Fruit {
        _rarity: 99,
        ..GRAPES
    };
    assert_ne!(a, tweaked);
}

#[test]
fn fruits_with_same_emoji_but_different_rarity_are_distinct_map_keys() {
    use std::collections::HashMap;

    let a = Fruit {
        name: "Apple",
        emoji: "🍎",
        category: Category::Standard,
        _rarity: 0,
    };
    let b = Fruit { _rarity: 100, ..a };

    let mut map = HashMap::new();
    map.insert(a, 1u32);
    map.insert(b, 2u32);

    assert_eq!(map.len(), 2);
    assert_eq!(map[&a], 1);
    assert_eq!(map[&b], 2);
}
