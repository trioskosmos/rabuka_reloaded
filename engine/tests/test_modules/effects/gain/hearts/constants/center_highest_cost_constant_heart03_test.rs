use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn p_print_tied_center_cost_grants_no_heart() {
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
fn lone_member_without_center_grants_no_heart() {
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

#[test]
fn center_highest_between_two_members_grants_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let center_high = game.id("PL!SP-pb1-001-R");
    game.state.player1.stage.stage = [-1, center_high, -1];
    // sumire is at left? Actually we need sumire on stage to check its heart, but sumire is at left with cost 9, center is 11, so center is highest, sumire should gain
    game.state.player1.stage.stage[0] = sumire;
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 1);
}

#[test]
fn empty_center_with_lone_member_grants_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    game.state.player1.stage.stage = [sumire, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0, "no center card -> no heart");
}

#[test]
fn all_three_equal_cost_grants_no_heart() {
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
fn center_highest_with_low_sides_grants_heart03() {
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

#[test]
fn left_exceeds_center_without_self_on_stage_no_heart() {
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
fn center_lowest_with_high_sides_no_heart() {
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

#[test]
fn center_highest_with_lone_left_self_grants_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let center = game.id("PL!SP-pb1-001-R");
    game.state.player1.stage.stage = [-1, center, -1];
    // sumire is not on stage, but we check its heart modifier - should be 0 because sumire not on stage
    // Actually we need sumire on stage to check its own heart
    game.state.player1.stage.stage[0] = sumire;
    game.state.recalculate_constants();
    // Center is 11, left sumire is 9, so center is highest -> sumire gains
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 1);
}

#[test]
fn no_members_on_stage_grants_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();
    let sumire = game.id("PL!SP-bp2-004-R");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0);
}

#[test]
fn p_print_center_highest_with_left_self_grants_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire_p = game.id("PL!SP-bp2-004-P");
    let center_high = game.id("PL!SP-pb1-001-R");
    game.state.player1.stage.stage = [sumire_p, center_high, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire_p, HeartColor::Heart03), 1);
}

#[test]
fn p_print_tied_center_cost_left_self_grants_no_heart() {
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

#[test]
fn center_highest_with_right_member_grants_heart03() {
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
fn empty_center_with_two_side_members_grants_no_heart() {
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

#[test]
fn right_cost_above_center_grants_no_heart() {
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
fn self_at_right_with_center_highest_grants_heart03() {
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

#[test]
fn center_highest_with_two_member_stage_grants_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let center_high = game.id("PL!SP-pb1-001-R");
    game.state.player1.stage.stage = [sumire, center_high, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 1);
}

#[test]
fn center_cost_below_left_grants_no_heart() {
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

#[test]
fn lone_self_at_center_is_highest_grants_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    // Only sumire at center with cost 9, no other members -> center is highest (only member)
    game.state.player1.stage.stage = [-1, sumire, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 1);
}

#[test]
fn center_lowest_with_high_right_grants_no_heart() {
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
