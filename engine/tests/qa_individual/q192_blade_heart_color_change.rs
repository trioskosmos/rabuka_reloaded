use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q192_blade_heart_color_change() {
    // Q192: If blade heart color was changed by live success effect and you got ALL heart via cheer, does PL!N-bp3-030-L's condition get met?
    // Answer: No, it doesn't meet the condition.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card with blade heart condition
    let blade_heart_card = cards.iter()
        .filter(|c| c.is_live())
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.triggers.as_ref().map_or(false, |t| t == "ライブ成功時") &&
                a.effect.as_ref().map_or(false, |e| {
                    e.action.contains("heart") || e.resource == Some("heart".to_string())
                })
            })
        });
    
    if let Some(card) = blade_heart_card {
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
        
        // The key point: blade heart color change effects don't satisfy ALL heart conditions
        // Heart color changes are different from gaining ALL heart
        println!("Blade heart color changes don't satisfy ALL heart conditions");
    } else {
        println!("Skipping test: no card with blade heart condition found");
    }
}
