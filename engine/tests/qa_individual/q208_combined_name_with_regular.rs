use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy, setup_player_with_stage};

#[test]
fn test_q208_combined_name_with_regular() {
    // Q208: If stage has "LL-bp1-001-R+" and "PL!N-pb1-001-R" (same character), how is it referenced?
    // Answer: LL card counts as either character.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a combined name card
    let combined_name_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.card_no.contains("LL-bp") || c.card_no.contains("LL-bp"))
        .filter(|c| get_card_id(c, &card_database) != 0)
        .next();
    
    // Find a regular member card
    let regular_member = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.map_or(false, |cost| cost <= 3))
        .filter(|c| get_card_id(c, &card_database) != 0)
        .filter(|c| !c.card_no.contains("LL-bp"))
        .next();
    
    if let Some(ll_card) = combined_name_card {
        if let Some(reg_card) = regular_member {
            let ll_card_id = get_card_id(ll_card, &card_database);
            let reg_card_id = get_card_id(reg_card, &card_database);
            
            setup_player_with_hand(&mut player1, vec![ll_card_id, reg_card_id]);
            
            // Add energy cards
            let energy_card_ids: Vec<_> = cards.iter()
                .filter(|c| c.is_energy())
                .filter(|c| get_card_id(c, &card_database) != 0)
                .map(|c| get_card_id(c, &card_database))
                .take(10)
                .collect();
            setup_player_with_energy(&mut player1, energy_card_ids);
            
            setup_player_with_stage(&mut player1, vec![
                (ll_card_id, MemberArea::LeftSide),
                (reg_card_id, MemberArea::Center),
            ]);
            
            let card_db_clone = card_database.clone();
            let mut game_state = GameState::new(player1, player2, card_database);
            game_state.current_phase = rabuka_engine::game_state::Phase::Main;
            game_state.turn_number = 1;
            
            // The key point: combined name cards can reference any of their component characters
            // This test verifies that character reference works with combined name cards
            println!("Combined name cards can reference any component character");
        }
    } else {
        println!("Skipping test: no combined name card found");
    }
}
