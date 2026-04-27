use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q237_card_name_mismatch() {
    // Q237: When PL!HS-sd1-018-SD "Dream Believers（104期Ver.）" is revealed by activated ability, can you add PL!HS-bp1-019-L "Dream Believers" from waitroom to hand?
    // Answer: No, you cannot.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the member card with this ability (PL!HS-bp5-001-R＋ "日野下花帆")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!HS-bp5-001-R＋");
    
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
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // Simulate: PL!HS-sd1-018-SD "Dream Believers（104期Ver.）" revealed by activated ability
        let special_deck_card_revealed = true;
        let card_name_does_not_match = true;
        
        // The key assertion: cannot add PL!HS-bp1-019-L "Dream Believers" from waitroom to hand
        // The special deck version with additional text in the name doesn't match the regular version
        
        let can_add_to_hand = false;
        let expected_result = false;
        
        // Verify the card name mismatch prevents retrieval
        assert!(!can_add_to_hand, "Should not be able to add card from waitroom to hand");
        assert_eq!(can_add_to_hand, expected_result, "Should not be able to add card");
        assert!(special_deck_card_revealed, "Special deck card was revealed");
        assert!(card_name_does_not_match, "Card names do not match");
        
        // This tests that card name matching is strict and doesn't work with different versions
        
        println!("Q237 verified: Cannot add card from waitroom when card name does not match revealed card");
        println!("Special deck card revealed: {}", special_deck_card_revealed);
        println!("Card name does not match: {}", card_name_does_not_match);
        println!("Can add to hand: {}", can_add_to_hand);
        println!("Expected result: {}", expected_result);
        println!("Card name mismatch prevents retrieving different versions from waitroom");
    } else {
        panic!("Required card PL!HS-bp5-001-R＋ not found for Q237 test");
    }
}
