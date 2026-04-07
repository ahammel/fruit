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
/// Within-category rarity is stored as a raw `u16`; use [`Fruit::rarity`] to
/// obtain the normalised `f64` value in `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fruit {
    /// Display name.
    pub name: &'static str,
    /// Unicode emoji, e.g. `"🍓"`.
    pub emoji: &'static str,
    /// Rarity tier.
    pub category: Category,
    /// Raw within-category rarity; use [`Fruit::rarity`] instead of accessing
    /// this field directly.
    _rarity: u16,
}

impl Fruit {
    /// Within-category rarity normalised to `[0.0, 1.0]`.
    ///
    /// `0.0` is the most common position within the tier; `1.0` is the rarest.
    /// Higher rarity means the fruit is scarcer and commands a higher drop weight
    /// at any luck level.
    pub fn rarity(&self) -> f64 {
        self._rarity as f64 / u16::MAX as f64
    }
}

// ── Standard (9) ─────────────────────────────────────────────────────────────

/// 🍇 Standard-tier fruit.
pub static GRAPES: Fruit = Fruit {
    emoji: "🍇",
    name: "Grapes",
    category: Category::Standard,
    _rarity: 0,
};
/// 🍈 Standard-tier fruit.
pub static MELON: Fruit = Fruit {
    emoji: "🍈",
    name: "Melon",
    category: Category::Standard,
    _rarity: 8224, // 32 × 257
};
/// 🍉 Standard-tier fruit.
pub static WATERMELON: Fruit = Fruit {
    emoji: "🍉",
    name: "Watermelon",
    category: Category::Standard,
    _rarity: 16448, // 64 × 257
};
/// 🍊 Standard-tier fruit.
pub static TANGERINE: Fruit = Fruit {
    emoji: "🍊",
    name: "Tangerine",
    category: Category::Standard,
    _rarity: 24672, // 96 × 257
};
/// 🍋 Standard-tier fruit.
pub static LEMON: Fruit = Fruit {
    emoji: "🍋",
    name: "Lemon",
    category: Category::Standard,
    _rarity: 32896, // 128 × 257
};
/// 🍌 Standard-tier fruit.
pub static BANANA: Fruit = Fruit {
    emoji: "🍌",
    name: "Banana",
    category: Category::Standard,
    _rarity: 40863, // 159 × 257
};
/// 🍍 Standard-tier fruit.
pub static PINEAPPLE: Fruit = Fruit {
    emoji: "🍍",
    name: "Pineapple",
    category: Category::Standard,
    _rarity: 49087, // 191 × 257
};
/// 🍎 Standard-tier fruit.
pub static RED_APPLE: Fruit = Fruit {
    emoji: "🍎",
    name: "Red Apple",
    category: Category::Standard,
    _rarity: 57311, // 223 × 257
};
/// 🍏 Standard-tier fruit.
pub static GREEN_APPLE: Fruit = Fruit {
    emoji: "🍏",
    name: "Green Apple",
    category: Category::Standard,
    _rarity: 65535, // 255 × 257
};

// ── Rare (9) ──────────────────────────────────────────────────────────────────

/// 🍐 Rare-tier fruit.
pub static PEAR: Fruit = Fruit {
    emoji: "🍐",
    name: "Pear",
    category: Category::Rare,
    _rarity: 0,
};
/// 🍑 Rare-tier fruit.
pub static PEACH: Fruit = Fruit {
    emoji: "🍑",
    name: "Peach",
    category: Category::Rare,
    _rarity: 8224, // 32 × 257
};
/// 🍒 Rare-tier fruit.
pub static CHERRIES: Fruit = Fruit {
    emoji: "🍒",
    name: "Cherries",
    category: Category::Rare,
    _rarity: 16448, // 64 × 257
};
/// 🍓 Rare-tier fruit.
pub static STRAWBERRY: Fruit = Fruit {
    emoji: "🍓",
    name: "Strawberry",
    category: Category::Rare,
    _rarity: 24672, // 96 × 257
};
/// 🥑 Rare-tier fruit.
pub static AVOCADO: Fruit = Fruit {
    emoji: "🥑",
    name: "Avocado",
    category: Category::Rare,
    _rarity: 32896, // 128 × 257
};
/// 🥒 Rare-tier fruit.
pub static CUCUMBER: Fruit = Fruit {
    emoji: "🥒",
    name: "Cucumber",
    category: Category::Rare,
    _rarity: 40863, // 159 × 257
};
/// 🥜 Rare-tier fruit.
pub static PEANUT: Fruit = Fruit {
    emoji: "🥜",
    name: "Peanut",
    category: Category::Rare,
    _rarity: 49087, // 191 × 257
};
/// 🥝 Rare-tier fruit.
pub static KIWI: Fruit = Fruit {
    emoji: "🥝",
    name: "Kiwi",
    category: Category::Rare,
    _rarity: 57311, // 223 × 257
};
/// 🥥 Rare-tier fruit.
pub static COCONUT: Fruit = Fruit {
    emoji: "🥥",
    name: "Coconut",
    category: Category::Rare,
    _rarity: 65535, // 255 × 257
};

// ── Exotic (8) ────────────────────────────────────────────────────────────────

/// 🥭 Exotic-tier fruit.
pub static MANGO: Fruit = Fruit {
    emoji: "🥭",
    name: "Mango",
    category: Category::Exotic,
    _rarity: 0,
};
/// 🍅 Exotic-tier fruit.
pub static TOMATO: Fruit = Fruit {
    emoji: "🍅",
    name: "Tomato",
    category: Category::Exotic,
    _rarity: 9252, // 36 × 257
};
/// 🌰 Exotic-tier fruit.
pub static CHESTNUT: Fruit = Fruit {
    emoji: "🌰",
    name: "Chestnut",
    category: Category::Exotic,
    _rarity: 18761, // 73 × 257
};
/// 🌶 Exotic-tier fruit.
pub static HOT_PEPPER: Fruit = Fruit {
    emoji: "🌶",
    name: "Hot Pepper",
    category: Category::Exotic,
    _rarity: 28013, // 109 × 257
};
/// 🫑 Exotic-tier fruit.
pub static BELL_PEPPER: Fruit = Fruit {
    emoji: "🫑",
    name: "Bell Pepper",
    category: Category::Exotic,
    _rarity: 37522, // 146 × 257
};
/// 🫚 Exotic-tier fruit.
pub static GINGER_ROOT: Fruit = Fruit {
    emoji: "🫚",
    name: "Ginger Root",
    category: Category::Exotic,
    _rarity: 46774, // 182 × 257
};
/// 🫐 Exotic-tier fruit.
pub static BLUEBERRIES: Fruit = Fruit {
    emoji: "🫐",
    name: "Blueberries",
    category: Category::Exotic,
    _rarity: 56283, // 219 × 257
};
/// 🫒 Exotic-tier fruit.
pub static OLIVE: Fruit = Fruit {
    emoji: "🫒",
    name: "Olive",
    category: Category::Exotic,
    _rarity: 65535, // 255 × 257
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
#[path = "fruit_tests.rs"]
mod tests;
