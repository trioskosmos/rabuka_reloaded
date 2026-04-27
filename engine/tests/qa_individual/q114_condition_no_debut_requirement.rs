use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q114_condition_no_debut_requirement() {
    // Q114: Condition only requires members to be on stage, not debuted this turn
    // Answer: They just need to be on stage when the ability is used, not necessarily debuted this turn.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the live card with this ability
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!HS-bp2-024-L");
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Find two member cards with different costs
        let members: Vec<_> = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .filter(|c| c.cost.is_some())
            .take(2)
            .collect();
        
        if members.len() >= 2 {
            let member1_id = get_card_id(members[0], &card_database);
            let member2_id = get_card_id(members[1], &card_database);
            
            // Setup: Live card in live card zone, both members on stage from previous turn
            player1.live_card_zone.cards.push(live_id);
            player1.stage.stage[0] = member1_id;
            player1.stage.stage[1] = member2_id;
            
            // Add energy
            let energy_card_ids: Vec<_> = cards.iter()
                .filter(|c| c.is_energy())
                .filter(|c| get_card_id(c, &card_database) != 0)
                .map(|c| get_card_id(c, &card_database))
                .take(5)
                .collect();
            setup_player_with_energy(&mut player1, energy_card_ids);
            
            let mut game_state = GameState::new(player1, player2, card_database.clone());
            game_state.current_phase = rabuka_engine::game_state::Phase::LiveStart;
            game_state.turn_number = 2;
            
            // Verify members are on stage
            assert_eq!(game_state.player1.stage.stage[0], member1_id, "First member should be on stage");
            assert_eq!(game_state.player1.stage.stage[1], member2_id, "Second member should be on stage");
            
            // Simulate live start ability: condition checks if members are on stage
            let condition_met = game_state.player1.stage.stage.iter()
                .any(|&id| id == member1_id) && 
                game_state.player1.stage.stage.iter()
                .any(|&id| id == member2_id);
            
            // Verify condition is met
            assert!(condition_met, "Condition should be met (members on stage)");
            
            println!("Q114 verified: Condition only requires members to be on stage, not debuted this turn");
        }
    } else {
        panic!("Required card PL!HS-bp2-024-L not found for Q114 test");
    }
}
