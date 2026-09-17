use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

fn fire_pl_sp_bp2_005_r_debut(game: &mut TestGame, cid: i16) {
    fire_trigger(game, cid, AbilityTrigger::Debut, "登場");
}

#[test]
fn pl_sp_bp2_005_r_paid_energy_looks_seven_fetches_liella() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp2-005-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(6);
    let kanon = game.new_id("PL!SP-sd1-002-SD");
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, kanon);

    fire_pl_sp_bp2_005_r_debut(&mut game, me);
    assert!(
        game.has_pending_choice(),
        "optional {{E}}{{E}} gate offered"
    );
    game.select_option(1);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&kanon),
        "paid -> 『Liella!』 card revealed to hand"
    );
}

#[test]
fn pl_sp_bp2_005_r_unpayable_energy_skips_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp2-005-R");
    game.state.player1.stage.stage[1] = me;
    let kanon = game.new_id("PL!SP-sd1-002-SD");
    game.state.player1.main_deck.cards.insert(0, kanon);
    let deck_before = game.state.player1.main_deck.cards.len();

    fire_pl_sp_bp2_005_r_debut(&mut game, me);
    assert!(
        !game.has_pending_choice(),
        "unpayable optional {{E}}{{E}} gate must auto-skip without prompting"
    );

    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before);
    assert!(!game.state.player1.hand.cards.contains(&kanon));
}
