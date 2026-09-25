use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Target: PL!N-bp4-007-R+ (Hanayo)
// 常時: If combined total energy of both players >= 15 → gain heart02 x2.
// So: card_count_condition across BOTH players.
// ====================================================================

/// Edge case: Combined energy exactly 15 → gain 2x heart02.
#[test]
fn combined_energy_exactly_fifteen_grants_two_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanayo = game.id("PL!N-bp4-007-R+");
    let energy = game.id("LL-E-001-SD");

    game.add_to_stage(MemberArea::Center, hanayo);

    // P1: 10 energy, P2: 5 energy = 15 total
    let e1 = game.state.player1.energy_zone.active_count();
    game.state.player1.energy_zone.set_active_count(e1.min(10));
    for _ in 0..10 {
        game.state.player1.energy_zone.cards.push(energy);
    }
    for _ in 0..5 {
        game.state.player2.energy_zone.cards.push(energy);
    }

    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(hanayo, HeartColor::Heart02);
    assert_eq!(
        heart_mod, 2,
        "Hanayo: combined energy=15 → +2 heart02, got {}",
        heart_mod
    );
}

/// Edge case: Combined energy 14 → no heart gain.
#[test]
fn combined_energy_fourteen_grants_no_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanayo = game.id("PL!N-bp4-007-R+");
    let energy = game.id("LL-E-001-SD");

    game.add_to_stage(MemberArea::Center, hanayo);

    for _ in 0..10 {
        game.state.player1.energy_zone.cards.push(energy);
    }
    for _ in 0..4 {
        game.state.player2.energy_zone.cards.push(energy);
    }

    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(hanayo, HeartColor::Heart02);
    assert_eq!(
        heart_mod, 0,
        "Hanayo: combined energy=14 → no heart, got {}",
        heart_mod
    );
}
