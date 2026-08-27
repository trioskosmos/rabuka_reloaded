use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_center_highest_with_only_two_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let center_high = game.id("PL!SP-pb1-001-R");
    game.state.player1.stage.stage = [sumire, center_high, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 1);
}

#[test]
fn sp_bp2_004_center_low_with_two_members_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let left_high = game.id("PL!SP-pb1-001-R");
    let center_low = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [left_high, center_low, -1];
    game.state.player1.stage.stage[0] = sumire;
    // Actually left is sumire with 9, center is 4, so center not highest
    game.state.player1.stage.stage = [sumire, center_low, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0, "center 4 < left 9 -> no heart");
}
