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
    /// Personal luck score; influences the rarity of fruits this member receives each tick.
    pub luck: f64,
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
        assert_eq!(
            member,
            Member {
                id: member.id,
                display_name: "Alice".to_string(),
                luck: 0.0,
                bag: Bag::new()
            }
        );
    }

    #[test]
    fn member_can_receive_fruit() {
        let mut member = Member::new("Bob");
        member.receive(GRAPES);
        assert_eq!(
            member,
            Member {
                id: member.id,
                display_name: "Bob".to_string(),
                luck: 0.0,
                bag: Bag::new().insert(GRAPES),
            }
        );
    }

    #[test]
    fn member_can_receive_multiple_fruits() {
        let mut member = Member::new("Bob");
        member.receive(GRAPES).receive(GRAPES).receive(RED_APPLE);

        assert_eq!(
            member,
            Member {
                id: member.id,
                display_name: "Bob".to_string(),
                luck: 0.0,
                bag: Bag::new().insert(GRAPES).insert(GRAPES).insert(RED_APPLE)
            }
        );
    }
}
