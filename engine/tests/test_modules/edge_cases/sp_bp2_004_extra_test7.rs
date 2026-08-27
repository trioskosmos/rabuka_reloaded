use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_p_variant_center_highest() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire_p = game.id("PL!SP-bp2-004-P");
    let center_high = game.id("PL!SP-pb1-001-R");
    game.state.player1.stage.stage = [sumire_p, center_high, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire_p, HeartColor::Heart03), 1);
}

#[test]
fn sp_bp2_004_center_tie_no_heart_p_variant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire_p = game.id("PL!SP-bp2-004-P");
    let c9b = game.id("PL!-PR-005-PR");
    game.state.player1.stage.stage = [sumire_p, c9b, -1];
    game.state.recalculate_constants();
    // Both cost 9? sumire_p cost 9, center 9 -> tie -> no heart
    // Need to check sumire_p cost: it is also 9
    assert_eq!(game.state.mods.get_heart_modifier(sumire_p, HeartColor::Heart03), 0);
}
