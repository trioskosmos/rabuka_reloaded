use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn pl_s_bp6_016_n_waitroom_debut_looks_three_takes_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-016-N");
    let a = game.new_id("PL!-sd1-010-SD");
    let b = game.new_id("PL!S-sd1-001-SD");
    let c = game.new_id("PL!N-sd1-025-SD");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.main_deck.cards.insert(0, c);
    game.state.player1.main_deck.cards.insert(0, b);
    game.state.player1.main_deck.cards.insert(0, a);

    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "discard");
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert!(game.has_pending_choice(), "waitroom debut -> look prompt");
    game.select_indices(&[0]);

    let in_hand = [&a, &b, &c]
        .iter()
        .filter(|x| game.state.player1.hand.cards.contains(x))
        .count();
    let in_wait = [&a, &b, &c]
        .iter()
        .filter(|x| game.state.player1.waitroom.cards.contains(x))
        .count();
    let on_deck = [&a, &b, &c]
        .iter()
        .filter(|x| game.state.player1.main_deck.cards.contains(x))
        .count();
    assert_eq!(in_hand, 1, "exactly one looked card added to hand");
    assert_eq!(in_wait, 2, "remaining two looked cards -> waitroom");
    assert_eq!(on_deck, 0, "all three looked cards left the deck");
}

#[test]
fn pl_s_bp6_016_n_hand_debut_no_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-016-N");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "hand");
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert!(
        !game.has_pending_choice(),
        "hand debut must NOT open the look prompt"
    );
}

#[test]
fn pl_s_bp6_016_n_no_appearance_record_no_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-016-N");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    game.state.player1.stage.stage[0] = me;
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert!(!game.has_pending_choice(), "unrecorded debut -> no prompt");
}

#[test]
fn pl_s_bp6_016_n_short_deck_takes_one_and_discards_remainder() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-016-N");
    let b = game.new_id("PL!S-sd1-001-SD");
    let c = game.new_id("PL!N-sd1-025-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(b);
    game.state.player1.main_deck.cards.push(c);
    game.give_energy(10);

    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "discard");
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert!(game.has_pending_choice(), "2 available -> still prompted");
    game.select_indices(&[0]);

    let got_one =
        game.state.player1.hand.cards.contains(&b) || game.state.player1.hand.cards.contains(&c);
    assert!(got_one, "took one of the two available cards");
    let other_in_wait = if game.state.player1.hand.cards.contains(&b) {
        game.state.player1.waitroom.cards.contains(&c)
    } else {
        game.state.player1.waitroom.cards.contains(&b)
    };
    assert!(other_in_wait, "the leftover card went to the waitroom");
    assert!(
        game.state.player1.main_deck.cards.is_empty(),
        "deck fully consumed by the short look"
    );
}
