use super::*;
use crate::fruit::{Category, COCONUT, FRUITS, GRAPES, GREEN_APPLE, MANGO, OLIVE, PEAR};

/// Asserts element-wise approximate equality within `1e-10`.
///
/// A tolerance this tight will catch any formula error (which shifts weights
/// by several percent) while ignoring IEEE 754 rounding noise.
fn assert_weights_approx_eq(actual: Vec<f64>, expected: Vec<f64>) {
    assert_eq!(actual.len(), expected.len(), "length mismatch");
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!((a - e).abs() < 1e-10, "weights[{i}]: got {a}, expected {e}");
    }
}

// ── Standard ──────────────────────────────────────────────────────────────
//
// Formula: tier(r) × 10 / (1 + 2·luck)  where tier(r) = 1 + 2r
//
// GRAPES: r = 0.0  →  tier = 1.0
// GREEN_APPLE: r = 1.0  →  tier = 3.0

#[test]
fn standard_at_neutral_luck() {
    assert_weights_approx_eq(
        compute_raw_weights(&[GRAPES, GREEN_APPLE], 0.0),
        vec![10.0, 30.0],
    );
}

#[test]
fn standard_at_unit_luck() {
    assert_weights_approx_eq(
        compute_raw_weights(&[GRAPES, GREEN_APPLE], 1.0),
        // divisor = 1 + 2 = 3
        vec![10.0 / 3.0, 10.0],
    );
}

#[test]
fn standard_at_max_luck() {
    assert_weights_approx_eq(
        compute_raw_weights(&[GRAPES, GREEN_APPLE], 2.0),
        // divisor = 1 + 4 = 5
        vec![2.0, 6.0],
    );
}

// ── Rare ──────────────────────────────────────────────────────────────────
//
// Formula: tier(r) × (1 + luck/2)
//
// PEAR: r = 0.0  →  tier = 1.0
// COCONUT: r = 1.0  →  tier = 3.0

#[test]
fn rare_at_neutral_luck() {
    assert_weights_approx_eq(compute_raw_weights(&[PEAR, COCONUT], 0.0), vec![1.0, 3.0]);
}

#[test]
fn rare_at_unit_luck() {
    assert_weights_approx_eq(
        compute_raw_weights(&[PEAR, COCONUT], 1.0),
        // multiplier = 1 + 0.5 = 1.5
        vec![1.5, 4.5],
    );
}

#[test]
fn rare_at_max_luck() {
    assert_weights_approx_eq(
        compute_raw_weights(&[PEAR, COCONUT], 2.0),
        // multiplier = 1 + 1 = 2
        vec![2.0, 6.0],
    );
}

// ── Exotic ────────────────────────────────────────────────────────────────
//
// Formula: tier(r) × 0.125 × (1 + luck)²
//
// MANGO: r = 0.0  →  tier = 1.0
// OLIVE: r = 1.0  →  tier = 3.0

#[test]
fn exotic_at_neutral_luck() {
    // (1 + 0)² = 1  →  weights = [0.125, 0.375]
    assert_weights_approx_eq(
        compute_raw_weights(&[MANGO, OLIVE], 0.0),
        vec![0.125, 0.375],
    );
}

#[test]
fn exotic_at_unit_luck() {
    // (1 + 1)² = 4  →  weights = [0.5, 1.5]
    assert_weights_approx_eq(compute_raw_weights(&[MANGO, OLIVE], 1.0), vec![0.5, 1.5]);
}

#[test]
fn exotic_at_max_luck() {
    // (1 + 2)² = 9  →  weights = [1.125, 3.375]
    assert_weights_approx_eq(
        compute_raw_weights(&[MANGO, OLIVE], 2.0),
        vec![1.125, 3.375],
    );
}

#[test]
fn exotic_at_half_luck() {
    // (1 + 0.5)² = 2.25  →  weights = [0.28125, 0.84375]
    assert_weights_approx_eq(
        compute_raw_weights(&[MANGO, OLIVE], 0.5),
        vec![0.28125, 0.84375],
    );
}

// ── Structural ────────────────────────────────────────────────────────────

#[test]
fn output_length_matches_input() {
    assert_eq!(compute_raw_weights(&[], 0.0).len(), 0);
    assert_eq!(compute_raw_weights(&[GRAPES], 0.0).len(), 1);
    assert_eq!(compute_raw_weights(&[GRAPES, PEAR, MANGO], 0.0).len(), 3);
}

#[test]
fn all_weights_are_positive() {
    let all_fruits = [GRAPES, GREEN_APPLE, PEAR, COCONUT, MANGO, OLIVE];
    for luck in [0.0, 0.5, 1.0, 2.0] {
        assert!(compute_raw_weights(&all_fruits, luck)
            .iter()
            .all(|&w| w > 0.0));
    }
}

// ── Within-tier ordering ──────────────────────────────────────────────────
//
// The within-tier factor `1 + 2r` gives an exact 3:1 weight ratio between
// the max-rarity (r=1) and min-rarity (r=0) fruit in any category, at any
// luck value.

#[test]
fn max_rarity_is_exactly_3x_more_likely_than_min_rarity() {
    for luck in [0.0, 0.5, 1.0, 2.0] {
        let std_w = compute_raw_weights(&[GRAPES, GREEN_APPLE], luck);
        assert!(
            (std_w[1] - 3.0 * std_w[0]).abs() < 1e-10,
            "standard: ratio ≠ 3 at luck={luck}"
        );

        let rare_w = compute_raw_weights(&[PEAR, COCONUT], luck);
        assert!(
            (rare_w[1] - 3.0 * rare_w[0]).abs() < 1e-10,
            "rare: ratio ≠ 3 at luck={luck}"
        );

        let exotic_w = compute_raw_weights(&[MANGO, OLIVE], luck);
        assert!(
            (exotic_w[1] - 3.0 * exotic_w[0]).abs() < 1e-10,
            "exotic: ratio ≠ 3 at luck={luck}"
        );
    }
}

// ── Category proportions ──────────────────────────────────────────────────
//
// With the full FRUITS pool (9 standard, 9 rare, 8 exotic):
//
//   Σ tier(r) for standard ≈ 18   (category scale × 18 determines total weight)
//   Σ tier(r) for rare     ≈ 18
//   Σ tier(r) for exotic   = 16
//
// At l=0 the per-category totals are ≈ 180 : 18 : 2  →  90% : 9% : 1%.
// At l=2 the per-category totals are ≈  36 : 36 : 18  →  40% : 40% : 20%.

fn category_fractions(luck: f64) -> (f64, f64, f64) {
    let weights = compute_raw_weights(FRUITS, luck);
    let total: f64 = weights.iter().sum();
    let (mut s, mut r, mut e) = (0.0_f64, 0.0_f64, 0.0_f64);
    for (fruit, &w) in FRUITS.iter().zip(weights.iter()) {
        match fruit.category {
            Category::Standard => s += w,
            Category::Rare => r += w,
            Category::Exotic => e += w,
        }
    }
    (s / total, r / total, e / total)
}

#[test]
fn category_proportions_at_neutral_luck() {
    let (std, rare, exotic) = category_fractions(0.0);
    assert!(
        (std - 0.90).abs() < 0.005,
        "standard ≈ 90% at luck=0, got {std:.3}"
    );
    assert!(
        (rare - 0.09).abs() < 0.005,
        "rare ≈ 9% at luck=0, got {rare:.3}"
    );
    assert!(
        (exotic - 0.01).abs() < 0.002,
        "exotic ≈ 1% at luck=0, got {exotic:.3}"
    );
}

#[test]
fn category_proportions_at_max_luck() {
    let (std, rare, exotic) = category_fractions(2.0);
    assert!(
        (std - 0.40).abs() < 0.005,
        "standard ≈ 40% at luck=2, got {std:.3}"
    );
    assert!(
        (rare - 0.40).abs() < 0.005,
        "rare ≈ 40% at luck=2, got {rare:.3}"
    );
    assert!(
        (exotic - 0.20).abs() < 0.005,
        "exotic ≈ 20% at luck=2, got {exotic:.3}"
    );
}

// ── DefaultFruitWeights wiring ────────────────────────────────────────────

#[test]
fn default_fruit_weights_builds_valid_index() {
    use rand::distributions::Distribution;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    // Single-fruit pool: must always return index 0 regardless of luck.
    let dist = DefaultFruitWeights.fruit_weights(&[GRAPES], 1.0);
    let mut rng = StdRng::seed_from_u64(0);
    assert_eq!(dist.sample(&mut rng), 0);
}
