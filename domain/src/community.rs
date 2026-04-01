use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    id::UuidIdentifier,
    member::{Member, MemberId},
};

/// Typed identifier for a [`Community`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommunityId(Uuid);

impl UuidIdentifier for CommunityId {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }

    fn as_uuid(&self) -> Uuid {
        self.0
    }
}

/// A group of members that share a collective luck modifier.
#[derive(Debug, PartialEq)]
pub struct Community {
    /// Unique identifier for this community.
    pub id: CommunityId,
    luck: f64,
    /// Members belonging to this community, keyed by their ID.
    pub members: HashMap<MemberId, Member>,
}

impl Community {
    /// Creates a new community with a random ID, neutral luck (`0.0`), and no members.
    pub fn new() -> Self {
        Self {
            id: CommunityId::new(),
            luck: 0.0,
            members: HashMap::new(),
        }
    }

    /// Overrides the ID. Useful when reconstituting a community from stored data.
    pub fn with_id(mut self, id: CommunityId) -> Self {
        self.id = id;
        self
    }

    /// Overrides the luck score.
    ///
    /// # Panics
    ///
    /// Panics if `luck` is not finite.
    pub fn with_luck(mut self, luck: f64) -> Self {
        assert!(luck.is_finite(), "luck must be finite; got {luck}");
        self.luck = luck;
        self
    }

    /// Returns this community's luck score.
    pub fn luck(&self) -> f64 {
        self.luck
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
}

impl Default for Community {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::UuidIdentifier;

    #[test]
    fn new_community_is_empty() {
        let community = Community::new();
        assert_eq!(community, Community::new().with_id(community.id));
    }

    #[test]
    fn add_member_inserts_by_id() {
        let mut community = Community::new();
        let member = Member::new("Alice");
        let id = member.id;
        assert!(community.add_member(member));
        let mut expected = Community::new().with_id(community.id);
        expected.members = HashMap::from([(id, Member::new("Alice").with_id(id))]);
        assert_eq!(community, expected);
    }

    #[test]
    fn add_member_returns_false_on_duplicate() {
        let mut community = Community::new();
        let member = Member::new("Alice");
        let id = member.id;
        let duplicate = Member::new("Alice2").with_id(id);
        assert!(community.add_member(member));
        assert!(!community.add_member(duplicate));
        let mut expected = Community::new().with_id(community.id);
        expected.members = HashMap::from([(id, Member::new("Alice").with_id(id))]);
        assert_eq!(community, expected);
    }

    #[test]
    fn remove_member_returns_member() {
        let mut community = Community::new();
        let member = Member::new("Bob");
        let id = member.id;
        community.add_member(member);
        let removed = community.remove_member(id).unwrap();
        assert_eq!(removed, Member::new("Bob").with_id(id));
        assert_eq!(community, Community::new().with_id(community.id));
    }

    #[test]
    fn remove_member_returns_none_for_unknown_id() {
        let mut community = Community::new();
        assert!(community.remove_member(MemberId::new()).is_none());
    }

    #[test]
    #[should_panic(expected = "luck must be finite")]
    fn with_luck_rejects_infinite() {
        Community::new().with_luck(f64::INFINITY);
    }
}
