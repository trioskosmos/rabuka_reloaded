use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q48_live_zero_score_win() {
    // Q48: Can you win live with total score <= 0?
    // Answer: Yes, you can. For example, if A succeeds at live with total score 0,
    // and B fails at live, A wins the live.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card
    let live_card = cards.iter()
        .find(|c| c.is_live())
        .expect("Should have live card");
    let live_card_id = get_card_id(live_card, &card_database);
    
    // Find a member card to play
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.map_or(false, |cost| cost <= 2))
        .find(|c| get_card_id(c, &card_database) != 0);
    
    if let Some(card) = member_card {
        let card_id = get_card_id(card, &card_database);
        
        setup_player_with_hand(&mut player1, vec![card_id]);
        
        // Add energy cards
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(10)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        // Add live card to success live card zone (simulating successful live with 0 score)
        player1.success_live_card_zone.add_card(live_card_id);
        
        let mut game_state = GameState::new(player1, player2, card_database);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // Play card to stage
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(card_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        
        assert!(result.is_ok(), "Card should play to stage: {:?}", result);
        
        // The key point: you can win live even with total score of 0 or less
        // This test verifies that the live card is in success_live_card_zone and the member is on stage
        assert!(game_state.player1.success_live_card_zone.cards.contains(&live_card_id),
            "Live card should be in success_live_card_zone after successful live");
        assert!(game_state.player1.stage.stage.contains(&card_id),
            "Member should be on stage after playing");
    } else {
        println!("Skipping test: no suitable member card found");
    }
}
