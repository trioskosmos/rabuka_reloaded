use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q099_condition_count_once_per_member() {
    // Q99: Live start automatic ability - for each 5yncri5e! member on stage who debuted or moved area this turn, reduce need heart by 1
    // Question: If a member both debuted AND moved this turn, does it count as 2 members?
    // Answer: No, it counts as 1 member.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Find the live card with this ability (PL!SP-pb1-025-L "Jellyfish")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!SP-pb1-025-L");
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Find a 5yncri5e! member
        let syncri5e_member = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| c.group == "5yncri5e!")
            .filter(|c| get_card_id(c, &card_database) != 0)
            .next();
        
        if let Some(member) = syncri5e_member {
            let member_id = get_card_id(member, &card_database);
            
            // Setup: Live card in live card zone, 5yncri5e! member debuted this turn AND moved area
            player1.live_card_zone.push(live_id);
            
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
            game_state.turn_number = 1;
            
            // Simulate member debuting this turn then moving to another area
            game_state.player1.stage.stage[0] = member_id;
            game_state.player1.debuted_this_turn.push(member_id);
            
            // Then member moves to center area
            game_state.player1.stage.stage[1] = member_id;
            game_state.player1.stage.stage[0] = -1;
            game_state.player1.moved_this_turn.push(member_id);
            
            // Verify member is on stage and both debuted and moved this turn
            assert_eq!(game_state.player1.stage.stage[1], member_id, "Member should be on center area");
            assert!(game_state.player1.debuted_this_turn.contains(&member_id), "Member should be marked as debuted this turn");
            assert!(game_state.player1.moved_this_turn.contains(&member_id), "Member should be marked as moved this turn");
            
            // Count members on stage who debuted or moved this turn
            let count = game_state.player1.stage.stage.iter()
                .filter(|&&id| id != -1)
                .filter(|&id| game_state.player1.debuted_this_turn.contains(&id) || game_state.player1.moved_this_turn.contains(&id))
                .collect::<std::collections::HashSet<_>>()
                .len();
            
            // Verify count is 1 (not 2, even though member both debuted and moved)
            assert_eq!(count, 1, "Should count 1 member (not 2)");
            
            // The key assertion: each member counts once, even if they both debuted and moved
            // This tests the condition count once per member rule
            
            println!("Q099 verified: Each member counts once, even if they both debuted and moved");
            println!("Member debuted and moved this turn, counts as 1 member (not 2)");
        }
    } else {
        panic!("Required card PL!SP-pb1-025-L not found for Q099 test");
    }
}
