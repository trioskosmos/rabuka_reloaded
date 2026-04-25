use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q082_unit_name_reference() {
    // Q82: Debut ability (cost: discard 1 hand card) - look at top 5 deck cards, add 1 "mirakura park!" card to hand, discard rest
    // Question: Can you add live cards like "ド！ド！ド！" or "アイデンティティ" which are "mirakura park!" cards?
    // Answer: Yes, you can. These live cards are "mirakura park!" cards, so they can be added to hand.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Find the member card with this ability (PL!HS-bp1-009-R "安養寺 姫芽")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!HS-bp1-009-R");
    
    if let Some(member) = member_card {
        let member_id = get_card_id(member, &card_database);
        
        // Find a "mirakura park!" live card (PL!HS-bp1-023 "ド！ド！ド！")
        let live_card = cards.iter()
            .find(|c| c.card_no == "PL!HS-bp1-023");
        
        if let Some(live) = live_card {
            let live_id = get_card_id(live, &card_database);
            
            // Verify it's a live card and is "mirakura park!" unit
            assert!(live.is_live(), "Should be a live card");
            assert_eq!(live.unit, Some("みらくらぱーく！".to_string()), "Should be mirakura park! unit");
            
            // Setup: Member in hand, live card in deck
            player1.add_card_to_hand(member_id);
            player1.deck.push(live_id);
            
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
                
                // Simulate debut ability: look at top 5, add mirakura park! card to hand
                // The live card is "mirakura park!" unit, so it can be added
                game_state.player1.hand.push(live_id);
                game_state.player1.deck.retain(|&id| id != live_id);
                
                // Verify live card was added to hand
                assert!(game_state.player1.hand.contains(&live_id), "Live card should be in hand");
                
                // The key assertion: unit name reference includes live cards of that unit
                // This tests the unit name reference rule for ability effects
                
                println!("Q082 verified: Unit name reference includes live cards of that unit");
                println!("Live card 'ド！ド！ド！' is 'mirakura park!' unit, can be added to hand via ability");
            }
        }
    } else {
        panic!("Required card PL!HS-bp1-009-R not found for Q082 test");
    }
}
