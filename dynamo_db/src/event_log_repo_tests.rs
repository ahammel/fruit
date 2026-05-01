use std::collections::HashMap;

use aws_sdk_dynamodb::{
    operation::{
        batch_get_item::{BatchGetItemError, BatchGetItemOutput},
        get_item::{GetItemError, GetItemOutput},
        put_item::{PutItemError, PutItemOutput},
        query::{QueryError, QueryOutput},
        update_item::{UpdateItemError, UpdateItemOutput},
    },
    types::{
        error::{ConditionalCheckFailedException, InternalServerError},
        AttributeValue,
    },
};
use aws_smithy_mocks::{mock, mock_client, RuleMode};
use fruit_domain::{
    event_log::{Effect, Event, EventPayload, Record, StateMutation},
    event_log_repo::{EventLogPersistor, EventLogProvider},
    fruit::GRAPES,
};
use newtype_ids::IntegerIdentifier as _;

use crate::{
    dto::event::{encode_effect, encode_event},
    error::Error,
    event_log_repo::DynamoDbEventLogRepo,
};

mod test_helpers {
    use fruit_domain::{
        community::CommunityId,
        event_log::{Effect, Event, EventPayload, SequenceId, StateMutation},
        member::MemberId,
    };
    use uuid::Uuid;

    /// A fixed community ID for test reproducibility.
    pub fn community_id() -> CommunityId {
        CommunityId::from(Uuid::from_bytes([20u8; 16]))
    }

    /// A fixed member ID for test reproducibility.
    pub fn member_id() -> MemberId {
        MemberId::from(Uuid::from_bytes([21u8; 16]))
    }

    /// A fixed secondary member ID.
    pub fn member_id_2() -> MemberId {
        MemberId::from(Uuid::from_bytes([22u8; 16]))
    }

    /// Wraps `n` as a `SequenceId`.
    pub fn seq(n: u64) -> SequenceId {
        SequenceId::new(n)
    }

    /// Builds a test Event.
    pub fn make_event(id: SequenceId, payload: EventPayload) -> Event {
        Event {
            id,
            community_id: community_id(),
            payload,
        }
    }

    /// Builds a test Effect.
    pub fn make_effect(id: SequenceId, mutations: Vec<StateMutation>) -> Effect {
        Effect {
            id,
            community_id: community_id(),
            mutations,
        }
    }
}

use test_helpers::*;

// ── Helpers for building mock outputs ────────────────────────────────────────

fn update_item_output_with_counter(n: u64) -> UpdateItemOutput {
    UpdateItemOutput::builder()
        .attributes("n", AttributeValue::N(n.to_string()))
        .build()
}

fn batch_get_output_for_table(
    table: &str,
    items: Vec<HashMap<String, AttributeValue>>,
) -> BatchGetItemOutput {
    let mut responses = HashMap::new();
    responses.insert(table.to_string(), items);
    BatchGetItemOutput::builder()
        .set_responses(Some(responses))
        .build()
}

// ── next_seq_id (via append_event) ───────────────────────────────────────────

#[tokio::test]
async fn append_event_success_returns_event_with_correct_id_and_payload() {
    let payload = EventPayload::Grant { count: 3 };

    let counter_rule = mock!(aws_sdk_dynamodb::Client::update_item)
        .then_output(|| update_item_output_with_counter(1));
    let put_rule =
        mock!(aws_sdk_dynamodb::Client::put_item).then_output(|| PutItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&counter_rule, &put_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let event = repo
        .append_event(community_id(), payload.clone())
        .await
        .unwrap();
    assert_eq!(
        event,
        Event {
            id: seq(1),
            community_id: community_id(),
            payload,
        }
    );
}

#[tokio::test]
async fn append_event_missing_counter_attribute_returns_codec_error() {
    // update_item returns empty attributes — no "n" key
    let counter_rule = mock!(aws_sdk_dynamodb::Client::update_item)
        .then_output(|| UpdateItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&counter_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo
        .append_event(community_id(), EventPayload::Grant { count: 1 })
        .await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Codec { .. })),
        "expected Codec error, got {:?}",
        result
    );
}

#[tokio::test]
async fn append_event_conflict_returns_already_exists() {
    let counter_rule = mock!(aws_sdk_dynamodb::Client::update_item)
        .then_output(|| update_item_output_with_counter(1));
    let put_rule = mock!(aws_sdk_dynamodb::Client::put_item).then_error(|| {
        PutItemError::ConditionalCheckFailedException(
            ConditionalCheckFailedException::builder().build(),
        )
    });
    let client = mock_client!(aws_sdk_dynamodb, [&counter_rule, &put_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo
        .append_event(community_id(), EventPayload::Grant { count: 1 })
        .await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::AlreadyExists { .. })),
        "expected AlreadyExists, got {:?}",
        result
    );
}

#[tokio::test]
async fn append_event_sdk_error_returns_sdk_error() {
    let counter_rule = mock!(aws_sdk_dynamodb::Client::update_item)
        .then_output(|| update_item_output_with_counter(1));
    let put_rule = mock!(aws_sdk_dynamodb::Client::put_item)
        .then_error(|| PutItemError::InternalServerError(InternalServerError::builder().build()));
    let client = mock_client!(aws_sdk_dynamodb, [&counter_rule, &put_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo
        .append_event(community_id(), EventPayload::Grant { count: 1 })
        .await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── append_effect ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn append_effect_success_returns_effect() {
    let mutations = vec![StateMutation::BurnLuckBonus { delta: 5 }];

    let rule =
        mock!(aws_sdk_dynamodb::Client::put_item).then_output(|| PutItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let effect = repo
        .append_effect(seq(2), community_id(), mutations.clone())
        .await
        .unwrap();
    assert_eq!(
        effect,
        Effect {
            id: seq(2),
            community_id: community_id(),
            mutations,
        }
    );
}

#[tokio::test]
async fn append_effect_conflict_returns_already_exists() {
    let rule = mock!(aws_sdk_dynamodb::Client::put_item).then_error(|| {
        PutItemError::ConditionalCheckFailedException(
            ConditionalCheckFailedException::builder().build(),
        )
    });
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo.append_effect(seq(1), community_id(), vec![]).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::AlreadyExists { .. })),
        "expected AlreadyExists, got {:?}",
        result
    );
}

#[tokio::test]
async fn append_effect_sdk_error_returns_sdk_error() {
    let rule = mock!(aws_sdk_dynamodb::Client::put_item)
        .then_error(|| PutItemError::InternalServerError(InternalServerError::builder().build()));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo.append_effect(seq(1), community_id(), vec![]).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── get_record ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_record_found_event_and_effect() {
    let event = make_event(seq(1), EventPayload::Grant { count: 2 });
    let effect = make_effect(seq(1), vec![StateMutation::BurnLuckBonus { delta: 1 }]);
    let event_item = encode_event(&event).unwrap();
    let effect_item = encode_effect(&effect).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::batch_get_item).then_output(move || {
        batch_get_output_for_table("test-table", vec![event_item.clone(), effect_item.clone()])
    });
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo.get_record(community_id(), seq(1)).await.unwrap();
    assert_eq!(
        result,
        Some(Record {
            event,
            effect: Some(effect),
        })
    );
}

#[tokio::test]
async fn get_record_found_event_only() {
    let event = make_event(seq(1), EventPayload::Grant { count: 2 });
    let event_item = encode_event(&event).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(move || batch_get_output_for_table("test-table", vec![event_item.clone()]));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo.get_record(community_id(), seq(1)).await.unwrap();
    assert_eq!(
        result,
        Some(Record {
            event,
            effect: None,
        })
    );
}

#[tokio::test]
async fn get_record_not_found_returns_none() {
    let rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(|| BatchGetItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo.get_record(community_id(), seq(1)).await.unwrap();
    assert_eq!(result, None);
}

// ── get_effect_for_event ──────────────────────────────────────────────────────

#[tokio::test]
async fn get_effect_for_event_found_returns_effect() {
    let effect = make_effect(seq(3), vec![]);
    let effect_item = encode_effect(&effect).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::get_item).then_output(move || {
        GetItemOutput::builder()
            .set_item(Some(effect_item.clone()))
            .build()
    });
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo
        .get_effect_for_event(community_id(), seq(3))
        .await
        .unwrap();
    assert_eq!(result, Some(effect));
}

#[tokio::test]
async fn get_effect_for_event_not_found_returns_none() {
    let rule =
        mock!(aws_sdk_dynamodb::Client::get_item).then_output(|| GetItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo
        .get_effect_for_event(community_id(), seq(3))
        .await
        .unwrap();
    assert_eq!(result, None);
}

// ── get_effects_after ─────────────────────────────────────────────────────────

#[tokio::test]
async fn get_effects_after_returns_effects() {
    let e1 = make_effect(seq(2), vec![]);
    let e2 = make_effect(seq(3), vec![StateMutation::BurnLuckBonus { delta: 2 }]);
    let item1 = encode_effect(&e1).unwrap();
    let item2 = encode_effect(&e2).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::query).then_output(move || {
        QueryOutput::builder()
            .items(item1.clone())
            .items(item2.clone())
            .build()
    });
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let effects = repo
        .get_effects_after(community_id(), 10, seq(1))
        .await
        .unwrap();
    assert_eq!(effects, vec![e1, e2]);
}

// ── get_records_before ────────────────────────────────────────────────────────

#[tokio::test]
async fn get_records_before_returns_records_with_effects() {
    let event = make_event(seq(4), EventPayload::Grant { count: 1 });
    let effect = make_effect(seq(4), vec![]);
    let event_item = encode_event(&event).unwrap();
    let effect_item = encode_effect(&effect).unwrap();

    // query returns event items; batch_get_item returns effect items
    let query_item = event_item.clone();
    let query_rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(query_item.clone()).build());
    let batch_rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(move || batch_get_output_for_table("test-table", vec![effect_item.clone()]));
    let client = mock_client!(aws_sdk_dynamodb, [&query_rule, &batch_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let records = repo
        .get_records_before(community_id(), 10, Some(seq(10)))
        .await
        .unwrap();
    assert_eq!(
        records,
        vec![Record {
            event,
            effect: Some(effect),
        }]
    );
}

#[tokio::test]
async fn get_records_before_none_bound_returns_records() {
    let event = make_event(seq(1), EventPayload::Grant { count: 1 });
    let event_item = encode_event(&event).unwrap();

    let query_item = event_item.clone();
    let query_rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(query_item.clone()).build());
    let batch_rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(|| BatchGetItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&query_rule, &batch_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let records = repo
        .get_records_before(community_id(), 10, None)
        .await
        .unwrap();
    assert_eq!(
        records,
        vec![Record {
            event,
            effect: None,
        }]
    );
}

// ── get_latest_grant_events ───────────────────────────────────────────────────

#[tokio::test]
async fn get_latest_grant_events_returns_events() {
    let e1 = make_event(seq(5), EventPayload::Grant { count: 4 });
    let item1 = encode_event(&e1).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(item1.clone()).build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let events = repo
        .get_latest_grant_events(community_id(), 5)
        .await
        .unwrap();
    assert_eq!(events, vec![e1]);
}

// ── get_latest_gift_records ───────────────────────────────────────────────────

#[tokio::test]
async fn get_latest_gift_records_returns_records() {
    let event = make_event(
        seq(6),
        EventPayload::Gift {
            sender_id: member_id(),
            recipient_id: member_id_2(),
            fruit: GRAPES,
            message: None,
        },
    );
    let effect = make_effect(seq(6), vec![]);
    let event_item = encode_event(&event).unwrap();
    let effect_item = encode_effect(&effect).unwrap();

    let qitem = event_item.clone();
    let query_rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(qitem.clone()).build());
    let batch_rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(move || batch_get_output_for_table("test-table", vec![effect_item.clone()]));
    let client = mock_client!(aws_sdk_dynamodb, [&query_rule, &batch_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let records = repo
        .get_latest_gift_records(community_id(), 5)
        .await
        .unwrap();
    assert_eq!(
        records,
        vec![Record {
            event,
            effect: Some(effect),
        }]
    );
}

// ── get_records_between ───────────────────────────────────────────────────────

#[tokio::test]
async fn get_records_between_single_page() {
    let event = make_event(seq(2), EventPayload::Grant { count: 1 });
    let event_item = encode_event(&event).unwrap();

    let qitem = event_item.clone();
    let query_rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(qitem.clone()).build());
    let batch_rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(|| BatchGetItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&query_rule, &batch_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let records = repo
        .get_records_between(community_id(), seq(1), seq(5))
        .await
        .unwrap();
    assert_eq!(
        records,
        vec![Record {
            event,
            effect: None,
        }]
    );
}

#[tokio::test]
async fn get_records_between_two_pages() {
    let event1 = make_event(seq(2), EventPayload::Grant { count: 1 });
    let event2 = make_event(seq(3), EventPayload::Grant { count: 2 });
    let item1 = encode_event(&event1).unwrap();
    let item2 = encode_event(&event2).unwrap();

    // First query returns item1 with a pagination token; second returns item2 without one.
    let pagination_key: HashMap<String, AttributeValue> =
        [("pk".to_string(), AttributeValue::S("page".to_string()))].into();
    let pk = pagination_key.clone();
    let i1 = item1.clone();
    let i2 = item2.clone();
    let query_rule = mock!(aws_sdk_dynamodb::Client::query)
        .sequence()
        .output(move || {
            QueryOutput::builder()
                .items(i1.clone())
                .set_last_evaluated_key(Some(pk.clone()))
                .build()
        })
        .output(move || QueryOutput::builder().items(i2.clone()).build())
        .build();
    let batch_rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(|| BatchGetItemOutput::builder().build());
    let client = mock_client!(
        aws_sdk_dynamodb,
        RuleMode::Sequential,
        [&query_rule, &batch_rule]
    );
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let records = repo
        .get_records_between(community_id(), seq(1), seq(5))
        .await
        .unwrap();
    assert_eq!(
        records,
        vec![
            Record {
                event: event1,
                effect: None,
            },
            Record {
                event: event2,
                effect: None,
            },
        ]
    );
}

// ── query_events_by_type pagination ──────────────────────────────────────────

#[tokio::test]
async fn get_latest_grant_events_paginates_until_limit_met() {
    let e1 = make_event(seq(1), EventPayload::Grant { count: 1 });
    let e2 = make_event(seq(2), EventPayload::Grant { count: 2 });
    let item1 = encode_event(&e1).unwrap();
    let item2 = encode_event(&e2).unwrap();

    let pagination_key: HashMap<String, AttributeValue> =
        [("pk".to_string(), AttributeValue::S("page2".to_string()))].into();
    let pk = pagination_key.clone();
    let i1 = item1.clone();
    let i2 = item2.clone();
    // First query returns e1 and a pagination token; second query returns e2 (no token).
    let query_rule = mock!(aws_sdk_dynamodb::Client::query)
        .sequence()
        .output(move || {
            QueryOutput::builder()
                .items(i1.clone())
                .set_last_evaluated_key(Some(pk.clone()))
                .build()
        })
        .output(move || QueryOutput::builder().items(i2.clone()).build())
        .build();
    let client = mock_client!(aws_sdk_dynamodb, RuleMode::Sequential, [&query_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let events = repo
        .get_latest_grant_events(community_id(), 2)
        .await
        .unwrap();
    assert_eq!(events, vec![e1, e2]);
}

// ── batch_get_effects: single-chunk path (≤ 100 seq IDs) ─────────────────────

#[tokio::test]
async fn get_records_before_batch_get_effects_single_chunk() {
    let events: Vec<Event> = (1u64..=5)
        .map(|i| make_event(seq(i), EventPayload::Grant { count: i as usize }))
        .collect();
    let effects: Vec<Effect> = (1u64..=5).map(|i| make_effect(seq(i), vec![])).collect();

    let event_items: Vec<HashMap<String, AttributeValue>> =
        events.iter().map(|e| encode_event(e).unwrap()).collect();
    let effect_items: Vec<HashMap<String, AttributeValue>> =
        effects.iter().map(|e| encode_effect(e).unwrap()).collect();

    let qitems = event_items.clone();
    let query_rule = mock!(aws_sdk_dynamodb::Client::query).then_output(move || {
        let mut builder = QueryOutput::builder();
        for item in &qitems {
            builder = builder.items(item.clone());
        }
        builder.build()
    });

    let eitems = effect_items.clone();
    let batch_rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(move || batch_get_output_for_table("test-table", eitems.clone()));
    let client = mock_client!(aws_sdk_dynamodb, [&query_rule, &batch_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let records = repo
        .get_records_before(community_id(), 10, None)
        .await
        .unwrap();

    let expected: Vec<Record> = events
        .into_iter()
        .zip(effects.into_iter())
        .map(|(event, effect)| Record {
            event,
            effect: Some(effect),
        })
        .collect();

    // Order may differ due to HashMap iteration; compare as sorted sets.
    let mut actual_ids: Vec<u64> = records.iter().map(|r| r.event.id.as_u64()).collect();
    let mut expected_ids: Vec<u64> = expected.iter().map(|r| r.event.id.as_u64()).collect();
    actual_ids.sort_unstable();
    expected_ids.sort_unstable();
    assert_eq!(actual_ids, expected_ids);
    assert_eq!(records.len(), expected.len());
}

// ── counter SDK error ─────────────────────────────────────────────────────────

#[tokio::test]
async fn append_event_counter_sdk_error_returns_sdk_error() {
    let counter_rule = mock!(aws_sdk_dynamodb::Client::update_item).then_error(|| {
        UpdateItemError::InternalServerError(InternalServerError::builder().build())
    });
    let client = mock_client!(aws_sdk_dynamodb, [&counter_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo
        .append_event(community_id(), EventPayload::Grant { count: 1 })
        .await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── get_record SDK errors ─────────────────────────────────────────────────────

#[tokio::test]
async fn get_record_sdk_error_returns_sdk_error() {
    let rule = mock!(aws_sdk_dynamodb::Client::batch_get_item).then_error(|| {
        BatchGetItemError::InternalServerError(InternalServerError::builder().build())
    });
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo.get_record(community_id(), seq(1)).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── get_effect_for_event SDK error ────────────────────────────────────────────

#[tokio::test]
async fn get_effect_for_event_sdk_error_returns_sdk_error() {
    let rule = mock!(aws_sdk_dynamodb::Client::get_item)
        .then_error(|| GetItemError::InternalServerError(InternalServerError::builder().build()));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo.get_effect_for_event(community_id(), seq(1)).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── get_effects_after SDK error ───────────────────────────────────────────────

#[tokio::test]
async fn get_effects_after_sdk_error_returns_sdk_error() {
    let rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_error(|| QueryError::InternalServerError(InternalServerError::builder().build()));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo.get_effects_after(community_id(), 10, seq(0)).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── get_records_before SDK error ──────────────────────────────────────────────

#[tokio::test]
async fn get_records_before_sdk_error_returns_sdk_error() {
    let rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_error(|| QueryError::InternalServerError(InternalServerError::builder().build()));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo.get_records_before(community_id(), 10, None).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── query_events_by_type SDK error ────────────────────────────────────────────

#[tokio::test]
async fn get_latest_grant_events_sdk_error_returns_sdk_error() {
    let rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_error(|| QueryError::InternalServerError(InternalServerError::builder().build()));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo.get_latest_grant_events(community_id(), 5).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── get_records_between SDK error ─────────────────────────────────────────────

#[tokio::test]
async fn get_records_between_sdk_error_returns_sdk_error() {
    let rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_error(|| QueryError::InternalServerError(InternalServerError::builder().build()));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let result = repo
        .get_records_between(community_id(), seq(1), seq(5))
        .await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── batch_get_effects: table name absent from response ────────────────────────

#[tokio::test]
async fn batch_get_effects_table_not_in_response_returns_records_without_effects() {
    let event = make_event(seq(1), EventPayload::Grant { count: 1 });
    let event_item = encode_event(&event).unwrap();

    let qitem = event_item.clone();
    let query_rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(qitem.clone()).build());

    // Batch response has a different table key — triggers the else branch in batch_get_effects.
    let mut alt_responses = HashMap::new();
    alt_responses.insert("other-table".to_string(), vec![]);
    let batch_rule = mock!(aws_sdk_dynamodb::Client::batch_get_item).then_output(move || {
        BatchGetItemOutput::builder()
            .set_responses(Some(alt_responses.clone()))
            .build()
    });

    let client = mock_client!(aws_sdk_dynamodb, [&query_rule, &batch_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let records = repo
        .get_records_before(community_id(), 10, None)
        .await
        .unwrap();
    assert_eq!(
        records,
        vec![Record {
            event,
            effect: None,
        }]
    );
}

// ── Reference forwarding impls ────────────────────────────────────────────────

#[tokio::test]
async fn get_record_via_ref() {
    let event = make_event(seq(1), EventPayload::Grant { count: 2 });
    let event_item = encode_event(&event).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(move || batch_get_output_for_table("test-table", vec![event_item.clone()]));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r).get_record(community_id(), seq(1)).await.unwrap();
    assert_eq!(
        result,
        Some(Record {
            event,
            effect: None
        })
    );
}

#[tokio::test]
async fn get_effect_for_event_via_ref() {
    let effect = make_effect(seq(2), vec![]);
    let effect_item = encode_effect(&effect).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::get_item).then_output(move || {
        GetItemOutput::builder()
            .set_item(Some(effect_item.clone()))
            .build()
    });
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r)
        .get_effect_for_event(community_id(), seq(2))
        .await
        .unwrap();
    assert_eq!(result, Some(effect));
}

#[tokio::test]
async fn get_effects_after_via_ref() {
    let effect = make_effect(seq(3), vec![]);
    let item = encode_effect(&effect).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(item.clone()).build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r)
        .get_effects_after(community_id(), 10, seq(0))
        .await
        .unwrap();
    assert_eq!(result, vec![effect]);
}

#[tokio::test]
async fn get_records_before_via_ref() {
    let event = make_event(seq(1), EventPayload::Grant { count: 1 });
    let event_item = encode_event(&event).unwrap();

    let qitem = event_item.clone();
    let query_rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(qitem.clone()).build());
    let batch_rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(|| BatchGetItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&query_rule, &batch_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r)
        .get_records_before(community_id(), 10, None)
        .await
        .unwrap();
    assert_eq!(
        result,
        vec![Record {
            event,
            effect: None,
        }]
    );
}

#[tokio::test]
async fn get_latest_grant_events_via_ref() {
    let event = make_event(seq(1), EventPayload::Grant { count: 1 });
    let item = encode_event(&event).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(item.clone()).build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r)
        .get_latest_grant_events(community_id(), 5)
        .await
        .unwrap();
    assert_eq!(result, vec![event]);
}

#[tokio::test]
async fn get_latest_gift_records_via_ref() {
    let event = make_event(
        seq(2),
        EventPayload::Gift {
            sender_id: member_id(),
            recipient_id: member_id_2(),
            fruit: GRAPES,
            message: None,
        },
    );
    let event_item = encode_event(&event).unwrap();

    let qitem = event_item.clone();
    let query_rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(qitem.clone()).build());
    let batch_rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(|| BatchGetItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&query_rule, &batch_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r)
        .get_latest_gift_records(community_id(), 5)
        .await
        .unwrap();
    assert_eq!(
        result,
        vec![Record {
            event,
            effect: None,
        }]
    );
}

#[tokio::test]
async fn get_records_between_via_ref() {
    let event = make_event(seq(2), EventPayload::Grant { count: 1 });
    let event_item = encode_event(&event).unwrap();

    let qitem = event_item.clone();
    let query_rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(qitem.clone()).build());
    let batch_rule = mock!(aws_sdk_dynamodb::Client::batch_get_item)
        .then_output(|| BatchGetItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&query_rule, &batch_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r)
        .get_records_between(community_id(), seq(1), seq(5))
        .await
        .unwrap();
    assert_eq!(
        result,
        vec![Record {
            event,
            effect: None,
        }]
    );
}

#[tokio::test]
async fn append_event_via_ref() {
    let payload = EventPayload::Grant { count: 1 };
    let counter_rule = mock!(aws_sdk_dynamodb::Client::update_item)
        .then_output(|| update_item_output_with_counter(1));
    let put_rule =
        mock!(aws_sdk_dynamodb::Client::put_item).then_output(|| PutItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&counter_rule, &put_rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r)
        .append_event(community_id(), payload.clone())
        .await
        .unwrap();
    assert_eq!(result.payload, payload);
}

#[tokio::test]
async fn append_effect_via_ref() {
    let mutations = vec![StateMutation::BurnLuckBonus { delta: 1 }];
    let rule =
        mock!(aws_sdk_dynamodb::Client::put_item).then_output(|| PutItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbEventLogRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r)
        .append_effect(seq(1), community_id(), mutations.clone())
        .await
        .unwrap();
    assert_eq!(result.mutations, mutations);
}
