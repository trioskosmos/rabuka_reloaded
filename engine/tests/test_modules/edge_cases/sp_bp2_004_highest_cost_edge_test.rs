use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_center_tie_with_both_sides_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    // All three same cost 9 -> center not strictly highest
    let c9a = game.id("PL!SP-bp2-004-R");
    let c9b = game.id("PL!-PR-005-PR");
    let c9c = game.id("PL!-PR-005-PR");
    game.state.player1.stage.stage = [c9a, c9b, c9c];
    game.state.recalculate_constants();
    let before = game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03);
    // Already recalculated, should be 0
    assert_eq!(before, 0);
}

#[test]
fn sp_bp2_004_center_highest_with_empty_side() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let center_high = game.id("PL!SP-pb1-001-R"); // 11
    game.state.player1.stage.stage = [sumire, center_high, -1];
    game.state.recalculate_constants();
    let h = game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03);
    assert_eq!(h, 1, "center 11 > left 9, right empty -> gain");
}

#[test]
fn sp_bp2_004_p_variant_same() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire_p = game.id("PL!SP-bp2-004-P");
    let center_high = game.id("PL!SP-pb1-001-R");
    game.state.player1.stage.stage = [sumire_p, center_high, -1];
    game.state.recalculate_constants();
    let h = game.state.mods.get_heart_modifier(sumire_p, HeartColor::Heart03);
    assert_eq!(h, 1);
}
