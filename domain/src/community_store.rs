use crate::{
    community::{Community, CommunityId},
    community_repo::CommunityRepo,
    error::Error,
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

impl<CR: CommunityRepo, ELP: EventLogProvider> CommunityStore<CR, ELP> {
    /// Creates a new `CommunityStore` backed by `community_repo` and `event_log_provider`.
    pub fn new(community_repo: CR, event_log_provider: ELP) -> Self {
        Self {
            community_repo,
            event_log_provider,
        }
    }

    /// Creates and persists a new community at version zero.
    pub fn init(&self) -> Result<Community, Error> {
        self.community_repo.put(Community::new())
    }

    /// Returns the community snapshot at the given `version`, or `None` if not found.
    pub fn get(&self, id: CommunityId, version: SequenceId) -> Result<Option<Community>, Error> {
        self.community_repo.get(id, version)
    }

    /// Returns the community at `id` with all unapplied effects folded in, or `None`
    /// if the community has never been persisted.
    ///
    /// Any effects recorded after the latest stored snapshot are applied in sequence.
    /// If new effects were applied, the resulting snapshot is saved before being returned.
    pub fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Error> {
        let Some(mut community) = self.community_repo.get_latest(id)? else {
            return Ok(None);
        };
        let initial_version = community.version;
        loop {
            let batch = self.event_log_provider.get_effects_after(
                id,
                EFFECTS_PAGE_SIZE,
                community.version,
            )?;
            let done = batch.len() < EFFECTS_PAGE_SIZE;
            community.apply_effects(batch);
            if done {
                break;
            }
        }
        if community.version == initial_version {
            return Ok(Some(community));
        }
        let saved = self.community_repo.put(community)?;
        Ok(Some(saved))
    }
}

#[cfg(test)]
#[path = "community_store_tests.rs"]
mod tests;
