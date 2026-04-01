use crate::{
    community::{Community, CommunityId},
    error::Error,
    repo::CommunityRepo,
};

/// Reads and writes communities via a [`CommunityRepo`].
pub struct CommunityStore<R: CommunityRepo> {
    repo: R,
}

impl<R: CommunityRepo> CommunityStore<R> {
    /// Creates a new `CommunityStore` backed by `repo`.
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Returns the community with the given `id`, or `None` if it does not exist.
    pub fn get(&self, id: CommunityId) -> Result<Option<Community>, Error> {
        self.repo.get(id)
    }

    /// Writes `community` to storage and returns the saved value.
    pub fn put(&self, community: Community) -> Result<Community, Error> {
        self.repo.put(community)
    }
}
