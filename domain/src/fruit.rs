/// Rarity tier of a fruit, controlling its base drop-rate and how luck affects it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    /// Everyday fruits; their probability decreases as luck rises.
    Standard,
    /// Uncommon fruits; their probability increases with luck.
    Rare,
    /// Very uncommon fruits; their probability increases sharply with luck and
    /// their within-tier spread compresses so the rarest exotics benefit more.
    Exotic,
}

/// A fruit that can be held in a player's bag, gifted, or burned.
///
/// Rarity is stored as an integer in `[0, 100]` (higher = rarer within the
/// tier), which allows all traits to be derived without manual `f64` handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fruit {
    /// Display name.
    pub name: &'static str,
    /// Unicode emoji, e.g. `"🍓"`.
    pub emoji: &'static str,
    /// Rarity tier.
    pub category: Category,
    /// Within-category rarity in `[0, 100]`; higher values are rarer within
    /// that tier.
    pub rarity: u8,
}

// ── Standard (9) ─────────────────────────────────────────────────────────────

pub static GRAPES: Fruit = Fruit {
    emoji: "🍇",
    name: "Grapes",
    category: Category::Standard,
    rarity: 0,
};
pub static MELON: Fruit = Fruit {
    emoji: "🍈",
    name: "Melon",
    category: Category::Standard,
    rarity: 13,
};
pub static WATERMELON: Fruit = Fruit {
    emoji: "🍉",
    name: "Watermelon",
    category: Category::Standard,
    rarity: 25,
};
pub static TANGERINE: Fruit = Fruit {
    emoji: "🍊",
    name: "Tangerine",
    category: Category::Standard,
    rarity: 38,
};
pub static LEMON: Fruit = Fruit {
    emoji: "🍋",
    name: "Lemon",
    category: Category::Standard,
    rarity: 50,
};
pub static BANANA: Fruit = Fruit {
    emoji: "🍌",
    name: "Banana",
    category: Category::Standard,
    rarity: 63,
};
pub static PINEAPPLE: Fruit = Fruit {
    emoji: "🍍",
    name: "Pineapple",
    category: Category::Standard,
    rarity: 75,
};
pub static RED_APPLE: Fruit = Fruit {
    emoji: "🍎",
    name: "Red Apple",
    category: Category::Standard,
    rarity: 88,
};
pub static GREEN_APPLE: Fruit = Fruit {
    emoji: "🍏",
    name: "Green Apple",
    category: Category::Standard,
    rarity: 100,
};

// ── Rare (9) ──────────────────────────────────────────────────────────────────

pub static PEAR: Fruit = Fruit {
    emoji: "🍐",
    name: "Pear",
    category: Category::Rare,
    rarity: 0,
};
pub static PEACH: Fruit = Fruit {
    emoji: "🍑",
    name: "Peach",
    category: Category::Rare,
    rarity: 13,
};
pub static CHERRIES: Fruit = Fruit {
    emoji: "🍒",
    name: "Cherries",
    category: Category::Rare,
    rarity: 25,
};
pub static STRAWBERRY: Fruit = Fruit {
    emoji: "🍓",
    name: "Strawberry",
    category: Category::Rare,
    rarity: 38,
};
pub static AVOCADO: Fruit = Fruit {
    emoji: "🥑",
    name: "Avocado",
    category: Category::Rare,
    rarity: 50,
};
pub static CUCUMBER: Fruit = Fruit {
    emoji: "🥒",
    name: "Cucumber",
    category: Category::Rare,
    rarity: 63,
};
pub static PEANUT: Fruit = Fruit {
    emoji: "🥜",
    name: "Peanut",
    category: Category::Rare,
    rarity: 75,
};
pub static KIWI: Fruit = Fruit {
    emoji: "🥝",
    name: "Kiwi",
    category: Category::Rare,
    rarity: 88,
};
pub static COCONUT: Fruit = Fruit {
    emoji: "🥥",
    name: "Coconut",
    category: Category::Rare,
    rarity: 100,
};

// ── Exotic (8) ────────────────────────────────────────────────────────────────

pub static MANGO: Fruit = Fruit {
    emoji: "🥭",
    name: "Mango",
    category: Category::Exotic,
    rarity: 0,
};
pub static TOMATO: Fruit = Fruit {
    emoji: "🍅",
    name: "Tomato",
    category: Category::Exotic,
    rarity: 14,
};
pub static CHESTNUT: Fruit = Fruit {
    emoji: "🌰",
    name: "Chestnut",
    category: Category::Exotic,
    rarity: 29,
};
pub static HOT_PEPPER: Fruit = Fruit {
    emoji: "🌶",
    name: "Hot Pepper",
    category: Category::Exotic,
    rarity: 43,
};
pub static BELL_PEPPER: Fruit = Fruit {
    emoji: "🫑",
    name: "Bell Pepper",
    category: Category::Exotic,
    rarity: 57,
};
pub static GINGER_ROOT: Fruit = Fruit {
    emoji: "🫚",
    name: "Ginger Root",
    category: Category::Exotic,
    rarity: 71,
};
pub static BLUEBERRIES: Fruit = Fruit {
    emoji: "🫐",
    name: "Blueberries",
    category: Category::Exotic,
    rarity: 86,
};
pub static OLIVE: Fruit = Fruit {
    emoji: "🫒",
    name: "Olive",
    category: Category::Exotic,
    rarity: 100,
};

/// All defined fruits, ordered by category (Standard → Rare → Exotic) then by
/// ascending within-category rarity.
pub static FRUITS: &[Fruit] = &[
    // Standard (9)
    GRAPES,
    MELON,
    WATERMELON,
    TANGERINE,
    LEMON,
    BANANA,
    PINEAPPLE,
    RED_APPLE,
    GREEN_APPLE,
    // Rare (9)
    PEAR,
    PEACH,
    CHERRIES,
    STRAWBERRY,
    AVOCADO,
    CUCUMBER,
    PEANUT,
    KIWI,
    COCONUT,
    // Exotic (8)
    MANGO,
    TOMATO,
    CHESTNUT,
    HOT_PEPPER,
    BELL_PEPPER,
    GINGER_ROOT,
    BLUEBERRIES,
    OLIVE,
];

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers used by both passing and failure tests ────────────────────────

    fn assert_ordered_by_category_then_rarity(fruits: &[Fruit]) {
        let expected = [Category::Standard, Category::Rare, Category::Exotic];
        let mut cat_idx = 0;
        let mut prev_rarity = 0u8;
        for f in fruits {
            if f.category != expected[cat_idx] {
                cat_idx += 1;
                prev_rarity = 0;
                assert_eq!(
                    f.category, expected[cat_idx],
                    "unexpected category order at fruit {}",
                    f.name
                );
            }
            assert!(
                f.rarity >= prev_rarity,
                "rarity not monotonically increasing within {:?} (at {})",
                f.category,
                f.name
            );
            prev_rarity = f.rarity;
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
        assert!(FRUITS.iter().all(|f| f.rarity <= 100));
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
            rarity: 99,
            ..GRAPES
        };
        let low = Fruit {
            rarity: 1,
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
            rarity: 0,
        };
        let b = Fruit { rarity: 50, ..a };
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
            rarity: 99,
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
            rarity: 0,
        };
        let b = Fruit { rarity: 100, ..a };

        let mut map = HashMap::new();
        map.insert(a, 1u32);
        map.insert(b, 2u32);

        assert_eq!(map.len(), 2);
        assert_eq!(map[&a], 1);
        assert_eq!(map[&b], 2);
    }
}
