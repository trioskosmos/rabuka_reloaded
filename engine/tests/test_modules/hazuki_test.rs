/// Tests for 葉月 恋 (PL!SP-bp5-005-R＋) ab#0
///
/// ab#0 (起動/ターン1): Cost: discard 3 from deck top.
///   Until live end, for each discarded Liella! member, gain 1 blade.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// ab#0: Activate, discard 3 from deck. 2 are Liella! members → gain 2 blade.
#[test]
fn hazuki_activate_two_liella_discarded_gain_2_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hazuki = game.id("PL!SP-bp5-005-R＋");
    let filler = game.id("PL!-sd1-010-SD");
    let liella = game.id("PL!SP-bp1-004-R");

    game.state.player1.stage.stage[1] = hazuki;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(liella);
    game.state.player1.main_deck.cards.push(liella);
    game.state.player1.main_deck.cards.push(filler);
    game.give_energy(13);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(hazuki),
        None,
        None,
        None,
    )
    .expect("activate");

    let blade_val = game
        .state
        .mods
        .blade_modifiers
        .get(&hazuki)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        blade_val, 2,
        "2 Liella! members discarded → 2 blade on activating card"
    );
}

/// ab#0: 0 Liella! members discarded → 0 blade gained.
#[test]
fn hazuki_activate_no_liella_discarded_gain_0_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hazuki = game.id("PL!SP-bp5-005-R＋");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = hazuki;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(13);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(hazuki),
        None,
        None,
        None,
    )
    .expect("activate");

    let blade_val = game
        .state
        .mods
        .blade_modifiers
        .get(&hazuki)
        .copied()
        .unwrap_or(0);
    assert_eq!(blade_val, 0, "0 Liella! members discarded → 0 blade");
}

/// ab#0: Not enough cards in deck — cost fails, ability is not applied.
#[test]
fn hazuki_activate_not_enough_deck_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hazuki = game.id("PL!SP-bp5-005-R＋");

    game.state.player1.stage.stage[1] = hazuki;
    game.state.player1.main_deck.cards.clear(); // 0 cards
    game.give_energy(13);

    let _ = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(hazuki),
        None,
        None,
        None,
    );

    let blade_val = game
        .state
        .mods
        .blade_modifiers
        .get(&hazuki)
        .copied()
        .unwrap_or(0);
    assert_eq!(blade_val, 0, "Cost failed → no blade applied");
}
