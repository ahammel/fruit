use anomalies::{
    anomaly::{Anomaly, HasCategory, HasStatus},
    category::Category,
    status::Status,
};
use exn::Exn;
use thiserror::Error;

/// Errors that can occur in the domain layer
#[derive(Debug, Error, Anomaly)]
pub enum Error {
    /// A storage layer error with dynamic category and status.
    #[error(transparent)]
    #[anomaly(transparent)]
    StorageLayerError(StorageLayerError),

    /// A storage layer error that is guaranteed to be retryable.
    #[error("Retryable storage layer error: {message}")]
    #[category(unavailable)]
    RetryableStorageLayerError { message: String },
}

/// Returned to re-wrap errors from the storage layer
#[derive(Debug, Error)]
#[error("Storage layer error: {message}")]
pub struct StorageLayerError {
    message: String,
    category: Category,
    status: Status,
}

impl StorageLayerError {
    /// Builds a domain-layer [`Error`] wrapping this type with explicit category and status.
    pub fn build(message: impl Into<String>, category: Category, status: Status) -> Error {
        Error::StorageLayerError(StorageLayerError {
            message: message.into(),
            category,
            status,
        })
    }

    /// Wraps a database-layer [`Exn`] in a domain-layer [`Exn`], preserving category and status.
    pub fn raise(message: impl Into<String>, err: Exn<impl DbError>) -> Exn<Error> {
        let storage_error = Error::StorageLayerError(StorageLayerError {
            message: message.into(),
            status: err.status(),
            category: err.category(),
        });
        err.raise(storage_error)
    }
}

impl HasCategory for StorageLayerError {
    fn category(&self) -> Category {
        self.category
    }
}

impl HasStatus for StorageLayerError {
    fn status(&self) -> Status {
        self.status
    }
}

impl Anomaly for StorageLayerError {}

/// Marker trait for errors returned by database port implementations.
///
/// Implementors satisfy the bounds required by [`exn::Exn`] and carry enough
/// information for callers to categorise and act on failures without knowing the
/// concrete database backend.
pub trait DbError: Anomaly + Send + Sync + 'static {}

impl DbError for Error {}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
