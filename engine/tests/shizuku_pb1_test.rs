/// Tests for 桜坂しずく (PL!N-pb1-003-R) — Q196
///
/// 起動/2E: Discard 1 from hand → draw 1, until live end,
/// 1 にこ member on stage gains 1 blade.
///
/// Note: Group filter "にこ" requires engine name-match support.
/// Test validates activation + draw work; blade gain is a known engine gap.
mod helpers;
use helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::game_setup::ActionType;

/// Activate ability: pay 2E + discard 1 → draw 1.
#[test]
fn shizuku_q196_draw_after_discard_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-pb1-003-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, filler, -1];
    game.state.player1.hand.cards.push(shizuku);
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }
    game.give_energy(15);

    let hand_before = game.state.player1.hand.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::UseAbility,
        Some(shizuku), None, None, None,
    ).expect("activate ability");

    while game.has_pending_choice() { game.select_indices(&[1]); }

    // Hand: start=2, cost discards 1 → 1, draw 1 → 2
    assert_eq!(game.state.player1.hand.cards.len(), hand_before,
        "Q196: Cost discards 1, draw adds 1 → net zero");
    assert!(game.state.player1.hand.cards.contains(&shizuku),
        "Shizuku stays in hand (filler was discarded)");
}
