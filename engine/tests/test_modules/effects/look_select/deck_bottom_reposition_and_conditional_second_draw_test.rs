//! 高海千歌's 登場 — a reposition by INDEX on a deck whose two ends mean different
//! things.
//!
//! 「自分のデッキの一番下のカード**を見る**。それをデッキの上から**4番目**に置いても**よい**。」
//!
//! Two separable claims, and two distinct ways to write a test that passes for the
//! wrong reason:
//!
//!   * 一番下 — the deck's BOTTOM. `MainDeck::draw()` takes index 0 and
//!     `draw_bottom()` takes the last, so "the top" and "the bottom" are different
//!     ends of the same vector. A fixture that seeds the wrong end produces a test
//!     that never notices which card was examined.
//!   * 4番目 — the reposition target is a fixed INDEX, not "four cards in". Landing
//!     the card at the front, or leaving it where it was read from, is a different
//!     position and a different ability.
//!
//! The deck is seeded with eight individually-identified cards and the assertion is
//! on the resulting POSITION and IDENTITY of the slot, not on "a card moved" — which
//! is what makes this a test of 4番目 rather than of 見る. Eight is chosen so
//! index 3 is neither the first nor the last, and a front or back placement cannot
//! be mistaken for it.
//!
//! ## Not covered here: sweet&sweet holiday `PL!-bp6-023-L`
//!
//! 「カードを1枚引く。自分の成功ライブカード置き場に『μ's』のカードがある場合、さらに
//! カードを1枚引く。」 — a conditional SECOND draw. Three claims are separately
//! breakable and "it drew" catches none of them: さらに (the observable is a COUNT,
//! one or two), 自分の成功ライブカード置き場 (p1's own SUCCESS zone — not the live
//! zone, not the opponent's), and 『μ's』のカード (a group filter on top).
//!
//! Four drafts failed, all on the MEASUREMENT WINDOW rather than the ability, and
//! what a probe established is worth more than the attempts:
//!
//! ```text
//!   guard=3  hand +0  deck -0  live_zone=0
//! ```
//!
//! ライブ成功時 fires at victory determination and only for a live that SUCCEEDED.
//! Dispatching the trigger directly at ライブ開始時 draws nothing because the phase
//! gate is not met; seven `pass()` calls stop one step short and read zero; an
//! open-ended walk crosses the rollover and the next turn's Draw phase puts a card
//! in hand that reads as a second draw. Adopting the
//! `integration/per_card/live_end_expiry_rollover_and_dual_trigger_window_gates_test.rs`
//! idiom (five passes, `set_live_card` from hand, two more, then pass while
//! `current_turn_phase == TurnPhase::Live` WITH `drain_auto_ability_choices` each
//! step) is what got the walk to the rollover at all — without the auto-ability
//! drain it stalls in `FirstAttackerNormal` — and from there the draw is still
//! zero with the live card having left the live zone.
//!
//! So the remaining unknown is not the card: it is where in this harness a
//! ライブ成功時 dispatch can be observed at all. That is worth resolving once,
//! because it gates every ライブ成功時 negative in the coverage report, and
//! `PL!-bp6-023-L` is only the first card to need it.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const CHIKA: &str = "PL!S-bp7-010-N"; // 高海千歌 — look at the deck BOTTOM, optionally reposition
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_decks(game: &mut TestGame) {
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Seed the deck with `count` individually-identified cards and return them plus
/// the card at the LAST index.
///
/// The bottom is REPORTED, not assumed. Which end of the vector 「一番下」 means is
/// the fact under test, and naming it in the fixture is what keeps a top-reading
/// implementation from passing here.
fn seeded_deck(game: &mut TestGame, count: usize) -> (Vec<i16>, i16) {
    game.state.player1.main_deck.cards.clear();
    // The id is allocated and bound first: `new_id` takes `&mut game` and cannot be
    // called inside the same expression that also borrows its state.
    let mut ids: Vec<i16> = Vec::with_capacity(count);
    for _ in 0..count {
        let id = game.new_id(FILLER);
        ids.push(id);
        game.state.player1.main_deck.cards.push(id);
    }
    let bottom = ids[ids.len() - 1];
    (ids, bottom)
}

/// Her 登場 examines the deck's last card and may leave it fourth from the top.
///
/// Asserted on the POSITION and the IDENTITY of the slot rather than on a count
/// having changed: an implementation that moved some other card, or moved the
/// right card to the front, leaves the same deck size.
#[test]
fn chika_repositions_the_examined_card_to_the_fourth_from_the_top() {
    let mut game = TestGame::new(load_real_database());
    let chika = game.id(CHIKA);
    game.assert_card_identity(chika, CHIKA);
    fill_decks(&mut game);
    game.give_energy(10);

    let (ids, bottom) = seeded_deck(&mut game, 8);
    assert_eq!(
        ids.len(),
        8,
        "precondition: an eight-card deck, so index 3 is neither the first nor the \
         last and a front or back placement cannot be mistaken for 4番目"
    );
    // `play_to_stage` takes her from HAND, so the deck seed and the hand entry are
    // the only two things that move before her debut.
    game.add_to_hand(chika);
    game.state.player1.stage.stage = [-1, -1, -1];
    assert_eq!(
        game.state.player1.main_deck.cards.to_vec(),
        ids,
        "precondition: the deck is exactly the seeded order before she resolves"
    );

    game.play_to_stage(chika, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 6 {
        guard += 1;
        game.select_indices(&[0]);
    }

    let deck = &game.state.player1.main_deck.cards;
    assert_eq!(
        deck.len(),
        ids.len(),
        "a look plus a reposition returns the card, so the deck size is unchanged — a \
         read that CONSUMED the card would show {}",
        ids.len() - 1
    );
    assert_eq!(
        deck[3], bottom,
        "『デッキの上から4番目』 is a fixed INDEX: the examined card — the deck's \
         BOTTOM one — now sits at index 3. A top-reading implementation would have \
         examined {} instead, and an index-0 placement would have put it at the \
         front.",
        deck[0]
    );
    assert_eq!(
        deck.iter().filter(|c| **c == bottom).count(),
        1,
        "exactly one copy of the examined card is in the deck: a reposition that \
         duplicated it would pass a membership check"
    );
    let others: Vec<i16> = ids.iter().copied().filter(|c| *c != bottom).collect();
    let got_others: Vec<i16> = deck.iter().copied().filter(|c| *c != bottom).collect();
    assert_eq!(
        got_others, others,
        "and every other card is unmoved and in order: the reposition relocates the \
         ONE examined card rather than reordering the deck"
    );
}
