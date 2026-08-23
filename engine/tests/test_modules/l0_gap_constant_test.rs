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

/// PL!N-bp4-007-R+ 近江彼方: 常時 自分と相手のエネルギー合計が15枚以上 → heart02×2。
/// TODO: card_no needs verification — bp4-007-R+ not found in DB.
#[test]
#[ignore = "card_no PL!N-bp4-007-R+ not found in database"]
fn kanata_energy_15_grants_heart02x2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Use a member that actually has this ability — 近江彼方 PL!N-bp4-007-R
    let kanata = game.id("PL!N-bp4-007-R\u{ff0b}");
    game.state.player1.stage.stage = [kanata, -1, -1];
    game.give_energy(8);
    game.state.player2.energy_zone
        .cards
        .push(game.id("LL-E-001-SD"));
    game.state.player2.energy_zone.add_active(7);
    game.state.recalculate_constants();

    let h02 = game
        .state
        .mods
        .get_heart_modifier(kanata, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(h02, 2, "total energy >= 15 -> +2 heart02");
}

