use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use exn::Exn;
use fruit_domain::{
    community::{Community, CommunityId},
    event_log::SequenceId,
    member::Member,
};
use newtype_ids::IntegerIdentifier as _;
use newtype_ids_uuid::UuidIdentifier as _;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::event::MemberDto;
use crate::error::{codec_err, Error};

// ── CommunityDto ──────────────────────────────────────────────────────────────

/// The DynamoDB item structure for a community snapshot.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CommunityItem {
    pub pk: String,
    pub sk: String,
    pub luck: u8,
    pub version: u64,
    /// Members keyed by UUID string so DynamoDB stores them as a native Map.
    pub members: HashMap<String, MemberDto>,
}

pub(crate) fn sk_community(version: SequenceId) -> String {
    format!("COMMUNITY#{:020}", version.as_u64())
}

pub(crate) fn sk_community_range() -> (String, String) {
    (
        format!("COMMUNITY#{:020}", 0u64),
        format!("COMMUNITY#{:020}", u64::MAX),
    )
}

/// Encodes a [`Community`] into a DynamoDB attribute map.
pub(crate) fn encode_community(
    community: &Community,
) -> Result<HashMap<String, AttributeValue>, Exn<Error>> {
    let item = CommunityItem {
        pk: community.id.as_uuid().to_string(),
        sk: sk_community(community.version),
        luck: community.luck_raw(),
        version: community.version.as_u64(),
        members: community
            .members
            .values()
            .map(|m| (m.id.as_uuid().to_string(), MemberDto::from(m)))
            .collect(),
    };
    serde_dynamo::aws_sdk_dynamodb_1::to_item(&item)
        .map_err(|e| Exn::new(codec_err("failed to encode community", e)))
}

/// Decodes a DynamoDB attribute map into a [`Community`].
pub(crate) fn decode_community(
    item: HashMap<String, AttributeValue>,
) -> Result<Community, Exn<Error>> {
    let dto: CommunityItem = serde_dynamo::aws_sdk_dynamodb_1::from_item(item)
        .map_err(|e| Exn::new(codec_err("failed to decode community", e)))?;

    let community_id = Uuid::parse_str(&dto.pk)
        .map(CommunityId::from)
        .map_err(|e| Exn::new(codec_err("invalid community pk", e)))?;

    let members: HashMap<_, Member> = dto
        .members
        .into_values()
        .map(|m| Member::try_from(m).map(|member| (member.id, member)))
        .collect::<Result<_, _>>()
        .map_err(Exn::new)?;

    let mut community = Community::new()
        .with_id(community_id)
        .with_luck(dto.luck)
        .with_version(SequenceId::new(dto.version));
    community.members = members;

    Ok(community)
}
