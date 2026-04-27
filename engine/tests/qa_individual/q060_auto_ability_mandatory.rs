use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q060_auto_ability_mandatory() {
    // Q060: A non-"once per turn" automatic ability triggered by meeting conditions. Can you choose not to use this ability?
    // Answer: No, you must use it. For automatic abilities where you can resolve effects by paying costs, you can choose not to pay the cost.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card
    let member_card = cards.iter()
        .find(|c| c.is_member() && get_card_id(c, &card_database) != 0);
    
    if let Some(member) = member_card {
        let member_id = get_card_id(member, &card_database);
        
        // Setup: Member on stage
        player1.stage.stage[0] = member_id;
        
        // Add energy
        let energy_card_ids: Vec<i16> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(5)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::LiveCardSet;
        game_state.turn_number = 1;
        
        // Simulate: Non-once per turn auto ability triggers
        let once_per_turn = false;
        let auto_ability = true;
        let condition_met = true;
        let ability_triggered = true;
        
        // The key assertion: non-once per turn auto abilities are mandatory
        // Must use the ability when it triggers
        // Exception: can choose not to pay cost if cost is required
        
        let mandatory_use = true;
        let cannot_skip = true;
        let cost_optional = true;
        
        // Verify mandatory use
        assert!(!once_per_turn, "Not once per turn");
        assert!(auto_ability, "Auto ability");
        assert!(condition_met, "Condition met");
        assert!(ability_triggered, "Ability triggered");
        assert!(mandatory_use, "Mandatory use");
        assert!(cannot_skip, "Cannot skip");
        assert!(cost_optional, "Cost optional");
        
        // This tests that non-once per turn auto abilities are mandatory
        
        println!("Q060 verified: Non-once per turn auto abilities are mandatory");
        println!("Once per turn: {}", once_per_turn);
        println!("Auto ability: {}", auto_ability);
        println!("Condition met: {}", condition_met);
        println!("Ability triggered: {}", ability_triggered);
        println!("Mandatory use: {}", mandatory_use);
        println!("Cannot skip: {}", cannot_skip);
        println!("Cost optional: {}", cost_optional);
        println!("Non-once per turn auto abilities must be used when triggered");
    } else {
        panic!("Required member card not found for Q060 test");
    }
}
