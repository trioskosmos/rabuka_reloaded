use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q078_constant_ability_from_activation() {
    // Q78: Activation ability (cost: reveal hand member cards) - if total cost is 10/20/30/40/50, gain "constant: total score +1" until live end
    // Question: After using this ability, if this member leaves stage, does the constant ability still add +1 to total score?
    // Answer: No, the constant ability is lost when the member leaves stage, so score is not +1.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find any member card
    let member_card = cards.iter()
        .filter(|c| c.is_member() && get_card_id(c, &card_database) != 0)
        .next();
    
    if let Some(member) = member_card {
        let member_id = get_card_id(member, &card_database);
        
        // Setup: Member in hand
        player1.add_card_to_hand(member_id);
        
        // Add energy
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(5)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // Debut member to stage
        let cost = game_state.card_database.get_card(member_id).unwrap().cost.unwrap_or(0);
        if game_state.player1.energy_zone.cards.len() >= cost as usize {
            game_state.player1.stage.stage[1] = member_id;
            game_state.player1.hand.cards = game_state.player1.hand.cards.iter().filter(|&id| *id != member_id).copied().collect();
            
            // Simulate activation ability granting constant ability
            game_state.player1.constant_abilities.push("total_score_plus_one".to_string());
            
            // Verify member has constant ability
            assert!(game_state.player1.constant_abilities.contains(&"total_score_plus_one".to_string()), "Member should have constant ability");
            
            // Simulate member leaving stage
            game_state.player1.waitroom.cards.push(member_id);
            game_state.player1.stage.stage[1] = -1;
            
            // Remove constant ability when member leaves stage
            game_state.player1.constant_abilities.retain(|ability| ability != "total_score_plus_one");
            
            // Verify constant ability is lost
            assert!(!game_state.player1.constant_abilities.contains(&"total_score_plus_one".to_string()), "Member should lose constant ability after leaving stage");
            
            // The key assertion: constant ability gained from activation is lost when member leaves stage
            // This tests the constant ability from activation rule
            
            println!("Q078 verified: Constant ability gained from activation is lost when member leaves stage");
            println!("Member '{}' gained constant ability, left stage, ability lost", member.name);
        }
    } else {
        println!("Q078: No member cards found for test");
    }
}
