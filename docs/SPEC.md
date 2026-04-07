# fruit — Technical Specification

## Overview

fruit is a sharing economy simulation game. Players belong to a **community** and
hold a **bag** of **fruits**. At each game tick every member receives new fruits drawn
by weighted-random selection; the weights are shaped by the member's and community's
**luck** scores.

The game rewards generosity and communal contribution. Actions that raise luck:
gifting fruit to another player (+personal luck) and burning fruit (+community luck).
Actions that lower luck: hoarding fruit without gifting or burning (−community luck),
ostentatious gifts (−personal luck), and quid-pro-quo trades (−community luck).

---

## Architecture

The project follows Domain-Driven Design and Hexagonal Architecture, structured as a
Cargo workspace with three crates:

```
fruit (workspace)
├── domain/               # Pure domain logic; no I/O
├── in_memory_db/         # Implements domain storage ports in RAM
└── command_line_service/ # Wires everything together; hosts the REPL
```

### Dependency rule

All arrows point toward `domain`. `domain` has no outward dependencies on internal
crates. `in_memory_db` and `command_line_service` depend on `domain`; only
`command_line_service` depends on `in_memory_db`.

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

### Fruit (`fruit.rs`)

```rust
pub enum Category { Standard, Rare, Exotic }  // derives Ord

pub struct Fruit {
    pub name:     &'static str,
    pub emoji:    &'static str,
    pub category: Category,
    _rarity:      u16,         // private; access via rarity()
}

impl Fruit {
    pub fn rarity(&self) -> f64  // _rarity / u16::MAX, in [0.0, 1.0]
}
```

`Fruit` derives `Debug, Clone, Copy, PartialEq, Eq, Hash`. Equality and hashing use
all fields (not emoji alone), so two fruits with the same emoji but different rarity are
distinct.

**Fruit pool** — 26 static constants split across categories.
Within-category rarity values are evenly spaced across the `u16` range; old `u8` values
multiplied by 257 (since `255 × 257 = 65535 = u16::MAX`):

| Category | Count | `_rarity` values (approx, ×257 spacing) |
|----------|-------|------------------------------------------|
| Standard | 9 | 0, 8224, 16448, 24672, 32896, 40863, 49087, 57311, 65535 |
| Rare | 9 | 0, 8224, 16448, 24672, 32896, 40863, 49087, 57311, 65535 |
| Exotic | 8 | 0, 9252, 18761, 28013, 37522, 46774, 56283, 65535 |

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
    _luck:            u16,    // private; access via luck() / with_luck[_f64]()
    pub bag:          Bag,
}
```

Builders:

| Method | Description |
|--------|-------------|
| `new(display_name)` | Random ID, neutral luck (0), empty bag |
| `with_id(MemberId)` | Override ID |
| `with_bag(Bag)` | Override bag |
| `with_luck(u16)` | Set raw luck |
| `with_luck_f64(f64)` | Set luck from normalised float; round-trip not guaranteed |

Getters:

| Method | Returns |
|--------|---------|
| `luck() -> f64` | `_luck / u16::MAX`, in `[0.0, 1.0]` |

Mutation:

| Method | Description |
|--------|-------------|
| `receive(fruit) -> &mut Self` | Adds one fruit to the bag |

### Community (`community.rs`)

```rust
pub struct Community {
    pub id:      CommunityId,
    _luck:       u16,                      // private
    pub members: HashMap<MemberId, Member>,
    pub version: SequenceId,               // last applied effect; zero = no effects applied
}
```

Builders:

| Method | Description |
|--------|-------------|
| `new()` | Random ID, neutral luck, no members, version zero |
| `with_id(CommunityId)` | Override ID |
| `with_luck(u16)` | Set raw luck |
| `with_luck_f64(f64)` | Set luck from normalised float; round-trip not guaranteed |
| `with_version(SequenceId)` | Override version; useful when reconstituting from storage |

Getters:

| Method | Returns |
|--------|---------|
| `luck() -> f64` | `_luck / u16::MAX`, in `[0.0, 1.0]` |

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

Both `Member` and `Community` store luck as a `u16` in `[0, 65535]` but expose it as
`f64` in `[0.0, 1.0]` via the `luck()` getter:

```
luck() = _luck as f64 / u16::MAX as f64
```

`with_luck_f64(v)` performs the inverse:

```
_luck = (v * u16::MAX as f64).round() as u16
```

Round-trips are not exact because not every `f64` in `[0,1]` maps to a distinct `u16`.

### Event log (`event_log.rs`)

```rust
pub struct SequenceId(u64);   // implements IntegerIdentifier; zero() is a sentinel

pub enum Record {             // a single log entry
    Event(Event),
    Effect(Effect),
}

pub enum EventPayload {
    Grant { count: usize },
    AddMember { display_name: String },
    RemoveMember { member_id: MemberId },
    SetCommunityLuck { luck: u16 },
    SetMemberLuck { member_id: MemberId, luck: u16 },
}

pub struct Event {
    pub id:           SequenceId,
    pub community_id: CommunityId,
    pub payload:      EventPayload,
}

pub enum StateMutation {
    AddFruitToMember { member_id: MemberId, fruit: Fruit },
    AddMember { member: Member },
    RemoveMember { member_id: MemberId },
    SetCommunityLuck { luck: u16 },
    SetMemberLuck { member_id: MemberId, luck: u16 },
}

pub struct Effect {
    pub id:           SequenceId,
    pub event_id:     SequenceId,
    pub community_id: CommunityId,
    pub mutations:    Vec<StateMutation>,
}
```

`EventPayload` and `StateMutation` derive `Debug, Clone, PartialEq, Eq`.
`Event`, `Effect`, and `Record` derive `Debug, Clone, PartialEq, Eq`.

`Effect::apply` handles all five `StateMutation` variants:
- `AddFruitToMember` — calls `member.receive(fruit)`; silently skips absent members.
- `AddMember` — calls `community.add_member(member.clone())`.
- `RemoveMember` — calls `community.remove_member(member_id)`.
- `SetCommunityLuck` — sets the community's raw luck value.
- `SetMemberLuck` — sets a member's raw luck value; silently skips absent members.

### Granter port (`granter.rs`)

```rust
pub trait Granter {
    fn grant(&mut self, community: &Community, count: usize) -> Vec<StateMutation>;
}
```

Computes the `StateMutation`s that would result from distributing `count` fruits to
**each** member of `community`. The caller is responsible for recording the returned
mutations as an `Effect` and applying them.

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
using weighted-random selection with an injectable weight strategy.

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

```rust
pub trait CommunityProvider {
    fn get(&self, id: CommunityId, version: SequenceId) -> Result<Option<Community>, Error>;
    fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Error>;
}

pub trait CommunityPersistor {
    fn put(&self, community: Community) -> Result<Community, Error>;
}

pub trait CommunityRepo: CommunityProvider + CommunityPersistor {}
```

All methods take `&self` (not `&mut self`). Implementations manage interior mutability
(e.g. via a connection pool or `RwLock`).

Every community write is a new snapshot version. There is no overwrite/upsert operation:
communities are always advanced by appending a new version via `put`.

### Event log ports (`event_log_repo.rs`)

```rust
pub trait EventLogProvider {
    fn get_record(&self, id: SequenceId) -> Result<Option<Record>, Error>;
    fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Error>;
    fn get_effects_after(&self, community_id: CommunityId, after: SequenceId) -> Result<Vec<Effect>, Error>;
    fn get_latest_records(&self, community_id: CommunityId, n: usize) -> Result<Vec<Record>, Error>;
}

pub trait EventLogPersistor {
    fn append_event(&self, community_id: CommunityId, payload: EventPayload) -> Result<Event, Error>;
    fn append_effect(&self, event_id: SequenceId, community_id: CommunityId, mutations: Vec<StateMutation>) -> Result<Effect, Error>;
}

pub trait EventLogRepo: EventLogProvider + EventLogPersistor {}
```

`get_effects_after` returns all effects for `community_id` whose sequence ID is
strictly greater than `after`, sorted ascending. Useful for replaying effects since a
known snapshot.

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

### Error (`error.rs`)

`Error` is a domain-level error type that wraps `std::io::Error` and
`std::sync::PoisonError`. Used as the `Err` variant of all storage port results.

---

## In-Memory Database (`in_memory_db` crate)

`InMemoryCommunityRepo` implements `CommunityRepo` using a
`RwLock<HashMap<CommunityId, BTreeMap<SequenceId, Community>>>`. Each community maps to
a `BTreeMap` of versioned snapshots keyed by `SequenceId`; `get_latest` returns the
entry with the greatest key.

`InMemoryEventLogRepo` implements `EventLogRepo` using two separate
`RwLock<HashMap>`s — one for events (keyed by sequence ID) and one for effects (keyed
by event ID) — plus a shared `AtomicU64` sequence counter.

For both types:
- Reads acquire a shared read lock.
- Writes acquire an exclusive write lock.
- A poisoned lock propagates as a domain `Error`.
- Both implement `EventLogProvider`/`EventLogPersistor` for both owned and `&`-reference
  types so they can be shared between a `CommunityStore` and an `EventLogStore` without
  cloning.

---

## Command-Line Service (`command_line_service` crate)

A terminal REPL (`repl::run()`) for interactive testing of the game loop.

**Start-up**: creates one community backed by `InMemoryCommunityRepo` and
`InMemoryEventLogRepo`; creates a `RandomGranter` seeded from `rand::thread_rng()`.

**Commands**:

| Command | Description |
|---------|-------------|
| `add <name>` | Add a member (recorded as `AddMember` event + effect) |
| `remove <name>` | Remove a member by display name (recorded as `RemoveMember` event + effect) |
| `grant <count>` | Grant N fruits to every member (recorded as `Grant` event + effect) |
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
| Rarity | `u16` (`_rarity`) | `f64` via `u16::MAX` | struct literal only |
| Luck | `u16` (`_luck`) | `f64` via `u16::MAX` | `with_luck(u16)` / `with_luck_f64(f64)` |

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
Tests are **not** run in the pre-commit hook (run `make t` or `make tc` manually).
