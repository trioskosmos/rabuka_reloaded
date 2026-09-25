use crate::helpers::*;

fn give_opp_energy(game: &mut TestGame, count: usize) {
    for _ in 0..count {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(count as u8);
}

#[test]
fn s_bp7_014_constant_opponent_energy_ahead_grants_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp7-014-N");
    game.state.player1.stage.stage[0] = me;
    game.give_energy(1);
    give_opp_energy(&mut game, 3);

    game.state.recalculate_constants();

    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    assert!(
        game.state.mods.get_heart_modifier(me, H02) > 0,
        "opponent energy ahead -> heart02 granted"
    );
}

#[test]
fn s_bp7_014_constant_energy_tied_grants_no_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp7-014-N");
    game.state.player1.stage.stage[0] = me;
    game.give_energy(2);
    give_opp_energy(&mut game, 2);

    game.state.recalculate_constants();

    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    assert_eq!(
        game.state.mods.get_heart_modifier(me, H02),
        0,
        "tied energy -> no heart02"
    );
}
