use crate::helpers::*;

fn give_opp_energy(game: &mut TestGame, count: usize) {
    for _ in 0..count {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(count as u8);
}

#[test]
fn sp_bp7_020_constant_own_energy_ahead_grants_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp7-020-N");
    game.state.player1.stage.stage[0] = me;
    game.give_energy(3);
    give_opp_energy(&mut game, 1);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        2,
        "energy ahead -> +2 blades"
    );
}

#[test]
fn sp_bp7_020_constant_own_energy_behind_grants_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp7-020-N");
    game.state.player1.stage.stage[0] = me;
    game.give_energy(1);
    give_opp_energy(&mut game, 3);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "energy behind -> no blades"
    );
}
