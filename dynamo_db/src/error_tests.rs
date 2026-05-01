use std::error::Error as StdError;

use anomalies::{
    anomaly::{HasCategory, HasStatus},
    category::{Busy, Fault, Unavailable},
    status::Status,
};
use aws_sdk_dynamodb::error::SdkError;
use newtype_ids::IntegerIdentifier as _;
use newtype_ids_uuid::UuidIdentifier as _;

use super::*;

mod test_helpers {
    use fruit_domain::{community::CommunityId, event_log::SequenceId};
    use uuid::Uuid;

    /// A fixed community ID.
    pub fn community_id() -> CommunityId {
        CommunityId::from(Uuid::from_bytes([1u8; 16]))
    }

    /// A fixed sequence ID.
    pub fn seq(n: u64) -> SequenceId {
        SequenceId::new(n)
    }
}

use test_helpers::*;

// ── raise_codec_err ───────────────────────────────────────────────────────────

#[test]
fn raise_codec_err_produces_codec_variant() {
    let inner = std::io::Error::other("inner cause");
    let exn = raise_codec_err("test context", inner);
    assert!(matches!(*exn, Error::Codec { .. }));
}

#[test]
fn raise_codec_err_message_matches_context() {
    let inner = std::io::Error::other("inner cause");
    let exn = raise_codec_err("test context", inner);
    let Error::Codec { message } = &*exn else {
        panic!("expected Codec variant");
    };
    assert_eq!(message, "test context");
}

#[test]
fn raise_codec_err_display_contains_codec_prefix() {
    let inner = std::io::Error::other("inner cause");
    let exn = raise_codec_err("test context", inner);
    let display = exn.to_string();
    assert!(
        display.contains("DynamoDB codec error"),
        "display was: {display}"
    );
}

// ── raise_conflict_err ────────────────────────────────────────────────────────

#[test]
fn raise_conflict_err_produces_already_exists_event() {
    let cid = community_id();
    let ver = seq(5);
    let err: SdkError<aws_sdk_dynamodb::operation::put_item::PutItemError> =
        SdkError::construction_failure("simulated conflict");
    let exn = raise_conflict_err(cid, ver, Entity::Event, err);
    let Error::AlreadyExists {
        community,
        version,
        entity,
    } = &*exn
    else {
        panic!("expected AlreadyExists, got {:?}", &*exn);
    };
    assert_eq!(community.as_uuid(), cid.as_uuid());
    assert_eq!(version.as_u64(), 5);
    assert!(matches!(entity, Entity::Event));
}

#[test]
fn raise_conflict_err_produces_already_exists_community() {
    let cid = community_id();
    let ver = seq(3);
    let err: SdkError<aws_sdk_dynamodb::operation::put_item::PutItemError> =
        SdkError::construction_failure("simulated conflict");
    let exn = raise_conflict_err(cid, ver, Entity::Community, err);
    let Error::AlreadyExists { entity, .. } = &*exn else {
        panic!("expected AlreadyExists");
    };
    assert!(matches!(entity, Entity::Community));
}

#[test]
fn raise_conflict_err_produces_already_exists_effect() {
    let cid = community_id();
    let ver = seq(1);
    let err: SdkError<aws_sdk_dynamodb::operation::put_item::PutItemError> =
        SdkError::construction_failure("simulated conflict");
    let exn = raise_conflict_err(cid, ver, Entity::Effect, err);
    let Error::AlreadyExists { entity, .. } = &*exn else {
        panic!("expected AlreadyExists");
    };
    assert!(matches!(entity, Entity::Effect));
}

// ── raise_sdk_err – service error code classification ─────────────────────────

#[test]
fn raise_sdk_err_throttle_maps_to_busy_temporary() {
    use aws_sdk_dynamodb::operation::put_item::PutItemError;
    use aws_sdk_dynamodb::types::error::ProvisionedThroughputExceededException;
    use aws_smithy_runtime_api::http::{Response, StatusCode};
    use aws_smithy_types::{body::SdkBody, error::ErrorMetadata};

    let meta = ErrorMetadata::builder()
        .code("ProvisionedThroughputExceededException")
        .build();
    let typed = ProvisionedThroughputExceededException::builder()
        .meta(meta)
        .build();
    let err: SdkError<PutItemError> = SdkError::service_error(
        PutItemError::ProvisionedThroughputExceededException(typed),
        Response::new(StatusCode::try_from(400u16).unwrap(), SdkBody::empty()),
    );
    let exn = raise_sdk_err("throttle ctx", err);
    assert_eq!(exn.category(), Busy);
    assert_eq!(exn.status(), Status::Temporary);
}

#[test]
fn raise_sdk_err_construction_failure_maps_to_fault_permanent() {
    let err: SdkError<aws_sdk_dynamodb::operation::put_item::PutItemError> =
        SdkError::construction_failure("bad request build");
    let exn = raise_sdk_err("construction ctx", err);
    // ConstructionFailure is Incorrect but raise_sdk_err remaps to Fault/Permanent
    assert_eq!(exn.status(), Status::Permanent);
    assert_eq!(exn.category(), Fault);
}

#[test]
fn raise_sdk_err_internal_server_error_maps_to_unavailable_temporary() {
    use aws_sdk_dynamodb::operation::put_item::PutItemError;
    use aws_sdk_dynamodb::types::error::InternalServerError;
    use aws_smithy_runtime_api::http::{Response, StatusCode};
    use aws_smithy_types::{body::SdkBody, error::ErrorMetadata};

    let meta = ErrorMetadata::builder().code("InternalServerError").build();
    let typed = InternalServerError::builder().meta(meta).build();
    let err: SdkError<PutItemError> = SdkError::service_error(
        PutItemError::InternalServerError(typed),
        Response::new(StatusCode::try_from(500u16).unwrap(), SdkBody::empty()),
    );
    let exn = raise_sdk_err("ise ctx", err);
    assert_eq!(exn.category(), Unavailable);
    assert_eq!(exn.status(), Status::Temporary);
}

// ── DynamoSdkError Display / source ──────────────────────────────────────────

#[test]
fn dynamo_sdk_error_display_shows_context() {
    let err: SdkError<aws_sdk_dynamodb::operation::put_item::PutItemError> =
        SdkError::construction_failure("inner");
    let exn = raise_sdk_err("my context string", err);
    let display = exn.to_string();
    assert!(display.contains("my context string"), "got: {display}");
}

#[test]
fn dynamo_sdk_error_source_is_some() {
    let err: SdkError<aws_sdk_dynamodb::operation::put_item::PutItemError> =
        SdkError::construction_failure("inner");
    let exn = raise_sdk_err("ctx", err);
    let Error::Sdk(sdk_err) = &*exn else {
        panic!("expected Sdk variant");
    };
    assert!(StdError::source(sdk_err).is_some());
}

// ── Error::AlreadyExists Display ─────────────────────────────────────────────

#[test]
fn already_exists_display_contains_community_and_version() {
    let cid = community_id();
    let ver = seq(7);
    let err = Error::AlreadyExists {
        community: cid,
        version: ver,
        entity: Entity::Event,
    };
    let s = err.to_string();
    assert!(
        s.contains(&cid.as_uuid().to_string()),
        "display missing community id: {s}"
    );
    assert!(s.contains('7'), "display missing version: {s}");
}
