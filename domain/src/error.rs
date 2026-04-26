use std::{error, fmt::Display};

use anomalies::{
    anomaly::{Anomaly, HasCategory, HasStatus},
    category::Category,
    status::Status,
};
use exn::Exn;

/// Returned to re-wrap errors from the storage layer
#[derive(Debug)]
pub struct StorageLayerError {
    message: String,
    category: Category,
    status: Status,
}

impl StorageLayerError {
    pub fn build(message: impl Into<String>, category: Category, status: Status) -> Error {
        Error::StorageLayerError(StorageLayerError {
            message: message.into(),
            category,
            status,
        })
    }

    pub fn raise(message: impl Into<String>, err: Exn<impl DbError>) -> Exn<Error> {
        let storage_error = Error::StorageLayerError(StorageLayerError {
            message: message.into(),
            status: err.status(),
            category: err.category(),
        });
        err.raise(storage_error)
    }
}

impl Display for StorageLayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Storage layer error: {}", self.message)
    }
}

impl error::Error for StorageLayerError {}

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

/// Errors from the storage layer that are guaranteed to be retryable by business logic
#[derive(Debug, Anomaly)]
#[category(unavailable)]
pub struct RetryableStorageLayerError {
    message: String,
}

impl Display for RetryableStorageLayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Retryable storage layer error: {}", self.message)
    }
}

impl error::Error for RetryableStorageLayerError {}

/// Errors that can occur in the domain layer
#[derive(Debug)]
pub enum Error {
    StorageLayerError(StorageLayerError),
    RetryableStorageLayerError(RetryableStorageLayerError),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::StorageLayerError(e) => e.fmt(f),
            Error::RetryableStorageLayerError(e) => e.fmt(f),
        }
    }
}

impl error::Error for Error {}

impl HasCategory for Error {
    fn category(&self) -> Category {
        match self {
            Error::StorageLayerError(e) => e.category(),
            Error::RetryableStorageLayerError(e) => e.category(),
        }
    }
}

impl HasStatus for Error {
    fn status(&self) -> Status {
        match self {
            Error::StorageLayerError(e) => e.status(),
            Error::RetryableStorageLayerError(e) => e.status(),
        }
    }
}

impl Anomaly for Error {}

impl From<StorageLayerError> for Error {
    fn from(value: StorageLayerError) -> Self {
        Error::StorageLayerError(value)
    }
}

impl From<RetryableStorageLayerError> for Error {
    fn from(value: RetryableStorageLayerError) -> Self {
        Error::RetryableStorageLayerError(value)
    }
}

/// Marker trait for errors returned by database port implementations.
///
/// Implementors satisfy the bounds required by [`exn::Exn`] and carry enough
/// information for callers to categorise and act on failures without knowing the
/// concrete database backend.
pub trait DbError: Anomaly + Send + Sync + 'static {}

impl DbError for Error {}

#[cfg(test)]
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        StorageLayerError::build(e.to_string(), anomalies::category::Fault, Status::Permanent)
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
