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
    /// Community-wide luck score; stacks with individual member luck.
    pub luck: f64,
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
    use crate::{bag::Bag, id::UuidIdentifier};

    #[test]
    fn new_community_is_empty() {
        let community = Community::new();
        assert_eq!(
            community,
            Community {
                id: community.id,
                luck: 0.0,
                members: HashMap::new(),
            }
        );
    }

    #[test]
    fn add_member_inserts_by_id() {
        let mut community = Community::new();
        let member = Member::new("Alice");
        let id = member.id;
        assert!(community.add_member(member));
        assert_eq!(
            community,
            Community {
                id: community.id,
                luck: 0.0,
                members: HashMap::from([(
                    id,
                    Member {
                        id,
                        display_name: "Alice".to_string(),
                        luck: 0.0,
                        bag: Bag::new()
                    }
                )]),
            }
        );
    }

    #[test]
    fn add_member_returns_false_on_duplicate() {
        let mut community = Community::new();
        let member = Member::new("Alice");
        let id = member.id;
        let duplicate = Member {
            id,
            display_name: "Alice2".to_string(),
            luck: 0.0,
            bag: Bag::new(),
        };
        assert!(community.add_member(member));
        assert!(!community.add_member(duplicate));
        assert_eq!(
            community,
            Community {
                id: community.id,
                luck: 0.0,
                members: HashMap::from([(
                    id,
                    Member {
                        id,
                        display_name: "Alice".to_string(),
                        luck: 0.0,
                        bag: Bag::new()
                    }
                )]),
            }
        );
    }

    #[test]
    fn remove_member_returns_member() {
        let mut community = Community::new();
        let member = Member::new("Bob");
        let id = member.id;
        community.add_member(member);
        let removed = community.remove_member(id).unwrap();
        assert_eq!(
            removed,
            Member {
                id,
                display_name: "Bob".to_string(),
                luck: 0.0,
                bag: Bag::new()
            }
        );
        assert_eq!(
            community,
            Community {
                id: community.id,
                luck: 0.0,
                members: HashMap::new()
            }
        );
    }

    #[test]
    fn remove_member_returns_none_for_unknown_id() {
        let mut community = Community::new();
        assert!(community.remove_member(MemberId::new()).is_none());
    }
}
