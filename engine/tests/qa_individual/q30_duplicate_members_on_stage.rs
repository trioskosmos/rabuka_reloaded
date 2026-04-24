// Q30: ステージに同じカードを2枚以上登場させることはできますか？
// Answer: はい、できます。カードナンバーが同じカード、カード名が同じカードであっても、2枚以上登場させることができます。

use crate::qa_individual::common::*;

#[test]
fn test_q30_duplicate_members_can_be_on_stage() {
    // Test: Multiple copies of the same member card can be on stage
    // This tests that the engine allows duplicate members on stage
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card with low cost
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0 && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have member card with cost <= 2");
    let member_card_id = get_card_id(member_card, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    // Add two copies of the same member to hand
    setup_player_with_hand(&mut player1, vec![member_card_id, member_card_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    let initial_hand_count = game_state.player1.hand.cards.len();
    let initial_stage_count = game_state.player1.stage.stage.iter().filter(|&&id| id != -1).count();
    
    // Play first copy to center
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(member_card_id),
        None,
        Some(rabuka_engine::zones::MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play first copy to center: {:?}", result);
    
    // Advance turn to allow playing to different area
    game_state.turn_number = 2;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    game_state.player1.areas_locked_this_turn.clear();
    
    // Play second copy to left side
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(member_card_id),
        None,
        Some(rabuka_engine::zones::MemberArea::LeftSide),
        Some(false),
    );
    
    // Should succeed - duplicates are allowed on stage
    assert!(result.is_ok(), "Should play second copy to left side: {:?}", result);
    
    // Verify both copies are on stage
    assert_eq!(game_state.player1.stage.stage.iter().filter(|&&id| id != -1).count(), initial_stage_count + 2,
        "Stage should have 2 more members");
    
    // Verify both positions have the same card
    assert_eq!(game_state.player1.stage.stage[1], member_card_id,
        "Center should have the member");
    assert_eq!(game_state.player1.stage.stage[0], member_card_id,
        "Left side should have the same member");
    
    // Verify hand is empty (both copies played)
    assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count - 2,
        "Hand should have 2 fewer cards");
}
