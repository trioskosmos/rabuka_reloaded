/// Untested-abilities batch 19 — draw-then-discard sequentials:
/// - PL!SP-bp1-009-R (起動 ターン1回, 1E): draw 1, then discard 1 from hand
/// - PL!S-pb1-024-L (ライブ成功時): draw 2, then discard 2 from hand
use crate::helpers::*;

use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

// ====================================================================
// PL!SP-bp1-009-R (起動 ターン1回, 1E):
// 「カードを1枚引き、手札を1枚控え室に置く。」
// ====================================================================

#[test]
fn bp1009_activation_draws_one_discards_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp1-009-R");
    game.state.player1.stage.stage[0] = me;
    game.give_energy(1);

    // Pre-existing hand card, plus a known deck order.
    let held = game.new_id(FILLER);
    game.state.player1.hand.cards.push(held);
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);

    let waitroom_before = game.state.player1.waitroom.cards.len();

    game.activate_ability(me);
    assert!(
        game.has_pending_choice(),
        "hand discard prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard"
    );
    // Discard 1 from hand — pick index 0 (whichever card that is).
    game.select_indices(&[0]);

    assert_eq!(game.state.player1.hand.cards.len(), 1, "net hand size stays 1");
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 1,
        "exactly one card was discarded to the waitroom"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&drawn),
        "the stocked deck card was drawn"
    );
}

// ====================================================================
// PL!S-pb1-024-L (ライブ成功時):
// 「カードを2枚引き、手札を2枚控え室に置く。」
// ====================================================================

#[test]
fn pb1024_live_success_draws_two_discards_two_empty_hand() {
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
fn pb1024_live_success_keeps_non_selected_cards() {
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
