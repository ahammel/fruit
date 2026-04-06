use std::{
    collections::{BTreeMap, HashMap},
    io,
    sync::RwLock,
};

use fruit_domain::{
    community::{Community, CommunityId},
    community_repo::{CommunityPersistor, CommunityProvider, CommunityRepo},
    error::Error,
    event_log::SequenceId,
    id::{IntegerIdentifier, UuidIdentifier},
};

/// In-memory implementation of [`CommunityRepo`].
///
/// Communities are stored as versioned snapshots: each call to [`put`] records
/// the community at its current [`SequenceId`] version. Reads and writes are
/// protected by a [`RwLock`], allowing concurrent reads and exclusive writes
/// without `&mut self`.
///
/// [`put`]: CommunityPersistor::put
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log_repo::InMemoryEventLogRepo;
    use fruit_domain::{community_store::CommunityStore, id::UuidIdentifier};

    fn repo() -> InMemoryCommunityRepo {
        InMemoryCommunityRepo::new()
    }

    fn store() -> CommunityStore<InMemoryCommunityRepo, InMemoryEventLogRepo> {
        CommunityStore::new(InMemoryCommunityRepo::new(), InMemoryEventLogRepo::new())
    }

    /// Forces monomorphization of `EventLogPersistor::append_event` for type `P`,
    /// preventing LLVM from inlining delegation wrappers (e.g. `&InMemoryEventLogRepo`).
    fn append_event_via<P: fruit_domain::event_log_repo::EventLogPersistor>(
        p: P,
        community_id: CommunityId,
        count: usize,
    ) -> fruit_domain::event_log::Event {
        p.append_event(
            community_id,
            fruit_domain::event_log::EventPayload::Grant { count },
        )
        .unwrap()
    }

    /// Forces monomorphization of `EventLogPersistor::append_effect` for type `P`.
    fn append_effect_via<P: fruit_domain::event_log_repo::EventLogPersistor>(
        p: P,
        event_id: fruit_domain::event_log::SequenceId,
        community_id: CommunityId,
        mutations: Vec<fruit_domain::event_log::StateMutation>,
    ) -> fruit_domain::event_log::Effect {
        p.append_effect(event_id, community_id, mutations).unwrap()
    }

    // --- helpers ---

    fn poisoned_store() -> RwLock<HashMap<CommunityId, BTreeMap<SequenceId, Community>>> {
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
            .get(CommunityId::new(), SequenceId::zero())
            .unwrap()
            .is_none());
    }

    // --- poisoned lock error paths ---

    #[test]
    fn get_returns_err_when_lock_is_poisoned() {
        let repo = InMemoryCommunityRepo {
            store: poisoned_store(),
        };
        assert!(repo.get(CommunityId::new(), SequenceId::zero()).is_err());
    }

    #[test]
    fn get_latest_returns_err_when_lock_is_poisoned() {
        let repo = InMemoryCommunityRepo {
            store: poisoned_store(),
        };
        assert!(repo.get_latest(CommunityId::new()).is_err());
    }

    #[test]
    fn put_returns_err_when_lock_is_poisoned() {
        let repo = InMemoryCommunityRepo {
            store: poisoned_store(),
        };
        assert!(repo.put(Community::new()).is_err());
    }

    // --- repo: put ---

    #[test]
    fn repo_get_returns_none_for_unknown_id() {
        assert!(repo()
            .get(CommunityId::new(), SequenceId::zero())
            .unwrap()
            .is_none());
    }

    #[test]
    fn repo_get_latest_returns_none_for_unknown_id() {
        assert!(repo().get_latest(CommunityId::new()).unwrap().is_none());
    }

    #[test]
    fn repo_put_and_get_round_trips_community() {
        let repo = repo();
        let community = Community::new();
        let id = community.id;
        let version = community.version;
        repo.put(community.clone()).unwrap();
        assert_eq!(repo.get(id, version).unwrap(), Some(community));
    }

    #[test]
    fn repo_put_and_get_latest_returns_latest() {
        let repo = repo();
        let community = Community::new();
        let id = community.id;
        repo.put(community.clone()).unwrap();
        assert_eq!(repo.get_latest(id).unwrap(), Some(community));
    }

    #[test]
    fn repo_put_fails_on_duplicate_version() {
        let repo = repo();
        let community = Community::new();
        repo.put(community.clone()).unwrap();
        assert!(repo.put(community).is_err());
    }

    // --- repo: get_latest ---

    #[test]
    fn repo_get_latest_returns_highest_version() {
        let repo = repo();
        let community = Community::new();
        let id = community.id;
        let v0 = community.version;
        repo.put(community).unwrap();
        let v1 = SequenceId::from_u64(1);
        let newer = Community::new().with_id(id).with_luck(500).with_version(v1);
        repo.put(newer.clone()).unwrap();
        assert_eq!(repo.get_latest(id).unwrap(), Some(newer));
        assert!(repo.get(id, v0).unwrap().is_some());
    }

    // --- store ---

    #[test]
    fn store_get_returns_none_for_unknown_id() {
        assert!(store()
            .get(CommunityId::new(), SequenceId::zero())
            .unwrap()
            .is_none());
    }

    #[test]
    fn store_get_latest_returns_none_for_unknown_id() {
        assert!(store().get_latest(CommunityId::new()).unwrap().is_none());
    }

    #[test]
    fn store_put_and_get_round_trips_community() {
        let store = store();
        let community = Community::new();
        let id = community.id;
        let version = community.version;
        store.put(community.clone()).unwrap();
        assert_eq!(store.get(id, version).unwrap(), Some(community));
    }

    #[test]
    fn store_put_and_get_latest_returns_community() {
        let store = store();
        let community = Community::new();
        let id = community.id;
        store.put(community.clone()).unwrap();
        assert_eq!(store.get_latest(id).unwrap(), Some(community));
    }

    #[test]
    fn store_get_latest_applies_pending_effects_with_owned_event_log() {
        use crate::event_log_repo::InMemoryEventLogRepo;
        use fruit_domain::{
            event_log::EventPayload, event_log::StateMutation, event_log_repo::EventLogPersistor,
            fruit::STRAWBERRY, member::Member,
        };

        // Build community and pre-populate event log before moving it into the store.
        let mut community = Community::new();
        let member = Member::new("Alice");
        let alice_id = member.id;
        community.add_member(member);
        let id = community.id;

        let event_log = InMemoryEventLogRepo::new();
        let event = event_log
            .append_event(id, EventPayload::Grant { count: 1 })
            .unwrap();
        event_log
            .append_effect(
                event.id,
                id,
                vec![StateMutation::AddFruitToMember {
                    member_id: alice_id,
                    fruit: STRAWBERRY,
                }],
            )
            .unwrap();

        // Store community at version zero, then hand ownership of the log to the store.
        let repo = InMemoryCommunityRepo::new();
        repo.put(community).unwrap();
        let store = CommunityStore::new(repo, event_log);

        // get_latest must apply the pending effect and advance the version.
        let latest = store.get_latest(id).unwrap().unwrap();
        assert_eq!(latest.members[&alice_id].bag.count(STRAWBERRY), 1);
    }

    // --- store via &InMemoryEventLogRepo: covers delegation impls and store.init / get_latest with effects ---

    #[test]
    fn store_ref_event_log_init_creates_persisted_community() {
        use crate::event_log_repo::InMemoryEventLogRepo;
        let event_log = InMemoryEventLogRepo::new();
        let store = CommunityStore::new(InMemoryCommunityRepo::new(), &event_log);
        let community = store.init().unwrap();
        assert_eq!(store.get_latest(community.id).unwrap(), Some(community));
    }

    #[test]
    fn store_ref_event_log_get_latest_applies_pending_effects() {
        use crate::event_log_repo::InMemoryEventLogRepo;
        use fruit_domain::{event_log::StateMutation, fruit::STRAWBERRY, member::Member};

        let event_log = InMemoryEventLogRepo::new();
        let store = CommunityStore::new(InMemoryCommunityRepo::new(), &event_log);

        // create a community with one member
        let mut community = Community::new();
        let member = Member::new("Alice");
        let alice_id = member.id;
        community.add_member(member);
        store.put(community.clone()).unwrap();
        let id = community.id;

        // record an event + effect via generic helper (forces &InMemoryEventLogRepo monomorphization)
        let event = append_event_via(&event_log, id, 1);
        append_effect_via(
            &event_log,
            event.id,
            id,
            vec![StateMutation::AddFruitToMember {
                member_id: alice_id,
                fruit: STRAWBERRY,
            }],
        );

        // get_latest should apply the pending effect; version advances to the effect's sequence ID
        let latest = store.get_latest(id).unwrap().unwrap();
        assert_eq!(latest.members[&alice_id].bag.count(STRAWBERRY), 1);
        assert!(latest.version > event.id); // effect id > event id (shared sequence)
    }

    #[test]
    fn store_ref_event_log_query_methods_delegate_through_ref() {
        use crate::event_log_repo::InMemoryEventLogRepo;
        use fruit_domain::{event_log::EventPayload, event_log_repo::EventLogPersistor};

        let event_log = InMemoryEventLogRepo::new();
        let store = CommunityStore::new(InMemoryCommunityRepo::new(), &event_log);
        let cid = CommunityId::new();

        // append via direct log so sequence starts at 1
        let event = event_log
            .append_event(cid, EventPayload::Grant { count: 1 })
            .unwrap();
        let effect = event_log.append_effect(event.id, cid, vec![]).unwrap();

        // get_record, get_effect_for_event, get_latest_events all go through &InMemoryEventLogRepo
        assert_eq!(
            store.get_record(event.id).unwrap(),
            Some(event.clone().into())
        );
        assert_eq!(
            store.get_effect_for_event(event.id).unwrap(),
            Some(effect.clone())
        );
        assert_eq!(
            store.get_latest_records(cid, 5).unwrap(),
            vec![effect.into(), event.into()]
        );
    }
}
