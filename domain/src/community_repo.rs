use exn::Exn;

use crate::{
    community::{Community, CommunityId},
    error::DbError,
    event_log::SequenceId,
};

/// Port for loading a [`Community`] from storage.
pub trait CommunityProvider {
    /// The error type returned by storage operations.
    type Error: DbError;

    /// Returns the community snapshot at the given `version`, or `None` if not found.
    fn get(
        &self,
        id: CommunityId,
        version: SequenceId,
    ) -> Result<Option<Community>, Exn<Self::Error>>;

    /// Returns the most recently stored community snapshot for `id`, or `None` if the
    /// community has never been persisted.
    fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Exn<Self::Error>>;
}

/// Port for persisting a [`Community`] to storage.
///
/// `&self` is used rather than `&mut self` because concurrent writes are
/// permitted; implementations are expected to manage shared state internally
/// (e.g. via a connection pool or mutex).
pub trait CommunityPersistor {
    /// The error type returned by storage operations.
    type Error: DbError;

    /// Writes `community` as a new snapshot version.
    ///
    /// Returns `Err` if a snapshot at `community.version` already exists for this
    /// community.
    fn put(&self, community: Community) -> Result<Community, Exn<Self::Error>>;
}

/// Combined read/write port. Implement this when CQRS separation is not needed.
pub trait CommunityRepo:
    CommunityProvider + CommunityPersistor<Error = <Self as CommunityProvider>::Error>
{
}
