/// Untested-abilities batch 33 — waitroom-debut gates (控え室から登場している場合).
///
/// - PL!S-bp6-016-N (登場): if debuted FROM the waitroom, look at top 3 deck
///   cards, add 1 to hand, put the rest in the waitroom.
/// - PL!S-bp6-011-N (登場): if debuted FROM the waitroom, draw 2 then
///   discard 1 from hand.
///
/// The engine records each debut's source zone (record_card_appearance);
/// these tests simulate a real waitroom debut by recording source "discard"
/// before firing the 登場 trigger.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

// ====================================================================
// PL!S-bp6-016-N — waitroom-debut look 3 / take 1 / rest to waitroom
// ====================================================================

#[test]
fn bp6_016n_waitroom_debut_looks_three_takes_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-016-N");
    let a = game.new_id("PL!-sd1-010-SD");
    let b = game.new_id("PL!S-sd1-001-SD");
    let c = game.new_id("PL!N-sd1-025-SD");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    // Deck top order: [a, b, c, ...filler].
    game.state.player1.main_deck.cards.insert(0, c);
    game.state.player1.main_deck.cards.insert(0, b);
    game.state.player1.main_deck.cards.insert(0, a);

    // Simulate a waitroom debut, then fire 登場.
    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "discard");
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert!(game.has_pending_choice(), "waitroom debut -> look prompt");
    game.select_indices(&[0]); // take the first looked card

    // Exactly one of {a,b,c} reached the hand; the other two went to the
    // waitroom; none remain on the deck top.
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
fn bp6_016n_hand_debut_no_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-016-N");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    // Normal hand debut — wrong appearance source.
    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "hand");
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert!(
        !game.has_pending_choice(),
        "hand debut must NOT open the look prompt"
    );
}

#[test]
fn bp6_016n_no_appearance_record_no_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-016-N");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    // Direct placement with no recorded debut at all (e.g. engine-internal
    // setup) — gate must fail closed.
    game.state.player1.stage.stage[0] = me;
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert!(!game.has_pending_choice(), "unrecorded debut -> no prompt");
}

#[test]
fn bp6_016n_deck_shorter_than_three_looks_whats_there() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-016-N");
    let b = game.new_id("PL!S-sd1-001-SD");
    let c = game.new_id("PL!N-sd1-025-SD");

    // Deck holds EXACTLY two cards — fewer than the look count.
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

    let got_one = game.state.player1.hand.cards.contains(&b)
        || game.state.player1.hand.cards.contains(&c);
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

// ====================================================================
// PL!S-bp6-011-N — waitroom-debut draw 2 + discard 1
// ====================================================================

#[test]
fn bp6_011n_waitroom_debut_draws_two_discards_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-011-N");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let hand_card = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_card);

    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "discard");

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");
    if game.has_pending_choice() {
        game.select_indices(&[0]); // choose which hand card to discard
    }

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        2,
        "waitroom debut -> draw exactly 2"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&hand_card),
        "second step discards 1 hand card"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&hand_card),
        "discarded card lands in the waitroom"
    );
}

#[test]
fn bp6_011n_hand_debut_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-011-N");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let hand_card = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_card);

    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "hand"); // wrong source

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert_eq!(
        deck_before,
        game.state.player1.main_deck.cards.len(),
        "hand debut -> no draw"
    );
    assert!(
        game.state.player1.hand.cards.contains(&hand_card),
        "no discard either"
    );
}

#[test]
fn bp6_011n_deck_with_one_card_still_discards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-011-N");

    // Deck holds exactly ONE card — draw comes up short but the discard
    // step is unconditional and must still run.
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(game.new_id("PL!-sd1-010-SD"));
    game.give_energy(10);

    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "discard");

    let discard_me = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(discard_me);

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.main_deck.cards.is_empty(),
        "drew the single available card"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&discard_me),
        "discard step still ran despite short draw"
    );
    assert!(game.state.player1.waitroom.cards.contains(&discard_me));
}

#[test]
fn bp6_011n_empty_deck_and_empty_hand_is_a_clean_noop() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-011-N");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();

    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "discard");

    // Must not panic: draw 0 from an empty deck, discard from an empty hand.
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");
}
