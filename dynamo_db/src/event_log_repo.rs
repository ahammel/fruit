use std::collections::HashMap;

use aws_sdk_dynamodb::types::{AttributeValue, KeysAndAttributes, ReturnValue};
use exn::Exn;
use fruit_domain::{
    community::CommunityId,
    event_log::{Effect, Event, EventPayload, Record, SequenceId, StateMutation},
    event_log_repo::{EventLogPersistor, EventLogProvider, EventLogRepo},
};
use newtype_ids::IntegerIdentifier as _;
use newtype_ids_uuid::UuidIdentifier as _;

use crate::{
    dto::event::{
        build_records, decode_effect, decode_event, encode_effect, encode_event, sk_effect,
        sk_effect_range_after, sk_event_range, EVENT_TYPE_GIFT, EVENT_TYPE_GRANT,
    },
    error::{sdk_err, Entity, Error},
};

/// DynamoDB implementation of [`EventLogRepo`].
///
/// Uses a single-table design with `pk = community_id` and
/// `sk = {ENTITY_TYPE}#{seq_id_zero_padded_20}`. Events, effects, and a global
/// sequence counter all live in the same table.
///
/// A GSI named `seq-index` (PK: `seq` N) supports the two operations that look
/// up by sequence ID without a community ID ([`get_record`] and
/// [`get_effect_for_event`]).
///
/// The constructor accepts an already-built [`aws_sdk_dynamodb::Client`]. The
/// struct owns a [`tokio::runtime::Runtime`] used to drive async SDK calls from
/// the synchronous port methods; do not call port methods from within an
/// existing tokio runtime.
pub struct DynamoDbEventLogRepo {
    client: aws_sdk_dynamodb::Client,
    table_name: String,
    rt: tokio::runtime::Runtime,
}

impl DynamoDbEventLogRepo {
    /// Creates a new `DynamoDbEventLogRepo` backed by the given client and table.
    pub fn new(client: aws_sdk_dynamodb::Client, table_name: impl Into<String>) -> Self {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        Self {
            client,
            table_name: table_name.into(),
            rt,
        }
    }

    // ── Sequence counter ──────────────────────────────────────────────────────

    async fn next_seq_id(&self) -> Result<SequenceId, Exn<Error>> {
        let resp = self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("pk", AttributeValue::S("__COUNTER__".to_string()))
            .key("sk", AttributeValue::S("SEQUENCE".to_string()))
            .update_expression("ADD #n :inc")
            .expression_attribute_names("#n", "n")
            .expression_attribute_values(":inc", AttributeValue::N("1".to_string()))
            .return_values(ReturnValue::UpdatedNew)
            .send()
            .await
            .map_err(|e| Exn::new(sdk_err("failed to increment sequence counter", e)))?;

        let n = resp
            .attributes()
            .and_then(|a| a.get("n"))
            .and_then(|v| v.as_n().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| {
                Exn::new(Error::Codec {
                    message: "sequence counter response missing numeric 'n'".to_string(),
                })
            })?;

        Ok(SequenceId::new(n))
    }

    // ── Batch effect fetch ────────────────────────────────────────────────────

    async fn batch_get_effects(
        &self,
        community_id: CommunityId,
        seq_ids: &[SequenceId],
    ) -> Result<HashMap<SequenceId, Effect>, Exn<Error>> {
        let pk_val = community_id.as_uuid().to_string();
        let mut result: HashMap<SequenceId, Effect> = HashMap::new();

        for chunk in seq_ids.chunks(100) {
            let keys: Vec<HashMap<String, AttributeValue>> = chunk
                .iter()
                .map(|&seq| {
                    HashMap::from([
                        ("pk".to_string(), AttributeValue::S(pk_val.clone())),
                        ("sk".to_string(), AttributeValue::S(sk_effect(seq))),
                    ])
                })
                .collect();

            let kaa = KeysAndAttributes::builder()
                .set_keys(Some(keys))
                .build()
                .map_err(|e| Exn::new(sdk_err("failed to build batch-get keys", e)))?;

            let resp = self
                .client
                .batch_get_item()
                .request_items(&self.table_name, kaa)
                .send()
                .await
                .map_err(|e| Exn::new(sdk_err("failed to batch-get effects", e)))?;

            if let Some(responses) = resp.responses {
                if let Some(items) = responses.get(&self.table_name) {
                    for item in items {
                        let effect = decode_effect(item.clone())?;
                        result.insert(effect.id, effect);
                    }
                }
            }
        }

        Ok(result)
    }

    // ── Core async implementations ────────────────────────────────────────────

    async fn get_record_async(&self, id: SequenceId) -> Result<Option<Record>, Exn<Error>> {
        let items = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("seq-index")
            .key_condition_expression("#seq = :seq")
            .expression_attribute_names("#seq", "seq")
            .expression_attribute_values(":seq", AttributeValue::N(id.as_u64().to_string()))
            .send()
            .await
            .map_err(|e| Exn::new(sdk_err("failed to query by seq", e)))?
            .items
            .unwrap_or_default();

        let mut event_item = None;
        let mut effect_item = None;
        for item in items {
            match item.get("sk").and_then(|v| v.as_s().ok()) {
                Some(sk) if sk.starts_with("EVENT#") => event_item = Some(item),
                Some(sk) if sk.starts_with("EFFECT#") => effect_item = Some(item),
                _ => {}
            }
        }

        let Some(ev_item) = event_item else {
            return Ok(None);
        };
        let event = decode_event(ev_item)?;
        let effect = effect_item.map(decode_effect).transpose()?;
        Ok(Some(Record { event, effect }))
    }

    async fn get_effect_for_event_async(
        &self,
        event_id: SequenceId,
    ) -> Result<Option<Effect>, Exn<Error>> {
        let items = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("seq-index")
            .key_condition_expression("#seq = :seq")
            .expression_attribute_names("#seq", "seq")
            .expression_attribute_values(":seq", AttributeValue::N(event_id.as_u64().to_string()))
            .filter_expression("begins_with(sk, :prefix)")
            .expression_attribute_values(":prefix", AttributeValue::S("EFFECT#".to_string()))
            .send()
            .await
            .map_err(|e| Exn::new(sdk_err("failed to query effect by seq", e)))?
            .items
            .unwrap_or_default();

        items.into_iter().next().map(decode_effect).transpose()
    }

    async fn get_effects_after_async(
        &self,
        community_id: CommunityId,
        limit: usize,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        let (lower, upper) = sk_effect_range_after(after);
        let items = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("pk = :pk AND sk BETWEEN :lower AND :upper")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S(community_id.as_uuid().to_string()),
            )
            .expression_attribute_values(":lower", AttributeValue::S(lower))
            .expression_attribute_values(":upper", AttributeValue::S(upper))
            .scan_index_forward(true)
            .limit(limit as i32)
            .send()
            .await
            .map_err(|e| Exn::new(sdk_err("failed to query effects", e)))?
            .items
            .unwrap_or_default();

        items.into_iter().map(decode_effect).collect()
    }

    async fn get_records_before_async(
        &self,
        community_id: CommunityId,
        limit: usize,
        before: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        // Use zero as the "start from beginning" sentinel; SequenceId::zero() is never
        // a real event, so passing it as `after` to sk_event_range covers all events.
        let (lower, upper) = sk_event_range(SequenceId::zero(), before);
        let event_items = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("pk = :pk AND sk BETWEEN :lower AND :upper")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S(community_id.as_uuid().to_string()),
            )
            .expression_attribute_values(":lower", AttributeValue::S(lower))
            .expression_attribute_values(":upper", AttributeValue::S(upper))
            .scan_index_forward(false)
            .limit(limit as i32)
            .send()
            .await
            .map_err(|e| Exn::new(sdk_err("failed to query records", e)))?
            .items
            .unwrap_or_default();

        let seq_ids: Vec<SequenceId> = event_items
            .iter()
            .filter_map(|item| {
                item.get("seq")
                    .and_then(|v| v.as_n().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(SequenceId::new)
            })
            .collect();

        let effect_map = self.batch_get_effects(community_id, &seq_ids).await?;
        build_records(event_items, &effect_map)
    }

    /// Queries EVENT items of a specific type for a community, paging until
    /// `limit` matches are found or all events are exhausted.
    async fn query_events_by_type(
        &self,
        community_id: CommunityId,
        event_type: &str,
        limit: usize,
    ) -> Result<Vec<HashMap<String, AttributeValue>>, Exn<Error>> {
        let pk = community_id.as_uuid().to_string();
        let (lower, upper) = sk_event_range(SequenceId::new(0), None);
        let mut results = Vec::new();
        let mut last_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut req = self
                .client
                .query()
                .table_name(&self.table_name)
                .key_condition_expression("pk = :pk AND sk BETWEEN :lower AND :upper")
                .filter_expression("event_type = :et")
                .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                .expression_attribute_values(":lower", AttributeValue::S(lower.clone()))
                .expression_attribute_values(":upper", AttributeValue::S(upper.clone()))
                .expression_attribute_values(":et", AttributeValue::S(event_type.to_string()))
                .scan_index_forward(false);

            if let Some(key) = last_key.take() {
                for (k, v) in key {
                    req = req.exclusive_start_key(k, v);
                }
            }

            let resp = req
                .send()
                .await
                .map_err(|e| Exn::new(sdk_err("failed to query events by type", e)))?;

            let items = resp.items.unwrap_or_default();
            let remaining = limit - results.len();
            results.extend(items.into_iter().take(remaining));

            if results.len() >= limit || resp.last_evaluated_key.is_none() {
                break;
            }
            last_key = resp.last_evaluated_key;
        }

        Ok(results)
    }

    async fn get_latest_grant_events_async(
        &self,
        community_id: CommunityId,
        limit: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        let items = self
            .query_events_by_type(community_id, EVENT_TYPE_GRANT, limit)
            .await?;
        items.into_iter().map(decode_event).collect()
    }

    async fn get_latest_gift_records_async(
        &self,
        community_id: CommunityId,
        limit: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        let event_items = self
            .query_events_by_type(community_id, EVENT_TYPE_GIFT, limit)
            .await?;

        let seq_ids: Vec<SequenceId> = event_items
            .iter()
            .filter_map(|item| {
                item.get("seq")
                    .and_then(|v| v.as_n().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(SequenceId::new)
            })
            .collect();

        let effect_map = self.batch_get_effects(community_id, &seq_ids).await?;
        build_records(event_items, &effect_map)
    }

    async fn get_records_between_async(
        &self,
        community_id: CommunityId,
        after: SequenceId,
        before: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        let (lower, upper) = sk_event_range(after, Some(before));
        let pk = community_id.as_uuid().to_string();
        let mut event_items = Vec::new();
        let mut last_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut req = self
                .client
                .query()
                .table_name(&self.table_name)
                .key_condition_expression("pk = :pk AND sk BETWEEN :lower AND :upper")
                .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                .expression_attribute_values(":lower", AttributeValue::S(lower.clone()))
                .expression_attribute_values(":upper", AttributeValue::S(upper.clone()))
                .scan_index_forward(true);

            if let Some(key) = last_key.take() {
                for (k, v) in key {
                    req = req.exclusive_start_key(k, v);
                }
            }

            let resp = req
                .send()
                .await
                .map_err(|e| Exn::new(sdk_err("failed to query records between", e)))?;

            event_items.extend(resp.items.unwrap_or_default());

            if resp.last_evaluated_key.is_none() {
                break;
            }
            last_key = resp.last_evaluated_key;
        }

        let seq_ids: Vec<SequenceId> = event_items
            .iter()
            .filter_map(|item| {
                item.get("seq")
                    .and_then(|v| v.as_n().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(SequenceId::new)
            })
            .collect();

        let effect_map = self.batch_get_effects(community_id, &seq_ids).await?;
        build_records(event_items, &effect_map)
    }

    async fn append_event_async(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        let id = self.next_seq_id().await?;
        let event = Event {
            id,
            community_id,
            payload,
        };
        let item = encode_event(&event)?;

        match self
            .client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(sk)")
            .send()
            .await
        {
            Ok(_) => Ok(event),
            Err(e) => {
                let is_conflict = matches!(
                    &e,
                    aws_sdk_dynamodb::error::SdkError::ServiceError(se)
                        if se.err().is_conditional_check_failed_exception()
                );
                if is_conflict {
                    Err(Exn::new(Error::AlreadyExists {
                        community: community_id,
                        version: id,
                        entity: Entity::Event,
                    }))
                } else {
                    Err(Exn::new(sdk_err("failed to write event", e)))
                }
            }
        }
    }

    async fn append_effect_async(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        let effect = Effect {
            id: event_id,
            community_id,
            mutations,
        };
        let item = encode_effect(&effect)?;

        match self
            .client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(sk)")
            .send()
            .await
        {
            Ok(_) => Ok(effect),
            Err(e) => {
                let is_conflict = matches!(
                    &e,
                    aws_sdk_dynamodb::error::SdkError::ServiceError(se)
                        if se.err().is_conditional_check_failed_exception()
                );
                if is_conflict {
                    Err(Exn::new(Error::AlreadyExists {
                        community: community_id,
                        version: event_id,
                        entity: Entity::Effect,
                    }))
                } else {
                    Err(Exn::new(sdk_err("failed to write effect", e)))
                }
            }
        }
    }
}

// ── Trait implementations ─────────────────────────────────────────────────────

impl EventLogProvider for DynamoDbEventLogRepo {
    type Error = Error;

    fn get_record(&self, id: SequenceId) -> Result<Option<Record>, Exn<Error>> {
        self.rt.block_on(self.get_record_async(id))
    }

    fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Exn<Error>> {
        self.rt.block_on(self.get_effect_for_event_async(event_id))
    }

    fn get_effects_after(
        &self,
        community_id: CommunityId,
        limit: usize,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        self.rt
            .block_on(self.get_effects_after_async(community_id, limit, after))
    }

    fn get_records_before(
        &self,
        community_id: CommunityId,
        limit: usize,
        before: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        self.rt
            .block_on(self.get_records_before_async(community_id, limit, before))
    }

    fn get_latest_grant_events(
        &self,
        community_id: CommunityId,
        limit: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        self.rt
            .block_on(self.get_latest_grant_events_async(community_id, limit))
    }

    fn get_latest_gift_records(
        &self,
        community_id: CommunityId,
        limit: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        self.rt
            .block_on(self.get_latest_gift_records_async(community_id, limit))
    }

    fn get_records_between(
        &self,
        community_id: CommunityId,
        after: SequenceId,
        before: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        self.rt
            .block_on(self.get_records_between_async(community_id, after, before))
    }
}

impl EventLogPersistor for DynamoDbEventLogRepo {
    type Error = Error;

    fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        self.rt
            .block_on(self.append_event_async(community_id, payload))
    }

    fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        self.rt
            .block_on(self.append_effect_async(event_id, community_id, mutations))
    }
}

impl EventLogRepo for DynamoDbEventLogRepo {}

impl EventLogProvider for &DynamoDbEventLogRepo {
    type Error = Error;

    fn get_record(&self, id: SequenceId) -> Result<Option<Record>, Exn<Error>> {
        (*self).get_record(id)
    }

    fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Exn<Error>> {
        (*self).get_effect_for_event(event_id)
    }

    fn get_effects_after(
        &self,
        community_id: CommunityId,
        limit: usize,
        after: SequenceId,
    ) -> Result<Vec<Effect>, Exn<Error>> {
        (*self).get_effects_after(community_id, limit, after)
    }

    fn get_records_before(
        &self,
        community_id: CommunityId,
        limit: usize,
        before: Option<SequenceId>,
    ) -> Result<Vec<Record>, Exn<Error>> {
        (*self).get_records_before(community_id, limit, before)
    }

    fn get_latest_grant_events(
        &self,
        community_id: CommunityId,
        limit: usize,
    ) -> Result<Vec<Event>, Exn<Error>> {
        (*self).get_latest_grant_events(community_id, limit)
    }

    fn get_latest_gift_records(
        &self,
        community_id: CommunityId,
        limit: usize,
    ) -> Result<Vec<Record>, Exn<Error>> {
        (*self).get_latest_gift_records(community_id, limit)
    }

    fn get_records_between(
        &self,
        community_id: CommunityId,
        after: SequenceId,
        before: SequenceId,
    ) -> Result<Vec<Record>, Exn<Error>> {
        (*self).get_records_between(community_id, after, before)
    }
}

impl EventLogPersistor for &DynamoDbEventLogRepo {
    type Error = Error;

    fn append_event(
        &self,
        community_id: CommunityId,
        payload: EventPayload,
    ) -> Result<Event, Exn<Error>> {
        (*self).append_event(community_id, payload)
    }

    fn append_effect(
        &self,
        event_id: SequenceId,
        community_id: CommunityId,
        mutations: Vec<StateMutation>,
    ) -> Result<Effect, Exn<Error>> {
        (*self).append_effect(event_id, community_id, mutations)
    }
}

impl EventLogRepo for &DynamoDbEventLogRepo {}
