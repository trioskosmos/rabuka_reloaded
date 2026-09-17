use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const LIVE_START: &str = "ライブ開始時";
const FILLER: &str = "PL!-sd1-010-SD";

fn setup_pl_s_bp7_023_l_energy_return_score(game: &mut TestGame) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);

    let live = game.id("PL!S-bp7-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);

    let chika_a = game.new_id("PL!S-sd1-001-SD");
    let chika_b = game.new_id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage[0] = chika_a;
    game.state.player1.stage.stage[1] = chika_b;

    let e = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(e);
    game.state.player1.energy_zone.add_active(1);
    live
}

fn give_pl_s_bp7_023_l_opponent_active_energy(game: &mut TestGame, n: usize) {
    for _ in 0..n {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(n as u8);
}

#[test]
fn pl_s_bp7_023_l_return_energy_opponent_one_ahead_grants_one_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_pl_s_bp7_023_l_energy_return_score(&mut game);
    give_pl_s_bp7_023_l_opponent_active_energy(&mut game, 1);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_option(1);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "opponent exactly 1 energy ahead after the return -> live +1"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "our energy was returned to the energy deck"
    );
}

#[test]
fn pl_s_bp7_023_l_return_energy_opponent_two_ahead_grants_two_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_pl_s_bp7_023_l_energy_return_score(&mut game);
    give_pl_s_bp7_023_l_opponent_active_energy(&mut game, 2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_option(1);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "opponent 2+ energies ahead -> alternative +2 instead"
    );
}

#[test]
fn pl_s_bp7_023_l_tied_energy_after_return_grants_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = setup_pl_s_bp7_023_l_energy_return_score(&mut game);
    give_pl_s_bp7_023_l_opponent_active_energy(&mut game, 0);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_option(1);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "tied energy counts -> no score bonus"
    );
}
