use std::fmt;

use crate::{community::CommunityId, id::IntegerIdentifier};

/// A position in the shared event/effect log sequence.
///
/// Sequence IDs start at 1 and increase monotonically. Both events and effects
/// draw from the same counter, so their IDs are globally ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SequenceId(u64);

impl IntegerIdentifier for SequenceId {
    fn zero() -> Self {
        Self(0)
    }

    fn from_u64(id: u64) -> Self {
        Self(id)
    }

    fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for SequenceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The action a player intended to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPayload {
    /// Distribute `count` fruits to every member of the community.
    Grant { count: usize },
}

/// A recorded player intention. Events do not modify
/// [`Community`](crate::community::Community) state directly; their consequences are
/// computed as [`Effect`](crate::effect::Effect)s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Position in the shared event/effect sequence.
    pub id: SequenceId,
    /// The community this event applies to.
    pub community_id: CommunityId,
    /// The action the player intended.
    pub payload: EventPayload,
}
