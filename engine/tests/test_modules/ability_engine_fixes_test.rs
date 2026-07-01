use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// PL!SP-pb1-008-R (若菜四季) — 登場 trigger: draw → select area → move → conditional swap
// ====================================================================
// Full text:
//   カードを1枚引く。その後、登場したエリアとは別の自分のエリア1つを選ぶ。
//   このメンバーをそのエリアに移動する。選んだエリアにメンバーがいる場合、
//   そのメンバーは、このメンバーがいたエリアに移動させる。
//
// Parsed structure (after 移動する dispatch rule fix):
//   sequential [
//     draw_card(1),
//     sequential [
//       select(area, count=1),
//       position_change(no dest),
//       position_change(destination="same_area", condition=card_count_condition)
//     ]
//   ]
//
// Parts:
//   - Trigger: 登場 (debut, auto-fires on play_to_stage)
//   - Step 1: draw 1 card from deck to hand
//   - Step 2a: area_select choice (excludes current position)
//   - Step 2b: position_change uses selected area as destination
//   - Step 2c: conditional position_change with same_area → no-op (swap already done)
// ====================================================================

/// Play 若菜四季 to Center, select empty Right → card moves to Right.
#[test]
fn wakana_008_debut_move_to_empty_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wakana = game.id("PL!SP-pb1-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    // Fill deck so draw works
    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    game.add_to_hand(wakana);
    game.give_energy(15);
    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(wakana, MemberArea::Center);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "Deck should have 1 fewer card after draw"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "Should have drawn 1 card"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&wakana),
        "Wakana should not be in hand (played to stage)"
    );

    // Step 2a: area_select choice should appear
    let choice = game
        .state
        .ability_queue
        .is_waiting_for_choice()
        .cloned()
        .expect("Area select choice should be pending");
    // Verify it's a SelectTarget with area_select target
    match &choice {
        rabuka_engine::ability::types::Choice::SelectTarget {
            target, options, ..
        } => {
            assert_eq!(target, "area_select", "Choice target should be area_select");
            // Current position is Center(1), so options should exclude center
            if let Some(opts) = options {
                assert!(
                    !opts.contains(&"center".to_string()),
                    "Center should NOT be a valid option (current position)"
                );
                assert!(
                    opts.contains(&"left".to_string()),
                    "Left should be a valid option"
                );
                assert!(
                    opts.contains(&"right".to_string()),
                    "Right should be a valid option"
                );
            }
        }
        _ => panic!("Expected SelectTarget choice, got {:?}", choice),
    }

    // Select Right (index 2 among [left, right] → option 1 since we have 2 options)
    // Options are ["left", "right"], so option 1 = "right"
    game.select_option(1);

    // After position_change: wakana should be at Right, Center empty
    assert_eq!(
        game.state.player1.stage.stage[0], -1,
        "Left should be empty"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], -1,
        "Center should be empty (wakana moved out)"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], wakana,
        "Wakana should now be at Right"
    );

    // No further choice should be pending (same_area was a no-op)
    assert!(
        game.state.ability_queue.is_waiting_for_choice().is_none(),
        "No choice should remain after position change completes"
    );
}

/// Play 若菜四季 to Center where Right is occupied → swap occurs.
#[test]
fn wakana_008_debut_move_to_occupied_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wakana = game.id("PL!SP-pb1-008-R");
    let occupant = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.add_to_hand(wakana);
    game.give_energy(15);

    // Occupant at Right, Center empty for debut
    game.state.player1.stage.stage = [-1, -1, occupant];

    game.play_to_stage(wakana, MemberArea::Center);

    // Draw happened
    // Area select choice → pick Right
    assert!(
        game.has_pending_choice(),
        "Area select choice should appear"
    );

    // Options should exclude Center (current), include Left and Right
    // Options: ["left", "right"] → option 1 = "right"
    game.select_option(1);

    // After position_change (swap): occupant goes to Center, wakana goes to Right
    assert_eq!(
        game.state.player1.stage.stage[0], -1,
        "Left should be empty"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], occupant,
        "Occupant should now be at Center (swapped)"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], wakana,
        "Wakana should be at Right"
    );

    assert!(
        game.state.ability_queue.is_waiting_for_choice().is_none(),
        "No choice should remain"
    );
}

// ====================================================================
// PL!SP-bp2-008-R (若菜四季) — 起動 trigger: select area → move → conditional swap
// ====================================================================
// Text:
//   起動 ターン1回 E：このメンバーがいるエリアとは別の自分のエリア1つを選ぶ。
//   このメンバーをそのエリアに移動する。選んだエリアにメンバーがいる場合、
//   そのメンバーは、このメンバーがいたエリアに移動させる。
//
// Parts:
//   - Trigger: 起動 (activation, manual)
//   - Use limit: ターン1回 (once per turn)
//   - Cost: E (1 active energy)
//   - Effect: identical body to pb1-008-R (area select → position_change → same_area)
// ====================================================================

/// Activate bp2-008-R, pay cost, select empty area → move succeeds.
#[test]
fn wakana_bp2_008_activate_move_to_empty_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wakana = game.id("PL!SP-bp2-008-R");
    let _filler = game.id("PL!-sd1-010-SD");

    // Deploy wakana to stage manually (no debut ability test here)
    game.state.player1.stage.stage = [wakana, -1, -1];
    game.give_energy(1);

    let energy_before = game.state.player1.energy_zone.active_count();

    // Activate ability
    game.activate_ability(wakana);

    // Cost is paid first (1 energy)
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before - 1,
        "1 energy should be consumed"
    );

    // Then area_select choice appears
    let choice = game
        .state
        .ability_queue
        .is_waiting_for_choice()
        .cloned()
        .expect("Area select choice should appear after cost payment");
    match &choice {
        rabuka_engine::ability::types::Choice::SelectTarget {
            target, options, ..
        } => {
            assert_eq!(target, "area_select", "Choice target should be area_select");
            // Current position is Left(0), so options should exclude left
            if let Some(opts) = options {
                assert!(
                    !opts.contains(&"left".to_string()),
                    "Left should NOT be a valid option (current position)"
                );
                assert!(
                    opts.contains(&"center".to_string()),
                    "Center should be valid"
                );
                assert!(opts.contains(&"right".to_string()), "Right should be valid");
            }
        }
        _ => panic!("Expected SelectTarget choice, got {:?}", choice),
    }

    // Select Center → wakana moves from Left to Center
    // Options: ["center", "right"] → option 0 = "center"
    game.select_option(0);

    assert_eq!(
        game.state.player1.stage.stage[0], -1,
        "Left should be empty after move"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], wakana,
        "Wakana should be at Center"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], -1,
        "Right should still be empty"
    );

    assert!(
        game.state.ability_queue.is_waiting_for_choice().is_none(),
        "No choice should remain"
    );
}

/// Activate bp2-008-R, swap with occupant at selected area.
#[test]
fn wakana_bp2_008_activate_move_to_occupied_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wakana = game.id("PL!SP-bp2-008-R");
    let occupant = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [wakana, occupant, -1];
    game.give_energy(1);

    game.activate_ability(wakana);

    // Cost paid, area select appears
    assert!(
        game.has_pending_choice(),
        "Area select should appear after cost"
    );

    // Current position is Left(0), options: ["center", "right"]
    // Select Center → option 0 (occupied → triggers swap)
    game.select_option(0);

    // Swap: occupant→Left, wakana→Center
    assert_eq!(
        game.state.player1.stage.stage[0], occupant,
        "Occupant should be at Left (swapped)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], wakana,
        "Wakana should be at Center (swapped)"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], -1,
        "Right should be empty"
    );

    assert!(
        game.state.ability_queue.is_waiting_for_choice().is_none(),
        "No choice should remain"
    );
}

/// Use limit: second activation in same turn is blocked.
#[test]
fn wakana_bp2_008_use_limit_blocks_second_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wakana = game.id("PL!SP-bp2-008-R");
    let _filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [wakana, -1, -1];
    game.give_energy(3);

    // First activation: succeeds
    game.activate_ability(wakana);
    // Pay cost, select Center
    assert!(
        game.has_pending_choice(),
        "Area select for first activation"
    );
    game.select_option(0); // Center
    assert!(
        game.state.player1.energy_zone.active_count() == 2,
        "1 energy spent, 2 remaining"
    );

    // Second activation: should fail due to use limit
    let result = game.try_activate_ability(wakana);
    assert!(result.is_err(), "Second activation should fail (use limit)");
}

// ====================================================================
// PL!SP-pb1-008-R (若菜四季) — ADDITIONAL EDGE CASE TESTS
// ====================================================================

fn setup_wakana_debut_game(deck_count: usize) -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakana = game.id("PL!SP-pb1-008-R");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..deck_count {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.add_to_hand(wakana);
    game.give_energy(15);
    (game, wakana, filler)
}

/// Area_select choice is NOT a yes/no binary — it must be SelectTarget with
/// target="area_select", allow_skip=false, and valid area options.
#[test]
fn wakana_008_choice_is_select_target_not_yes_no() {
    let (mut game, wakana, _) = setup_wakana_debut_game(3);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(wakana, MemberArea::Center);

    let choice = game.get_pending_choice();
    match choice {
        rabuka_engine::ability::types::Choice::SelectTarget {
            target,
            allow_skip,
            options,
            ..
        } => {
            assert_eq!(target, "area_select", "Must be area_select, not yes/no");
            assert!(!allow_skip, "Area selection must NOT be skippable");
            if let Some(opts) = options {
                assert!(!opts.contains(&"center".to_string()), "Center excluded");
                assert!(opts.contains(&"left".to_string()), "Left is option");
                assert!(opts.contains(&"right".to_string()), "Right is option");
            }
        }
        _ => panic!("Choice must be SelectTarget(area_select), got {:?}", choice),
    }
}

/// Play to Center, move to empty Left.
#[test]
fn wakana_008_debut_center_to_empty_left() {
    let (mut game, wakana, _) = setup_wakana_debut_game(3);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(wakana, MemberArea::Center);
    game.select_option(0); // Left (index 0 in ["left", "right"])

    assert_eq!(game.state.player1.stage.stage[0], wakana, "Wakana at Left");
    assert_eq!(game.state.player1.stage.stage[1], -1, "Center empty");
    assert_eq!(game.state.player1.stage.stage[2], -1, "Right empty");
    assert!(!game.has_pending_choice(), "No remaining choice");
}

/// Play to Center, swap with occupant at Left.
#[test]
fn wakana_008_debut_center_swap_with_left_occupant() {
    let (mut game, wakana, filler) = setup_wakana_debut_game(3);
    game.state.player1.stage.stage = [filler, -1, -1];
    game.play_to_stage(wakana, MemberArea::Center);
    game.select_option(0); // Left

    assert_eq!(game.state.player1.stage.stage[0], wakana, "Wakana at Left");
    assert_eq!(
        game.state.player1.stage.stage[1], filler,
        "Filler swapped to Center"
    );
    assert_eq!(game.state.player1.stage.stage[2], -1, "Right empty");
}

/// Play to Left, move to empty Center.
#[test]
fn wakana_008_debut_left_to_empty_center() {
    let (mut game, wakana, _) = setup_wakana_debut_game(3);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(wakana, MemberArea::LeftSide);

    // Options should exclude Left, include Center and Right
    let choice = game.get_pending_choice();
    if let rabuka_engine::ability::types::Choice::SelectTarget { options, .. } = choice {
        if let Some(opts) = options {
            assert!(!opts.contains(&"left".to_string()), "Left excluded");
            assert!(opts.contains(&"center".to_string()), "Center is option");
            assert!(opts.contains(&"right".to_string()), "Right is option");
        }
    }
    game.select_option(0); // Center (index 0 in ["center", "right"])

    assert_eq!(game.state.player1.stage.stage[0], -1, "Left empty");
    assert_eq!(
        game.state.player1.stage.stage[1], wakana,
        "Wakana at Center"
    );
    assert_eq!(game.state.player1.stage.stage[2], -1, "Right empty");
}

/// Play to Left, swap with occupant at Center.
#[test]
fn wakana_008_debut_left_swap_with_center_occupant() {
    let (mut game, wakana, filler) = setup_wakana_debut_game(3);
    game.state.player1.stage.stage = [-1, filler, -1];
    game.play_to_stage(wakana, MemberArea::LeftSide);
    game.select_option(0); // Center (index 0 in ["center", "right"])

    assert_eq!(
        game.state.player1.stage.stage[0], filler,
        "Filler swapped to Left"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], wakana,
        "Wakana at Center"
    );
    assert_eq!(game.state.player1.stage.stage[2], -1, "Right empty");
}

/// Play to Right, move to empty Center.
#[test]
fn wakana_008_debut_right_to_empty_center() {
    let (mut game, wakana, _) = setup_wakana_debut_game(3);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(wakana, MemberArea::RightSide);

    let choice = game.get_pending_choice();
    if let rabuka_engine::ability::types::Choice::SelectTarget { options, .. } = choice {
        if let Some(opts) = options {
            assert!(!opts.contains(&"right".to_string()), "Right excluded");
            assert!(opts.contains(&"left".to_string()), "Left is option");
            assert!(opts.contains(&"center".to_string()), "Center is option");
        }
    }
    // Options: ["left", "center"] → option 0 = "left"
    game.select_option(0); // Left

    assert_eq!(game.state.player1.stage.stage[0], wakana, "Wakana at Left");
    assert_eq!(game.state.player1.stage.stage[1], -1, "Center empty");
    assert_eq!(game.state.player1.stage.stage[2], -1, "Right empty");
}

/// Play to Right, swap with occupant at Center.
#[test]
fn wakana_008_debut_right_swap_with_center_occupant() {
    let (mut game, wakana, filler) = setup_wakana_debut_game(3);
    game.state.player1.stage.stage = [-1, filler, -1];
    game.play_to_stage(wakana, MemberArea::RightSide);
    game.select_option(1); // Center (index 1 in ["left", "center"])

    assert_eq!(game.state.player1.stage.stage[0], -1, "Left empty");
    assert_eq!(
        game.state.player1.stage.stage[1], wakana,
        "Wakana at Center"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], filler,
        "Filler swapped to Right"
    );
}

/// Both other areas occupied → still must select one, swap occurs.
#[test]
fn wakana_008_debut_both_occupied_swap_either() {
    let (mut game, wakana, filler) = setup_wakana_debut_game(3);
    let occupant2 = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [filler, -1, occupant2];
    game.play_to_stage(wakana, MemberArea::Center);

    // Both Left and Right are options (Center excluded)
    let choice = game.get_pending_choice();
    if let rabuka_engine::ability::types::Choice::SelectTarget { options, .. } = choice {
        if let Some(opts) = options {
            assert!(opts.contains(&"left".to_string()), "Left is option");
            assert!(opts.contains(&"right".to_string()), "Right is option");
            assert_eq!(opts.len(), 2, "Exactly 2 options");
        }
    }
    game.select_option(0); // Left → swap with filler

    assert_eq!(game.state.player1.stage.stage[0], wakana, "Wakana at Left");
    assert_eq!(
        game.state.player1.stage.stage[1], filler,
        "Filler at Center"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], occupant2,
        "Occupant2 at Right"
    );
}

/// Draw when deck has exactly 1 card → draw succeeds, area change still happens.
#[test]
fn wakana_008_debut_draw_last_card_then_move() {
    let (mut game, wakana, filler) = setup_wakana_debut_game(1);
    game.state.player1.main_deck.cards.push(filler);
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(wakana, MemberArea::Center);

    // play_to_stage removes Wakana from hand, debut draw adds 1 → net hand change = 0
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "Last card drawn from deck"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Hand size: -1 Wakana +1 draw = unchanged"
    );

    game.select_option(0); // Left
    assert_eq!(
        game.state.player1.stage.stage[0], wakana,
        "Wakana at Left after draw+move"
    );
}

/// Draw when deck is empty → draw is a no-op, area change still happens.
#[test]
fn wakana_008_debut_empty_deck_draw_noop_then_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakana = game.id("PL!SP-pb1-008-R");
    game.add_to_hand(wakana);
    game.give_energy(15);
    game.state.player1.main_deck.cards.clear();
    let hand_before = game.state.player1.hand.cards.len();

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(wakana, MemberArea::Center);

    // Wakana removed from hand, empty deck draw adds nothing → hand = hand_before - 1
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "Hand has 1 fewer card (Wakana played, empty deck draw adds nothing)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "Deck remains empty"
    );

    // Area change STILL happens even though draw did nothing
    game.select_option(0); // Left
    assert_eq!(
        game.state.player1.stage.stage[0], wakana,
        "Wakana at Left despite empty deck"
    );
}

/// Position change events are cleared after ability resolution, but
/// position_change_occurred_this_turn flag persists — verify it's set
/// after moving to an empty area.
#[test]
fn wakana_008_debut_position_change_flag_empty() {
    let (mut game, wakana, _) = setup_wakana_debut_game(3);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(wakana, MemberArea::Center);
    game.select_option(0); // Left (empty)

    assert!(
        game.state.position_change_occurred_this_turn,
        "position_change_occurred_this_turn should be true after move to empty"
    );
}

/// position_change_occurred_this_turn is set after a swap.
#[test]
fn wakana_008_debut_position_change_flag_swap() {
    let (mut game, wakana, filler) = setup_wakana_debut_game(3);
    game.state.player1.stage.stage = [-1, -1, filler];
    game.play_to_stage(wakana, MemberArea::Center);
    game.select_option(1); // Right (occupied)

    assert!(
        game.state.position_change_occurred_this_turn,
        "position_change_occurred_this_turn should be true after swap"
    );
}

// ====================================================================
// PL!SP-bp2-008-R (若菜四季) — 起動 ADDITIONAL EDGE CASE TESTS
// ====================================================================

/// Activate from Left, select empty Right.
#[test]
fn wakana_bp2_008_activate_left_to_empty_right() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakana = game.id("PL!SP-bp2-008-R");
    game.state.player1.stage.stage = [wakana, -1, -1];
    game.give_energy(1);
    game.activate_ability(wakana);

    // Options: ["center", "right"] → option 1 = "right"
    game.select_option(1);

    assert_eq!(game.state.player1.stage.stage[0], -1, "Left empty");
    assert_eq!(game.state.player1.stage.stage[1], -1, "Center empty");
    assert_eq!(game.state.player1.stage.stage[2], wakana, "Wakana at Right");
}

/// Activate from Center, select empty Left.
#[test]
fn wakana_bp2_008_activate_center_to_empty_left() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakana = game.id("PL!SP-bp2-008-R");
    game.state.player1.stage.stage = [-1, wakana, -1];
    game.give_energy(1);
    game.activate_ability(wakana);

    // Options: ["left", "right"] → option 0 = "left"
    game.select_option(0);

    assert_eq!(game.state.player1.stage.stage[0], wakana, "Wakana at Left");
    assert_eq!(game.state.player1.stage.stage[1], -1, "Center empty");
    assert_eq!(game.state.player1.stage.stage[2], -1, "Right empty");
}

/// Activate from Right, swap with occupant at Center.
#[test]
fn wakana_bp2_008_activate_right_swap_with_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakana = game.id("PL!SP-bp2-008-R");
    let occupant = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, occupant, wakana];
    game.give_energy(1);
    game.activate_ability(wakana);

    // Options: ["left", "center"] (Right is current, excluded)
    game.select_option(1); // Center

    assert_eq!(game.state.player1.stage.stage[0], -1, "Left empty");
    assert_eq!(
        game.state.player1.stage.stage[1], wakana,
        "Wakana at Center"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], occupant,
        "Occupant swapped to Right"
    );
}

/// Activate choice is SelectTarget(area_select), not yes/no.
#[test]
fn wakana_bp2_008_choice_is_select_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakana = game.id("PL!SP-bp2-008-R");
    game.state.player1.stage.stage = [wakana, -1, -1];
    game.give_energy(1);
    game.activate_ability(wakana);

    let choice = game.get_pending_choice();
    match choice {
        rabuka_engine::ability::types::Choice::SelectTarget {
            target, allow_skip, ..
        } => {
            assert_eq!(target, "area_select", "Must be area_select, not yes/no");
            assert!(
                !allow_skip,
                "Area selection must NOT be skippable for activation ability"
            );
        }
        _ => panic!("Choice must be SelectTarget(area_select), got {:?}", choice),
    }
}

/// With 0 active energy, the cost (E = 1 active energy) can't be paid,
/// so the ability effect is skipped entirely — no area select appears.
#[test]
fn wakana_bp2_008_no_energy_skips_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakana = game.id("PL!SP-bp2-008-R");
    game.state.player1.stage.stage = [wakana, -1, -1];
    // No energy given
    game.activate_ability(wakana);
    // Energy stays 0 (nothing to consume)
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "No energy consumed"
    );
    // No area select appears — effect is skipped because cost can't be paid
    assert!(
        !game.has_pending_choice(),
        "With 0 energy, cost can't be paid so effect is skipped entirely"
    );
}

/// Activate from Center, swap: position_change_occurred_this_turn is set.
#[test]
fn wakana_bp2_008_position_change_flag_swap() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakana = game.id("PL!SP-bp2-008-R");
    let occupant = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, wakana, occupant];
    game.give_energy(1);
    game.activate_ability(wakana);
    game.select_option(1); // Right (occupied)

    assert!(
        game.state.position_change_occurred_this_turn,
        "Flag set after activation position change swap"
    );
}

// ====================================================================
// PL!SP-bp5-013-N (唐 可可) — 登場: look 5, select SunnyPassion member OR
//   Liella! member with blade heart, discard rest
// ====================================================================
// Full text:
//   手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。
//   その中から『SunnyPassion』のメンバーカードかブレードハートを持つ
//   『Liella!』のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。
// ====================================================================

/// SunnyPassion member (no blade heart needed) should be selectable.
#[test]
fn keke_look_select_sunny_passion_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let keke = game.id("PL!SP-bp5-013-N");
    let filler = game.id("PL!-sd1-010-SD");
    // SunnyPassion member (unit=SunnyPassion, no blade_heart)
    let sunny = game.id("PL!SP-bp5-111-R");

    game.state.player1.hand.cards.push(keke);
    game.state.player1.hand.cards.push(filler);
    // Deck top 5: [sunny, filler, filler, filler, filler]
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(sunny);
    for _ in 0..4 {
        game.state.player1.main_deck.cards.push(filler);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(keke, MemberArea::Center);

    // Pay optional cost: discard 1 from hand (filler)
    if game.has_pending_choice() {
        game.select_indices(&[0]); // discard index 0 (the filler)
    }

    // The choice should now be to select 1 from the matching looked-at cards.
    if game.has_pending_choice() {
        // Select the first visible card (should be Sunny)
        game.select_indices(&[0]);
    }

    // Sunny should now be in hand
    assert!(
        game.state.player1.hand.cards.contains(&sunny),
        "SunnyPassion member should be added to hand"
    );
}

/// Liella! member WITHOUT blade heart should NOT be selectable.
#[test]
fn keke_look_select_liella_no_blade_heart_excluded() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let keke = game.id("PL!SP-bp5-013-N");
    let filler = game.id("PL!-sd1-010-SD");
    // Liella! member (series=スーパースター) without blade_heart
    let liella_no_bh = game.id("PL!SP-bp1-013-PR");

    game.state.player1.hand.cards.push(keke);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(liella_no_bh);
    for _ in 0..4 {
        game.state.player1.main_deck.cards.push(filler);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(keke, MemberArea::Center);

    // Drain all choices: optional cost (pay), then look+filter, then any remaining prompts
    while game.has_pending_choice() {
        // Pay optional cost: first prompt is Yes/No, second is select card to discard
        game.select_indices(&[0]);
    }

    // After all choices resolved, liella_no_bh should have been discarded
    // since it's Liella! but lacks blade_heart.
    assert!(!game.has_pending_choice(), "All choices should be resolved");
    assert!(
        !game.state.player1.hand.cards.contains(&liella_no_bh),
        "Liella member without blade heart should NOT be in hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&liella_no_bh),
        "Liella member without blade heart should be in waitroom"
    );
}

/// Liella! member WITH blade heart should be selectable.
#[test]
fn keke_look_select_liella_with_blade_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let keke = game.id("PL!SP-bp5-013-N");
    let filler = game.id("PL!-sd1-010-SD");
    // Liella! member (series=スーパースター) WITH blade_heart
    let liella_bh = game.id("PL!SP-pb1-001-PR");

    game.state.player1.hand.cards.push(keke);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(liella_bh);
    for _ in 0..4 {
        game.state.player1.main_deck.cards.push(filler);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(4);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(keke, MemberArea::Center);

    // Pay optional cost
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Liella member with blade heart should be selectable
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&liella_bh),
        "Liella member with blade heart should be added to hand"
    );
}

// ====================================================================
// PL!SP-pb1-003-R (嵐 千砂都) — 登場 trigger: rotation
// ====================================================================
// Text:
//   自分のステージにいるメンバーが『5yncri5e!』のみの場合、自分と対戦相手は、
//   センター→左、左→右、右→センターにそれぞれ移動させる。
//
// Parts:
//   - Trigger: 登場 (debut)
//   - Condition: all stage members are 5yncri5e!
//   - Target: both (self then opponent)
//   - Rotation: center→left, left→right, right→center
// ====================================================================

/// Full 3-member 5yncri5e! rotation on self stage.
#[test]
fn chisato_003_rotation_full_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chisato = game.id("PL!SP-pb1-003-R");
    // Two other 5yncri5e! members (use N rarity for filler status but same unit)
    let member_a = game.id("PL!SP-pb1-014-N");
    let member_b = game.id("PL!SP-pb1-019-N");

    game.add_to_hand(chisato);
    game.give_energy(9);

    // Set up: 2 existing 5yncri5e! members at Left and Center
    game.state.player1.stage.stage = [member_a, member_b, -1];
    // We'll play chisato to Right

    game.play_to_stage(chisato, MemberArea::RightSide);

    // Debut trigger fires, condition check passes (all are 5yncri5e!)
    // Rotation should have happened automatically:
    // Before: [member_a, member_b, chisato]
    // After:  [member_b, chisato, member_a]
    //   left(0)←center(1): member_b←old member_b (stays... wait)
    //   Actually rotation: center→left, left→right, right→center
    //   center(1)→left(0): member_b moves to left
    //   left(0)→right(2): member_a moves to right
    //   right(2)→center(1): chisato moves to center
    //   Result: [member_b, chisato, member_a]

    assert!(
        game.state.ability_queue.is_waiting_for_choice().is_none(),
        "Rotation should complete without choices (predetermined destinations)"
    );

    assert_eq!(
        game.state.player1.stage.stage[0], member_b,
        "Left should have the member that was at Center (rotation: center→left)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], chisato,
        "Center should have the member that was at Right (rotation: right→center)"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], member_a,
        "Right should have the member that was at Left (rotation: left→right)"
    );
}

/// Partial stage (2 members): only occupied positions rotate.
#[test]
fn chisato_003_rotation_partial_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chisato = game.id("PL!SP-pb1-003-R");
    let member_a = game.id("PL!SP-pb1-014-N");

    game.add_to_hand(chisato);
    game.give_energy(9);

    // Only 1 other 5yncri5e! member at Left
    game.state.player1.stage.stage = [member_a, -1, -1];

    game.play_to_stage(chisato, MemberArea::Center);

    // Before: [member_a, chisato, -1]
    // Rotation: center(1)→left(0): chisato moves to Left
    //           left(0)→right(2): member_a moves to Right
    //           right(2)→center(1): empty, nothing moves
    // Result: [chisato, -1, member_a]

    assert!(
        game.state.ability_queue.is_waiting_for_choice().is_none(),
        "Rotation should complete without choices"
    );

    assert_eq!(
        game.state.player1.stage.stage[0], chisato,
        "Left should have chisato (center→left)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], -1,
        "Center should be empty (right was empty)"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], member_a,
        "Right should have member_a (left→right)"
    );
}

/// Condition fails when non-5yncri5e! member on stage → no rotation.
#[test]
fn chisato_003_rotation_condition_fails_with_non_5yncri5e() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chisato = game.id("PL!SP-pb1-003-R");
    let non_5yncri5e = game.id("PL!-sd1-010-SD");

    game.add_to_hand(chisato);
    game.give_energy(9);

    game.state.player1.stage.stage = [non_5yncri5e, -1, -1];

    game.play_to_stage(chisato, MemberArea::Center);

    // Condition fails → no rotation, card stays in its debut position
    assert_eq!(
        game.state.player1.stage.stage[0], non_5yncri5e,
        "Left should still have non-5yncri5e member"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], chisato,
        "Center should still have chisato (no rotation)"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], -1,
        "Right should still be empty"
    );
}

/// Single occupant 5yncri5e! only.
#[test]
fn chisato_003_rotation_single_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chisato = game.id("PL!SP-pb1-003-R");

    game.add_to_hand(chisato);
    game.give_energy(9);

    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(chisato, MemberArea::Center);

    // Before: [-1, chisato, -1]
    // Rotation: center(1)→left(0): chisato moves to Left
    //           left was empty, right was empty → result: [chisato, -1, -1]

    assert_eq!(
        game.state.player1.stage.stage[0], chisato,
        "Left should have chisato (center→left rotation of solo member)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], -1,
        "Center should be empty after move"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], -1,
        "Right should still be empty"
    );
}

// ====================================================================
// PL!SP-pb1-009-R (鬼塚夏美) — 登場 trigger: conditional draw
// ====================================================================
// Text:
//   自分のステージにほかの『5yncri5e!』のメンバーがいる場合、カードを1枚引く。
//
// Parts:
//   - Trigger: 登場 (debut)
//   - Condition: group_condition(5yncri5e!, location=stage, exclude_self=true)
//   - Effect: draw_card(1)
// ====================================================================

/// Other 5yncri5e! member on stage → draw 1.
#[test]
fn natsumi_009_draw_with_other_5yncri5e() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let natsumi = game.id("PL!SP-pb1-009-R");
    let other_5yncri5e = game.id("PL!SP-pb1-014-N");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    game.add_to_hand(natsumi);
    game.give_energy(4);

    game.state.player1.stage.stage = [other_5yncri5e, -1, -1];

    game.play_to_stage(natsumi, MemberArea::Center);

    assert!(
        !game.state.player1.hand.cards.contains(&natsumi),
        "Natsumi should not be in hand (played to stage)"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "Deck should have 1 fewer card after draw"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "Should draw 1 card when other 5yncri5e! on stage"
    );
}

/// No other 5yncri5e! member → no draw.
#[test]
fn natsumi_009_no_draw_when_alone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let natsumi = game.id("PL!SP-pb1-009-R");
    let non_5yncri5e = game.id("PL!-sd1-010-SD");

    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(non_5yncri5e);
    }
    let hand_before = game.state.player1.hand.cards.len();

    game.add_to_hand(natsumi);
    game.give_energy(4);

    game.state.player1.stage.stage = [-1, -1, -1];

    game.play_to_stage(natsumi, MemberArea::Center);

    assert!(
        !game.state.player1.hand.cards.contains(&natsumi),
        "Natsumi should not be in hand (played to stage)"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Should NOT draw when no other 5yncri5e! on stage"
    );
}

/// Non-5yncri5e! member on stage instead → no draw.
#[test]
fn natsumi_009_no_draw_with_non_5yncri5e() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let natsumi = game.id("PL!SP-pb1-009-R");
    let non_5yncri5e = game.id("PL!-sd1-010-SD");

    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(non_5yncri5e);
    }
    let hand_before = game.state.player1.hand.cards.len();

    game.add_to_hand(natsumi);
    game.give_energy(4);

    game.state.player1.stage.stage = [non_5yncri5e, -1, -1];

    game.play_to_stage(natsumi, MemberArea::Center);

    assert!(
        !game.state.player1.hand.cards.contains(&natsumi),
        "Natsumi should not be in hand (played to stage)"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Should NOT draw when only non-5yncri5e! on stage"
    );
}

// ====================================================================
// Duplicate card_no regression: second copy in hand should not break exclude_self
// ====================================================================
// Bug: trigger_debut_abilities found the card on stage but passed None for
// explicit_card_id, causing find_card_by_number_for_player to search hand
// first. If a second copy of the same card_no was in hand, activating_card_id
// pointed to the hand copy, making exclude_self fail because the hand copy
// wasn't on stage.
//
// Fix: pass the stage card_id as explicit_card_id so the unique i16 is used.
// ====================================================================

/// Two copies of PL!SP-pb1-009-R in hand. Play one to empty stage.
/// The second copy in hand should NOT cause a false draw.
#[test]
fn natsumi_009_no_draw_with_duplicate_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let natsumi1 = game.id("PL!SP-pb1-009-R");
    let natsumi2 = game.id("PL!SP-pb1-009-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.add_to_hand(natsumi1);
    game.add_to_hand(natsumi2);
    game.give_energy(4);

    game.state.player1.stage.stage = [-1, -1, -1];
    let hand_before = game.state.player1.hand.cards.len(); // 2: natsumi1 + natsumi2
    game.play_to_stage(natsumi1, MemberArea::Center);

    assert!(
        !game.state.player1.hand.cards.contains(&natsumi1),
        "natsumi1 should not be in hand (played to stage)"
    );
    assert!(
        game.state.player1.hand.cards.contains(&natsumi2),
        "natsumi2 should still be in hand"
    );
    // After play: natsumi1 removed from hand → hand = [natsumi2] → len = 1
    // If draw occurred: hand = [natsumi2, drawn_card] → len = 2
    let hand_after = game.state.player1.hand.cards.len();
    assert!(
        hand_after == hand_before - 1,
        "Should NOT draw when playing to empty stage with duplicate in hand. hand {} -> {}",
        hand_before,
        hand_after
    );
}

// ====================================================================
// PL!SP-pb1-001-R (澁谷かのん ab#0) — unless-pay (しないかぎり) pattern
// LiveStart: {{E}}{{E}}支払わないかぎり、自分の手札を2枚控え室に置く。
// ====================================================================

fn advance_to_live_card_set_p1_kanon(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start_kanon(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Pay 2 energy → discard effect is skipped (hand unchanged).
#[test]
fn kanon_unless_pay_pay_avoids_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    // Kanon is a member — play her to stage for live_start ability
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(13);

    // Seed deck for yell + draws
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(live_card);
    // Play Kanon to Center, then set live card
    game.play_to_stage(kanon, rabuka_engine::zones::MemberArea::Center);

    advance_to_live_card_set_p1_kanon(&mut game);
    game.set_live_card(live_card);
    // Record hand AFTER live card is set (removed from hand)
    let hand_before = game.state.player1.hand.cards.len();
    // Passing from LiveCardSetFirstAttacker draws 1 card per live card placed
    advance_to_live_start_kanon(&mut game);

    // Kanon's ability fires: "unless pay 2, discard 2"
    // Pay 2 energy to avoid discard
    if game.has_pending_choice() {
        game.select_option(1);
    }

    let hand_after = game.state.player1.hand.cards.len();
    // LiveCardSet pass draws 1 card; card selection prompt might exist but no discard
    assert_eq!(
        hand_after,
        hand_before + 1,
        "Paying 2 energy should avoid discard (hand {} -> {})",
        hand_before,
        hand_after
    );
    let energy_after = game.state.player1.energy_zone.active_count();
    assert!(
        energy_after < 11,
        "2 energy should be consumed (had 11 after playing kanon, now {})",
        energy_after
    );
    // Consume any remaining choices
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

/// Skip paying → discard effect fires (2 cards removed from hand).
#[test]
fn kanon_unless_pay_skip_triggers_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    // Kanon is a member — play her to stage for live_start ability
    // Hand: kanon + 3 extra fillers to have cards after play_to_stage
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(live_card);
    game.give_energy(13);

    // Seed deck for yell + draws
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Play Kanon to Center (removes kanon from hand)
    game.play_to_stage(kanon, rabuka_engine::zones::MemberArea::Center);
    // After play_to_stage: hand has 3 fillers + live_card = 4

    advance_to_live_card_set_p1_kanon(&mut game);
    game.set_live_card(live_card);
    // Record hand AFTER live card is set (removed from hand)
    let hand_before = game.state.player1.hand.cards.len();
    // Passing from LiveCardSetFirstAttacker draws 1 card per live card placed
    advance_to_live_start_kanon(&mut game);

    // Kanon's ability fires: "unless pay 2, discard 2"
    // Skip paying → discard 2 should fire → then choose first 2 cards to discard
    if game.has_pending_choice() {
        game.select_option(0); // skip (don't pay)
    }
    // Card selection prompt for which 2 to discard
    if game.has_pending_choice() {
        game.try_select_indices(&[0, 1]).unwrap(); // discard first 2 cards
    }

    let hand_after = game.state.player1.hand.cards.len();
    // LiveCardSet pass draws 1 card; discard removes 2 → hand = hand_before - 1
    assert_eq!(
        hand_after,
        hand_before - 1,
        "Skipping payment should discard 2 cards from hand (hand {} -> {}; expected {})",
        hand_before,
        hand_after,
        hand_before - 1
    );
    // Energy should NOT be consumed (we skipped payment)
    let energy_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        energy_after, 2,
        "Energy should remain 2 when skipping payment (was 2, now {})",
        energy_after
    );
    // Consume any remaining choices
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    let energy_after = game.state.player1.energy_zone.active_count();
    assert!(
        energy_after < 11,
        "2 energy should be consumed (had 11 after playing kanon, now {})",
        energy_after
    );
}

// ====================================================================
// PL!N-pb1-004-R 朝香果林 — Constant: blade 2 if not_moved; LiveStart: reveal+position_change
// ====================================================================

fn fill_decks_kari(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_live_start_kari(game: &mut TestGame, live_card: i16) {
    game.add_to_hand(live_card);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    game.pass(); // LiveCardSetSecondAttacker → LiveStart triggers
    game.pass(); // Live
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }
}

/// When Karin moves via her LiveStart position_change, her constant blade bonus
/// (not_moved condition) must be re-evaluated immediately so it drops to 0.
#[test]
fn karin_position_change_removes_not_moved_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks_kari(&mut game);

    // Put a member with cost≤9 at deck index 1 (index 0 gets drawn during phase advance
    // to LiveCardSetFirstAttacker, exposing our card as the new top for LiveStart reveal).
    let cheap_member = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.insert(1, cheap_member);

    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    // Before LiveStart: Karin has not moved → gets blade 2.
    game.state.recalculate_constants();
    let blade_before = game.state.mods.get_blade_modifier(karin);
    assert_eq!(blade_before, 2, "Karin should have +2 blade before moving");

    // Record her initial position so we can verify she moved.
    let pos_before = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .position(|&c| c == karin);
    assert_eq!(pos_before, Some(1), "Karin starts at Center");

    // Advance to LiveStart — fires Karin's ab#1 which reveals top card (cheap_member,
    // cost≤9 member), moves it to hand, and position-changes Karin.
    advance_to_live_start_kari(&mut game, live_card);

    // Verify: cheap_member was added to hand (not discarded).
    assert!(
        game.state.player1.hand.cards.contains(&cheap_member),
        "cheap member card should be in hand after LiveStart"
    );

    // Verify: Karin moved.
    let pos_after = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .position(|&c| c == karin);
    assert!(
        pos_after != Some(1),
        "Karin should have moved from Center after position change"
    );
    assert!(
        pos_after.is_some(),
        "Karin should still be on stage (not removed)"
    );

    // Verify: after position change, constant ability is re-evaluated and blade is 0.
    let blade_after = game.state.mods.get_blade_modifier(karin);
    assert_eq!(
        blade_after, 0,
        "Karin's blade should be 0 after moving (not_moved condition fails)"
    );

    // Pass through the rest of the Live phase into the next turn.
    // Sequence: FirstAttackerPerformance → SecondAttackerPerformance →
    // LiveVictoryDetermination → End → Active (next turn).
    // During LiveVictoryDetermination, cards_moved_this_turn is cleared.
    // Next turn's Active phase runs recalculate_constants() → Karin hasn't
    // moved this turn → not_moved passes again → blade comes back.
    game.pass(); // → SecondAttackerPerformance
    game.pass(); // → LiveVictoryDetermination
    game.pass(); // → End
    game.pass(); // → Active (next turn, recalculate_constants runs)

    let blade_next_turn = game.state.mods.get_blade_modifier(karin);
    assert_eq!(
        blade_next_turn, 2,
        "Karin's blade should return to 2 in the next turn (movement tracking was reset)"
    );
}
