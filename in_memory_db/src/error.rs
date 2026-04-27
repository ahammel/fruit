use std::{fmt, sync::PoisonError};

use anomalies::anomaly::Anomaly;
use fruit_domain::{
    community::{Community, CommunityId},
    error::DbError,
    event_log::SequenceId,
};
use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;
use thiserror::Error;

/// Errors that can occur when accessing in-memory domain storage.
#[derive(Debug, Error, Anomaly)]
pub enum Error {
    /// A write was attempted for a record that already exists.
    #[error("could not write {:?} at version {} in community {} because it already exists",
        .entity, .version.as_u64(), .community.as_uuid())]
    #[category(conflict)]
    AlreadyExists {
        community: CommunityId,
        version: SequenceId,
        entity: Entity,
    },
    /// A lock was poisoned because a thread panicked while holding it.
    #[error("{lock} was poisoned: {message}")]
    #[category(interrupted)]
    #[status(temporary)]
    LockPoisoned { message: String, lock: Lock },
}

impl DbError for Error {}

// Entities stored in the database
#[derive(Debug)]
pub enum Entity {
    Community,
    Event,
    Effect,
}

// Unit struct use for implementation of build fucntions
pub struct AlreadyExists {}

impl AlreadyExists {
    pub fn community(community: &Community) -> Error {
        Error::AlreadyExists {
            community: community.id,
            version: community.version,
            entity: Entity::Community,
        }
    }

    pub fn event(community: CommunityId, version: SequenceId) -> Error {
        Error::AlreadyExists {
            community,
            version,
            entity: Entity::Event,
        }
    }

    pub fn effect(community: CommunityId, version: SequenceId) -> Error {
        Error::AlreadyExists {
            community,
            version,
            entity: Entity::Effect,
        }
    }
}

#[derive(Debug)]
pub enum Lock {
    CommunityRead,
    CommunityWrite,
    EventLogRead,
    EventLogWrite,
    EffectLogRead,
    EffectLogWrite,
}

impl fmt::Display for Lock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Lock::CommunityRead => "community read lock",
                Lock::CommunityWrite => "community write lock",
                Lock::EventLogRead => "event log read lock",
                Lock::EventLogWrite => "event log write lock",
                Lock::EffectLogRead => "effect log read lock",
                Lock::EffectLogWrite => "effect log write lock",
            }
        )
    }
}

/// A lock was poisoned because a thread panicked while holding it.
///
/// `PoisonError<T>` carries a lock guard with a non-`'static` lifetime, so the
/// message is extracted as a `String` at the call site. As a result,
/// `std::error::Error::source` returns `None` for this variant.
pub struct LockPoisoned;

impl LockPoisoned {
    pub fn build<T>(e: &PoisonError<T>, lock: Lock) -> Error {
        Error::LockPoisoned {
            message: e.to_string(),
            lock,
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
