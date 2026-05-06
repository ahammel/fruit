use fruit_domain::{
    community::CommunityId,
    community_repo::CommunityPersistor,
    community_store::CommunityStore,
    event_log::{EventPayload, StateMutation},
    event_log_store::EventLogStore,
    fruit::FRUITS,
    member::{Member, MemberId},
};
use fruit_in_memory_db::{
    community_repo::InMemoryCommunityRepo, event_log_repo::InMemoryEventLogRepo,
};
use uuid::Uuid;

use super::dispatch;

// ── Test fixtures ─────────────────────────────────────────────────────────────

const CHANNEL: &str = "CTEST";
const NS: Uuid = Uuid::nil();

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

fn stores<'a>(
    cr: &'a InMemoryCommunityRepo,
    elr: &'a InMemoryEventLogRepo,
) -> (
    CommunityStore<&'a InMemoryCommunityRepo, &'a InMemoryEventLogRepo>,
    EventLogStore<&'a InMemoryEventLogRepo>,
) {
    (CommunityStore::new(cr, elr), EventLogStore::new(elr))
}

/// Provisions a community and adds alice as a member with effects applied,
/// so subsequent `get_latest` calls reflect her membership.
async fn setup_alice(
    community_repo: &InMemoryCommunityRepo,
    event_log_repo: &InMemoryEventLogRepo,
) -> (CommunityId, MemberId) {
    let (community_id, member_id) = ids();
    community_repo
        .put(fruit_domain::community::Community::new().with_id(community_id))
        .await
        .unwrap();
    let event_log = EventLogStore::new(event_log_repo);
    let event = event_log
        .append_event(
            community_id,
            EventPayload::AddMember {
                display_name: "alice".to_string(),
                member_id,
                external_id: None,
            },
        )
        .await
        .unwrap();
    event_log
        .append_effect(
            event.id,
            community_id,
            vec![StateMutation::AddMember {
                member: Member::new("alice").with_id(member_id),
            }],
        )
        .await
        .unwrap();
    (community_id, member_id)
}

/// Adds bob to an already-provisioned community with effects applied.
async fn setup_bob(
    event_log_repo: &InMemoryEventLogRepo,
    community_id: CommunityId,
    bob_id: MemberId,
) {
    let event_log = EventLogStore::new(event_log_repo);
    let event = event_log
        .append_event(
            community_id,
            EventPayload::AddMember {
                display_name: "bob".to_string(),
                member_id: bob_id,
                external_id: None,
            },
        )
        .await
        .unwrap();
    event_log
        .append_effect(
            event.id,
            community_id,
            vec![StateMutation::AddMember {
                member: Member::new("bob").with_id(bob_id),
            }],
        )
        .await
        .unwrap();
}

/// Provisions a community, adds alice and bob, gives alice a 🍎 via a grant effect.
async fn setup_gift_scenario(
    community_repo: &InMemoryCommunityRepo,
    event_log_repo: &InMemoryEventLogRepo,
) -> (CommunityId, MemberId, MemberId) {
    let (community_id, alice_id) = setup_alice(community_repo, event_log_repo).await;
    let bob_id = bob_id();
    setup_bob(event_log_repo, community_id, bob_id).await;

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
async fn join_provisions_community_and_records_event() {
    let (cr, elr) = repos();
    let (cs, els) = stores(&cr, &elr);
    let (community_id, member_id) = ids();

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "join",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");

    let event_log = EventLogStore::new(&elr);
    let records = event_log
        .get_records_before()
        .community_id(community_id)
        .limit(10)
        .call()
        .await
        .unwrap();
    assert!(records.iter().any(|r| matches!(
        &r.event.payload,
        EventPayload::AddMember { member_id: m, .. } if *m == member_id
    )));
}

#[tokio::test]
async fn join_second_member_does_not_reprovision() {
    let (cr, elr) = repos();
    let (community_id, _alice_id) = setup_alice(&cr, &elr).await;
    let bob_id = bob_id();
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(&cs, &els, community_id, bob_id, "bob", "U_BOB", NS, "join")
        .await
        .unwrap();

    assert_eq!(response["response_type"], "ephemeral");

    let event_log = EventLogStore::new(&elr);
    let records = event_log
        .get_records_before()
        .community_id(community_id)
        .limit(10)
        .call()
        .await
        .unwrap();
    assert!(records.iter().any(|r| matches!(
        &r.event.payload,
        EventPayload::AddMember { member_id: m, .. } if *m == bob_id
    )));
}

#[tokio::test]
async fn join_already_member_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (community_id, member_id) = setup_alice(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "join",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("already"), "got: {text}");
}

// ── leave ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn leave_records_remove_member_event() {
    let (cr, elr) = repos();
    let (community_id, member_id) = setup_alice(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "leave",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");

    let event_log = EventLogStore::new(&elr);
    let records = event_log
        .get_records_before()
        .community_id(community_id)
        .limit(10)
        .call()
        .await
        .unwrap();
    assert!(records.iter().any(|r| matches!(
        &r.event.payload,
        EventPayload::RemoveMember { member_id: m } if *m == member_id
    )));
}

#[tokio::test]
async fn leave_not_member_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (community_id, _) = setup_alice(&cr, &elr).await;
    let outsider_id = bob_id();
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        outsider_id,
        "bob",
        "U_BOB",
        NS,
        "leave",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

#[tokio::test]
async fn leave_no_community_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (cs, els) = stores(&cr, &elr);
    let (community_id, member_id) = ids();

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "leave",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── bag ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn bag_shows_empty_bag_without_luck() {
    let (cr, elr) = repos();
    let (community_id, member_id) = setup_alice(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
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
    assert!(
        !text.contains("luck"),
        "luck must not be shown, got: {text}"
    );
}

#[tokio::test]
async fn bag_not_member_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (cs, els) = stores(&cr, &elr);
    let (community_id, _) = ids();

    let response = dispatch(
        &cs,
        &els,
        community_id,
        crate::identity::member_id_for(NS, "U_STRANGER"),
        "stranger",
        "U_STRANGER",
        NS,
        "bag",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── gift ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn gift_records_gift_event() {
    let (cr, elr) = repos();
    let (community_id, alice_id, _bob_id) = setup_gift_scenario(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        alice_id,
        "alice",
        "U_ALICE",
        NS,
        "gift <@U_BOB> 🍎 here you go",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "in_channel");

    let event_log = EventLogStore::new(&elr);
    let records = event_log
        .get_records_before()
        .community_id(community_id)
        .limit(10)
        .call()
        .await
        .unwrap();
    assert!(records.iter().any(|r| matches!(
        &r.event.payload,
        EventPayload::Gift { sender_id, .. } if *sender_id == alice_id
    )));
}

#[tokio::test]
async fn gift_missing_args_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (community_id, member_id) = setup_alice(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "gift",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

#[tokio::test]
async fn gift_unknown_emoji_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (community_id, member_id) = setup_alice(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "gift <@U_BOB> 🚀",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

#[tokio::test]
async fn gift_fruit_not_held_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (community_id, alice_id, _) = setup_gift_scenario(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    // Alice holds 🍎 but not 🍋
    let response = dispatch(
        &cs,
        &els,
        community_id,
        alice_id,
        "alice",
        "U_ALICE",
        NS,
        "gift <@U_BOB> 🍋",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

#[tokio::test]
async fn gift_recipient_not_member_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (community_id, alice_id, _) = setup_gift_scenario(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        alice_id,
        "alice",
        "U_ALICE",
        NS,
        "gift <@U_STRANGER> 🍎",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── burn ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn burn_records_burn_event() {
    let (cr, elr) = repos();
    let (community_id, alice_id, _) = setup_gift_scenario(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        alice_id,
        "alice",
        "U_ALICE",
        NS,
        "burn 🍎",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "in_channel");

    let event_log = EventLogStore::new(&elr);
    let records = event_log
        .get_records_before()
        .community_id(community_id)
        .limit(10)
        .call()
        .await
        .unwrap();
    assert!(records.iter().any(|r| matches!(
        &r.event.payload,
        EventPayload::Burn { member_id: m, .. } if *m == alice_id
    )));
}

#[tokio::test]
async fn burn_with_message_includes_message_in_event_and_response() {
    let (cr, elr) = repos();
    let (community_id, alice_id, _) = setup_gift_scenario(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        alice_id,
        "alice",
        "U_ALICE",
        NS,
        "burn 🍎 for the good of all",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "in_channel");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("for the good of all"), "got: {text}");

    let event_log = EventLogStore::new(&elr);
    let records = event_log
        .get_records_before()
        .community_id(community_id)
        .limit(10)
        .call()
        .await
        .unwrap();
    let burn = records
        .iter()
        .find(|r| {
            matches!(&r.event.payload, EventPayload::Burn { member_id: m, .. } if *m == alice_id)
        })
        .unwrap();
    assert!(matches!(
        &burn.event.payload,
        EventPayload::Burn { message: Some(m), .. } if m == "for the good of all"
    ));
}

#[tokio::test]
async fn burn_missing_args_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (community_id, member_id) = setup_alice(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "burn",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

#[tokio::test]
async fn burn_not_holding_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (community_id, member_id) = setup_alice(&cr, &elr).await;
    let (cs, els) = stores(&cr, &elr);

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "burn 🍎",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── help ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn help_returns_ephemeral_response() {
    let (cr, elr) = repos();
    let (cs, els) = stores(&cr, &elr);
    let (community_id, member_id) = ids();

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "help",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("join"), "got: {text}");
    assert!(text.contains("gift"), "got: {text}");
    assert!(
        !text.contains("luck"),
        "luck must not appear in help, got: {text}"
    );
    assert!(
        !text.contains("leaderboard"),
        "leaderboard must not appear in help, got: {text}"
    );
}

// ── unknown subcommand ────────────────────────────────────────────────────────

#[tokio::test]
async fn unknown_subcommand_returns_ephemeral_error() {
    let (cr, elr) = repos();
    let (cs, els) = stores(&cr, &elr);
    let (community_id, member_id) = ids();

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "frobnicate",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
}

// ── luck / leaderboard — removed ─────────────────────────────────────────────

#[tokio::test]
async fn luck_is_unrecognised_subcommand() {
    let (cr, elr) = repos();
    let (cs, els) = stores(&cr, &elr);
    let (community_id, member_id) = ids();

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "luck",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("Unknown"), "got: {text}");
}

#[tokio::test]
async fn leaderboard_is_unrecognised_subcommand() {
    let (cr, elr) = repos();
    let (cs, els) = stores(&cr, &elr);
    let (community_id, member_id) = ids();

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "leaderboard",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("Unknown"), "got: {text}");
}

// ── empty text (defaults to help) ────────────────────────────────────────────

#[tokio::test]
async fn empty_text_returns_help() {
    let (cr, elr) = repos();
    let (cs, els) = stores(&cr, &elr);
    let (community_id, member_id) = ids();

    let response = dispatch(
        &cs,
        &els,
        community_id,
        member_id,
        "alice",
        "U_ALICE",
        NS,
        "",
    )
    .await
    .unwrap();

    assert_eq!(response["response_type"], "ephemeral");
    let text = response["blocks"][0]["text"]["text"].as_str().unwrap();
    assert!(text.contains("join"), "got: {text}");
}

// ── CommunityStore used directly in tests ────────────────────────────────────

#[tokio::test]
async fn store_reflects_setup_alice_membership() {
    let (cr, elr) = repos();
    let (community_id, member_id) = setup_alice(&cr, &elr).await;

    let store = CommunityStore::new(&cr, &elr);
    let community = store.get_latest(community_id).await.unwrap().unwrap();
    assert!(community.members.contains_key(&member_id));
}
