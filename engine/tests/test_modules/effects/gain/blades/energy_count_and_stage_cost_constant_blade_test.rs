use crate::helpers::*;

#[test]
fn nico_pr021_constant_exactly_seven_energy_grants_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-PR-021-PR");
    game.state.player1.stage.stage[1] = nico;

    // 6 energy → condition fails.
    game.give_energy(6);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(nico),
        0,
        "6 energy ≠ exactly 7 → no blades"
    );

    // 7 energy → exactly → +2 blades.
    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(nico),
        2,
        "exactly 7 energy → ブレード2つ"
    );

    // 8 energy → past the boundary → back to 0 ("〜あるかぎり" is live state).
    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(nico),
        0,
        "8 energy ≠ exactly 7 → blades lost again"
    );
}

#[test]
fn natsumi_sp_bp4_009_constant_cheaper_stage_total_grants_three_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp4-009-R"); // cost 9
    let big = game.id("PL!S-bp5-009-R"); // cost 15
    let small = game.id(CLEAN_KOTORI); // cost 5

    game.state.player1.stage.stage[1] = natsumi;
    game.state.player2.stage.stage[1] = big;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        3,
        "my total 9 < opponent 15 → ブレード3つ"
    );

    game.state.player2.stage.stage[1] = small;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        0,
        "my total 9 > opponent 5 → blades gone"
    );
}

const CLEAN_KOTORI: &str = "PL!-pb1-021-PR";
