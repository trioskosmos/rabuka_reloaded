use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_center_highest_with_high_left_low_right() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let left_high = game.id("PL!SP-pb1-001-R"); // 11
    let center_mid = game.id("PL!HS-PR-001-PR"); // 10
    let right_low = game.id("PL!-sd1-010-SD"); // 4
    game.state.player1.stage.stage = [left_high, center_mid, right_low];
    game.state.recalculate_constants();
    // Sumire is at left with 11, center is 10, so center is NOT highest
    // But sumire's heart is for sumire card itself, which is at left with 11, center is 10, so center (10) < left (11) -> no heart
    // Actually sumire is at left with 11, center is 10, right is 4, so center is not highest
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0);
}

#[test]
fn sp_bp2_004_center_lowest_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let left_high = game.id("PL!SP-pb1-001-R");
    let center_low = game.id("PL!-sd1-010-SD");
    let right_mid = game.id("PL!HS-PR-001-PR");
    game.state.player1.stage.stage = [left_high, center_low, right_mid];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0);
}
