use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_all_three_same_cost_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let c9a = game.id("PL!SP-bp2-004-R");
    let c9b = game.id("PL!-PR-005-PR");
    let c9c = game.id("PL!-PR-005-PR");
    game.state.player1.stage.stage = [c9a, c9b, c9c];
    // Put sumire at left with same cost as center and right
    game.state.player1.stage.stage[0] = sumire;
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0);
}

#[test]
fn sp_bp2_004_center_highest_with_low_left_right() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let center_high = game.id("PL!SP-pb1-001-R");
    let left_low = game.id("PL!-sd1-010-SD");
    let right_low = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [left_low, center_high, right_low];
    // Need sumire on stage to check its heart, but sumire is not at center, it's at left with low cost
    // Actually sumire is at left with low cost, center is high, so sumire should gain
    game.state.player1.stage.stage[0] = sumire;
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 1);
}
