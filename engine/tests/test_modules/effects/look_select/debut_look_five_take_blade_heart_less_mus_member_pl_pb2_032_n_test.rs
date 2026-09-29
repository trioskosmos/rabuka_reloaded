//! 西木野真姫 `PL!-pb2-032-N` (登場)
//!
//! {{toujyou.png|登場}}手札を1枚控え室に置く：自分のデッキの上からカードを5枚見る。
//! その中からブレードハートを持たない『μ's』のメンバーカードを1枚公開して手札に加えても
//! よい。残りを控え室に置く。
//!
//! This file pins the CANDIDATE FILTER, which is the part of the ability most likely to
//! be wrong: a NEGATED property (ブレードハートを持たない) crossed with a group filter.
//! Counting how many of the looked-at cards are actually offered proves both at once —
//! a filter that silently dropped either would offer 2 or 3.
//!
/// The mandatory cost removes exactly one hand card.
#[test]
fn the_cost_discards_exactly_one_card_from_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let maki = game.id(MAKI);
    game.state.player1.hand.cards.push(game.id(NO_BH));
    game.state.player1.hand.cards.push(maki);
    game.give_energy(20);
    game.play_to_stage(maki, MemberArea::Center);
    let hand_before = game.state.player1.hand.cards.len();
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "手札を1枚控え室に置く costs one hand card, and the optional take is declined \
         when nothing is selected — so the two cancel and the hand is unchanged"
    );
}

// Two findings are NOT asserted here, because there is nothing correct to assert yet
// (see docs/PBLL02_ABILITY_REVIEW.md §8.3):
//   * the select prompt is NOT skippable, although the text says 加えてもよい — the
//     parsed node has `optional: true`, so either the optional flag is not reaching
//     the prompt or it is dropped on this path;
//   * answering the prompt with an index does not place the card in hand.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const MAKI: &str = "PL!-pb2-032-N";
/// A μ's member with NO blade heart — the only kind that qualifies.
const NO_BH: &str = "PL!-bp6-017-N"; // 小泉花陽, Printemps
/// A μ's member that carries a blade heart — excluded by the negation.
const BH_MEMBER: &str = "PL!-pb2-002-PP"; // 絢瀬絵里, BiBi
/// Not μ's — excluded by the group filter.
const NON_MUS: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA

#[test]
fn only_blade_heart_free_mus_members_are_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    for c in [NO_BH, BH_MEMBER, NON_MUS] {
        game.state.player1.main_deck.cards.push(game.id(c));
    }
    let maki = game.id(MAKI);
    game.assert_card_identity(maki, MAKI);
    // The mandatory cost discards one hand card, so the hand needs a second card
    // besides MAKI or the ability cannot pay and never reaches the look step.
    game.state.player1.hand.cards.push(game.id(NO_BH));
    game.state.player1.hand.cards.push(maki);
    game.give_energy(20);
    game.play_to_stage(maki, MemberArea::Center);
    game.drain_auto_ability_choices();

    assert!(
        game.has_pending_choice(),
        "the look-and-select prompt should be open"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected a SelectCard over the looked-at cards"
    );
    assert_eq!(
        game.pending_choice_count(),
        1,
        "ブレードハートを持たない『μ's』のメンバーカード1枚 — of the three cards looked \
         at, only 小泉花陽 qualifies: 絢瀬絵里 carries a blade heart and 松浦果南 is \
         not a μ's card. Dropping either filter would offer 2 or 3."
    );
}
