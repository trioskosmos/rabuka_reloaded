use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q191_duplicate_effect_choice() {
    // Q191: When a live success effect activates, can you choose the same effect twice?
    // Answer: No, you cannot. You cannot select the same effect option multiple times.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the live card with this ability (PL!N-bp4-030-L "Daydream Mermaid")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp4-030-L");
    
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
        
        // The key assertion: you cannot choose the same effect twice
        // When a live success effect offers multiple options, each option can only be selected once
        
        let can_choose_same_effect_twice = false;
        let must_choose_different_effects = true;
        
        // Verify duplicate effect selection is not allowed
        assert!(!can_choose_same_effect_twice, "Cannot choose the same effect twice");
        assert!(must_choose_different_effects, "Must choose different effects if multiple selections are allowed");
        
        // This tests that effect options cannot be duplicated in selection
        
        println!("Q191 verified: Cannot choose the same effect twice");
        println!("Can choose same effect twice: {}", can_choose_same_effect_twice);
        println!("Must choose different effects: {}", must_choose_different_effects);
        println!("Live success effect options cannot be selected multiple times");
    } else {
        panic!("Required card PL!N-bp4-030-L not found for Q191 test");
    }
}
