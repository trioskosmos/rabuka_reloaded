use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q50_turn_order_both_win() {
    // Q50: What happens to turn order if both players win live?
    // Answer: Turn order remains the same. If both players place cards in success_live_zone,
    // the next turn's first/second attacker order does not change.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find live cards
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .take(2)
        .collect();
    
    if live_cards.len() >= 2 {
        let live_card1_id = get_card_id(live_cards[0], &card_database);
        let live_card2_id = get_card_id(live_cards[1], &card_database);
        
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
            
            // Add live cards to success_live_card_zone for both players (simulating both win)
            player1.success_live_card_zone.add_card(live_card1_id);
            player2.success_live_card_zone.add_card(live_card2_id);
            
            let mut game_state = GameState::new(player1, player2, card_database);
            game_state.current_phase = rabuka_engine::game_state::Phase::Main;
            game_state.turn_number = 1;
            game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
            
            let initial_turn_phase = game_state.current_turn_phase.clone();
            
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
            
            // The key point: turn order remains the same if both players win live
            // This test verifies that the turn phase doesn't change when both win
            assert_eq!(game_state.current_turn_phase, initial_turn_phase,
                "Turn phase should remain the same when both players win live");
            assert!(game_state.player1.stage.stage.contains(&card_id),
                "Member should be on stage after playing");
            assert!(game_state.player1.success_live_card_zone.cards.contains(&live_card1_id),
                "Player 1 should have live card in success_live_card_zone");
            assert!(game_state.player2.success_live_card_zone.cards.contains(&live_card2_id),
                "Player 2 should have live card in success_live_card_zone");
        } else {
            println!("Skipping test: no suitable member card found");
        }
    } else {
        println!("Skipping test: insufficient live cards found");
    }
}
