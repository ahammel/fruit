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

    /// Overwrites an existing snapshot, or inserts it if absent.
    pub fn replace(&self, community: Community) -> Result<Community, Error> {
        self.community_repo.replace(community)
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
mod tests {
    use std::io;

    use super::*;
    use crate::{
        community_repo::{CommunityPersistor, CommunityProvider},
        event_log::Effect,
        event_log::Record,
        id::{IntegerIdentifier, UuidIdentifier},
    };

    fn err() -> Error {
        io::Error::new(io::ErrorKind::Other, "test error").into()
    }

    // --- mock repo that always errors ---

    struct ErrorRepo;

    impl CommunityProvider for ErrorRepo {
        fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Error> {
            Err(err())
        }
        fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Error> {
            Err(err())
        }
    }

    impl CommunityPersistor for ErrorRepo {
        fn put(&self, _: Community) -> Result<Community, Error> {
            Err(err())
        }
        fn replace(&self, _: Community) -> Result<Community, Error> {
            Err(err())
        }
    }

    impl CommunityRepo for ErrorRepo {}

    // --- mock repo that returns one community then fails puts ---

    struct GetOkPutErrorRepo {
        community: Community,
    }

    impl CommunityProvider for GetOkPutErrorRepo {
        fn get(&self, _: CommunityId, _: SequenceId) -> Result<Option<Community>, Error> {
            Ok(None)
        }
        fn get_latest(&self, _: CommunityId) -> Result<Option<Community>, Error> {
            Ok(Some(self.community.clone()))
        }
    }

    impl CommunityPersistor for GetOkPutErrorRepo {
        fn put(&self, _: Community) -> Result<Community, Error> {
            Err(err())
        }
        fn replace(&self, _: Community) -> Result<Community, Error> {
            Err(err())
        }
    }

    impl CommunityRepo for GetOkPutErrorRepo {}

    // --- mock event log that always errors ---

    struct ErrorEventLog;

    impl EventLogProvider for ErrorEventLog {
        fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Error> {
            Err(err())
        }
        fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Error> {
            Err(err())
        }
        fn get_effects_after(&self, _: CommunityId, _: SequenceId) -> Result<Vec<Effect>, Error> {
            Err(err())
        }
        fn get_latest_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Error> {
            Err(err())
        }
    }

    // --- mock event log that returns one effect ---

    struct OneEffectEventLog {
        effect: Effect,
    }

    impl EventLogProvider for OneEffectEventLog {
        fn get_record(&self, _: SequenceId) -> Result<Option<Record>, Error> {
            Ok(None)
        }
        fn get_effect_for_event(&self, _: SequenceId) -> Result<Option<Effect>, Error> {
            Ok(None)
        }
        fn get_effects_after(&self, _: CommunityId, _: SequenceId) -> Result<Vec<Effect>, Error> {
            Ok(vec![self.effect.clone()])
        }
        fn get_latest_records(&self, _: CommunityId, _: usize) -> Result<Vec<Record>, Error> {
            Ok(vec![])
        }
    }

    // --- error propagation tests ---

    #[test]
    fn get_propagates_repo_error() {
        let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
        assert!(store.get(CommunityId::new(), SequenceId::zero()).is_err());
    }

    #[test]
    fn put_propagates_repo_error() {
        let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
        assert!(store.put(Community::new()).is_err());
    }

    #[test]
    fn replace_propagates_repo_error() {
        let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
        assert!(store.replace(Community::new()).is_err());
    }

    #[test]
    fn get_latest_propagates_repo_error() {
        let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
        assert!(store.get_latest(CommunityId::new()).is_err());
    }

    #[test]
    fn get_latest_propagates_event_log_error() {
        let community = Community::new();
        let id = community.id;
        let store = CommunityStore::new(GetOkPutErrorRepo { community }, ErrorEventLog);
        assert!(store.get_latest(id).is_err());
    }

    #[test]
    fn get_latest_propagates_put_error_after_applying_effects() {
        use crate::id::IntegerIdentifier;
        let community = Community::new();
        let id = community.id;
        let effect = Effect {
            id: SequenceId::from_u64(2),
            event_id: SequenceId::from_u64(1),
            community_id: id,
            mutations: vec![],
        };
        let store = CommunityStore::new(
            GetOkPutErrorRepo { community },
            OneEffectEventLog { effect },
        );
        assert!(store.get_latest(id).is_err());
    }

    #[test]
    fn get_with_get_ok_put_error_repo_returns_none() {
        let community = Community::new();
        let id = community.id;
        let store = CommunityStore::new(GetOkPutErrorRepo { community }, ErrorEventLog);
        assert!(store.get(id, SequenceId::zero()).unwrap().is_none());
    }

    #[test]
    fn replace_with_get_ok_put_error_repo_returns_err() {
        let community = Community::new();
        let id = community.id;
        let store = CommunityStore::new(GetOkPutErrorRepo { community }, ErrorEventLog);
        assert!(store.replace(Community::new().with_id(id)).is_err());
    }

    #[test]
    fn get_record_propagates_error() {
        let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
        assert!(store.get_record(SequenceId::zero()).is_err());
    }

    #[test]
    fn get_effect_for_event_propagates_error() {
        let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
        assert!(store.get_effect_for_event(SequenceId::zero()).is_err());
    }

    #[test]
    fn get_latest_events_propagates_error() {
        let store = CommunityStore::new(ErrorRepo, ErrorEventLog);
        assert!(store.get_latest_records(CommunityId::new(), 5).is_err());
    }

    #[test]
    fn get_record_returns_none_via_one_effect_log() {
        use crate::id::IntegerIdentifier;
        let effect = Effect {
            id: SequenceId::from_u64(2),
            event_id: SequenceId::from_u64(1),
            community_id: CommunityId::new(),
            mutations: vec![],
        };
        let store = CommunityStore::new(ErrorRepo, OneEffectEventLog { effect });
        assert!(store.get_record(SequenceId::zero()).unwrap().is_none());
    }

    #[test]
    fn get_effect_for_event_returns_none_via_one_effect_log() {
        use crate::id::IntegerIdentifier;
        let effect = Effect {
            id: SequenceId::from_u64(2),
            event_id: SequenceId::from_u64(1),
            community_id: CommunityId::new(),
            mutations: vec![],
        };
        let store = CommunityStore::new(ErrorRepo, OneEffectEventLog { effect });
        assert!(store
            .get_effect_for_event(SequenceId::zero())
            .unwrap()
            .is_none());
    }

    #[test]
    fn get_latest_events_returns_empty_via_one_effect_log() {
        use crate::id::IntegerIdentifier;
        let effect = Effect {
            id: SequenceId::from_u64(2),
            event_id: SequenceId::from_u64(1),
            community_id: CommunityId::new(),
            mutations: vec![],
        };
        let store = CommunityStore::new(ErrorRepo, OneEffectEventLog { effect });
        assert!(store
            .get_latest_records(CommunityId::new(), 5)
            .unwrap()
            .is_empty());
    }
}
