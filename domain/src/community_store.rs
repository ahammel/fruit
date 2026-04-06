use crate::{
    community::{Community, CommunityId},
    community_repo::CommunityRepo,
    error::Error,
    event_log::Effect,
    event_log::Record,
    event_log::SequenceId,
    event_log_repo::EventLogProvider,
};

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
        self.put(Community::new())
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
        let unapplied = self
            .event_log_provider
            .get_effects_after(id, community.version)?;
        if unapplied.is_empty() {
            return Ok(Some(community));
        }
        community.apply_effects(unapplied);
        let saved = self.put(community)?;
        Ok(Some(saved))
    }

    /// Writes `community` as a new snapshot. Returns `Err` if that version already exists.
    pub fn put(&self, community: Community) -> Result<Community, Error> {
        self.community_repo.put(community)
    }

    /// Returns the log entry with the given sequence ID, or `None` if not found.
    pub fn get_record(&self, id: SequenceId) -> Result<Option<Record>, Error> {
        self.event_log_provider.get_record(id)
    }

    /// Returns the effect whose `event_id` matches `event_id`, or `None` if not yet processed.
    pub fn get_effect_for_event(&self, event_id: SequenceId) -> Result<Option<Effect>, Error> {
        self.event_log_provider.get_effect_for_event(event_id)
    }

    /// Returns the `n` most recent events for `community_id`, sorted by sequence ID descending.
    pub fn get_latest_records(&self, id: CommunityId, n: usize) -> Result<Vec<Record>, Error> {
        self.event_log_provider.get_latest_records(id, n)
    }
}

#[cfg(test)]
#[path = "community_store_tests.rs"]
mod tests;
