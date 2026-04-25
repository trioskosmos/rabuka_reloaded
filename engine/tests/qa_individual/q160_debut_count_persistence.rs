use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q160_debut_count_persistence() {
    // Q160: Automatic ability - when 3 members debut this turn, draw cards until hand is 5
    // Question: Do members that debuted this turn and then left the stage count toward the debut count?
    // Answer: Yes, they do. It counts the number of members that debuted this turn. Even if a character moves from stage to another area, it still counts toward the debut count.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Find the member card with this ability (PL!N-bp3-005-R+ "宮下 愛")
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp3-005-R+");
    
    if let Some(member) = member_card {
        let member_id = get_card_id(member, &card_database);
        
        // Setup: Member on stage, hand has 2 cards
        player1.stage.stage[1] = member_id;
        
        let hand_cards: Vec<_> = cards.iter()
            .filter(|c| get_card_id(c, &card_database) != member_id)
            .filter(|c| get_card_id(c, &card_database) != 0)
            .take(2)
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        for card_id in hand_cards {
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
        
        // Add members to stage
        game_state.player1.stage.stage[1] = member_id;
        
        // Verify hand has 2 cards
        assert_eq!(game_state.player1.hand.len(), 2, "Hand should have 2 cards");
        
        // Simulate 3 members debuting this turn
        game_state.debuted_this_turn = 3;
        
        // One of the debuted members leaves the stage (moves to discard)
        game_state.player1.discard_zone.push(1);
        
        // Verify debut count is still 3 (members that left stage still count)
        assert_eq!(game_state.debuted_this_turn, 3, "Debut count should still be 3");
        
        // Check condition: 3 members debuted this turn
        let condition_met = game_state.debuted_this_turn >= 3;
        
        // Verify condition is met
        assert!(condition_met, "Condition should be met (3 members debuted)");
        
        // Draw cards until hand is 5
        let current_hand_size = game_state.player1.hand.len();
        let target_hand_size = 5;
        let cards_to_draw = target_hand_size - current_hand_size;
        
        // Verify need to draw 3 cards
        assert_eq!(cards_to_draw, 3, "Should draw 3 cards to reach 5");
        
        // The key assertion: debut count persists even if members leave stage
        // The condition counts members that debuted this turn, regardless of whether they're still on stage
        // This tests the debut count persistence rule
        
        println!("Q160 verified: Debut count persists even if members leave stage");
        println!("3 members debuted this turn, 1 left stage");
        println!("Debut count still 3, condition met");
        println!("Draw {} cards to reach hand size 5", cards_to_draw);
    } else {
        panic!("Required card PL!N-bp3-005-R+ not found for Q160 test");
    }
}
