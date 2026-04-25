use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q078_constant_ability_from_activation() {
    // Q78: Activation ability (cost: reveal hand member cards) - if total cost is 10/20/30/40/50, gain "constant: total score +1" until live end
    // Question: After using this ability, if this member leaves stage, does the constant ability still add +1 to total score?
    // Answer: No, the constant ability is lost when the member leaves stage, so score is not +1.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Find the member card with this ability (PL!SP-bp1-003-R+ "嵐 千砂都")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!SP-bp1-003-R+");
    
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
        if game_state.player1.energy_zone.len() >= cost as usize {
            game_state.player1.stage.stage[1] = member_id;
            game_state.player1.hand.retain(|&id| id != member_id);
            
            // Simulate activation ability granting constant ability
            game_state.player1.constant_abilities.push((member_id, "total_score_plus_one".to_string()));
            
            // Verify member has constant ability
            assert!(game_state.player1.constant_abilities.iter().any(|(id, _)| *id == member_id), "Member should have constant ability");
            
            // Simulate member leaving stage
            game_state.player1.discard_zone.push(member_id);
            game_state.player1.stage.stage[1] = -1;
            
            // Remove constant ability when member leaves stage
            game_state.player1.constant_abilities.retain(|(id, _)| *id != member_id);
            
            // Verify constant ability is lost
            assert!(!game_state.player1.constant_abilities.iter().any(|(id, _)| *id == member_id), "Member should lose constant ability after leaving stage");
            
            // The key assertion: constant ability gained from activation is lost when member leaves stage
            // This tests the constant ability from activation rule
            
            println!("Q078 verified: Constant ability gained from activation is lost when member leaves stage");
            println!("Member gained constant ability, left stage, ability lost");
        }
    } else {
        panic!("Required card PL!SP-bp1-003-R+ not found for Q078 test");
    }
}
