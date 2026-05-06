use std::collections::HashMap;

use exn::Exn;
use fruit_domain::{
    community::CommunityId,
    community_repo::{CommunityProvider, CommunityRepo},
    community_store::CommunityStore,
    error::DbError,
    event_log::StateMutation,
    event_log_repo::{EventLogProvider, EventLogRepo},
    fruit::Fruit,
    granter::Granter,
    member::{ExternalSystem, MemberId},
    providence::Providence,
};
use serde::Deserialize;

use crate::{
    error::{Error, GrantError},
    identity,
    notify::Notifier,
};

/// EventBridge event detail for a scheduled fruit grant.
#[derive(Debug, Deserialize)]
pub struct GrantDetail {
    /// Slack workspace ID (e.g. `"T012AB3C4"`). Used with `channel_id` to
    /// derive the [`CommunityId`] via the same UUIDv5 identity mapping as the
    /// slash command handler.
    pub team_id: String,
    /// Slack channel ID (e.g. `"C012AB3C4"`).
    pub channel_id: String,
    /// Number of fruits to grant per member.
    pub count: usize,
}

impl GrantDetail {
    /// Derives the [`CommunityId`] for this grant's channel.
    pub fn community_id(&self) -> CommunityId {
        let ns = identity::workspace_namespace(&self.team_id);
        identity::community_id_for(ns, &self.channel_id)
    }
}

/// Runs a fruit grant for the community identified by `detail`, posts a
/// summary to the Slack channel, and DMs each recipient their fruits.
///
/// Returns `Ok(())` immediately if no community exists yet or the community
/// has no members. Both cases are idempotent no-ops.
pub async fn handle_grant<E, CR, ELR, G, N>(
    community_store: &CommunityStore<CR, ELR>,
    providence: &mut Providence<ELR, CR, G>,
    notifier: &N,
    detail: &GrantDetail,
) -> Result<(), Exn<Error>>
where
    E: DbError,
    CR: CommunityRepo + CommunityProvider<Error = E>,
    ELR: EventLogRepo + EventLogProvider<Error = E>,
    G: Granter,
    N: Notifier,
{
    let community_id = detail.community_id();

    let community = match community_store
        .get_latest(community_id)
        .await
        .map_err(|e| GrantError::raise(community_id, &detail.channel_id, detail.count, e))?
    {
        None => return Ok(()),
        Some(c) if c.members.is_empty() => return Ok(()),
        Some(c) => c,
    };

    let mutations = providence
        .grant_fruit(&community, detail.count)
        .await
        .map_err(|e| GrantError::raise(community_id, &detail.channel_id, detail.count, e))?;

    let text = format_channel_summary(&mutations);
    notifier.post_message(&detail.channel_id, &text).await?;

    for (member_id, fruits) in fruits_by_member(&mutations) {
        if let Some(member) = community.members.get(&member_id) {
            if let Some(ref ext) = member.external_id {
                if ext.system == ExternalSystem::Slack {
                    let dm = format_member_dm(&fruits);
                    notifier.post_dm(&ext.id, &dm).await?;
                }
            }
        }
    }

    Ok(())
}

fn fruits_by_member(mutations: &[StateMutation]) -> HashMap<MemberId, Vec<Fruit>> {
    let mut map: HashMap<MemberId, Vec<Fruit>> = HashMap::new();
    for m in mutations {
        if let StateMutation::AddFruitToMember { member_id, fruit } = m {
            map.entry(*member_id).or_default().push(*fruit);
        }
    }
    map
}

fn format_channel_summary(mutations: &[StateMutation]) -> String {
    let fruit_count = mutations
        .iter()
        .filter(|m| matches!(m, StateMutation::AddFruitToMember { .. }))
        .count();

    if fruit_count == 0 {
        "\u{1f333} Fruit grant complete \u{2014} no fruits distributed.".to_string()
    } else {
        format!("\u{1f333} Fruit grant complete \u{2014} {fruit_count} fruit(s) distributed!")
    }
}

fn format_member_dm(fruits: &[Fruit]) -> String {
    let list = fruits
        .iter()
        .map(|f| format!("{} {}", f.emoji, f.name))
        .collect::<Vec<_>>()
        .join(", ");
    format!("\u{1f333} You received {list} in the fruit grant!")
}

#[cfg(test)]
#[path = "grant_tests.rs"]
mod tests;
