use std::collections::HashMap;

use fruit_domain::{
    bag::Bag,
    event_log::{Effect, EventPayload, Record, SequenceId, StateMutation},
    fruit::{FRUITS, GRAPES, MELON},
    member::Member,
};
use uuid::Uuid;

use super::*;
use crate::dto::event::{
    build_records, decode_effect, decode_event, encode_effect, encode_event, event_type_name,
    sk_effect, sk_effect_range_after, sk_event, sk_event_range, EVENT_TYPE_ADD_MEMBER,
    EVENT_TYPE_BURN, EVENT_TYPE_GIFT, EVENT_TYPE_GRANT, EVENT_TYPE_REMOVE_MEMBER,
    EVENT_TYPE_SET_COMMUNITY_LUCK, EVENT_TYPE_SET_MEMBER_LUCK,
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
        CommunityId::from(Uuid::from_bytes([1u8; 16]))
    }

    /// A fixed member ID for test reproducibility.
    pub fn member_id() -> MemberId {
        MemberId::from(Uuid::from_bytes([2u8; 16]))
    }

    /// A fixed secondary member ID for tests needing two distinct members.
    pub fn member_id_2() -> MemberId {
        MemberId::from(Uuid::from_bytes([3u8; 16]))
    }

    /// Wraps `n` as a `SequenceId`.
    pub fn seq(n: u64) -> SequenceId {
        SequenceId::new(n)
    }

    /// Constructs an `Event` with the given id and payload.
    pub fn make_event(id: SequenceId, payload: EventPayload) -> Event {
        Event {
            id,
            community_id: community_id(),
            payload,
        }
    }

    /// Constructs an `Effect` with the given id and mutations.
    pub fn make_effect(id: SequenceId, mutations: Vec<StateMutation>) -> Effect {
        Effect {
            id,
            community_id: community_id(),
            mutations,
        }
    }
}

use test_helpers::*;

// ── EventPayload round-trips ──────────────────────────────────────────────────

#[test]
fn event_payload_grant_round_trips() {
    let event = make_event(seq(1), EventPayload::Grant { count: 5 });
    let encoded = encode_event(&event).unwrap();
    let decoded = decode_event(encoded).unwrap();
    assert_eq!(decoded, event);
}

#[test]
fn event_payload_add_member_round_trips() {
    let event = make_event(
        seq(2),
        EventPayload::AddMember {
            display_name: "Alice".to_string(),
            member_id: member_id(),
            external_id: None,
        },
    );
    let encoded = encode_event(&event).unwrap();
    let decoded = decode_event(encoded).unwrap();
    assert_eq!(decoded, event);
}

#[test]
fn event_payload_remove_member_round_trips() {
    let event = make_event(
        seq(3),
        EventPayload::RemoveMember {
            member_id: member_id(),
        },
    );
    let encoded = encode_event(&event).unwrap();
    let decoded = decode_event(encoded).unwrap();
    assert_eq!(decoded, event);
}

#[test]
fn event_payload_set_community_luck_round_trips() {
    let event = make_event(seq(4), EventPayload::SetCommunityLuck { luck: 128 });
    let encoded = encode_event(&event).unwrap();
    let decoded = decode_event(encoded).unwrap();
    assert_eq!(decoded, event);
}

#[test]
fn event_payload_set_member_luck_round_trips() {
    let event = make_event(
        seq(5),
        EventPayload::SetMemberLuck {
            member_id: member_id(),
            luck: 64,
        },
    );
    let encoded = encode_event(&event).unwrap();
    let decoded = decode_event(encoded).unwrap();
    assert_eq!(decoded, event);
}

#[test]
fn event_payload_gift_with_message_round_trips() {
    let event = make_event(
        seq(6),
        EventPayload::Gift {
            sender_id: member_id(),
            recipient_id: member_id_2(),
            fruit: FRUITS[0],
            message: Some("hello".to_string()),
        },
    );
    let encoded = encode_event(&event).unwrap();
    let decoded = decode_event(encoded).unwrap();
    assert_eq!(decoded, event);
}

#[test]
fn event_payload_gift_without_message_round_trips() {
    let event = make_event(
        seq(7),
        EventPayload::Gift {
            sender_id: member_id(),
            recipient_id: member_id_2(),
            fruit: FRUITS[0],
            message: None,
        },
    );
    let encoded = encode_event(&event).unwrap();
    let decoded = decode_event(encoded).unwrap();
    assert_eq!(decoded, event);
}

#[test]
fn event_payload_burn_round_trips() {
    let event = make_event(
        seq(8),
        EventPayload::Burn {
            member_id: member_id(),
            fruits: vec![FRUITS[0], FRUITS[1]],
            message: None,
        },
    );
    let encoded = encode_event(&event).unwrap();
    let decoded = decode_event(encoded).unwrap();
    assert_eq!(decoded, event);
}

// ── StateMutation round-trips ─────────────────────────────────────────────────

fn round_trip_effect(mutations: Vec<StateMutation>) {
    let effect = make_effect(seq(1), mutations);
    let encoded = encode_effect(&effect).unwrap();
    let decoded = decode_effect(encoded).unwrap();
    assert_eq!(decoded, effect);
}

#[test]
fn mutation_add_fruit_to_member_round_trips() {
    round_trip_effect(vec![StateMutation::AddFruitToMember {
        member_id: member_id(),
        fruit: FRUITS[0],
    }]);
}

#[test]
fn mutation_remove_fruit_from_member_round_trips() {
    round_trip_effect(vec![StateMutation::RemoveFruitFromMember {
        member_id: member_id(),
        fruit: FRUITS[0],
    }]);
}

#[test]
fn mutation_add_member_round_trips() {
    let bag = Bag::new().insert(GRAPES).insert(MELON);
    let member = Member::new("Alice").with_id(member_id()).with_bag(bag);
    round_trip_effect(vec![StateMutation::AddMember { member }]);
}

#[test]
fn mutation_remove_member_round_trips() {
    round_trip_effect(vec![StateMutation::RemoveMember {
        member_id: member_id(),
    }]);
}

#[test]
fn mutation_set_community_luck_round_trips() {
    round_trip_effect(vec![StateMutation::SetCommunityLuck { luck: 200 }]);
}

#[test]
fn mutation_set_member_luck_round_trips() {
    round_trip_effect(vec![StateMutation::SetMemberLuck {
        member_id: member_id(),
        luck: 100,
    }]);
}

#[test]
fn mutation_gift_luck_bonus_round_trips() {
    round_trip_effect(vec![StateMutation::GiftLuckBonus {
        member_id: member_id(),
        delta: 10,
    }]);
}

#[test]
fn mutation_burn_luck_bonus_round_trips() {
    round_trip_effect(vec![StateMutation::BurnLuckBonus { delta: 5 }]);
}

#[test]
fn mutation_ostentatious_gift_penalty_round_trips() {
    round_trip_effect(vec![StateMutation::OstentatiousGiftPenalty {
        member_id: member_id(),
        delta: -15,
    }]);
}

#[test]
fn mutation_ostentatious_burn_penalty_round_trips() {
    round_trip_effect(vec![StateMutation::OstentatiousBurnPenalty {
        member_id: member_id(),
        delta: -20,
    }]);
}

#[test]
fn mutation_quid_pro_quo_penalty_round_trips() {
    round_trip_effect(vec![StateMutation::QuidProQuoPenalty { delta: -3 }]);
}

// ── event_type_name ───────────────────────────────────────────────────────────

#[test]
fn event_type_name_grant() {
    assert_eq!(
        event_type_name(&EventPayload::Grant { count: 1 }),
        EVENT_TYPE_GRANT
    );
}

#[test]
fn event_type_name_add_member() {
    assert_eq!(
        event_type_name(&EventPayload::AddMember {
            display_name: "X".to_string(),
            member_id: member_id(),
            external_id: None,
        }),
        EVENT_TYPE_ADD_MEMBER
    );
}

#[test]
fn event_type_name_remove_member() {
    assert_eq!(
        event_type_name(&EventPayload::RemoveMember {
            member_id: member_id()
        }),
        EVENT_TYPE_REMOVE_MEMBER
    );
}

#[test]
fn event_type_name_set_community_luck() {
    assert_eq!(
        event_type_name(&EventPayload::SetCommunityLuck { luck: 0 }),
        EVENT_TYPE_SET_COMMUNITY_LUCK
    );
}

#[test]
fn event_type_name_set_member_luck() {
    assert_eq!(
        event_type_name(&EventPayload::SetMemberLuck {
            member_id: member_id(),
            luck: 0
        }),
        EVENT_TYPE_SET_MEMBER_LUCK
    );
}

#[test]
fn event_type_name_gift() {
    assert_eq!(
        event_type_name(&EventPayload::Gift {
            sender_id: member_id(),
            recipient_id: member_id_2(),
            fruit: FRUITS[0],
            message: None,
        }),
        EVENT_TYPE_GIFT
    );
}

#[test]
fn event_type_name_burn() {
    assert_eq!(
        event_type_name(&EventPayload::Burn {
            member_id: member_id(),
            fruits: vec![],
            message: None,
        }),
        EVENT_TYPE_BURN
    );
}

// ── Sort key helpers ──────────────────────────────────────────────────────────

#[test]
fn sk_event_formats_zero_padded_20_digits() {
    assert_eq!(sk_event(seq(1)), "EVENT#00000000000000000001");
}

#[test]
fn sk_event_formats_large_value() {
    assert_eq!(sk_event(seq(u64::MAX)), format!("EVENT#{:020}", u64::MAX));
}

#[test]
fn sk_effect_formats_zero_padded_20_digits() {
    assert_eq!(sk_effect(seq(1)), "EFFECT#00000000000000000001");
}

#[test]
fn sk_event_range_lower_is_after_plus_one() {
    let (lower, _) = sk_event_range(seq(5), None);
    assert_eq!(lower, "EVENT#00000000000000000006");
}

#[test]
fn sk_event_range_upper_is_u64_max_when_no_before() {
    let (_, upper) = sk_event_range(seq(5), None);
    assert_eq!(upper, format!("EVENT#{:020}", u64::MAX));
}

#[test]
fn sk_event_range_upper_is_before_minus_one() {
    let (_, upper) = sk_event_range(seq(0), Some(seq(10)));
    assert_eq!(upper, "EVENT#00000000000000000009");
}

#[test]
fn sk_event_range_saturating_add_at_max() {
    let (lower, _) = sk_event_range(SequenceId::new(u64::MAX), None);
    assert_eq!(lower, format!("EVENT#{:020}", u64::MAX));
}

#[test]
fn sk_event_range_saturating_sub_at_zero() {
    let (_, upper) = sk_event_range(seq(0), Some(SequenceId::new(0)));
    // saturating_sub(1) on 0 = 0
    assert_eq!(upper, "EVENT#00000000000000000000");
}

#[test]
fn sk_effect_range_after_lower_is_after_plus_one() {
    let (lower, _) = sk_effect_range_after(seq(3));
    assert_eq!(lower, "EFFECT#00000000000000000004");
}

#[test]
fn sk_effect_range_after_upper_is_u64_max() {
    let (_, upper) = sk_effect_range_after(seq(3));
    assert_eq!(upper, format!("EFFECT#{:020}", u64::MAX));
}

// ── build_records ─────────────────────────────────────────────────────────────

#[test]
fn build_records_with_matching_effect() {
    let event = make_event(seq(1), EventPayload::Grant { count: 3 });
    let effect = make_effect(seq(1), vec![]);
    let event_item = encode_event(&event).unwrap();
    let effect_map: HashMap<SequenceId, Effect> = [(seq(1), effect.clone())].into();

    let records = build_records(vec![event_item], &effect_map).unwrap();
    assert_eq!(
        records,
        vec![Record {
            event,
            effect: Some(effect),
        }]
    );
}

#[test]
fn build_records_without_effect() {
    let event = make_event(seq(1), EventPayload::Grant { count: 3 });
    let event_item = encode_event(&event).unwrap();
    let effect_map: HashMap<SequenceId, Effect> = HashMap::new();

    let records = build_records(vec![event_item], &effect_map).unwrap();
    assert_eq!(
        records,
        vec![Record {
            event,
            effect: None,
        }]
    );
}

// ── decode_event error paths ──────────────────────────────────────────────────

#[test]
fn decode_event_bad_pk_returns_codec_error() {
    // pk with wrong byte count (not 16)
    let bad_pk = serde_bytes::ByteBuf::from(vec![0u8; 5]);
    let item: EventItem = EventItem {
        pk: bad_pk,
        sk: sk_event(seq(1)),
        seq: 1,
        entity_type: "EVENT".to_string(),
        event_type: "Grant".to_string(),
        payload: EventPayloadDto::Grant { count: 1 },
    };
    let attr_map =
        serde_dynamo::aws_sdk_dynamodb_1::to_item(&item).expect("to_item should not fail");
    let result = decode_event(attr_map);
    assert!(result.is_err());
}

#[test]
fn decode_event_unknown_fruit_in_gift_payload_returns_codec_error() {
    use newtype_ids_uuid::UuidIdentifier as _;
    let community_id_val = CommunityId::from(Uuid::from_bytes([1u8; 16]));
    let mid_bytes = serde_bytes::ByteBuf::from(Uuid::from_bytes([2u8; 16]).as_bytes().to_vec());
    let item = EventItem {
        pk: serde_bytes::ByteBuf::from(community_id_val.as_uuid().as_bytes().to_vec()),
        sk: sk_event(seq(1)),
        seq: 1,
        entity_type: "EVENT".to_string(),
        event_type: "Gift".to_string(),
        payload: EventPayloadDto::Gift {
            sender_id: mid_bytes.clone(),
            recipient_id: mid_bytes,
            fruit: "NotARealFruit".to_string(),
            message: None,
        },
    };
    let attr_map =
        serde_dynamo::aws_sdk_dynamodb_1::to_item(&item).expect("to_item should not fail");
    let result = decode_event(attr_map);
    assert!(result.is_err());
}

#[test]
fn decode_effect_bad_pk_returns_codec_error() {
    let item = EffectItem {
        pk: serde_bytes::ByteBuf::from(vec![0u8; 5]),
        sk: sk_effect(seq(1)),
        seq: 1,
        entity_type: "EFFECT".to_string(),
        mutations: vec![],
    };
    let attr_map =
        serde_dynamo::aws_sdk_dynamodb_1::to_item(&item).expect("to_item should not fail");
    let result = decode_effect(attr_map);
    assert!(result.is_err());
}

#[test]
fn decode_effect_unknown_fruit_in_mutation_returns_codec_error() {
    use newtype_ids_uuid::UuidIdentifier as _;
    let community_id_val = CommunityId::from(Uuid::from_bytes([1u8; 16]));
    let mid_bytes = serde_bytes::ByteBuf::from(Uuid::from_bytes([2u8; 16]).as_bytes().to_vec());
    let item = EffectItem {
        pk: serde_bytes::ByteBuf::from(community_id_val.as_uuid().as_bytes().to_vec()),
        sk: sk_effect(seq(1)),
        seq: 1,
        entity_type: "EFFECT".to_string(),
        mutations: vec![StateMutationDto::AddFruitToMember {
            member_id: mid_bytes,
            fruit: "NotAFruit".to_string(),
        }],
    };
    let attr_map =
        serde_dynamo::aws_sdk_dynamodb_1::to_item(&item).expect("to_item should not fail");
    let result = decode_effect(attr_map);
    assert!(result.is_err());
}

#[test]
fn decode_event_invalid_dynamo_item_returns_codec_error() {
    // Pass an empty map — serde_dynamo will fail to deserialize
    let result = decode_event(HashMap::new());
    assert!(result.is_err());
}

#[test]
fn decode_effect_invalid_dynamo_item_returns_codec_error() {
    let result = decode_effect(HashMap::new());
    assert!(result.is_err());
}
