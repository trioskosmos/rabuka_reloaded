/// L0 gap coverage: additional Constant position-based blade abilities.
use crate::helpers::*;

/// PL!SP-sd2-004-SD2: 常時 センター → ブレード+4。
#[test]
fn center_grants_four_blades_but_left_does_not_pl_sp_sd2_004() {
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
fn left_side_grants_two_blades_pl_sp_pb2_035() {
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
fn right_side_grants_two_blades_pl_sp_pb2_041() {
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
