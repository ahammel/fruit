use fruit_domain::{community::CommunityId, member::MemberId};
use uuid::Uuid;

/// Derives the workspace-scoped UUID v5 namespace from a Slack `team_id`.
pub fn workspace_namespace(team_id: &str) -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, team_id.as_bytes())
}

/// Derives the [`CommunityId`] for a Slack channel within a workspace.
pub fn community_id_for(workspace_ns: Uuid, channel_id: &str) -> CommunityId {
    CommunityId::from(Uuid::new_v5(&workspace_ns, channel_id.as_bytes()))
}

/// Derives the [`MemberId`] for a Slack user within a workspace.
pub fn member_id_for(workspace_ns: Uuid, slack_user_id: &str) -> MemberId {
    MemberId::from(Uuid::new_v5(&workspace_ns, slack_user_id.as_bytes()))
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
