use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_center_highest_with_tie_left_right_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let left = game.id("PL!SP-pb1-001-R"); // 11
    let center = game.id("PL!HS-PR-001-PR"); // 10
    let right = game.id("PL!SP-pb1-001-R"); // 11 tie with left
    game.state.player1.stage.stage = [left, center, right];
    game.state.player1.stage.stage[0] = sumire; // sumire at left? Actually left is 11, center 10, right 11 -> center not highest
    // Let's set left 4, center 11, right 11 -> center tie with right, not highest
    let left_low = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [left_low, center, right];
    game.state.player1.stage.stage[0] = sumire;
    // Simplify: sumire at left with 9, center 10, right 11 -> center not highest (right is)
    let sumire_id = game.id("PL!SP-bp2-004-R");
    let center_mid = game.id("PL!HS-PR-001-PR");
    let right_high = game.id("PL!SP-pb1-001-R");
    game.state.player1.stage.stage = [sumire_id, center_mid, right_high];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire_id, HeartColor::Heart03), 0, "center 10 < right 11 -> no heart");
}

#[test]
fn sp_bp2_004_sumire_at_right_center_highest_gains() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let center_high = game.id("PL!SP-pb1-001-R");
    let left_low = game.id("PL!-sd1-010-SD");
    // sumire at right with 9, center 11, left 4 -> center is highest, sumire should gain even though sumire is at right
    game.state.player1.stage.stage = [left_low, center_high, sumire];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 1);
}
