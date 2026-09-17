use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn pl_sp_bp5_011_r_constant_position_changes_replace_granted_heart_color() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fuyuko = game.id("PL!SP-bp5-011-R");

    game.state.player1.stage.stage[0] = fuyuko;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart02),
        3,
        "左サイド: heart02×3"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart03),
        0
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart05),
        0
    );

    game.state.player1.stage.stage[0] = -1;
    game.state.player1.stage.stage[1] = fuyuko;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart03),
        3,
        "センター: heart03×3"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart02),
        0
    );

    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[2] = fuyuko;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart05),
        3,
        "右サイド: heart05×3"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(fuyuko, HeartColor::Heart03),
        0
    );
}
