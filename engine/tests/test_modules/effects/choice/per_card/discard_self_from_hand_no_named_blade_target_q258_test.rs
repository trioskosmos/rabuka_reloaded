use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;

#[test]
fn discard_self_from_hand_without_named_blade_target_q258() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himege = game.id("PL!HS-bp6-014-R");
    let drawn = game.id("PL!-sd1-010-SD");

    // Setup: Himege in hand, no Megumi or Rurino on stage
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(drawn);
    game.add_to_hand(himege);
    let hand_before = game.state.player1.hand.cards.len();


    // Action: Activate ability (this will move Himege to discard)
    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &rabuka_engine::game_setup::ActionType::UseAbility,
        Some(himege),
        None,
        None,
        None,
    );

    assert!(
        result.is_ok(),
        "Ability activation should succeed: {:?}",
        result.err()
    );

    // Verification:
    // 1. Himege is now in discard
    assert!(game.state.player1.waitroom.cards.contains(&himege));
    assert!(game.state.player1.hand.cards.contains(&drawn));
    assert_eq!(game.state.player1.hand.cards.len(), hand_before);

    // 2. No members on stage gained a blade (stage is empty anyway)
    assert_eq!(game.state.mods.blade_modifiers.len(), 0);
}
