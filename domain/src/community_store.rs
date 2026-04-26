use exn::Exn;

use crate::{
    community::{Community, CommunityId},
    community_repo::{CommunityProvider, CommunityRepo},
    error::{DbError, Error, StorageLayerError},
    event_log::SequenceId,
    event_log_repo::EventLogProvider,
};

/// Maximum number of effects fetched per page when advancing a community snapshot.
pub const EFFECTS_PAGE_SIZE: usize = 1000;

/// Reads and writes communities via a [`CommunityRepo`] and [`EventLogProvider`].
pub struct CommunityStore<CR: CommunityRepo, ELP: EventLogProvider> {
    community_repo: CR,
    event_log_provider: ELP,
}

impl<E, CR, ELP> CommunityStore<CR, ELP>
where
    E: DbError,
    CR: CommunityRepo + CommunityProvider<Error = E>,
    ELP: EventLogProvider<Error = E>,
{
    /// Creates a new `CommunityStore` backed by `community_repo` and `event_log_provider`.
    pub fn new(community_repo: CR, event_log_provider: ELP) -> Self {
        Self {
            community_repo,
            event_log_provider,
        }
    }

    /// Creates and persists a new community at version zero.
    pub fn init(&self) -> Result<Community, Exn<Error>> {
        self.community_repo
            .put(Community::new())
            .map_err(|e| StorageLayerError::raise("failed to initialize community", e))
    }

    /// Returns the community snapshot at the given `version`, or `None` if not found.
    pub fn get(
        &self,
        id: CommunityId,
        version: SequenceId,
    ) -> Result<Option<Community>, Exn<Error>> {
        self.community_repo
            .get(id, version)
            .map_err(|e| StorageLayerError::raise("failed to retrieve community snapshot", e))
    }

    /// Returns the community at `id` with all unapplied effects folded in, or `None`
    /// if the community has never been persisted.
    ///
    /// Any effects recorded after the latest stored snapshot are applied in sequence.
    /// If new effects were applied, the resulting snapshot is saved before being returned.
    pub fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        let mut community = match self.community_repo.get_latest(id).map_err(|e| {
            StorageLayerError::raise("failed to retrieve latest version of community", e)
        })? {
            Some(c) => c,
            None => return Ok(None),
        };
        let initial_version = community.version;
        let mut done = false;
        let i = 0;
        while !done {
            let i = i + 1;
            let prev_version = community.version;
            let batch = self
                .event_log_provider
                .get_effects_after(id, EFFECTS_PAGE_SIZE, prev_version)
                .map_err(|e| {
                    StorageLayerError::raise(
                        format!("failed to retrieve effects for community at batch number {i}"),
                        e,
                    )
                })?;
            done = batch.len() < EFFECTS_PAGE_SIZE;
            community.apply_effects(batch);
            if community.version == prev_version {
                done = true;
            }
        }
        if community.version == initial_version {
            return Ok(Some(community));
        }
        let saved = self
            .community_repo
            .put(community)
            .map_err(|e| StorageLayerError::raise("failed to persist updated community", e))?;
        Ok(Some(saved))
    }
}

#[cfg(test)]
#[path = "community_store_tests.rs"]
mod tests;
