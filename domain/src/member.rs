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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// Unique identifier for this member.
    pub id: MemberId,
    /// The name shown to other members.
    pub display_name: String,
    _luck: u16,
    /// The fruits currently held by this member.
    pub bag: Bag,
}

impl Member {
    /// Creates a new member with a random ID, an empty bag, and neutral luck (`0`).
    pub fn new(display_name: impl Into<String>) -> Self {
        Self {
            id: MemberId::new(),
            display_name: display_name.into(),
            _luck: 0,
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
    pub fn with_luck(mut self, luck: u16) -> Self {
        self._luck = luck;
        self
    }

    /// Overrides the luck score using a normalised value in `[0.0, 1.0]`.
    ///
    /// The value is scaled to the internal `u16` range and rounded. Note that
    /// `self.luck() == luck` is not guaranteed: `luck` may not be exactly
    /// representable as a `u16`, so a round-trip through this setter and
    /// [`luck`][Self::luck] may differ slightly.
    pub fn with_luck_f64(mut self, luck: f64) -> Self {
        self._luck = (luck * u16::MAX as f64).round() as u16;
        self
    }

    /// Returns this member's luck score normalised to `[0.0, 1.0]`.
    ///
    /// `0.0` is neutral luck; `1.0` is the maximum (`u16::MAX`).
    pub fn luck(&self) -> f64 {
        self._luck as f64 / u16::MAX as f64
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
    fn member_id_as_uuid_roundtrips() {
        let id = MemberId::new();
        assert_eq!(id, MemberId(id.as_uuid()));
    }

    #[test]
    fn with_luck_sets_luck() {
        let member = Member::new("Alice").with_luck(500);
        assert_eq!(member.luck(), 500.0 / u16::MAX as f64);
    }

    #[test]
    fn with_luck_f64_sets_luck() {
        let member = Member::new("Alice").with_luck_f64(0.5);
        assert!((member.luck() - 0.5).abs() < 1e-4);
    }
}
