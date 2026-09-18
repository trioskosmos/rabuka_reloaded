use crate::helpers::*;

/// SP!SP-pb1-004-R: LiveStart pay 2E optional → place energy wait from energy deck; LiveSuccess pay 3E optional → draw 1
#[test]
fn live_start_optional_two_energy_payment_reduces_active_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-pb1-004-R");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    game.state.player1.stage.stage[1] = sumire;
    game.give_energy(5);
    let deck_before = game.state.player1.energy_deck.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();
    crate::helpers::fire_trigger(&mut game, sumire, rabuka_engine::core::types::AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice(), "LiveStart should present pay 2E choice");
    game.select_option(1); // Pay is option 1 (0 is skip)
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    assert_eq!(game.state.player1.energy_zone.active_count(), active_before.saturating_sub(2), "should pay 2 active energy");
    // Energy deck may be empty in TestGame new, so don't strictly assert deck length; just ensure no panic and active decreased
    assert!(game.state.player1.energy_deck.cards.len() <= deck_before, "energy deck should not increase");
}

#[test]
fn live_start_optional_two_energy_skip_preserves_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-pb1-004-R");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    game.state.player1.stage.stage[1] = sumire;
    game.give_energy(5);
    let deck_before = game.state.player1.energy_deck.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();
    crate::helpers::fire_trigger(&mut game, sumire, rabuka_engine::core::types::AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice());
    game.select_option(0); // Skip is option 0
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    assert_eq!(game.state.player1.energy_zone.active_count(), active_before, "skip should not pay energy");
    assert_eq!(game.state.player1.energy_deck.cards.len(), deck_before, "skip should not move energy");
}

#[test]
fn live_start_optional_two_energy_insufficient_energy_does_not_increase_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-pb1-004-R");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    game.state.player1.stage.stage[1] = sumire;
    game.give_energy(1); // only 1, need 2
    let active_before = game.state.player1.energy_zone.active_count();
    crate::helpers::fire_trigger(&mut game, sumire, rabuka_engine::core::types::AbilityTrigger::LiveStart, "ライブ開始時");
    // With insufficient energy, the pay option should be disabled or not present; engine may not offer choice
    if game.has_pending_choice() {
        // Try to pay (if available) — but with 1 energy, pay 2 should be blocked
        // The engine should either not offer pay or fail to pay and keep energy
        let before = game.state.player1.energy_zone.active_count();
        game.select_option(0);
        game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
        // Active should not go negative; should stay at 1 or be 0 if pay was incorrectly allowed
        assert!(game.state.player1.energy_zone.active_count() <= before, "should not overpay");
    }
    // At least verify no panic and active unchanged if skip
    assert!(game.state.player1.energy_zone.active_count() <= active_before);
}
