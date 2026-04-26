use std::{fmt, sync::PoisonError};

use anomalies::{
    anomaly::{Anomaly, HasCategory, HasStatus},
    category::Category,
    status::Status,
};
use fruit_domain::{
    community::{Community, CommunityId},
    error::DbError,
    event_log::SequenceId,
};
use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;

#[derive(Debug)]
pub(crate) enum Entity {
    Community,
    Event,
    Effect,
}

/// Returned when trying to write a record that already exists at the given ID or version.
#[derive(Anomaly, Debug)]
#[category(conflict)]
pub struct AlreadyExists {
    community: CommunityId,
    version: SequenceId,
    entity: Entity,
}

impl AlreadyExists {
    pub(crate) fn community(community: &Community) -> Error {
        Error::AlreadyExists(AlreadyExists {
            community: community.id,
            version: community.version,
            entity: Entity::Community,
        })
    }

    pub(crate) fn event(community: CommunityId, version: SequenceId) -> Error {
        Error::AlreadyExists(AlreadyExists {
            community,
            version,
            entity: Entity::Event,
        })
    }

    pub(crate) fn effect(community: CommunityId, version: SequenceId) -> Error {
        Error::AlreadyExists(AlreadyExists {
            community,
            version,
            entity: Entity::Effect,
        })
    }
}

impl fmt::Display for AlreadyExists {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "could not write {:?} at version {} in community {} because it already exists",
            self.entity,
            self.version.as_u64(),
            self.community.as_uuid(),
        )
    }
}

impl std::error::Error for AlreadyExists {}

#[derive(Debug)]
pub(crate) enum Lock {
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
#[derive(Anomaly, Debug)]
#[category(interrupted)]
pub struct LockPoisoned {
    message: String,
    lock: Lock,
}

impl LockPoisoned {
    pub(crate) fn build<T>(e: &PoisonError<T>, lock: Lock) -> Error {
        Error::LockPoisoned(LockPoisoned {
            message: e.to_string(),
            lock,
        })
    }
}

impl HasStatus for LockPoisoned {
    fn status(&self) -> Status {
        Status::Temporary
    }
}

impl fmt::Display for LockPoisoned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} was poisoned: {}", self.lock, self.message)
    }
}

impl std::error::Error for LockPoisoned {}

/// Errors that can occur when accessing in-memory domain storage.
#[derive(Debug)]
pub enum Error {
    /// A write was attempted for a record that already exists.
    AlreadyExists(AlreadyExists),
    /// A lock was poisoned because a thread panicked while holding it.
    LockPoisoned(LockPoisoned),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AlreadyExists(e) => e.fmt(f),
            Error::LockPoisoned(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl HasCategory for Error {
    fn category(&self) -> Category {
        match self {
            Error::AlreadyExists(e) => e.category(),
            Error::LockPoisoned(e) => e.category(),
        }
    }
}

impl HasStatus for Error {
    fn status(&self) -> Status {
        match self {
            Error::AlreadyExists(e) => e.status(),
            Error::LockPoisoned(e) => e.status(),
        }
    }
}

impl Anomaly for Error {}

impl DbError for Error {}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
