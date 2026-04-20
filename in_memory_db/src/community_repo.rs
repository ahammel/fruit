use std::{
    collections::{BTreeMap, HashMap},
    io,
    sync::RwLock,
};

use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;

use fruit_domain::{
    community::{Community, CommunityId},
    community_repo::{CommunityPersistor, CommunityProvider, CommunityRepo},
    error::Error,
    event_log::SequenceId,
};

/// In-memory implementation of [`CommunityRepo`].
///
/// Communities are stored as versioned snapshots: each call to [`put`] records
/// the community at its current [`SequenceId`] version. Reads and writes are
/// protected by a [`RwLock`], allowing concurrent reads and exclusive writes
/// without `&mut self`.
///
/// [`put`]: CommunityPersistor::put
#[derive(Debug)]
pub struct InMemoryCommunityRepo {
    store: RwLock<HashMap<CommunityId, BTreeMap<SequenceId, Community>>>,
}

impl InMemoryCommunityRepo {
    /// Creates a new empty `InMemoryCommunityRepo`.
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCommunityRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl CommunityProvider for InMemoryCommunityRepo {
    fn get(&self, id: CommunityId, version: SequenceId) -> Result<Option<Community>, Error> {
        Ok(self
            .store
            .read()?
            .get(&id)
            .and_then(|versions| versions.get(&version))
            .cloned())
    }

    fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Error> {
        Ok(self
            .store
            .read()?
            .get(&id)
            .and_then(|versions| versions.values().next_back())
            .cloned())
    }
}

impl CommunityPersistor for InMemoryCommunityRepo {
    fn put(&self, community: Community) -> Result<Community, Error> {
        let mut store = self.store.write()?;
        let versions = store.entry(community.id).or_default();
        if versions.contains_key(&community.version) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "community {} already has a snapshot at version {}",
                    community.id.as_uuid(),
                    community.version.as_u64(),
                ),
            )
            .into());
        }
        versions.insert(community.version, community.clone());
        Ok(community)
    }
}

impl CommunityRepo for InMemoryCommunityRepo {}

impl CommunityProvider for &InMemoryCommunityRepo {
    fn get(&self, id: CommunityId, version: SequenceId) -> Result<Option<Community>, Error> {
        (*self).get(id, version)
    }

    fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Error> {
        (*self).get_latest(id)
    }
}

impl CommunityPersistor for &InMemoryCommunityRepo {
    fn put(&self, community: Community) -> Result<Community, Error> {
        (*self).put(community)
    }
}

impl CommunityRepo for &InMemoryCommunityRepo {}

#[cfg(test)]
#[path = "community_repo_tests.rs"]
mod tests;
