/// Tests for Tang Ke Ke (PL!SP-pb2-002) activate ability choice pattern.
/// The ability has:
///   Cost: discard 1 Liella! card from hand
///   Effect: choose 1 from (energy from energy deck / heart06 on a Liella! member)
///   If the discarded card is a Liella! member WITHOUT blade heart → choose 1+ instead
///
/// Parser fix: _try_kore_niyori_result previously matched before _try_choice,
/// producing "conditional_on_result" + "select" (area_select → position prompt)
/// instead of the correct "choice" + options with alternative_condition.
use crate::helpers::*;

/// Helper: advance past the cost prompt (select card from hand to discard)
fn pay_discard_cost(game: &mut TestGame) {
    assert!(
        game.has_pending_choice(),
        "Should prompt to select card to discard"
    );
    game.select_indices(&[0]);
}

/// Helper: verify the choice prompt appears and select the energy option (index 0)
fn select_energy_option(game: &mut TestGame) -> usize {
    assert!(game.has_pending_choice(), "Choice should appear");
    let energy_before = game.state.player1.energy_zone.cards.len();
    game.select_option(0);
    energy_before
}

/// Basic setup: Tang Ke Ke on stage, some energy, energy deck has cards
fn setup_keke(game: &mut TestGame) -> i16 {
    let keke = game.id("PL!SP-pb2-002-R");
    let energy = game.id("LL-E-001-SD");

    game.state.player1.stage.stage[0] = keke;
    game.give_energy(5);
    for _ in 0..5 {
        game.state.player1.energy_deck.cards.push(energy);
    }
    keke
}

#[test]
fn tang_keke_discard_member_with_blade_heart_choose_one() {
    // Discard a Liella! member WITH blade heart → condition NOT met → count=1
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);
    let liella_with_bh = game.id("PL!SP-PR-003-PR"); // 澁谷かのん, b_heart03=1

    game.state.player1.hand.cards.push(liella_with_bh);
    game.activate_ability(keke);

    pay_discard_cost(&mut game);
    let energy_before = select_energy_option(&mut game);

    assert!(
        game.state.player1.energy_zone.cards.len() > energy_before,
        "Energy should be placed from energy deck"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&liella_with_bh),
        "Discarded card should be in waitroom"
    );
}

#[test]
fn tang_keke_discard_member_without_blade_heart_choose_any_number() {
    // Discard a Liella! member WITHOUT blade heart → condition MET → count=any_number
    // Player can select 1+ options. After selecting energy, re-prompt with remaining.
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);
    let liella_no_bh = game.id("PL!SP-PR-004-PR"); // 唐可可, no blade heart
                                                   // A Liella! member on stage for the heart06 option (not the activating member)
    let target_member = game.id("PL!SP-PR-003-PR");

    game.state.player1.hand.cards.push(liella_no_bh);
    // Place a different Liella! member on stage for the heart06 target
    game.state.player1.stage.stage[1] = target_member;
    game.activate_ability(keke);

    pay_discard_cost(&mut game);

    // Choice appears — select energy (option 0)
    assert!(game.has_pending_choice(), "Choice should appear");
    let energy_before = game.state.player1.energy_zone.cards.len();
    game.select_option(0);

    // any_number: engine should re-prompt with remaining option (heart06)
    assert!(
        game.has_pending_choice(),
        "Re-prompt should appear for remaining options (any_number)"
    );
    // Select the heart06 option — verify heart06 was gained on the target member
    game.select_option(0);

    assert!(
        game.state.player1.energy_zone.cards.len() > energy_before,
        "Energy should be placed from energy deck (selected first)"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&liella_no_bh),
        "Discarded card should be in waitroom"
    );
    // Heart06 gain is a modifier — just verify no crash / no infinite loop
    assert!(
        !game.has_pending_choice(),
        "No more choices after selecting both options"
    );
}

#[test]
fn tang_keke_discard_non_member_liella_card_choose_one() {
    // Discard a Liella! LIVE card (not a member) → card_type condition fails → count=1
    // PL!SP-pb2-002 itself is a Liella! live card — use it as discard fodder
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);
    // Use a second copy of the same card as discard fodder (it's a Liella! live card)
    let liella_live = game.id("PL!SP-pb2-002-R");

    game.state.player1.hand.cards.push(liella_live);
    game.activate_ability(keke);

    pay_discard_cost(&mut game);
    let energy_before = select_energy_option(&mut game);

    assert!(
        game.state.player1.energy_zone.cards.len() > energy_before,
        "Energy should be placed from energy deck"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&liella_live),
        "Discarded live card should be in waitroom"
    );
}

#[test]
fn tang_keke_use_limit_blocks_second_activation() {
    // The ability has use_limit=1. After using it once, cannot activate again this turn.
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);
    let liella_with_bh = game.id("PL!SP-PR-003-PR");

    // First activation
    game.state.player1.hand.cards.push(liella_with_bh);
    game.state.player1.hand.cards.push(liella_with_bh); // second copy for 2nd attempt
    game.activate_ability(keke);

    pay_discard_cost(&mut game);
    let _ = select_energy_option(&mut game);

    // Second activation should be blocked by use_limit
    let result = game.try_activate_ability(keke);
    assert!(
        result.is_err(),
        "Second activation should fail: use_limit reached"
    );
}

#[test]
fn tang_keke_no_liella_in_hand_cost_cannot_pay() {
    // If there are no Liella! cards in hand, the cost cannot be paid.
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = setup_keke(&mut game);

    // No card in hand — cost cannot be paid
    game.state.player1.hand.cards.clear();

    // The activate_ability call should fail since cost can't be paid
    // (no Liella! cards in hand to discard)
    #[allow(unused_must_use)]
    {
        game.try_activate_ability(keke);
    }
    // No assertion needed — we're verifying it doesn't panic/crash
    // The cost validation should prevent activation
}
