use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

fn live_score_2_no(db: &rabuka_engine::core::card::CardDatabase) -> String {
    db.cards
        .values()
        .find(|c| c.card_type == rabuka_engine::core::card::CardType::Live && c.score == Some(2))
        .unwrap()
        .card_no
        .to_string()
}

#[test]
fn shizuku_no_live_in_discard_no_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shizuku = game.id("PL!N-bp5-003-R");
    let filler = game.id("PL!-sd1-010-SD");
    let hand_cost = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [filler, shizuku, filler];
    game.state.player1.hand.cards.push(hand_cost);
    // waitroom empty
    game.give_energy(5);
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(shizuku),
        None,
        None,
        None,
    )
    .unwrap();
    game.select_indices(&[0]); // pay hand cost
    // No live to select, should auto-complete without further choice
    assert!(
        !game.has_pending_choice(),
        "no live in discard should not prompt selection"
    );
    assert_eq!(game.state.player1.hand.cards.len(), 0, "hand cost was discarded");
}

#[test]
fn shizuku_turn_limit_blocks_second_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let shizuku = game.id("PL!N-bp5-003-R");
    let live_no = live_score_2_no(&db);
    let live = game.id(&live_no);
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [filler, shizuku, filler];
    game.state.player1.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.state.player1.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.state.player1.waitroom.cards.push(live);
    game.state.player1.waitroom.cards.push(game.id(&live_no)); // second live for second activation
    game.give_energy(10);
    // First activation
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(shizuku),
        None,
        None,
        None,
    )
    .unwrap();
    game.select_indices(&[0]);
    game.select_indices(&[0]);
    game.select_option(1);
    assert!(game.state.player1.hand.cards.contains(&live));
    // Second activation same turn should be blocked (turn1)
    let before_hand = game.state.player1.hand.cards.len();
    let res = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(shizuku),
        None,
        None,
        None,
    );
    assert!(res.is_err() || !game.has_pending_choice(), "second activation should be blocked by ターン1回, got {:?}", res);
    // At minimum, it should not consume hand
    assert_eq!(game.state.player1.hand.cards.len(), before_hand, "hand should not be consumed on blocked second activation");
}

#[test]
fn shizuku_p_and_ar_variants_work() {
    let db = load_real_database();
    for variant in ["PL!N-bp5-003-P", "PL!N-bp5-003-AR"] {
        let mut game = TestGame::new(db.clone());
        let shizuku = game.id(variant);
        let live_no = live_score_2_no(&db);
        let live = game.id(&live_no);
        let filler = game.id("PL!-sd1-010-SD");
        // Need to put variant on stage (even though P/AR are maybe not stageable 3-cost? but test harness allows)
        game.state.player1.stage.stage = [filler, shizuku, filler];
        game.state.player1.hand.cards.push(game.id("PL!-sd1-010-SD"));
        game.state.player1.waitroom.cards.push(live);
        game.give_energy(5);
        TurnEngine::execute_main_phase_action(
            &mut game.state,
            &ActionType::UseAbility,
            Some(shizuku),
            None,
            None,
            None,
        )
        .unwrap();
        game.select_indices(&[0]);
        game.select_indices(&[0]);
        game.select_option(1);
        assert!(
            game.state.player1.hand.cards.contains(&live),
            "{} should also recover live",
            variant
        );
    }
}
