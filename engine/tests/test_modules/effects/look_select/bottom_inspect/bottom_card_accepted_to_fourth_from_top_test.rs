//! 高海千歌 `PL!S-bp7-010-N`:
//!
//!   {{toujyou.png|登場}}自分のデッキの一番下のカード**を見る**。それをデッキの上から
//!   **4番目**に置**いてもよい**。
//!
//! Two things are being tested, and the previous version of this file tested neither
//! convincingly.
//!
//! **The decline path was vacuous.** `pl_s_bp7_010_n_decline_keeps_bottom_card` pushed a
//! single card into the deck and then asserted
//!
//! ```text
//! deck.contains(&x)      → trivially true, the deck holds one card and it is x
//! deck.last() == Some(&x) → trivially true, for the same reason
//! ```
//!
//! Nothing about "stays at the bottom" or "stays in the deck" was actually checked, because
//! a one-card deck cannot distinguish those from each other or from the card being
//! teleported anywhere. 見る under rule 5.7 informs without acquiring, so the deck must come
//! back *byte-for-byte identical* — and that is now asserted on a real deck.
//!
//! **The accept path never checked the rest of the deck.** It confirmed `get(3) == x` but
//! not that the deck still holds the same cards, nor that the cards below index 3 kept
//! their order. Inserting at 3 and then re-sorting, or duplicating, would both have passed.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";
const CHIKA: &str = "PL!S-bp7-010-N";
/// Stands in for the bottom card; a live so it is unmistakably not one of the fillers.
const BOTTOM_CARD: &str = "PL!S-sd1-001-SD";

/// A deck of `n` distinct-ish fillers with `x` pinned at the very bottom, and the
/// top-to-bottom card list as it stood before the trigger.
fn deck_with_bottom_card(game: &mut TestGame, n: usize) -> (i16, Vec<i16>) {
    let mut order = Vec::new();
    for _ in 0..n {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
        order.push(f);
    }
    let x = game.id(BOTTOM_CARD);
    game.assert_card_identity(x, BOTTOM_CARD);
    game.state.player1.main_deck.cards.push(x);
    order.push(x);
    (x, order)
}

#[test]
fn pl_s_bp7_010_n_accept_bottom_card_to_fourth_from_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id(CHIKA);
    game.assert_card_identity(chika, CHIKA);
    game.add_to_stage(MemberArea::Center, chika);

    let (x, order) = deck_with_bottom_card(&mut game, 4);
    let deck_before = order.clone();

    fire_trigger(&mut game, chika, AbilityTrigger::Debut, "登場");
    assert!(
        game.has_pending_choice(),
        "置いてよい — the move is optional, so it must be offered"
    );
    // Answer whatever is asked, accepting.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1),
            other => panic!("unexpected prompt {other:?}"),
        }
    }
    assert!(!game.has_pending_choice(), "every prompt must be answerable");

    let deck: Vec<i16> = game.state.player1.main_deck.cards.to_vec();
    assert_eq!(
        deck.get(3),
        Some(&x),
        "デッキの上から4番目 — X sits at index 3 (4th from the top)"
    );
    assert_ne!(
        deck.last(),
        Some(&x),
        "X left the bottom of the deck"
    );

    // The rest of the deck is untouched: same length, same cards, same relative order,
    // and the four cards that were above X are still above it in their original order.
    assert_eq!(
        deck.len(),
        deck_before.len(),
        "moving X must not add or remove a card: {:?} before, {:?} after",
        deck_before.len(),
        deck.len()
    );
    assert_eq!(
        deck[0..3].to_vec(),
        deck_before[0..3].to_vec(),
        "the three cards above index 3 keep their order and identity"
    );
    // Original deck was [f0, f1, f2, f3, X]. Lifting X out of the bottom and reinserting it
    // at index 3 shifts f3 down one place, giving [f0, f1, f2, X, f3]. So the tail after
    // the insertion point is the ORIGINAL range 3..len-1, not the original tail.
    assert_eq!(
        deck[4..].to_vec(),
        deck_before[3..deck_before.len() - 1].to_vec(),
        "the cards below the insertion point keep their order and identity, shifted down one \
         by X's removal from the bottom: {:?} vs {:?}",
        &deck_before[3..deck_before.len() - 1],
        &deck[4..]
    );
    assert_eq!(
        deck.iter().filter(|&&c| c == x).count(),
        1,
        "X must appear exactly once, not be duplicated"
    );
}

#[test]
fn pl_s_bp7_010_n_decline_keeps_bottom_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id(CHIKA);
    game.add_to_stage(MemberArea::Center, chika);

    // A REAL deck. The previous version used a one-card deck, which made both its
    // assertions true for any implementation whatsoever.
    let (x, order) = deck_with_bottom_card(&mut game, 6);
    let deck_before = order.clone();

    fire_trigger(&mut game, chika, AbilityTrigger::Debut, "登場");
    assert!(
        game.has_pending_choice(),
        "置いてよい — declining requires the option to be offered"
    );

    let mut guard = 0;
    let mut declined = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                game.select_indices(&[]);
                declined += 1;
            }
            Some("SelectTarget") => game.select_option(1),
            other => panic!("unexpected prompt {other:?}"),
        }
    }
    assert!(
        declined >= 1,
        "precondition: the optional move must actually have been declined"
    );

    let deck: Vec<i16> = game.state.player1.main_deck.cards.to_vec();
    assert_eq!(
        deck,
        deck_before,
        "5.7.1: 見る only informs — declining 置いてよい must leave the deck EXACTLY as it \
         was, top to bottom. Any reordering, any removal, any addition is a different reading"
    );
    assert_eq!(
        deck.last(),
        Some(&x),
        "X must still be the bottom card (deck={deck:?})"
    );
    assert_eq!(
        deck.len(),
        7,
        "7 cards in, 7 cards out (deck={deck:?})"
    );
}

/// The card that moves is the BOTTOM one, not just "a card".
///
/// Staging a deck where the bottom card is distinguishable from every other card and
/// checking the resulting position pins both halves of 「一番下のカード…4番目に」 at once.
#[test]
fn pl_s_bp7_010_n_the_card_that_moves_is_specifically_the_bottom_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id(CHIKA);
    game.add_to_stage(MemberArea::Center, chika);

    // Top card is a DIFFERENT distinctive card from the bottom one, so taking the top
    // instead of the bottom would be caught.
    let top = game.id("PL!S-PR-016-PR");
    game.state.player1.main_deck.cards.push(top);
    for _ in 0..5 {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
    let bottom = game.id(BOTTOM_CARD);
    game.state.player1.main_deck.cards.push(bottom);

    fire_trigger(&mut game, chika, AbilityTrigger::Debut, "登場");
    assert!(game.has_pending_choice(), "the optional move is offered");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1),
            other => panic!("unexpected prompt {other:?}"),
        }
    }

    let deck: Vec<i16> = game.state.player1.main_deck.cards.to_vec();
    assert_eq!(
        deck.get(3),
        Some(&bottom),
        "一番下のカード — the card that reaches index 3 must be the BOTTOM card"
    );
    assert_eq!(
        deck.first(),
        Some(&top),
        "the top card must still be the top card; it was not the one 見る'd"
    );
}
