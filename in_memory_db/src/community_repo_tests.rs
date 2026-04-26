use super::*;
use crate::event_log_repo::InMemoryEventLogRepo;
use exn::Exn;
use fruit_domain::{
    community_repo::{CommunityPersistor, CommunityProvider},
    community_store::{CommunityStore, EFFECTS_PAGE_SIZE},
    event_log::EventPayload,
    event_log_repo::EventLogPersistor,
};
use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;

fn repo() -> InMemoryCommunityRepo {
    InMemoryCommunityRepo::new()
}

fn store() -> CommunityStore<InMemoryCommunityRepo, InMemoryEventLogRepo> {
    CommunityStore::new(InMemoryCommunityRepo::new(), InMemoryEventLogRepo::new())
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
    let v1 = SequenceId::new(1);
    let newer = Community::new().with_id(id).with_luck(50).with_version(v1);
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

// --- store: init / get_latest with multiple effects ---

#[test]
fn store_init_creates_persisted_community() {
    let store = store();
    let community = store.init().unwrap();
    assert_eq!(store.get_latest(community.id).unwrap(), Some(community));
}

#[test]
fn store_get_latest_applies_multiple_pending_effects() {
    use fruit_domain::{event_log::StateMutation, fruit::STRAWBERRY, member::Member};

    // Start with a bare community (no members yet).
    let community = Community::new();
    let id = community.id;

    // Pre-populate the event log: AddMember then Grant effects.
    let event_log = InMemoryEventLogRepo::new();
    let alice = Member::new("Alice");
    let alice_id = alice.id;
    let add_member_event = event_log
        .append_event(
            id,
            EventPayload::AddMember {
                display_name: "Alice".to_string(),
                member_id: alice_id,
            },
        )
        .unwrap();
    event_log
        .append_effect(
            add_member_event.id,
            id,
            vec![StateMutation::AddMember { member: alice }],
        )
        .unwrap();
    let grant_event = event_log
        .append_event(id, EventPayload::Grant { count: 1 })
        .unwrap();
    event_log
        .append_effect(
            grant_event.id,
            id,
            vec![StateMutation::AddFruitToMember {
                member_id: alice_id,
                fruit: STRAWBERRY,
            }],
        )
        .unwrap();

    // Hand ownership of the log to the store and apply effects via get_latest.
    let repo = InMemoryCommunityRepo::new();
    repo.put(community).unwrap();
    let store = CommunityStore::new(repo, event_log);

    let latest = store.get_latest(id).unwrap().unwrap();
    assert_eq!(latest.members[&alice_id].bag.count(STRAWBERRY), 1);
    assert_eq!(latest.version, grant_event.id); // effect and event share the same ID
}

#[test]
fn store_get_latest_paginates_through_effects_exceeding_page_size() {
    // Appends EFFECTS_PAGE_SIZE + 1 effects so get_latest must loop a second time,
    // covering the loop-continues branch in the InMemory monomorphization.
    use fruit_domain::event_log::SequenceId;

    let community = Community::new();
    let id = community.id;
    let event_log = InMemoryEventLogRepo::new();
    let n = EFFECTS_PAGE_SIZE + 1;
    for _ in 0..n {
        let event = event_log
            .append_event(id, EventPayload::Grant { count: 0 })
            .unwrap();
        event_log.append_effect(event.id, id, vec![]).unwrap();
    }
    let repo = InMemoryCommunityRepo::new();
    repo.put(community).unwrap();
    let store = CommunityStore::new(repo, event_log);
    let latest = store.get_latest(id).unwrap().unwrap();
    assert_eq!(latest.version, SequenceId::from(n as u64));
}

// --- &InMemoryCommunityRepo delegation ---

fn via_provider_get<T: CommunityProvider>(
    p: T,
    id: CommunityId,
    version: SequenceId,
) -> Result<Option<Community>, Exn<T::Error>> {
    p.get(id, version)
}

fn via_provider_get_latest<T: CommunityProvider>(
    p: T,
    id: CommunityId,
) -> Result<Option<Community>, Exn<T::Error>> {
    p.get_latest(id)
}

fn via_persistor_put<T: CommunityPersistor>(
    p: T,
    community: Community,
) -> Result<Community, Exn<T::Error>> {
    p.put(community)
}

#[test]
fn ref_delegates_get() {
    let repo = repo();
    let community = Community::new();
    let id = community.id;
    let version = community.version;
    repo.put(community.clone()).unwrap();
    assert_eq!(
        via_provider_get(&repo, id, version).unwrap(),
        Some(community)
    );
}

#[test]
fn ref_delegates_get_latest() {
    let repo = repo();
    let community = Community::new();
    let id = community.id;
    repo.put(community.clone()).unwrap();
    assert_eq!(via_provider_get_latest(&repo, id).unwrap(), Some(community));
}

#[test]
fn ref_delegates_put() {
    let repo = repo();
    let community = Community::new();
    let id = community.id;
    let version = community.version;
    via_persistor_put(&repo, community.clone()).unwrap();
    assert_eq!(repo.get(id, version).unwrap(), Some(community));
}
