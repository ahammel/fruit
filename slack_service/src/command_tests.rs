use fruit_domain::{
    community::CommunityId,
    community_store::CommunityStore,
    event_log::{EventPayload, StateMutation},
    event_log_store::EventLogStore,
    fruit::FRUITS,
    member::MemberId,
};
use fruit_in_memory_db::{
    community_repo::InMemoryCommunityRepo, event_log_repo::InMemoryEventLogRepo,
};
use uuid::Uuid;

use super::dispatch;

// ── Test fixtures ─────────────────────────────────────────────────────────────

const CHANNEL: &str = "CTEST";
const NS: Uuid = Uuid::nil(); // deterministic namespace for tests

fn repos() -> (InMemoryCommunityRepo, InMemoryEventLogRepo) {
    (InMemoryCommunityRepo::new(), InMemoryEventLogRepo::new())
}

fn ids() -> (CommunityId, MemberId) {
    let community_id = crate::identity::community_id_for(NS, CHANNEL);
    let member_id = crate::identity::member_id_for(NS, "U_ALICE");
    (community_id, member_id)
}

fn bob_id() -> MemberId {
    crate::identity::member_id_for(NS, "U_BOB")
}

/// Provisions a community and adds `alice` as a member, returning the community ID and member ID.
async fn setup_alice(
    community_repo: &InMemoryCommunityRepo,
    event_log_repo: &InMemoryEventLogRepo,
) -> (CommunityId, MemberId) {
    let (community_id, member_id) = ids();
    dispatch(
        community_repo,
        event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "join",
    )
    .await
    .unwrap();
    (community_id, member_id)
}

/// Provisions a community, adds alice and bob, gives alice a 🍎.
async fn setup_gift_scenario(
    community_repo: &InMemoryCommunityRepo,
    event_log_repo: &InMemoryEventLogRepo,
) -> (CommunityId, MemberId, MemberId) {
    let (community_id, alice_id) = setup_alice(community_repo, event_log_repo).await;
    let bob_id = bob_id();
    dispatch(
        community_repo,
        event_log_repo,
        community_id,
        bob_id,
        "bob",
        NS,
        "join",
    )
    .await
    .unwrap();

    // Give alice a 🍎 directly via event log
    let apple = *FRUITS.iter().find(|f| f.emoji == "🍎").unwrap();
    let event_log = EventLogStore::new(event_log_repo);
    let event = event_log
        .append_event(community_id, EventPayload::Grant { count: 1 })
        .await
        .unwrap();
    event_log
        .append_effect(
            event.id,
            community_id,
            vec![StateMutation::AddFruitToMember {
                member_id: alice_id,
                fruit: apple,
            }],
        )
        .await
        .unwrap();

    (community_id, alice_id, bob_id)
}

// ── join ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn join_provisions_community_and_adds_member() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = ids();

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "join",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "in_channel");

    let store = CommunityStore::new(&community_repo, &event_log_repo);
    let community = store.get_latest(community_id).await.unwrap().unwrap();
    assert!(community.members.contains_key(&member_id));
}

#[tokio::test]
async fn join_second_member_does_not_reprovision() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, alice_id) = setup_alice(&community_repo, &event_log_repo).await;
    let bob_id = bob_id();

    dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        bob_id,
        "bob",
        NS,
        "join",
    )
    .await
    .unwrap();

    let store = CommunityStore::new(&community_repo, &event_log_repo);
    let community = store.get_latest(community_id).await.unwrap().unwrap();
    assert_eq!(community.members.len(), 2);
    assert!(community.members.contains_key(&alice_id));
    assert!(community.members.contains_key(&bob_id));
}

#[tokio::test]
async fn join_already_member_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = setup_alice(&community_repo, &event_log_repo).await;

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "join",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── leave ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn leave_removes_member() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = setup_alice(&community_repo, &event_log_repo).await;

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "leave",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "in_channel");

    let store = CommunityStore::new(&community_repo, &event_log_repo);
    let community = store.get_latest(community_id).await.unwrap().unwrap();
    assert!(!community.members.contains_key(&member_id));
}

#[tokio::test]
async fn leave_not_member_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, _) = setup_alice(&community_repo, &event_log_repo).await;
    let outsider_id = bob_id();

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        outsider_id,
        "bob",
        NS,
        "leave",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

#[tokio::test]
async fn leave_no_community_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = ids();

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "leave",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── bag ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn bag_shows_empty_bag() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = setup_alice(&community_repo, &event_log_repo).await;

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "bag",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(
        text.contains("empty"),
        "expected empty bag text, got: {text}"
    );
}

#[tokio::test]
async fn bag_not_member_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, _) = ids();

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        crate::identity::member_id_for(NS, "U_STRANGER"),
        "stranger",
        NS,
        "bag",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── gift ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn gift_transfers_fruit() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, alice_id, bob_id) =
        setup_gift_scenario(&community_repo, &event_log_repo).await;

    let bob_slack_id = "U_BOB";
    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        alice_id,
        "alice",
        NS,
        &format!("gift <@{bob_slack_id}> 🍎 here you go"),
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "in_channel");

    let store = CommunityStore::new(&community_repo, &event_log_repo);
    let community = store.get_latest(community_id).await.unwrap().unwrap();
    let apple = *FRUITS.iter().find(|f| f.emoji == "🍎").unwrap();
    assert_eq!(community.members[&alice_id].bag.count(apple), 0);
    assert_eq!(community.members[&bob_id].bag.count(apple), 1);
}

#[tokio::test]
async fn gift_missing_args_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = setup_alice(&community_repo, &event_log_repo).await;

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "gift",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

#[tokio::test]
async fn gift_unknown_emoji_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = setup_alice(&community_repo, &event_log_repo).await;

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "gift <@U_BOB> 🚀",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

#[tokio::test]
async fn gift_fruit_not_held_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, alice_id, _) = setup_gift_scenario(&community_repo, &event_log_repo).await;

    // Alice holds 🍎 but not 🍋
    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        alice_id,
        "alice",
        NS,
        "gift <@U_BOB> 🍋",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

#[tokio::test]
async fn gift_recipient_not_member_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, alice_id, _) = setup_gift_scenario(&community_repo, &event_log_repo).await;

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        alice_id,
        "alice",
        NS,
        "gift <@U_STRANGER> 🍎",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── burn ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn burn_removes_fruit() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, alice_id, _) = setup_gift_scenario(&community_repo, &event_log_repo).await;

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        alice_id,
        "alice",
        NS,
        "burn 🍎",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "in_channel");

    let store = CommunityStore::new(&community_repo, &event_log_repo);
    let community = store.get_latest(community_id).await.unwrap().unwrap();
    let apple = *FRUITS.iter().find(|f| f.emoji == "🍎").unwrap();
    assert_eq!(community.members[&alice_id].bag.count(apple), 0);
}

#[tokio::test]
async fn burn_missing_args_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = setup_alice(&community_repo, &event_log_repo).await;

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "burn",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

#[tokio::test]
async fn burn_not_holding_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = setup_alice(&community_repo, &event_log_repo).await;

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "burn 🍎",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── luck ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn luck_shows_community_and_member_luck() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = setup_alice(&community_repo, &event_log_repo).await;

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "luck",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("Community:"), "got: {text}");
    assert!(text.contains("You:"), "got: {text}");
}

#[tokio::test]
async fn luck_non_member_shows_only_community_luck() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, _) = setup_alice(&community_repo, &event_log_repo).await;
    let outsider_id = bob_id();

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        outsider_id,
        "bob",
        NS,
        "luck",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("Community:"), "got: {text}");
    assert!(!text.contains("You:"), "got: {text}");
}

// ── leaderboard ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn leaderboard_lists_members() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, _) = setup_alice(&community_repo, &event_log_repo).await;
    dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        bob_id(),
        "bob",
        NS,
        "join",
    )
    .await
    .unwrap();

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        crate::identity::member_id_for(NS, "U_ALICE"),
        "alice",
        NS,
        "leaderboard",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "in_channel");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("alice"), "got: {text}");
    assert!(text.contains("bob"), "got: {text}");
}

// ── help ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn help_returns_ephemeral_response() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = ids();

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "help",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("join"), "got: {text}");
    assert!(text.contains("gift"), "got: {text}");
}

// ── unknown subcommand ────────────────────────────────────────────────────────

#[tokio::test]
async fn unknown_subcommand_returns_ephemeral_error() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = ids();

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "frobnicate",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── empty text (defaults to help) ────────────────────────────────────────────

#[tokio::test]
async fn empty_text_returns_help() {
    let (community_repo, event_log_repo) = repos();
    let (community_id, member_id) = ids();

    let response = dispatch(
        &community_repo,
        &event_log_repo,
        community_id,
        member_id,
        "alice",
        NS,
        "",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("join"), "got: {text}");
}
