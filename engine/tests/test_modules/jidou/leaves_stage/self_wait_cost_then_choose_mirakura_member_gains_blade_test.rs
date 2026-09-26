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
    // Already wait: "wait this member" cannot be paid for an already-waited
    // member (Q137) — the mandatory cost is unpayable, so activation is
    // refused (Rule 9.4.2.3/Q56) and no blade is granted.
    assert!(res.is_err(), "re-wait of an already-wait member must be refused, got {:?}", res);
    assert!(!game.has_pending_choice(), "no dangling prompt");
    game.drain_auto_ability_choices();
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
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
    // The first activation must really have happened, or a second refusal
    // proves nothing about the ターン1回 limit.
    let turn = game.state.turn_number;
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(card, 0, turn)),
        "the first activation must record its ターン1回 use (card={card}, \
         ab#0, turn={turn})"
    );
    let blade_after_first = game.state.mods.get_blade_modifier(card);
    let wait_after_first: Option<String> = game
        .state
        .mods
        .get_orientation_modifier(card)
        .map(|m| m.to_string());
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
    // A refused attempt must leave the card exactly as the first one left it:
    // no second wait, no second blade, no dangling prompt.
    let wait_now: Option<String> = game
        .state
        .mods
        .get_orientation_modifier(card)
        .map(|m| m.to_string());
    assert_eq!(
        wait_now, wait_after_first,
        "a ターン1回 refusal must not wait the member a second time"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        blade_after_first,
        "a ターン1回 refusal must not grant a second blade"
    );
    assert!(
        !game.has_pending_choice(),
        "a ターン1回 refusal must not open a prompt"
    );
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
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
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
