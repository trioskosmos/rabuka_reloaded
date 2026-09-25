use rabuka_engine::zones::ResolutionZone;

#[test]
fn rule_5_8_swap_uses_exact_slots_and_moves_owners() {
    let mut zone = ResolutionZone::new();
    zone.add_card_for_owner(7, 1);
    zone.add_card_for_owner(7, 2);

    zone.swap_slots(0, 1).unwrap();

    assert_eq!(zone.cards.as_slice(), &[7, 7]);
    assert_eq!(zone.owners.as_slice(), &[2, 1]);
}

#[test]
fn rule_5_8_swap_preflight_failure_leaves_zone_unchanged() {
    let mut zone = ResolutionZone::new();
    zone.add_card_for_owner(11, 1);
    zone.add_card_for_owner(12, 2);

    assert!(zone.swap_slots(1, 2).is_err());
    assert_eq!(zone.cards.as_slice(), &[11, 12]);
    assert_eq!(zone.owners.as_slice(), &[1, 2]);

    assert!(zone.swap_slots(0, 0).is_err());
    assert_eq!(zone.cards.as_slice(), &[11, 12]);
    assert_eq!(zone.owners.as_slice(), &[1, 2]);

    zone.owners.pop();
    assert!(zone.swap_slots(0, 1).is_err());
    assert_eq!(zone.cards.as_slice(), &[11, 12]);
    assert_eq!(zone.owners.as_slice(), &[1]);
}
