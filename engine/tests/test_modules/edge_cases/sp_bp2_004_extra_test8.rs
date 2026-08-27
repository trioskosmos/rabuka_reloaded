use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_center_highest_with_high_right_low_left() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let left_low = game.id("PL!-sd1-010-SD");
    let center_high = game.id("PL!SP-pb1-001-R");
    let right_low = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [left_low, center_high, right_low];
    game.state.player1.stage.stage[0] = sumire; // actually sumire at left with low? Let's set correctly
    // sumire at 0 with cost 9, center 11, right 4 -> center highest -> sumire gains
    game.state.player1.stage.stage = [sumire, center_high, right_low];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 1);
}

#[test]
fn sp_bp2_004_no_gain_when_center_empty_and_others_present() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let left = game.id("PL!SP-pb1-001-R");
    let right = game.id("PL!HS-PR-001-PR");
    game.state.player1.stage.stage = [left, -1, right];
    game.state.player1.stage.stage[0] = sumire; // sumire at left, center empty
    game.state.recalculate_constants();
    // Center empty -> no highest, so no heart
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0);
}
