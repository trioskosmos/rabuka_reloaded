/// L0 gap coverage: simple Constant (常時) abilities.
///
/// Each test places the card, sets up the trigger condition, and asserts
/// the exact modifier value. Positive + negative pairs where possible.
use crate::helpers::*;

/// PL!SP-bp1-004-PR 平安名すみれ: 常時 センターにいる場合、ブレード+5。
#[test]
fn sumire_pr_center_position_grants_blade() {
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

/// PL!N-bp4-007-R+ 優木せつ菜: 常時 自分と相手のエネルギー合計が15枚以上
/// のかぎり、heart02×2を得る。
#[test]
fn bp4_007_setsuna_both_energy_15_grants_heart02x2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let setsuna = game.id("PL!N-bp4-007-R\u{ff0b}");
    game.state.player1.stage.stage = [setsuna, -1, -1];
    game.give_energy(8);
    // Opponent energy must be real cards in the zone (active counter alone
    // doesn't create them).
    for _ in 0..7 {
        game.state
            .player2
            .energy_zone
            .cards
            .push(game.id("LL-E-001-SD"));
    }
    game.state.player2.energy_zone.add_active(7);
    game.state.recalculate_constants();

    let h02 = game
        .state
        .mods
        .get_heart_modifier(setsuna, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(h02, 2, "total energy >= 15 -> +2 heart02");

    // Negative: drop total below 15 → modifier gone.
    game.state.player2.energy_zone.cards.truncate(3);
    game.state.player2.energy_zone.set_active_count(3);
    game.state.recalculate_constants();
    let h02_low = game
        .state
        .mods
        .get_heart_modifier(setsuna, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(h02_low, 0, "total energy < 15 -> no heart02");

    // Wait-state energy still counts: the condition says 「エネルギーの合計が
    // 15枚以上ある」 with no state qualifier (unlike effects that explicitly
    // require アクティブ状態のエネルギー), so every card in the zone counts.
    // 15 total cards but only 10 active — an active-only implementation
    // would grant nothing here.
    for _ in 0..4 {
        game.state
            .player2
            .energy_zone
            .cards
            .push(game.id("LL-E-001-SD"));
    }
    game.state.player2.energy_zone.set_active_count(2);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.player1.energy_zone.cards.len()
            + game.state.player2.energy_zone.cards.len(),
        15,
        "precondition: 15 total energy cards"
    );
    let h02_wait = game
        .state
        .mods
        .get_heart_modifier(setsuna, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(
        h02_wait, 2,
        "15 total cards incl. wait-state energy -> +2 heart02"
    );
}

