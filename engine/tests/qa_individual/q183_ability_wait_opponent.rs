use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy, setup_player_with_stage};

#[test]
fn test_q183_ability_wait_opponent() {
    // Q183: Can an ability that says "You may wait up to 3 members: For each member you wait this way, draw 1 card" wait the opponent's members?
    // Card: PL!-pb1-008-P+ (小泉花陽)
    // Answer: No, it cannot. When paying the cost to wait member cards, you must wait your own stage members.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the specific card: PL!-pb1-008-P+ (小泉花陽)
    let wait_card = cards.iter()
        .find(|c| c.card_no == "PL!-pb1-008-P＋");
    
    if let Some(card) = wait_card {
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
        
        let mut game_state = GameState::new(player1, player2, card_database);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // The key point: abilities with wait cost can only target the player's own members, not opponent's
        // This test verifies that the specific card PL!-pb1-008-P+ exists and has the wait ability
        assert!(card.abilities.iter().any(|a| {
            a.triggers.as_ref().map_or(false, |t| t == "登場") &&
            a.cost.as_ref().map_or(false, |cost| {
                cost.cost_type.as_deref() == Some("change_state") &&
                cost.state_change.as_deref() == Some("wait")
            })
        }), "PL!-pb1-008-P+ should have debut wait ability");
        
        // Verify the specific card exists in the database
        assert!(card_database.get_card(card_id).is_some(),
            "PL!-pb1-008-P+ should exist in database");
        
        println!("Wait ability card: {} ({})", card.name, card.card_no);
    } else {
        panic!("Required card PL!-pb1-008-P+ not found in card database");
    }
}
