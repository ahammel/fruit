# Events and Effects

## Overview

Player actions (gifting a fruit, burning a fruit) are expressed as **Events** stored in
an append-only event log. Events do not modify Community state directly. After an Event is
stored, the current Community state is derived from prior Effects, and then the **Effect**
of that Event is calculated and stored. The Effect is the authoritative record of what
actually changed.

---

## Motivation

Gifting and burning are interactions between members (and between members and their
community) that are naturally expressed as a sequence of immutable facts. A log-structured
model gives us:

- **Audit history** — every change is traceable to the intent that caused it.
- **Derived state** — Community state at any point is a fold over the Effect log.
- **Future extensibility** — luck adjustments triggered by past behaviour, projections,
  and replays are straightforward to add.

---

## Shared ID Sequence

Events and Effects share a single auto-incrementing integer sequence. This gives a total
ordering across all Events and Effects in the system and is the foundation for determining
"state at the time an Event was stored."

An Event is assigned an ID when it is stored. Its corresponding Effect is assigned the
next available ID from the same sequence when it is calculated and stored. Because other
Events may be stored in the interim, the Effect's ID is not necessarily `event_id + 1`.

---

## Data Model

### Event

An Event captures a player's **intent**. It is a compound type: a single Event may
describe an action that will touch many aspects of Community state (e.g. a gift names the
sender, the receiver, and the fruit). An Event is immutable once stored.

```
Event {
    id:           u64              // from the shared sequence
    community_id: CommunityId
    payload:      EventPayload     // enum of action variants (Gift, Burn, …)
}
```

### Effect

An Effect is the **computed consequence** of its Event. It is calculated by applying the
Event to the Community state as it existed immediately before that Event's ID in the
sequence. An Effect may be a no-op (e.g. the command was invalid — sender didn't hold the
fruit) or may describe multiple state mutations (e.g. remove a fruit from one bag, add it
to another, adjust personal and community luck).

```
Effect {
    id:           u64              // from the shared sequence
    event_id:     u64             // pointer back to the Event that caused this Effect
    community_id: CommunityId
    mutations:    Vec<StateMutation>  // empty = no-op
}
```

### Community Snapshot

A Community Snapshot is a cached copy of the Community state after a given Effect has been
applied. Snapshots are a **performance optimisation** — without them, deriving current
state would require replaying the entire Effect log from the beginning each time.

```
CommunitySnapshot {
    community_id: CommunityId
    effect_id:    u64          // the Effect after which this snapshot was taken
    community:    Community
}
```

The baseline snapshot is **version 0**: an empty Community created when the Community is
first initialised. Version 0 is defined in the domain code and requires no Event or Effect
to produce.

Snapshots do not need to exist for every Effect. A snapshot for the most recently
processed Effect is sufficient for production use. Older snapshots can be cleaned up by a
background process.

---

## Processing Flow

1. A player submits a command (e.g. "gift 🍇 to Alice").
2. The command is validated structurally (correct syntax, known member names, etc.) and,
   if valid, written to the event log as an **Event** with the next available ID.
3. The **current Community state** is derived: find the most recent snapshot with
   `effect_id < event.id`, then apply all Effects with IDs between that snapshot's
   `effect_id` (exclusive) and `event.id` (exclusive) in order.
4. The Event is applied to that state to produce an **Effect**, which is written with the
   next available ID from the shared sequence.
5. An updated Community Snapshot is stored, keyed by the Effect's ID.

In the first implementation steps (3–5) happen synchronously in the same request. The
design anticipates moving to an asynchronous worker that tails the event log and processes
Events independently.

---

## Interleaving

Because Effects are calculated after their Events are stored, two Events can be stored
before either Effect is calculated:

```
ID 1  Event  (Alice gifts 🍇 to Bob)
ID 2  Event  (Bob burns 🍓)           ← stored before Effect 1 is written
ID 3  Effect (Gift effect for ID 1)
ID 4  Effect (Burn effect for ID 2, computed against state before Effect 3)
```

In this scenario Bob's burn (ID 2) is computed against state that does not yet include
Alice's gift. At Slack / Teams / Discord interaction volumes — where concurrent submissions
from the same community are rare — this should not cause observable problems in practice.

### Invariant violations under interleaving

Because each Effect is computed against the state produced by all prior Effects (not prior
Events), invariants are enforced at Effect-calculation time rather than Event-storage time.
This means a command that looked valid when the player issued it may produce a no-op Effect
once the preceding Effects have been applied.

Example: Bob holds exactly one 🍓. He submits two gift commands in quick succession before
either Effect is calculated.

```
ID 1  Event  (Bob gifts 🍓 to Alice)
ID 2  Event  (Bob gifts 🍓 to Carol)   ← stored before Effect 1 is written
ID 3  Effect (Gift 🍓 from Bob to Alice — valid; Bob held 🍓 at state before ID 1)
ID 4  Effect (no-op — invalid; Bob no longer holds 🍓 after Effect ID 3)
```

Bob is not penalised for the second command; the no-op Effect is simply a record that the
Event was processed and found to violate an invariant.
