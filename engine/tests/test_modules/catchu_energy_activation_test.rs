use crate::helpers::*;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

// ====================================================================
// Fix verification: per-unit + distinct + change_state (energy activate)
// Card text (PL!SP-pb2-018-R 米女メイ):
//   自分のステージにいるこのメンバーと名前の異なる『CatChu!』のメンバー1人につき、
//   エネルギーを1枚アクティブにする。
// Effect JSON uses per_unit=true, distinct="card_name", group_names=["CatChu!"],
// card_type="energy_card", state_change="active", count=1
//
// IMPORTANT: 米女メイ is a MEMBER card played to STAGE, NOT a live card.
// Her ability fires from the stage during live start phase.
// ====================================================================

/// 2 different-named CatChu! members on stage (Mei + 1 other) → activates 2 energy
#[test]
fn catchu_basic_two_distinct_activate_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let mei = game.id("PL!SP-pb2-018-R");
    let catchu = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    let card = game
        .state
        .card_database
        .get_card(mei)
        .expect("mei card not found");
    assert!(
        !card.abilities.is_empty(),
        "mei should have abilities loaded"
    );
    assert!(card.abilities.iter().any(|a| a
        .triggers
        .as_ref()
        .is_some_and(|t| t.contains("ライブ開始時"))));

    fill_decks(&mut game);

    // Mei + 1 other CatChu on stage = 2 distinct CatChu names
    game.state.player1.stage.stage[0] = mei;
    game.state.player1.stage.stage[1] = catchu;
    game.state.player1.stage.stage[2] = filler;

    game.give_energy(3);
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);

    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);

    assert!(!game.has_pending_choice(), "No pending choices expected");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        5,
        "2 distinct CatChu names on stage → activate 2: 3→5"
    );
}

/// 0 CatChu! members on stage → 0 energy activated (no change from 3)
#[test]
fn catchu_zero_members_nothing_activates() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let _mei = game.id("PL!SP-pb2-018-R");
    let filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = filler;
    game.state.player1.stage.stage[2] = filler;

    // Mei NOT on stage → ability doesn't fire
    game.give_energy(3);

    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);

    assert!(!game.has_pending_choice(), "No pending choices expected");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        3,
        "No CatChu members → no activation"
    );
}

/// 3 different-named CatChu! members → activates 3 energy
#[test]
fn catchu_three_distinct_activate_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mei = game.id("PL!SP-pb2-018-R");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-004-SD");
    let _filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    game.state.player1.stage.stage[0] = mei;
    game.state.player1.stage.stage[1] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;

    game.give_energy(3);
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);

    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);

    assert!(!game.has_pending_choice(), "No pending choices expected");
}

/// Duplicate names: 2 copies of the SAME CatChu + Mei → 2 distinct names
#[test]
fn catchu_duplicate_names_dedup_to_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mei = game.id("PL!SP-pb2-018-R");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_dup = game.new_id("PL!SP-sd1-001-SD");
    let _filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    // 2 same-name CatChu (かのん x2) + Mei (米女メイ) = 2 distinct names
    game.state.player1.stage.stage[0] = mei;
    game.state.player1.stage.stage[1] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_dup;

    game.give_energy(3);
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);

    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);

    assert!(!game.has_pending_choice(), "No pending choices expected");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        5,
        "2 distinct names (mei + kanon) → activate 2: 3→5"
    );
}

/// 1 CatChu member on stage (Mei alone) → activates 1 energy
#[test]
fn catchu_single_member_activate_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mei = game.id("PL!SP-pb2-018-R");
    let filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    game.state.player1.stage.stage[0] = mei;
    game.state.player1.stage.stage[1] = filler;

    game.give_energy(3);
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);

    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);

    assert!(!game.has_pending_choice(), "No pending choices expected");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        4,
        "1 CatChu member → activate 1 energy: 3→4"
    );
}

/// Only non-CatChu members on stage (no Mei) → activates 0
#[test]
fn catchu_only_non_catchu_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = filler;
    game.state.player1.stage.stage[2] = filler;

    game.give_energy(3);

    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);

    assert!(!game.has_pending_choice(), "No pending choices expected");

    // No CatChu members on stage at all → no abilities fire → energy unchanged
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);
}

/// Self as member on stage + 1 other CatChu → activates 2
#[test]
fn catchu_self_as_member_on_stage_plus_one_other() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mei = game.id("PL!SP-pb2-018-R");
    let catchu = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    game.state.player1.stage.stage[0] = mei;
    game.state.player1.stage.stage[1] = catchu;
    game.state.player1.stage.stage[2] = filler;

    game.give_energy(3);
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);

    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);

    assert!(!game.has_pending_choice(), "No pending choices expected");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        5,
        "2 unique CatChu names on stage → activate 2: 3→5"
    );
}
