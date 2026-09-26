//! The last two abilities the depth ladder called thin: a bare 登場 draw, and a
//! ライブ成功時 whose condition is a DISJUNCTION across two different zones.
//!
//! 平安名すみれ is the more interesting of the two. Her ライブ成功時 reads:
//!
//!   「自分のライブカード置き場の中に元々のスコアより高いスコアのライブカードがあるか、
//!   エールにより公開された自分のカードの中に{{スコア}}を持つライブカードがある場合、
//!   カードを1枚引く。」
//!
//! That is an OR of two conditions over two UNRELATED zones — p1's own live-card
//! zone, and the yell-revealed set — each of which can satisfy it alone. A single
//! positive reading cannot say which branch was taken, and an implementation that
//! required BOTH (reading the comma as AND) would pass a test that happened to
//! satisfy both. So each branch is exercised alone, and then with neither.
//!
//! 大沢瑠璃乃's 登場 is 「カードを1枚引く」 with nothing on it at all, which makes it
//! the cheapest draw in the deck — and therefore the right place to pin what a draw
//! does at the DECK EDGE, since an empty deck is the one case a well-stocked fixture
//! never reaches.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const SUMIRE: &str = "PL!SP-pb2-004-R"; // 平安名すみれ — OR of two zone conditions → draw 1
const RURINO: &str = "PL!HS-bp5-011-N"; // 大沢瑠璃乃 — 登場 draw 1
const FILLER: &str = "PL!-sd1-010-SD";
const LIVE: &str = "PL!-sd1-019-SD";

fn drain(game: &mut TestGame) {
    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectTarget") {
            game.select_option(0);
        } else {
            game.select_indices(&[]);
        }
    }
}

/// Run すみれ's ライブ成功時 with the given revealed set and live zone, and return
/// how many cards reached hand.
fn draw_under(game: &mut TestGame, revealed_live: bool, boost_live_zone: bool) -> i32 {
    let sumire = game.id(SUMIRE);
    game.state.player1.stage.stage = [sumire, -1, -1];
    game.state.player1.live_card_zone.cards.clear();
    game.state.revealed_cards.clear();

    let live = game.new_id(LIVE);
    game.state.player1.live_card_zone.cards.push(live);
    if boost_live_zone {
        // 「元々のスコアより高い」 — a live whose CURRENT score exceeds its PRINTED
        // one. The modifier is what makes the first clause true; the card sitting in
        // the zone is not enough on its own.
        game.state.mods.add_score_modifier(live, 5);
    }
    if revealed_live {
        game.state.revealed_cards.push(game.new_id(LIVE));
    }
    // A hand with cards to draw into, so "did it draw" is about the DECK, not
    // about a full hand.
    for _ in 0..3 {
        game.state.player1.hand.cards.push(game.new_id(FILLER));
    }
    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();

    fire_trigger(game, sumire, AbilityTrigger::LiveSuccess, "ライブ成功時");
    drain(game);

    // The hand grew by exactly what the deck lost — a draw, not a copy. Stated as a
    // real equality rather than a presence check, so an implementation that added a
    // card without removing one (or drew from elsewhere) fails here.
    let drawn = game.state.player1.hand.cards.len() as i32 - hand_before as i32;
    let lost = deck_before as i32 - game.state.player1.main_deck.cards.len() as i32;
    assert_eq!(
        drawn, lost,
        "the hand grew by {drawn} while the deck lost {lost} — a draw moves one card, \
         it does not duplicate or source it elsewhere"
    );
    drawn
}

/// The OR: each clause alone is enough, and the absence of both withholds the draw.
///
/// Four runs, because the failure this guards against — reading the printed comma as
/// an AND — only shows up when exactly ONE clause is true.
#[test]
fn sumire_draws_when_either_zone_condition_alone_holds() {
    let mut game = TestGame::new(load_real_database());
    game.assert_card_identity(game.id(SUMIRE), SUMIRE);
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // NEITHER clause: a live in the zone with no boost, and an empty revealed set.
    assert_eq!(
        draw_under(&mut game, false, false),
        0,
        "with no live scoring above its printed value AND no scored live in the \
         revealed set, the OR is false and nothing is drawn"
    );

    // CLAUSE A alone: the live zone's current score exceeds its printed score.
    assert_eq!(
        draw_under(&mut game, false, true),
        1,
        "『元々のスコアより高いスコアのライブカードがあるか』 alone satisfies the OR and \
         draws exactly one card"
    );

    // CLAUSE B alone: a scored live in the revealed set.
    assert_eq!(
        draw_under(&mut game, true, false),
        1,
        "『エールにより公開された自分のカードの中にスコアを持つライブカードがある場合』 \
         alone satisfies the OR — and satisfying ONLY this clause is the case an AND \
         reading fails"
    );

    // Both: still one card, not two. The OR grants the effect once.
    assert_eq!(
        draw_under(&mut game, true, true),
        1,
        "both clauses true still draws exactly ONE card — the printed text is a single \
         draw guarded by a disjunction, not one draw per satisfied branch"
    );
}

/// 「カードを1枚引く」 with nothing else on the card, at the DECK EDGE.
///
/// The empty-deck case is the one a well-stocked fixture never reaches, and it is the
/// only place a draw implementation can be caught: a draw from an empty deck must
/// leave the hand unchanged and must not consume a card from somewhere else.
#[test]
fn rurino_draws_one_card_and_draws_nothing_from_an_empty_deck() {
    let mut game = TestGame::new(load_real_database());
    let rurino = game.id(RURINO);
    game.assert_card_identity(rurino, RURINO);

    // A stocked deck: exactly one card is drawn, from the deck.
    game.state.player1.main_deck.cards.clear();
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(game.new_id(FILLER));
    }
    game.state.player1.hand.cards.clear();
    game.add_to_hand(rurino);
    game.give_energy(20);
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        5,
        "precondition: a stocked deck, and the card in hand is not counted here"
    );

    game.play_to_stage(rurino, MemberArea::Center);
    drain(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "『カードを1枚引く』 — exactly one card, and it came from the deck"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        4,
        "5 - 1 = 4: the draw is a real deck-to-hand move, not a hand-only shuffle"
    );

    // The edge: an empty deck.
    let mut empty = TestGame::new(load_real_database());
    let rurino2 = empty.id(RURINO);
    empty.state.player1.main_deck.cards.clear();
    empty.state.player1.hand.cards.clear();
    for _ in 0..3 {
        empty.state.player1.hand.cards.push(empty.new_id(FILLER));
    }
    empty.add_to_hand(rurino2);
    empty.give_energy(20);
    assert!(
        empty.state.player1.main_deck.cards.is_empty(),
        "precondition: the deck really is empty"
    );

    empty.play_to_stage(rurino2, MemberArea::Center);
    drain(&mut empty);

    assert_eq!(
        empty.state.player1.hand.cards.len(),
        3,
        "an empty deck yields no card and consumes nothing — the 3 cards already in \
         hand are untouched, and a draw that fell back to the waitroom or the hand \
         would show up here (got {})",
        empty.state.player1.hand.cards.len()
    );
    assert!(
        empty.state.player1.waitroom.cards.is_empty(),
        "and nothing was taken from the waitroom as a substitute"
    );
}
