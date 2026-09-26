use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!SP-bp7-009-R: 常時 左サイドまたは右サイドにいる場合、heart02+1。
#[test]
fn left_and_right_sides_each_grant_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-bp7-009-R");
    game.assert_card_identity(member, "PL!SP-bp7-009-R");

    // Center is the negative case for 左サイドまたは右サイド: no heart02 there.
    game.state.player1.stage.stage = [-1, member, -1];
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(member, HeartColor::Heart02),
        0,
        "center is neither 左サイド nor 右サイド → no heart02"
    );

    // Left side
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.recalculate_constants();
    let h02_left = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart02);
    assert_eq!(h02_left, 1, "左サイドにいる場合、heart02+1 — exactly one");

    // Right side
    game.state.player1.stage.stage = [-1, -1, member];
    game.state.recalculate_constants();
    let h02_right = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart02);
    assert_eq!(h02_right, 1, "右サイドにいる場合、heart02+1 — exactly one");
}
