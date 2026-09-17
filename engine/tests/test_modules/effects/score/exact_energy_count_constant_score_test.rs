use crate::helpers::*;

#[test]
fn yuna_sp_bp5_222_constant_exactly_eight_energy_grants_one_total_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yuna = game.id("PL!SP-bp5-222-R");
    game.state.player1.stage.stage[1] = yuna;

    game.give_energy(7);
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);

    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "exactly 8 energy → live total score +1"
    );

    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "9 energy → bonus gone"
    );
}
