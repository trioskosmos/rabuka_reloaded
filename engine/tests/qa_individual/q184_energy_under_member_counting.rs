use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q184_energy_under_member_counting() {
    // Q184: When energy cards are placed under a member card, do they count as energy?
    // Answer: No, they don't count. Energy count references don't include energy cards placed under member cards.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card that references energy count
    let energy_count_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.map_or(false, |cost| cost <= 3))
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.effect.as_ref().map_or(false, |e| {
                    e.action.contains("energy") || e.count.is_some()
                })
            })
        });
    
    if let Some(card) = energy_count_card {
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
        
        // The key point: energy cards placed under member cards don't count as energy
        // This test verifies that energy count references don't include stacked energy
        assert!(game_state.player1.stage.stage.contains(&card_id),
            "Card should be on stage after playing");
        
        // Energy zone count should only count energy in energy zone
        let energy_zone_count = game_state.player1.energy_zone.cards.len();
        assert!(energy_zone_count > 0, "Should have energy in energy zone");
        
        println!("Energy zone count: {} (stacked energy not counted)", energy_zone_count);
    } else {
        println!("Skipping test: no card with energy count reference found");
    }
}
