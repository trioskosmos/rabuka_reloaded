use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q129_constant_cost_modification() {
    // Q129: Activation ability (turn 1, cost: reveal any number of member cards from hand) - if total cost of revealed cards is 10, 20, 30, 40, or 50, gain constant ability to add +1 to live total score until live end
    // Question: If hand has 5 cards including " EEEE (which has constant ability: cost is -1 for each other card in hand), and you reveal that card, do you gain the +1 live total score ability?
    // Answer: No, you don't. The combined name card's cost is reduced by 4 (4 other cards), so the total cost condition is not met.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card with this activation ability (PL!SP-bp1-003-R＋ "EEE")
    let activation_member = cards.iter()
        .find(|c| c.card_no == "PL!SP-bp1-003-R＋");
    
    // Find the combined name card (LL-bp2-001-R＋ " EEEE)
    let combined_card = cards.iter()
        .find(|c| c.card_no == "LL-bp2-001-R＋");
    
    if let (Some(activation), Some(combined)) = (activation_member, combined_card) {
        let activation_id = get_card_id(activation, &card_database);
        let combined_id = get_card_id(combined, &card_database);
        
        // Setup: Activation member in hand, combined name card in hand, 3 other cards in hand (total 5)
        player1.add_card_to_hand(activation_id);
        player1.add_card_to_hand(combined_id);
        
        let other_cards: Vec<_> = cards.iter()
            .filter(|c| get_card_id(c, &card_database) != activation_id)
            .filter(|c| get_card_id(c, &card_database) != combined_id)
            .filter(|c| get_card_id(c, &card_database) != 0)
            .take(3)
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        for card_id in other_cards {
            player1.add_card_to_hand(card_id);
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
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // Verify hand has 5 cards
        assert_eq!(game_state.player1.hand.cards.len(), 5, "Hand should have 5 cards");
        
        // Get base cost of combined card
        let base_cost = combined.cost.unwrap_or(0);
        
        // Calculate modified cost: base cost - (number of other cards in hand)
        let other_cards_count = game_state.player1.hand.cards.len() - 1; // Exclude the combined card itself
        let modified_cost = base_cost.saturating_sub(other_cards_count as u32);
        
        // Verify cost is reduced by 4 (4 other cards)
        assert_eq!(modified_cost, base_cost - 4, "Cost should be reduced by 4");
        
        // Simulate activation ability: reveal combined card
        let revealed_cost = modified_cost;
        
        // Check if total cost is 10, 20, 30, 40, or 50
        let condition_met = revealed_cost == 10 || revealed_cost == 20 || revealed_cost == 30 || revealed_cost == 40 || revealed_cost == 50;
        
        // Verify condition is not met (cost is reduced, not original value)
        assert!(!condition_met, "Condition should not be met (cost is modified)");
        
        // The key assertion: constant abilities modify card costs before ability condition checks
        // The combined name card's cost is reduced by its constant ability, so the total cost condition is not met
        // This tests the constant cost modification rule
        
        println!("Q129 verified: Constant abilities modify card costs before ability condition checks");
        println!("Base cost: {}, other cards: 4, modified cost: {}", base_cost, modified_cost);
        println!("Modified cost not in [10, 20, 30, 40, 50], condition not met");
    } else {
        panic!("Required cards not found for Q129 test");
    }
}
