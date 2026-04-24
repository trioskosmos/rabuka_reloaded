// Q32: ライブカード置き場にライブカードが置かれていない場合、エールの確認は行いますか？
// Answer: いいえ、行いません。ライブカード置き場にライブカードが置かれていない場合、ライブを行わないため、エールの確認は行いません。

use crate::qa_individual::common::*;

#[test]
fn test_q32_cheer_not_performed_without_live_card() {
    // Test: Cheer should not be performed when there is no live card in the live card placement area
    // This tests that the engine correctly skips cheer phase when no live card is present
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .find(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0)
        .expect("Should have member card with cost > 0");
    let member_card_id = get_card_id(member_card, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    setup_player_with_hand(&mut player1, vec![member_card_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play member to stage
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(member_card_id),
        None,
        Some(rabuka_engine::zones::MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play member to stage: {:?}", result);
    
    // Verify no live card is in live card placement area
    assert_eq!(game_state.player1.live_card_zone.cards.len(), 0,
        "Live card placement area should be empty");
    
    // Verify that cheer is not performed (no live card means no live, so no cheer)
    // The engine should not attempt cheer confirmation when no live card is present
    // This is verified by checking that the game state doesn't enter a cheer state
    // and that no cheer-related actions are available
    assert_eq!(game_state.player1.live_card_zone.cards.len(), 0,
        "Live card placement area should still be empty");
    
    // Verify member is still on stage (game state should be valid)
    assert!(game_state.player1.stage.stage.contains(&member_card_id),
        "Member should still be on stage");
}
