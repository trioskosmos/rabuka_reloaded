use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Verify two copies of the same member card on stage have unique card IDs.
#[test]
fn duplicate_cards_on_stage_have_unique_ids() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    game.give_energy(10);

    let card1 = game.id("PL!-sd1-005-SD");
    let card2 = game.id("PL!-sd1-005-SD");

    assert_ne!(
        card1, card2,
        "Two copies of the same card must have unique IDs"
    );

    game.add_to_hand(card1);
    game.add_to_hand(card2);

    game.play_to_stage(card1, MemberArea::Center);
    assert_eq!(game.state.player1.stage.stage[1], card1);

    game.play_to_stage(card2, MemberArea::RightSide);
    assert_eq!(game.state.player1.stage.stage[2], card2);

    assert_ne!(
        game.state.player1.stage.stage[1], game.state.player1.stage.stage[2],
        "Center and right must have different card IDs"
    );

    assert!(
        !game.state.player1.waitroom.cards.contains(&card1),
        "Center card should not be in waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&card2),
        "Right card should not be in waitroom"
    );
}

/// Verify mulligan select/deselect toggle works for both hand indices.
#[test]
fn mulligan_toggle_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    game.state.current_phase = Phase::MulliganFirstAttacker;
    game.state.player1.is_first_attacker = true;

    let card1 = game.id("PL!-sd1-005-SD");
    let card2 = game.id("PL!-sd1-005-SD");
    game.state.player1.hand.cards.push(card1);
    game.state.player1.hand.cards.push(card2);

    assert!(game.state.mulligan_selected_indices.is_empty());

    // Select index 0
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::SelectMulligan,
        None,
        Some(vec![0]),
        None,
        None,
    )
    .unwrap();
    assert_eq!(game.state.mulligan_selected_indices.len(), 1);
    assert!(game.state.mulligan_selected_indices.contains(&0));

    // Deselect index 0 (toggle)
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::SelectMulligan,
        None,
        Some(vec![0]),
        None,
        None,
    )
    .unwrap();
    assert!(
        game.state.mulligan_selected_indices.is_empty(),
        "After deselect, no cards should be selected"
    );

    // Select index 1
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::SelectMulligan,
        None,
        Some(vec![1]),
        None,
        None,
    )
    .unwrap();
    assert_eq!(game.state.mulligan_selected_indices.len(), 1);
    assert!(game.state.mulligan_selected_indices.contains(&1));

    // Deselect index 1
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::SelectMulligan,
        None,
        Some(vec![1]),
        None,
        None,
    )
    .unwrap();
    assert!(game.state.mulligan_selected_indices.is_empty());
}
