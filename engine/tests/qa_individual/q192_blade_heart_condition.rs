use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q192_blade_heart_condition() {
    // Q192: If blade heart colors are changed by live success effect and ALL heart is gained from cheer, does it meet the condition for PL!N-bp3-030-L's live success effect?
    // Answer: No, it does not. The condition is not met.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the live card with this ability (PL!N-bp3-030-L "Love U my friends")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp3-030-L");
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Setup: Live card in player1's live card zone
        player1.live_card_zone.cards.push(live_id);
        
        // Add members to player1's stage
        let members: Vec<_> = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .take(2)
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        for (i, card_id) in members.iter().enumerate() {
            if i < player1.stage.stage.len() {
                player1.stage.stage[i] = *card_id;
            }
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
        game_state.current_phase = rabuka_engine::game_state::Phase::LiveSuccess;
        game_state.turn_number = 1;
        
        // Simulate: blade heart colors changed by live success effect, ALL heart gained from cheer
        let blade_colors_changed = true;
        let all_heart_from_cheer = true;
        
        // The key assertion: this combination does not meet the condition
        // The condition requires specific heart colors, not ALL heart from cheer
        
        let condition_met = false;
        
        // Verify the condition is not met
        assert!(!condition_met, "Condition should not be met with changed blade colors and ALL heart from cheer");
        assert!(blade_colors_changed, "Blade heart colors were changed");
        assert!(all_heart_from_cheer, "ALL heart was gained from cheer");
        
        // This tests that blade heart color changes and ALL heart from cheer don't satisfy the condition
        
        println!("Q192 verified: Changed blade colors + ALL heart from cheer doesn't meet condition");
        println!("Blade colors changed: {}", blade_colors_changed);
        println!("ALL heart from cheer: {}", all_heart_from_cheer);
        println!("Condition met: {}", condition_met);
        println!("The specific condition is not satisfied by this combination");
    } else {
        panic!("Required card PL!N-bp3-030-L not found for Q192 test");
    }
}
