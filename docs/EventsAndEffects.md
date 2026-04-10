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

Events and their corresponding Effects share the same sequence ID. The counter
auto-increments only for Events; storing an Effect does not advance it. Event n and
Effect n both carry ID n. Within a given ID, the Event is logically ordered before its
Effect.

This gives a total ordering across all log entries and guarantees that no Effect can
appear at a later sequence position than a subsequent Event. Effect n is always computed
against the state produced by applying Effects 1 through n−1.

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
Event to the Community state produced by all Effects with ID < event.id. An Effect may be
a no-op (e.g. the command was invalid — sender didn't hold the fruit) or may describe
multiple state mutations (e.g. remove a fruit from one bag, add it to another, adjust
personal and community luck).

```
Effect {
    id:           u64              // same as the Event's ID
    community_id: CommunityId
    mutations:    Vec<StateMutation>  // empty = no-op
}
```

The `event_id` pointer is omitted because `effect.id == event.id`; the pairing is
established by the shared sequence ID.

### Community Snapshot

A Community Snapshot is a cached copy of the Community state after a given Effect has been
applied. Snapshots are a **performance optimisation** — without them, deriving current
state would require replaying the entire Effect log from the beginning each time.

In the implementation, the `Community` struct itself serves as the snapshot. It carries a
`version: SequenceId` field that records the sequence ID of the last Effect that was
folded into this instance:

```
Community {
    id:      CommunityId
    version: SequenceId   // the Effect after which this snapshot was taken; zero = baseline
    ...                   // members, luck, etc.
}
```

The baseline snapshot is **version 0** (`SequenceId::zero()`): an empty Community created
when the Community is first initialised. Version 0 is defined in the domain code and
requires no Event or Effect to produce.

Snapshots do not need to exist for every Effect. A snapshot for the most recently
processed Effect is sufficient for production use. Older snapshots can be cleaned up by a
background process.

---

## Processing Flow

1. A player submits a command (e.g. "gift 🍇 to Alice").
2. The command is validated structurally (correct syntax, known member names, etc.) and,
   if valid, written to the event log as an **Event** with the next available ID n.
3. The **Community state before event n** is derived: find the most recent snapshot with
   `version < n`, then apply all Effects with IDs in `(snapshot.version, n)` in order.
4. The Event is applied to that state to produce an **Effect**, which is written with ID n
   (the same ID as the Event).
5. An updated Community Snapshot is stored, keyed by ID n.

In the first implementation steps (3–5) happen synchronously in the same request. The
design anticipates moving to an asynchronous worker that tails the event log and processes
Events independently.

---

## Interleaving

Because Effects are calculated after their Events are stored, two Events can be stored
before either Effect is calculated. Under the shared-ID model, Effects are always computed
in event-ID order, each building on the state produced by all preceding Effects:

```
ID 1  Event  (Alice gifts 🍇 to Bob)
ID 2  Event  (Bob burns 🍓)              ← may be stored before Effect 1 is written
ID 1  Effect (Gift effect — computed against state before ID 1)
ID 2  Effect (Burn effect — computed against state through Effect 1)
```

Effect 2 is always computed against a state that includes Effect 1, regardless of whether
Event 2 was stored before Effect 1 was written. This eliminates the class of anomalies
possible in models where effects are assigned independent IDs and may interleave with
later events.

### Invariant violations under interleaving

Invariants are enforced at Effect-calculation time rather than Event-storage time. A
command that looked valid when the player issued it may produce a no-op Effect once the
preceding Effects have been applied.

Example: Bob holds exactly one 🍓. He submits two gift commands in quick succession before
either Effect is calculated.

```
ID 1  Event  (Bob gifts 🍓 to Alice)
ID 2  Event  (Bob gifts 🍓 to Carol)     ← stored before Effect 1 is written
ID 1  Effect (Gift 🍓 from Bob to Alice — valid; Bob held 🍓 before ID 1)
ID 2  Effect (no-op — invalid; Bob no longer holds 🍓 after Effect 1)
```

Bob is not penalised for the second command; the no-op Effect is simply a record that the
Event was processed and found to violate an invariant.
