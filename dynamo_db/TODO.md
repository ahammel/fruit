# DynamoDB Implementation — Review Notes

- [ ] Delete this file before merging.
- [x] **Add a standing pre-merge checklist to `CLAUDE.md`.** Extract the generally-applicable items from this file (docs pass, binary UUIDs, test coverage, etc.) into a reusable checklist in `~/repos/claude-md/CLAUDE.md` under a "Before merging a feature branch" heading so future branches start with the same bar.

## Performance

- [x] **Store UUID fields as binary.** `pk` and UUID attributes are stored as 36-character hyphenated strings (`AttributeValue::S`). Switching to `AttributeValue::B` (16-byte raw UUID) would reduce item size and read/write costs. Trade-off: binary values appear as Base64 in the console and CLI, making manual inspection harder.

## Correctness

- [x] **Sequence ID gaps on process failure.** `next_seq_id` atomically increments the counter, but if the process dies before `append_event` completes the sequence ID is lost forever, leaving a gap in the log. Decide whether gaps are acceptable, or whether the counter should only advance on successful write (e.g. use a conditional put on the counter item itself, or derive the next ID from the last written event key). **Decision: gaps are acceptable.**

- [x] **Sequence ID consistency.** The counter uses `ADD` (eventually consistent read path on the update). Confirm whether DynamoDB `UpdateItem` with `ReturnValue::UpdatedNew` gives a strongly consistent view of the counter or whether two concurrent callers can receive the same ID. **Decision: duplicate IDs are acceptable; the second writer will hit the `attribute_not_exists` condition and get `AlreadyExists`, which is the retry signal.**

- [x] **Consistent writes for event append.** `append_event_async` uses `attribute_not_exists(sk)` — confirm this is a strongly consistent conditional write and that it correctly rejects a duplicate at the same key. Document the chosen consistency model explicitly.

- [x] **GSI `seq-index` is an API error.** `get_record(SequenceId)` and `get_effect_for_event(SequenceId)` omit `community_id`, but every item in the table belongs to a community. These port methods should take a `CommunityId` parameter and use the main table's PK rather than a GSI, eliminating the GSI entirely.

- [x] **SDK errors mapped to a single anomaly type.** All `SdkError` variants (throttling, provisioned throughput exceeded, network timeout, unretryable client errors, etc.) are collapsed into `Error::Sdk { category: unavailable }`. Each should be inspected and mapped to the appropriate anomaly category/status so that callers can make correct retry decisions without parsing the message string.

- [x] **`query_events_by_type` hard-codes a dummy Gift payload to extract the type name** (`event_log_repo.rs:325`). `event_type_name` should take the variant discriminant directly (e.g. a plain string constant or a separate enum) rather than constructing a throwaway value with fabricated field data.

- [x] **`query_events_by_type` inlines the EVENT SK range** (`event_log_repo.rs:263`) instead of calling `sk_event_range`. Use the existing helper for consistency.

## Architecture

- [x] **Async should be defined at the domain boundary.** The async helpers (`get_record_async`, etc.) are an accidental layer caused by bridging sync ports to an async SDK. The ports themselves should be defined as `async fn` in the domain, removing the need for the private async twin methods and the owned `tokio::runtime::Runtime`.

## Reliability

- [x] **Retry/back-off on concurrent event append.** If two writers race to append at the same sequence ID, one will get `AlreadyExists`. Decide whether the repo should transparently retry with a fresh sequence ID, or surface the conflict to the caller and let the service layer decide. **Decision: not needed.** `UpdateItem ADD` is atomic — each concurrent caller receives a distinct counter value, so `AlreadyExists` from `append_event` is unreachable in normal operation. Surfacing it to the caller is the right behaviour if it somehow occurs.

## Documentation

- [x] **Documentation pass.** Audit all public members for missing or stale docstrings, update `README.md` and `docs/SPEC.md` to reflect the DynamoDB implementation (table schema, key design, consistency model, counter-per-community design).

## Testing

- [x] **100% line coverage.** Every line in the crate must be exercised by unit tests using a mocked DynamoDB client.

- [ ] **100% mutation coverage.** Run `cargo-mutants` (or equivalent) against the crate and drive the mutation score to 100% using the same mock-based unit test suite. The Localstack integration tests cover happy-path wiring, not exhaustive branch coverage.

- [ ] **Localstack test bed.** No integration tests exist yet. Set up a Localstack container (Docker Compose or similar) and a test harness that:
  - provisions the table on startup
  - runs integration tests for both `EventLogRepo` and `CommunityRepo`
  - optionally wires the command-line service to Localstack for end-to-end smoke testing
