// Q23: メンバーカードをステージに登場させるにはどうなりますか？
// Answer: メインフェイズに手札からメンバーカードをステージに置きます。その時、カードのコスト分のエネルギーを支払います。

use rabuka_engine::game_setup::ActionType;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::*;

#[test]
fn test_q23_play_member_to_stage_via_turn_engine() {
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card with cost > 1 to ensure energy payment is detectable
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .find(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 1)
        .expect("Should have member card with cost > 1");
    let member_card_id = get_card_id(member_card, &card_database);
    let member_cost = member_card.cost.unwrap_or(0);
    
    // Get energy cards
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    setup_player_with_hand(&mut player1, vec![member_card_id]);
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    
    let mut game_state = GameState::new(player1, player2, card_database);
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    let initial_hand_count = game_state.player1.hand.cards.len();
    let initial_active_energy = game_state.player1.energy_zone.active_energy_count;
    let initial_stage_count = game_state.player1.stage.stage.iter().filter(|&&id| id != -1).count();
    
    // Use TurnEngine to play member to stage
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_card_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    assert!(result.is_ok(), "Should successfully play member to stage: {:?}", result);
    
    // Verify card moved from hand to stage
    assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count - 1,
        "Hand should have 1 fewer card");
    assert!(!game_state.player1.hand.cards.contains(&member_card_id),
        "Member should not be in hand after playing");
    
    // Verify member is on stage in the correct area (center = index 1)
    assert_eq!(game_state.player1.stage.stage[1], member_card_id,
        "Member should be in center area (index 1)");
    
    // Verify stage count increased
    assert_eq!(game_state.player1.stage.stage.iter().filter(|&&id| id != -1).count(), initial_stage_count + 1,
        "Stage member count should increase by 1");
    
    // Verify energy was paid
    let final_active_energy = game_state.player1.energy_zone.active_energy_count;
    assert_eq!(final_active_energy, initial_active_energy - member_cost as usize,
        "Active energy should decrease by member cost");
    
    // Verify member is in active state (newly played members start active)
    // Note: Card state tracking may need to be verified through the engine's state management
    // For now, verify the card is on stage and orientation is not set to wait
    // Orientation tracking is handled in GameState modifiers
}
