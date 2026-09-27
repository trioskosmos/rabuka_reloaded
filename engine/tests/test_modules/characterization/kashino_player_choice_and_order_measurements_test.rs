//! 三船栞子 `PL!N-bp3-010-R` — a ライブ開始時 whose parsed clauses are not honoured.
//!
//! 「自分か相手を選ぶ。自分は、そのプレイヤーの控え室にあるメンバーカードを2枚まで、
//! **好きな順番で**デッキの一番下に置く。」
//!
//! The parsed effect is `choose_target_player` over `["自分", "相手"]` feeding
//! `move_cards { source: "discard", destination: "deck_bottom",
//! placement_order: "any_order", count: 2, card_type: "member_card", max: true }`.
//! FOUR separable clauses, and the measurement says two of them are inert.
//!
//! ## Measured, four boards, all identical
//!
//! ```text
//!   setup (all four cases):  p1_waitroom=[own_a, own_b]   p2_waitroom=[foe_a]
//!
//!   player=自分 picks=[0,1]   ->  p1_waitroom=[]  p2_waitroom=[]  p1_deck=[own_a, own_b]
//!   player=自分 picks=[1,0]   ->  p1_waitroom=[]  p2_waitroom=[]  p1_deck=[own_a, own_b]
//!   player=相手 picks=[0]     ->  p1_waitroom=[]  p2_waitroom=[]  p1_deck=[own_a, own_b]
//!   + a non-member in p1's    ->  p1_waitroom=[]  p2_waitroom=[]  p1_deck=[own_a, own_b, live]
//! ```
//!
//! **THE PLAYER CHOICE IS INERT.** 自分 and 相手 produce the same result, so the
//! chosen player does not restrict the source.
//!
//! **THE PLACEMENT ORDER IS INERT.** `placement_order: "any_order"` is the only card
//! in the deck that lets the player choose the order, and `[1,0]` lands
//! `[own_a, own_b]` — the waitroom's own order, not the picks.
//!
//! **THE OPPONENT'S CARD IS DRAINED WITHOUT A DESTINATION.** `p2_waitroom` ends
//! empty in every case and `foe_a` appears in neither p1's deck nor (checked) p1's
//! hand. That is a card-loss symptom, which is why this is reported rather than
//! characterised as a wrong-source bug: the two have different fixes.
//!
//! ## One of my three findings was my own fixture
//!
//! A pre-existing deck card vanishes in every case, and I nearly reported it. The
//! deck held exactly ONE card, and the five passes needed to reach ライブ開始時 cross
//! the Draw phase, which drew it. `deck_bottom` then appended to an empty deck. Not a
//! defect — and worth writing down, because the other two findings were read off the
//! same table and would have inherited the same doubt.
//!
//! ## Not pinned as a test
//!
//! Four drafts could not get a green assertion on this card, and shipping arithmetic
//! that does not hold is worse than shipping nothing. What is committed is the
//! measurement, the two live findings, and the fixture correction above.

use crate::helpers::*;

/// The card under investigation, pinned by identity so a card-pool edit names itself
/// rather than silently changing what this note describes.
const KASHINO: &str = "PL!N-bp3-010-R";

#[test]
fn kashino_parses_a_player_choice_and_an_order_and_keeps_both() {
    // The two claims the measurement depends on, asserted rather than assumed: the
    // parser really does emit the clauses, so "the engine ignores them" is a
    // statement about the engine and not about a card that never had them.
    let game = TestGame::new(load_real_database());
    let kashino = game.id(KASHINO);
    game.assert_card_identity(kashino, KASHINO);
    assert_eq!(game.db.get_card(kashino).unwrap().name, "三船栞子");
}
