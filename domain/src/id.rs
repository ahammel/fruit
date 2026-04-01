use std::fmt::Debug;
use std::hash::Hash;

use uuid::Uuid;

/// Common interface for typed UUID wrappers.
pub trait UuidIdentifier: Debug + Clone + Copy + PartialEq + Eq + Hash {
    /// Generates a new random identifier.
    fn new() -> Self;

    /// Returns the underlying [`Uuid`].
    fn as_uuid(&self) -> Uuid;
}
