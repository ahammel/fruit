use async_trait::async_trait;
use aws_sdk_dynamodb::{primitives::Blob, types::AttributeValue};
use exn::Exn;
use fruit_domain::{
    community::{Community, CommunityId},
    community_repo::{CommunityPersistor, CommunityProvider, CommunityRepo},
    event_log::SequenceId,
};
use newtype_ids_uuid::UuidIdentifier as _;

use crate::{
    dto::community::{decode_community, encode_community, sk_community, sk_community_range},
    error::{raise_sdk_err, Entity, Error},
};

/// DynamoDB implementation of [`CommunityRepo`].
///
/// Stores community snapshots in the same table as events and effects.
/// `pk = community_id (UUID bytes, 16-byte binary)`, `sk = "COMMUNITY#{version_padded_20}"`.
///
/// The constructor accepts an already-built [`aws_sdk_dynamodb::Client`].
pub struct DynamoDbCommunityRepo {
    client: aws_sdk_dynamodb::Client,
    table_name: String,
}

impl DynamoDbCommunityRepo {
    /// Creates a new `DynamoDbCommunityRepo` backed by the given client and table.
    pub fn new(client: aws_sdk_dynamodb::Client, table_name: impl Into<String>) -> Self {
        Self {
            client,
            table_name: table_name.into(),
        }
    }
}

// ── CommunityProvider ─────────────────────────────────────────────────────────

#[async_trait]
impl CommunityProvider for DynamoDbCommunityRepo {
    type Error = Error;

    async fn get(
        &self,
        id: CommunityId,
        version: SequenceId,
    ) -> Result<Option<Community>, Exn<Self::Error>> {
        let pk = Blob::new(id.as_uuid().as_bytes().to_vec());
        let sk = sk_community(version);

        let resp = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("pk", AttributeValue::B(pk))
            .key("sk", AttributeValue::S(sk))
            .send()
            .await
            .map_err(|e| raise_sdk_err("get_item (community)", e))?;

        resp.item.map(decode_community).transpose()
    }

    async fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Exn<Self::Error>> {
        let pk = Blob::new(id.as_uuid().as_bytes().to_vec());
        let (sk_lo, sk_hi) = sk_community_range();

        let resp = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("pk = :pk AND sk BETWEEN :lo AND :hi")
            .expression_attribute_values(":pk", AttributeValue::B(pk))
            .expression_attribute_values(":lo", AttributeValue::S(sk_lo))
            .expression_attribute_values(":hi", AttributeValue::S(sk_hi))
            .scan_index_forward(false)
            .limit(1)
            .send()
            .await
            .map_err(|e| raise_sdk_err("query (latest community)", e))?;

        resp.items
            .unwrap_or_default()
            .into_iter()
            .next()
            .map(decode_community)
            .transpose()
    }
}

#[async_trait]
impl CommunityProvider for &DynamoDbCommunityRepo {
    type Error = Error;

    async fn get(
        &self,
        id: CommunityId,
        version: SequenceId,
    ) -> Result<Option<Community>, Exn<Self::Error>> {
        (*self).get(id, version).await
    }

    async fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Exn<Self::Error>> {
        (*self).get_latest(id).await
    }
}

// ── CommunityPersistor ────────────────────────────────────────────────────────

#[async_trait]
impl CommunityPersistor for DynamoDbCommunityRepo {
    type Error = Error;

    async fn put(&self, community: Community) -> Result<Community, Exn<Self::Error>> {
        let item = encode_community(&community)?;

        let result = self
            .client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(sk)")
            .send()
            .await;

        match result {
            Ok(_) => Ok(community),
            Err(e) => {
                let is_conflict = matches!(
                    &e,
                    aws_sdk_dynamodb::error::SdkError::ServiceError(se)
                        if se.err().is_conditional_check_failed_exception()
                );
                if is_conflict {
                    Err(Exn::new(Error::AlreadyExists {
                        community: community.id,
                        version: community.version,
                        entity: Entity::Community,
                    }))
                } else {
                    Err(raise_sdk_err("put_item (community)", e))
                }
            }
        }
    }
}

#[async_trait]
impl CommunityPersistor for &DynamoDbCommunityRepo {
    type Error = Error;

    async fn put(&self, community: Community) -> Result<Community, Exn<Self::Error>> {
        (*self).put(community).await
    }
}

// ── CommunityRepo ─────────────────────────────────────────────────────────────

impl CommunityRepo for DynamoDbCommunityRepo {}
impl CommunityRepo for &DynamoDbCommunityRepo {}
