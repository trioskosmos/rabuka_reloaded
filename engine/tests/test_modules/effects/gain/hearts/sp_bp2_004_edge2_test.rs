use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sp_bp2_004_all_empty_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();
    let sumire = game.id("PL!SP-bp2-004-R");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 0);
}

#[test]
fn sp_bp2_004_center_only_one_member_gains() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    game.state.player1.stage.stage = [-1, sumire, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart03), 1, "single center member should be highest by default");
}
