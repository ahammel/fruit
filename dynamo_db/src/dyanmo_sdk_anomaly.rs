use std::fmt;

use anomalies::{
    anomaly::{Anomaly, HasCategory, HasStatus},
    category::{Busy, Category, Fault, Incorrect, Interrupted, Unavailable},
    status::Status,
};
use aws_sdk_dynamodb::error::{ProvideErrorMetadata, SdkError};
use thiserror::Error;

// ── SdkAnomaly ─────────────────────────────────────────────────────────────────

/// A structured, classified wrapper around [`SdkError`] that implements [`Anomaly`].
///
/// Each variant mirrors the corresponding [`SdkError`] variant and carries the
/// appropriate [`category`](anomalies::category) and retry [`Status`], derived
/// from the error structure and (for service errors) the error code string.
#[non_exhaustive]
#[derive(Debug, Error, Anomaly)]
pub enum SdkAnomaly {
    /// The request failed during construction. It was not dispatched over the network.
    #[error(transparent)]
    #[anomaly(transparent)]
    ConstructionFailure(ConstructionFailure),

    /// The request failed due to a timeout. The request MAY have been sent and received.
    #[error(transparent)]
    #[anomaly(transparent)]
    TimeoutError(TimeoutError),

    /// The request failed during dispatch. An HTTP response was not received. The request MAY
    /// have been sent.
    #[error(transparent)]
    #[anomaly(transparent)]
    DispatchFailure(DispatchFailure),

    /// A response was received but it was not parseable according to the protocol (for example
    /// the server hung up without sending a complete response).
    #[error(transparent)]
    #[anomaly(transparent)]
    ResponseError(ResponseError),

    /// An error response was received from the service.
    #[error(transparent)]
    #[anomaly(transparent)]
    ServiceError(ServiceError),
}

// ── From<SdkError<E, R>> ───────────────────────────────────────────────────────

impl<E, R> From<SdkError<E, R>> for SdkAnomaly
where
    E: ProvideErrorMetadata + std::error::Error + Send + Sync + 'static,
    R: fmt::Debug + Send + Sync + 'static,
{
    fn from(e: SdkError<E, R>) -> Self {
        match e {
            SdkError::ConstructionFailure(inner) => Self::ConstructionFailure(ConstructionFailure(
                Box::new(SdkError::<E, R>::ConstructionFailure(inner)),
            )),
            SdkError::TimeoutError(inner) => Self::TimeoutError(TimeoutError(Box::new(
                SdkError::<E, R>::TimeoutError(inner),
            ))),
            SdkError::DispatchFailure(d) => Self::DispatchFailure(DispatchFailure::new::<E, R>(d)),
            SdkError::ResponseError(inner) => Self::ResponseError(ResponseError(Box::new(
                SdkError::<E, R>::ResponseError(inner),
            ))),
            SdkError::ServiceError(se) => Self::ServiceError(ServiceError::new::<E, R>(se)),

            // SdkError is non-exhaustive; treat unknown variants as construction faults.
            e => Self::ConstructionFailure(ConstructionFailure(Box::new(e))),
        }
    }
}
// ── Inner wrapper types ────────────────────────────────────────────────────────

/// The request failed during construction; it was never sent to the service.
///
/// This is always a permanent fault: construction failures indicate a bug in how
/// the SDK call was assembled.
#[derive(Debug, Anomaly)]
#[category(incorrect)]
pub struct ConstructionFailure(Box<dyn std::error::Error + Send + Sync + 'static>);

impl fmt::Display for ConstructionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for ConstructionFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// The request timed out before a response was received.
///
/// This is always interrupted/temporary: the same request may succeed if retried.
#[derive(Debug, Anomaly)]
#[category(interrupted)]
#[status(temporary)]
pub struct TimeoutError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for TimeoutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// The request failed during dispatch before an HTTP response was received.
///
/// Category and status vary by dispatch kind:
/// - `is_timeout()` → interrupted / temporary
/// - `is_io()` or `is_other()` → unavailable / temporary
/// - `is_user()` → fault / permanent (caller configuration error)
#[derive(Debug)]
pub struct DispatchFailure {
    category: Category,
    status: Status,
    err: Box<dyn std::error::Error + Send + Sync + 'static>,
}
impl DispatchFailure {
    fn new<E, R>(d: aws_smithy_runtime_api::client::result::DispatchFailure) -> Self
    where
        E: ProvideErrorMetadata + std::error::Error + Send + Sync + 'static,
        R: fmt::Debug + Send + Sync + 'static,
    {
        let (category, status) = if d.is_timeout() {
            (Interrupted, Status::Temporary)
        } else if d.is_user() {
            (Incorrect, Status::Permanent)
        } else if d.is_io() {
            (Unavailable, Status::Temporary)
        } else {
            (Fault, Status::Permanent)
        };
        Self {
            category,
            status,
            err: Box::new(SdkError::<E, R>::DispatchFailure(d)),
        }
    }
}

impl fmt::Display for DispatchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.err.fmt(f)
    }
}
impl std::error::Error for DispatchFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.err.source()
    }
}
impl HasCategory for DispatchFailure {
    fn category(&self) -> Category {
        self.category
    }
}
impl HasStatus for DispatchFailure {
    fn status(&self) -> Status {
        self.status
    }
}
impl Anomaly for DispatchFailure {}

/// An HTTP response was received but could not be parsed.
///
/// This is always a permanent fault: an unparseable response indicates a protocol
/// mismatch or a bug.
#[derive(Debug, Anomaly)]
#[category(fault)]
pub struct ResponseError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for ResponseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// The service returned a typed error response.
///
/// Category and status are determined by the error code:
/// - Throttling codes → busy / temporary
/// - Internal server error / service unavailable → unavailable / temporary
/// - Everything else → fault / permanent
#[derive(Debug)]
pub struct ServiceError {
    category: Category,
    status: Status,
    err: Box<dyn std::error::Error + Send + Sync + 'static>,
}

impl ServiceError {
    fn new<E, R>(se: aws_smithy_runtime_api::client::result::ServiceError<E, R>) -> Self
    where
        E: ProvideErrorMetadata + std::error::Error + Send + Sync + 'static,
        R: fmt::Debug + Send + Sync + 'static,
    {
        let (category, status) = match se.err().code() {
            Some(
                "ProvisionedThroughputExceededException"
                | "RequestLimitExceeded"
                | "ThrottlingException",
            ) => (Busy, Status::Temporary),
            Some("InternalServerError" | "ServiceUnavailable") => (Unavailable, Status::Temporary),
            _ => (Fault, Status::Permanent),
        };
        Self {
            category,
            status,
            err: Box::new(SdkError::<E, R>::ServiceError(se)),
        }
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.err, f)
    }
}
impl std::error::Error for ServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.err)
    }
}
impl HasCategory for ServiceError {
    fn category(&self) -> Category {
        self.category
    }
}
impl HasStatus for ServiceError {
    fn status(&self) -> Status {
        self.status
    }
}
impl Anomaly for ServiceError {}

#[cfg(test)]
#[path = "dyanmo_sdk_anomaly_tests.rs"]
mod tests;
