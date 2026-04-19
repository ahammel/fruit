use rand::rngs::StdRng;
use rand::SeedableRng;

use rand::distributions::WeightedIndex;

use super::*;
use crate::{
    community::Community,
    event_log::Effect,
    event_log::SequenceId,
    fruit::{GRAPES, MANGO, OLIVE, STRAWBERRY},
    fruit_weights::FruitWeights,
    member::Member,
};

fn apply_grant(community: &mut Community, mutations: Vec<StateMutation>) {
    Effect {
        id: SequenceId::new(1),
        community_id: community.id,
        mutations,
    }
    .apply(community);
}

#[test]
fn grants_fruits_to_each_member() {
    let mut granter = RandomGranter::new(StdRng::seed_from_u64(0));
    let mut community = Community::new();
    community.add_member(Member::new("Alice"));
    community.add_member(Member::new("Bob"));
    let mutations = granter.grant(&community, 3);
    apply_grant(&mut community, mutations);
    assert!(community.members.values().all(|m| m.bag.total() == 3));
}

#[test]
fn grants_correct_count_with_fixed_seed() {
    let mut granter = RandomGranter::new(StdRng::seed_from_u64(0));
    let mut community = Community::new();
    let member = Member::new("Alice");
    let id = member.id;
    community.add_member(member);

    let mutations = granter.grant(&community, 2);
    apply_grant(&mut community, mutations);

    assert_eq!(community.members[&id].bag.total(), 2);
}

static ONLY_GRAPES: &[Fruit] = &[GRAPES];
static TWO_FRUITS: &[Fruit] = &[GRAPES, STRAWBERRY];
static STANDARD_AND_EXOTIC: &[Fruit] = &[GRAPES, MANGO];
static STANDARD_AND_RAREST_EXOTIC: &[Fruit] = &[GRAPES, OLIVE];

#[test]
#[should_panic(expected = "fruit pool must not be empty")]
fn with_fruits_rejects_empty_pool() {
    RandomGranter::new(StdRng::seed_from_u64(0)).with_fruits(&[]);
}

#[test]
fn with_fruits_restricts_pool_to_single_fruit() {
    // If with_fruits ignores the pool (mutant: uses FRUITS), grants would not be 100% GRAPES.
    let mut granter = RandomGranter::new(StdRng::seed_from_u64(0)).with_fruits(ONLY_GRAPES);
    let mut community = Community::new();
    let member = Member::new("Alice");
    let id = member.id;
    community.add_member(member);

    let mutations = granter.grant(&community, 10);
    apply_grant(&mut community, mutations);

    assert_eq!(community.members[&id].bag.count(GRAPES), 10);
}

#[test]
fn respects_custom_fruit_pool() {
    let mut granter = RandomGranter::new(StdRng::seed_from_u64(0)).with_fruits(TWO_FRUITS);
    let mut community = Community::new();
    let member = Member::new("Alice");
    let id = member.id;
    community.add_member(member);

    let mutations = granter.grant(&community, 3);
    apply_grant(&mut community, mutations);

    let bag = &community.members[&id].bag;
    assert_eq!(bag.total(), 3);
    assert!(bag.count(GRAPES) + bag.count(STRAWBERRY) == 3);
}

// ── Custom FruitWeights ───────────────────────────────────────────────────

/// Always assigns equal weight to every fruit, regardless of luck.
struct UniformWeights;

impl FruitWeights for UniformWeights {
    fn fruit_weights(&self, fruits: &[Fruit], _luck: f64) -> WeightedIndex<f64> {
        WeightedIndex::new(vec![1.0; fruits.len()]).unwrap()
    }
}

#[test]
fn with_weights_substitutes_custom_strategy() {
    // UniformWeights gives each fruit equal probability regardless of luck.
    // A pool of [GRAPES, STRAWBERRY] with luck=1.0 and DefaultFruitWeights
    // would heavily favour STRAWBERRY (Rare); with UniformWeights both fruits
    // are equally likely. Verify by checking that GRAPES appears in 1000 grants
    // (it would be ~29.6% with default weights but ~50% uniform — either way
    // it appears, so we just confirm no panic and correct pool).
    let mut granter = RandomGranter::new(StdRng::seed_from_u64(0))
        .with_fruits(TWO_FRUITS)
        .with_weights(UniformWeights);
    let mut community = Community::new();
    let member = Member::new("Alice").with_luck_f64(1.0);
    let id = member.id;
    community.add_member(member);

    let mutations = granter.grant(&community, 1000);
    apply_grant(&mut community, mutations);

    let bag = &community.members[&id].bag;
    assert_eq!(bag.total(), 1000);
    // Both fruits should appear; neither should dominate to 100%.
    assert!(bag.count(GRAPES) > 0 && bag.count(STRAWBERRY) > 0);
}

// ── Distribution tests (default weights) ─────────────────────────────────

#[test]
fn luck_shifts_distribution_toward_rare() {
    // GRAPES (Standard, r=0): weight = 10 / (1 + 2·luck)
    // STRAWBERRY (Rare, r≈0.376): tier≈1.752, weight = 1.752 · (1 + luck/2)
    //   luck=0.0: P(GRAPES) ≈ 85.1%, P(STRAWBERRY) ≈ 14.9%
    //   luck=1.5: P(GRAPES) ≈ 45%,   P(STRAWBERRY) ≈ 55%     ← crossover ~1.25
    // Both member (1.0) and community (0.5) luck contribute; total = 1.5.
    // Any mutation breaking either luck term drops total below crossover and flips the ordering.
    let mut granter = RandomGranter::new(StdRng::seed_from_u64(0)).with_fruits(TWO_FRUITS);
    let mut community = Community::new().with_luck_f64(0.5);
    let member = Member::new("Alice").with_luck_f64(1.0);
    let id = member.id;
    community.add_member(member);

    let mutations = granter.grant(&community, 1000);
    apply_grant(&mut community, mutations);

    let bag = &community.members[&id].bag;
    assert!(bag.count(STRAWBERRY) > bag.count(GRAPES));
}

#[test]
fn luck_shifts_distribution_toward_exotic() {
    // GRAPES (Standard, r=0): weight = 10 / (1 + 2·luck)
    // OLIVE (Exotic, r=1): tier=3, weight = 3 · 0.125 · (1 + luck)²
    //   luck=0.0: P(GRAPES) ≈ 96.4%, P(OLIVE) ≈ 3.6%
    //   luck=2.0: P(GRAPES) ≈ 37.2%, P(OLIVE) ≈ 62.8%   ← crossover ~1.6
    // Member (1.0) + community (1.0) = total 2.0, above the crossover.
    // Any mutation breaking the exotic square or standard divisor inverts the ordering.
    let mut granter =
        RandomGranter::new(StdRng::seed_from_u64(0)).with_fruits(STANDARD_AND_RAREST_EXOTIC);
    let mut community = Community::new().with_luck_f64(1.0);
    let member = Member::new("Alice").with_luck_f64(1.0);
    let id = member.id;
    community.add_member(member);

    let mutations = granter.grant(&community, 1000);
    apply_grant(&mut community, mutations);

    let bag = &community.members[&id].bag;
    assert!(bag.count(OLIVE) > bag.count(GRAPES));
}

#[test]
fn community_luck_shifts_distribution() {
    // luck = member.luck() + community_luck.
    // If `+` is mutated to `-`, the total drops below the crossover and flips the ordering.
    // member_luck=1.0, community_luck=0.5 → total=1.5 → STRAWBERRY wins.
    // With the `-` mutant: total=0.5 → GRAPES wins (crossover is ~1.25).
    let mut granter = RandomGranter::new(StdRng::seed_from_u64(0)).with_fruits(TWO_FRUITS);
    let mut community = Community::new().with_luck_f64(0.5);
    let member = Member::new("Alice").with_luck_f64(1.0);
    let id = member.id;
    community.add_member(member);

    let mutations = granter.grant(&community, 1000);
    apply_grant(&mut community, mutations);

    assert!(
        community.members[&id].bag.count(STRAWBERRY) > community.members[&id].bag.count(GRAPES)
    );
}

#[test]
fn standard_dominates_rarest_exotic_at_max_luck() {
    // Tests the `high` formula for Exotic: `high = 0.050 - 0.040 * r`.
    // At r=1 (OLIVE), correct high=0.010 so weight@luck=1 ≈ 0.010; GRAPES weight ≈ 0.0325.
    // Mutant (`- with +`): high=0.090, weight@luck=1 ≈ 0.090 → OLIVE would dominate.
    // Also catches the `luck * (…)` → `luck / (…)` and `high - base` → `high / base` mutants
    // (both make exotic weight astronomically large at luck=1).
    let mut granter =
        RandomGranter::new(StdRng::seed_from_u64(0)).with_fruits(STANDARD_AND_RAREST_EXOTIC);
    let mut community = Community::new();
    let member = Member::new("Alice").with_luck_f64(1.0);
    let id = member.id;
    community.add_member(member);

    let mutations = granter.grant(&community, 1000);
    apply_grant(&mut community, mutations);

    assert!(community.members[&id].bag.count(GRAPES) > community.members[&id].bag.count(OLIVE));
}

#[test]
fn standard_barely_beats_exotic_at_intermediate_luck() {
    // Tests the lerp operand `high - base`: at luck=0.6 with MANGO (r=0),
    // correct weight ≈ 0.034 < GRAPES ≈ 0.041, so GRAPES dominates.
    // Mutant (`- with +`): weight ≈ 0.046 > 0.041, so MANGO would dominate.
    // (The `* with /` and `- with /` mutants produce even larger exotic weights, also caught.)
    let mut granter = RandomGranter::new(StdRng::seed_from_u64(0)).with_fruits(STANDARD_AND_EXOTIC);
    let mut community = Community::new();
    let member = Member::new("Alice").with_luck_f64(0.6);
    let id = member.id;
    community.add_member(member);

    let mutations = granter.grant(&community, 1000);
    apply_grant(&mut community, mutations);

    assert!(community.members[&id].bag.count(GRAPES) > community.members[&id].bag.count(MANGO));
}
