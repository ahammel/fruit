use std::error::Error as StdError;

use anomalies::{
    anomaly::{HasCategory, HasStatus},
    category::{Conflict, Fault, Unavailable},
    status::Status,
};
use exn::Exn;
use regex::Regex;

use super::*;

// ── StorageLayerError::build ──────────────────────────────────────────────────

#[test]
fn storage_layer_error_build_display() {
    let err = StorageLayerError::build("something went wrong", Conflict, Status::Permanent);
    assert_eq!(err.to_string(), "Storage layer error: something went wrong");
}

#[test]
fn storage_layer_error_build_preserves_conflict_category() {
    let Error::StorageLayerError(inner) =
        StorageLayerError::build("msg", Conflict, Status::Permanent)
    else {
        panic!("wrong variant")
    };
    assert_eq!(inner.category(), Conflict);
}

#[test]
fn storage_layer_error_build_preserves_fault_category() {
    let Error::StorageLayerError(inner) = StorageLayerError::build("msg", Fault, Status::Permanent)
    else {
        panic!("wrong variant")
    };
    assert_eq!(inner.category(), Fault);
}

#[test]
fn storage_layer_error_build_preserves_permanent_status() {
    let Error::StorageLayerError(inner) =
        StorageLayerError::build("msg", Conflict, Status::Permanent)
    else {
        panic!("wrong variant")
    };
    assert_eq!(inner.status(), Status::Permanent);
}

#[test]
fn storage_layer_error_build_preserves_temporary_status() {
    let Error::StorageLayerError(inner) =
        StorageLayerError::build("msg", Unavailable, Status::Temporary)
    else {
        panic!("wrong variant")
    };
    assert_eq!(inner.status(), Status::Temporary);
}

#[test]
fn storage_layer_error_build_source_is_none() {
    let Error::StorageLayerError(inner) =
        StorageLayerError::build("msg", Conflict, Status::Permanent)
    else {
        panic!("wrong variant")
    };
    assert!(StdError::source(&inner).is_none());
}

// ── RetryableStorageLayerError ────────────────────────────────────────────────

#[test]
fn retryable_storage_layer_error_display() {
    let err = RetryableStorageLayerError {
        message: "try again".to_string(),
    };
    assert_eq!(err.to_string(), "Retryable storage layer error: try again");
}

#[test]
fn retryable_storage_layer_error_has_unavailable_category() {
    let err = RetryableStorageLayerError {
        message: "msg".to_string(),
    };
    assert_eq!(err.category(), Unavailable);
}

#[test]
fn retryable_storage_layer_error_has_temporary_status() {
    let err = RetryableStorageLayerError {
        message: "msg".to_string(),
    };
    assert_eq!(err.status(), Status::Temporary);
}

#[test]
fn retryable_storage_layer_error_source_is_none() {
    let err = RetryableStorageLayerError {
        message: "msg".to_string(),
    };
    assert!(StdError::source(&err).is_none());
}

// ── Error enum ───────────────────────────────────────────────────────────────

#[test]
fn error_storage_layer_variant_display() {
    let err = StorageLayerError::build("oops", Conflict, Status::Permanent);
    assert_eq!(err.to_string(), "Storage layer error: oops");
}

#[test]
fn error_retryable_storage_layer_variant_display() {
    let err = Error::RetryableStorageLayerError(RetryableStorageLayerError {
        message: "try again".to_string(),
    });
    assert_eq!(err.to_string(), "Retryable storage layer error: try again");
}

#[test]
fn error_storage_layer_variant_has_conflict_category() {
    let err = StorageLayerError::build("msg", Conflict, Status::Permanent);
    assert_eq!(err.category(), Conflict);
}

#[test]
fn error_storage_layer_variant_has_fault_category() {
    let err = StorageLayerError::build("msg", Fault, Status::Permanent);
    assert_eq!(err.category(), Fault);
}

#[test]
fn error_retryable_storage_layer_variant_has_unavailable_category() {
    let err = Error::RetryableStorageLayerError(RetryableStorageLayerError {
        message: "msg".to_string(),
    });
    assert_eq!(err.category(), Unavailable);
}

#[test]
fn error_storage_layer_variant_has_permanent_status() {
    let err = StorageLayerError::build("msg", Conflict, Status::Permanent);
    assert_eq!(err.status(), Status::Permanent);
}

#[test]
fn error_storage_layer_variant_has_temporary_status() {
    let err = StorageLayerError::build("msg", Unavailable, Status::Temporary);
    assert_eq!(err.status(), Status::Temporary);
}

#[test]
fn error_retryable_storage_layer_variant_has_temporary_status() {
    let err = Error::RetryableStorageLayerError(RetryableStorageLayerError {
        message: "msg".to_string(),
    });
    assert_eq!(err.status(), Status::Temporary);
}

#[test]
fn error_storage_layer_variant_source_is_none() {
    let err = StorageLayerError::build("msg", Conflict, Status::Permanent);
    assert!(StdError::source(&err).is_none());
}

#[test]
fn error_retryable_storage_layer_variant_source_is_none() {
    let err = Error::RetryableStorageLayerError(RetryableStorageLayerError {
        message: "msg".to_string(),
    });
    assert!(StdError::source(&err).is_none());
}

// ── From impls ────────────────────────────────────────────────────────────────

#[test]
fn from_storage_layer_error_produces_correct_variant() {
    let Error::StorageLayerError(inner) =
        StorageLayerError::build("x", Conflict, Status::Permanent)
    else {
        panic!("wrong variant")
    };
    let err = Error::from(inner);
    assert!(matches!(err, Error::StorageLayerError(_)));
}

#[test]
fn from_retryable_storage_layer_error_produces_correct_variant() {
    let inner = RetryableStorageLayerError {
        message: "x".to_string(),
    };
    let err = Error::from(inner);
    assert!(matches!(err, Error::RetryableStorageLayerError(_)));
}

// ── DbError ───────────────────────────────────────────────────────────────────

#[test]
fn error_implements_db_error() {
    fn assert_db_error<E: DbError>() {}
    assert_db_error::<Error>();
}

// ── From<io::Error> (cfg(test)) ───────────────────────────────────────────────

#[test]
fn error_from_io_error_display() {
    let err = Error::from(std::io::Error::other("connection refused"));
    assert_eq!(err.to_string(), "Storage layer error: connection refused");
}

// ── StorageLayerError::raise ──────────────────────────────────────────────────

#[test]
fn raise_propagates_conflict_category() {
    let db_exn: Exn<Error> = Exn::new(StorageLayerError::build(
        "low-level failure",
        Conflict,
        Status::Permanent,
    ));
    let domain_exn = StorageLayerError::raise("high-level failure", db_exn);
    assert_eq!(domain_exn.category(), Conflict);
}

#[test]
fn raise_propagates_fault_category() {
    let db_exn: Exn<Error> = Exn::new(StorageLayerError::build(
        "internal error",
        Fault,
        Status::Permanent,
    ));
    let domain_exn = StorageLayerError::raise("high-level failure", db_exn);
    assert_eq!(domain_exn.category(), Fault);
}

#[test]
fn raise_propagates_permanent_status() {
    let db_exn: Exn<Error> = Exn::new(StorageLayerError::build(
        "conflict",
        Conflict,
        Status::Permanent,
    ));
    let domain_exn = StorageLayerError::raise("could not write", db_exn);
    assert_eq!(domain_exn.status(), Status::Permanent);
}

#[test]
fn raise_propagates_temporary_status() {
    let db_exn: Exn<Error> = Exn::new(Error::RetryableStorageLayerError(
        RetryableStorageLayerError {
            message: "connection failed".to_string(),
        },
    ));
    let domain_exn = StorageLayerError::raise("could not load community", db_exn);
    assert_eq!(domain_exn.status(), Status::Temporary);
}

#[test]
fn raise_debug_shows_domain_message_wrapping_db_message() {
    let db_exn: Exn<Error> = Exn::new(Error::RetryableStorageLayerError(
        RetryableStorageLayerError {
            message: "connection failed".to_string(),
        },
    ));
    let domain_exn = StorageLayerError::raise("could not load community", db_exn);
    let debug = format!("{domain_exn:?}");
    let re = Regex::new(concat!(
        r"^Storage layer error: could not load community, at [^\n]+\n",
        r"\|\n",
        r"\|-> Retryable storage layer error: connection failed, at [^\n]+$",
    ))
    .unwrap();
    assert!(re.is_match(&debug), "debug output:\n{debug}");
}

#[test]
fn raise_display_shows_only_domain_message() {
    let db_exn: Exn<Error> = Exn::new(Error::RetryableStorageLayerError(
        RetryableStorageLayerError {
            message: "connection failed".to_string(),
        },
    ));
    let domain_exn = StorageLayerError::raise("could not load community", db_exn);
    assert_eq!(
        domain_exn.to_string(),
        "Storage layer error: could not load community"
    );
}
