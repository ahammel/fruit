use std::{collections::HashMap, sync::RwLock};

use gib_fruit_domain::{
    community::{Community, CommunityId},
    error::Error,
    repo::{CommunityPersistor, CommunityProvider, CommunityRepo},
};

/// In-memory implementation of [`CommunityRepo`], backed by a [`HashMap`].
///
/// Reads and writes are protected by a [`RwLock`], allowing concurrent reads
/// and exclusive writes without requiring `&mut self`.
pub struct InMemoryCommunityRepo {
    store: RwLock<HashMap<CommunityId, Community>>,
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
    fn get(&self, id: CommunityId) -> Result<Option<Community>, Error> {
        Ok(self.store.read()?.get(&id).cloned())
    }
}

impl CommunityPersistor for InMemoryCommunityRepo {
    fn put(&self, community: Community) -> Result<Community, Error> {
        self.store.write()?.insert(community.id, community.clone());
        Ok(community)
    }
}

impl CommunityRepo for InMemoryCommunityRepo {}

#[cfg(test)]
mod tests {
    use super::*;
    use gib_fruit_domain::{id::UuidIdentifier, store::CommunityStore};

    fn repo() -> InMemoryCommunityRepo {
        InMemoryCommunityRepo::new()
    }

    fn store() -> CommunityStore<InMemoryCommunityRepo> {
        CommunityStore::new(InMemoryCommunityRepo::new())
    }

    // --- helpers ---

    /// Creates a `RwLock<HashMap>` that has been poisoned by a panicking writer thread.
    /// Used to exercise the `?` error branches in `get` and `put`.
    fn poisoned_store() -> RwLock<HashMap<CommunityId, Community>> {
        use std::sync::Arc;
        let lock = Arc::new(RwLock::new(HashMap::new()));
        let l = Arc::clone(&lock);
        std::thread::spawn(move || {
            let _guard = l.write().unwrap();
            panic!("intentional poison");
        })
        .join()
        .ok();
        Arc::try_unwrap(lock).unwrap()
    }

    // --- default ---

    #[test]
    fn default_produces_empty_repo() {
        assert!(InMemoryCommunityRepo::default()
            .get(CommunityId::new())
            .unwrap()
            .is_none());
    }

    // --- poisoned lock error paths ---

    #[test]
    fn get_returns_err_when_lock_is_poisoned() {
        let repo = InMemoryCommunityRepo {
            store: poisoned_store(),
        };
        assert!(repo.get(CommunityId::new()).is_err());
    }

    #[test]
    fn put_returns_err_when_lock_is_poisoned() {
        let repo = InMemoryCommunityRepo {
            store: poisoned_store(),
        };
        assert!(repo.put(Community::new()).is_err());
    }

    // --- repo tests ---

    #[test]
    fn repo_get_returns_none_for_unknown_id() {
        assert!(repo().get(CommunityId::new()).unwrap().is_none());
    }

    #[test]
    fn repo_put_and_get_round_trips_community() {
        let repo = repo();
        let community = Community::new();
        let id = community.id;
        repo.put(community.clone()).unwrap();
        assert_eq!(repo.get(id).unwrap(), Some(community));
    }

    #[test]
    fn repo_put_overwrites_existing_community() {
        let repo = repo();
        let community = Community::new();
        let id = community.id;
        repo.put(community).unwrap();
        let updated = Community::new().with_id(id).with_luck(0.5);
        repo.put(updated.clone()).unwrap();
        assert_eq!(repo.get(id).unwrap(), Some(updated));
    }

    // --- store tests ---

    #[test]
    fn store_get_returns_none_for_unknown_id() {
        assert!(store().get(CommunityId::new()).unwrap().is_none());
    }

    #[test]
    fn store_put_and_get_round_trips_community() {
        let store = store();
        let community = Community::new();
        let id = community.id;
        store.put(community.clone()).unwrap();
        assert_eq!(store.get(id).unwrap(), Some(community));
    }

    #[test]
    fn store_put_overwrites_existing_community() {
        let store = store();
        let community = Community::new();
        let id = community.id;
        store.put(community).unwrap();
        let updated = Community::new().with_id(id).with_luck(0.5);
        store.put(updated.clone()).unwrap();
        assert_eq!(store.get(id).unwrap(), Some(updated));
    }
}
