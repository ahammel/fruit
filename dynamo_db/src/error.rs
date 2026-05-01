use std::fmt::{self, Display};

use anomalies::{
    anomaly::{Anomaly, HasCategory, HasStatus},
    category::{Category, Fault, Incorrect},
    status::Status,
};
use aws_sdk_dynamodb::error::{ProvideErrorMetadata, SdkError};
use exn::{ErrorExt, Exn};
use fruit_domain::{community::CommunityId, error::DbError, event_log::SequenceId};
use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;
use thiserror::Error;

use crate::dyanmo_sdk_anomaly::{ConstructionFailure, SdkAnomaly};

/// Errors that can occur when accessing DynamoDB domain storage.
#[derive(Debug, Error, Anomaly)]
pub enum Error {
    /// A conditional write failed because the item already exists.
    #[error(
        "could not write {entity:?} at version {} in community {} because it already exists",
        .version.as_u64(),
        .community.as_uuid()
    )]
    #[category(conflict)]
    AlreadyExists {
        community: CommunityId,
        version: SequenceId,
        entity: Entity,
    },

    /// An AWS SDK or network error, classified for retry decisions.
    #[error(transparent)]
    #[anomaly(transparent)]
    Sdk(DynamoSdkError),

    /// A stored item could not be decoded into a domain type.
    #[error("DynamoDB codec error: {message}")]
    #[category(fault)]
    Codec { message: String },
}

impl DbError for Error {}

/// The entity kind involved in a storage conflict.
#[derive(Debug)]
pub enum Entity {
    /// A community snapshot.
    Community,
    /// An event item.
    Event,
    /// An effect item.
    Effect,
}

/// A classified AWS SDK error carrying retry metadata.
///
/// Wraps the underlying [`SdkAnomaly`] and attaches a human-readable context
/// string. The [`HasCategory`] and [`HasStatus`] implementations let callers
/// decide whether to retry without parsing the message string.
#[derive(Debug)]
pub struct DynamoSdkError {
    context: String,
    cause: SdkAnomaly,
    category: Category,
    status: Status,
}

impl Display for DynamoSdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.context)
    }
}

impl std::error::Error for DynamoSdkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

impl HasCategory for DynamoSdkError {
    fn category(&self) -> Category {
        self.category
    }
}

impl HasStatus for DynamoSdkError {
    fn status(&self) -> Status {
        self.status
    }
}

impl Anomaly for DynamoSdkError {}

/// Classifies and wraps an AWS SDK operation error.
pub(crate) fn raise_sdk_err<E>(context: &str, err: SdkError<E>) -> Exn<Error>
where
    E: ProvideErrorMetadata + std::error::Error + Send + Sync + 'static,
{
    let sdk_anomaly: SdkAnomaly = err.into();
    let (category, status) = if sdk_anomaly.category() == Incorrect {
        // ConstructionFailure means a bug in our call assembly, not a service problem.
        (Fault, Status::Permanent)
    } else {
        (sdk_anomaly.category(), sdk_anomaly.status())
    };
    let sdk_err = DynamoSdkError {
        context: context.into(),
        cause: sdk_anomaly,
        category,
        status,
    };
    Error::Sdk(sdk_err).raise()
}

/// Wraps a request-builder error as a permanent construction fault.
pub(crate) fn raise_build_err(
    context: impl Into<String>,
    e: impl std::error::Error + Send + Sync + 'static,
) -> Exn<Error> {
    let sdk_err = DynamoSdkError {
        context: context.into(),
        cause: SdkAnomaly::ConstructionFailure(ConstructionFailure::new(Box::new(e))),
        category: Fault,
        status: Status::Permanent,
    };
    Error::Sdk(sdk_err).raise()
}

/// Wraps a conditional-check SDK error as an [`Error::AlreadyExists`], with the original
/// SDK error in the causality chain.
pub(crate) fn raise_conflict_err<E>(
    community: CommunityId,
    version: SequenceId,
    entity: Entity,
    err: SdkError<E>,
) -> Exn<Error>
where
    E: ProvideErrorMetadata + std::error::Error + Send + Sync + 'static,
{
    let sdk_anomaly: SdkAnomaly = err.into();
    Exn::new(sdk_anomaly).raise(Error::AlreadyExists {
        community,
        version,
        entity,
    })
}

/// Builds an [`Error::Codec`] from any displayable value.
pub(crate) fn raise_codec_err<E>(context: impl Into<String>, e: E) -> Exn<Error>
where
    E: std::error::Error + Send + Sync + 'static,
{
    Exn::new(e).raise(Error::Codec {
        message: context.into(),
    })
}
