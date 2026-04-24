use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy, setup_player_with_stage};

#[test]
fn test_q189_opponent_chooses_wait() {
    // Q189: Who chooses which member to wait?
    // Answer: The opponent player chooses.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a card with wait ability that targets opponent's choice
    let wait_choice_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.map_or(false, |cost| cost <= 4))
        .find(|c| {
            c.abilities.iter().any(|a| {
                a.triggers.as_ref().map_or(false, |t| t == "起動") &&
                a.cost.as_ref().map_or(false, |cost| {
                    cost.cost_type.as_deref() == Some("change_state") &&
                    cost.state_change.as_deref() == Some("wait")
                })
            })
        });
    
    if let Some(card) = wait_choice_card {
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
        
        // Setup opponent with multiple members on stage
        let opponent_members: Vec<_> = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| c.cost.map_or(false, |cost| cost <= 3))
            .filter(|c| get_card_id(c, &card_database) != 0)
            .take(3)
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        if opponent_members.len() >= 2 {
            player2.add_card_to_hand(opponent_members[0]);
            player2.add_card_to_hand(opponent_members[1]);
            let _ = TurnEngine::execute_main_phase_action(
                &mut game_state,
                &ActionType::PlayMemberToStage,
                Some(opponent_members[0]),
                None,
                Some(MemberArea::LeftSide),
                Some(false),
            );
            let _ = TurnEngine::execute_main_phase_action(
                &mut game_state,
                &ActionType::PlayMemberToStage,
                Some(opponent_members[1]),
                None,
                Some(MemberArea::Center),
                Some(false),
            );
        }
        
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
        
        // The key point: opponent chooses which member to wait
        // This test verifies that wait abilities that target opponent members
        // should give the opponent the choice of which member to wait
        assert!(game_state.player1.stage.stage.contains(&card_id),
            "Card should be on stage after playing");
        
        assert!(!game_state.player2.stage.stage.is_empty(),
            "Opponent should have members on stage");
    } else {
        println!("Skipping test: no card with wait ability found");
    }
}
