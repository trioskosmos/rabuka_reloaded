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

    // Negative: move to left → no bonus at all.
    //
    // This was `assert_ne!(blade, 4)`, which also accepts +1, +2 and +3. A position gate
    // that granted *something* off-centre, or a partial one, passed. The printed text is
    // 「{{jyouji.png|常時}}{{center.png|センター}}ブレード×4」 — the whole effect is
    // position-gated, so off-centre is exactly 0.
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[0] = member;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        0,
        "左サイド — the centre-only effect must contribute nothing off-centre"
    );
    // And the right side is equally excluded, so a gate keyed to the wrong area cannot pass.
    game.state.player1.stage.stage[0] = -1;
    game.state.player1.stage.stage[2] = member;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(member),
        0,
        "右サイド — the centre-only effect must contribute nothing on the right either"
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

    // The effect is left-only. This test previously had no negative case at all, so a
    // position filter that was simply ABSENT — granting the bonus in every area — passed.
    for (idx, area) in [(1usize, "center"), (2, "right")] {
        let mut stage = [-1i16; 3];
        stage[idx] = member;
        game.state.player1.stage.stage = stage;
        game.state.recalculate_constants();
        assert_eq!(
            game.state.mods.get_blade_modifier(member),
            0,
            "{area} — 「{{{{leftside.png|左サイド}}}}ブレード×2」 grants nothing outside the left"
        );
    }
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

    // Right-only, likewise previously untested from the other side.
    for (idx, area) in [(0usize, "left"), (1, "center")] {
        let mut stage = [-1i16; 3];
        stage[idx] = member;
        game.state.player1.stage.stage = stage;
        game.state.recalculate_constants();
        assert_eq!(
            game.state.mods.get_blade_modifier(member),
            0,
            "{area} — 「{{{{rightside.png|右サイド}}}}ブレード×2」 grants nothing outside the right"
        );
    }
}
