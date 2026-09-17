use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn natsumi_sp_bp7_009_constant_on_sides_grants_heart02_but_not_in_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp7-009-R");

    // Left side → heart02.
    game.state.player1.stage.stage[0] = natsumi;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(natsumi, HeartColor::Heart02),
        1,
        "左サイド: heart02 granted"
    );

    // Center → gone.
    game.state.player1.stage.stage[0] = -1;
    game.state.player1.stage.stage[1] = natsumi;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(natsumi, HeartColor::Heart02),
        0,
        "センター: neither side ability applies"
    );

    // Right side → back.
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[2] = natsumi;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(natsumi, HeartColor::Heart02),
        1,
        "右サイド: heart02 granted again"
    );
}
