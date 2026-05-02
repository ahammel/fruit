use fruit_domain::{community::Community, event_log::SequenceId};

use super::{decode_community, encode_community, sk_community, sk_community_range};

mod test_helpers {
    use fruit_domain::{
        bag::Bag,
        community::{Community, CommunityId},
        event_log::SequenceId,
        fruit::{GRAPES, MELON},
        member::{Member, MemberId},
    };
    use uuid::Uuid;

    /// A fixed community ID for test reproducibility.
    pub fn community_id() -> CommunityId {
        CommunityId::from(Uuid::from_bytes([1u8; 16]))
    }

    /// A fixed member ID for test reproducibility.
    pub fn member_id() -> MemberId {
        MemberId::from(Uuid::from_bytes([2u8; 16]))
    }

    /// A fixed secondary member ID.
    pub fn member_id_2() -> MemberId {
        MemberId::from(Uuid::from_bytes([3u8; 16]))
    }

    /// Wraps `n` as a `SequenceId`.
    pub fn seq(n: u64) -> SequenceId {
        SequenceId::new(n)
    }

    /// Builds a test community with two members who hold some fruit.
    pub fn make_community() -> Community {
        let bag1 = Bag::new().insert(GRAPES).insert(GRAPES).insert(MELON);
        let bag2 = Bag::new().insert(MELON);
        let m1 = Member::new("Alice").with_id(member_id()).with_bag(bag1);
        let m2 = Member::new("Bob").with_id(member_id_2()).with_bag(bag2);
        let mut c = Community::new()
            .with_id(community_id())
            .with_luck(42)
            .with_version(seq(7));
        c.add_member(m1);
        c.add_member(m2);
        c
    }
}

use test_helpers::*;

// ── encode/decode round-trip ──────────────────────────────────────────────────

#[test]
fn community_round_trips_with_members_and_bags() {
    let community = make_community();
    let encoded = encode_community(&community).unwrap();
    let decoded = decode_community(encoded).unwrap();
    assert_eq!(decoded, community);
}

#[test]
fn community_round_trips_empty_community() {
    let community = Community::new()
        .with_id(community_id())
        .with_luck(0)
        .with_version(seq(0));
    let encoded = encode_community(&community).unwrap();
    let decoded = decode_community(encoded).unwrap();
    assert_eq!(decoded, community);
}

// ── sk_community ──────────────────────────────────────────────────────────────

#[test]
fn sk_community_formats_zero_padded_20_digits() {
    let sk = sk_community(seq(1));
    assert_eq!(sk, "COMMUNITY#00000000000000000001");
}

#[test]
fn sk_community_formats_large_value() {
    let sk = sk_community(SequenceId::new(u64::MAX));
    assert_eq!(sk, format!("COMMUNITY#{:020}", u64::MAX));
}

// ── sk_community_range ────────────────────────────────────────────────────────

#[test]
fn sk_community_range_lower_starts_at_zero() {
    let (lower, _) = sk_community_range();
    assert_eq!(lower, "COMMUNITY#00000000000000000000");
}

#[test]
fn sk_community_range_upper_is_u64_max() {
    let (_, upper) = sk_community_range();
    assert_eq!(upper, format!("COMMUNITY#{:020}", u64::MAX));
}

// ── decode_community error paths ──────────────────────────────────────────────

#[test]
fn decode_community_bad_pk_returns_codec_error() {
    use crate::dto::community::CommunityItem;
    use serde_bytes::ByteBuf;
    use std::collections::HashMap;

    // pk with wrong byte count (not 16)
    let item = CommunityItem {
        pk: ByteBuf::from(vec![0u8; 5]),
        sk: sk_community(seq(1)),
        luck: 0,
        version: 1,
        members: HashMap::new(),
    };
    let attr_map =
        serde_dynamo::aws_sdk_dynamodb_1::to_item(&item).expect("to_item should not fail");
    let result = decode_community(attr_map);
    assert!(result.is_err());
}

#[test]
fn decode_community_invalid_dynamo_item_returns_codec_error() {
    let result = decode_community(std::collections::HashMap::new());
    assert!(result.is_err());
}
