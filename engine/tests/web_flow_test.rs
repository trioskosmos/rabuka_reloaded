use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::game_setup;
use rabuka_engine::game_setup::ActionType;
use std::sync::Arc;
use std::path::Path;

#[test]
fn test_game_flow_from_init_to_active() {
    // Load card database
    let cards = CardLoader::load_cards_from_file(Path::new("../cards/cards.json")).unwrap();
    let card_database = Arc::new(CardDatabase::load_or_create(cards));
    
    // Create players with proper arguments
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Create game state
    let mut game_state = GameState::new(player1, player2, card_database);
    
    // Setup game (Rule 6.2)
    game_setup::setup_game(&mut game_state);
    
    // Verify initial phase
    assert_eq!(game_state.current_phase, rabuka_engine::game_state::Phase::RockPaperScissors);
    
    // Simulate P1 RPS choice (Rock)
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::RockChoice,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_ok());
    
    // Should still be in RPS phase waiting for P2
    assert_eq!(game_state.current_phase, rabuka_engine::game_state::Phase::RockPaperScissors);
    assert!(game_state.player1_rps_choice.is_some());
    assert!(game_state.player2_rps_choice.is_none());
    
    // Simulate P2 RPS choice (Paper - P2 wins)
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PaperChoice,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_ok());
    
    // Should be in ChooseFirstAttacker phase
    assert_eq!(game_state.current_phase, rabuka_engine::game_state::Phase::ChooseFirstAttacker);
    assert_eq!(game_state.rps_winner, Some(2)); // P2 won
    
    // P2 (winner) chooses to be first attacker (human controls both)
    // Use ChooseSecondAttacker to make P2 first attacker
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::ChooseSecondAttacker,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_ok());
    
    // Should be in MulliganP2Turn (P2 is first attacker)
    assert_eq!(game_state.current_phase, rabuka_engine::game_state::Phase::MulliganP2Turn);
    assert!(!game_state.player1.is_first_attacker);
    assert!(game_state.player2.is_first_attacker);
    
    // P2 skips mulligan
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::SkipMulligan,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_ok());
    
    // Should be in MulliganP1Turn
    assert_eq!(game_state.current_phase, rabuka_engine::game_state::Phase::MulliganP1Turn);
    
    // P1 skips mulligan
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::SkipMulligan,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_ok());
    
    // Should be in Active phase
    assert_eq!(game_state.current_phase, rabuka_engine::game_state::Phase::Active);
    assert_eq!(game_state.current_turn_phase, rabuka_engine::game_state::TurnPhase::FirstAttackerNormal);
    
    // Advance through automatic phases (Active -> Energy -> Draw -> Main)
    // These are automatic phases handled by settle_single_player_state
    for _ in 0..3 {
        rabuka_engine::turn::TurnEngine::advance_phase(&mut game_state);
    }
    
    // Should be in Main phase (first attacker's turn)
    assert_eq!(game_state.current_phase, rabuka_engine::game_state::Phase::Main);
    assert_eq!(game_state.current_turn_phase, rabuka_engine::game_state::TurnPhase::FirstAttackerNormal);
    
    // Advance to second attacker's turn
    rabuka_engine::turn::TurnEngine::advance_phase(&mut game_state);
    
    // Should be back in Active phase for second attacker
    assert_eq!(game_state.current_phase, rabuka_engine::game_state::Phase::Active);
    assert_eq!(game_state.current_turn_phase, rabuka_engine::game_state::TurnPhase::SecondAttackerNormal);
    
    // Advance through automatic phases for second attacker
    for _ in 0..3 {
        rabuka_engine::turn::TurnEngine::advance_phase(&mut game_state);
    }
    
    // Should be in Main phase (second attacker's turn)
    assert_eq!(game_state.current_phase, rabuka_engine::game_state::Phase::Main);
    assert_eq!(game_state.current_turn_phase, rabuka_engine::game_state::TurnPhase::SecondAttackerNormal);
    
    println!("Game flow test passed: Init -> RPS -> ChooseFirstAttacker -> Mulligan -> Active -> Energy -> Draw -> Main (both attackers)");
}
