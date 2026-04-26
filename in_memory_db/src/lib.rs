//! In-memory implementations of the domain storage ports.
//!
//! Provides [`community_repo::InMemoryCommunityRepo`] and
//! [`event_log_repo::InMemoryEventLogRepo`], which back all data with
//! `RwLock`-guarded collections. Intended for development, testing, and the
//! command-line REPL; not suitable for production persistence.

pub mod community_repo;
pub mod error;
pub mod event_log_repo;
