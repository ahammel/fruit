use uuid::Uuid;

use crate::{bag::Bag, fruit::Fruit, id::UuidIdentifier};

/// Typed identifier for a [`Member`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemberId(Uuid);

impl UuidIdentifier for MemberId {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }

    fn as_uuid(&self) -> Uuid {
        self.0
    }
}

/// A participant in the game.
#[derive(Debug, PartialEq)]
pub struct Member {
    /// Unique identifier for this member.
    pub id: MemberId,
    /// The name shown to other members.
    pub display_name: String,
    luck: f64,
    /// The fruits currently held by this member.
    pub bag: Bag,
}

impl Member {
    /// Creates a new member with a random ID, an empty bag, and neutral luck (`0.0`).
    pub fn new(display_name: impl Into<String>) -> Self {
        Self {
            id: MemberId::new(),
            display_name: display_name.into(),
            luck: 0.0,
            bag: Bag::new(),
        }
    }

    /// Overrides the ID. Useful when reconstituting a member from stored data.
    pub fn with_id(mut self, id: MemberId) -> Self {
        self.id = id;
        self
    }

    /// Overrides the bag contents.
    pub fn with_bag(mut self, bag: Bag) -> Self {
        self.bag = bag;
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

    /// Returns this member's luck score.
    pub fn luck(&self) -> f64 {
        self.luck
    }

    /// Adds one instance of `fruit` to this member's bag.
    pub fn receive(&mut self, fruit: Fruit) -> &mut Self {
        self.bag = std::mem::take(&mut self.bag).insert(fruit);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fruit::{GRAPES, RED_APPLE};

    #[test]
    fn new_member_has_neutral_luck() {
        let member = Member::new("Alice");
        assert_eq!(member, Member::new("Alice").with_id(member.id));
    }

    #[test]
    fn member_can_receive_fruit() {
        let mut member = Member::new("Bob");
        member.receive(GRAPES);
        assert_eq!(
            member,
            Member::new("Bob")
                .with_id(member.id)
                .with_bag(Bag::new().insert(GRAPES))
        );
    }

    #[test]
    fn member_can_receive_multiple_fruits() {
        let mut member = Member::new("Bob");
        member.receive(GRAPES).receive(GRAPES).receive(RED_APPLE);
        assert_eq!(
            member,
            Member::new("Bob")
                .with_id(member.id)
                .with_bag(Bag::new().insert(GRAPES).insert(GRAPES).insert(RED_APPLE))
        );
    }

    #[test]
    #[should_panic(expected = "luck must be finite")]
    fn with_luck_rejects_infinite() {
        Member::new("Alice").with_luck(f64::INFINITY);
    }
}
