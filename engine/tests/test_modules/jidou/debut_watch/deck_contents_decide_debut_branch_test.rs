//! Deck-driven 登場 outcomes: a branch decided by WHAT THE DECK CONTAINS, and mills
//! whose requested size can exceed the deck.
//!
//! 徒町小鈴 is the sharpest case in the deck. Her 登場 is a SEQUENTIAL with a
//! conditional second step:
//!
//!   「自分のデッキの上からカードを3枚控え室に置く。**それらがすべてメンバーカードの場合**、
//!   カードを1枚引く。」
//!
//! So the deck she is dealt decides whether the second clause runs, and BOTH branches
//! are reachable by choosing deck contents — which makes it the one card where a test
//! can state the branch it is exercising rather than infer it. The quantifier is
//! 「すべて」: ONE non-member among the three kills the draw, so a test that only sees
//! the all-member case proves nothing about the quantifier.
//!
//! 津島善子 and 村野さやか are pure mills — 「デッキの上からカードをN枚控え室に置く」 —
//! and the N is larger than a legal deck in the tight case. What happens when the deck
//! cannot supply N cards is not stated by the printed text, and each of these had a
//! single test that never varied the deck size, so the short-deck reading was pinned by
//! accident rather than by choice.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KOSUZU: &str = "PL!HS-bp1-008-R"; // 徒町小鈴 — mill 3, draw if all 3 are members
const YOSHIKO: &str = "PL!S-bp5-015-N"; // 津島善子 — mill 10
const SAYAKA_MILL: &str = "PL!HS-bp2-011-PR"; // 村野さやか — mill 5
const FILLER: &str = "PL!-sd1-010-SD";
/// A ライブ card — a NON-member, so it is what disqualifies the all-member branch.
const NON_MEMBER: &str = "PL!-sd1-019-SD";

/// Reset p1's deck to `cards`, in order, from the top down.
fn stack_deck(game: &mut TestGame, cards: &[&str]) {
    game.state.player1.main_deck.cards.clear();
    for card_no in cards {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(card_no));
    }
}

/// Reset the waitroom and hand so a count is attributable to the ability.
fn clear_pools(game: &mut TestGame) {
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
}

fn drain(game: &mut TestGame) {
    while game.has_pending_choice() {
        if game.pending_choice_type().as_deref() == Some("SelectTarget") {
            game.select_option(0);
        } else {
            game.select_indices(&[]);
        }
    }
}

// ====================================================================
// 徒町小鈴 — 「それらがすべてメンバーカードの場合」
// ====================================================================

/// The ALL-members branch: three members on top, so the draw runs.
///
/// Stated as an exact count on BOTH pools, because the ability moves cards between
/// them: three in, one out. Asserting only the waitroom would pass if the draw had
/// quietly milled a fourth card.
#[test]
fn kosuzu_draws_when_the_three_milled_cards_are_all_members() {
    let mut game = TestGame::new(load_real_database());
    let kosuzu = game.id(KOSUZU);
    game.assert_card_identity(kosuzu, KOSUZU);
    clear_pools(&mut game);
    game.give_energy(10);
    // Three MEMBERS on top. PL!-sd1-010-SD is a member (used as the filler
    // throughout this suite), so all three qualify.
    stack_deck(&mut game, &[FILLER, FILLER, FILLER]);
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    game.add_to_hand(kosuzu);
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        13,
        "precondition: 13 cards with three on top"
    );
    game.state.player1.stage.stage = [kosuzu, -1, -1];

    game.play_to_stage(kosuzu, MemberArea::Center);
    drain(&mut game);

    // The card itself leaves the deck on play; the ability mills 3 more.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        9,
        "playing her removes 1 and her 登場 mills 3: 13 - 1 - 3 = 9"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        3,
        "exactly the three milled cards are in the waitroom"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "『それらがすべてメンバーカードの場合』 holds, so exactly one card was drawn — \
         the hand is the second clause's whole observable"
    );
}

/// The NEGATIVE branch: ONE ライブ among the three, so 「すべて」 is false and no
/// card is drawn even though the same three cards were milled.
///
/// This is the assertion the quantifier needs. A `any`-style implementation grants
/// here, and a test that only ran the all-member case would pass it.
#[test]
fn kosuzu_does_not_draw_when_any_of_the_three_milled_cards_is_not_a_member() {
    let mut game = TestGame::new(load_real_database());
    let kosuzu = game.id(KOSUZU);
    game.assert_card_identity(kosuzu, KOSUZU);
    clear_pools(&mut game);
    game.give_energy(10);
    // Two members and one ライブ: the mill is identical, the branch is not.
    stack_deck(&mut game, &[FILLER, NON_MEMBER, FILLER]);
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    game.add_to_hand(kosuzu);
    game.state.player1.stage.stage = [kosuzu, -1, -1];

    game.play_to_stage(kosuzu, MemberArea::Center);
    drain(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        3,
        "the same three cards are milled either way — the first clause does not \
         depend on the branch"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "『すべてメンバーカード』 is false with one ライブ among the three, so NO \
         card is drawn. The waitroom count above is identical to the positive case, \
         which is what makes this a test of the quantifier rather than of the mill."
    );
}

/// A deck with FEWER than three cards: the mill asks for 3 and cannot supply them.
/// Whatever the engine does, the interesting part is that the two clauses cannot be
/// satisfied by the same three cards, so no draw may follow.
#[test]
fn kosuzu_with_a_two_card_deck_mills_what_there_is_and_draws_nothing() {
    let mut game = TestGame::new(load_real_database());
    let kosuzu = game.id(KOSUZU);
    clear_pools(&mut game);
    game.give_energy(10);
    stack_deck(&mut game, &[FILLER, FILLER]);
    game.add_to_hand(kosuzu);
    game.state.player1.stage.stage = [kosuzu, -1, -1];
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        2,
        "precondition: fewer cards than the mill requests"
    );

    game.play_to_stage(kosuzu, MemberArea::Center);
    drain(&mut game);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "the deck is emptied — an under-supplied mill takes what is there rather \
         than leaving cards behind, and the count is what says so"
    );
    assert!(
        game.state.player1.hand.cards.is_empty(),
        "no card is drawn: there were only two cards to be 『すべてメンバーカード』, \
         and a short mill must not be read as a satisfied one (got {:?})",
        game.state.player1.hand.cards
    );
}

// ====================================================================
// The two mills — requested size vs deck size
// ====================================================================

/// 津島善子 mills exactly TEN. The deck can hold fewer than that, and the printed
/// text does not say what happens then — so the reading is pinned deliberately here
/// rather than left to whichever order the suite happened to build the deck in.
#[test]
fn yoshiko_mills_exactly_ten_and_a_short_deck_mills_what_there_is() {
    let mut game = TestGame::new(load_real_database());
    let yoshiko = game.id(YOSHIKO);
    game.assert_card_identity(yoshiko, YOSHIKO);
    clear_pools(&mut game);
    game.give_energy(20);
    stack_deck(&mut game, &[FILLER; 20]);
    game.add_to_hand(yoshiko);

    game.play_to_stage(yoshiko, MemberArea::Center);
    drain(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        10,
        "『デッキの上からカードを10枚控え室に置く』 moves exactly 10"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        10,
        "20 - 10 (milled) = 10"
    );

    // Now the short-deck reading, which the single existing test never varied.
    let mut short = TestGame::new(load_real_database());
    let yoshiko2 = short.id(YOSHIKO);
    clear_pools(&mut short);
    short.give_energy(20);
    stack_deck(&mut short, &[FILLER; 3]);
    short.add_to_hand(yoshiko2);
    assert_eq!(
        short.state.player1.main_deck.cards.len(),
        3,
        "precondition: three cards, ten requested"
    );

    short.play_to_stage(yoshiko2, MemberArea::Center);
    drain(&mut short);

    assert_eq!(
        short.state.player1.main_deck.cards.len(),
        0,
        "an under-supplied mill empties the deck rather than refusing: the ability \
         still resolves, so the three remaining cards go to the waitroom"
    );
    assert_eq!(
        short.state.player1.waitroom.cards.len(),
        3,
        "and the waitroom receives those three — not ten. This is the reading the \
         text does not state, pinned so a change to it is a deliberate one."
    );
}

/// 村野さやか mills 5. A different requested size, so the pair separates "the mill
/// runs" from "the mill runs at the printed size".
#[test]
fn sayaka_mills_exactly_five_not_ten() {
    let mut game = TestGame::new(load_real_database());
    let sayaka = game.id(SAYAKA_MILL);
    game.assert_card_identity(sayaka, SAYAKA_MILL);
    clear_pools(&mut game);
    game.give_energy(10);
    stack_deck(&mut game, &[FILLER; 20]);
    game.add_to_hand(sayaka);

    game.play_to_stage(sayaka, MemberArea::Center);
    drain(&mut game);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        5,
        "『デッキの上からカードを5枚控え室に置く』 moves 5 — 津島善子's 10 from the same \
         fixture is what shows the printed size is honoured rather than a shared \
         default"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        15,
        "20 - 5 (milled) = 15"
    );
}
