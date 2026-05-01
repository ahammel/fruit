//! Integration tests for `DynamoDbEventLogRepo` and `DynamoDbCommunityRepo`.
//!
//! Requires a running Localstack instance. Set `LOCALSTACK_ENDPOINT` to the
//! DynamoDB endpoint URL (e.g. `http://localhost:4566`). Tests are skipped when
//! the environment variable is absent. The Makefile `ti` target sets it automatically
//! after starting Localstack via Docker Compose.

use aws_sdk_dynamodb::{
    config::{BehaviorVersion, Credentials, Region},
    error::ProvideErrorMetadata,
    types::{AttributeDefinition, BillingMode, KeySchemaElement, KeyType, ScalarAttributeType},
    Client,
};
use fruit_domain::{
    community::{Community, CommunityId},
    community_repo::{CommunityPersistor, CommunityProvider},
    event_log::{EventPayload, Record, SequenceId, StateMutation},
    event_log_repo::{EventLogPersistor, EventLogProvider},
    member::MemberId,
};
use fruit_dynamo_db::{
    community_repo::DynamoDbCommunityRepo, event_log_repo::DynamoDbEventLogRepo,
};
use uuid::Uuid;

// ── Constants ─────────────────────────────────────────────────────────────────

const TABLE_NAME: &str = "fruit-integration-test";

// ── Client and table setup ────────────────────────────────────────────────────

fn localstack_endpoint() -> Option<String> {
    std::env::var("LOCALSTACK_ENDPOINT").ok()
}

async fn make_client(endpoint: &str) -> Client {
    let config = aws_sdk_dynamodb::Config::builder()
        .behavior_version(BehaviorVersion::v2026_01_12())
        .endpoint_url(endpoint)
        .region(Region::new("us-east-1"))
        .credentials_provider(Credentials::new("test", "test", None, None, "test"))
        .build();
    Client::from_conf(config)
}

/// Creates the single-table schema if it does not already exist.
async fn ensure_table(client: &Client) {
    let result = client
        .create_table()
        .table_name(TABLE_NAME)
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("pk")
                .attribute_type(ScalarAttributeType::B)
                .build()
                .unwrap(),
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("sk")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .unwrap(),
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("pk")
                .key_type(KeyType::Hash)
                .build()
                .unwrap(),
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("sk")
                .key_type(KeyType::Range)
                .build()
                .unwrap(),
        )
        .billing_mode(BillingMode::PayPerRequest)
        .send()
        .await;
    match result {
        Ok(_) => {}
        Err(e) if e.code() == Some("ResourceInUseException") => {}
        Err(e) => panic!("failed to create table: {e}"),
    }
}

/// Returns `None` when `LOCALSTACK_ENDPOINT` is unset (test should skip).
async fn setup() -> Option<(DynamoDbEventLogRepo, DynamoDbCommunityRepo)> {
    let ep = localstack_endpoint()?;
    let client = make_client(&ep).await;
    ensure_table(&client).await;
    let event_log = DynamoDbEventLogRepo::new(client.clone(), TABLE_NAME);
    let community = DynamoDbCommunityRepo::new(client, TABLE_NAME);
    Some((event_log, community))
}

fn new_community_id() -> CommunityId {
    CommunityId::from(Uuid::new_v4())
}

fn seq(n: u64) -> SequenceId {
    SequenceId::new(n)
}

fn make_community(id: CommunityId, version: SequenceId) -> Community {
    Community::new().with_id(id).with_version(version)
}

// ── EventLogRepo integration paths ───────────────────────────────────────────

#[tokio::test]
async fn event_log_append_and_get_record_without_effect() {
    let Some((repo, _)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let event = repo
        .append_event(cid, EventPayload::Grant { count: 3 })
        .await
        .unwrap();
    let record = repo.get_record(cid, event.id).await.unwrap();
    assert_eq!(
        record,
        Some(Record {
            event,
            effect: None
        })
    );
}

#[tokio::test]
async fn event_log_append_event_and_effect_then_get_record() {
    let Some((repo, _)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let event = repo
        .append_event(cid, EventPayload::Grant { count: 1 })
        .await
        .unwrap();
    let mutations = vec![StateMutation::BurnLuckBonus { delta: 5_i16 }];
    let effect = repo.append_effect(event.id, cid, mutations).await.unwrap();
    let record = repo.get_record(cid, event.id).await.unwrap();
    assert_eq!(
        record,
        Some(Record {
            event,
            effect: Some(effect)
        })
    );
}

#[tokio::test]
async fn event_log_get_effect_for_event_returns_effect() {
    let Some((repo, _)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let event = repo
        .append_event(cid, EventPayload::Grant { count: 1 })
        .await
        .unwrap();
    let effect = repo.append_effect(event.id, cid, vec![]).await.unwrap();
    let result = repo.get_effect_for_event(cid, event.id).await.unwrap();
    assert_eq!(result, Some(effect));
}

#[tokio::test]
async fn event_log_get_effect_for_event_returns_none_when_absent() {
    let Some((repo, _)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let event = repo
        .append_event(cid, EventPayload::Grant { count: 1 })
        .await
        .unwrap();
    let result = repo.get_effect_for_event(cid, event.id).await.unwrap();
    assert_eq!(result, None);
}

#[tokio::test]
async fn event_log_get_effects_after_returns_ascending_effects() {
    let Some((repo, _)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let mut effects = Vec::new();
    for i in 1..=3usize {
        let event = repo
            .append_event(cid, EventPayload::Grant { count: i })
            .await
            .unwrap();
        let effect = repo.append_effect(event.id, cid, vec![]).await.unwrap();
        effects.push(effect);
    }
    let result = repo.get_effects_after(cid, 10, seq(0)).await.unwrap();
    assert_eq!(result, effects);
}

#[tokio::test]
async fn event_log_get_records_before_returns_descending_by_id() {
    let Some((repo, _)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let mut events = Vec::new();
    for i in 1..=3usize {
        let event = repo
            .append_event(cid, EventPayload::Grant { count: i })
            .await
            .unwrap();
        events.push(event);
    }
    let result = repo.get_records_before(cid, 10, None).await.unwrap();
    let result_ids: Vec<SequenceId> = result.iter().map(|r| r.event.id).collect();
    let expected_ids: Vec<SequenceId> = events.iter().rev().map(|e| e.id).collect();
    assert_eq!(result_ids, expected_ids);
}

#[tokio::test]
async fn event_log_get_latest_grant_events_returns_most_recent_first() {
    let Some((repo, _)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let mut grant_events = Vec::new();
    for i in 1..=3usize {
        let e = repo
            .append_event(cid, EventPayload::Grant { count: i })
            .await
            .unwrap();
        grant_events.push(e);
    }
    let result = repo.get_latest_grant_events(cid, 10).await.unwrap();
    let result_ids: Vec<SequenceId> = result.iter().map(|e| e.id).collect();
    let expected_ids: Vec<SequenceId> = grant_events.iter().rev().map(|e| e.id).collect();
    assert_eq!(result_ids, expected_ids);
}

#[tokio::test]
async fn event_log_get_latest_gift_records_returns_most_recent_first() {
    let Some((repo, _)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let sender = MemberId::from(Uuid::new_v4());
    let recipient = MemberId::from(Uuid::new_v4());
    let fruit = fruit_domain::fruit::GRAPES;
    let mut gift_events = Vec::new();
    for _ in 0..2usize {
        let e = repo
            .append_event(
                cid,
                EventPayload::Gift {
                    sender_id: sender,
                    recipient_id: recipient,
                    fruit,
                    message: None,
                },
            )
            .await
            .unwrap();
        gift_events.push(e);
    }
    let result = repo.get_latest_gift_records(cid, 10).await.unwrap();
    let result_ids: Vec<SequenceId> = result.iter().map(|r| r.event.id).collect();
    let expected_ids: Vec<SequenceId> = gift_events.iter().rev().map(|e| e.id).collect();
    assert_eq!(result_ids, expected_ids);
}

#[tokio::test]
async fn event_log_get_records_between_returns_exclusive_range() {
    let Some((repo, _)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let mut events = Vec::new();
    for i in 1..=5usize {
        let e = repo
            .append_event(cid, EventPayload::Grant { count: i })
            .await
            .unwrap();
        events.push(e);
    }
    // Query strictly between events[0] and events[4] → should return events[1..4]
    let result = repo
        .get_records_between(cid, events[0].id, events[4].id)
        .await
        .unwrap();
    let result_ids: Vec<SequenceId> = result.iter().map(|r| r.event.id).collect();
    let expected_ids: Vec<SequenceId> = events[1..4].iter().map(|e| e.id).collect();
    assert_eq!(result_ids, expected_ids);
}

#[tokio::test]
async fn event_log_append_effect_twice_returns_already_exists() {
    let Some((repo, _)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let event = repo
        .append_event(cid, EventPayload::Grant { count: 1 })
        .await
        .unwrap();
    repo.append_effect(event.id, cid, vec![]).await.unwrap();
    let result = repo.append_effect(event.id, cid, vec![]).await;
    assert!(
        result.is_err(),
        "expected AlreadyExists on duplicate effect"
    );
}

// ── CommunityRepo integration paths ──────────────────────────────────────────

#[tokio::test]
async fn community_put_and_get_by_version() {
    let Some((_, repo)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let community = make_community(cid, seq(1));
    let stored = repo.put(community.clone()).await.unwrap();
    assert_eq!(stored, community);
    let fetched = repo.get(cid, seq(1)).await.unwrap();
    assert_eq!(fetched, Some(community));
}

#[tokio::test]
async fn community_get_missing_version_returns_none() {
    let Some((_, repo)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let result = repo.get(cid, seq(99)).await.unwrap();
    assert_eq!(result, None);
}

#[tokio::test]
async fn community_get_latest_returns_highest_version() {
    let Some((_, repo)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let v1 = make_community(cid, seq(1));
    let v5 = make_community(cid, seq(5));
    repo.put(v1).await.unwrap();
    repo.put(v5.clone()).await.unwrap();
    let result = repo.get_latest(cid).await.unwrap();
    assert_eq!(result, Some(v5));
}

#[tokio::test]
async fn community_get_latest_missing_community_returns_none() {
    let Some((_, repo)) = setup().await else {
        return;
    };
    let result = repo.get_latest(new_community_id()).await.unwrap();
    assert_eq!(result, None);
}

#[tokio::test]
async fn community_put_duplicate_version_returns_already_exists() {
    let Some((_, repo)) = setup().await else {
        return;
    };
    let cid = new_community_id();
    let community = make_community(cid, seq(1));
    repo.put(community.clone()).await.unwrap();
    let result = repo.put(community).await;
    assert!(
        result.is_err(),
        "expected AlreadyExists on duplicate version"
    );
}
