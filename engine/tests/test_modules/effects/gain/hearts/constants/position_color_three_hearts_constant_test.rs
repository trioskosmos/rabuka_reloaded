use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!SP-bp5-011-R: 常時 left→heart02×3, center→heart03×3, right→heart05×3.
/// Tests all three position variants.
#[test]
fn constant_position_grants_three_hearts_of_corresponding_color() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-bp5-011-R");
    game.state.player1.stage.stage = [-1, member, -1];
    game.state.recalculate_constants();

    // Left → heart02×3
    game.state.player1.stage.stage[0] = member;
    game.state.player1.stage.stage[1] = -1;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(member, HeartColor::Heart02),
        3,
        "left → +3 heart02"
    );

    // Center → heart03×3
    game.state.player1.stage.stage[0] = -1;
    game.state.player1.stage.stage[1] = member;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(member, HeartColor::Heart03),
        3,
        "center → +3 heart03"
    );

    // Right → heart05×3
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[2] = member;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(member, HeartColor::Heart05),
        3,
        "right → +3 heart05"
    );
}
