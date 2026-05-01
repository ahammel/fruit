use std::error::Error as StdError;

use anomalies::{
    anomaly::{HasCategory, HasStatus},
    category::{Busy, Fault, Incorrect, Interrupted, Unavailable},
    status::Status,
};
use aws_sdk_dynamodb::{error::SdkError, operation::put_item::PutItemError};
use aws_smithy_runtime_api::client::result::ConnectorError;
use aws_smithy_runtime_api::http::{Response, StatusCode};
use aws_smithy_types::body::SdkBody;

use super::*;

type TestSdkError = SdkError<PutItemError>;

// ── From<SdkError> – TimeoutError arm ────────────────────────────────────────

#[test]
fn from_sdk_error_timeout_maps_to_timeout_variant() {
    let err: TestSdkError = SdkError::timeout_error(std::io::Error::other("timeout"));
    let anomaly = SdkAnomaly::from(err);
    assert!(matches!(anomaly, SdkAnomaly::TimeoutError(_)));
}

#[test]
fn timeout_variant_is_interrupted_temporary() {
    let err: TestSdkError = SdkError::timeout_error(std::io::Error::other("timeout"));
    let anomaly = SdkAnomaly::from(err);
    assert_eq!(anomaly.category(), Interrupted);
    assert_eq!(anomaly.status(), Status::Temporary);
}

// ── From<SdkError> – DispatchFailure arm ─────────────────────────────────────

#[test]
fn from_sdk_error_dispatch_failure_maps_to_dispatch_variant() {
    let ce = ConnectorError::user(Box::new(std::io::Error::other("user err")));
    let err: TestSdkError = SdkError::dispatch_failure(ce);
    let anomaly = SdkAnomaly::from(err);
    assert!(matches!(anomaly, SdkAnomaly::DispatchFailure(_)));
}

#[test]
fn dispatch_failure_timeout_connector_maps_to_interrupted_temporary() {
    let ce = ConnectorError::timeout(Box::new(std::io::Error::other("conn timeout")));
    let err: TestSdkError = SdkError::dispatch_failure(ce);
    let anomaly = SdkAnomaly::from(err);
    assert_eq!(anomaly.category(), Interrupted);
    assert_eq!(anomaly.status(), Status::Temporary);
}

#[test]
fn dispatch_failure_user_connector_maps_to_incorrect_permanent() {
    let ce = ConnectorError::user(Box::new(std::io::Error::other("user")));
    let err: TestSdkError = SdkError::dispatch_failure(ce);
    let anomaly = SdkAnomaly::from(err);
    assert_eq!(anomaly.category(), Incorrect);
    assert_eq!(anomaly.status(), Status::Permanent);
}

#[test]
fn dispatch_failure_io_connector_maps_to_unavailable_temporary() {
    let ce = ConnectorError::io(Box::new(std::io::Error::other("io")));
    let err: TestSdkError = SdkError::dispatch_failure(ce);
    let anomaly = SdkAnomaly::from(err);
    assert_eq!(anomaly.category(), Unavailable);
    assert_eq!(anomaly.status(), Status::Temporary);
}

#[test]
fn dispatch_failure_other_connector_maps_to_fault_permanent() {
    let ce = ConnectorError::other(Box::new(std::io::Error::other("other")), None);
    let err: TestSdkError = SdkError::dispatch_failure(ce);
    let anomaly = SdkAnomaly::from(err);
    assert_eq!(anomaly.category(), Fault);
    assert_eq!(anomaly.status(), Status::Permanent);
}

// ── From<SdkError> – ResponseError arm ───────────────────────────────────────

#[test]
fn from_sdk_error_response_error_maps_to_response_variant() {
    let err: TestSdkError = SdkError::response_error(
        std::io::Error::other("parse failed"),
        Response::new(StatusCode::try_from(200u16).unwrap(), SdkBody::empty()),
    );
    let anomaly = SdkAnomaly::from(err);
    assert!(matches!(anomaly, SdkAnomaly::ResponseError(_)));
}

#[test]
fn response_error_variant_is_fault_permanent() {
    let err: TestSdkError = SdkError::response_error(
        std::io::Error::other("parse failed"),
        Response::new(StatusCode::try_from(200u16).unwrap(), SdkBody::empty()),
    );
    let anomaly = SdkAnomaly::from(err);
    assert_eq!(anomaly.category(), Fault);
    assert_eq!(anomaly.status(), Status::Permanent);
}

// ── From<SdkError> – ServiceError unknown code → Fault/Permanent ─────────────

#[test]
fn service_error_unknown_code_maps_to_fault_permanent() {
    use aws_sdk_dynamodb::types::error::ConditionalCheckFailedException;
    use aws_smithy_types::error::ErrorMetadata;

    let meta = ErrorMetadata::builder()
        .code("ConditionalCheckFailedException")
        .build();
    let typed = ConditionalCheckFailedException::builder()
        .meta(meta)
        .build();
    let err: TestSdkError = SdkError::service_error(
        PutItemError::ConditionalCheckFailedException(typed),
        Response::new(StatusCode::try_from(400u16).unwrap(), SdkBody::empty()),
    );
    let anomaly = SdkAnomaly::from(err);
    assert_eq!(anomaly.category(), Fault);
    assert_eq!(anomaly.status(), Status::Permanent);
}

// ── TimeoutError inner type ───────────────────────────────────────────────────

#[test]
fn timeout_error_display_contains_inner_message() {
    let te = TimeoutError(Box::new(std::io::Error::other("timed out msg")));
    assert!(te.to_string().contains("timed out msg"));
}

#[test]
fn timeout_error_source_is_none() {
    let te = TimeoutError(Box::new(std::io::Error::other("x")));
    assert!(StdError::source(&te).is_none());
}

// ── DispatchFailure inner type ────────────────────────────────────────────────

#[test]
fn dispatch_failure_direct_display() {
    let df = DispatchFailure {
        category: Fault,
        status: Status::Permanent,
        err: Box::new(std::io::Error::other("dispatch msg")),
    };
    assert!(df.to_string().contains("dispatch msg"));
}

#[test]
fn dispatch_failure_direct_source_is_none() {
    let df = DispatchFailure {
        category: Fault,
        status: Status::Permanent,
        err: Box::new(std::io::Error::other("x")),
    };
    assert!(StdError::source(&df).is_none());
}

#[test]
fn dispatch_failure_direct_category() {
    let df = DispatchFailure {
        category: Unavailable,
        status: Status::Temporary,
        err: Box::new(std::io::Error::other("x")),
    };
    assert_eq!(df.category(), Unavailable);
}

#[test]
fn dispatch_failure_direct_status() {
    let df = DispatchFailure {
        category: Unavailable,
        status: Status::Temporary,
        err: Box::new(std::io::Error::other("x")),
    };
    assert_eq!(df.status(), Status::Temporary);
}

// ── ResponseError inner type ──────────────────────────────────────────────────

#[test]
fn response_error_display_contains_inner_message() {
    let re = ResponseError(Box::new(std::io::Error::other("response msg")));
    assert!(re.to_string().contains("response msg"));
}

#[test]
fn response_error_source_is_none() {
    let re = ResponseError(Box::new(std::io::Error::other("x")));
    assert!(StdError::source(&re).is_none());
}

// ── ServiceError inner type ───────────────────────────────────────────────────

#[test]
fn service_error_direct_display() {
    let se = ServiceError {
        category: Busy,
        status: Status::Temporary,
        err: Box::new(std::io::Error::other("service msg")),
    };
    assert!(se.to_string().contains("service msg"));
}

#[test]
fn service_error_source_is_some() {
    let se = ServiceError {
        category: Busy,
        status: Status::Temporary,
        err: Box::new(std::io::Error::other("x")),
    };
    assert!(StdError::source(&se).is_some());
}

#[test]
fn service_error_direct_category() {
    let se = ServiceError {
        category: Busy,
        status: Status::Temporary,
        err: Box::new(std::io::Error::other("x")),
    };
    assert_eq!(se.category(), Busy);
}

#[test]
fn service_error_direct_status() {
    let se = ServiceError {
        category: Busy,
        status: Status::Temporary,
        err: Box::new(std::io::Error::other("x")),
    };
    assert_eq!(se.status(), Status::Temporary);
}
