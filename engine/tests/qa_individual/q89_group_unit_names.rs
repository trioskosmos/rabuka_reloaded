use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q89_group_unit_names() {
    // Q89: Does this card have group names and unit names?
    // Answer: It has the group names listed on the card, but does not have unit names not listed on the card.
    // Specific cards: LL-bp1-001-R+, LL-bp2-001-R+, LL-bp3-001-R+
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the specific combined name cards
    let card1 = cards.iter().find(|c| c.card_no == "LL-bp1-001-R＋");
    let card2 = cards.iter().find(|c| c.card_no == "LL-bp2-001-R＋");
    let card3 = cards.iter().find(|c| c.card_no == "LL-bp3-001-R＋");
    
    if let (Some(c1), Some(c2), Some(c3)) = (card1, card2, card3) {
        let card1_id = get_card_id(c1, &card_database);
        let card2_id = get_card_id(c2, &card_database);
        let card3_id = get_card_id(c3, &card_database);
        
        // Add cards to player1's hand
        player1.hand.cards.push(card1_id);
        player1.hand.cards.push(card2_id);
        player1.hand.cards.push(card3_id);
        
        let game_state = GameState::new(player1, player2, card_database);
        
        // The key point: cards have group names listed on them, but not unit names not listed
        // This test verifies that the combined name cards exist in the database
        assert!(game_state.card_database.get_card(card1_id).is_some(),
            "LL-bp1-001-R+ should exist in database");
        assert!(game_state.card_database.get_card(card2_id).is_some(),
            "LL-bp2-001-R+ should exist in database");
        assert!(game_state.card_database.get_card(card3_id).is_some(),
            "LL-bp3-001-R+ should exist in database");
        
        // Verify the cards have combined names (contain "&")
        assert!(c1.name.contains("&"), "LL-bp1-001-R+ should have combined name: {}", c1.name);
        assert!(c2.name.contains("&"), "LL-bp2-001-R+ should have combined name: {}", c2.name);
        assert!(c3.name.contains("&"), "LL-bp3-001-R+ should have combined name: {}", c3.name);
        
        println!("Combined name cards: {} ({}), {} ({}), {} ({})", 
            c1.name, c1.card_no, c2.name, c2.card_no, c3.name, c3.card_no);
    } else {
        panic!("Required combined name cards not found in card database");
    }
}
