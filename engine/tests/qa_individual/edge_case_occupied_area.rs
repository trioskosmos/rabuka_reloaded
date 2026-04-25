use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_edge_case_occupied_area() {
    // Edge case: Try to play a member to an already occupied area
    // Expected: Should fail or handle gracefully
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find two member cards
    let member1 = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have member card");
    let member1_id = get_card_id(member1, &card_database);
    
    let member2 = cards.iter()
        .filter(|c| c.is_member() && get_card_id(c, &card_database) != member1_id)
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have member card");
    let member2_id = get_card_id(member2, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(10)
        .collect();
    
    setup_player_with_hand(&mut player1, vec![member1_id, member2_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play first member to center
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member1_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play first member to center: {:?}", result);
    
    // Try to play second member to the same occupied center area
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member2_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    // Verify the play failed (area is occupied)
    assert!(result.is_err(), "Should fail to play to occupied area: {:?}", result);
    
    // Verify second card is still in hand
    assert!(game_state.player1.hand.cards.contains(&member2_id),
        "Second card should still be in hand when play fails");
    
    // Verify first card is still on stage
    assert_eq!(game_state.player1.stage.stage[1], member1_id,
        "First card should still be on stage");
    
    println!("Edge case test: Occupied area handled correctly");
}
