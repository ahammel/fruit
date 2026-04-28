use aws_sdk_dynamodb::types::AttributeValue;
use exn::Exn;
use fruit_domain::{
    community::{Community, CommunityId},
    community_repo::{CommunityPersistor, CommunityProvider, CommunityRepo},
    event_log::SequenceId,
};
use newtype_ids_uuid::UuidIdentifier as _;

use crate::{
    dto::community::{decode_community, encode_community, sk_community, sk_community_range},
    error::{sdk_err, Entity, Error},
};

/// DynamoDB implementation of [`CommunityRepo`].
///
/// Stores community snapshots in the same table as events and effects.
/// `pk = community_id (UUID string)`, `sk = "COMMUNITY#{version_padded_20}"`.
///
/// The constructor accepts an already-built [`aws_sdk_dynamodb::Client`]. The
/// struct owns a [`tokio::runtime::Runtime`] used to drive async SDK calls from
/// the synchronous port methods; do not call port methods from within an
/// existing tokio runtime.
pub struct DynamoDbCommunityRepo {
    client: aws_sdk_dynamodb::Client,
    table_name: String,
    rt: tokio::runtime::Runtime,
}

impl DynamoDbCommunityRepo {
    /// Creates a new `DynamoDbCommunityRepo` backed by the given client and table.
    pub fn new(client: aws_sdk_dynamodb::Client, table_name: impl Into<String>) -> Self {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        Self { client, table_name: table_name.into(), rt }
    }

    // ── Async helpers ─────────────────────────────────────────────────────────

    async fn get_async(
        &self,
        id: CommunityId,
        version: SequenceId,
    ) -> Result<Option<Community>, Exn<Error>> {
        let pk = id.as_uuid().to_string();
        let sk = sk_community(version);

        let resp = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("pk", AttributeValue::S(pk))
            .key("sk", AttributeValue::S(sk))
            .send()
            .await
            .map_err(|e| Exn::new(sdk_err("get_item (community)", e)))?;

        resp.item.map(decode_community).transpose()
    }

    async fn get_latest_async(
        &self,
        id: CommunityId,
    ) -> Result<Option<Community>, Exn<Error>> {
        let pk = id.as_uuid().to_string();
        let (sk_lo, sk_hi) = sk_community_range();

        let resp = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("pk = :pk AND sk BETWEEN :lo AND :hi")
            .expression_attribute_values(":pk", AttributeValue::S(pk))
            .expression_attribute_values(":lo", AttributeValue::S(sk_lo))
            .expression_attribute_values(":hi", AttributeValue::S(sk_hi))
            .scan_index_forward(false)
            .limit(1)
            .send()
            .await
            .map_err(|e| Exn::new(sdk_err("query (latest community)", e)))?;

        resp.items
            .unwrap_or_default()
            .into_iter()
            .next()
            .map(decode_community)
            .transpose()
    }

    async fn put_async(&self, community: Community) -> Result<Community, Exn<Error>> {
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
                let se = e.into_service_error();
                if se.is_conditional_check_failed_exception() {
                    Err(Exn::new(Error::AlreadyExists {
                        community: community.id,
                        version: community.version,
                        entity: Entity::Community,
                    }))
                } else {
                    Err(Exn::new(sdk_err("put_item (community)", se)))
                }
            }
        }
    }
}

// ── CommunityProvider ─────────────────────────────────────────────────────────

impl CommunityProvider for DynamoDbCommunityRepo {
    type Error = Error;

    fn get(
        &self,
        id: CommunityId,
        version: SequenceId,
    ) -> Result<Option<Community>, Exn<Self::Error>> {
        self.rt.block_on(self.get_async(id, version))
    }

    fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Exn<Self::Error>> {
        self.rt.block_on(self.get_latest_async(id))
    }
}

impl CommunityProvider for &DynamoDbCommunityRepo {
    type Error = Error;

    fn get(
        &self,
        id: CommunityId,
        version: SequenceId,
    ) -> Result<Option<Community>, Exn<Self::Error>> {
        self.rt.block_on(self.get_async(id, version))
    }

    fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Exn<Self::Error>> {
        self.rt.block_on(self.get_latest_async(id))
    }
}

// ── CommunityPersistor ────────────────────────────────────────────────────────

impl CommunityPersistor for DynamoDbCommunityRepo {
    type Error = Error;

    fn put(&self, community: Community) -> Result<Community, Exn<Self::Error>> {
        self.rt.block_on(self.put_async(community))
    }
}

impl CommunityPersistor for &DynamoDbCommunityRepo {
    type Error = Error;

    fn put(&self, community: Community) -> Result<Community, Exn<Self::Error>> {
        self.rt.block_on(self.put_async(community))
    }
}

// ── CommunityRepo ─────────────────────────────────────────────────────────────

impl CommunityRepo for DynamoDbCommunityRepo {}
impl CommunityRepo for &DynamoDbCommunityRepo {}
