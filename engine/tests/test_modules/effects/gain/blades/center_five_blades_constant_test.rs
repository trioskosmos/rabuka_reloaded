use crate::helpers::*;

/// PL!SP-bp1-004-PR 平安名すみれ: 常時 センターにいる場合、ブレード+5。
#[test]
fn center_grants_five_blades_and_moving_left_removes_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp1-004-PR");
    game.state.player1.stage.stage = [-1, sumire, -1];
    game.give_energy(20);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(sumire),
        5,
        "center position grants +5 blade"
    );

    // Negative: move out of center → modifier drops.
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[0] = sumire;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(sumire),
        0,
        "left position → no blade bonus"
    );
}
