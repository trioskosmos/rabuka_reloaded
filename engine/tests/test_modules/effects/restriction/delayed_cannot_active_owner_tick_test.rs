use crate::helpers::*;

#[test]
fn delayed_cannot_active_expires_after_one_owner_tick() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");
    let _filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = himeno;
    game.state.mods.add_orientation_modifier(himeno, "wait");

    game.state.mods.add_delayed_cannot_active(himeno, 1);

    // Check that is_delayed_cannot_active returns true
    assert!(game.state.mods.is_delayed_cannot_active(himeno));

    // Tick (simulate next Active phase processing) — owner-scoped for the card's owner
    let owned: std::collections::HashSet<i16> =
        game.state.player1.all_card_ids().into_iter().collect();
    game.state.mods.tick_delayed_cannot_active_for(&owned);

    // After one tick, flag should be 0 → is_delayed_cannot_active returns false
    assert!(!game.state.mods.is_delayed_cannot_active(himeno));
}

#[test]
fn repeated_delayed_cannot_active_does_not_extend_owner_ticks() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");

    game.state.player1.stage.stage[0] = himeno;
    game.state.mods.add_orientation_modifier(himeno, "wait");

    // Set delayed flag twice — should keep max (not increase beyond 1)
    game.state.mods.add_delayed_cannot_active(himeno, 1);
    game.state.mods.add_delayed_cannot_active(himeno, 1);

    let owned: std::collections::HashSet<i16> =
        game.state.player1.all_card_ids().into_iter().collect();
    game.state.mods.tick_delayed_cannot_active_for(&owned); // 1 → 0
    assert!(!game.state.mods.is_delayed_cannot_active(himeno));
}
