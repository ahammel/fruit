/// A fruit that can be held in a player's bag, gifted, or burned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fruit {
    /// Display name.
    pub name: &'static str,
    /// Unicode emoji, e.g. `"🍓"`.
    pub emoji: &'static str,
    /// Normalised rarity in `[0.0, 1.0]`; higher values are rarer.
    pub rarity: f64,
}

pub static GRAPES: Fruit = Fruit {
    emoji: "🍇",
    name: "Grapes",
    rarity: 0.0010,
};
pub static MELON: Fruit = Fruit {
    emoji: "🍈",
    name: "Melon",
    rarity: 0.0016,
};
pub static WATERMELON: Fruit = Fruit {
    emoji: "🍉",
    name: "Watermelon",
    rarity: 0.0021,
};
pub static TANGERINE: Fruit = Fruit {
    emoji: "🍊",
    name: "Tangerine",
    rarity: 0.0026,
};
pub static LEMON: Fruit = Fruit {
    emoji: "🍋",
    name: "Lemon",
    rarity: 0.0031,
};
pub static BANANA: Fruit = Fruit {
    emoji: "🍌",
    name: "Banana",
    rarity: 0.0036,
};
pub static PINEAPPLE: Fruit = Fruit {
    emoji: "🍍",
    name: "Pineapple",
    rarity: 0.0042,
};
pub static RED_APPLE: Fruit = Fruit {
    emoji: "🍎",
    name: "Red Apple",
    rarity: 0.0047,
};
pub static GREEN_APPLE: Fruit = Fruit {
    emoji: "🍏",
    name: "Green Apple",
    rarity: 0.0052,
};
pub static PEAR: Fruit = Fruit {
    emoji: "🍐",
    name: "Pear",
    rarity: 0.0057,
};
pub static PEACH: Fruit = Fruit {
    emoji: "🍑",
    name: "Peach",
    rarity: 0.0063,
};
pub static CHERRIES: Fruit = Fruit {
    emoji: "🍒",
    name: "Cherries",
    rarity: 0.0068,
};
pub static STRAWBERRY: Fruit = Fruit {
    emoji: "🍓",
    name: "Strawberry",
    rarity: 0.0073,
};
pub static AVOCADO: Fruit = Fruit {
    emoji: "🥑",
    name: "Avocado",
    rarity: 0.8020,
};
pub static CUCUMBER: Fruit = Fruit {
    emoji: "🥒",
    name: "Cucumber",
    rarity: 0.8023,
};
pub static PEANUT: Fruit = Fruit {
    emoji: "🥜",
    name: "Peanut",
    rarity: 0.8075,
};
pub static KIWI: Fruit = Fruit {
    emoji: "🥝",
    name: "Kiwi",
    rarity: 0.8082,
};
pub static COCONUT: Fruit = Fruit {
    emoji: "🥥",
    name: "Coconut",
    rarity: 0.8123,
};
pub static MANGO: Fruit = Fruit {
    emoji: "🥭",
    name: "Mango",
    rarity: 0.8164,
};
pub static TOMATO: Fruit = Fruit {
    emoji: "🍅",
    name: "Tomato",
    rarity: 0.8500,
};
pub static CHESTNUT: Fruit = Fruit {
    emoji: "🌰",
    name: "Chestnut",
    rarity: 0.8600,
};
pub static HOT_PEPPER: Fruit = Fruit {
    emoji: "🌶",
    name: "Hot Pepper",
    rarity: 0.8750,
};
pub static BELL_PEPPER: Fruit = Fruit {
    emoji: "🫑",
    name: "Bell Pepper",
    rarity: 0.9200,
};
pub static GINGER_ROOT: Fruit = Fruit {
    emoji: "🫚",
    name: "Ginger Root",
    rarity: 0.9700,
};
pub static BLUEBERRIES: Fruit = Fruit {
    emoji: "🫐",
    name: "Blueberries",
    rarity: 0.9990,
};
pub static OLIVE: Fruit = Fruit {
    emoji: "🫒",
    name: "Olive",
    rarity: 1.0000,
};

/// All defined fruits, ordered ascending by rarity.
pub static FRUITS: &[Fruit] = &[
    GRAPES,
    MELON,
    WATERMELON,
    TANGERINE,
    LEMON,
    BANANA,
    PINEAPPLE,
    RED_APPLE,
    GREEN_APPLE,
    PEAR,
    PEACH,
    CHERRIES,
    STRAWBERRY,
    AVOCADO,
    CUCUMBER,
    PEANUT,
    KIWI,
    COCONUT,
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

    #[test]
    fn rarity_bounds() {
        assert!(FRUITS.iter().all(|f| (0.0..=1.0).contains(&f.rarity)));
    }

    #[test]
    fn rarity_monotonically_increasing() {
        assert!(FRUITS.windows(2).all(|w| w[0].rarity <= w[1].rarity));
    }

    #[test]
    fn all_emojis_are_single_codepoint() {
        assert!(FRUITS.iter().all(|f| f.emoji.chars().count() == 1));
    }
}
