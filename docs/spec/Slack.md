# fruit — Slack Integration Specification

## Overview

The Slack integration surfaces the fruit game through a Slack workspace. Each workspace
maps to exactly one community. Players interact via slash commands; the game engine runs
in AWS Lambda backed by DynamoDB.

---

## Design Decisions

### One community per workspace

Each Slack workspace corresponds to exactly one fruit community. Workspace identity
(`team_id` from Slack's OAuth payload) is the durable identifier for the community.

### Identity mapping via UUIDv5

Slack user IDs (`U…`) are mapped to fruit `MemberId` values using deterministic UUIDv5,
namespaced per workspace. The namespace UUID for the workspace is itself derived from
`team_id` via UUIDv5 under the DNS namespace:

```
workspace_ns  = UUIDv5(DNS_NAMESPACE, team_id)
member_id     = UUIDv5(workspace_ns, slack_user_id)
```

This means:
- The mapping is stateless and reproducible.
- Members from different workspaces never collide, even if Slack user IDs were ever
  reused across workspaces.
- No separate identity-mapping table is needed.

### Grants scheduled by EventBridge cron

The periodic fruit grant (game tick) is triggered by an Amazon EventBridge scheduled
rule targeting the grant Lambda. The grant Lambda calls the domain's `CommunityStore`
to compute the grant, appends the event and effect to DynamoDB, then optionally notifies
recipients via `chat.postMessage`.

---

## Interaction Model

### Slash commands

Players interact with the game using a single Slack slash command (e.g. `/fruit`):

| Subcommand | Effect |
|---|---|
| `/fruit bag` | Display the caller's current bag |
| `/fruit gift @user <fruit>` | Gift a fruit to another member |
| `/fruit burn <fruit>` | Burn a fruit |
| `/fruit leaderboard` | Show community luck rankings |

Slash command requests are signed by Slack (HMAC-SHA256 on the request body + timestamp).
The Lambda verifies the signature before processing any command.

### Proactive notifications

Slash commands are request/response only and cannot push messages to other users.
Recipient notifications (e.g. "You were gifted a mango by @alice") require the
`chat:write` bot scope and an explicit `chat.postMessage` call after each gift or grant
event is persisted.

The Lambda for each mutating command notifies affected recipients before returning the
slash command response. The grant cron Lambda notifies each member who received fruits.
Both share the same bot token stored in AWS Secrets Manager.

---

## AWS Architecture

A single Lambda handles all events. It inspects the event source and dispatches
accordingly.

```
Slack workspace
     │  HTTPS POST (slash command)
     ▼
API Gateway ──────────────────┐
                              ▼
EventBridge cron ────────► FruitLambda
                              │  reads/writes
                              ▼
                           DynamoDB
```

### Lambda dispatch

| Event source | Handler responsibilities |
|---|---|
| API Gateway (Slack POST) | Verify signature; dispatch subcommand; call domain; notify affected users; return slash command response |
| EventBridge cron | Load community; compute grant; persist event+effect; notify all recipients |

---

## Open Questions

_None at this time._
