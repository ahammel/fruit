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
