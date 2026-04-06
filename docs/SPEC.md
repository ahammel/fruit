# fruit — Technical Specification

## Overview

fruit is a gift-economy simulation game. Players belong to a **community** and
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
    _rarity:      u8,          // private; access via rarity()
}

impl Fruit {
    pub fn rarity(&self) -> f64  // _rarity / u8::MAX, in [0.0, 1.0]
}
```

`Fruit` derives `Debug, Clone, Copy, PartialEq, Eq, Hash`. Equality and hashing use
all fields (not emoji alone), so two fruits with the same emoji but different rarity are
distinct.

**Fruit pool** — 26 static constants split across categories:

| Category | Count | `_rarity` values (evenly spaced) |
|----------|-------|----------------------------------|
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
}
```

Builders:

| Method | Description |
|--------|-------------|
| `new()` | Random ID, neutral luck, no members |
| `with_id(CommunityId)` | Override ID |
| `with_luck(u16)` | Set raw luck |
| `with_luck_f64(f64)` | Set luck from normalised float; round-trip not guaranteed |

Getters:

| Method | Returns |
|--------|---------|
| `luck() -> f64` | `_luck / u16::MAX`, in `[0.0, 1.0]` |

Mutation:

| Method | Returns | Description |
|--------|---------|-------------|
| `add_member(Member) -> bool` | `true` if inserted, `false` if ID already present |
| `remove_member(MemberId) -> Option<Member>` | Removed member, or `None` |

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
pub enum EventPayload {
    Grant { count: usize },
    AddMember { display_name: String },
    RemoveMember { member_id: MemberId },
    SetCommunityLuck { luck: u16 },
    SetMemberLuck { member_id: MemberId, luck: u16 },
}

pub enum StateMutation {
    AddFruitToMember { member_id: MemberId, fruit: Fruit },
    AddMember { member: Member },
    RemoveMember { member_id: MemberId },
    SetCommunityLuck { luck: u16 },
    SetMemberLuck { member_id: MemberId, luck: u16 },
}
```

`EventPayload` and `StateMutation` derive `Debug, Clone, PartialEq, Eq`.
`Event` and `Effect` derive `Debug, Clone, PartialEq, Eq`.

`Effect::apply` handles all five `StateMutation` variants:
- `AddFruitToMember` — calls `member.receive(fruit)`; silently skips absent members.
- `AddMember` — calls `community.add_member(member.clone())`.
- `RemoveMember` — calls `community.remove_member(member_id)`.
- `SetCommunityLuck` — sets the community's raw luck value.
- `SetMemberLuck` — sets a member's raw luck value; silently skips absent members.

### Granter port (`granter.rs`)

```rust
pub trait Granter {
    fn grant(&mut self, community: &mut Community, count: usize);
}
```

Distributes `count` fruits to **each** member of `community`.

### Random granter (`random_granter.rs`)

`RandomGranter<R: Rng>` implements `Granter` using weighted-random selection.

**Effective luck** for a member is the sum of member and community luck (each in
`[0.0, 1.0]`):

```
luck = member.luck() + community.luck()   // ∈ [0.0, 2.0]
```

**Per-fruit weight formula** (where `r = fruit.rarity()` ∈ `[0.0, 1.0]`):

```
Standard : (0.065 − 0.030·r) / (1 + luck)
Rare     : (0.050 − 0.030·r) · (1 + luck)
Exotic   : lerp(0.010 − 0.009·r,  0.050 − 0.040·r,  luck)
```

Weights are floored at `f64::EPSILON` so no fruit is ever completely excluded.

**Approximate drop probabilities at neutral luck (`luck = 0`)**:

| Category | Most common | Rarest |
|----------|-------------|--------|
| Standard | ~1/15 | ~1/22 |
| Rare | ~1/19 | ~1/48 |
| Exotic | ~1/96 | ~1/958 |

**At max luck (`luck = 2.0`)** standard drops are heavily suppressed; exotic range
compresses to roughly 1/21 – 1/104 with the rarest exotics gaining the largest boost.

Construction:

```rust
RandomGranter::new(rng)                  // uses full FRUITS pool
    .with_fruits(&[GRAPES, STRAWBERRY])  // restrict pool (panics if empty)
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

### Store wrapper (`community_store.rs`)

```rust
pub struct CommunityStore<CR: CommunityRepo, ELP: EventLogProvider> { ... }
```

Thin wrapper that exposes `get`, `get_latest`, and `put` without requiring callers to
depend on the port traits directly. `get_latest` folds any unapplied effects from the
event log into the latest snapshot and persists the result before returning.

### Error (`error.rs`)

`Error` is a domain-level error type that wraps `std::io::Error` and
`std::sync::PoisonError`. Used as the `Err` variant of all storage port results.

---

## In-Memory Database (`in_memory_db` crate)

`InMemoryCommunityRepo` implements `CommunityRepo` (and therefore both
`CommunityProvider` and `CommunityPersistor`) using a `RwLock<HashMap<CommunityId,
Community>>`.

- Reads acquire a shared read lock.
- Writes acquire an exclusive write lock.
- A poisoned lock propagates as a domain `Error`.

---

## Command-Line Service (`command_line_service` crate)

A terminal REPL (`repl::run()`) for interactive testing of the game loop.

**Start-up**: creates one community backed by `InMemoryCommunityRepo`; creates a
`RandomGranter` seeded from `rand::thread_rng()`.

**Commands**:

| Command | Description |
|---------|-------------|
| `add <name>` | Add a member (recorded as `AddMember` event + effect) |
| `remove <name>` | Remove a member by display name (recorded as `RemoveMember` event + effect) |
| `grant <count>` | Grant N fruits to every member (recorded as `Grant` event + effect) |
| `luck <value>` | Set community luck (float in `[0.0, 1.0]`) |
| `luck <name> <value>` | Set a member's luck |
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
- Fixed-seed deterministic tests (`StdRng::seed_from_u64(0)`) verify the exact fruit
  sequence produced by `RandomGranter`; update the expected value if `FRUITS` or the
  weight formula changes.

---

## Conventions

### Numeric types for game scores

| Score | Storage | Getter return | Setter |
|-------|---------|---------------|--------|
| Rarity | `u8` (`_rarity`) | `f64` via `u8::MAX` | struct literal only |
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
