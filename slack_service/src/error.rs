use std::fmt;

use anomalies::{
    anomaly::{Anomaly, HasCategory, HasStatus},
    category::Category,
    status::Status,
};
use exn::Exn;
use fruit_domain::error::DbError;
use thiserror::Error;

/// Service-level error for the slash-command handler.
#[derive(Debug, Error, Anomaly)]
pub enum Error {
    /// A storage operation failed while handling a user command.
    #[error(transparent)]
    #[anomaly(transparent)]
    CommandProcessing(CommandProcessingError),
}

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
