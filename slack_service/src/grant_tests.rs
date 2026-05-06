use std::sync::{Arc, Mutex};

use exn::Exn;
use fruit_domain::{
    community::CommunityId,
    community_repo::CommunityPersistor,
    community_store::CommunityStore,
    event_log::{EventPayload, StateMutation},
    event_log_store::EventLogStore,
    member::{ExternalUserId, Member, MemberId},
    providence::Providence,
    random_granter::RandomGranter,
};
use fruit_in_memory_db::{
    community_repo::InMemoryCommunityRepo, event_log_repo::InMemoryEventLogRepo,
};
use rand::thread_rng;

use super::{handle_grant, GrantDetail};
use crate::{
    error::{Error, NotificationError},
    identity,
    notify::Notifier,
};

// ── Test infrastructure ───────────────────────────────────────────────────────

const TEAM_ID: &str = "T_TEST";
const CHANNEL: &str = "C_GRANT_TEST";

fn repos() -> (InMemoryCommunityRepo, InMemoryEventLogRepo) {
    (InMemoryCommunityRepo::new(), InMemoryEventLogRepo::new())
}

fn community_id() -> CommunityId {
    let ns = identity::workspace_namespace(TEAM_ID);
    identity::community_id_for(ns, CHANNEL)
}

fn alice_id() -> MemberId {
    let ns = identity::workspace_namespace(TEAM_ID);
    identity::member_id_for(ns, "U_ALICE")
}

fn detail(count: usize) -> GrantDetail {
    GrantDetail {
        team_id: TEAM_ID.to_string(),
        channel_id: CHANNEL.to_string(),
        count,
    }
}

fn providence<'a>(
    cr: &'a InMemoryCommunityRepo,
    elr: &'a InMemoryEventLogRepo,
) -> Providence<
    &'a InMemoryEventLogRepo,
    &'a InMemoryCommunityRepo,
    RandomGranter<rand::rngs::ThreadRng>,
> {
    Providence::new(elr, cr, RandomGranter::new(thread_rng()))
}

/// Sets up a community with Alice as a member (with a Slack external ID).
async fn setup_community(
    community_repo: &InMemoryCommunityRepo,
    event_log_repo: &InMemoryEventLogRepo,
) -> CommunityId {
    let cid = community_id();
    let mid = alice_id();
    community_repo
        .put(fruit_domain::community::Community::new().with_id(cid))
        .await
        .unwrap();
    let event_log = EventLogStore::new(event_log_repo);
    let event = event_log
        .append_event(
            cid,
            EventPayload::AddMember {
                display_name: "alice".to_string(),
                member_id: mid,
                external_id: Some(ExternalUserId::slack("U_ALICE")),
            },
        )
        .await
        .unwrap();
    event_log
        .append_effect(
            event.id,
            cid,
            vec![StateMutation::AddMember {
                member: Member::new("alice")
                    .with_id(mid)
                    .with_external_id(ExternalUserId::slack("U_ALICE")),
            }],
        )
        .await
        .unwrap();
    cid
}

/// A [`Notifier`] that records calls and optionally fails.
struct RecordingNotifier {
    calls: Arc<Mutex<Vec<(String, String)>>>,
    dm_calls: Arc<Mutex<Vec<(String, String)>>>,
    fail: bool,
}

impl RecordingNotifier {
    fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(vec![])),
            dm_calls: Arc::new(Mutex::new(vec![])),
            fail: false,
        }
    }

    fn failing() -> Self {
        Self {
            calls: Arc::new(Mutex::new(vec![])),
            dm_calls: Arc::new(Mutex::new(vec![])),
            fail: true,
        }
    }

    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }

    fn dm_calls(&self) -> Vec<(String, String)> {
        self.dm_calls.lock().unwrap().clone()
    }
}

impl Notifier for RecordingNotifier {
    async fn post_message(&self, channel_id: &str, text: &str) -> Result<(), Exn<Error>> {
        if self.fail {
            return Err(Exn::new(NotificationError::slack_api("channel_not_found")));
        }
        self.calls
            .lock()
            .unwrap()
            .push((channel_id.to_string(), text.to_string()));
        Ok(())
    }

    async fn post_dm(&self, slack_user_id: &str, text: &str) -> Result<(), Exn<Error>> {
        if self.fail {
            return Err(Exn::new(NotificationError::slack_api("channel_not_found")));
        }
        self.dm_calls
            .lock()
            .unwrap()
            .push((slack_user_id.to_string(), text.to_string()));
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn grant_dispatched_to_community_with_members() {
    let (cr, elr) = repos();
    setup_community(&cr, &elr).await;
    let notifier = RecordingNotifier::new();
    let community_store = CommunityStore::new(&cr, &elr);
    let mut prov = providence(&cr, &elr);

    let result = handle_grant(&community_store, &mut prov, &notifier, &detail(1)).await;

    assert!(result.is_ok());
    let calls = notifier.calls();
    assert_eq!(calls.len(), 1);
    let (channel, text) = &calls[0];
    assert_eq!(channel, CHANNEL);
    assert!(text.contains("fruit(s) distributed"), "message was: {text}");
}

#[tokio::test]
async fn grant_dms_each_recipient() {
    let (cr, elr) = repos();
    setup_community(&cr, &elr).await;
    let notifier = RecordingNotifier::new();
    let community_store = CommunityStore::new(&cr, &elr);
    let mut prov = providence(&cr, &elr);

    let result = handle_grant(&community_store, &mut prov, &notifier, &detail(1)).await;

    assert!(result.is_ok());
    let dms = notifier.dm_calls();
    assert_eq!(dms.len(), 1);
    let (user_id, text) = &dms[0];
    assert_eq!(user_id, "U_ALICE");
    assert!(text.contains("You received"), "dm was: {text}");
}

#[tokio::test]
async fn grant_skipped_when_no_community() {
    let (cr, elr) = repos();
    let notifier = RecordingNotifier::new();
    let community_store = CommunityStore::new(&cr, &elr);
    let mut prov = providence(&cr, &elr);

    let result = handle_grant(&community_store, &mut prov, &notifier, &detail(1)).await;

    assert!(result.is_ok());
    assert!(notifier.calls().is_empty());
    assert!(notifier.dm_calls().is_empty());
}

#[tokio::test]
async fn grant_skipped_when_community_has_no_members() {
    let (cr, elr) = repos();
    let cid = community_id();
    cr.put(fruit_domain::community::Community::new().with_id(cid))
        .await
        .unwrap();
    let notifier = RecordingNotifier::new();
    let community_store = CommunityStore::new(&cr, &elr);
    let mut prov = providence(&cr, &elr);

    let result = handle_grant(&community_store, &mut prov, &notifier, &detail(1)).await;

    assert!(result.is_ok());
    assert!(notifier.calls().is_empty());
    assert!(notifier.dm_calls().is_empty());
}

#[tokio::test]
async fn notification_failure_propagated() {
    let (cr, elr) = repos();
    setup_community(&cr, &elr).await;
    let notifier = RecordingNotifier::failing();
    let community_store = CommunityStore::new(&cr, &elr);
    let mut prov = providence(&cr, &elr);

    let result = handle_grant(&community_store, &mut prov, &notifier, &detail(1)).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn grant_count_zero_distributes_no_fruits_but_notifies() {
    let (cr, elr) = repos();
    setup_community(&cr, &elr).await;
    let notifier = RecordingNotifier::new();
    let community_store = CommunityStore::new(&cr, &elr);
    let mut prov = providence(&cr, &elr);

    let result = handle_grant(&community_store, &mut prov, &notifier, &detail(0)).await;

    assert!(result.is_ok());
    let calls = notifier.calls();
    assert_eq!(calls.len(), 1);
    let (_, text) = &calls[0];
    assert!(
        text.contains("no fruits distributed"),
        "message was: {text}"
    );
    assert!(notifier.dm_calls().is_empty());
}
