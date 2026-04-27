use std::{
    error::Error as StdError,
    sync::{Arc, Mutex},
};

use regex::Regex;

use anomalies::{
    anomaly::{HasCategory, HasStatus},
    category::{Conflict, Interrupted},
    status::Status,
};
use exn::Exn;
use fruit_domain::{
    community::{Community, CommunityId},
    error::StorageLayerError,
    event_log::SequenceId,
};
use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;

use super::*;

// ── helpers ──────────────────────────────────────────────────────────────────

fn make_poison_error() -> Arc<Mutex<i32>> {
    let mutex = Arc::new(Mutex::new(0i32));
    let m2 = Arc::clone(&mutex);
    std::thread::spawn(move || {
        let _guard = m2.lock().unwrap();
        panic!("intentional poison");
    })
    .join()
    .ok();
    mutex
}

// ── Lock Display ──────────────────────────────────────────────────────────────

#[test]
fn lock_community_read_display() {
    assert_eq!(Lock::CommunityRead.to_string(), "community read lock");
}

#[test]
fn lock_community_write_display() {
    assert_eq!(Lock::CommunityWrite.to_string(), "community write lock");
}

#[test]
fn lock_event_log_read_display() {
    assert_eq!(Lock::EventLogRead.to_string(), "event log read lock");
}

#[test]
fn lock_event_log_write_display() {
    assert_eq!(Lock::EventLogWrite.to_string(), "event log write lock");
}

#[test]
fn lock_effect_log_read_display() {
    assert_eq!(Lock::EffectLogRead.to_string(), "effect log read lock");
}

#[test]
fn lock_effect_log_write_display() {
    assert_eq!(Lock::EffectLogWrite.to_string(), "effect log write lock");
}

// ── AlreadyExists::community ──────────────────────────────────────────────────

#[test]
fn already_exists_community_display() {
    let community = Community::new();
    let err = AlreadyExists::community(&community);
    assert_eq!(
        err.to_string(),
        format!(
            "could not write Community at version {} in community {} because it already exists",
            community.version.as_u64(),
            community.id.as_uuid(),
        )
    );
}

#[test]
fn already_exists_community_has_conflict_category() {
    let err = AlreadyExists::community(&Community::new());
    assert_eq!(err.category(), Conflict);
}

#[test]
fn already_exists_community_has_permanent_status() {
    let err = AlreadyExists::community(&Community::new());
    assert_eq!(err.status(), Status::Permanent);
}

#[test]
fn already_exists_community_source_is_none() {
    let err = AlreadyExists::community(&Community::new());
    assert!(StdError::source(&err).is_none());
}

// ── AlreadyExists::event ──────────────────────────────────────────────────────

#[test]
fn already_exists_event_display() {
    let cid = CommunityId::new();
    let version = SequenceId::new(42);
    let err = AlreadyExists::event(cid, version);
    assert_eq!(
        err.to_string(),
        format!(
            "could not write Event at version 42 in community {} because it already exists",
            cid.as_uuid(),
        )
    );
}

#[test]
fn already_exists_event_has_conflict_category() {
    let err = AlreadyExists::event(CommunityId::new(), SequenceId::new(1));
    assert_eq!(err.category(), Conflict);
}

#[test]
fn already_exists_event_has_permanent_status() {
    let err = AlreadyExists::event(CommunityId::new(), SequenceId::new(1));
    assert_eq!(err.status(), Status::Permanent);
}

#[test]
fn already_exists_event_source_is_none() {
    let err = AlreadyExists::event(CommunityId::new(), SequenceId::new(1));
    assert!(StdError::source(&err).is_none());
}

// ── AlreadyExists::effect ─────────────────────────────────────────────────────

#[test]
fn already_exists_effect_display() {
    let cid = CommunityId::new();
    let version = SequenceId::new(7);
    let err = AlreadyExists::effect(cid, version);
    assert_eq!(
        err.to_string(),
        format!(
            "could not write Effect at version 7 in community {} because it already exists",
            cid.as_uuid(),
        )
    );
}

#[test]
fn already_exists_effect_has_conflict_category() {
    let err = AlreadyExists::effect(CommunityId::new(), SequenceId::new(1));
    assert_eq!(err.category(), Conflict);
}

#[test]
fn already_exists_effect_has_permanent_status() {
    let err = AlreadyExists::effect(CommunityId::new(), SequenceId::new(1));
    assert_eq!(err.status(), Status::Permanent);
}

#[test]
fn already_exists_effect_source_is_none() {
    let err = AlreadyExists::effect(CommunityId::new(), SequenceId::new(1));
    assert!(StdError::source(&err).is_none());
}

// ── LockPoisoned ──────────────────────────────────────────────────────────────

#[test]
fn lock_poisoned_build_community_read() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let message = pe.to_string();
    let err = LockPoisoned::build(&pe, Lock::CommunityRead);
    assert_eq!(
        err.to_string(),
        format!("community read lock was poisoned: {message}")
    );
}

#[test]
fn lock_poisoned_build_community_write() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let message = pe.to_string();
    let err = LockPoisoned::build(&pe, Lock::CommunityWrite);
    assert_eq!(
        err.to_string(),
        format!("community write lock was poisoned: {message}")
    );
}

#[test]
fn lock_poisoned_build_event_log_read() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let message = pe.to_string();
    let err = LockPoisoned::build(&pe, Lock::EventLogRead);
    assert_eq!(
        err.to_string(),
        format!("event log read lock was poisoned: {message}")
    );
}

#[test]
fn lock_poisoned_build_event_log_write() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let message = pe.to_string();
    let err = LockPoisoned::build(&pe, Lock::EventLogWrite);
    assert_eq!(
        err.to_string(),
        format!("event log write lock was poisoned: {message}")
    );
}

#[test]
fn lock_poisoned_build_effect_log_read() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let message = pe.to_string();
    let err = LockPoisoned::build(&pe, Lock::EffectLogRead);
    assert_eq!(
        err.to_string(),
        format!("effect log read lock was poisoned: {message}")
    );
}

#[test]
fn lock_poisoned_build_effect_log_write() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let message = pe.to_string();
    let err = LockPoisoned::build(&pe, Lock::EffectLogWrite);
    assert_eq!(
        err.to_string(),
        format!("effect log write lock was poisoned: {message}")
    );
}

#[test]
fn lock_poisoned_has_interrupted_category() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let err = LockPoisoned::build(&pe, Lock::CommunityRead);
    assert_eq!(err.category(), Interrupted);
}

#[test]
fn lock_poisoned_has_temporary_status() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let err = LockPoisoned::build(&pe, Lock::CommunityRead);
    assert_eq!(err.status(), Status::Temporary);
}

#[test]
fn lock_poisoned_source_is_none() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let err = LockPoisoned::build(&pe, Lock::CommunityRead);
    assert!(StdError::source(&err).is_none());
}

// ── Error source ─────────────────────────────────────────────────────────────

#[test]
fn error_already_exists_source_is_none() {
    let err = AlreadyExists::community(&Community::new());
    assert!(StdError::source(&err).is_none());
}

#[test]
fn error_lock_poisoned_source_is_none() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let err = LockPoisoned::build(&pe, Lock::CommunityRead);
    assert!(StdError::source(&err).is_none());
}

// ── Exn error chain ───────────────────────────────────────────────────────────

#[test]
fn exn_already_exists_debug_shows_domain_and_db_messages() {
    let community = Community::new();
    let db_exn: Exn<Error> = Exn::new(AlreadyExists::community(&community));
    let domain_exn = StorageLayerError::raise("failed to create community", db_exn);
    let debug = format!("{domain_exn:?}");
    let re = Regex::new(concat!(
        r"^Storage layer error: failed to create community, at [^\n]+\n",
        r"\|\n",
        r"\|-> could not write Community at version 0 in community [0-9a-f-]+ because it already exists, at [^\n]+$",
    ))
    .unwrap();
    assert!(re.is_match(&debug), "debug output:\n{debug}");
}

#[test]
fn exn_lock_poisoned_debug_shows_domain_and_db_messages() {
    let mutex = make_poison_error();
    let pe = mutex.lock().unwrap_err();
    let db_exn: Exn<Error> = Exn::new(LockPoisoned::build(&pe, Lock::EventLogRead));
    let domain_exn = StorageLayerError::raise("failed to read event log", db_exn);
    let debug = format!("{domain_exn:?}");
    let re = Regex::new(concat!(
        r"^Storage layer error: failed to read event log, at [^\n]+\n",
        r"\|\n",
        r"\|-> event log read lock was poisoned: .+, at [^\n]+$",
    ))
    .unwrap();
    assert!(re.is_match(&debug), "debug output:\n{debug}");
}

#[test]
fn exn_display_shows_only_domain_message() {
    let community = Community::new();
    let db_exn: Exn<Error> = Exn::new(AlreadyExists::community(&community));
    let domain_exn = StorageLayerError::raise("failed to create community", db_exn);
    let display = format!("{domain_exn}");
    assert_eq!(display, "Storage layer error: failed to create community");
}
