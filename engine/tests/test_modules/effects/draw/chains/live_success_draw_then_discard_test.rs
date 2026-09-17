use crate::helpers::*;

use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

#[test]
fn s_pb1_024_live_success_empty_hand_draws_two_discards_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-pb1-024-L");
    game.state.player1.live_card_zone.cards.push(live);

    let d1 = game.new_id(FILLER);
    let d2 = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(d1);
    game.state.player1.main_deck.cards.push(d2);
    // Hand is empty: both drawn cards are then discarded.

    let waitroom_before = game.state.player1.waitroom.cards.len();

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "drew 2 then discarded 2 (hand was empty)"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 2,
        "two cards went to the waitroom"
    );
}

#[test]
fn s_pb1_024_live_success_discarding_drawn_cards_keeps_existing_hand_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-pb1-024-L");
    game.state.player1.live_card_zone.cards.push(live);

    let kept = game.new_id(FILLER);
    game.state.player1.hand.cards.push(kept);
    let d1 = game.new_id(FILLER);
    let d2 = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(d1);
    game.state.player1.main_deck.cards.push(d2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // Discard selection exists — discard the two drawn cards (last two indices).
    assert!(
        game.has_pending_choice(),
        "discard-2 prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the 2-card discard"
    );
    let n = game.state.player1.hand.cards.len();
    assert!(n >= 2, "need at least the 2 drawn cards in hand");
    game.select_indices(&[n - 2, n - 1]);

    assert!(
        game.state.player1.hand.cards.contains(&kept),
        "the pre-existing hand card can be kept"
    );
}
