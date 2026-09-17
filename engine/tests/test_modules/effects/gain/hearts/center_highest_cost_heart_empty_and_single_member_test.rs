use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn empty_stage_leaves_source_without_heart03_bonus_pl_sp_bp2_004() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();
    let sumire = game.id("PL!SP-bp2-004-R");
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(sumire, HeartColor::Heart03),
        0
    );
}

#[test]
fn sole_center_member_gains_one_heart03_pl_sp_bp2_004() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    game.state.player1.stage.stage = [-1, sumire, -1];
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(sumire, HeartColor::Heart03),
        1,
        "single center member should be highest by default"
    );
}
