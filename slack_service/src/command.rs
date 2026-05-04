use fruit_domain::{
    burner::compute_burn,
    community::{Community, CommunityId},
    community_repo::{CommunityProvider, CommunityRepo},
    community_store::CommunityStore,
    error::DbError,
    event_log::EventPayload,
    event_log_repo::{EventLogProvider, EventLogRepo},
    event_log_store::EventLogStore,
    fruit::FRUITS,
    gifter::compute_gift,
    member::MemberId,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::Error;

/// Dispatches `/fruit <text>` to the appropriate command and returns a Slack Block Kit response.
pub async fn dispatch<E, CR, ELR>(
    community_repo: &CR,
    event_log_repo: &ELR,
    community_id: CommunityId,
    member_id: MemberId,
    display_name: &str,
    workspace_ns: Uuid,
    text: &str,
) -> Result<Value, Error>
where
    E: DbError,
    CR: CommunityRepo + CommunityProvider<Error = E>,
    ELR: EventLogRepo + EventLogProvider<Error = E>,
{
    let community_store = CommunityStore::new(community_repo, event_log_repo);
    let event_log_store = EventLogStore::new(event_log_repo);

    let tokens: Vec<&str> = text.split_whitespace().collect();
    let (subcommand, args) = tokens
        .split_first()
        .map(|(s, rest)| (*s, rest))
        .unwrap_or(("help", &[]));

    match subcommand {
        "join" => {
            join(
                &community_store,
                &event_log_store,
                community_repo,
                community_id,
                member_id,
                display_name,
            )
            .await
        }
        "leave" => {
            leave(
                &community_store,
                &event_log_store,
                community_id,
                member_id,
                display_name,
            )
            .await
        }
        "bag" => bag(&community_store, community_id, member_id).await,
        "gift" => {
            gift(
                &community_store,
                &event_log_store,
                community_id,
                member_id,
                workspace_ns,
                args,
            )
            .await
        }
        "burn" => {
            burn(
                &community_store,
                &event_log_store,
                community_id,
                member_id,
                display_name,
                args,
            )
            .await
        }
        "help" => Ok(help()),
        other => Ok(ephemeral(format!(
            "Unknown subcommand `{other}`. Try `/fruit help`."
        ))),
    }
}

async fn join<E, CR, ELR>(
    community_store: &CommunityStore<&CR, &ELR>,
    event_log_store: &EventLogStore<&ELR>,
    community_repo: &CR,
    community_id: CommunityId,
    member_id: MemberId,
    display_name: &str,
) -> Result<Value, Error>
where
    E: DbError,
    CR: CommunityRepo + CommunityProvider<Error = E>,
    ELR: EventLogRepo + EventLogProvider<Error = E>,
{
    let maybe_community = community_store
        .get_latest(community_id)
        .await
        .map_err(storage)?;

    match &maybe_community {
        Some(c) if c.members.contains_key(&member_id) => {
            return Ok(ephemeral("You're already a member of this community."));
        }
        None => {
            community_repo
                .put(fruit_domain::community::Community::new().with_id(community_id))
                .await
                .map_err(storage)?;
        }
        _ => {}
    }

    event_log_store
        .append_event(
            community_id,
            EventPayload::AddMember {
                display_name: display_name.to_string(),
                member_id,
            },
        )
        .await
        .map_err(storage)?;

    Ok(ephemeral(format!(
        "*{display_name}* joined the community! 🎉"
    )))
}

async fn leave<E, CR, ELR>(
    community_store: &CommunityStore<&CR, &ELR>,
    event_log_store: &EventLogStore<&ELR>,
    community_id: CommunityId,
    member_id: MemberId,
    display_name: &str,
) -> Result<Value, Error>
where
    E: DbError,
    CR: CommunityRepo + CommunityProvider<Error = E>,
    ELR: EventLogRepo + EventLogProvider<Error = E>,
{
    match community_store
        .get_latest(community_id)
        .await
        .map_err(storage)?
    {
        None => {
            return Ok(ephemeral(
                "No community here yet. Use `/fruit join` to start one.",
            ));
        }
        Some(c) if !c.members.contains_key(&member_id) => {
            return Ok(ephemeral(
                "You're not a member of this community. Use `/fruit join` first.",
            ));
        }
        _ => {}
    }

    event_log_store
        .append_event(community_id, EventPayload::RemoveMember { member_id })
        .await
        .map_err(storage)?;

    Ok(ephemeral(format!(
        "*{display_name}* left the community. 👋"
    )))
}

async fn bag<E, CR, ELR>(
    community_store: &CommunityStore<&CR, &ELR>,
    community_id: CommunityId,
    member_id: MemberId,
) -> Result<Value, Error>
where
    E: DbError,
    CR: CommunityRepo + CommunityProvider<Error = E>,
    ELR: EventLogProvider<Error = E>,
{
    let community = match require_member(community_store, community_id, member_id).await? {
        Ok(c) => c,
        Err(msg) => return Ok(ephemeral(msg)),
    };

    let member = &community.members[&member_id];

    let bag_text = if member.bag.is_empty() {
        "_Your bag is empty._".to_string()
    } else {
        let mut fruits: Vec<_> = member.bag.iter().collect();
        fruits.sort_by(|(a, _), (b, _)| {
            a.category.cmp(&b.category).then(
                a.rarity()
                    .partial_cmp(&b.rarity())
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        fruits
            .iter()
            .map(|(f, n)| format!("{} {} ×{n}", f.emoji, f.name))
            .collect::<Vec<_>>()
            .join("\n")
    };

    Ok(ephemeral(format!("*Your bag*\n{bag_text}")))
}

async fn gift<E, CR, ELR>(
    community_store: &CommunityStore<&CR, &ELR>,
    event_log_store: &EventLogStore<&ELR>,
    community_id: CommunityId,
    sender_id: MemberId,
    workspace_ns: Uuid,
    args: &[&str],
) -> Result<Value, Error>
where
    E: DbError,
    CR: CommunityRepo + CommunityProvider<Error = E>,
    ELR: EventLogRepo + EventLogProvider<Error = E>,
{
    if args.len() < 2 {
        return Ok(ephemeral("Usage: `/fruit gift <@user> <emoji> [message]`"));
    }

    let recipient_slack_id = match parse_slack_mention(args[0]) {
        Some(id) => id,
        None => {
            return Ok(ephemeral(format!(
                "Could not parse `{}` as a Slack mention. Try `/fruit gift <@user> <emoji>`.",
                args[0]
            )));
        }
    };

    let emoji = args[1];
    let message = if args.len() > 2 {
        Some(args[2..].join(" "))
    } else {
        None
    };

    let fruit = match FRUITS.iter().find(|f| f.emoji == emoji) {
        Some(f) => *f,
        None => return Ok(ephemeral(format!("Unknown fruit emoji `{emoji}`."))),
    };

    let recipient_id = crate::identity::member_id_for(workspace_ns, recipient_slack_id);

    let community = match require_member(community_store, community_id, sender_id).await? {
        Ok(c) => c,
        Err(msg) => return Ok(ephemeral(msg)),
    };

    if !community.members.contains_key(&recipient_id) {
        return Ok(ephemeral(
            "The recipient is not a member of this community.",
        ));
    }

    let mutations = compute_gift(&community, sender_id, recipient_id, fruit);
    if mutations.is_empty() {
        return Ok(ephemeral(format!(
            "You don't hold {} in your bag.",
            fruit.emoji
        )));
    }

    let sender_name = &community.members[&sender_id].display_name;
    let recipient_name = &community.members[&recipient_id].display_name;

    event_log_store
        .append_event(
            community_id,
            EventPayload::Gift {
                sender_id,
                recipient_id,
                fruit,
                message: message.clone(),
            },
        )
        .await
        .map_err(storage)?;

    let msg_suffix = message.map(|m| format!("\n_{m}_")).unwrap_or_default();
    Ok(in_channel(format!(
        "*{sender_name}* gifted {emoji} to *{recipient_name}*{msg_suffix}"
    )))
}

async fn burn<E, CR, ELR>(
    community_store: &CommunityStore<&CR, &ELR>,
    event_log_store: &EventLogStore<&ELR>,
    community_id: CommunityId,
    member_id: MemberId,
    display_name: &str,
    args: &[&str],
) -> Result<Value, Error>
where
    E: DbError,
    CR: CommunityRepo + CommunityProvider<Error = E>,
    ELR: EventLogRepo + EventLogProvider<Error = E>,
{
    if args.is_empty() {
        return Ok(ephemeral(
            "Usage: `/fruit burn <emoji> [<emoji>...] [message]`",
        ));
    }

    let (fruits, message) = match parse_emojis_and_message(args) {
        Ok(result) => result,
        Err(msg) => return Ok(ephemeral(msg)),
    };

    let community = match require_member(community_store, community_id, member_id).await? {
        Ok(c) => c,
        Err(msg) => return Ok(ephemeral(msg)),
    };

    let mutations = compute_burn(&community, member_id, &fruits);
    if mutations.is_empty() {
        return Ok(ephemeral("You don't hold any of those fruits."));
    }

    let emoji_str = fruits.iter().map(|f| f.emoji).collect::<Vec<_>>().join(" ");

    event_log_store
        .append_event(
            community_id,
            EventPayload::Burn {
                member_id,
                fruits,
                message: message.clone(),
            },
        )
        .await
        .map_err(storage)?;

    let msg_suffix = message.map(|m| format!("\n_{m}_")).unwrap_or_default();
    Ok(in_channel(format!(
        "*{display_name}* burned {emoji_str} 🔥{msg_suffix}"
    )))
}

fn help() -> Value {
    ephemeral(
        "*`/fruit` commands*\n\
        • `join` — join this community\n\
        • `leave` — leave this community\n\
        • `bag` — show your bag\n\
        • `gift <@user> <emoji> [message]` — gift a fruit\n\
        • `burn <emoji> [<emoji>...] [message]` — burn fruits\n\
        • `help` — show this message",
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn require_member<E, CR, ELP>(
    store: &CommunityStore<CR, ELP>,
    community_id: CommunityId,
    member_id: MemberId,
) -> Result<Result<Community, &'static str>, Error>
where
    E: DbError,
    CR: CommunityRepo + CommunityProvider<Error = E>,
    ELP: fruit_domain::event_log_repo::EventLogProvider<Error = E>,
{
    match store.get_latest(community_id).await.map_err(storage)? {
        None => Ok(Err(
            "No community here yet. Use `/fruit join` to start one.",
        )),
        Some(c) if !c.members.contains_key(&member_id) => Ok(Err(
            "You're not a member of this community. Use `/fruit join` first.",
        )),
        Some(c) => Ok(Ok(c)),
    }
}

/// Extracts the raw Slack user ID from a mention token like `<@U12345>` or `<@U12345|name>`.
fn parse_slack_mention(s: &str) -> Option<&str> {
    let inner = s.strip_prefix("<@")?.strip_suffix('>')?;
    Some(inner.split('|').next().unwrap_or(inner))
}

/// Greedily parses leading tokens as fruit emoji, then treats any remaining tokens as an
/// optional message. Returns an error if no valid emoji is found or if a token that starts
/// with non-emoji characters precedes the first emoji.
fn parse_emojis_and_message(
    tokens: &[&str],
) -> Result<(Vec<fruit_domain::fruit::Fruit>, Option<String>), String> {
    let mut fruits = Vec::new();
    let mut message_start = tokens.len();

    for (i, token) in tokens.iter().enumerate() {
        if !FRUITS.iter().any(|f| token.starts_with(f.emoji)) {
            message_start = i;
            break;
        }
        let mut remaining = *token;
        while !remaining.is_empty() {
            match FRUITS.iter().find(|f| remaining.starts_with(f.emoji)) {
                Some(f) => {
                    fruits.push(*f);
                    remaining = &remaining[f.emoji.len()..];
                }
                None => return Err(format!("Unknown fruit emoji in `{token}`.")),
            }
        }
    }

    if fruits.is_empty() {
        return Err(format!("Unknown fruit emoji in `{}`.", tokens[0]));
    }

    let message = if message_start < tokens.len() {
        Some(tokens[message_start..].join(" "))
    } else {
        None
    };

    Ok((fruits, message))
}

fn storage<E: std::error::Error + Send + Sync + 'static>(e: exn::Exn<E>) -> Error {
    Error::Storage(e.to_string())
}

fn ephemeral(text: impl Into<String>) -> Value {
    json!({
        "response_type": "ephemeral",
        "blocks": [{"type": "section", "text": {"type": "mrkdwn", "text": text.into()}}]
    })
}

fn in_channel(text: impl Into<String>) -> Value {
    json!({
        "response_type": "in_channel",
        "blocks": [{"type": "section", "text": {"type": "mrkdwn", "text": text.into()}}]
    })
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
