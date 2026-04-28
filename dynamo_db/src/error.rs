use anomalies::anomaly::Anomaly;
use fruit_domain::{community::CommunityId, error::DbError, event_log::SequenceId};
use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;
use thiserror::Error;

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

    /// An AWS SDK or network error.
    #[error("DynamoDB error: {message}")]
    #[category(unavailable)]
    Sdk { message: String },

    /// A stored item could not be decoded into a domain type.
    #[error("DynamoDB codec error: {message}")]
    #[category(fault)]
    Codec { message: String },
}

impl DbError for Error {}

/// Entity kinds stored in DynamoDB.
#[derive(Debug)]
pub enum Entity {
    Community,
    Event,
    Effect,
}

/// Builds an [`Error::Sdk`] from any displayable value.
pub(crate) fn sdk_err(context: &str, e: impl std::fmt::Display) -> Error {
    Error::Sdk { message: format!("{context}: {e}") }
}

/// Builds an [`Error::Codec`] from any displayable value.
pub(crate) fn codec_err(context: &str, e: impl std::fmt::Display) -> Error {
    Error::Codec { message: format!("{context}: {e}") }
}
