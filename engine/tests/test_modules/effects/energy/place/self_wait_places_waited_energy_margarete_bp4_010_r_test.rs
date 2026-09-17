use crate::helpers::*;

#[test]
fn margarete_bp4_010_r_activation_waits_self_and_places_waited_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);

    let zone_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();

    game.activate_ability(me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "activation cost waits this member"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "one energy card placed into the energy zone"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before - 1,
        "the energy cost consumed one active energy; the placed card is WAITED"
    );
}

#[test]
fn margarete_bp4_010_r_empty_energy_deck_still_waits_self_without_placing_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    let zone_before = game.state.player1.energy_zone.cards.len();

    game.activate_ability(me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "the wait is the cost and applies regardless"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "empty energy deck -> nothing placed"
    );
}
