use exn::Exn;
use fruit_domain::{
    community::CommunityId,
    community_repo::{CommunityProvider, CommunityRepo},
    community_store::CommunityStore,
    error::DbError,
    event_log::StateMutation,
    event_log_repo::{EventLogProvider, EventLogRepo},
    providence::Providence,
    random_granter::RandomGranter,
};
use rand::thread_rng;
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

/// Runs a fruit grant for the community identified by `detail` and posts a
/// summary to the Slack channel.
///
/// Returns `Ok(())` immediately if no community exists yet or the community
/// has no members. Both cases are idempotent no-ops.
pub async fn handle_grant<E, CR, ELR, N>(
    community_store: &CommunityStore<CR, ELR>,
    community_repo: CR,
    event_log_repo: ELR,
    notifier: &N,
    detail: &GrantDetail,
) -> Result<(), Exn<Error>>
where
    E: DbError,
    CR: CommunityRepo + CommunityProvider<Error = E>,
    ELR: EventLogRepo + EventLogProvider<Error = E>,
    N: Notifier,
{
    let community_id = detail.community_id();

    let community = match community_store
        .get_latest(community_id)
        .await
        .map_err(|e| GrantError::raise(community_id, e))?
    {
        None => return Ok(()),
        Some(c) if c.members.is_empty() => return Ok(()),
        Some(c) => c,
    };

    let mut providence = Providence::new(
        event_log_repo,
        community_repo,
        RandomGranter::new(thread_rng()),
    );

    let mutations = providence
        .grant_fruit(&community, detail.count)
        .await
        .map_err(|e| GrantError::raise(community_id, e))?;

    let text = format_grant_summary(&mutations);
    notifier.post_message(&detail.channel_id, &text).await?;

    Ok(())
}

fn format_grant_summary(mutations: &[StateMutation]) -> String {
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

#[cfg(test)]
#[path = "grant_tests.rs"]
mod tests;
