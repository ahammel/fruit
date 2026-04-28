//! DynamoDB implementations of the domain storage ports.
//!
//! Provides [`event_log_repo::DynamoDbEventLogRepo`] and
//! [`community_repo::DynamoDbCommunityRepo`], backed by a single DynamoDB table
//! using a single-table design with a composite sort key of entity type and
//! sequence ID.

pub mod community_repo;
pub mod error;
pub mod event_log_repo;
mod dto;
