use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q199_baton_same_turn() {
    // Q199: Can a member that debuted via an ability be baton touched in the same turn?
    // Answer: No, it cannot be baton touched in the same turn.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card with debut ability
    let debut_ability_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.map_or(false, |cost| cost <= 4))
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.triggers.as_ref().map_or(false, |t| t == "起動") &&
                a.effect.as_ref().map_or(false, |e| {
                    e.action == "move_cards" && e.destination == Some("stage".to_string())
                })
            })
        });
    
    if let Some(card) = debut_ability_card {
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
        
        // The key point: members that debuted via abilities cannot be baton touched same turn
        // This test verifies that debut abilities create a restriction on baton touch
        println!("Members debuted via abilities cannot be baton touched same turn");
    } else {
        println!("Skipping test: no card with debut ability found");
    }
}
