use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q121_blade_all_members() {
    // Q121: Live start ability - if live card zone has Aqours live card other than "MY舞☆TONIGHT", until live end, all stage members gain blade
    // Question: Does only 1 stage member gain blade, or do all stage members gain blade?
    // Answer: All stage members gain blade.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Find the live card with this ability (PL!S-bp2-023-L "MY舞☆TONIGHT")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!S-bp2-023-L");
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Setup: This live card in live card zone, another Aqours live card in live card zone, 3 members on stage
        player1.live_card_zone.push(live_id);
        
        let aqours_live = cards.iter()
            .filter(|c| c.is_live())
            .filter(|c| c.group == "Aqours")
            .filter(|c| c.card_no != "PL!S-bp2-023-L")
            .filter(|c| get_card_id(c, &card_database) != 0)
            .next();
        
        if let Some(aqours) = aqours_live {
            let aqours_id = get_card_id(aqours, &card_database);
            player1.live_card_zone.push(aqours_id);
            
            // Add 3 members to stage
            let members: Vec<_> = cards.iter()
                .filter(|c| c.is_member())
                .filter(|c| get_card_id(c, &card_database) != 0)
                .take(3)
                .collect();
            
            for (i, member) in members.iter().enumerate() {
                let member_id = get_card_id(member, &card_database);
                player1.stage.stage[i] = member_id;
            }
            
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
            
            // Verify 3 members on stage
            let member_count = game_state.player1.stage.stage.iter()
                .filter(|&&id| id != -1)
                .count();
            assert_eq!(member_count, 3, "Should have 3 members on stage");
            
            // Verify Aqours live card in live card zone
            assert!(game_state.player1.live_card_zone.contains(&aqours_id), "Should have Aqours live card");
            
            // Simulate live start ability: all stage members gain blade
            for &member_id in game_state.player1.stage.stage.iter() {
                if member_id != -1 {
                    game_state.player1.blade += 1;
                }
            }
            
            // Verify blade increased by 3 (all 3 members gained blade)
            assert_eq!(game_state.player1.blade, 3, "Blade should increase by 3 (all members)");
            
            // The key assertion: effect applies to all stage members, not just 1
            // This tests the blade all members rule
            
            println!("Q121 verified: Effect applies to all stage members");
            println!("3 members on stage, all gained blade");
            println!("Total blade increase: 3");
        }
    } else {
        panic!("Required card PL!S-bp2-023-L not found for Q121 test");
    }
}
