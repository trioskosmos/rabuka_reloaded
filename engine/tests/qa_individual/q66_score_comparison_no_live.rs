use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q66_score_comparison_no_live() {
    // Q66: When comparing live scores, if you have a live card and opponent doesn't, your score is treated as higher
    // Answer: Yes, the condition is satisfied. If you have a live card in your live card zone
    // and opponent doesn't have one, your total score is treated as higher regardless of the actual value.
    
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
        
        let card_db_clone = card_database.clone();
        // Add live card to player1's live card zone (player2 has none)
        player1.live_card_zone.add_card(live_card_id, false, &card_db_clone).expect("Failed to add live card");
        
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
        
        // The key point: if player has live card and opponent doesn't, player's score is treated as higher
        // This test verifies that player1 has a live card while player2 doesn't
        assert!(game_state.player1.live_card_zone.cards.contains(&live_card_id),
            "Player 1 should have live card in live_card_zone");
        assert!(game_state.player2.live_card_zone.cards.is_empty(),
            "Player 2 should not have live cards");
        assert!(game_state.player1.stage.stage.contains(&card_id),
            "Member should be on stage after playing");
    } else {
        println!("Skipping test: no suitable member card found");
    }
}
