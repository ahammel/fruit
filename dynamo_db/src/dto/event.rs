use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use exn::Exn;
use fruit_domain::{
    community::CommunityId,
    event_log::{Effect, Event, EventPayload, Record, SequenceId, StateMutation},
    fruit::{Fruit, FRUITS},
    member::{Member, MemberId},
};
use newtype_ids::IntegerIdentifier as _;
use newtype_ids_uuid::UuidIdentifier as _;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{raise_codec_err, Error};

// ── Fruit codec ───────────────────────────────────────────────────────────────

pub(crate) fn fruit_name(fruit: Fruit) -> String {
    fruit.name.to_string()
}

fn fruit_from_name(name: &str) -> Result<Fruit, Error> {
    FRUITS
        .iter()
        .copied()
        .find(|f| f.name == name)
        .ok_or_else(|| Error::Codec {
            message: format!("unknown fruit '{name}'"),
        })
}

// ── ID helpers ────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str, context: &str) -> Result<Uuid, Error> {
    Uuid::parse_str(s).map_err(|e| Error::Codec {
        message: format!("{context}: {e}"),
    })
}

fn member_id(s: &str) -> Result<MemberId, Error> {
    parse_uuid(s, "invalid member_id").map(MemberId::from)
}

fn community_id_from_str(s: &str) -> Result<CommunityId, Error> {
    parse_uuid(s, "invalid community_id").map(CommunityId::from)
}

// ── EventPayload ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum EventPayloadDto {
    Grant {
        count: usize,
    },
    AddMember {
        display_name: String,
        member_id: String,
    },
    RemoveMember {
        member_id: String,
    },
    SetCommunityLuck {
        luck: u8,
    },
    SetMemberLuck {
        member_id: String,
        luck: u8,
    },
    Gift {
        sender_id: String,
        recipient_id: String,
        fruit: String,
        message: Option<String>,
    },
    Burn {
        member_id: String,
        fruits: Vec<String>,
    },
}

impl From<&EventPayload> for EventPayloadDto {
    fn from(p: &EventPayload) -> Self {
        match p {
            EventPayload::Grant { count } => EventPayloadDto::Grant { count: *count },
            EventPayload::AddMember {
                display_name,
                member_id,
            } => EventPayloadDto::AddMember {
                display_name: display_name.clone(),
                member_id: member_id.as_uuid().to_string(),
            },
            EventPayload::RemoveMember { member_id } => EventPayloadDto::RemoveMember {
                member_id: member_id.as_uuid().to_string(),
            },
            EventPayload::SetCommunityLuck { luck } => {
                EventPayloadDto::SetCommunityLuck { luck: *luck }
            }
            EventPayload::SetMemberLuck { member_id, luck } => EventPayloadDto::SetMemberLuck {
                member_id: member_id.as_uuid().to_string(),
                luck: *luck,
            },
            EventPayload::Gift {
                sender_id,
                recipient_id,
                fruit,
                message,
            } => EventPayloadDto::Gift {
                sender_id: sender_id.as_uuid().to_string(),
                recipient_id: recipient_id.as_uuid().to_string(),
                fruit: fruit_name(*fruit),
                message: message.clone(),
            },
            EventPayload::Burn { member_id, fruits } => EventPayloadDto::Burn {
                member_id: member_id.as_uuid().to_string(),
                fruits: fruits.iter().copied().map(fruit_name).collect(),
            },
        }
    }
}

impl TryFrom<EventPayloadDto> for EventPayload {
    type Error = Error;

    fn try_from(dto: EventPayloadDto) -> Result<Self, Error> {
        Ok(match dto {
            EventPayloadDto::Grant { count } => EventPayload::Grant { count },
            EventPayloadDto::AddMember {
                display_name,
                member_id: mid,
            } => EventPayload::AddMember {
                display_name,
                member_id: member_id(&mid)?,
            },
            EventPayloadDto::RemoveMember { member_id: mid } => EventPayload::RemoveMember {
                member_id: member_id(&mid)?,
            },
            EventPayloadDto::SetCommunityLuck { luck } => EventPayload::SetCommunityLuck { luck },
            EventPayloadDto::SetMemberLuck {
                member_id: mid,
                luck,
            } => EventPayload::SetMemberLuck {
                member_id: member_id(&mid)?,
                luck,
            },
            EventPayloadDto::Gift {
                sender_id,
                recipient_id,
                fruit,
                message,
            } => EventPayload::Gift {
                sender_id: member_id(&sender_id)?,
                recipient_id: member_id(&recipient_id)?,
                fruit: fruit_from_name(&fruit)?,
                message,
            },
            EventPayloadDto::Burn {
                member_id: mid,
                fruits,
            } => EventPayload::Burn {
                member_id: member_id(&mid)?,
                fruits: fruits
                    .iter()
                    .map(|n| fruit_from_name(n))
                    .collect::<Result<_, _>>()?,
            },
        })
    }
}

// ── StateMutation ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum StateMutationDto {
    AddFruitToMember { member_id: String, fruit: String },
    RemoveFruitFromMember { member_id: String, fruit: String },
    AddMember { member: MemberDto },
    RemoveMember { member_id: String },
    SetCommunityLuck { luck: u8 },
    SetMemberLuck { member_id: String, luck: u8 },
    GiftLuckBonus { member_id: String, delta: i16 },
    BurnLuckBonus { delta: i16 },
    OstentatiousGiftPenalty { member_id: String, delta: i16 },
    OstentatiousBurnPenalty { member_id: String, delta: i16 },
    QuidProQuoPenalty { delta: i16 },
}

impl From<&StateMutation> for StateMutationDto {
    fn from(m: &StateMutation) -> Self {
        match m {
            StateMutation::AddFruitToMember { member_id, fruit } => {
                StateMutationDto::AddFruitToMember {
                    member_id: member_id.as_uuid().to_string(),
                    fruit: fruit_name(*fruit),
                }
            }
            StateMutation::RemoveFruitFromMember { member_id, fruit } => {
                StateMutationDto::RemoveFruitFromMember {
                    member_id: member_id.as_uuid().to_string(),
                    fruit: fruit_name(*fruit),
                }
            }
            StateMutation::AddMember { member } => StateMutationDto::AddMember {
                member: MemberDto::from(member),
            },
            StateMutation::RemoveMember { member_id } => StateMutationDto::RemoveMember {
                member_id: member_id.as_uuid().to_string(),
            },
            StateMutation::SetCommunityLuck { luck } => {
                StateMutationDto::SetCommunityLuck { luck: *luck }
            }
            StateMutation::SetMemberLuck { member_id, luck } => StateMutationDto::SetMemberLuck {
                member_id: member_id.as_uuid().to_string(),
                luck: *luck,
            },
            StateMutation::GiftLuckBonus { member_id, delta } => StateMutationDto::GiftLuckBonus {
                member_id: member_id.as_uuid().to_string(),
                delta: *delta,
            },
            StateMutation::BurnLuckBonus { delta } => {
                StateMutationDto::BurnLuckBonus { delta: *delta }
            }
            StateMutation::OstentatiousGiftPenalty { member_id, delta } => {
                StateMutationDto::OstentatiousGiftPenalty {
                    member_id: member_id.as_uuid().to_string(),
                    delta: *delta,
                }
            }
            StateMutation::OstentatiousBurnPenalty { member_id, delta } => {
                StateMutationDto::OstentatiousBurnPenalty {
                    member_id: member_id.as_uuid().to_string(),
                    delta: *delta,
                }
            }
            StateMutation::QuidProQuoPenalty { delta } => {
                StateMutationDto::QuidProQuoPenalty { delta: *delta }
            }
        }
    }
}

impl TryFrom<StateMutationDto> for StateMutation {
    type Error = Error;

    fn try_from(dto: StateMutationDto) -> Result<Self, Error> {
        Ok(match dto {
            StateMutationDto::AddFruitToMember {
                member_id: mid,
                fruit,
            } => StateMutation::AddFruitToMember {
                member_id: member_id(&mid)?,
                fruit: fruit_from_name(&fruit)?,
            },
            StateMutationDto::RemoveFruitFromMember {
                member_id: mid,
                fruit,
            } => StateMutation::RemoveFruitFromMember {
                member_id: member_id(&mid)?,
                fruit: fruit_from_name(&fruit)?,
            },
            StateMutationDto::AddMember { member } => StateMutation::AddMember {
                member: Member::try_from(member)?,
            },
            StateMutationDto::RemoveMember { member_id: mid } => StateMutation::RemoveMember {
                member_id: member_id(&mid)?,
            },
            StateMutationDto::SetCommunityLuck { luck } => StateMutation::SetCommunityLuck { luck },
            StateMutationDto::SetMemberLuck {
                member_id: mid,
                luck,
            } => StateMutation::SetMemberLuck {
                member_id: member_id(&mid)?,
                luck,
            },
            StateMutationDto::GiftLuckBonus {
                member_id: mid,
                delta,
            } => StateMutation::GiftLuckBonus {
                member_id: member_id(&mid)?,
                delta,
            },
            StateMutationDto::BurnLuckBonus { delta } => StateMutation::BurnLuckBonus { delta },
            StateMutationDto::OstentatiousGiftPenalty {
                member_id: mid,
                delta,
            } => StateMutation::OstentatiousGiftPenalty {
                member_id: member_id(&mid)?,
                delta,
            },
            StateMutationDto::OstentatiousBurnPenalty {
                member_id: mid,
                delta,
            } => StateMutation::OstentatiousBurnPenalty {
                member_id: member_id(&mid)?,
                delta,
            },
            StateMutationDto::QuidProQuoPenalty { delta } => {
                StateMutation::QuidProQuoPenalty { delta }
            }
        })
    }
}

// ── Member ────────────────────────────────────────────────────────────────────

/// Serialised form of a [`Member`].
///
/// The `bag` field maps fruit names to counts so that DynamoDB stores it as a
/// native Map attribute rather than a list of pairs.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct MemberDto {
    pub id: String,
    pub display_name: String,
    pub luck: u8,
    pub bag: HashMap<String, usize>,
}

impl From<&Member> for MemberDto {
    fn from(m: &Member) -> Self {
        MemberDto {
            id: m.id.as_uuid().to_string(),
            display_name: m.display_name.clone(),
            luck: m.luck_raw(),
            bag: m
                .bag
                .iter()
                .map(|(fruit, count)| (fruit_name(fruit), count))
                .collect(),
        }
    }
}

impl TryFrom<MemberDto> for Member {
    type Error = Error;

    fn try_from(dto: MemberDto) -> Result<Self, Error> {
        use fruit_domain::bag::Bag;
        let id = member_id(&dto.id)?;
        let bag = dto.bag.iter().try_fold(Bag::new(), |bag, (name, &count)| {
            let fruit = fruit_from_name(name)?;
            Ok((0..count).fold(bag, |b, _| b.insert(fruit)))
        })?;
        Ok(Member::new(dto.display_name)
            .with_id(id)
            .with_luck(dto.luck)
            .with_bag(bag))
    }
}

// ── DynamoDB item encode/decode ───────────────────────────────────────────────

/// The DynamoDB item structure for an event.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EventItem {
    pub pk: String,
    pub sk: String,
    pub seq: u64,
    pub entity_type: String,
    /// Payload variant name used for event-type filtering queries.
    pub event_type: String,
    pub payload: EventPayloadDto,
}

/// The DynamoDB item structure for an effect.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EffectItem {
    pub pk: String,
    pub sk: String,
    pub seq: u64,
    pub entity_type: String,
    pub mutations: Vec<StateMutationDto>,
}

pub(crate) const EVENT_TYPE_GRANT: &str = "Grant";
pub(crate) const EVENT_TYPE_ADD_MEMBER: &str = "AddMember";
pub(crate) const EVENT_TYPE_REMOVE_MEMBER: &str = "RemoveMember";
pub(crate) const EVENT_TYPE_SET_COMMUNITY_LUCK: &str = "SetCommunityLuck";
pub(crate) const EVENT_TYPE_SET_MEMBER_LUCK: &str = "SetMemberLuck";
pub(crate) const EVENT_TYPE_GIFT: &str = "Gift";
pub(crate) const EVENT_TYPE_BURN: &str = "Burn";

/// Returns the `event_type` DynamoDB attribute value for a payload variant.
pub(crate) fn event_type_name(payload: &EventPayload) -> &'static str {
    match payload {
        EventPayload::Grant { .. } => EVENT_TYPE_GRANT,
        EventPayload::AddMember { .. } => EVENT_TYPE_ADD_MEMBER,
        EventPayload::RemoveMember { .. } => EVENT_TYPE_REMOVE_MEMBER,
        EventPayload::SetCommunityLuck { .. } => EVENT_TYPE_SET_COMMUNITY_LUCK,
        EventPayload::SetMemberLuck { .. } => EVENT_TYPE_SET_MEMBER_LUCK,
        EventPayload::Gift { .. } => EVENT_TYPE_GIFT,
        EventPayload::Burn { .. } => EVENT_TYPE_BURN,
    }
}

/// Encodes an [`Event`] into a DynamoDB attribute map.
pub(crate) fn encode_event(event: &Event) -> Result<HashMap<String, AttributeValue>, Exn<Error>> {
    let item = EventItem {
        pk: event.community_id.as_uuid().to_string(),
        sk: sk_event(event.id),
        seq: event.id.as_u64(),
        entity_type: "EVENT".to_string(),
        event_type: event_type_name(&event.payload).to_string(),
        payload: EventPayloadDto::from(&event.payload),
    };
    serde_dynamo::aws_sdk_dynamodb_1::to_item(&item)
        .map_err(|e| raise_codec_err("failed to encode event", e))
}

/// Decodes a DynamoDB attribute map into an [`Event`].
pub(crate) fn decode_event(item: HashMap<String, AttributeValue>) -> Result<Event, Exn<Error>> {
    let dto: EventItem = serde_dynamo::aws_sdk_dynamodb_1::from_item(item)
        .map_err(|e| raise_codec_err("failed to decode event", e))?;
    let community_id = community_id_from_str(&dto.pk).map_err(Exn::new)?;
    let payload = EventPayload::try_from(dto.payload).map_err(Exn::new)?;
    Ok(Event {
        id: SequenceId::new(dto.seq),
        community_id,
        payload,
    })
}

/// Encodes an [`Effect`] into a DynamoDB attribute map.
pub(crate) fn encode_effect(
    effect: &Effect,
) -> Result<HashMap<String, AttributeValue>, Exn<Error>> {
    let item = EffectItem {
        pk: effect.community_id.as_uuid().to_string(),
        sk: sk_effect(effect.id),
        seq: effect.id.as_u64(),
        entity_type: "EFFECT".to_string(),
        mutations: effect
            .mutations
            .iter()
            .map(StateMutationDto::from)
            .collect(),
    };
    serde_dynamo::aws_sdk_dynamodb_1::to_item(&item)
        .map_err(|e| raise_codec_err("failed to encode effect", e))
}

/// Decodes a DynamoDB attribute map into an [`Effect`].
pub(crate) fn decode_effect(item: HashMap<String, AttributeValue>) -> Result<Effect, Exn<Error>> {
    let dto: EffectItem = serde_dynamo::aws_sdk_dynamodb_1::from_item(item)
        .map_err(|e| raise_codec_err("failed to decode effect", e))?;
    let community_id = community_id_from_str(&dto.pk).map_err(Exn::new)?;
    let mutations = dto
        .mutations
        .into_iter()
        .map(StateMutation::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(Exn::new)?;
    Ok(Effect {
        id: SequenceId::new(dto.seq),
        community_id,
        mutations,
    })
}

/// Assembles [`Record`]s by pairing events with their effects from separate item maps.
pub(crate) fn build_records(
    event_items: Vec<HashMap<String, AttributeValue>>,
    effect_map: &HashMap<SequenceId, Effect>,
) -> Result<Vec<Record>, Exn<Error>> {
    event_items
        .into_iter()
        .map(|item| {
            let event = decode_event(item)?;
            let effect = effect_map.get(&event.id).cloned();
            Ok(Record { event, effect })
        })
        .collect()
}

// ── Sort key helpers ──────────────────────────────────────────────────────────

pub(crate) fn sk_event(seq: SequenceId) -> String {
    format!("EVENT#{:020}", seq.as_u64())
}

pub(crate) fn sk_effect(seq: SequenceId) -> String {
    format!("EFFECT#{:020}", seq.as_u64())
}

pub(crate) fn sk_event_range(after: SequenceId, before: Option<SequenceId>) -> (String, String) {
    let lower = format!("EVENT#{:020}", after.as_u64().saturating_add(1));
    let upper = match before {
        Some(b) => format!("EVENT#{:020}", b.as_u64().saturating_sub(1)),
        None => format!("EVENT#{:020}", u64::MAX),
    };
    (lower, upper)
}

pub(crate) fn sk_effect_range_after(after: SequenceId) -> (String, String) {
    let lower = format!("EFFECT#{:020}", after.as_u64().saturating_add(1));
    let upper = format!("EFFECT#{:020}", u64::MAX);
    (lower, upper)
}
