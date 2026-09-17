use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_center_highest_with_only_center() {
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
fn sp_bp2_004_no_stage_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();
    let sumire = game.id("PL!SP-bp2-004-R");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0);
}
