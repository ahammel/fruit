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

/// Common interface for typed `u64` wrappers used as identifiers or sequence positions.
///
/// Implies [`Ord`] and [`PartialOrd`]; implementors must derive or implement those traits.
pub trait IntegerIdentifier:
    Debug + Clone + Copy + PartialEq + Eq + Hash + Ord + PartialOrd
{
    /// Returns an instance initialised to zero (the smallest valid value).
    fn zero() -> Self;

    /// Wraps a raw `u64` as a typed identifier.
    fn from_u64(id: u64) -> Self;

    /// Returns the underlying `u64`.
    fn as_u64(&self) -> u64;
}
