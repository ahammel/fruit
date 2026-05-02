use super::{community_id_for, member_id_for, workspace_namespace};

const TEAM_A: &str = "T00001";
const TEAM_B: &str = "T00002";
const CHANNEL: &str = "C00001";
const USER: &str = "U00001";

#[test]
fn workspace_namespace_is_deterministic() {
    assert_eq!(workspace_namespace(TEAM_A), workspace_namespace(TEAM_A));
}

#[test]
fn workspace_namespace_differs_by_team() {
    assert_ne!(workspace_namespace(TEAM_A), workspace_namespace(TEAM_B));
}

#[test]
fn community_id_is_deterministic() {
    let ns = workspace_namespace(TEAM_A);
    assert_eq!(community_id_for(ns, CHANNEL), community_id_for(ns, CHANNEL));
}

#[test]
fn community_id_differs_across_workspaces() {
    let ns_a = workspace_namespace(TEAM_A);
    let ns_b = workspace_namespace(TEAM_B);
    assert_ne!(
        community_id_for(ns_a, CHANNEL),
        community_id_for(ns_b, CHANNEL)
    );
}

#[test]
fn member_id_is_deterministic() {
    let ns = workspace_namespace(TEAM_A);
    assert_eq!(member_id_for(ns, USER), member_id_for(ns, USER));
}

#[test]
fn member_id_differs_across_workspaces() {
    let ns_a = workspace_namespace(TEAM_A);
    let ns_b = workspace_namespace(TEAM_B);
    assert_ne!(member_id_for(ns_a, USER), member_id_for(ns_b, USER));
}

#[test]
fn different_users_get_different_member_ids() {
    let ns = workspace_namespace(TEAM_A);
    assert_ne!(member_id_for(ns, "U00001"), member_id_for(ns, "U00002"));
}

#[test]
fn different_channels_get_different_community_ids() {
    let ns = workspace_namespace(TEAM_A);
    assert_ne!(
        community_id_for(ns, "C00001"),
        community_id_for(ns, "C00002")
    );
}
