//! 东条 希's 登場, from BOTH sides of its group filter.
//!
//! 「手札を1枚控え室に置いてもよい：自分のデッキの上からカードを4枚見る。その中から
//! **『lilywhite』**のカードを1枚公開して手札に加えてもよい。残りを控え室に置く。」
//!
//! 4 of the 18 abilities in this deck share that shape — 平安名すみれ and ウィーン and
//! 桜小路きな子 among them — and all eighteen name UNIT-level groups, which is the
//! sharper kind: see `recover/to_hand/unit_group_recovery_excludes_same_series_test.rs`
//! for why a series-level name would not discriminate here.
//!
//! The card's only prior coverage took the matching branch, and on a board where
//! every looked card qualifies — or none does — a take-first-offerable test cannot
//! tell a working group filter from no filter at all. What separates them is ONE
//! card among the four, and the exact deltas that follow from it.
//!
//! ## The prompt shape, measured rather than assumed
//!
//! Three earlier drafts of this file were wrong in the same way: the helper decided
//! what the ability SHOULD do instead of reporting what it DID, so each failure
//! surfaced as an arithmetic mismatch with nothing readable in it. Recording the
//! prompt summaries settled it — there is exactly ONE prompt, the optional cost, and
//! the cost, the look and the banking ALL resolve on it:
//!
//! ```text
//!   after play:  deck=44 hand=3 waitroom=0 looked=0
//!   prompt:      SelectCard zone=hand count=1 allow_skip=true
//!   after [0]:   deck=40 hand=2 waitroom=5 looked=0     (nothing matched)
//!                deck=40 hand=3 waitroom=4 looked=0     (one matched)
//! ```
//!
//! So there is no "after the cost, before the look" state to sample: a test that
//! tries to take one compares a post-resolution pool against a pre-resolution
//! number. The two observable points are the play and the answer, and both tests
//! below assert deltas between exactly those two.
//!
//! ## The skipped-cost path, and why it is NOT a defect
//!
//! Declining the optional cost cancels the rest of this ability. Measured on
//! 朝香果林 `PL!N-pb1-028-N` (same shape, N=2):
//!
//! ```text
//!   cost SKIPPED:  deck=6 hand=3 waitroom=0 looked=0  ->  deck=6 hand=3 waitroom=0 looked=0
//!   cost PAID:     deck=6 hand=3 waitroom=0 looked=0  ->  deck=4 hand=2 waitroom=1 looked=2
//! ```
//!
//! An earlier draft of this header called that a defect and left it unpatched. It
//! is not one, and the engine says so in its own source
//! (`ability/cost/handlers.rs:364-366`):
//!
//! > NOTE: this cost colon-gates the effect. When skipped (or empty-hand auto-skip),
//! > the resolver's cost_was_skipped path prevents the gated effect from firing
//! > (e.g. 「手札をすべて控え室に置いてもよい：カードを6枚引く」).
//!
//! The worked example is nearly this card. So the convention in this engine is that
//! a declined optional cost gates everything after the 「：」 — which is why
//! `turn/actions/mod.rs` resolves the resume as `CompleteSkipped` before it
//! considers `effect_ready`, and why the cost handlers clear `pending_actions` on a
//! decline rather than re-queueing the effect. Three separate sites agree; this is
//! a design decision with a stated rationale, not an oversight.
//!
//! Whether the OFFICIAL card rules read 「…置いてもよい：A」 as gating A is a
//! rules-interpretation question that the code cannot settle, and it is the only
//! thing here worth escalating. An attempt to "fix" the resume branch was written,
//! verified to compile, and confirmed to flip `optional_skipped` to false — then
//! reverted, because it contradicted the note above without producing the intended
//! behaviour: the effect is never queued behind a declined cost, so no resume policy
//! can recover it. Both tests here take the PAY path, which is the path the engine
//! handles and which the numbers below describe exactly.


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

/// Resolve 希's 登場 and report the pools either side of its single prompt.

///
/// The helper ASSERTS nothing about the outcome — that is the caller's job, with
/// numbers the caller can see. An earlier version tried to decide "was something
/// takeable" here and got it wrong twice: it treated a declining answer as a
/// takeable one, and it measured the hand against a baseline that predated the
/// cost. Both mistakes were invisible because the helper, not the test, owned the
/// expectation.
///
/// Measured on the real dispatch (this is the shape, not a guess):
///
/// ```text
///   after play:  deck=44 hand=3 waitroom=0 looked=0
///   prompt:      SelectCard zone=hand count=1 allow_skip=true
///   after [0]:   deck=40 hand=2 waitroom=5 looked=0     (nothing matched)
/// ```
///
/// Note there is only ONE prompt and everything resolves on it — cost, look and
/// banking included. So there is no "after the cost, before the look" state to
/// sample, and a test that tries to take one ends up comparing a post-resolution
/// pool against a pre-resolution number. The two observable points are the play and
/// the answer.
fn resolve_and_report(game: &mut TestGame, card: &str, top: &[&str], hand_fillers: usize) -> (Pools, Pools) {
    let filler = game.id(FILLER);
    let me = game.id(card);
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
    game.add_to_hand(me);
    game.give_energy(20);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(me, MemberArea::Center);
    let at_play = pools(game);

    // Answer the cost prompt; the rest of the ability resolves with it.
    assert!(
        game.has_pending_choice(),
        "precondition: the optional cost must raise a prompt"
    );
    game.select_indices(&[0]);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 6 {
        guard += 1;
        game.select_indices(&[0]);
    }
    let _ = filler;
    (at_play, pools(game))
}

/// The negative this family was missing: a deck with NO lilywhite card.
///
/// 「『lilywhite』のカードを1枚…もよい」 cannot be satisfied, so nothing is recovered.
/// The three deltas say which steps ran: the hand LOSES one (the cost was paid),
/// the deck loses four (the look ran), and the waitroom gains five (the cost card
/// plus all four looked). A run where the look never happened leaves the waitroom
/// at 1 and the deck at 44; a run where the group filter was ignored ends with the
/// hand at 3 and the waitroom at 4.
#[test]
fn a_group_filter_that_matches_nothing_banks_every_looked_card() {
    let mut game = TestGame::new(load_real_database());
    let nozomi = game.id(NOZOMI);
    let outsiders = ["PL!-sd1-001-SD", "PL!-sd1-002-SD", "PL!-sd1-003-SD", FILLER];
    game.assert_card_identity(nozomi, NOZOMI);
    for card_no in outsiders {
        assert!(
            !rabuka_engine::ability::util::card_matches_group_str(
                &game.db,
                game.id(card_no),
                Some("lilywhite")
            ),
            "precondition: {card_no} must be OUTSIDE 『lilywhite』, or the group \
             filter is never exercised in the negative direction"
        );
    }

    let (at_play, final_pools) = resolve_and_report(&mut game, NOZOMI, &outsiders, 3);

    assert_eq!(at_play.hand, 3, "precondition: three cards in hand before the cost");
    assert_eq!(
        at_play.deck, 44,
        "precondition: four seeded cards on top plus 40 filler"
    );
    assert_eq!(
        final_pools.deck,
        at_play.deck - 4,
        "『4枚見る』 — exactly four cards left the deck, so the look really ran"
    );
    assert_eq!(
        final_pools.hand,
        at_play.hand - 1,
        "the optional cost is PAID and nothing is recovered: the hand ends one \
         lower. A group filter that ignored itself would end at {} here.",
        at_play.hand
    );
    assert_eq!(
        final_pools.waitroom,
        at_play.waitroom + 5,
        "one cost card plus all four looked cards are banked — the filter decides \
         what is TAKEN, not what is banked, and this is the whole claim"
    );
    assert_eq!(
        final_pools.looked, 0,
        "nothing is left looked at once the selection resolves"
    );
}

/// The positive, and the control for the test above: one lilywhite among the four
/// moves the hand and the waitroom by exactly one relative to it.
#[test]
fn a_group_filter_takes_exactly_the_matching_card() {
    let mut game = TestGame::new(load_real_database());
    let nozomi = game.id(NOZOMI);
    let lilywhite = game.id("PL!-sd1-004-SD"); // 園田海未, lilywhite
    game.assert_card_identity(nozomi, NOZOMI);
    assert!(
        rabuka_engine::ability::util::card_matches_group_str(&game.db, lilywhite, Some("lilywhite")),
        "precondition: the qualifying card must be a 『lilywhite』 member"
    );

    let (at_play, final_pools) = resolve_and_report(
        &mut game,
        NOZOMI,
        &[FILLER, FILLER, FILLER, "PL!-sd1-004-SD"],
        3,
    );

    assert_eq!(
        final_pools.deck,
        at_play.deck - 4,
        "the same four left the deck as in the negative arm, so the pair differs \
         only in the filter's verdict"
    );
    assert_eq!(
        final_pools.hand,
        at_play.hand,
        "『『lilywhite』のカードを1枚…手札に加え』 — the cost's -1 and the recovery's \
         +1 cancel, so the hand ends where it started. That is the ONE number that \
         separates this from the negative arm, which ends one lower."
    );
    assert_eq!(
        final_pools.waitroom,
        at_play.waitroom + 4,
        "『残り』 — the cost card plus three of the four looked stay banked: the \
         group filter decides what is TAKEN, not what is banked"
    );
}
