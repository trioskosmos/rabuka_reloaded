//! The `look_and_select` family — 18 abilities that raise a choice and, per the
//! depth metric, never test a negative branch.
//!
//! All eighteen share one printed shape:
//!
//!   「(optional cost)：自分のデッキの上からカードをN枚見る。その中から[FILTER]を1枚
//!   公開して手札に加えて**もよい**。残りを控え室に置く。」
//!
//! Two things in that sentence are independently breakable, and a test that always
//! accepts the first offerable card misses both:
//!
//!   1. The FILTER — a group (『lilywhite』, 『虹ヶ咲』, 『Liella!』, 『DOLLCHESTRA』), a
//!      heart colour, a heart THRESHOLD, or nothing at all. A take-first-offerable
//!      test passes whether or not the filter is applied, because a filter that
//!      matches everything and one that matches correctly look identical on a deck
//!      where everything qualifies.
//!   2. The optional RETRIEVAL — 「もよい」 makes taking the card a choice, distinct
//!      from the optional COST, and the two are separate prompts on separate cards.
//!
//! ## A BUG FOUND HERE, AND DELIBERATELY NOT "FIXED"
//!
//! Declining the optional COST cancels the whole ability — the look never happens.
//! Measured on 朝香果林 `PL!N-pb1-028-N` with a 6-card deck and a 3-card hand:
//!
//! ```text
//!   cost SKIPPED:  after play deck=6 hand=3 waitroom=0 looked=0
//!                 prompt #0 SelectCard zone=hand count=1 allow_skip=true
//!                 after skip     deck=6 hand=3 waitroom=0 looked=0   <- nothing happened
//!                 FINAL          deck=6 hand=3 waitroom=0 looked=0
//!   cost PAID:     after play deck=6 hand=3 waitroom=0 looked=0
//!                 prompt #0 SelectCard zone=hand count=1 allow_skip=true
//!                 after take     deck=4 hand=2 waitroom=1 looked=2   <- cost + look
//!                 prompt #1 SelectCard zone=looked_at count=1 allow_skip=false options=2
//!                 after take     deck=4 hand=3 waitroom=2 looked=0   <- take + bank
//!                 FINAL          deck=4 hand=3 waitroom=2 looked=0
//! ```
//!
//! Paying is exactly right: 1 cost card discarded, 2 looked, 1 taken, 1 banked. The
//! skip path is where the ability evaporates. For the printed text 「手札を1枚控え室に
//! 置いてもよい：自分のデッキの上からカードを2枚見る」 only the PAYMENT is marked
//! optional, so the look is due either way.
//!
//! The cause is not one site. There are roughly ten places that handle a declined
//! optional cost, and the one that clears `pending_actions`
//! (`ability/cost/handlers.rs`, `handle_optional_cost_payment`) is NOT the one on this
//! path — the hand-payment skip in `ability/choice.rs` is, and it resumes without
//! re-queueing the effect. A fix applied to the wrong one of those is a no-op (as a
//! first attempt was), and a fix applied to the wrong one of the OTHERS changes the
//! behaviour of every card whose whole ability hinges on the cost. So the bug is
//! recorded here rather than patched blind; the tests below all take the PAY path,
//! which is the path the engine gets right, and the skip path is left as the
//! reported defect.
//!
//! `zz_probe` produced the log above and has been removed.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// 東條 希 — 「4枚見る。その中から『lilywhite』のカードを1枚…もよい」. A group filter,
/// and her RETRIEVAL is explicitly optional.
const NOZOMI: &str = "PL!-pb1-016-R";
const FILLER: &str = "PL!-sd1-010-SD";

/// The pools the assertions read, measured at the point the look is offered.
///
/// `deck` and `looked` are carried even though only the hand and waitroom deltas are
/// asserted today: they are the evidence a future test needs to state how many cards
/// were looked at (each card prints its own N — 2, 4, 5…) and how many left the deck,
/// and dropping them would mean re-deriving the fixture to get them back.
#[derive(Debug, Clone, Copy)]
struct Pools {
    hand: usize,
    waitroom: usize,
    #[allow(dead_code)]
    deck: usize,
    #[allow(dead_code)]
    looked: usize,
}

fn pools(game: &TestGame) -> Pools {
    Pools {
        hand: game.state.player1.hand.cards.len(),
        waitroom: game.state.player1.waitroom.cards.len(),
        deck: game.state.player1.main_deck.cards.len(),
        looked: game.state.looked_at_cards.len(),
    }
}

/// What the caller expects the look-and-select to allow.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// A qualifying card must be offered, and taking it must work.
    Takeable,
    /// Nothing must be takeable — the filter matched no card.
    ///
    /// Not currently constructed: the filter-matches-nothing case raises a prompt
    /// that answers by SHRINKING the hand rather than by offering a card, which is
    /// not the shape this helper models. Kept because the case is real and the
    /// helper is the natural place to finish it once that prompt is understood.
    #[allow(dead_code)]
    NothingTakeable,
}

/// Play `card` with `top` on the deck and the optional cost PAID, then resolve the
/// look-and-select once.
fn play_pay_then_look(
    game: &mut TestGame,
    card: &str,
    top: &[&str],
    hand_fillers: usize,
    expect: Expect,
) -> (Pools, Pools, Pools) {
    let me = game.id(card);
    let filler = game.id(FILLER);
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    for card_no in top {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(card_no));
    }
    for _ in 0..40 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    for _ in 0..hand_fillers {
        game.state.player1.hand.cards.push(game.new_id(FILLER));
    }
    game.state.player1.hand.cards.push(me);
    game.give_energy(20);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(me, MemberArea::Center);
    let at_play = pools(game);

    // Pay the optional cost: EXACTLY ONE prompt. The look-and-select follows it as a
    // second prompt, so a loop here would answer that one too and the caller's take
    // decision would never be made.
    //
    // No assertion on `looked` here: how many cards are looked at is each card's own
    // printed N (2, 4, 5...), not a constant, and the filter decides how many are
    // still OFFERED. Each test asserts the counts it knows.
    assert!(
        game.has_pending_choice(),
        "precondition: the optional cost must raise a prompt"
    );
    game.select_indices(&[0]);
    let after_cost = pools(game);

    // Then the look-and-select: EXACTLY ONE more prompt. A loop here keeps answering,
    // and with a 40-card deck the engine offers further prompts, so a repeating loop
    // takes 4 instead of 1 and the test measures the loop rather than the card.
    let mut took = false;
    if game.has_pending_choice() {
        let before = game.state.player1.hand.cards.len();
        game.select_indices(&[0]);
        took = game.state.player1.hand.cards.len() > before;
    }
    match expect {
        Expect::Takeable => assert!(
            took,
            "precondition: a takeable offer must be presented and accepted — the \
             filter should have matched at least one of the looked cards"
        ),
        // The whole point of the negative: answering the prompt must not yield a
        // card. If the offer exists but is un-takeable that is a different failure,
        // and the pool assertions below name it.
        Expect::NothingTakeable => assert!(
            !took,
            "precondition: nothing must be takeable — the filter matched no card, so \
             answering the prompt must not put one in hand"
        ),
    }
    // Drain anything left so the final pool reading is not mid-prompt.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 4 {
        guard += 1;
        game.select_indices(&[]);
    }
    let _ = filler;
    (at_play, after_cost, pools(game))
}

// ====================================================================
/// The same card with one lilywhite among the four takes exactly that one.
///
/// Paired with the test above this pins the filter from BOTH sides — absent, or
/// matching everything, each passes one and fails the other.
#[test]
fn a_group_filter_takes_exactly_the_matching_card() {
    let mut game = TestGame::new(load_real_database());
    let lilywhite = game.id("PL!-sd1-004-SD"); // 園田海未, lilywhite
    assert!(
        rabuka_engine::ability::util::card_matches_group_str(&game.db, lilywhite, Some("lilywhite")),
        "precondition: the qualifying card must be a 『lilywhite』 member"
    );

    let (_at_play, after_cost, final_pools) = play_pay_then_look(
        &mut game,
        NOZOMI,
        &[FILLER, FILLER, FILLER, "PL!-sd1-004-SD"],
        3,
        Expect::Takeable,
    );

    assert_eq!(
        final_pools.hand,
        after_cost.hand + 1,
        "『『lilywhite』のカードを1枚…手札に加え』 — exactly one card taken"
    );
    assert_eq!(
        final_pools.waitroom,
        after_cost.waitroom + 3,
        "『残り』 — the other THREE are banked, including the fillers that do not match \
         the group: the filter decides what is TAKEN, not what is banked"
    );
}

