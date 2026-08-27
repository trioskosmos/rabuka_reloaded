use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;

#[test]
fn test_q258_himege_activate_no_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himege = game.id("PL!HS-bp6-014-R");

    // Setup: Himege in hand, no Megumi or Rurino on stage
    game.add_to_hand(himege);

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

    // 2. No members on stage gained a blade (stage is empty anyway)
    assert_eq!(game.state.mods.blade_modifiers.len(), 0);
}
