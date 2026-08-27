/// Comprehensive edges for PL!-bp5-333 idx426
/// 常時 このメンバーがウェイト状態であるかぎり、heart05を得る。
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn h05(game: &TestGame, id: i16) -> i32 {
    game.state.mods.get_heart_modifier(id, HeartColor::Heart05)
}

// Two Erenas independent: one wait, one active
#[test]
fn bp5_333_two_copies_independent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let erena_left = game.id("PL!-bp5-333-R");
    let erena_center = game.new_id("PL!-bp5-333-R");
    game.state.player1.stage.stage = [erena_left, erena_center, -1];
    game.state.mods.add_orientation_modifier(erena_left, "wait");
    game.state.mods.add_orientation_modifier(erena_center, "active");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena_left), 1, "left waited -> 1");
    assert_eq!(h05(&game, erena_center), 0, "center active -> 0");
    // Swap
    game.state.mods.add_orientation_modifier(erena_left, "active");
    game.state.mods.add_orientation_modifier(erena_center, "wait");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena_left), 0);
    assert_eq!(h05(&game, erena_center), 1);
    // Both wait
    game.state.mods.add_orientation_modifier(erena_left, "wait");
    game.state.mods.add_orientation_modifier(erena_center, "wait");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena_left), 1);
    assert_eq!(h05(&game, erena_center), 1);
}

// Wait persists after position change (stage swap)
#[test]
fn bp5_333_wait_persists_after_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let erena = game.id("PL!-bp5-333-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [erena, filler, -1];
    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 1);
    // Move erena to center via direct stage swap (simulates position_change)
    game.state.player1.stage.stage = [filler, erena, -1];
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 1, "wait heart persists after area move");
    // Move to right as well
    game.state.player1.stage.stage = [-1, filler, erena];
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 1);
}

// Leaving stage clears heart even if orientation still "wait" stored; returning active gives 0
#[test]
fn bp5_333_leave_stage_clears_reenter_active_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let erena = game.id("PL!-bp5-333-R");
    game.state.player1.stage.stage = [erena, -1, -1];
    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 1);
    // Remove to discard
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.waitroom.cards.push(erena);
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 0, "not on stage -> 0 even though still marked wait");
    // Return to stage but as active
    game.state.player1.waitroom.cards.retain(|c| *c != erena);
    game.state.player1.stage.stage = [erena, -1, -1];
    game.state.mods.add_orientation_modifier(erena, "active");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 0, "re-entered as active -> 0");
    // Flip back to wait
    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 1);
}

// Default orientation is active -> no heart (explicit)
#[test]
fn bp5_333_no_orientation_defaults_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let erena = game.id("PL!-bp5-333-R");
    game.state.player1.stage.stage = [erena, -1, -1];
    // Do not set orientation at all
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 0, "default (no orientation) -> active -> 0");
}

// Multiple recalculations without state change keep value stable
#[test]
fn bp5_333_stable_across_recalc() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let erena = game.id("PL!-bp5-333-R");
    game.state.player1.stage.stage = [erena, -1, -1];
    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 1);
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 1);
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 1);
}

// P+ variant behaves identically
#[test]
fn bp5_333_p_plus_variant_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let erena = game.id("PL!-bp5-333-P＋");
    game.state.player1.stage.stage = [-1, erena, -1];
    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 1, "P+ at center waited -> 1");
    game.state.mods.add_orientation_modifier(erena, "active");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 0);
}

// Interaction: other stage members waited/active does not affect Erena's own condition
#[test]
fn bp5_333_other_members_wait_irrelevant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let erena = game.id("PL!-bp5-333-R");
    let other = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [erena, other, -1];
    game.state.mods.add_orientation_modifier(other, "wait");
    game.state.mods.add_orientation_modifier(erena, "active");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 0, "erena active -> 0 even though other is wait");
    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();
    assert_eq!(h05(&game, erena), 1, "erena wait -> 1 regardless of other");
}
