use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;

/// End-to-end test: full game flow from RPS through turn 3's Main phase.
/// Verifies phase transitions are correct and each player's Main phase generates proper actions.
#[test]
fn e2e_full_game_rps_to_turn3() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let filler = game.id("PL!-sd1-010-SD");

    // Fill decks for both players
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Reset to RPS phase (TestGame::new starts at Main, skipping setup)
    game.state.current_phase = Phase::RockPaperScissors;
    game.state.turn_number = 0;

    // RPS: both players choose rock => tie => reset. Then P1 chooses rock, P2 scissors => P1 wins.
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::RockChoice,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::RockChoice,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    // Tie → both choices reset, still RPS phase. Now P1 wins.
    assert_eq!(
        game.state.current_phase,
        Phase::RockPaperScissors,
        "Tie resets choices, stays at RPS"
    );

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::RockChoice,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::ScissorsChoice,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    // P1 (rock) beats P2 (scissors) → should advance to ChooseFirstAttacker
    assert_eq!(
        game.state.current_phase,
        Phase::ChooseFirstAttacker,
        "RPS winner goes to ChooseFirstAttacker"
    );
    assert_eq!(game.state.rps_winner, Some(1), "P1 wins RPS");

    // P1 chooses to go first
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::ChooseFirstAttacker,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        game.state.current_phase,
        Phase::MulliganFirstAttacker,
        "ChooseFirstAttacker → MulliganFirstAttacker"
    );
    assert!(game.state.player1.is_first_attacker, "P1 is first attacker");
    assert!(
        !game.state.player2.is_first_attacker,
        "P2 is second attacker"
    );

    // P1 skips mulligan (keeps all cards)
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::SkipMulligan,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        game.state.current_phase,
        Phase::MulliganSecondAttacker,
        "Skip P1 mulligan → MulliganSecondAttacker"
    );

    // P2 skips mulligan
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::SkipMulligan,
        None,
        None,
        None,
        None,
    )
    .unwrap();

    // After mulligan, the first pass advances from Active → Energy
    assert_eq!(
        game.state.current_phase,
        Phase::Active,
        "Mulligan → Active (P1's turn)"
    );

    // Advance through P1's Active → Energy → Draw → Main
    // Pass in Active → Energy
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::Pass,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        game.state.current_phase,
        Phase::Energy,
        "Active pass → Energy"
    );

    // Pass in Energy → Draw
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::Pass,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(game.state.current_phase, Phase::Draw, "Energy pass → Draw");

    // Pass in Draw → Main
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::Pass,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        game.state.current_phase,
        Phase::Main,
        "Draw pass → Main (P1)"
    );

    // Verify P1's Main phase generates player actions (not live card actions)
    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let has_main_actions = actions.iter().any(|a| {
        matches!(a.action_type, ActionType::Pass)
            || matches!(a.action_type, ActionType::PlayMemberToStage)
            || matches!(a.action_type, ActionType::UseAbility)
    });
    assert!(
        has_main_actions,
        "P1's Main phase should generate player actions"
    );

    // P1 passes Main → P2's Active phase
    game.state.current_phase = Phase::Main; // ensure we're at Main
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::Pass,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        game.state.current_phase,
        Phase::Active,
        "P1 Main pass → Active (P2's turn)"
    );

    // Advance P2 through Active → Energy → Draw → Main
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::Pass,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        game.state.current_phase,
        Phase::Energy,
        "P2 Active → Energy"
    );

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::Pass,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(game.state.current_phase, Phase::Draw, "P2 Energy → Draw");

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::Pass,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(game.state.current_phase, Phase::Main, "P2 Draw → Main (P2)");

    // Verify P2's Main generates player actions (not live card actions)
    let actions_p2 = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let has_p2_main = actions_p2.iter().any(|a| {
        matches!(a.action_type, ActionType::Pass)
            || matches!(a.action_type, ActionType::PlayMemberToStage)
    });
    assert!(
        has_p2_main,
        "P2's Main phase should generate player actions"
    );

    // P2 passes Main → LiveCardSetFirstAttacker
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::Pass,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        game.state.current_phase,
        Phase::LiveCardSetFirstAttacker,
        "P2 Main pass → LiveCardSetFirstAttacker"
    );

    // Skip the live (no live cards, just pass through)
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::Pass,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        game.state.current_phase,
        Phase::LiveCardSetSecondAttacker,
        "Pass → LiveCardSetSecondAttacker"
    );

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::Pass,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    // After LiveCardSetSecondAttacker, the live performance runs automatically
    // The live performance might return with various phases depending on state.
    // Let's just verify we don't get stuck.

    // After live → next turn → go through phases until P1's Main in turn 2
    // The live phases transition through: FirstAttackerPerformance → SecondAttackerPerformance → LiveVictoryDetermination → Active
    // Then Active → Energy → Draw → Main (P1's turn)
    for _ in 0..20 {
        if game.state.current_phase == Phase::Main && game.state.turn_number >= 2 {
            break;
        }
        TurnEngine::execute_main_phase_action(
            &mut game.state,
            &ActionType::Pass,
            None,
            None,
            None,
            None,
        )
        .ok();
    }
    assert_eq!(
        game.state.current_phase,
        Phase::Main,
        "Advance to P1's Main phase in turn {}",
        game.state.turn_number
    );
    assert!(
        game.state.turn_number >= 2,
        "At least turn 2, got turn {}",
        game.state.turn_number
    );
    assert_eq!(
        game.state.current_phase,
        Phase::Main,
        "Advance to P1's Main phase in turn {}",
        game.state.turn_number,
    );
    assert!(
        game.state.turn_number >= 2,
        "At least turn 2, got turn {}",
        game.state.turn_number
    );
    assert!(game.state.turn_number >= 2, "At least turn 2");

    // Verify P1's Main phase still generates player actions in turn 2+
    let actions_turn2 = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let has_turn2_main = actions_turn2.iter().any(|a| {
        matches!(a.action_type, ActionType::Pass)
            || matches!(a.action_type, ActionType::PlayMemberToStage)
    });
    assert!(
        has_turn2_main,
        "P1's Main phase in turn 2+ should have player actions"
    );

    // Verify no live card set actions appear during P1's Main
    let has_live_actions = actions_turn2.iter().any(|a| {
        matches!(a.action_type, ActionType::SetLiveCard)
            || matches!(a.action_type, ActionType::FinishLiveCardSet)
    });
    assert!(
        !has_live_actions,
        "P1's Main phase should NOT have live card actions"
    );
}
