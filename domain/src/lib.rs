//! Domain model for the fruit gift-economy simulation game.
//!
//! This crate contains pure business logic with no I/O or side effects.
//! Storage ports ([`community_repo`], [`event_log_repo`]) are defined here as
//! traits; concrete implementations live in sibling crates such as
//! `fruit_in_memory_db`.

pub mod bag;
pub mod burner;
pub mod community;
pub mod community_repo;
pub mod community_store;
pub mod error;
pub mod event_log;
pub mod event_log_repo;
pub mod event_log_store;
pub mod fruit;
pub mod fruit_weights;
pub mod gifter;
pub mod granter;
pub mod id;
pub mod member;
pub mod random_granter;
