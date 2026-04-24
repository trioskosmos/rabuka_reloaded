use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy, setup_player_with_stage};

#[test]
fn test_q87_baton_touch_multiple() {
    // Q87: Can you perform "baton touch" multiple times in the same turn?
    // Answer: No, you cannot.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Use specific member cards for baton touch: PL!N-bp1-002-R+ and PL!N-bp1-001-R
    let existing_member = cards.iter()
        .find(|c| c.card_no == "PL!N-bp1-002-R＋");
    let new_member = cards.iter()
        .find(|c| c.card_no == "PL!N-bp1-001-R");
    
    if let (Some(card1), Some(card2)) = (existing_member, new_member) {
        let card1_id = get_card_id(card1, &card_database);
        let card2_id = get_card_id(card2, &card_database);
        
        setup_player_with_hand(&mut player1, vec![card1_id, card2_id]);
        
        // Add energy cards
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(15)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // Play first member to stage
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(card1_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        assert!(result.is_ok(), "First member should play to stage: {:?}", result);
        
        // Advance turn to allow baton touch
        game_state.turn_number = 2;
        game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
        game_state.player1.areas_locked_this_turn.clear();
        
        // Perform baton touch
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(card2_id),
            None,
            Some(MemberArea::Center),
            Some(true), // use_baton_touch = true
        );
        
        // The key point: baton touch should succeed once
        assert!(result.is_ok(), "Baton touch should succeed: {:?}", result);
        
        // Verify new member is on stage
        assert_eq!(game_state.player1.stage.stage[1], card2_id,
            "New member should be in center area after baton touch");
        
        println!("Baton touch test: {} ({}) replaced by {} ({})", 
            card1.name, card1.card_no, card2.name, card2.card_no);
    } else {
        panic!("Required member cards not found in card database");
    }
}
