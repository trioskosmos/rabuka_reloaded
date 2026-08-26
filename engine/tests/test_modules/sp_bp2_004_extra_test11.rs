use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_center_highest_with_only_sumire_at_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    // Only sumire at center with cost 9, no other members -> center is highest (only member)
    game.state.player1.stage.stage = [-1, sumire, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 1);
}

#[test]
fn sp_bp2_004_no_gain_when_center_is_lowest() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let left_high = game.id("PL!SP-pb1-001-R");
    let center_low = game.id("PL!-sd1-010-SD");
    let right_mid = game.id("PL!HS-PR-001-PR");
    game.state.player1.stage.stage = [left_high, center_low, right_mid];
    game.state.player1.stage.stage[0] = sumire;
    // Actually left is sumire with 9, center is 4, right is 10 -> center is lowest, not highest
    game.state.player1.stage.stage = [sumire, center_low, right_mid];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0);
}
