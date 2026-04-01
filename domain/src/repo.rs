use crate::{
    community::{Community, CommunityId},
    error::Error,
};

/// Port for loading a [`Community`] from storage.
pub trait CommunityProvider {
    /// Returns the community with the given `id`, or `None` if it does not exist.
    fn get(&self, id: CommunityId) -> Result<Option<Community>, Error>;
}

/// Port for persisting a [`Community`] to storage.
///
/// `&self` is used rather than `&mut self` because concurrent writes are
/// permitted; implementations are expected to manage shared state internally
/// (e.g. via a connection pool or mutex).
pub trait CommunityPersistor {
    /// Writes `community` to storage and returns the saved value.
    fn put(&self, community: Community) -> Result<Community, Error>;
}

/// Combined read/write port. Implement this when CQRS separation is not needed.
pub trait CommunityRepo: CommunityProvider + CommunityPersistor {}
