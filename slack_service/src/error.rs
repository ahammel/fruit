use std::fmt;

use anomalies::{
    anomaly::{Anomaly, HasCategory, HasStatus},
    category::{Category, Fault, Unavailable},
    status::Status,
};
use exn::Exn;
use fruit_domain::community::CommunityId;
use fruit_domain::error::DbError;
use thiserror::Error;

/// Service-level error for the slash-command handler.
#[derive(Debug, Error, Anomaly)]
pub enum Error {
    /// A storage operation failed while handling a user command.
    #[error(transparent)]
    #[anomaly(transparent)]
    CommandProcessing(CommandProcessingError),

    /// A storage operation failed while running a scheduled grant.
    #[error(transparent)]
    #[anomaly(transparent)]
    Grant(GrantError),

    /// A Slack API call failed while sending a grant notification.
    #[error(transparent)]
    #[anomaly(transparent)]
    Notification(NotificationError),
}

// ── CommandProcessingError ────────────────────────────────────────────────────

/// A storage failure encountered while handling a user command.
///
/// Captures the command text that triggered the failure and inherits the
/// underlying error's category and retry status.
#[derive(Debug)]
pub struct CommandProcessingError {
    command_text: String,
    category: Category,
    status: Status,
}

impl CommandProcessingError {
    /// Wraps a lower-layer [`Exn`] in a service-layer [`Exn`], capturing
    /// `command_text` and inheriting `category` and `status` from the cause.
    pub fn raise<E: DbError>(command_text: impl Into<String>, err: Exn<E>) -> Exn<Error> {
        let processing_error = Error::CommandProcessing(CommandProcessingError {
            command_text: command_text.into(),
            category: err.category(),
            status: err.status(),
        });
        err.raise(processing_error)
    }
}

impl fmt::Display for CommandProcessingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error processing command {:?}", self.command_text)
    }
}

impl std::error::Error for CommandProcessingError {}

impl HasCategory for CommandProcessingError {
    fn category(&self) -> Category {
        self.category
    }
}

impl HasStatus for CommandProcessingError {
    fn status(&self) -> Status {
        self.status
    }
}

impl Anomaly for CommandProcessingError {}

// ── GrantError ────────────────────────────────────────────────────────────────

/// A storage failure encountered while processing a scheduled grant.
///
/// Captures the community context and inherits the underlying error's category
/// and retry status.
#[derive(Debug)]
pub struct GrantError {
    community_id: CommunityId,
    channel_id: String,
    count: usize,
    category: Category,
    status: Status,
}

impl GrantError {
    /// Wraps a lower-layer [`Exn`] in a service-layer [`Exn`], capturing
    /// `community_id`, `channel_id`, and `count` and inheriting `category` and
    /// `status` from the cause.
    pub fn raise<E: DbError>(
        community_id: CommunityId,
        channel_id: &str,
        count: usize,
        err: Exn<E>,
    ) -> Exn<Error> {
        let grant_error = Error::Grant(GrantError {
            community_id,
            channel_id: channel_id.to_string(),
            count,
            category: err.category(),
            status: err.status(),
        });
        err.raise(grant_error)
    }
}

impl fmt::Display for GrantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error processing grant for community {:?} (channel: {:?}, count: {})",
            self.community_id, self.channel_id, self.count
        )
    }
}

impl std::error::Error for GrantError {}

impl HasCategory for GrantError {
    fn category(&self) -> Category {
        self.category
    }
}

impl HasStatus for GrantError {
    fn status(&self) -> Status {
        self.status
    }
}

impl Anomaly for GrantError {}

// ── NotificationError ─────────────────────────────────────────────────────────

/// A failure sending a Slack notification via `chat.postMessage`.
#[derive(Debug)]
pub struct NotificationError {
    message: String,
    category: Category,
    status: Status,
}

impl NotificationError {
    /// Builds a temporary, unavailable-category notification error for use in
    /// [`or_raise`][exn::ResultExt::or_raise] closures.
    pub fn network(context: &str) -> Error {
        Error::Notification(NotificationError {
            message: context.to_string(),
            category: Unavailable,
            status: Status::Temporary,
        })
    }

    /// Builds a permanent fault notification error for use with [`Exn::new`]
    /// when there is no upstream error to chain.
    pub fn slack_api(api_error: impl Into<String>) -> Error {
        Error::Notification(NotificationError {
            message: api_error.into(),
            category: Fault,
            status: Status::Permanent,
        })
    }
}

impl fmt::Display for NotificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Slack notification failed: {}", self.message)
    }
}

impl std::error::Error for NotificationError {}

impl HasCategory for NotificationError {
    fn category(&self) -> Category {
        self.category
    }
}

impl HasStatus for NotificationError {
    fn status(&self) -> Status {
        self.status
    }
}

impl Anomaly for NotificationError {}
