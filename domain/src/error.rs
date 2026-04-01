use std::{fmt, io, sync::PoisonError};

/// Errors that can occur when accessing domain storage.
#[derive(Debug)]
pub enum Error {
    /// An I/O error from an underlying storage backend.
    Io(io::Error),
    /// A lock was poisoned because a thread panicked while holding it.
    ///
    /// `PoisonError<T>` implements `std::error::Error` but carries a lock guard
    /// as `T`, giving it a non-`'static` lifetime. That prevents storing it as
    /// `Box<dyn Error + 'static>` the way [`Error::Io`] is stored, so the
    /// message is extracted as a [`String`] at the call site instead. As a
    /// result, [`std::error::Error::source`] returns `None` for this variant.
    Poisoned(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Poisoned(msg) => write!(f, "poisoned lock: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Poisoned(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

// `T` is a lock guard with a non-`'static` lifetime, so `PoisonError<T>`
// cannot be boxed as `dyn Error + 'static`. Extract the message here instead.
impl<T> From<PoisonError<T>> for Error {
    fn from(e: PoisonError<T>) -> Self {
        Error::Poisoned(e.to_string())
    }
}
