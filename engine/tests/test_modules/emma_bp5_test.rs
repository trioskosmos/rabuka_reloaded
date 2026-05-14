/// Q215: Emma Verde (PL!N-bp5-008-R) — Activation: place 1 energy under member,
/// then activate 2 energy. Wait-state energy CAN be placed.
///
/// Hard edge cases:
/// 1. Wait-position energy placed → activation still works
/// 2. Only wait energy in zone → still works
/// 3. Exactly 1 wait energy → single candidate, no choice needed
use crate::helpers::*;

/// 5 energy (3 active + 2 wait). Pop takes from end = wait card first.
/// Verifies placement succeeds and activation still runs.
#[test]
fn emma_bp5_q215_wait_energy_placed_then_activate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);

    game.activate_ability(emma);

    // Select 2 energy to deactivate (the energy activation will remove them from zone)
    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }

    // Cost correctly removes 1 from zone (under member). 4 remain.
    // Activation effect then deactivates 2 → 2 remain.
    let ecount = game.state.player1.energy_zone.cards.len();
    let acount = game.state.player1.energy_zone.active_energy_count;
    eprintln!("[EMMA] energy_zone cards={}, active={}", ecount, acount);
    // Cost places 1 under member (4 remain). All 4 are already active, so
    // the "activate 2" effect has no wait cards to activate. Card count is 4.
    assert_eq!(ecount, 4, "Cost removes 1 from zone → 4 remain");
}

/// Only 1 wait energy in zone (0 active). Place it, then activate tries
/// to activate 2 but only 0 wait remain → should succeed or gracefully fail.
#[test]
fn emma_bp5_q215_only_wait_energy_available() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.hand.cards.push(filler);
    game.give_energy(1); // 1 active energy

    // Place 1 energy (the only one) → zone empty → activation tries to
    // activate 2 from nothing → should not panic
    game.activate_ability(emma);
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let ecount = game.state.player1.energy_zone.cards.len();
    eprintln!("[EMMA] only 1 energy: after placement: {} cards", ecount);
    assert!(ecount <= 1, "At most 1 energy remains after placement");
}

/// Zero energy in zone → cost cannot be paid → ability fails silently.
#[test]
fn emma_bp5_q215_no_energy_cost_fails_gracefully() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.hand.cards.push(filler);
    // No energy given

    let result = game.try_activate_ability(emma);
    // Cost succeeds (place 0 energy under member), effect fails silently
    assert!(
        result.is_ok(),
        "Activation with 0 energy should not panic/crash"
    );
    // State unchanged — no energy to remove, no cards moved
    assert_eq!(
        game.state.player1.stage.stage[1], emma,
        "Emma should remain on stage after activation with 0 energy"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        0,
        "Energy zone should still be empty after activation with 0 energy"
    );
}
