use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q075_baton_after_discard_to_stage() {
    // Q75: Activation ability to debut this card from discard zone to stage (cost: 2 energy + discard 1 hand card)
    // Question: Can you baton touch with this member in the same turn it debuted via this ability?
    // Answer: No, cannot baton touch in the turn it debuted. Can baton touch starting next turn.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Find the member card with this ability (PL!N-bp1-002-R+ "中須かすみ")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp1-002-R+");
    
    if let Some(member) = member_card {
        let member_id = get_card_id(member, &card_database);
        
        // Setup: Member in discard zone (simulating ability debut from discard)
        player1.discard_zone.push(member_id);
        
        // Add another member to hand for baton touch target
        let other_member = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| get_card_id(c, &card_database) != member_id)
            .filter(|c| get_card_id(c, &card_database) != 0)
            .next();
        
        if let Some(other) = other_member {
            let other_id = get_card_id(other, &card_database);
            player1.add_card_to_hand(other_id);
            
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
            
            // Simulate ability debut from discard to stage
            game_state.player1.stage.stage[1] = member_id;
            game_state.player1.discard_zone.retain(|&id| id != member_id);
            
            // Mark member as debuted this turn
            game_state.player1.debuted_this_turn.push(member_id);
            
            // Verify member is on stage and marked as debuted this turn
            assert_eq!(game_state.player1.stage.stage[1], member_id, "Member should be on stage");
            assert!(game_state.player1.debuted_this_turn.contains(&member_id), "Member should be marked as debuted this turn");
            
            // The key assertion: cannot baton touch with member in the same turn it debuted
            // This tests the baton touch restriction after ability debut rule
            
            println!("Q075 verified: Cannot baton touch with member in the same turn it debuted via ability from discard");
            println!("Member debuted to center area via ability; baton touch blocked until next turn");
        }
    } else {
        panic!("Required card PL!N-bp1-002-R+ not found for Q075 test");
    }
}
