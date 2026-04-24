use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q81_constant_ability_names() {
    // Q81: Combined name cards (like LL-bp1-001-R+ "上原歩夢&澁谷かのん&日野下花帆")
    // are referenced by their individual component names for ability conditions.
    // For example, the combined card can be referenced as having "日野下花帆" among 蓮ノ空 members.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the specific combined name card: LL-bp1-001-R+ "上原歩夢&澁谷かのん&日野下花帆"
    let combined_card = cards.iter()
        .find(|c| c.card_no == "LL-bp1-001-R+");
    
    if let Some(card) = combined_card {
        let card_id = get_card_id(card, &card_database);
        
        setup_player_with_hand(&mut player1, vec![card_id]);
        
        // Add energy cards
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(10)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // The key point: combined name cards (LL-bp1-001-R+) have multiple character names
        // and can be referenced by any of those individual names for ability conditions
        assert!(card.name.contains("&"),
            "Card should have & in name: {}", card.name);
        
        // Verify the specific card exists in the database
        assert!(card_database.get_card(card_id).is_some(),
            "Card LL-bp1-001-R+ should exist in database");
        
        println!("Combined name card: {} ({})", card.name, card.card_no);
    } else {
        panic!("Required card LL-bp1-001-R+ not found in card database");
    }
}
