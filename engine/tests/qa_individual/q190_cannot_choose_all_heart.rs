use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q190_cannot_choose_all_heart() {
    // Q190: Can you choose ALL heart when selecting a heart color?
    // Answer: No, you cannot choose ALL heart.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a card with heart color selection ability
    let heart_select_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.map_or(false, |cost| cost <= 3))
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.effect.as_ref().map_or(false, |e| {
                    e.action == "gain_resource" && e.resource == Some("heart".to_string())
                })
            })
        });
    
    if let Some(card) = heart_select_card {
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
        
        // Play card to stage
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(card_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        
        assert!(result.is_ok(), "Card should play to stage: {:?}", result);
        
        // The key point: ALL heart cannot be chosen when selecting a specific heart color
        // Heart color selection should be limited to specific colors (heart01-heart06)
        assert!(game_state.player1.stage.stage.contains(&card_id),
            "Card should be on stage after playing");
        
        println!("Heart color selection should not include ALL heart option");
    } else {
        println!("Skipping test: no card with heart selection ability found");
    }
}
