use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_center_highest_with_two_empty_sides() {
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
fn sp_bp2_004_no_center_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    game.state.player1.stage.stage = [sumire, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0, "no center card -> no heart");
}
