use crate::helpers::*;

#[test]
fn tomari_sp_bp7_022_activation_with_energy_returns_one_energy_and_leaves_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp7-022-N");
    // Start at CENTER; left side is free for the position change.
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);

    let deck_before = game.state.player1.energy_deck.cards.len();
    let zone_before = game.state.player1.energy_zone.cards.len();
    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before + 1,
        "one energy returned to the energy deck"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before - 1,
        "zone lost the paid energy"
    );
    assert_ne!(
        game.state.player1.stage.stage.iter().position(|&c| c == me),
        Some(1),
        "member changed position out of the center"
    );
}

#[test]
fn tomari_sp_bp7_022_activation_without_energy_keeps_member_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp7-022-N");
    game.state.player1.stage.stage[1] = me;
    // NO energy at all -> the {E} cost cannot be paid; activation no-ops
    // without panicking.
    game.activate_ability(me);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert!(
        game.state.player1.stage.stage.iter().any(|&c| c == me),
        "member stays on stage"
    );
}
