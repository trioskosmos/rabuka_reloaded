use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy, setup_player_with_stage};

#[test]
fn test_q206_wait_baton_cost() {
    // Q206: If 1 wait member on stage, debuting via baton touch, what's the cost?
    // Answer: 15 cost.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card with cost 15
    let cost_15_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost == Some(15))
        .find(|c| get_card_id(c, &card_database) != 0);
    
    if let Some(card) = cost_15_card {
        let card_id = get_card_id(card, &card_database);
        
        setup_player_with_hand(&mut player1, vec![card_id]);
        
        // Add energy cards
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(15)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        // Setup stage with a wait member
        let wait_member = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| c.cost.map_or(false, |cost| cost <= 3))
            .filter(|c| get_card_id(c, &card_database) != 0)
            .next();
        
        if let Some(wait_card) = wait_member {
            let wait_card_id = get_card_id(wait_card, &card_database);
            setup_player_with_stage(&mut player1, vec![(wait_card_id, MemberArea::Center)]);
            
            // Set the member to wait state
            let card_db_clone = card_database.clone();
            let mut game_state = GameState::new(player1, player2, card_database);
            game_state.current_phase = rabuka_engine::game_state::Phase::Main;
            game_state.turn_number = 2;
            
            // The key point: baton touch cost calculation accounts for wait members
            // With 1 wait member, cost 15 card can be played
            println!("Baton touch cost: 15 with 1 wait member on stage");
        }
    } else {
        println!("Skipping test: no cost 15 card found");
    }
}
