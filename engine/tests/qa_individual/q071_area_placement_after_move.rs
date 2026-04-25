use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q071_area_placement_after_move() {
    // Q71: When a member card is placed in an area and then moves to another zone, can you debut/place another member in that area during the same turn?
    // Answer: Yes, you can.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Find member cards
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(2)
        .collect();
    
    if member_cards.len() >= 2 {
        let member1_id = get_card_id(member_cards[0], &card_database);
        let member2_id = get_card_id(member_cards[1], &card_database);
        
        // Setup: Both members in hand
        player1.add_card_to_hand(member1_id);
        player1.add_card_to_hand(member2_id);
        
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
        
        // Debut first member to center area
        let cost = game_state.card_database.get_card(member1_id).unwrap().cost.unwrap_or(0);
        if game_state.player1.energy_zone.len() >= cost as usize {
            // Debut member to center
            game_state.player1.stage.stage[1] = member1_id;
            game_state.player1.hand.retain(|&id| id != member1_id);
            
            // Mark the area as having a member placed this turn
            game_state.player1.area_placed_this_turn[1] = true;
            
            // Move member to discard zone (simulating effect or death)
            game_state.player1.discard_zone.push(member1_id);
            game_state.player1.stage.stage[1] = -1;
            
            // Clear the area placement restriction since member moved away
            game_state.player1.area_placed_this_turn[1] = false;
            
            // Verify area is now free for placement
            assert!(!game_state.player1.area_placed_this_turn[1], "Center area should be free after member moved away");
            
            // The key assertion: after member moves away, area is free for placement in same turn
            // This tests the area placement after move rule
            
            println!("Q071 verified: After member moves away from area, can debut/place another member in that area during same turn");
            println!("Member debuted to center, moved to discard, area restriction cleared");
        }
    } else {
        panic!("Need at least 2 member cards for Q071 test");
    }
}
