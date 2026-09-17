use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn pl_s_bp6_011_n_waitroom_debut_draws_two_discards_one() {
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
    assert!(game.has_pending_choice(), "hand discard prompt expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard"
    );
    game.select_indices(&[0]);

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
fn pl_s_bp6_011_n_hand_debut_no_draw_or_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-011-N");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let hand_card = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(hand_card);

    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "hand");

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
fn pl_s_bp6_011_n_one_card_deck_still_discards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-011-N");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state
        .player1
        .main_deck
        .cards
        .push(game.new_id("PL!-sd1-010-SD"));
    game.give_energy(10);

    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "discard");

    let discard_me = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(discard_me);

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");
    assert!(
        game.has_pending_choice(),
        "hand discard prompt expected even when the draw comes up short"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard"
    );
    game.select_indices(&[0]);

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
fn pl_s_bp6_011_n_empty_deck_and_hand_move_no_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-bp6-011-N");

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();

    game.state.player1.stage.stage[0] = me;
    game.state.record_card_appearance(me, "discard");

    let hand_before = game.state.player1.hand.cards.len();
    let wait_before = game.state.player1.waitroom.cards.len();
    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "empty deck stays empty"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "empty hand gains nothing"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wait_before,
        "nothing discarded on noop"
    );
}
