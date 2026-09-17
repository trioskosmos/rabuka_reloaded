use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_s_pb1_013_n_debut_look_four_takes_live_requiring_two_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!S-pb1-013-N");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let seed = game.new_id("PL!HS-bp2-020-L");
    game.state.player1.main_deck.cards.insert(0, seed);

    fire_debut_accept(&mut game, me);

    assert!(
        game.state.player1.hand.cards.contains(&seed),
        "live requiring heart04x2 matches the OR filter"
    );
}

#[test]
fn pl_s_pb1_013_n_debut_look_four_excludes_plain_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!S-pb1-013-N");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let plain = game.new_id(FILLER);
    game.state.player1.main_deck.cards.insert(0, plain);

    fire_debut_accept(&mut game, me);

    assert!(
        !game.state.player1.hand.cards.contains(&plain),
        "member without heart04 tie must NOT be taken"
    );
}

#[test]
fn pl_s_pb1_014_n_debut_look_four_takes_live_requiring_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!S-pb1-014-N");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let seed = game.new_id("PL!SP-pb1-023-L");
    game.state.player1.main_deck.cards.insert(0, seed);

    fire_debut_accept(&mut game, me);

    assert!(game.state.player1.hand.cards.contains(&seed));
}

fn fire_debut_accept(game: &mut TestGame, me: i16) {
    fire_trigger(game, me, AbilityTrigger::Debut, "登場");
    assert!(
        game.has_pending_choice(),
        "optional discard cost prompt expected on debut"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (hand cost gate)"
    );
    game.select_option(0);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
}
