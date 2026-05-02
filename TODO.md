# Slack Lambda Service TODO

## Design decisions (resolve before coding)

- [x] **Community granularity**: one community per workspace.
- [x] **Interaction model**: slash commands with proactive `chat.postMessage` notifications
      to affected users (option B — same Lambda, extra `chat:write` scope).
- [x] **Grant scheduling**: EventBridge cron triggering the same `FruitLambda` (single
      Lambda handles both API Gateway and EventBridge events).
- [x] **Identity mapping**: deterministic UUIDv5 — workspace namespace derived from
      `team_id`, member ID derived from `(workspace_ns, slack_user_id)`.

---

## 1 — New `slack_service` crate

Add to the workspace alongside `command_line_service`. This is the service layer:
wires `dynamo_db` repos, handles HTTP from API Gateway, and returns Slack responses.

- [ ] Add `slack_service/` to `[workspace.members]` in root `Cargo.toml`
- [ ] Add dependencies:
  - `lambda_http` — AWS Lambda + API Gateway adapter
  - `tokio` with `macros` + `rt-multi-thread`
  - `aws-config` + `aws-sdk-dynamodb` (for wiring `DynamoDbEventLogRepo` /
    `DynamoDbCommunityRepo`)
  - `hmac` + `sha2` — Slack request signature verification
  - `serde` + `serde_json` — Slack payload parsing and Block Kit responses
  - `hex` — decode the `X-Slack-Signature` header
  - `fruit-domain` + `fruit-dynamo-db`
- [ ] Add `[[bin]] name = "bootstrap"` target (Lambda requires the binary to be named
      `bootstrap` when using the `provided.al2023` runtime)

---

## 2 — Slack request verification

- [ ] Implement `fn verify_slack_signature(signing_secret: &str, timestamp: &str,
      body: &[u8], sig_header: &str) -> bool` using `hmac::Hmac<sha2::Sha256>`
- [ ] Reject requests where `abs(now - timestamp) > 300s` (replay protection)
- [ ] Return HTTP 401 for invalid signatures; all other handler logic runs only after
      verification passes

---

## 3 — Slash command handler

Each slash command POST delivers `application/x-www-form-urlencoded`. Parse into a
shared `SlashPayload` struct, then dispatch.

- [ ] Parse `command`, `text`, `channel_id`, `user_id`, `user_name` from the POST body
- [ ] Map `channel_id` → `CommunityId` (via UUID v5 or lookup — see design decision)
- [ ] Map `user_id` → `MemberId` (same approach)
- [ ] Implement commands (each calls the same domain operations as the REPL):
  - [ ] `/fruit bag` — show the caller's current bag and luck
  - [ ] `/fruit grant <n>` — grant N fruits to every member (gated to admins or
        scheduled-only, depending on the grant-scheduling decision)
  - [ ] `/fruit gift <@user> <emoji>` — gift one fruit
  - [ ] `/fruit burn <emoji> [<emoji> ...]` — burn one or more fruits (reuse the
        greedy emoji-parsing logic from the REPL)
  - [ ] `/fruit luck` — show community and personal luck scores
  - [ ] `/fruit leaderboard` — show members sorted by luck
  - [ ] `/fruit help` — list available commands
- [ ] Auto-provision: if no community exists for `channel_id`, call
      `CommunityStore::init()` and add the calling user as the first member before
      processing the command
- [ ] Return Slack Block Kit JSON for all non-error responses

---

## 4 — Scheduled grant handler (EventBridge path in `FruitLambda`)

- [ ] Dispatch EventBridge events in the same Lambda entry point as slash commands
- [ ] Accept an EventBridge event carrying `community_id` and `count`
- [ ] Reuse `Providence::grant_fruit` exactly as the REPL does
- [ ] Post a Slack message to the channel via `chat.postMessage` after the grant completes

---

## 5 — Infrastructure

- [ ] Add `infrastructure/` directory with a SAM template (`template.yaml`) or CDK
      stack; record the choice in `SPEC.md`
- [ ] Lambda function:
  - Runtime: `provided.al2023`, architecture `arm64` (matches `aarch64-unknown-linux-musl`)
  - Memory: 256 MB (baseline; tune after load testing)
  - Timeout: 10 s for slash commands (Slack requires a response within 3 s; use
    immediate ack + async follow-up if domain logic may exceed that)
  - Environment variables: `TABLE_NAME`, `SLACK_SIGNING_SECRET`
- [ ] API Gateway HTTP API (not REST API — lower latency, lower cost) with a single
      `POST /slack/events` route
- [ ] DynamoDB table: same single-table schema as in `dynamo_db`; provisioned via the
      SAM/CDK template so it is created on first deploy
- [ ] IAM execution role: `dynamodb:GetItem`, `dynamodb:PutItem`, `dynamodb:UpdateItem`,
      `dynamodb:Query` on the table ARN only (least privilege)
- [ ] SSM Parameter Store (or Secrets Manager) for `SLACK_SIGNING_SECRET`; grant the
      Lambda role `ssm:GetParameter` on the specific path

---

## 6 — Slack app configuration

- [ ] Create a Slack app at <https://api.slack.com/apps>
- [ ] Enable **Slash Commands**; register `/fruit` with the API Gateway invoke URL
- [ ] OAuth scopes required: `commands` (slash commands), `chat:write` (post grant
      summaries), `users:read` (resolve display names if not relying on `user_name`)
- [ ] Copy the **Signing Secret** to SSM; copy the **Bot Token** if `chat.postMessage`
      is used

---

## 7 — Build and deployment

- [ ] Install `cargo-lambda` (`cargo install cargo-lambda`)
- [ ] Add `Makefile` targets:
  - `build-lambda` — `cargo lambda build --release --arm64 -p slack_service`
  - `deploy-lambda` — `cargo lambda deploy` or `sam deploy`
- [ ] Verify the binary is named `bootstrap` in the Lambda zip
- [ ] Add `aarch64-unknown-linux-musl` to `rust-toolchain.toml` (or document the
      `rustup target add` step in README)

---

## 8 — Testing

- [ ] Unit tests for `verify_slack_signature` (valid, expired timestamp, wrong secret)
- [ ] Unit tests for slash command parsing (all commands, missing args, unknown
      subcommand)
- [ ] Integration tests against a local Lambda emulator (`cargo lambda watch`) using
      recorded Slack payloads
- [ ] Update `make ti` or add `make ti-slack` to run Slack-specific integration tests
      against `amazon/dynamodb-local`

---

## 9 — Documentation

- [ ] Update `README.md`: add `slack_service` to the crate list and document the new
      Makefile targets
- [ ] Update `SPEC.md`: add a `Slack Service` section covering the handler architecture,
      community/member identity mapping decision, and the DynamoDB table (shared with or
      separate from the existing schema)
- [ ] Document the Slack app setup steps (invite the bot to a channel, first `/fruit`
      command auto-provisions the community)
