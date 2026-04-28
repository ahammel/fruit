use std::{
    collections::{BTreeMap, HashMap},
    sync::RwLock,
};

use async_trait::async_trait;
use exn::Exn;

use fruit_domain::{
    community::{Community, CommunityId},
    community_repo::{CommunityPersistor, CommunityProvider, CommunityRepo},
    event_log::SequenceId,
};

use crate::error::{AlreadyExists, Error, Lock, LockPoisoned};

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

#[async_trait]
impl CommunityProvider for InMemoryCommunityRepo {
    type Error = Error;

    async fn get(
        &self,
        id: CommunityId,
        version: SequenceId,
    ) -> Result<Option<Community>, Exn<Error>> {
        Ok(self
            .store
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::CommunityRead))?
            .get(&id)
            .and_then(|versions| versions.get(&version))
            .cloned())
    }

    async fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        Ok(self
            .store
            .read()
            .map_err(|e| LockPoisoned::build(&e, Lock::CommunityRead))?
            .get(&id)
            .and_then(|versions| versions.values().next_back())
            .cloned())
    }
}

#[async_trait]
impl CommunityPersistor for InMemoryCommunityRepo {
    type Error = Error;

    async fn put(&self, community: Community) -> Result<Community, Exn<Error>> {
        let mut store = self
            .store
            .write()
            .map_err(|e| LockPoisoned::build(&e, Lock::CommunityWrite))?;
        let versions = store.entry(community.id).or_default();
        if versions.contains_key(&community.version) {
            return Err(AlreadyExists::community(&community).into());
        }
        versions.insert(community.version, community.clone());
        Ok(community)
    }
}

impl CommunityRepo for InMemoryCommunityRepo {}

#[async_trait]
impl CommunityProvider for &InMemoryCommunityRepo {
    type Error = Error;

    async fn get(
        &self,
        id: CommunityId,
        version: SequenceId,
    ) -> Result<Option<Community>, Exn<Error>> {
        (*self).get(id, version).await
    }

    async fn get_latest(&self, id: CommunityId) -> Result<Option<Community>, Exn<Error>> {
        (*self).get_latest(id).await
    }
}

#[async_trait]
impl CommunityPersistor for &InMemoryCommunityRepo {
    type Error = Error;

    async fn put(&self, community: Community) -> Result<Community, Exn<Error>> {
        (*self).put(community).await
    }
}

impl CommunityRepo for &InMemoryCommunityRepo {}

#[cfg(test)]
#[path = "community_repo_tests.rs"]
mod tests;
