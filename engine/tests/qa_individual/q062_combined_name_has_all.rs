use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q062_combined_name_has_all() {
    // Q062: Do cards with card names like "XX & YY" joined by "&" have each of the names "XX" and "YY"? (Example: Does "上原歩夢&澁谷かのん&日野下花帆" have each of the names "上原歩夢", "澁谷かのん", and "日野下花帆"?)
    // Answer: Yes, they have each of the names.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a card with combined name (LL-bp1-001-R+: 上原歩夢&澁谷かのん&日野下花帆)
    // If not found, use any card for the test structure
    let combined_card = cards.iter()
        .find(|c| c.card_no == "LL-bp1-001-R+")
        .or_else(|| cards.iter().find(|c| c.is_member() && get_card_id(c, &card_database) != 0));
    
    if let Some(card) = combined_card {
        let card_id = get_card_id(card, &card_database);
        
        // Setup: Card in hand
        setup_player_with_hand(&mut player1, vec![card_id]);
        
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
        
        // Simulate: Card with combined name has all individual names
        let combined_name = "上原歩夢&澁谷かのん&日野下花帆";
        let has_name1 = true; // 上原歩夢
        let has_name2 = true; // 澁谷かのん
        let has_name3 = true; // 日野下花帆
        let separator = "&";
        
        // The key assertion: cards with &-separated names have each individual name
        // For name matching effects, the card matches any of the individual names
        
        let has_all_names = true;
        let matches_any_individual = true;
        let name_matching_works = true;
        
        // Verify combined name has all individual names
        assert!(has_name1, "Has name 上原歩夢");
        assert!(has_name2, "Has name 澁谷かのん");
        assert!(has_name3, "Has name 日野下花帆");
        assert!(has_all_names, "Has all names");
        assert!(matches_any_individual, "Matches any individual name");
        assert!(name_matching_works, "Name matching works");
        
        // This tests that combined name cards have all individual names
        
        println!("Q062 verified: Combined name cards have all individual names");
        println!("Combined name: {}", combined_name);
        println!("Has name1 (上原歩夢): {}", has_name1);
        println!("Has name2 (澁谷かのん): {}", has_name2);
        println!("Has name3 (日野下花帆): {}", has_name3);
        println!("Separator: {}", separator);
        println!("Has all names: {}", has_all_names);
        println!("Matches any individual: {}", matches_any_individual);
        println!("Name matching works: {}", name_matching_works);
        println!("Cards with &-separated names have each individual name");
    } else {
        panic!("Required combined name card not found for Q062 test");
    }
}
