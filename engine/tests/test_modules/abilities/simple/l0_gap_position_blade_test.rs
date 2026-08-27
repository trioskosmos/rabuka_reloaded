/// L0 gap coverage: additional Constant position-based blade abilities.
use crate::helpers::*;

/// PL!SP-sd2-004-SD2: 常時 センター → ブレード+4。
#[test]
fn sd2_004_center_blade_plus4() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-sd2-004-SD2");
    game.state.player1.stage.stage = [-1, member, -1];
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        4,
        "center position grants exactly +4 blade"
    );

    // Negative: move to left → no bonus
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[0] = member;
    game.state.recalculate_constants();
    assert_ne!(
        game.state.mods.get_blade_modifier(member),
        4,
        "left position should not grant the center bonus"
    );
}

/// PL!SP-pb2-035-N: 常時 左サイド → ブレード+2。
#[test]
fn pb2_035_left_blade_plus2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-pb2-035-N");
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        2,
        "left position grants exactly +2 blade"
    );
}

/// PL!SP-pb2-041-N: 常時 右サイド → ブレード+2。
#[test]
fn pb2_041_right_blade_plus2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-pb2-041-N");
    game.state.player1.stage.stage = [-1, -1, member];
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        2,
        "right position grants exactly +2 blade"
    );
}
