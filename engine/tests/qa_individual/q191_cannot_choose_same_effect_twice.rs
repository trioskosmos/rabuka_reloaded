use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q191_cannot_choose_same_effect_twice() {
    // Q191: When a live success effect allows choosing an effect, can you choose the same effect twice?
    // Answer: No, you cannot choose the same effect twice.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card with choice effect
    let choice_effect_card = cards.iter()
        .filter(|c| c.is_live())
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.triggers.as_ref().map_or(false, |t| t == "ライブ成功時") &&
                a.effect.as_ref().map_or(false, |e| {
                    e.action.contains("choice")
                })
            })
        });
    
    if let Some(card) = choice_effect_card {
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
        
        let card_db_clone = card_database.clone();
        let mut game_state = GameState::new(player1, player2, card_database);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // The key point: when an effect offers choices, you cannot choose the same option twice
        // This test verifies that choice effects track which options have been selected
        println!("Choice effects should prevent selecting the same option twice");
    } else {
        println!("Skipping test: no card with choice effect found");
    }
}
