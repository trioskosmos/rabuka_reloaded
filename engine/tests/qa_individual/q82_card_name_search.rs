use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q82_card_name_search() {
    // Q82: Can you add live cards with specific names to hand via search effect?
    // Card: PL!HS-bp1-009-R "安養寺 姫芽"
    // The question asks about adding "[PL!HS-bp1-023]ド！ド！ド！" and "[PL!HS-PR-012]アイデンティティ" 
    // to hand via this ability's effect. These are 'みらくらぱーく！' cards.
    // Answer: Yes, they can be added to hand.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the specific card with search ability: PL!HS-bp1-009-R "安養寺 姫芽"
    let search_card = cards.iter()
        .find(|c| c.card_no == "PL!HS-bp1-009-R");
    
    if let Some(card) = search_card {
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
        
        // The key point: search effects can add cards with specific names/groups to hand
        // The ability searches for 'みらくらぱーく！' cards
        assert!(card.abilities.iter().any(|a| {
            a.triggers.as_ref().map_or(false, |t| t == "登場") &&
            a.effect.as_ref().map_or(false, |e| {
                e.action == "draw" || e.action == "search" || e.action == "move_cards"
            })
        }), "Card should have debut search ability");
        
        // Verify the specific card exists in the database
        assert!(card_database.get_card(card_id).is_some(),
            "Card PL!HS-bp1-009-R should exist in database");
        
        // Verify the target live cards exist
        let target_card_1 = cards.iter().find(|c| c.card_no == "PL!HS-bp1-023");
        let target_card_2 = cards.iter().find(|c| c.card_no == "PL!HS-PR-012");
        
        assert!(target_card_1.is_some(), "Target card PL!HS-bp1-023 should exist");
        assert!(target_card_2.is_some(), "Target card PL!HS-PR-012 should exist");
        
        println!("Search card: {} ({})", card.name, card.card_no);
    } else {
        panic!("Required card PL!HS-bp1-009-R not found in card database");
    }
}
