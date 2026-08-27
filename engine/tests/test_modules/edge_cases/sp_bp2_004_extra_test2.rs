use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_p_variant_tie_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire_p = game.id("PL!SP-bp2-004-P");
    let c9a = game.id("PL!SP-bp2-004-P");
    let c9b = game.id("PL!-PR-005-PR");
    game.state.player1.stage.stage = [c9a, c9b, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire_p, HeartColor::Heart03), 0, "tie should not give");
}

#[test]
fn sp_bp2_004_center_empty_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    game.state.player1.stage.stage = [sumire, -1, -1];
    game.state.recalculate_constants();
    // Only sumire at left, center empty -> center not highest (no card), so no heart
    // The condition checks center's cost vs others; with center empty, it should be false
    let h = game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03);
    assert_eq!(h, 0);
}
