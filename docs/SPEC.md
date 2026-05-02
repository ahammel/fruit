# fruit — Technical Specification

## Overview

fruit is a sharing economy simulation game. Players belong to a **community** and
hold a **bag** of **fruits**. At each game tick every member receives new fruits drawn
by weighted-random selection; the weights are shaped by the member's and community's
**luck** scores.

The game rewards generosity and communal contribution. Actions that raise luck:
gifting fruit to another player (+personal luck) and burning fruit (+community luck).
Actions that lower luck: hoarding fruit without gifting or burning (−personal luck,
−community luck), ostentatious actions — gifts or burns that exceed what the recipient
or community average could plausibly match (−personal luck), and quid-pro-quo trades
(−community luck).

---

## Architecture

The project follows Domain-Driven Design and Hexagonal Architecture, structured as a
Cargo workspace with three crates:

```
fruit (workspace)
├── domain/               # Pure domain logic; no I/O
├── in_memory_db/         # Implements domain storage ports in RAM
├── dynamo_db/            # Implements domain storage ports against Amazon DynamoDB
└── command_line_service/ # Wires everything together; hosts the REPL
```

### Dependency rule

All arrows point toward `domain`. `domain` has no outward dependencies on internal
crates. `in_memory_db`, `dynamo_db`, and `command_line_service` depend on `domain`;
only `command_line_service` depends on `in_memory_db`.

---

## Domain Model (`domain` crate)

### Identifiers (`id.rs`)

```rust
pub trait UuidIdentifier: Debug + Clone + Copy + PartialEq + Eq + Hash {
    fn new() -> Self;          // generates a random UUID v4
    fn as_uuid(&self) -> Uuid;
}

pub trait IntegerIdentifier: Debug + Clone + Copy + PartialEq + Eq + Hash + Ord + PartialOrd {
    fn zero() -> Self;         // sentinel value (not a valid sequence position)
    fn from_u64(id: u64) -> Self;
    fn as_u64(&self) -> u64;
}
```

All entity IDs are newtype wrappers around `Uuid` that implement `UuidIdentifier`.
Currently defined: `CommunityId`, `MemberId`.

### Fruit value (`fruit.rs`)

A scalar **value** is defined for each fruit and is used by luck-adjustment calculations:

```
value(fruit) = category_base(fruit.category) × (1.0 + fruit.rarity())
```

| Category | `category_base` |
|----------|-----------------|
| Standard | 1.0 |
| Rare | 3.0 |
| Exotic | 10.0 |

The value range is [1.0, 2.0] for Standard, [3.0, 6.0] for Rare, and [10.0, 20.0] for Exotic.
Implemented as `Fruit::value() -> f64`.

### Bag value

The **value** of a bag is the sum of the values of all fruits it contains:

```
bag_value(bag) = Σ value(fruit) × count(fruit)   for each distinct fruit in bag
```

The **community average bag value** is the mean of all members' individual bag values:

```
community_avg = Σ bag_value(member.bag) / member_count
```

Both are implemented as free functions in `bag.rs` and `community.rs` respectively.

### Fruit (`fruit.rs`)

```rust
pub enum Category { Standard, Rare, Exotic }  // derives Ord

pub struct Fruit {
    pub name:     &'static str,
    pub emoji:    &'static str,
    pub category: Category,
    _rarity:      u8,          // private; access via rarity()
}

impl Fruit {
    pub fn rarity(&self) -> f64  // _rarity / u8::MAX, in [0.0, 1.0]
}
```

`Fruit` derives `Debug, Clone, Copy, PartialEq, Eq, Hash`. Equality and hashing use
all fields (not emoji alone), so two fruits with the same emoji but different rarity are
distinct.

**Fruit pool** — 26 static constants split across categories.
Within-category rarity values are evenly spaced across the `u8` range:

| Category | Count | `_rarity` values |
|----------|-------|-------------------|
| Standard | 9 | 0, 32, 64, 96, 128, 159, 191, 223, 255 |
| Rare | 9 | 0, 32, 64, 96, 128, 159, 191, 223, 255 |
| Exotic | 8 | 0, 36, 73, 109, 146, 182, 219, 255 |

`FRUITS: &[Fruit]` lists all 26 constants ordered by category (Standard → Rare →
Exotic) then ascending rarity within each category.

### Bag (`bag.rs`)

```rust
pub struct Bag { counts: HashMap<Fruit, usize> }
```

A multiset of fruits. Key operations:

| Method | Description |
|--------|-------------|
| `insert(fruit) -> Self` | Adds one instance; builder-style |
| `remove(fruit) -> bool` | Removes one instance; returns false if absent |
| `count(fruit) -> usize` | Count of a specific fruit |
| `total() -> usize` | Sum of all counts |
| `is_empty() -> bool` | True when no fruits held |
| `iter()` | Yields `(Fruit, usize)` pairs |

### Member (`member.rs`)

```rust
pub struct Member {
    pub id:           MemberId,
    pub display_name: String,
    _luck:            u8,     // private; access via luck() / with_luck[_f64]()
    pub bag:          Bag,
}
```

Builders:

| Method | Description |
|--------|-------------|
| `new(display_name)` | Random ID, neutral luck (0), empty bag |
| `with_id(MemberId)` | Override ID |
| `with_bag(Bag)` | Override bag |
| `with_luck(u8)` | Set raw luck |
| `with_luck_f64(f64)` | Set luck from normalised float; round-trip not guaranteed |

Getters:

| Method | Returns |
|--------|---------|
| `luck() -> f64` | `_luck / u8::MAX`, in `[0.0, 1.0]` |

Mutation:

| Method | Description |
|--------|-------------|
| `receive(fruit) -> &mut Self` | Adds one fruit to the bag |

### Community (`community.rs`)

```rust
pub struct Community {
    pub id:      CommunityId,
    _luck:       u8,                       // private
    pub members: HashMap<MemberId, Member>,
    pub version: SequenceId,               // last applied effect; zero = no effects applied
}
```

Builders:

| Method | Description |
|--------|-------------|
| `new()` | Random ID, neutral luck, no members, version zero |
| `with_id(CommunityId)` | Override ID |
| `with_luck(u8)` | Set raw luck |
| `with_luck_f64(f64)` | Set luck from normalised float; round-trip not guaranteed |
| `with_version(SequenceId)` | Override version; useful when reconstituting from storage |

Getters:

| Method | Returns |
|--------|---------|
| `luck() -> f64` | `_luck / u8::MAX`, in `[0.0, 1.0]` |

Mutation:

| Method | Returns | Description |
|--------|---------|-------------|
| `add_member(Member) -> bool` | `true` if inserted, `false` if ID already present |
| `remove_member(MemberId) -> Option<Member>` | Removed member, or `None` |
| `apply_effects(impl IntoIterator<Item = Effect>)` | Applies effects in order; advances `version` to the last effect's ID |

The `version` field doubles as the community snapshot marker: it records the sequence
ID of the last `Effect` that was folded into this instance. A `version` of
`SequenceId::zero()` means the community reflects only its initial state; no effects
have been applied.

### Shared identity traits (`community.rs`, `event_log.rs`)

```rust
pub trait HasCommunityId {
    fn community_id(&self) -> CommunityId;
}

pub trait HasSequenceId {
    fn sequence_id(&self) -> SequenceId;
}
```

`Event`, `Effect`, and `Record` all implement both traits.

### Luck normalisation

Both `Member` and `Community` store luck as a `u8` in `[0, 255]` but expose it as
`f64` in `[0.0, 1.0]` via the `luck()` getter:

```
luck() = _luck as f64 / u8::MAX as f64
```

`with_luck_f64(v)` performs the inverse:

```
_luck = (v * u8::MAX as f64).round() as u8
```

Round-trips are not exact because not every `f64` in `[0,1]` maps to a distinct `u8`.

### Event log (`event_log.rs`)

```rust
pub struct SequenceId(u64);   // implements IntegerIdentifier; zero() is a sentinel

pub struct Record {           // a single log entry: an event paired with its effect
    pub event:  Event,
    pub effect: Option<Effect>,  // None if the event has not yet been processed
}

pub enum EventPayload {
    Grant { count: usize },
    /// `member_id` is generated by the caller before the event is appended so
    /// the effect can be recomputed deterministically on retry.
    AddMember { display_name: String, member_id: MemberId },
    RemoveMember { member_id: MemberId },
    SetCommunityLuck { luck: u8 },
    SetMemberLuck { member_id: MemberId, luck: u8 },
    /// Transfer one fruit from sender to recipient.
    /// `message` is an optional free-text note from the sender; purely cosmetic,
    /// with no effect on luck calculations.
    Gift {
        sender_id: MemberId,
        recipient_id: MemberId,
        fruit: Fruit,
        message: Option<String>,
    },
    /// Destroy one or more fruits held by `member_id`.
    ///
    /// `fruits` may contain duplicates and may span multiple fruit types.
    /// If the member does not hold enough of a particular fruit, as many as they
    /// hold are burned and the remainder of that type is silently skipped.
    Burn {
        member_id: MemberId,
        fruits: Vec<Fruit>,
    },
}

pub struct Event {
    pub id:           SequenceId,
    pub community_id: CommunityId,
    pub payload:      EventPayload,
}

pub enum StateMutation {
    /// Add one fruit to a member's bag.
    AddFruitToMember { member_id: MemberId, fruit: Fruit },
    /// Remove one fruit from a member's bag. No-op if the member does not hold the fruit.
    RemoveFruitFromMember { member_id: MemberId, fruit: Fruit },
    /// Add a member to the community.
    AddMember { member: Member },
    /// Remove a member from the community.
    RemoveMember { member_id: MemberId },
    /// Overwrite the community's raw luck value.
    SetCommunityLuck { luck: u8 },
    /// Overwrite a member's raw luck value.
    SetMemberLuck { member_id: MemberId, luck: u8 },

    // ── Luck adjustments (computed at grant time) ──────────────────────────────
    //
    // Each carries a signed `delta: i16`. Applied as:
    //   new_raw = (current_raw as i16 + delta).clamp(0, 255) as u8
    //
    // Positive deltas increase luck; negative deltas decrease it.
    // Using i16 avoids overflow when summing multiple adjustments.

    /// +personal luck for the member who performed a gift since the last grant.
    /// Proportional to the total value of all fruits they gifted.
    GiftLuckBonus { member_id: MemberId, delta: i16 },

    /// +community luck for burns performed since the last grant.
    /// Proportional to the total value of all fruits burned.
    BurnLuckBonus { delta: i16 },

    /// −personal luck penalty for an ostentatious gift (gift value greatly exceeded
    /// the recipient's bag value at the time of the gift).
    OstentatiousGiftPenalty { member_id: MemberId, delta: i16 },

    /// −personal luck penalty for an ostentatious burn (burn value greatly exceeded
    /// the community average bag value at the time of the burn).
    OstentatiousBurnPenalty { member_id: MemberId, delta: i16 },

    /// −community luck penalty for quid-pro-quo gifting behaviour detected across
    /// the 100 most recent gift events.
    QuidProQuoPenalty { delta: i16 },
}

pub struct Effect {
    pub id:           SequenceId,   // same as the Event's ID
    pub community_id: CommunityId,
    pub mutations:    Vec<StateMutation>,
}
```

`EventPayload` and `StateMutation` derive `Debug, Clone, PartialEq, Eq`.
`Event`, `Effect`, and `Record` derive `Debug, Clone, PartialEq, Eq`.

`Effect::apply` handles all `StateMutation` variants:

**Bag mutations**
- `AddFruitToMember` — calls `member.receive(fruit)`; silently skips absent members.
- `RemoveFruitFromMember` — calls `member.bag.remove(fruit)`; silently skips absent members.

**Membership mutations**
- `AddMember` — calls `community.add_member(member.clone())`.
- `RemoveMember` — calls `community.remove_member(member_id)`.

**Absolute luck overrides**
- `SetCommunityLuck` — sets the community's raw luck value directly.
- `SetMemberLuck` — sets a member's raw luck value; silently skips absent members.

**Delta luck adjustments** (applied with saturating arithmetic)

Each delta variant applies: `new_raw = (current_raw as i16 + delta).clamp(0, 255) as u8`.

- `GiftLuckBonus` — adjusts member personal luck; silently skips absent members.
- `BurnLuckBonus` — adjusts community luck.
- `OstentatiousGiftPenalty` — adjusts member personal luck (delta is negative); silently skips absent members.
- `OstentatiousBurnPenalty` — adjusts member personal luck (delta is negative); silently skips absent members.
- `QuidProQuoPenalty` — adjusts community luck (delta is negative).

### Gift computation (`gifter.rs`)

```rust
pub fn compute_gift(
    community: &Community,
    sender_id: MemberId,
    recipient_id: MemberId,
    fruit: Fruit,
) -> Vec<StateMutation>
```

Returns `[RemoveFruitFromMember { sender_id }, AddFruitToMember { recipient_id }]` if the
sender holds the fruit, or an empty `Vec` (no-op) if the sender is unknown or does not
hold the fruit.

### Burn computation (`burner.rs`)

```rust
pub fn compute_burn(
    community: &Community,
    member_id: MemberId,
    fruits: &[Fruit],
) -> Vec<StateMutation>
```

Accepts a slice of fruits (duplicates allowed). For each distinct fruit type, burns
`min(requested, held)` instances. Returns one `RemoveFruitFromMember` per fruit actually
burned. Returns an empty `Vec` if the member is unknown, `fruits` is empty, or no
requested fruits are held.

### Luck adjustments (`luck_adjustments.rs`)

Pure computation layer. Accepts pre-fetched event slices and returns mutations; performs
no I/O.

```rust
pub fn compute(
    community_at_last_grant: &Community,
    records_since_last_grant: &[Record],
    recent_gift_records: &[Record],
) -> Vec<StateMutation>
```

The inputs are:

- `community_at_last_grant` — the community snapshot at the moment the previous `Grant`
  effect was applied (version = previous grant's `SequenceId`). Used as the starting
  state for replaying `records_since_last_grant` to derive intermediate bag values for
  ostentation calculations. Pass the initial community (version zero) if no prior grant
  exists.
- `records_since_last_grant` — all `Record`s whose sequence ID falls strictly between the
  previous grant and the current one. Only records with a non-empty effect are
  considered for luck calculations; no-op records (e.g. a gift where the sender did not
  hold the fruit) are ignored.
- `recent_gift_records` — up to the 100 most recent `Gift` `Record`s across all time,
  used for quid-pro-quo detection. Only records with non-empty effects are counted.

**Ostentation values are computed by replaying `records_since_last_grant`** against a
mutable clone of `community_at_last_grant`. Before applying each `Gift` or `Burn`
record's effect, the current running state provides the recipient's bag value or the
community average bag value needed for the ostentation check.

#### Gift luck bonus (`GiftLuckBonus`)

For each member who appears as `sender_id` in at least one `Gift` event in
`since_last_grant`:

```
total_gift_value = Σ value(fruit)   for each Gift by this member
delta = (total_gift_value / GIFT_LUCK_SCALE).round() as i16   (clamped to i16::MAX)
```

One `GiftLuckBonus { member_id, delta }` mutation is emitted per qualifying member.

#### Burn luck bonus (`BurnLuckBonus`)

```
total_burn_value = Σ value(fruit)   for all Burn events in since_last_grant
                                    (only fruits actually burned, i.e. min(requested, held))
delta = (total_burn_value / BURN_LUCK_SCALE).round() as i16   (clamped to i16::MAX)
```

One `BurnLuckBonus { delta }` mutation is emitted if `delta > 0`.

#### Ostentatious gift penalty (`OstentatiousGiftPenalty`)

For each `Gift` record in `records_since_last_grant` with a non-empty effect, the
recipient's bag value is read from the running community state **before** the effect is
applied:

```
recipient_bag_value = bag_value(running_state.members[recipient_id].bag)
excess = value(fruit) - OSTENTATION_RATIO × recipient_bag_value
```

If `excess > 0.0`:

```
delta = -(excess / OSTENTATION_SCALE).round() as i16   (clamped to i16::MIN)
```

One `OstentatiousGiftPenalty { member_id: sender_id, delta }` mutation is emitted per
qualifying gift. Multiple ostentatious gifts by the same member produce multiple
mutations; `Effect::apply` applies them in sequence with saturating arithmetic.

#### Ostentatious burn penalty (`OstentatiousBurnPenalty`)

For each `Burn` record in `records_since_last_grant` with a non-empty effect, the
community average is read from the running community state **before** the effect is
applied:

```
community_avg = community_avg_bag_value(running_state)
total_burn_value = Σ value(fruit)   for fruits in the effect's RemoveFruitFromMember mutations
excess = total_burn_value - OSTENTATION_RATIO × community_avg
```

If `excess > 0.0`:

```
delta = -(excess / OSTENTATION_SCALE).round() as i16
```

One `OstentatiousBurnPenalty { member_id, delta }` mutation is emitted per qualifying
burn record. Using the effect's mutations (not the event's `fruits` field) gives the
actually-burned quantity rather than the requested quantity.

#### Quid-pro-quo penalty (`QuidProQuoPenalty`)

A **reciprocal gift pair** is a pair of `Gift` events from `recent_gifts` where one goes
A→B and another goes B→A. A pair is **quasi-symmetrical** if the values are similar but
not equal:

```
let va = value(a_to_b.fruit)
let vb = value(b_to_a.fruit)
is_quasi_symmetrical =
    va != vb
    AND |va - vb| / va.max(vb) < QP_SIMILARITY_THRESHOLD
```

Each gift in `recent_gifts` is matched against every gift going in the opposite direction
between the same two members. Pairs are counted with repetition (one gift can form a
pair with multiple reciprocal gifts).

```
quasi_symmetrical_count = count of quasi-symmetrical reciprocal gift pairs in recent_gifts
ratio = quasi_symmetrical_count as f64 / recent_gifts.len().max(1) as f64
delta = -(ratio * QP_MAX_PENALTY).round() as i16
```

One `QuidProQuoPenalty { delta }` mutation is emitted if `delta < 0`.

#### Tuning constants

These constants are defined in `luck_adjustments.rs` and are subject to balance tuning:

| Constant | Initial value | Role |
|----------|--------------|------|
| `GIFT_LUCK_SCALE` | 10.0 | Divisor converting total gift value to luck delta |
| `BURN_LUCK_SCALE` | 10.0 | Divisor converting total burn value to luck delta |
| `OSTENTATION_RATIO` | 2.0 | Multiplier applied to baseline before measuring excess |
| `OSTENTATION_SCALE` | 5.0 | Divisor converting excess value to luck delta |
| `QP_SIMILARITY_THRESHOLD` | 0.2 | Max relative value difference to count as quasi-symmetrical |
| `QP_MAX_PENALTY` | 64.0 | Maximum quid-pro-quo community luck penalty (as raw u8 delta) |

### Luck adjuster (`luck_adjuster.rs`)

I/O layer that fetches the event slices needed by `luck_adjustments::compute` and
returns the computed mutations.

```rust
pub struct LuckAdjuster<ELP: EventLogProvider, CP: CommunityProvider> {
    event_log: ELP,
    community_provider: CP,
}

impl<ELP: EventLogProvider, CP: CommunityProvider> LuckAdjuster<ELP, CP> {
    pub fn new(event_log: ELP, community_provider: CP) -> Self;

    /// Fetches event history and computes all luck-adjustment mutations for a grant.
    ///
    /// `before` is the `SequenceId` of the `Grant` event just recorded; only records
    /// strictly before this ID are considered "since the last grant".
    pub fn compute(&self, community: &Community, before: SequenceId) -> Result<Vec<StateMutation>, Error>;
}
```

`compute` internally:
1. Calls `get_latest_grant_events(community.id, 1)` to find the previous grant's
   `SequenceId` (`prev_grant_id`); uses `SequenceId::zero()` if none exists.
2. Calls `community_provider.get(community.id, prev_grant_id)` to fetch the community
   snapshot at the last grant boundary. Falls back to the initial community (version
   zero) if the snapshot is absent.
3. Calls `get_records_between(community.id, prev_grant_id, before)` →
   `records_since_last_grant`.
4. Calls `get_latest_gift_records(community.id, 100)` → `recent_gift_records`.
5. Delegates to `luck_adjustments::compute(&community_at_last_grant, &records_since_last_grant, &recent_gift_records)`.

### Granter port (`granter.rs`)

```rust
pub trait Granter {
    fn grant(&mut self, community: &Community, count: usize) -> Vec<StateMutation>;
}
```

Computes `AddFruitToMember` mutations by distributing `count` fruits to each member of
`community` using weighted-random selection. Expects a community whose luck values already
reflect any adjustments for this grant cycle — the `Drop` struct (see below) is responsible
for providing that. The caller records the returned mutations as part of an `Effect` and
applies them.

### Idempotent effect computation

Every event → effect pair is designed to be safely retried if the process crashes between
appending the event and writing the effect:

- All `EventPayload` variants carry enough information to deterministically recompute their
  `Effect` without any additional random state. In particular, `AddMember` includes
  `member_id` so the member UUID is fixed at event-append time rather than re-generated on
  retry.
- Callers that have a known event ID can call `get_effect_for_event` to check whether an
  effect already exists before computing a new one.
- `Providence::grant_fruit` implements this pattern automatically: if the most recent
  `Grant` event for the community has no corresponding effect (orphaned), it resumes that
  event rather than appending a new one.

### Providence (`providence.rs`)

`Providence` is the domain-level orchestrator of a grant cycle. It combines a
`LuckAdjuster` and a `Granter` and owns the logic for how they work together.

```rust
pub struct Providence<ELP: EventLogProvider, CP: CommunityProvider, G: Granter> {
    luck_adjuster: LuckAdjuster<ELP, CP>,
    granter: G,
}

impl<ELP: EventLogProvider, CP: CommunityProvider, G: Granter> Providence<ELP, CP, G> {
    pub fn new(luck_adjuster: LuckAdjuster<ELP>, granter: G) -> Self;

    /// Computes the full set of state mutations for one grant cycle.
    ///
    /// `grant_event_id` is the `SequenceId` of the `Grant` event already appended to the
    /// log. Returns luck-adjustment mutations followed by fruit-grant mutations, in that
    /// order, as a single flat `Vec` ready to be recorded as one `Effect`.
    pub fn grant_fruit(
        &mut self,
        community: &Community,
        count: usize,
        grant_event_id: SequenceId,
    ) -> Result<Vec<StateMutation>, Error>;
}
```

`grant_fruit` implementation:

1. `luck_adjuster.compute(community, grant_event_id)` → `luck_mutations`.
2. Clone `community` and apply `luck_mutations` to the clone → `luck_adjusted`.
3. `granter.grant(&luck_adjusted, count)` → `fruit_mutations`.
4. Return `Ok(luck_mutations + fruit_mutations)`.

### Fruit weights (`fruit_weights.rs`)

`FruitWeights` is a strategy trait for computing a `WeightedIndex<f64>` over a fruit
pool given an effective luck value:

```rust
pub trait FruitWeights {
    fn fruit_weights(&self, fruits: &[Fruit], luck: f64) -> WeightedIndex<f64>;
}
pub struct DefaultFruitWeights;
impl FruitWeights for DefaultFruitWeights {}  // uses the formula below
```

**Default weight formula** (where `r = fruit.rarity()` ∈ `[0.0, 1.0]`,
`l = luck` ∈ `[0.0, 2.0]`, `tier(r) = 1 + 2·r`):

```
Standard : tier(r) × 10 / (1 + 2·l)
Rare     : tier(r) × (1 + l/2)
Exotic   : tier(r) × 0.125 × (1 + l)²
```

Weights are floored at `f64::EPSILON` so no fruit is ever excluded from sampling.

The `tier(r)` factor gives an exact **3:1 ratio** between the max-rarity (r=1) and
min-rarity (r=0) fruit within any category at any luck value.

**Approximate category drop-shares:**

| Luck | Standard | Rare | Exotic |
|------|----------|------|--------|
| 0.0  | ≈ 90%    | ≈ 9% | ≈ 1%   |
| 2.0  | ≈ 40%    | ≈ 40%| ≈ 20%  |

### Random granter (`random_granter.rs`)

`RandomGranter<R: Rng, W: FruitWeights = DefaultFruitWeights>` implements `Granter`
using weighted-random selection and an injectable weight strategy. It has no knowledge
of the event log; luck adjustments are applied by the caller before invoking `grant`.

**Effective luck** for a member is the sum of member and community luck (each in
`[0.0, 1.0]`):

```
luck = member.luck() + community.luck()   // ∈ [0.0, 2.0]
```

Construction:

```rust
RandomGranter::new(rng)                       // full FRUITS pool, DefaultFruitWeights
    .with_fruits(&[GRAPES, STRAWBERRY])        // restrict pool (panics if empty)
    .with_weights(custom_weights)              // substitute a FruitWeights strategy
```

### Storage ports (`community_repo.rs`)

All port traits are `async` (via `async_trait`). All methods take `&self` (not `&mut self`);
implementations manage interior mutability (e.g. via a connection pool or mutex). Return
types use `Exn<Self::Error>` where `Self::Error: DbError` (see [Error handling](#error-handling)).

```rust
pub trait CommunityProvider {
    type Error: DbError;
    async fn get(&self, id: CommunityId, version: SequenceId) -> Result<Option<Community>, Exn<Self::Error>>;
    async fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Exn<Self::Error>>;
}

pub trait CommunityPersistor {
    type Error: DbError;
    async fn put(&self, community: Community) -> Result<Community, Exn<Self::Error>>;
}

pub trait CommunityRepo: CommunityProvider + CommunityPersistor<Error = <Self as CommunityProvider>::Error> {}
```

Every community write is a new snapshot version. There is no overwrite/upsert operation:
communities are always advanced by appending a new version via `put`.

### Event log ports (`event_log_repo.rs`)

```rust
pub trait EventLogProvider {
    type Error: DbError;
    /// Returns the log entry at `id` for `community_id`, or `None` if not found.
    async fn get_record(&self, community_id: CommunityId, id: SequenceId) -> Result<Option<Record>, Exn<Self::Error>>;
    /// Returns the effect at `event_id`, or `None` if the event has not yet been processed.
    async fn get_effect_for_event(&self, community_id: CommunityId, event_id: SequenceId) -> Result<Option<Effect>, Exn<Self::Error>>;
    /// Returns up to `limit` effects with sequence ID > `after`, ascending. Keyset cursor: pass `SequenceId::zero()` to start from the beginning.
    async fn get_effects_after(&self, community_id: CommunityId, limit: usize, after: SequenceId) -> Result<Vec<Effect>, Exn<Self::Error>>;
    /// Returns up to `limit` records with sequence ID < `before`, descending. Pass `None` to start from the most recent.
    async fn get_records_before(&self, community_id: CommunityId, limit: usize, before: Option<SequenceId>) -> Result<Vec<Record>, Exn<Self::Error>>;
    /// Returns up to `limit` Grant events, sorted descending (most recent first).
    async fn get_latest_grant_events(&self, community_id: CommunityId, limit: usize) -> Result<Vec<Event>, Exn<Self::Error>>;
    /// Returns up to `limit` Gift records, sorted descending (most recent first).
    async fn get_latest_gift_records(&self, community_id: CommunityId, limit: usize) -> Result<Vec<Record>, Exn<Self::Error>>;
    /// Returns all records with sequence ID strictly between `after` and `before`, ascending.
    async fn get_records_between(&self, community_id: CommunityId, after: SequenceId, before: SequenceId) -> Result<Vec<Record>, Exn<Self::Error>>;
}

pub trait EventLogPersistor {
    type Error: DbError;
    /// Assigns the next sequence ID and stores the event.
    async fn append_event(&self, community_id: CommunityId, payload: EventPayload) -> Result<Event, Exn<Self::Error>>;
    /// Stores an effect whose `id` equals `event_id`. Returns an error if an effect already exists.
    async fn append_effect(&self, event_id: SequenceId, community_id: CommunityId, mutations: Vec<StateMutation>) -> Result<Effect, Exn<Self::Error>>;
}

pub trait EventLogRepo: EventLogProvider + EventLogPersistor<Error = <Self as EventLogProvider>::Error> {}
```

`get_effects_after` uses keyset pagination: if the returned slice length equals `limit`,
there may be more results — paginate by passing the last returned effect's ID as the
next `after`. Useful for replaying effects since a known snapshot.

### Store wrappers (`community_store.rs`, `event_log_store.rs`)

```rust
pub struct CommunityStore<CR: CommunityRepo, ELP: EventLogProvider> { ... }
```

| Method | Description |
|--------|-------------|
| `new(repo, event_log_provider)` | Construct with any `CommunityRepo` and `EventLogProvider` |
| `init() -> Result<Community, Error>` | Create and persist a new community at version zero |
| `get(id, version) -> Result<Option<Community>, Error>` | Fetch a specific snapshot |
| `get_latest(id) -> Result<Option<Community>, Error>` | Fetch the latest snapshot, folding in any unapplied effects from the event log and persisting the result |

```rust
pub struct EventLogStore<ELR: EventLogRepo> { ... }
```

Thin wrapper that exposes all `EventLogProvider` and `EventLogPersistor` methods
without requiring callers to depend on the port traits directly.

### Error handling (`error.rs`)

The domain defines a `DbError` marker trait:

```rust
pub trait DbError: Anomaly + Send + Sync + 'static {}
```

Each db crate defines its own `Error` enum (using `thiserror` and `anomalies`) and
implements `DbError` on it. Storage port traits carry an associated `type Error: DbError`
so that callers can inspect the [`anomalies`](https://crates.io/crates/anomalies)
`category` and `status` to make retry decisions without parsing message strings.

The domain's own `Error` enum is used by service-layer code (e.g. `LuckAdjuster`).
It wraps db errors via `StorageLayerError::raise`, which preserves the original category
and status from the db error through the `Exn` causality chain.

All failable operations return `Result<T, Exn<E>>` where
[`Exn`](https://crates.io/crates/exn) wraps the error with call-site location and a
causal chain. Crossing error-type boundaries (e.g. from a db `Error` to the domain
`Error`) requires an explicit conversion, which forces a conscious framing decision at
each boundary.

---

## In-Memory Database (`in_memory_db` crate)

`InMemoryCommunityRepo` implements `CommunityRepo` using a
`RwLock<HashMap<CommunityId, BTreeMap<SequenceId, Community>>>`. Each community maps to
a `BTreeMap` of versioned snapshots keyed by `SequenceId`; `get_latest` returns the
entry with the greatest key.

`InMemoryEventLogRepo` implements `EventLogRepo` using two separate
`RwLock<HashMap>`s — one for events (keyed by sequence ID) and one for effects (also
keyed by sequence ID, which equals the event's ID) — plus a shared `AtomicU64` sequence
counter that is incremented only when appending events. The three new provider methods
(`get_latest_grant_events`, `get_latest_gift_records`, `get_records_between`) scan the
events and effects `HashMap`s linearly and sort/filter/join in memory; no additional
index is maintained.

For both types:
- Reads acquire a shared read lock.
- Writes acquire an exclusive write lock.
- A poisoned lock propagates as a domain `Error`.
- Both implement `EventLogProvider`/`EventLogPersistor` for both owned and `&`-reference
  types so they can be shared between a `CommunityStore` and an `EventLogStore` without
  cloning.

---

## DynamoDB Database (`dynamo_db` crate)

`DynamoDbEventLogRepo` implements `EventLogRepo` and `DynamoDbCommunityRepo` implements
`CommunityRepo`, both backed by a single DynamoDB table using a composite key design.

### Table schema

| Attribute | Type | Description |
|-----------|------|-------------|
| `pk` | Binary (16 bytes) | Community UUID in raw bytes |
| `sk` | String | Sort key — encodes entity type and position |

All item types share the same table. The sort key prefix determines the entity type:

| Sort key format | Entity |
|-----------------|--------|
| `COUNTER` | Per-community sequence counter |
| `EVENT#{seq:020}` | Event item (seq zero-padded to 20 digits) |
| `EFFECT#{seq:020}` | Effect item |
| `COMMUNITY#{seq:020}` | Community snapshot at the given version |

UUID fields (`pk` and all member/sender/recipient IDs inside items) are stored as 16-byte
binary blobs (`AttributeValue::B`) rather than 36-character hyphenated strings, reducing
item size and per-request cost. Binary values appear as Base64 in the AWS console and CLI.

### Sequence counter

Each community has its own counter item at `pk = community_id, sk = "COUNTER"`. The
counter is incremented atomically via `UpdateItem ADD`, which guarantees each
`append_event` call receives a distinct value. Sequence IDs are community-scoped; the
same numeric ID may appear in different communities.

Gaps in the sequence are acceptable: if a process crashes after incrementing the counter
but before writing the event item, the counter advances but no item is written.

### Consistency model

`append_event` and `append_effect` use `PutItem` with `condition_expression =
"attribute_not_exists(sk)"`. DynamoDB conditional writes are atomic — the condition check
and the write happen as a single operation on the most recent version of the item. Two
concurrent callers at the same sort key will get exactly one success and one
`ConditionalCheckFailedException` (mapped to `Error::AlreadyExists`). Because the counter
guarantees distinct sequence IDs under normal operation, `AlreadyExists` from
`append_event` is not expected in practice.

### No GSI required

All queries use the main table's partition key (`pk = community_id`). There is no Global
Secondary Index. `get_record` and `get_effect_for_event` take `community_id` as a
parameter for this reason.

---

## Command-Line Service (`command_line_service` crate)

A terminal REPL (`repl::run()`) for interactive testing of the game loop.

**Start-up**: creates one community backed by `InMemoryCommunityRepo` and
`InMemoryEventLogRepo`; constructs a `Providence` wrapping a `LuckAdjuster` (backed by
both the event log repo and the community repo) and a `RandomGranter` (seeded from
`rand::thread_rng()`).

**Grant flow** (executed when the `grant` command is issued):

1. Append a `Grant { count }` event → `grant_event`.
2. `drop.grant_fruit(&community, count, grant_event.id)?` → `mutations`.
3. Append one effect containing `mutations`.
4. Apply the effect to the live community.

**Commands**:

| Command | Description |
|---------|-------------|
| `add <name>` | Add a member (recorded as `AddMember` event + effect) |
| `remove <name>` | Remove a member by display name (recorded as `RemoveMember` event + effect) |
| `grant <count>` | Grant N fruits to every member; computes luck adjustments (recorded as `Grant` event + effect) |
| `gift <from_name> <to_name> <emoji>` | Give one fruit from sender to recipient (recorded as `Gift` event + effect); no message support in the REPL |
| `burn <name> <emoji> [<emoji> ...]` | Destroy one or more fruits held by a member (recorded as `Burn` event + effect) |
| `luck <value>` | Set community luck (float in `[0.0, 1.0]`) |
| `luck <name> <value>` | Set a member's luck |
| `log <n>` | Show the N most recent log records |
| `help` | Show command list |
| `quit` / `exit` | Exit |

**Display**: the screen is cleared before each prompt. Members are listed alphabetically;
their bags are sorted by category then rarity. Luck values are shown to 3 decimal places.

---

## Events and Effects

Player actions are recorded as immutable Events; state changes are derived from their
corresponding Effects. See [docs/EventsAndEffects.md](EventsAndEffects.md) for the full
design: shared ID sequence, data model, processing flow, and interleaving behaviour.

---

## Testing

- 100% line and region coverage enforced via `cargo llvm-cov` (`make tc`).
- Tests prefer whole-object assertions (`assert_eq!(actual, expected_struct)`) over
  field-by-field checks.
- Failure paths (panic messages, error branches) are covered by `#[should_panic]` tests
  and poisoned-lock helper fixtures.
- Statistical distribution tests (`StdRng::seed_from_u64(0)`, 1000 grants) verify that
  luck shifts drop probabilities in the expected direction; update assertions if the weight
  formula or crossover points change.

---

## Conventions

### Numeric types for game scores

| Score | Storage | Getter return | Setter |
|-------|---------|---------------|--------|
| Rarity | `u8` (`_rarity`) | `f64` via `u8::MAX` | struct literal only |
| Luck | `u8` (`_luck`) | `f64` via `u8::MAX` | `with_luck(u8)` / `with_luck_f64(f64)` |

Private fields are prefixed `_` to signal that access should go through the getter.
Round-trips through `with_luck_f64` → `luck()` are approximate, not exact; this is
documented on each method.

### Port naming

| Suffix | Role |
|--------|------|
| `XxxProvider` | Read port; `&self` |
| `XxxPersistor` | Write port; `&self` with interior mutability |
| `XxxRepo` | Combined Provider + Persistor |
| `XxxStore` | Service-layer wrapper over a Repo |

### Pre-commit hook order

`cargo check` → `cargo fmt --check` → `cargo clippy`

---

## Slack Integration

See [docs/spec/Slack.md](spec/Slack.md) for the Slack workspace integration specification,
including design decisions, interaction model, and AWS architecture.
Tests are **not** run in the pre-commit hook (run `make t` or `make tc` manually).
