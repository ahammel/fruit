use std::collections::HashMap;

use newtype_ids::IntegerIdentifier;
use newtype_ids_uuid::UuidIdentifier;
use uuid::Uuid;

use crate::{
    bag::bag_value,
    event_log::{Effect, SequenceId},
    member::{Member, MemberId},
};

/// Typed identifier for a [`Community`].
#[derive(UuidIdentifier)]
#[allowed_values(all)]
pub struct CommunityId(Uuid);

/// Marker trait for types that belong to a [`Community`].
pub trait HasCommunityId {
    /// Returns the [`CommunityId`] of the community this value belongs to.
    fn community_id(&self) -> CommunityId;
}

/// A group of members that share a collective luck modifier.
#[derive(Debug, Clone, PartialEq)]
pub struct Community {
    /// Unique identifier for this community.
    pub id: CommunityId,
    _luck: u8,
    /// Members belonging to this community, keyed by their ID.
    pub members: HashMap<MemberId, Member>,
    /// The event log position up to which this snapshot has been computed.
    ///
    /// [`SequenceId::zero()`] means no effects have been applied yet; the snapshot
    /// reflects only the community's initial structural state (members, luck).
    pub version: SequenceId,
}

impl Community {
    /// Creates a new community with a random ID, neutral luck (`0`), and no members.
    pub fn new() -> Self {
        Self {
            id: CommunityId::new(),
            _luck: 0,
            members: HashMap::new(),
            version: SequenceId::zero(),
        }
    }

    /// Overrides the ID. Useful when reconstituting a community from stored data.
    pub fn with_id(mut self, id: CommunityId) -> Self {
        self.id = id;
        self
    }

    /// Overrides the luck score.
    pub fn with_luck(mut self, luck: u8) -> Self {
        self._luck = luck;
        self
    }

    /// Overrides the luck score using a normalised value in `[0.0, 1.0]`.
    ///
    /// The value is scaled to the internal `u8` range and rounded. Note that
    /// `self.luck() == luck` is not guaranteed: `luck` may not be exactly
    /// representable as a `u8`, so a round-trip through this setter and
    /// [`luck`][Self::luck] may differ slightly.
    pub fn with_luck_f64(mut self, luck: f64) -> Self {
        self._luck = (luck * u8::MAX as f64).round() as u8;
        self
    }

    /// Overrides the version. Useful when reconstituting a community from stored data.
    pub fn with_version(mut self, version: SequenceId) -> Self {
        self.version = version;
        self
    }

    /// Returns this community's luck score normalised to `[0.0, 1.0]`.
    ///
    /// `0.0` is neutral luck; `1.0` is the maximum (`u8::MAX`).
    pub fn luck(&self) -> f64 {
        self._luck as f64 / u8::MAX as f64
    }

    /// Returns the raw internal luck value in `[0, 255]`.
    pub(crate) fn luck_raw(&self) -> u8 {
        self._luck
    }

    /// Adds `member` to the community. Returns `true` if the member was newly inserted,
    /// `false` if a member with the same ID was already present.
    pub fn add_member(&mut self, member: Member) -> bool {
        if self.members.contains_key(&member.id) {
            return false;
        }
        self.members.insert(member.id, member);
        true
    }

    /// Removes the member with the given `id`. Returns the removed [`Member`], or `None`
    /// if no such member existed.
    pub fn remove_member(&mut self, id: MemberId) -> Option<Member> {
        self.members.remove(&id)
    }

    /// Applies `effects` to this community in sequence, advancing [`version`][Self::version]
    /// to the sequence ID of the last applied effect.
    ///
    /// Effects must be supplied in ascending sequence-ID order. If `effects` is empty
    /// the community is unchanged.
    pub fn apply_effects(&mut self, effects: impl IntoIterator<Item = Effect>) {
        for effect in effects {
            effect.apply(self);
            self.version = effect.id;
        }
    }
}

/// Mean bag value across all members of `community`, or `0.0` if there are no members.
pub fn community_avg_bag_value(community: &Community) -> f64 {
    if community.members.is_empty() {
        return 0.0;
    }
    let total: f64 = community.members.values().map(|m| bag_value(&m.bag)).sum();
    total / community.members.len() as f64
}

impl Default for Community {
    fn default() -> Self {
        Self::new()
    }
}

impl HasCommunityId for Community {
    fn community_id(&self) -> CommunityId {
        self.id
    }
}

#[cfg(test)]
#[path = "community_tests.rs"]
mod tests;
