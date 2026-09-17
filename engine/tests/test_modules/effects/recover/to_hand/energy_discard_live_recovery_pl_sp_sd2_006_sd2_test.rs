use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_sp_sd2_006_sd2_energy_discard_activation_recovers_liella_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-sd2-006-SD2");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    let cost_target = game.new_id(FILLER);
    game.add_to_hand(cost_target);
    let live = game.new_id("PL!SP-bp1-026-L");
    game.state.player1.waitroom.cards.push(live);

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "Liella! live card retrieved from the waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "live left the waitroom"
    );
}
