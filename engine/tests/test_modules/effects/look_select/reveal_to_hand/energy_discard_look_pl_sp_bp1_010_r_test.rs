use crate::helpers::*;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_sp_bp1_010_r_energy_discard_activation_fetches_liella_from_look_five() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp1-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    game.add_to_hand(game.new_id(FILLER));
    let kanon = game.new_id("PL!SP-sd1-002-SD");
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, kanon);

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&kanon),
        "『Liella!』 card revealed to hand after paying 2E + discard"
    );
}
