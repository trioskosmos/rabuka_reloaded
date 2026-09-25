use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

fn fire_subunit_look_debut(game: &mut TestGame, cid: i16) {
    fire_trigger(game, cid, AbilityTrigger::Debut, "登場");
}

fn resolve_paid_discard_subunit_look(
    game: &mut TestGame,
    holder_no: &str,
    seed_no: &str,
) -> (i16, i16) {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id(holder_no);
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let seed = game.new_id(seed_no);
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, seed);
    fire_subunit_look_debut(game, me);
    assert!(
        game.has_pending_choice(),
        "{holder_no}: optional discard gate must be offered"
    );
    game.select_option(0);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    (me, seed)
}

#[test]
fn pl_pb1_016_r_paid_discard_looks_four_fetches_lilywhite() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (_, seed) = resolve_paid_discard_subunit_look(&mut game, "PL!-pb1-016-R", "PL!-PR-007-PR");
    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "lilywhite card revealed to hand"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&seed),
        "fetched card left the deck"
    );
}

#[test]
fn pl_pb1_016_r_declined_discard_leaves_deck_count_unchanged() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let me = game.id("PL!-pb1-016-R");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let seed = game.new_id("PL!-PR-007-PR");
    game.state.player1.main_deck.cards.insert(0, seed);
    let deck_before = game.state.player1.main_deck.cards.len();

    fire_subunit_look_debut(&mut game, me);
    game.select_option(1);

    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before);
    assert!(!game.state.player1.hand.cards.contains(&seed));
}

#[test]
fn pl_sp_pb1_017_n_paid_discard_looks_five_fetches_5yncri5e() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (_, seed) =
        resolve_paid_discard_subunit_look(&mut game, "PL!SP-pb1-017-N", "PL!SP-PR-005-PR");
    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "『5yncri5e!』 card revealed to hand"
    );
}

#[test]
fn pl_hs_pb1_018_n_paid_discard_looks_five_fetches_dollchestra() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (_, seed) =
        resolve_paid_discard_subunit_look(&mut game, "PL!HS-pb1-018-N", "PL!HS-bp2-008-R");
    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "『DOLLCHESTRA』 card revealed to hand"
    );
}
