/// Tests for movement_condition (エリアを移動した) — two paths:
///   1. timing_condition on gain_resource (Aspire): per-card filter during blade application
///   2. movement_condition in condition evaluator: position_change + record_card_movement
///
/// Card: PL!SP-sd2-025-P (Aspire) — LiveStart blade for moved Liella! members
///   Text: ライブ開始時 ライブ終了時まで、自分のステージにいる、
///         このターン中にエリアを移動したすべての『Liella!』のメンバーは、ブレードを得る。
///
/// Card: PL!SP-bp2-003-R (嵐千砂都) — movement trigger via condition evaluator
///   Text: 自動 ターン1回 このメンバーがエリアを移動したとき、自分のエネルギーデッキから～ energy placement
use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_live_start(game: &mut TestGame, live_card: i16) {
    game.add_to_hand(live_card);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    game.pass(); // LiveCardSetP2
    game.pass(); // LiveStart
    while game.has_pending_choice() {
        game.drain_auto_ability_choices();
    }
}

// ====================================================================
// PATH 1: timing_condition on gain_resource (Aspire-style)
// Live card's LiveStart ability grants blade to Liella! members who moved this turn.
// ====================================================================

fn setup_aspire_game() -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    game.give_energy(10);
    game
}

/// Moved Liella! member gets blade. Unmoved Liella! gets 0.
/// Moved non-Liella! on opponent stage gets 0.
#[test]
fn aspire_moved_liella_gains_blade_unmoved_and_non_liella_do_not() {
    let mut game = setup_aspire_game();
    let aspire = game.id("PL!SP-sd2-025-P");
    let moved_liella = game.id("PL!SP-sd1-001-SD");
    let unmoved_liella = game.id("PL!SP-sd1-002-SD");
    let moved_non_liella = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = moved_liella;
    game.state.player1.stage.stage[2] = unmoved_liella;
    game.state.player2.stage.stage[0] = moved_non_liella;
    game.state.cards_moved_this_turn.insert(moved_liella);
    game.state.position_change_occurred_this_turn = true;

    advance_to_live_start(&mut game, aspire);

    assert_eq!(
        game.state.mods.get_blade_modifier(moved_liella),
        1,
        "Moved Liella! should get blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(unmoved_liella),
        0,
        "Unmoved Liella! should get 0"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(moved_non_liella),
        0,
        "Moved non-Liella! should get 0"
    );
}

/// Only a non-Liella! member moved → 0 blade (group filter excludes it).
#[test]
fn aspire_only_non_liella_moved_no_blade() {
    let mut game = setup_aspire_game();
    let aspire = game.id("PL!SP-sd2-025-P");
    let non_liella = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = non_liella;
    game.state.cards_moved_this_turn.insert(non_liella);
    game.state.position_change_occurred_this_turn = true;

    advance_to_live_start(&mut game, aspire);

    assert_eq!(
        game.state.mods.get_blade_modifier(non_liella),
        0,
        "Non-Liella! moved should get 0"
    );
}

/// Both Liella! AND non-Liella! members moved.
/// Only the Liella! member gets blade.
#[test]
fn aspire_liella_and_non_liella_moved_only_liella_gains_blade() {
    let mut game = setup_aspire_game();
    let aspire = game.id("PL!SP-sd2-025-P");
    let liella = game.id("PL!SP-sd1-001-SD");
    let non_liella = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = liella;
    game.state.player1.stage.stage[2] = non_liella;
    game.state.cards_moved_this_turn.insert(liella);
    game.state.cards_moved_this_turn.insert(non_liella);
    game.state.position_change_occurred_this_turn = true;

    advance_to_live_start(&mut game, aspire);

    assert_eq!(
        game.state.mods.get_blade_modifier(liella),
        1,
        "Liella! moved should get blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(non_liella),
        0,
        "Non-Liella! moved should get 0"
    );
}

/// No Liella! on stage → no blade.
#[test]
fn aspire_no_liella_on_stage_no_blade() {
    let mut game = setup_aspire_game();
    let aspire = game.id("PL!SP-sd2-025-P");
    let non_liella = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = non_liella;
    game.state.cards_moved_this_turn.insert(non_liella);
    game.state.position_change_occurred_this_turn = true;

    advance_to_live_start(&mut game, aspire);

    assert_eq!(
        game.state.mods.get_blade_modifier(non_liella),
        0,
        "No Liella! on stage → 0"
    );
}

/// No cards moved → 0 blade even for Liella!.
#[test]
fn aspire_no_movement_no_blade() {
    let mut game = setup_aspire_game();
    let aspire = game.id("PL!SP-sd2-025-P");
    let liella = game.id("PL!SP-sd1-001-SD");
    game.state.player1.stage.stage[1] = liella;

    advance_to_live_start(&mut game, aspire);

    assert_eq!(
        game.state.mods.get_blade_modifier(liella),
        0,
        "Liella! not moved → 0"
    );
}

/// Both Liella! members moved → both get blade.
#[test]
fn aspire_two_moved_liella_both_gain_blade() {
    let mut game = setup_aspire_game();
    let aspire = game.id("PL!SP-sd2-025-P");
    let liella_a = game.id("PL!SP-sd1-001-SD");
    let liella_b = game.id("PL!SP-sd1-002-SD");

    game.state.player1.stage.stage[1] = liella_a;
    game.state.player1.stage.stage[2] = liella_b;
    game.state.cards_moved_this_turn.insert(liella_a);
    game.state.cards_moved_this_turn.insert(liella_b);
    game.state.position_change_occurred_this_turn = true;

    advance_to_live_start(&mut game, aspire);

    assert_eq!(
        game.state.mods.get_blade_modifier(liella_a),
        1,
        "Liella! A moved → blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(liella_b),
        1,
        "Liella! B moved → blade"
    );
}

// ====================================================================
// PATH 2: movement_condition in condition evaluator
// Uses the full evaluate_movement_condition() path which checks:
//   has_card_moved_this_turn(cid) AND position_change_occurred_this_turn
//
// Card: PL!SP-bp2-003-R (嵐千砂都) — auto trigger on area movement
//   Text: 自動 ターン1回 このメンバーがエリアを移動したとき、
//         自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
// ====================================================================

fn setup_energy(game: &mut TestGame) {
    for _ in 0..5 {
        game.state
            .player1
            .energy_deck
            .cards
            .push(game.id("LL-E-001-SD"));
    }
}

/// Movement trigger fires when card moved AND position_change_occurred_this_turn is set.
#[test]
fn chisato_movement_triggers_with_both_flags() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chisato = game.id("PL!SP-bp2-003-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [chisato, -1, filler];
    setup_energy(&mut game);

    let energy_before = game.state.player1.energy_zone.cards.len();

    game.state
        .player1
        .stage
        .position_change(MemberArea::LeftSide, MemberArea::RightSide)
        .expect("position_change should succeed");

    game.state.record_card_movement(chisato);
    game.state.record_card_movement(filler);
    game.state.position_change_occurred_this_turn = true;

    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before + 1,
        "Area move should trigger 1 energy placement"
    );
}

/// Movement trigger does NOT fire when only card movement is set
/// but position_change_occurred_this_turn is false.
#[test]
fn chisato_no_trigger_without_position_change_flag() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chisato = game.id("PL!SP-bp2-003-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [chisato, -1, filler];
    setup_energy(&mut game);

    let energy_before = game.state.player1.energy_zone.cards.len();

    game.state
        .player1
        .stage
        .position_change(MemberArea::LeftSide, MemberArea::RightSide)
        .expect("position_change should succeed");

    game.state.record_card_movement(chisato);
    game.state.record_card_movement(filler);

    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before,
        "No energy placed without position_change_occurred_this_turn"
    );
}

// ====================================================================
// PATH 3: 登場か、エリアを移動 — debut OR area-move trigger
// Tests the PL!SP-bp4-011 鬼塚冬毬 card:
//   自動 このメンバーが登場か、エリアを移動したとき、
//         相手のステージにいる元々持つブレードの数が3つ以下のメンバー1人をウェイトにする。
// ====================================================================

/// Ability triggers on 登場 (appear): waits one opponent member with blade ≤ 3.
#[test]
fn fuyumari_appear_triggers_opponent_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fuyumari = game.id("PL!SP-bp4-011-R＋");
    let target = game.id("PL!-sd1-010-SD"); // blade=1, valid
    let filler = game.id("PL!-sd1-002-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Place 鬼塚冬毬 on P1 stage
    game.state.player1.stage.stage[0] = fuyumari;
    // Place target on P2 stage
    game.state.player2.stage.stage[0] = target;

    // Simulate 登場: card appeared and moved this turn
    game.state.record_card_appearance(fuyumari, "");
    game.state.record_card_movement(fuyumari);

    let pid = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    // With only 1 valid candidate and count=1, the effect auto-applies (no prompt).
    // Verify target is now in wait state.
    let ori = game.state.mods.get_orientation_modifier(target);
    assert_eq!(
        ori.map(|s| s.as_str()),
        Some("wait"),
        "Target should be in wait state"
    );

    // Verify no re-trigger caused another choice
    assert!(
        !game.has_pending_choice(),
        "Should NOT have another pending choice (no self-re-trigger)"
    );
}

/// Ability triggers on area move: waits one opponent member with blade ≤ 3.
#[test]
fn fuyumari_area_move_triggers_opponent_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fuyumari = game.id("PL!SP-bp4-011-R＋");
    let target = game.id("PL!-sd1-010-SD"); // blade=1, valid
    let filler = game.id("PL!-sd1-002-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[0] = fuyumari;
    game.state.player2.stage.stage[0] = target;

    // Simulate area move: position changed and card moved this turn
    game.state.record_card_movement(fuyumari);
    game.state.position_change_occurred_this_turn = true;

    let pid = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    // With only 1 valid candidate and count=1, the effect auto-applies (no prompt).
    let ori = game.state.mods.get_orientation_modifier(target);
    assert_eq!(
        ori.map(|s| s.as_str()),
        Some("wait"),
        "Target should be in wait state after area move"
    );

    assert!(
        !game.has_pending_choice(),
        "Should NOT have another pending choice"
    );
}

/// Blade limit: members with blade > 3 are excluded from selection.
#[test]
fn fuyumari_blade_limit_excludes_high_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fuyumari = game.id("PL!SP-bp4-011-R＋");
    let high_blade = game.id("PL!-sd1-009-SD"); // blade=5 > 3

    game.state.player1.stage.stage[0] = fuyumari;
    game.state.player2.stage.stage[0] = high_blade;

    game.state.record_card_appearance(fuyumari, "");
    game.state.record_card_movement(fuyumari);

    let pid = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    // No valid candidates → no choice prompt
    assert!(
        !game.has_pending_choice(),
        "No choice when all targets have blade > 3"
    );

    // Verify high-blade card is NOT in wait state
    let ori = game.state.mods.get_orientation_modifier(high_blade);
    assert!(
        ori.is_none() || ori.map(|s| s.as_str()) != Some("wait"),
        "High-blade member should NOT be waited"
    );
}

/// Already-wait members are excluded from candidates
/// (Fix B: when state_change="wait", filter out already-wait cards).
#[test]
fn fuyumari_already_wait_excluded_from_candidates() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fuyumari = game.id("PL!SP-bp4-011-R＋");
    let target = game.id("PL!-sd1-010-SD"); // blade=1, valid but already wait
    let other = game.id("PL!-sd1-002-SD"); // blade=1, valid

    game.state.player1.stage.stage[0] = fuyumari;
    game.state.player2.stage.stage[0] = target;
    game.state.player2.stage.stage[1] = other;

    // Mark target as already in wait state
    game.state.mods.add_orientation_modifier(target, "wait");

    game.state.record_card_appearance(fuyumari, "");
    game.state.record_card_movement(fuyumari);

    let pid = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    // Only 1 valid (non-wait) candidate and count=1 → auto-applies without prompt.

    // The non-wait card should now be waited
    let ori_other = game.state.mods.get_orientation_modifier(other);
    assert_eq!(
        ori_other.map(|s| s.as_str()),
        Some("wait"),
        "The non-wait card should be waited"
    );

    // The already-wait card should remain wait (unchanged)
    let ori_target = game.state.mods.get_orientation_modifier(target);
    assert_eq!(
        ori_target.map(|s| s.as_str()),
        Some("wait"),
        "Already-wait card should remain wait"
    );
}

/// Only one card is waited (count=1) even when multiple valid targets exist.
#[test]
fn fuyumari_waits_only_one_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fuyumari = game.id("PL!SP-bp4-011-R＋");
    let target_a = game.id("PL!-sd1-010-SD"); // blade=1
    let target_b = game.id("PL!-sd1-001-SD"); // blade=3

    game.state.player1.stage.stage[0] = fuyumari;
    game.state.player2.stage.stage[0] = target_a;
    game.state.player2.stage.stage[1] = target_b;

    game.state.record_card_appearance(fuyumari, "");
    game.state.record_card_movement(fuyumari);

    let pid = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    assert!(
        game.has_pending_choice(),
        "Should prompt with 2 valid targets"
    );
    game.assert_select_card("stage", 1, false);

    // Select target_a (stage position 0)
    game.select_indices(&[0]);

    let ori_a = game.state.mods.get_orientation_modifier(target_a);
    assert_eq!(
        ori_a.map(|s| s.as_str()),
        Some("wait"),
        "Selected target should be waited"
    );

    let ori_b = game.state.mods.get_orientation_modifier(target_b);
    assert!(
        ori_b.is_none() || ori_b.map(|s| s.as_str()) != Some("wait"),
        "Unselected target should NOT be waited"
    );
}
