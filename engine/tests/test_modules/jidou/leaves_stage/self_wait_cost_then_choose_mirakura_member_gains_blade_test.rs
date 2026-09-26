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

/// Fire 大沢瑠璃乃's 起動 with three みらくらぱーく！ members on stage, answer the
/// "choose one" prompt by picking area `pick`, and report the winning card plus
/// the area the blade landed in.
///
/// A runner so the choice can be exercised from both sides: if picking area 1
/// and picking area 2 both granted the blade to the same AREA, the prompt would
/// be decoration and the effect would be an auto-pick. The three cards are three
/// PRINTINGS of the same character, so this can only be judged by id and by area
/// — a name-based check could not tell the winner from the losers.
fn blade_recipient(pick: usize) -> (i16, usize) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-cl1-003-CL");
    let mira1 = game.id("PL!HS-bp5-003-R＋");
    let mira2 = game.id("PL!HS-bp5-003-AR");
    let outsider = game.id("PL!-sd1-010-SD");
    game.assert_card_identity(card, "PL!HS-cl1-003-CL");
    game.assert_card_identity(mira1, "PL!HS-bp5-003-R＋");
    game.assert_card_identity(mira2, "PL!HS-bp5-003-AR");
    // Premise: the filter is 『みらくらぱーく！』, so every member on stage must
    // be in that group — otherwise "the chosen area gained the blade" proves
    // nothing, because any member would have been eligible.
    for (id, ctx) in [
        (card, "the activating card"),
        (mira1, "the centre member"),
        (mira2, "the right member"),
    ] {
        game.assert_card_in_group(id, "みらくらぱーく！", ctx);
    }
    // The three are three PRINTINGS of the same character, which is why the
    // result has to be compared by id: a name-based check could not tell the
    // winner from the losers.
    game.assert_same_card_name(card, mira1, "three printings of one character");
    // The activating card has to be ON STAGE to be activated at all, and it is
    // a candidate like the others — the printed text says 「自分のステージにいる
    // 『みらくらぱーく！』のメンバー1人」, which includes the user.
    game.state.player1.stage.stage = [card, mira1, mira2];
    assert_eq!(
        game.state.player1.stage.stage[0],
        card,
        "precondition: the activating card is on the stage, in the left area"
    );
    let _ = outsider;
    game.drain_auto_ability_choices();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(card),
        None,
        None,
        None,
    )
    .unwrap();
    game.drain_auto_ability_choices();

    // The prompt is the whole point of the test, so it is asserted, not
    // tolerated: 「みらくらぱーく！のメンバー1人」 with three candidates must ask.
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "three みらくらぱーく！ members on stage: the effect must ask which one"
    );
    match game.get_pending_choice() {
        rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } => {
            assert_eq!(*count, 1, "「メンバー1人」 is a single card");
            assert!(
                zone.contains("stage"),
                "the candidates are the members on the stage, got zone {zone:?}"
            );
        }
        other => panic!("expected the choose-one prompt, got {other:?}"),
    }
    game.select_indices(&[pick]);
    game.drain_auto_ability_choices();

    let stage = game.state.player1.stage.stage;
    let winner = stage[pick];
    assert_eq!(
        game.state.mods.get_blade_modifier(winner),
        1,
        "the member in the CHOSEN area gains the blade, exactly one"
    );
    for (area, &id) in stage.iter().enumerate() {
        if area == pick {
            continue;
        }
        assert_eq!(
            game.state.mods.get_blade_modifier(id),
            0,
            "「1人」 means the other members get nothing (area {area})"
        );
    }
    let landed = stage
        .iter()
        .position(|&id| game.state.mods.get_blade_modifier(id) > 0)
        .unwrap_or_else(|| panic!("nobody gained the blade; stage {stage:?}"));
    (winner, landed)
}

/// The 起動 effect is a CHOICE: whichever area the player picks is the one that
/// gains the blade.
///
/// Was `assert!(b0 >= 1 || b1 >= 1 || b2 >= 1)` behind an `if let Some(choice)`
/// soft guard — so it passed whether the prompt appeared, whether the effect
/// auto-picked, and whichever card the engine happened to favour. Two runs
/// picking DIFFERENT areas are what separate a real choice from an auto-pick: an
/// auto-pick would grant to the same area both times and fail the second run.
#[test]
fn hs_cl1_choice_among_multiple_mirakura() {
    let (_center_winner, center_area) = blade_recipient(1);
    let (_right_winner, right_area) = blade_recipient(2);

    assert_eq!(center_area, 1, "picking area 1 grants the blade to area 1");
    assert_eq!(right_area, 2, "picking area 2 grants the blade to area 2");
    assert_ne!(
        center_area, right_area,
        "the two runs must land on different areas, or the second run is not \
         testing a choice at all"
    );
}
