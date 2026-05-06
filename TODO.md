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

- [x] Add `slack_service/` to `[workspace.members]` in root `Cargo.toml`
- [x] Add dependencies:
  - `lambda_http` — AWS Lambda + API Gateway adapter
  - `tokio` with `macros` + `rt-multi-thread`
  - `aws-config` + `aws-sdk-dynamodb` (for wiring `DynamoDbEventLogRepo` /
    `DynamoDbCommunityRepo`)
  - `hmac` + `sha2` — Slack request signature verification
  - `serde` + `serde_json` — Slack payload parsing and Block Kit responses
  - `hex` — decode the `X-Slack-Signature` header
  - `fruit-domain` + `fruit-dynamo-db`
- [x] Add `[[bin]] name = "bootstrap"` target (Lambda requires the binary to be named
      `bootstrap` when using the `provided.al2023` runtime)
- [x] Update `README.md`: add `slack_service` to the crate list
- [x] Update `SPEC.md`: cover handler architecture, identity mapping, and DynamoDB
      table sharing

---

## 2 — Slack request verification

- [x] Implement `fn verify_slack_signature(signing_secret: &str, timestamp: &str,
      body: &[u8], sig_header: &str) -> bool` using `hmac::Hmac<sha2::Sha256>`
- [x] Reject requests where `abs(now - timestamp) > 300s` (replay protection)
- [x] Return HTTP 401 for invalid signatures; all other handler logic runs only after
      verification passes
- [x] Unit tests: valid signature, expired timestamp, wrong secret

---

## 3 — Slash command handler

Each slash command POST delivers `application/x-www-form-urlencoded`. Parse into a
shared `SlashPayload` struct, then dispatch.

Design decisions: no `/fruit grant` (grants come from EventBridge only); explicit
`/fruit join` and `/fruit leave` instead of auto-provisioning on first command;
`/fruit gift` takes an optional message argument.

- [x] Parse `text`, `channel_id`, `user_id`, `user_name`, `team_id` from the POST body
- [x] Map `channel_id` → `CommunityId` (UUIDv5), `user_id` → `MemberId` (UUIDv5)
      via `identity.rs`; workspace namespace derived from `team_id`
- [x] Implement commands (each calls the same domain operations as the REPL):
  - [x] `/fruit join` — join (or provision) the community for this channel
  - [x] `/fruit leave` — leave the community
  - [x] `/fruit bag` — show the caller's current bag and luck
  - [x] `/fruit gift <@user> <emoji> [message]` — gift one fruit with optional message
  - [x] `/fruit burn <emoji> [<emoji> ...]` — burn one or more fruits (greedy
        emoji-parsing)
  - [x] `/fruit luck` — show community and personal luck scores
  - [x] `/fruit leaderboard` — show members sorted by luck
  - [x] `/fruit help` — list available commands
- [x] Provisioning: `/fruit join` creates the community with a deterministic
      `CommunityId` derived from `channel_id` if none exists
- [x] Return Slack Block Kit JSON for all responses
- [x] Unit tests: all commands, missing args, unknown subcommand (38 tests total)

---

## 4 — Scheduled grant handler (EventBridge path in `FruitLambda`)

- [x] Dispatch EventBridge events in the same Lambda entry point as slash commands
- [x] Accept an EventBridge event carrying `community_id` and `count`
- [x] Reuse `Providence::grant_fruit` exactly as the REPL does
- [x] Post a Slack message to the channel via `chat.postMessage` after the grant completes
- [x] Unit tests: grant dispatched, notification sent, error paths

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
- [ ] Update `SPEC.md`: record SAM vs CDK choice and infrastructure layout

---

## 6 — Slack app configuration

- [ ] Create a Slack app at <https://api.slack.com/apps>
- [ ] Enable **Slash Commands**; register `/fruit` with the API Gateway invoke URL
- [ ] OAuth scopes required: `commands` (slash commands), `chat:write` (post grant
      summaries), `users:read` (resolve display names if not relying on `user_name`)
- [ ] Copy the **Signing Secret** to SSM; copy the **Bot Token** if `chat.postMessage`
      is used
- [ ] Document setup steps in `SPEC.md` (invite the bot to a channel, first `/fruit`
      command auto-provisions the community)

---

## 7 — Build and deployment

- [ ] Install `cargo-lambda` (`cargo install cargo-lambda`)
- [ ] Add `Makefile` targets:
  - `build-lambda` — `cargo lambda build --release --arm64 -p slack_service`
  - `deploy-lambda` — `cargo lambda deploy` or `sam deploy`
- [ ] Verify the binary is named `bootstrap` in the Lambda zip
- [ ] Add `aarch64-unknown-linux-musl` to `rust-toolchain.toml` (or document the
      `rustup target add` step in README)
- [ ] Integration tests against a local Lambda emulator (`cargo lambda watch`) using
      recorded Slack payloads
- [ ] Update `make ti` or add `make ti-slack` to run Slack-specific integration tests
      against `amazon/dynamodb-local`
- [ ] Update `README.md`: document new Makefile targets
