use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn s_bp6_022_live_success_with_more_opponent_energy_gains_one_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp6-022-L");
    game.state.player1.live_card_zone.cards.push(live);

    game.give_energy(1);
    // Opponent gets 3 active energies.
    for _ in 0..3 {
        let e2 = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e2);
    }
    game.state.player2.energy_zone.add_active(3);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "opponent 3 > self 1 -> score +1"
    );
}

#[test]
fn s_bp6_022_live_success_with_equal_energy_gains_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-bp6-022-L");
    game.state.player1.live_card_zone.cards.push(live);

    game.give_energy(2);
    for _ in 0..2 {
        let e2 = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e2);
    }
    game.state.player2.energy_zone.add_active(2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "opponent 2 vs self 2 is not more -> no score"
    );
}
