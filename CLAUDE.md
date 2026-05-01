# Claude Instructions

Before writing any code, read:

- [`README.md`](README.md) — game concept, fruit catalogue, and development commands
- [`docs/SPEC.md`](docs/SPEC.md) — full technical specification: domain model, port conventions, numeric type conventions, testing rules

Keep both documents up to date whenever code changes affect them. Use them as the authoritative source of truth when generating code.

# Before Merging a Feature Branch

Work through this checklist before opening a PR or merging any feature branch.

## Documentation

- [ ] All public members have docstrings.
- [ ] `README.md` reflects any new commands, modules, or external dependencies.
- [ ] `docs/SPEC.md` reflects the implementation: schemas, key design, consistency model,
      and any architecture decisions made during the branch.

## Design decisions

- [ ] All open design questions are resolved. Each has an explicit decision note (in code
      comments, SPEC, or a checked-off TODO item). No unresolved "decide whether…" items remain.
- [ ] No placeholder or dummy implementations remain (fabricated data, hard-coded stubs, etc.).

## Storage and external interfaces

- [ ] IDs and enums are stored in their most compact correct representation (e.g. binary
      UUIDs rather than hyphenated strings, enum discriminants rather than full serialised values).
- [ ] All errors from external systems (SDKs, databases, HTTP clients) are mapped to
      structured anomaly categories (`Unavailable`, `Busy`, `Fault`, etc.) with a retry
      `Status` (`Temporary` / `Permanent`). Callers must not need to parse message strings
      to decide whether to retry.

## Test coverage

- [ ] 100% line coverage, exercised by unit tests with all external I/O mocked or stubbed.
- [ ] 100% mutation coverage via `cargo-mutants` (or equivalent for the language).

## Housekeeping

- [ ] No branch-specific work notes or scratch files remain (e.g. `TODO.md`).
