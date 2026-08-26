use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

#[test]
fn hs_cl1_already_wait_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-cl1-003-CL");
    game.state.player1.stage.stage = [-1, card, -1];
    // Put card already in wait
    game.state.mods.add_orientation_modifier(card, "wait");
    let res = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(card),
        None,
        None,
        None,
    );
    // Already wait: cost is already satisfied, engine still allows activation
    // but per current engine, no blade is granted (wait is not re-applied).
    // This test documents the current behavior.
    assert!(res.is_ok(), "re-wait should be ok, got {:?}", res);
    game.drain_auto_ability_choices();
    if game.has_pending_choice() {
        game.select_indices(&[0]);
        game.drain_auto_ability_choices();
    }
    let blade = game.state.mods.get_blade_modifier(card);
    assert_eq!(blade, 0, "already wait should not grant blade (cost already satisfied, no effect)");
}

#[test]
fn hs_cl1_turn_limit_blocks_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-cl1-003-CL");
    let other = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [card, other, -1];
    // First activation
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(card),
        None,
        None,
        None,
    )
    .unwrap();
    // Second same turn should be blocked
    let res = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(card),
        None,
        None,
        None,
    );
    assert!(res.is_err(), "ターン1回 should block second activation, got {:?}", res);
}

#[test]
fn hs_cl1_choice_among_multiple_mirakura() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-cl1-003-CL");
    let mira1 = game.id("PL!HS-bp5-003-R＋"); // also みらくらぱーく！
    let mira2 = game.id("PL!HS-bp5-003-AR");
    game.state.player1.stage.stage = [card, mira1, mira2];
    // Activate - should prompt to choose which MiraKura gets blade
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(card),
        None,
        None,
        None,
    )
    .unwrap();
    // If choice is required, it will be pending; otherwise it auto-picks.
    // We just verify that at least one of the three gets blade after resolution.
    game.drain_auto_ability_choices();
    if game.has_pending_choice() {
        game.select_indices(&[0]);
        game.drain_auto_ability_choices();
    }
    let b0 = game.state.mods.get_blade_modifier(card);
    let b1 = game.state.mods.get_blade_modifier(mira1);
    let b2 = game.state.mods.get_blade_modifier(mira2);
    assert!(
        b0 >= 1 || b1 >= 1 || b2 >= 1,
        "one of the MiraKura members should have blade, got card:{} mira1:{} mira2:{}",
        b0, b1, b2
    );
}
