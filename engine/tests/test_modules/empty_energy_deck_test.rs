/// Test: Empty energy deck during energy phase does NOT trigger a win.
/// Game should continue normally to the next phase as if nothing happened.
use crate::helpers::*;
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::types::GameResult;
use rabuka_engine::types::TurnPhase;

#[test]
fn empty_energy_deck_continues_game() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");

    // Ensure main deck has cards so draw_card won't fail
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Make Player 1's energy deck empty
    game.state.player1.energy_deck.cards.clear();

    // Add some energy to energy zone so stage can function
    game.give_energy(5);

    // Set up to start at Active phase (auto-advances: Active → Energy → Draw → Main)
    game.state.current_phase = Phase::Active;
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.turn_number = 3;

    // Sanity check: energy deck is empty
    assert!(game.state.player1.energy_deck.is_empty());

    // Advance Active → Energy
    TurnEngine::advance_phase(&mut game.state);
    assert_eq!(game.state.current_phase, Phase::Energy);
    assert!(!game.state.game_ended, "Game should NOT end from empty energy deck");
    assert_eq!(game.state.game_result, GameResult::Ongoing);

    // Advance Energy → Draw (empty energy deck, no state change -> no false loop)
    TurnEngine::advance_phase(&mut game.state);
    assert_eq!(game.state.current_phase, Phase::Draw);
    assert!(!game.state.game_ended, "Draw phase after empty energy deck should be fine");

    // Advance Draw → Main
    TurnEngine::advance_phase(&mut game.state);
    assert_eq!(game.state.current_phase, Phase::Main);
    assert!(!game.state.game_ended, "Game should reach Main without game_ended");
    assert_eq!(game.state.game_result, GameResult::Ongoing);

    // The turn should complete normally
    assert_eq!(game.state.turn_number, 3);
    assert_eq!(game.state.current_turn_phase, TurnPhase::FirstAttackerNormal);
}

#[test]
fn empty_energy_deck_player2_continues_game() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Add 1 energy to P1's energy deck so it's not also empty
    let energy_card = game.id("LL-E-001-SD");
    game.state.player1.energy_deck.cards.push(energy_card);

    // Make P2's energy deck empty
    game.state.player2.energy_deck.cards.clear();
    game.give_energy(5);

    game.state.current_phase = Phase::Active;
    game.state.current_turn_phase = TurnPhase::SecondAttackerNormal;
    game.state.turn_number = 3;

    assert!(game.state.player2.energy_deck.is_empty());
    assert!(!game.state.player1.energy_deck.is_empty());

    // Advance through P2's Active → Energy → Draw → Main
    TurnEngine::advance_phase(&mut game.state);
    assert_eq!(game.state.current_phase, Phase::Energy);
    assert!(!game.state.game_ended, "P2 empty energy deck should NOT end game");

    TurnEngine::advance_phase(&mut game.state);
    assert_eq!(game.state.current_phase, Phase::Draw);
    assert!(!game.state.game_ended);

    TurnEngine::advance_phase(&mut game.state);
    assert_eq!(game.state.current_phase, Phase::Main);
    assert!(!game.state.game_ended, "P2 should reach Main without game_ended");
    assert_eq!(game.state.game_result, GameResult::Ongoing);

    // P2's turn should complete into Live
    TurnEngine::advance_phase(&mut game.state);
    assert!(!game.state.game_ended);
    assert_eq!(game.state.current_turn_phase, TurnPhase::Live);
}
