use crate::helpers::*;

const FILLER: &str = "PL!N-sd1-010-SD";

fn answer_all(game: &mut TestGame, idx: usize) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        if game.pending_choice_type().as_deref() == Some("SelectHeartColor") {
            game.select_choice_option(idx);
        } else {
            game.select_indices(&[idx]);
        }
    }
}

#[test]
fn pl_sp_bp5_021_n_activation_six_energy_places_one_waited_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp5-021-N");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(6);
    fill_energy_deck(&mut game, 0, 2);
    let zone_before = game.state.player1.energy_zone.cards.len();

    game.activate_ability(me);
    answer_all(&mut game, 0);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "energy placed from the deck into the zone"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        6,
        "placed energy is WAITED (active count unchanged by placement)"
    );
}

#[test]
fn pl_sp_bp5_021_n_activation_five_energy_pays_self_waitroom_without_placing_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp5-021-N");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);
    let zone_before = game.state.player1.energy_zone.cards.len();

    game.activate_ability(me);
    answer_all(&mut game, 0);
    assert!(
        game.state.player1.waitroom.cards.contains(&me),
        "self-exile cost is paid even below the >=6 energy gate"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "energy <6 -> no energy placed from the deck"
    );
}
